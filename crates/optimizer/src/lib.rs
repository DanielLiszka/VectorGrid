pub mod grid;
pub mod mo;
pub mod stream;

use serde::{Deserialize, Serialize};
use ta_strategies::double_ma::{DoubleMaParams, Metrics};
use vector_ta::utilities::data_loader::Candles;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ObjectiveKind {
    Pnl,
    Sharpe,
    Sortino,
    Calmar,
    Omega,
    Volatility,
    MaxDrawdown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptimizationMode {
    Grid,
    CoarseToFine,

    GpuVdps,
    Auto,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScoreRankKey {
    pub valid: bool,
    pub score: f64,
    pub tie_index: usize,
}

pub fn normalize_rank_score(score: f64) -> f64 {
    if score.is_finite() {
        score
    } else {
        f64::NEG_INFINITY
    }
}

pub const HEATMAP_CELL_ORDER_SLOW_FAST_ROW_MAJOR: &str = "slow_fast_row_major_v1";

pub fn heatmap_axis_bins_for_period_count(requested_bins: usize, period_count: usize) -> usize {
    if requested_bins == 0 {
        return 0;
    }
    requested_bins.min(period_count.max(1)).max(1)
}

pub fn heatmap_axis_bins_for_period_span(
    requested_bins: usize,
    period_min: u16,
    period_max: u16,
) -> usize {
    let period_count = period_max.saturating_sub(period_min) as usize + 1;
    heatmap_axis_bins_for_period_count(requested_bins, period_count)
}

// Better candidates sort first. Equal normalized scores preserve earlier input order.
pub fn compare_score_rank_desc(
    left_score: f64,
    left_tie_index: usize,
    right_score: f64,
    right_tie_index: usize,
) -> std::cmp::Ordering {
    let left_score = normalize_rank_score(left_score);
    let right_score = normalize_rank_score(right_score);
    right_score
        .total_cmp(&left_score)
        .then_with(|| left_tie_index.cmp(&right_tie_index))
}

pub trait BacktestEngine {
    fn eval_batch(&self, combos: &[DoubleMaParams]) -> Result<Vec<Metrics>, String>;
}

pub struct CpuEngine<'a> {
    pub candles: &'a Candles,
    pub fast_ma_types: &'a [String],
    pub slow_ma_types: &'a [String],
    pub ma_source: &'a str,
}

impl<'a> BacktestEngine for CpuEngine<'a> {
    fn eval_batch(&self, combos: &[DoubleMaParams]) -> Result<Vec<Metrics>, String> {
        ta_strategies::double_ma::double_ma_batch_cpu(
            self.candles,
            combos,
            self.fast_ma_types,
            self.slow_ma_types,
            self.ma_source,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub best_params: DoubleMaParams,
    pub best_metrics: Metrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_score: Option<f64>,
    pub top: Vec<(DoubleMaParams, Metrics)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_scores: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<(DoubleMaParams, Metrics)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_scores: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heatmap: Option<OptimizationHeatmap>,
    pub num_combos: usize,
    pub num_candles: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizationHeatmap {
    pub bins_fast: usize,
    pub bins_slow: usize,
    pub fast_min: u16,
    pub fast_max: u16,
    pub slow_min: u16,
    pub slow_max: u16,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cell_order: Option<String>,
    pub values: Vec<Option<f64>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mean_values: Option<Vec<Option<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sample_counts: Option<Vec<u32>>,
}

#[cfg(test)]
mod tests {
    use super::{
        compare_score_rank_desc, heatmap_axis_bins_for_period_count,
        heatmap_axis_bins_for_period_span, normalize_rank_score, ObjectiveKind,
        OptimizationHeatmap, OptimizationMode, OptimizationResult,
    };
    use serde_json::{json, Value};
    use std::cmp::Ordering;
    use ta_strategies::double_ma::{DoubleMaParams, Metrics};

    fn sample_params() -> DoubleMaParams {
        DoubleMaParams {
            fast_len: 5,
            slow_len: 20,
            fast_ma_id: 1,
            slow_ma_id: 2,
        }
    }

    fn sample_metrics() -> Metrics {
        Metrics {
            pnl: 1.25,
            sharpe: 0.75,
            max_dd: 0.10,
            trades: 4,
            exposure: 0.50,
            net_exposure: 0.10,
            ..Metrics::default()
        }
    }

    #[test]
    fn objective_kind_round_trips_through_serde() {
        let encoded = serde_json::to_value(ObjectiveKind::Sharpe).expect("serialize objective");
        assert_eq!(encoded, Value::String("Sharpe".to_string()));
        let decoded: ObjectiveKind =
            serde_json::from_value(encoded).expect("deserialize objective");
        assert_eq!(decoded, ObjectiveKind::Sharpe);
    }

    #[test]
    fn optimization_mode_round_trips_through_serde() {
        let encoded =
            serde_json::to_value(OptimizationMode::GpuVdps).expect("serialize optimization mode");
        assert_eq!(encoded, Value::String("GpuVdps".to_string()));
        let decoded: OptimizationMode =
            serde_json::from_value(encoded).expect("deserialize optimization mode");
        assert_eq!(decoded, OptimizationMode::GpuVdps);
    }

    #[test]
    fn rank_score_policy_clamps_non_finite_scores_and_preserves_input_ties() {
        assert_eq!(normalize_rank_score(f64::NAN), f64::NEG_INFINITY);
        assert_eq!(normalize_rank_score(f64::INFINITY), f64::NEG_INFINITY);
        assert_eq!(normalize_rank_score(1.25), 1.25);
        assert_eq!(compare_score_rank_desc(2.0, 9, 1.0, 0), Ordering::Less);
        assert_eq!(compare_score_rank_desc(1.0, 0, 1.0, 9), Ordering::Less);
        assert_eq!(
            compare_score_rank_desc(f64::NAN, 0, 0.0, 9),
            Ordering::Greater
        );
    }

    #[test]
    fn heatmap_axis_bins_clamp_to_requested_period_span() {
        assert_eq!(heatmap_axis_bins_for_period_span(128, 5, 20), 16);
        assert_eq!(heatmap_axis_bins_for_period_span(128, 20, 80), 61);
        assert_eq!(heatmap_axis_bins_for_period_span(8, 20, 80), 8);
        assert_eq!(heatmap_axis_bins_for_period_span(0, 20, 80), 0);
        assert_eq!(heatmap_axis_bins_for_period_count(128, 3), 3);
    }

    #[test]
    fn optimization_result_deserializes_with_optional_fields_omitted() {
        let result: OptimizationResult = serde_json::from_value(json!({
            "best_params": {
                "fast_len": 5,
                "slow_len": 20,
                "fast_ma_id": 1,
                "slow_ma_id": 2
            },
            "best_metrics": {
                "pnl": 1.25,
                "sharpe": 0.75,
                "max_dd": 0.10,
                "trades": 4,
                "exposure": 0.50,
                "net_exposure": 0.10
            },
            "top": [],
            "num_combos": 12,
            "num_candles": 256
        }))
        .expect("deserialize optimization result");

        assert!(result.best_score.is_none());
        assert!(result.top_scores.is_none());
        assert!(result.all.is_none());
        assert!(result.all_scores.is_none());
        assert!(result.heatmap.is_none());
        assert_eq!(result.best_params.fast_len, 5);
        assert_eq!(result.best_metrics.pnl, 1.25);
    }

    #[test]
    fn optimization_heatmap_deserializes_without_optional_aggregates() {
        let heatmap: OptimizationHeatmap = serde_json::from_value(json!({
            "bins_fast": 2,
            "bins_slow": 3,
            "fast_min": 5,
            "fast_max": 10,
            "slow_min": 20,
            "slow_max": 40,
            "values": [1.0, null, 2.5, null, null, 3.5]
        }))
        .expect("deserialize heatmap");

        assert_eq!(heatmap.values.len(), 6);
        assert!(heatmap.cell_order.is_none());
        assert!(heatmap.mean_values.is_none());
        assert!(heatmap.sample_counts.is_none());
    }

    #[test]
    fn optimization_heatmap_deserializes_with_optional_aggregates() {
        let heatmap: OptimizationHeatmap = serde_json::from_value(json!({
            "bins_fast": 1,
            "bins_slow": 2,
            "fast_min": 3,
            "fast_max": 3,
            "slow_min": 7,
            "slow_max": 8,
            "cell_order": "slow_fast_row_major_v1",
            "values": [0.5, 0.75],
            "mean_values": [0.25, null],
            "sample_counts": [2, 0]
        }))
        .expect("deserialize heatmap with aggregates");

        assert_eq!(
            heatmap.cell_order.as_deref(),
            Some("slow_fast_row_major_v1")
        );
        assert_eq!(heatmap.mean_values, Some(vec![Some(0.25), None]));
        assert_eq!(heatmap.sample_counts, Some(vec![2, 0]));
    }

    #[test]
    fn optimization_result_omits_none_optional_fields_when_serialized() {
        let value = serde_json::to_value(OptimizationResult {
            best_params: sample_params(),
            best_metrics: sample_metrics(),
            best_score: None,
            top: vec![(sample_params(), sample_metrics())],
            top_scores: None,
            all: None,
            all_scores: None,
            heatmap: None,
            num_combos: 12,
            num_candles: 256,
        })
        .expect("serialize optimization result");

        let object = value.as_object().expect("optimization result object");
        assert!(!object.contains_key("best_score"));
        assert!(!object.contains_key("top_scores"));
        assert!(!object.contains_key("all"));
        assert!(!object.contains_key("all_scores"));
        assert!(!object.contains_key("heatmap"));
    }

    #[test]
    fn optimization_heatmap_omits_none_optional_aggregates_when_serialized() {
        let value = serde_json::to_value(OptimizationHeatmap {
            bins_fast: 2,
            bins_slow: 3,
            fast_min: 5,
            fast_max: 10,
            slow_min: 20,
            slow_max: 40,
            cell_order: None,
            values: vec![Some(1.0), None, Some(2.5), None, None, Some(3.5)],
            mean_values: None,
            sample_counts: None,
        })
        .expect("serialize heatmap");

        let object = value.as_object().expect("heatmap object");
        assert!(!object.contains_key("mean_values"));
        assert!(!object.contains_key("sample_counts"));
    }
}
