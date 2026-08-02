use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use vector_ta::utilities::data_loader::Candles;

pub const ANALYSIS_SERIES_MAX_POINTS: usize = 8_000;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisSeriesKind {
    Line,
    Histogram,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisSeriesUnit {
    Price,
    PriceDelta,
    Number,
    Ratio,
    Degrees,
    State,
    ZScore,
    Oscillator,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisSeriesAggregation {
    LastFinite,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AnalysisReferenceLine {
    pub value: f64,
    pub label: String,
}

impl AnalysisReferenceLine {
    pub(crate) fn new(value: f64, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AnalysisSeriesBar {
    pub time: i64,
    pub from_index: u32,
    pub to_index: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AnalysisNamedSeries {
    pub id: String,
    pub label: String,
    pub kind: AnalysisSeriesKind,
    pub pane_id: String,
    pub scale_id: String,
    pub unit: AnalysisSeriesUnit,
    pub default_visible: bool,
    pub values: Vec<Option<f64>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reference_lines: Vec<AnalysisReferenceLine>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct StrategyIndicatorBundle {
    pub strategy_kind: String,
    pub aggregation: AnalysisSeriesAggregation,
    pub bars: Vec<AnalysisSeriesBar>,
    pub series: Vec<AnalysisNamedSeries>,
}

pub(crate) struct AnalysisSeriesInput<'a> {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: AnalysisSeriesKind,
    pub pane_id: &'static str,
    pub scale_id: &'static str,
    pub unit: AnalysisSeriesUnit,
    pub default_visible: bool,
    pub values: &'a [f64],
    pub reference_lines: Vec<AnalysisReferenceLine>,
}

impl<'a> AnalysisSeriesInput<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: &'static str,
        label: &'static str,
        kind: AnalysisSeriesKind,
        pane_id: &'static str,
        scale_id: &'static str,
        unit: AnalysisSeriesUnit,
        default_visible: bool,
        values: &'a [f64],
    ) -> Self {
        Self {
            id,
            label,
            kind,
            pane_id,
            scale_id,
            unit,
            default_visible,
            values,
            reference_lines: Vec::new(),
        }
    }

    pub(crate) fn with_reference_lines(
        mut self,
        reference_lines: Vec<AnalysisReferenceLine>,
    ) -> Self {
        self.reference_lines = reference_lines;
        self
    }
}

fn point_cap(requested_bins: Option<usize>) -> usize {
    requested_bins
        .unwrap_or(ANALYSIS_SERIES_MAX_POINTS)
        .clamp(1, ANALYSIS_SERIES_MAX_POINTS)
}

pub(crate) fn build_strategy_indicator_bundle(
    candles: &Candles,
    strategy_kind: &str,
    requested_bins: Option<usize>,
    inputs: Vec<AnalysisSeriesInput<'_>>,
) -> Result<StrategyIndicatorBundle, String> {
    let strategy_kind = strategy_kind.trim();
    if strategy_kind.is_empty() {
        return Err("analysis series strategy kind must not be empty".to_string());
    }

    let n = candles.timestamp.len();
    let mut ids = HashSet::with_capacity(inputs.len());
    for input in &inputs {
        if input.id.trim().is_empty()
            || input.label.trim().is_empty()
            || input.pane_id.trim().is_empty()
            || input.scale_id.trim().is_empty()
        {
            return Err("analysis series metadata must not be empty".to_string());
        }
        if !ids.insert(input.id) {
            return Err(format!("duplicate analysis series id: {}", input.id));
        }
        if input.values.len() != n {
            return Err(format!(
                "analysis series {} length mismatch: expected {}, got {}",
                input.id,
                n,
                input.values.len()
            ));
        }
        if input
            .reference_lines
            .iter()
            .any(|line| !line.value.is_finite() || line.label.trim().is_empty())
        {
            return Err(format!(
                "analysis series {} has an invalid reference line",
                input.id
            ));
        }
    }

    let mut named_series: Vec<AnalysisNamedSeries> = inputs
        .iter()
        .map(|input| AnalysisNamedSeries {
            id: input.id.to_string(),
            label: input.label.to_string(),
            kind: input.kind,
            pane_id: input.pane_id.to_string(),
            scale_id: input.scale_id.to_string(),
            unit: input.unit,
            default_visible: input.default_visible,
            values: Vec::new(),
            reference_lines: input.reference_lines.clone(),
        })
        .collect();
    if n == 0 || inputs.is_empty() {
        return Ok(StrategyIndicatorBundle {
            strategy_kind: strategy_kind.to_string(),
            aggregation: AnalysisSeriesAggregation::LastFinite,
            bars: Vec::new(),
            series: named_series,
        });
    }

    let cap = point_cap(requested_bins);
    let step = n.div_ceil(cap);
    let expected_bars = n.div_ceil(step);
    let mut bars = Vec::with_capacity(expected_bars);
    for series in &mut named_series {
        series.values.reserve(expected_bars);
    }

    let mut previous_time: Option<i64> = None;
    let mut bucket_values = Vec::with_capacity(inputs.len());
    let mut from = 0usize;
    while from < n {
        let to_exclusive = n.min(from.saturating_add(step));
        let to = to_exclusive.saturating_sub(1);
        let time = candles.timestamp[from];
        if previous_time.is_some_and(|previous| time <= previous) {
            from = to_exclusive;
            continue;
        }

        bucket_values.clear();
        bucket_values.extend(inputs.iter().map(|input| {
            (from..to_exclusive).rev().find_map(|index| {
                let value = input.values[index];
                value.is_finite().then_some(value)
            })
        }));
        if bucket_values.iter().all(Option::is_none) {
            from = to_exclusive;
            continue;
        }

        bars.push(AnalysisSeriesBar {
            time,
            from_index: u32::try_from(from)
                .map_err(|_| "dataset too large for analysis series index".to_string())?,
            to_index: u32::try_from(to)
                .map_err(|_| "dataset too large for analysis series index".to_string())?,
        });
        for (series, value) in named_series.iter_mut().zip(bucket_values.iter().copied()) {
            series.values.push(value);
        }
        previous_time = Some(time);
        from = to_exclusive;
    }

    Ok(StrategyIndicatorBundle {
        strategy_kind: strategy_kind.to_string(),
        aggregation: AnalysisSeriesAggregation::LastFinite,
        bars,
        series: named_series,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candles_with_timestamps(timestamps: Vec<i64>) -> Candles {
        let len = timestamps.len();
        let close: Vec<f64> = (0..len).map(|index| 100.0 + index as f64).collect();
        Candles::new(
            timestamps,
            close.clone(),
            close.iter().map(|value| value + 1.0).collect(),
            close.iter().map(|value| value - 1.0).collect(),
            close,
            vec![1.0; len],
        )
    }

    #[test]
    fn compact_bundle_uses_shared_bars_and_last_finite_values() {
        let candles = candles_with_timestamps((0..10).map(|index| index * 60).collect());
        let first = [f64::NAN, 1.0, 2.0, 3.0, 4.0, f64::NAN, 6.0, 7.0, 8.0, 9.0];
        let second = [
            f64::NAN,
            f64::NAN,
            f64::NAN,
            30.0,
            f64::NAN,
            50.0,
            f64::NAN,
            70.0,
            f64::NAN,
            f64::NAN,
        ];
        let bundle = build_strategy_indicator_bundle(
            &candles,
            "Test",
            Some(3),
            vec![
                AnalysisSeriesInput::new(
                    "first",
                    "First",
                    AnalysisSeriesKind::Line,
                    "price",
                    "price",
                    AnalysisSeriesUnit::Price,
                    true,
                    &first,
                ),
                AnalysisSeriesInput::new(
                    "second",
                    "Second",
                    AnalysisSeriesKind::Histogram,
                    "oscillator",
                    "oscillator",
                    AnalysisSeriesUnit::Number,
                    false,
                    &second,
                ),
            ],
        )
        .expect("compact series bundle");

        assert_eq!(bundle.aggregation, AnalysisSeriesAggregation::LastFinite);
        assert_eq!(bundle.bars.len(), 3);
        assert_eq!(
            bundle
                .bars
                .iter()
                .map(|bar| (bar.from_index, bar.to_index))
                .collect::<Vec<_>>(),
            vec![(0, 3), (4, 7), (8, 9)]
        );
        assert_eq!(
            bundle.series[0].values,
            vec![Some(3.0), Some(7.0), Some(9.0)]
        );
        assert_eq!(bundle.series[1].values, vec![Some(30.0), Some(70.0), None]);
        assert!(bundle
            .series
            .iter()
            .all(|series| series.values.len() == bundle.bars.len()));
    }

    #[test]
    fn compact_bundle_caps_requested_points_at_eight_thousand() {
        let candles = candles_with_timestamps((0..16_001).map(|index| index as i64).collect());
        let values: Vec<f64> = (0..16_001).map(|index| index as f64).collect();
        let bundle = build_strategy_indicator_bundle(
            &candles,
            "Test",
            Some(usize::MAX),
            vec![AnalysisSeriesInput::new(
                "value",
                "Value",
                AnalysisSeriesKind::Line,
                "price",
                "price",
                AnalysisSeriesUnit::Price,
                true,
                &values,
            )],
        )
        .expect("capped series bundle");

        assert!(bundle.bars.len() <= ANALYSIS_SERIES_MAX_POINTS);
        assert_eq!(bundle.series[0].values.len(), bundle.bars.len());
        assert_eq!(
            bundle
                .bars
                .iter()
                .take(2)
                .map(|bar| (bar.time, bar.from_index, bar.to_index))
                .collect::<Vec<_>>(),
            vec![(0, 0, 2), (3, 3, 5)]
        );
        assert_eq!(bundle.bars.last().map(|bar| bar.to_index), Some(16_000));
    }

    #[test]
    fn compact_bundle_skips_non_monotonic_and_all_null_buckets_in_lockstep() {
        let candles = candles_with_timestamps(vec![10, 20, 15, 30, 40]);
        let values = [1.0, 2.0, 3.0, 4.0, f64::NAN];
        let bundle = build_strategy_indicator_bundle(
            &candles,
            "Test",
            Some(5),
            vec![AnalysisSeriesInput::new(
                "value",
                "Value",
                AnalysisSeriesKind::Line,
                "oscillator",
                "oscillator",
                AnalysisSeriesUnit::Number,
                true,
                &values,
            )],
        )
        .expect("filtered series bundle");

        assert_eq!(
            bundle.bars.iter().map(|bar| bar.time).collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
        assert_eq!(
            bundle.series[0].values,
            vec![Some(1.0), Some(2.0), Some(4.0)]
        );
    }

    #[test]
    fn compact_bundle_rejects_misaligned_duplicate_or_invalid_metadata() {
        let candles = candles_with_timestamps(vec![1, 2, 3]);
        let short = [1.0, 2.0];
        let err = build_strategy_indicator_bundle(
            &candles,
            "Test",
            None,
            vec![AnalysisSeriesInput::new(
                "short",
                "Short",
                AnalysisSeriesKind::Line,
                "price",
                "price",
                AnalysisSeriesUnit::Price,
                true,
                &short,
            )],
        )
        .expect_err("length mismatch");
        assert!(err.contains("length mismatch"));

        let values = [1.0, 2.0, 3.0];
        let err = build_strategy_indicator_bundle(
            &candles,
            "Test",
            None,
            vec![
                AnalysisSeriesInput::new(
                    "same",
                    "First",
                    AnalysisSeriesKind::Line,
                    "price",
                    "price",
                    AnalysisSeriesUnit::Price,
                    true,
                    &values,
                ),
                AnalysisSeriesInput::new(
                    "same",
                    "Second",
                    AnalysisSeriesKind::Line,
                    "price",
                    "price",
                    AnalysisSeriesUnit::Price,
                    true,
                    &values,
                ),
            ],
        )
        .expect_err("duplicate id");
        assert!(err.contains("duplicate analysis series id"));

        let err = build_strategy_indicator_bundle(
            &candles,
            "Test",
            None,
            vec![AnalysisSeriesInput::new(
                "value",
                "Value",
                AnalysisSeriesKind::Line,
                "price",
                "price",
                AnalysisSeriesUnit::Price,
                true,
                &values,
            )
            .with_reference_lines(vec![AnalysisReferenceLine::new(f64::NAN, "invalid")])],
        )
        .expect_err("invalid reference line");
        assert!(err.contains("invalid reference line"));
    }
}
