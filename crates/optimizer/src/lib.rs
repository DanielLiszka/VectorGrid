pub mod grid;
pub mod mo;
pub mod stream;

use serde::{Deserialize, Serialize};
use ta_strategies::double_ma::{DoubleMaParams, Metrics};
use vector_ta::utilities::data_loader::Candles;

/// Stable identity for the Sharpe metric emitted by the Double-MA evaluator.
///
/// Search-distribution consumers must compare this identity before using a
/// distribution as a validation peer universe. The broader metric surface
/// (`core17`, `trade22`, or a reduced device row) is recorded separately.
pub const DOUBLE_MA_SHARPE_METRIC_ID: &str = "double_ma_log_return_sharpe_v1";

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
    /// Full-search statistics observed by the evaluator. This is deliberately
    /// absent when only finalists were materialized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_distribution: Option<SearchDistributionSummary>,
    /// Precise reason why `search_distribution` could not be produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_distribution_unavailable_reason: Option<String>,
    pub num_combos: usize,
    pub num_candles: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchDistributionSummary {
    /// Runtime metric surface from which the Sharpe observations were decoded.
    pub metric_surface_kind: String,
    /// Stable Sharpe formula/semantics identity used for comparability checks.
    pub sharpe_metric_id: String,
    /// Candidate evaluation attempts, including duplicate/cache-hit attempts.
    pub attempted_count: usize,
    /// Distinct parameter vectors actually evaluated.
    pub unique_count: usize,
    /// Distinct evaluated vectors that satisfied all search constraints.
    pub feasible_count: usize,
    /// Feasible distinct vectors whose Sharpe value was finite and observed.
    pub finite_sharpe_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finite_sharpe_mean: Option<f64>,
    /// Bessel-corrected sample standard deviation over finite Sharpe values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finite_sharpe_sample_std: Option<f64>,
}

impl SearchDistributionSummary {
    pub fn is_comparable_with(&self, sharpe_metric_id: &str) -> bool {
        !self.metric_surface_kind.trim().is_empty()
            && !self.sharpe_metric_id.trim().is_empty()
            && self.sharpe_metric_id == sharpe_metric_id
            && self.attempted_count >= self.unique_count
            && self.unique_count >= self.feasible_count
            && self.feasible_count >= self.finite_sharpe_count
            && self.finite_sharpe_count > 0
            && self.finite_sharpe_mean.is_some_and(f64::is_finite)
            && self
                .finite_sharpe_sample_std
                .is_some_and(|value| value.is_finite() && value >= 0.0)
    }

    pub fn try_merge(&mut self, other: &Self) -> Result<(), String> {
        if self.metric_surface_kind != other.metric_surface_kind {
            return Err(format!(
                "search distribution metric-surface mismatch ('{}' vs '{}')",
                self.metric_surface_kind, other.metric_surface_kind
            ));
        }
        if self.sharpe_metric_id != other.sharpe_metric_id {
            return Err(format!(
                "search distribution Sharpe identity mismatch ('{}' vs '{}')",
                self.sharpe_metric_id, other.sharpe_metric_id
            ));
        }

        let left_n = self.finite_sharpe_count;
        let right_n = other.finite_sharpe_count;
        let left_mean = self.finite_sharpe_mean.unwrap_or(0.0);
        let right_mean = other.finite_sharpe_mean.unwrap_or(0.0);
        let left_m2 = sample_m2(left_n, self.finite_sharpe_sample_std);
        let right_m2 = sample_m2(right_n, other.finite_sharpe_sample_std);

        self.attempted_count = self.attempted_count.saturating_add(other.attempted_count);
        self.unique_count = self.unique_count.saturating_add(other.unique_count);
        self.feasible_count = self.feasible_count.saturating_add(other.feasible_count);
        self.finite_sharpe_count = left_n.saturating_add(right_n);

        match (left_n, right_n) {
            (0, 0) => {
                self.finite_sharpe_mean = None;
                self.finite_sharpe_sample_std = None;
            }
            (0, _) => {
                self.finite_sharpe_mean = other.finite_sharpe_mean;
                self.finite_sharpe_sample_std = other.finite_sharpe_sample_std;
            }
            (_, 0) => {}
            _ => {
                let total_n = self.finite_sharpe_count;
                let delta = right_mean - left_mean;
                let merged_mean = left_mean + delta * (right_n as f64 / total_n.max(1) as f64);
                let cross =
                    delta * delta * (left_n as f64 * right_n as f64 / total_n.max(1) as f64);
                let merged_m2 = (left_m2 + right_m2 + cross).max(0.0);
                self.finite_sharpe_mean = Some(merged_mean);
                self.finite_sharpe_sample_std = Some(if total_n > 1 {
                    (merged_m2 / (total_n - 1) as f64).sqrt()
                } else {
                    0.0
                });
            }
        }

        debug_assert!(self.finite_sharpe_count <= self.feasible_count);
        debug_assert!(self.feasible_count <= self.unique_count);
        debug_assert!(self.unique_count <= self.attempted_count);
        Ok(())
    }
}

