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
    /// VRAM-aware deterministic progressive search (GPU-oriented).
    /// Uses staged pruning/refinement for very large spaces.
    GpuVdps,
    Auto,
}

pub trait BacktestEngine {
    fn eval_batch(&self, combos: &[DoubleMaParams]) -> Vec<Metrics>;
}

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
    pub values: Vec<Option<f64>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mean_values: Option<Vec<Option<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sample_counts: Option<Vec<u32>>,
}
