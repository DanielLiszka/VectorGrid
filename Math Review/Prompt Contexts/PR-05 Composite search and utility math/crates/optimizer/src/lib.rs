pub mod grid;
pub mod mo;
pub mod stream;

use serde::{Deserialize, Serialize};
use ta_strategies::double_ma::{DoubleMaParams, Metrics};
use vector_ta::utilities::data_loader::Candles;

/// Objective vocabulary shared by optimizer request and result surfaces.
///
/// This enum names the metric being optimized. Maximize-vs-minimize policy is
/// defined by the consuming search implementation rather than by this enum alone.
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

/// Optimization request mode vocabulary shared by optimizer callers.
///
/// Not every downstream workflow necessarily honors every variant, so this enum
/// should be treated as shared request vocabulary rather than a universal
/// execution guarantee.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptimizationMode {
    Grid,
    CoarseToFine,
    /// VRAM-aware deterministic progressive search (GPU-oriented).
    /// Uses staged pruning/refinement for very large spaces.
    GpuVdps,
    Auto,
}

/// Batch evaluation contract used by grid and streaming search helpers.
///
/// Implementations are expected to return exactly one `Metrics` row per input
/// combo, in the same order as the provided slice.
pub trait BacktestEngine {
    fn eval_batch(&self, combos: &[DoubleMaParams]) -> Vec<Metrics>;
}

/// Thin adapter over `ta_strategies::double_ma::double_ma_batch_cpu`.
pub struct CpuEngine<'a> {
    pub candles: &'a Candles,
    pub fast_ma_types: &'a [String],
    pub slow_ma_types: &'a [String],
    pub ma_source: &'a str,
}

impl<'a> BacktestEngine for CpuEngine<'a> {
    fn eval_batch(&self, combos: &[DoubleMaParams]) -> Vec<Metrics> {
        ta_strategies::double_ma::double_ma_batch_cpu(
            self.candles,
            combos,
            self.fast_ma_types,
            self.slow_ma_types,
            self.ma_source,
        )
    }
}

/// Shared optimizer result DTO.
///
/// `top_scores` pairs with `top`, `all_scores` pairs with `all`, and omitted
/// optional fields are part of the serde-visible contract.
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

/// Flattened heatmap DTO with separate dimension metadata.
///
/// `values` stores the primary flattened grid, while `mean_values` and
/// `sample_counts` are optional compatibility-bearing aggregate arrays.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizationHeatmap {
    pub bins_fast: usize,
    pub bins_slow: usize,
    pub fast_min: u16,
    pub fast_max: u16,
    pub slow_min: u16,
    pub slow_max: u16,
    pub values: Vec<Option<f64>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mean_values: Option<Vec<Option<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sample_counts: Option<Vec<u32>>,
}

#[cfg(test)]
mod tests {
    use super::{ObjectiveKind, OptimizationHeatmap, OptimizationMode, OptimizationResult};
    use serde_json::{json, Value};
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
            "values": [0.5, 0.75],
            "mean_values": [0.25, null],
            "sample_counts": [2, 0]
        }))
        .expect("deserialize heatmap with aggregates");

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