fn sample_m2(count: usize, sample_std: Option<f64>) -> f64 {
    if count <= 1 {
        0.0
    } else {
        let std = sample_std.filter(|value| value.is_finite()).unwrap_or(0.0);
        std * std * (count - 1) as f64
    }
}

#[derive(Clone, Debug)]
pub struct SearchDistributionAccumulator {
    metric_surface_kind: String,
    sharpe_metric_id: String,
    attempted_count: usize,
    unique_count: usize,
    feasible_count: usize,
    finite_sharpe_count: usize,
    finite_sharpe_mean: f64,
    finite_sharpe_m2: f64,
}

impl SearchDistributionAccumulator {
    pub fn new(
        metric_surface_kind: impl Into<String>,
        sharpe_metric_id: impl Into<String>,
    ) -> Self {
        Self {
            metric_surface_kind: metric_surface_kind.into(),
            sharpe_metric_id: sharpe_metric_id.into(),
            attempted_count: 0,
            unique_count: 0,
            feasible_count: 0,
            finite_sharpe_count: 0,
            finite_sharpe_mean: 0.0,
            finite_sharpe_m2: 0.0,
        }
    }

    /// Records one distinct evaluator invocation.
    pub fn record_unique_evaluation(&mut self, metrics: &Metrics, feasible: bool) {
        self.attempted_count = self.attempted_count.saturating_add(1);
        self.unique_count = self.unique_count.saturating_add(1);
        if !feasible {
            return;
        }
        self.feasible_count = self.feasible_count.saturating_add(1);
        self.record_finite_sharpe(metrics.sharpe);
    }

    /// Records a repeated proposal satisfied from a cache or otherwise not
    /// evaluated again.
    pub fn record_duplicate_attempt(&mut self) {
        self.attempted_count = self.attempted_count.saturating_add(1);
    }

    pub fn record_duplicate_attempts(&mut self, count: usize) {
        self.attempted_count = self.attempted_count.saturating_add(count);
    }

    fn record_finite_sharpe(&mut self, sharpe: f64) {
        if !sharpe.is_finite() {
            return;
        }
        self.finite_sharpe_count = self.finite_sharpe_count.saturating_add(1);
        let count = self.finite_sharpe_count as f64;
        let delta = sharpe - self.finite_sharpe_mean;
        self.finite_sharpe_mean += delta / count;
        let delta2 = sharpe - self.finite_sharpe_mean;
        self.finite_sharpe_m2 += delta * delta2;
    }

    pub fn try_merge(&mut self, other: Self) -> Result<(), String> {
        let mut summary = self.clone().finalize();
        summary.try_merge(&other.finalize())?;
        *self = Self::from_summary(&summary);
        Ok(())
    }

    pub fn finalize(self) -> SearchDistributionSummary {
        debug_assert!(self.finite_sharpe_count <= self.feasible_count);
        debug_assert!(self.feasible_count <= self.unique_count);
        debug_assert!(self.unique_count <= self.attempted_count);
        SearchDistributionSummary {
            metric_surface_kind: self.metric_surface_kind,
            sharpe_metric_id: self.sharpe_metric_id,
            attempted_count: self.attempted_count,
            unique_count: self.unique_count,
            feasible_count: self.feasible_count,
            finite_sharpe_count: self.finite_sharpe_count,
            finite_sharpe_mean: (self.finite_sharpe_count > 0).then_some(self.finite_sharpe_mean),
            finite_sharpe_sample_std: (self.finite_sharpe_count > 0).then_some(
                if self.finite_sharpe_count > 1 {
                    (self.finite_sharpe_m2.max(0.0) / (self.finite_sharpe_count - 1) as f64).sqrt()
                } else {
                    0.0
                },
            ),
        }
    }

    fn from_summary(summary: &SearchDistributionSummary) -> Self {
        Self {
            metric_surface_kind: summary.metric_surface_kind.clone(),
            sharpe_metric_id: summary.sharpe_metric_id.clone(),
            attempted_count: summary.attempted_count,
            unique_count: summary.unique_count,
            feasible_count: summary.feasible_count,
            finite_sharpe_count: summary.finite_sharpe_count,
            finite_sharpe_mean: summary.finite_sharpe_mean.unwrap_or(0.0),
            finite_sharpe_m2: sample_m2(
                summary.finite_sharpe_count,
                summary.finite_sharpe_sample_std,
            ),
        }
    }
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
        OptimizationHeatmap, OptimizationMode, OptimizationResult, SearchDistributionAccumulator,
        SearchDistributionSummary, DOUBLE_MA_SHARPE_METRIC_ID,
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
        assert!(result.search_distribution.is_none());
        assert!(result.search_distribution_unavailable_reason.is_none());
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
            search_distribution: None,
            search_distribution_unavailable_reason: None,
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
        assert!(!object.contains_key("search_distribution"));
        assert!(!object.contains_key("search_distribution_unavailable_reason"));
    }

    #[test]
    fn search_distribution_keeps_attempt_unique_feasible_and_finite_counts_distinct() {
        let mut stats = SearchDistributionAccumulator::new("core17", DOUBLE_MA_SHARPE_METRIC_ID);
        stats.record_unique_evaluation(
            &Metrics {
                sharpe: 1.0,
                ..Metrics::default()
            },
            true,
        );
        stats.record_unique_evaluation(
            &Metrics {
                sharpe: 9.0,
                ..Metrics::default()
            },
            false,
        );
        stats.record_unique_evaluation(
            &Metrics {
                sharpe: 3.0,
                ..Metrics::default()
            },
            true,
        );
        stats.record_duplicate_attempt();

        let summary = stats.finalize();
        assert_eq!(summary.attempted_count, 4);
        assert_eq!(summary.unique_count, 3);
        assert_eq!(summary.feasible_count, 2);
        assert_eq!(summary.finite_sharpe_count, 2);
        assert_eq!(summary.finite_sharpe_mean, Some(2.0));
        assert!(
            (summary.finite_sharpe_sample_std.expect("sample std") - 2.0f64.sqrt()).abs()
                <= 1.0e-12
        );
        assert!(summary.is_comparable_with(DOUBLE_MA_SHARPE_METRIC_ID));
        assert!(!summary.is_comparable_with("other_sharpe"));
    }

    #[test]
    fn search_distribution_rejects_invalid_count_order_or_sample_deviation() {
        let valid = SearchDistributionSummary {
            metric_surface_kind: "core17".to_string(),
            sharpe_metric_id: DOUBLE_MA_SHARPE_METRIC_ID.to_string(),
            attempted_count: 4,
            unique_count: 3,
            feasible_count: 2,
            finite_sharpe_count: 2,
            finite_sharpe_mean: Some(0.5),
            finite_sharpe_sample_std: Some(0.25),
        };
        let mut invalid_counts = valid.clone();
        invalid_counts.unique_count = 5;
        let mut invalid_deviation = valid;
        invalid_deviation.finite_sharpe_sample_std = Some(-0.25);

        assert!(!invalid_counts.is_comparable_with(DOUBLE_MA_SHARPE_METRIC_ID));
        assert!(!invalid_deviation.is_comparable_with(DOUBLE_MA_SHARPE_METRIC_ID));
    }

    #[test]
    fn search_distribution_merge_matches_single_pass_moments() {
        let mut left = SearchDistributionAccumulator::new("core17", DOUBLE_MA_SHARPE_METRIC_ID);
        let mut right = SearchDistributionAccumulator::new("core17", DOUBLE_MA_SHARPE_METRIC_ID);
        let mut single = SearchDistributionAccumulator::new("core17", DOUBLE_MA_SHARPE_METRIC_ID);

        for (idx, sharpe) in [0.5, 1.5, 2.5, 4.5].into_iter().enumerate() {
            let metrics = Metrics {
                sharpe,
                ..Metrics::default()
            };
            if idx < 2 {
                left.record_unique_evaluation(&metrics, true);
            } else {
                right.record_unique_evaluation(&metrics, true);
            }
            single.record_unique_evaluation(&metrics, true);
        }
        left.try_merge(right).expect("compatible merge");
        assert_eq!(left.finalize(), single.finalize());
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
