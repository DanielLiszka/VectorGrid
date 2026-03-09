use crate::asr::{
    build_asr_desired_positions, compute_asr_fast_signals, compute_asr_kaufman_stops,
    compute_asr_slow_signals, compute_asr_start_t, eval_asr_curves_from_desired,
    eval_asr_metrics_window_from_desired, resolve_asr_periods, validate_asr_params, AsrFastSignals,
    AsrKaufmanStops, AsrSlowSignals, AsrStrategyParams,
};
use crate::composite_cpu::{compute_macz_hist, compute_ott_line, MaczGateParams, OttGateParams};
use crate::cscv_pbo::{CscvCandidatePboStats, CscvPboConfig, CscvPboResult, SliceReturnStats};
use crate::double_ma::{
    compute_halftrend_trend, compute_log_returns_f64, first_finite_idx, lookup_ma_info,
    ma_matrix_cpu_f64, resolve_alma_params_for_side, validate_ma_kind, validate_strategy_cfg,
    Backend, HalfTrendRegimeFilter,
};
use crate::iterative_sim::{IterativeMetricsDelta, IterativeSimConfig, ValidationEngine};
use crate::progress::ProgressSink;
use crate::rocb::{
    build_rocb_desired_positions, compute_rocb_breakout_signals, compute_rocb_risk_signals,
    compute_rocb_start_t, resolve_rocb_periods, validate_rocb_params, RocbBreakoutSignals,
    RocbRiskSignals, RocbStrategyParams,
};
use crate::validation_pipeline::ValidationPostCandidate;
use crate::vcc::{
    build_vcc_desired_positions, compute_vcc_fast_signals, compute_vcc_start_t,
    compute_vcc_trend_signals, resolve_vcc_periods, validate_vcc_params, VccFastSignals,
    VccStrategyParams, VccTrendSignals,
};
use crate::vemr::{
    build_vemr_desired_positions, compute_vemr_fast_signals, compute_vemr_risk_signals,
    compute_vemr_start_t, resolve_vemr_periods, validate_vemr_params, VemrFastSignals,
    VemrRiskSignals, VemrStrategyParams,
};

use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use ta_strategies::composite_cpu::CompositeSignalMode;
use ta_strategies::double_ma::{Metrics, StrategyConfig, Trade};

#[cfg(feature = "cuda")]
use crate::dynamic_indicator_adapter::PreparedCudaIndicatorInputs;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompositeParams {
    pub fast_ma_type: String,
    pub slow_ma_type: String,
    pub fast_len: u16,
    pub slow_len: u16,
    pub halftrend: HalfTrendRegimeFilter,
    #[serde(default)]
    pub asr: Option<AsrStrategyParams>,
    #[serde(default)]
    pub vcc: Option<VccStrategyParams>,
    #[serde(default)]
    pub vemr: Option<VemrStrategyParams>,
    #[serde(default)]
    pub rocb: Option<RocbStrategyParams>,
    #[serde(default)]
    pub macz: MaczGateParams,
    #[serde(default)]
    pub ott: OttGateParams,
    #[serde(default)]
    pub signal_mode: CompositeSignalMode,
    #[serde(default)]
    pub strategy: StrategyConfig,
}

fn default_ma_source() -> String {
    "close".to_string()
}

fn default_seed() -> u64 {
    1
}

fn default_max_candidates() -> usize {
    32
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ValidationArtifactContext {
    #[serde(default)]
    pub strategy_kind: String,
    #[serde(default)]
    pub csv_path: Option<String>,
    #[serde(default)]
    pub timeframe: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WalkForwardConfig {
    pub window_bars: usize,
    pub step_bars: usize,
}

fn default_walk_forward_window_bars() -> usize {
    20_000
}

fn default_walk_forward_step_bars() -> usize {
    20_000
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WalkForwardConfigOptional {
    #[serde(default = "default_walk_forward_window_bars")]
    pub window_bars: usize,
    #[serde(default = "default_walk_forward_step_bars")]
    pub step_bars: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct WalkForwardStats {
    pub windows: usize,
    pub sharpe_median: f64,
    pub sharpe_worst: f64,
    pub max_dd_median: f64,
    pub max_dd_worst: f64,
    pub pnl_median: f64,
    pub pnl_worst: f64,
    pub trades_median: u32,
    pub trades_worst: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PerturbConfig {
    #[serde(default)]
    pub iterations: usize,
    #[serde(default)]
    pub pct_min: f64,
    #[serde(default)]
    pub pct_max: f64,
    #[serde(default)]
    pub flip_bool_prob: f64,
}

fn default_mc_sims() -> usize {
    500
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TradeBootstrapConfig {
    #[serde(default = "default_mc_sims")]
    pub simulations: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct TradeBootstrapStats {
    pub simulations: usize,
    pub trade_count: usize,
    pub prob_pnl_positive: f64,
    pub pnl_p05: f64,
    pub pnl_p50: f64,
    pub pnl_p95: f64,
    pub max_dd_p50: f64,
    pub max_dd_p95: f64,
}

fn default_cross_max_datasets() -> usize {
    8
}

fn default_cross_gate_enabled() -> bool {
    true
}

fn default_cross_gate_require_pareto() -> bool {
    true
}

fn default_cross_gate_min_score() -> f64 {
    0.0
}

fn default_cross_gate_min_pnl_positive_frac() -> f64 {
    0.5
}

fn default_cross_gate_min_sharpe_median() -> f64 {
    0.0
}

fn default_cross_gate_min_datasets() -> usize {
    1
}

#[derive(Clone, Debug, Deserialize)]
pub struct CrossTickerDataset {
    pub csv_path: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CrossTickerGateConfig {
    #[serde(default = "default_cross_gate_enabled")]
    pub enabled: bool,
    #[serde(default = "default_cross_gate_require_pareto")]
    pub require_pareto: bool,
    #[serde(default = "default_cross_gate_min_score")]
    pub min_score: f64,
    #[serde(default = "default_cross_gate_min_pnl_positive_frac")]
    pub min_pnl_positive_frac: f64,
    #[serde(default = "default_cross_gate_min_sharpe_median")]
    pub min_sharpe_median: f64,
    #[serde(default)]
    pub max_dd_worst: Option<f64>,
    #[serde(default = "default_cross_gate_min_datasets")]
    pub min_datasets: usize,
}

impl Default for CrossTickerGateConfig {
    fn default() -> Self {
        Self {
            enabled: default_cross_gate_enabled(),
            require_pareto: default_cross_gate_require_pareto(),
            min_score: default_cross_gate_min_score(),
            min_pnl_positive_frac: default_cross_gate_min_pnl_positive_frac(),
            min_sharpe_median: default_cross_gate_min_sharpe_median(),
            max_dd_worst: None,
            min_datasets: default_cross_gate_min_datasets(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CrossTickerConfig {
    #[serde(default)]
    pub datasets: Vec<CrossTickerDataset>,
    #[serde(default = "default_cross_max_datasets")]
    pub max_datasets: usize,
    #[serde(default)]
    pub gate: CrossTickerGateConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositeValidationRequest {
    pub data_id: String,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    pub candidates: Vec<CompositeParams>,
    #[serde(default)]
    pub walk_forward: Option<WalkForwardConfigOptional>,
    #[serde(default)]
    pub perturb: Option<PerturbConfig>,
    #[serde(default)]
    pub trade_bootstrap: Option<TradeBootstrapConfig>,
    #[serde(default)]
    pub cross_ticker: Option<CrossTickerConfig>,
    #[serde(default)]
    pub cscv_pbo: Option<CscvPboConfig>,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default = "default_max_candidates")]
    pub max_candidates: usize,
    #[serde(default)]
    pub validation_engine: ValidationEngine,
    #[serde(default)]
    pub iterative: IterativeSimConfig,
    #[serde(default)]
    pub min_psr: Option<f64>,
    #[serde(default)]
    pub min_dsr: Option<f64>,
    #[serde(default)]
    pub validation_context: Option<ValidationArtifactContext>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PerturbStats {
    pub iterations: usize,
    pub pct_min: f64,
    pub pct_max: f64,
    pub sharpe_mean: f64,
    pub sharpe_worst: f64,
    pub max_dd_mean: f64,
    pub max_dd_worst: f64,
    pub pnl_mean: f64,
    pub pnl_worst: f64,
    pub pnl_positive_frac: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CrossTickerDatasetMeta {
    pub label: String,
    pub csv_path: String,
    pub bars: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CrossTickerValidationSummary {
    pub datasets: Vec<CrossTickerDatasetMeta>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CrossTickerStats {
    pub datasets: usize,
    pub sharpe_median: f64,
    pub sharpe_worst: f64,
    pub max_dd_median: f64,
    pub max_dd_worst: f64,
    pub pnl_median: f64,
    pub pnl_worst: f64,
    pub trades_median: u32,
    pub trades_worst: u32,
    pub pnl_positive_frac: f64,
    pub score: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeValidationCandidateResult {
    pub params: CompositeParams,
    pub in_sample: Metrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterative_in_sample: Option<Metrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterative_delta: Option<IterativeMetricsDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psr: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dsr: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walk_forward: Option<WalkForwardStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perturb: Option<PerturbStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_bootstrap: Option<TradeBootstrapStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_ticker: Option<CrossTickerStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_ticker_pareto: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cscv_pbo: Option<CscvCandidatePboStats>,
    pub robust_score: f64,
    pub robust_pareto: bool,
}

impl ValidationPostCandidate for CompositeValidationCandidateResult {
    fn in_sample_metrics(&self) -> &Metrics {
        &self.in_sample
    }

    fn iterative_in_sample_metrics(&self) -> Option<&Metrics> {
        self.iterative_in_sample.as_ref()
    }

    fn walk_forward_sharpe_maxdd(&self) -> Option<(f64, f64)> {
        self.walk_forward
            .as_ref()
            .map(|w| (w.sharpe_median, w.max_dd_worst))
    }

    fn cscv_oos_metric_p50(&self) -> Option<f64> {
        self.cscv_pbo.as_ref().and_then(|c| c.oos_metric_p50)
    }

    fn psr(&self) -> Option<f64> {
        self.psr
    }

    fn dsr(&self) -> Option<f64> {
        self.dsr
    }

    fn set_psr(&mut self, v: Option<f64>) {
        self.psr = v;
    }

    fn set_dsr(&mut self, v: Option<f64>) {
        self.dsr = v;
    }

    fn robust_score(&self) -> f64 {
        self.robust_score
    }

    fn set_robust_score(&mut self, v: f64) {
        self.robust_score = v;
    }

    fn set_robust_pareto(&mut self, v: bool) {
        self.robust_pareto = v;
    }

    fn set_cscv_pbo(&mut self, v: Option<CscvCandidatePboStats>) {
        self.cscv_pbo = v;
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeValidationResult {
    pub data_id: String,
    pub runtime_ms: u64,
    pub candidates: Vec<CompositeValidationCandidateResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_ticker: Option<CrossTickerValidationSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cscv_pbo: Option<CscvPboResult>,
}

fn emit_progress(
    progress: Option<&dyn ProgressSink>,
    processed: usize,
    total: usize,
    phase: &'static str,
) {
    crate::progress::emit_double_ma_progress(progress, processed, total, phase);
}

struct CompositeCandidateSeries<'a> {
    n: usize,
    lr: &'a [f64],
    ott_prices: &'a [f64],
    fast: Vec<f64>,
    slow: Vec<f64>,
    halftrend_trend: Vec<f64>,
    macz_hist: Vec<f64>,
    ott_line: Vec<f64>,
    start_t: usize,
    signal_mode: CompositeSignalMode,
    cfg: StrategyConfig,
}

struct AsrCandidateSeries<'a> {
    candles: &'a vector_ta::utilities::data_loader::Candles,
    n: usize,
    lr: &'a [f64],
    desired: Vec<i32>,
    start_t: usize,
    cfg: StrategyConfig,
}

struct VccCandidateSeries<'a> {
    candles: &'a vector_ta::utilities::data_loader::Candles,
    n: usize,
    lr: &'a [f64],
    desired: Vec<i32>,
    start_t: usize,
    cfg: StrategyConfig,
}

struct VemrCandidateSeries<'a> {
    candles: &'a vector_ta::utilities::data_loader::Candles,
    n: usize,
    lr: &'a [f64],
    desired: Vec<i32>,
    start_t: usize,
    cfg: StrategyConfig,
}

struct RocbCandidateSeries<'a> {
    candles: &'a vector_ta::utilities::data_loader::Candles,
    n: usize,
    lr: &'a [f64],
    desired: Vec<i32>,
    start_t: usize,
    cfg: StrategyConfig,
}

enum CandidateSeries<'a> {
    Composite(CompositeCandidateSeries<'a>),
    Asr(AsrCandidateSeries<'a>),
    Vcc(VccCandidateSeries<'a>),
    Vemr(VemrCandidateSeries<'a>),
    Rocb(RocbCandidateSeries<'a>),
}

fn build_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    ott_prices: &'a [f64],
    base_first_valid: usize,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    params: &CompositeParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    let custom_strategy_count = usize::from(params.asr.is_some())
        + usize::from(params.vcc.is_some())
        + usize::from(params.vemr.is_some())
        + usize::from(params.rocb.is_some());
    if custom_strategy_count > 1 {
        return Err("Select only one of ASR, VCC, VEMR, or ROCB strategy config.".to_string());
    }
    if let Some(asr_cfg) = params.asr.as_ref() {
        return build_asr_candidate_series(
            candles,
            lr,
            ott_prices,
            ma_source,
            params,
            asr_cfg,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs,
        );
    }
    if let Some(vcc_cfg) = params.vcc.as_ref() {
        return build_vcc_candidate_series(
            candles,
            lr,
            ott_prices,
            ma_source,
            params,
            vcc_cfg,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs,
        );
    }
    if let Some(vemr_cfg) = params.vemr.as_ref() {
        return build_vemr_candidate_series(
            candles,
            lr,
            ott_prices,
            ma_source,
            params,
            vemr_cfg,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs,
        );
    }
    if let Some(rocb_cfg) = params.rocb.as_ref() {
        return build_rocb_candidate_series(
            candles,
            lr,
            ott_prices,
            ma_source,
            params,
            rocb_cfg,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs,
        );
    }
    build_composite_candidate_series(
        candles,
        lr,
        ott_prices,
        base_first_valid,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        params,
    )
}

fn build_composite_candidate_series<'a>(
    candles: &vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    ott_prices: &'a [f64],
    base_first_valid: usize,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    params: &CompositeParams,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if lr.len() != n || ott_prices.len() != n {
        return Err("series length mismatch".to_string());
    }

    let fast_ma = params.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = params.slow_ma_type.trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("fast/slow MA type cannot be empty".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_info = lookup_ma_info(&fast_ma).ok_or_else(|| "unknown fast MA".to_string())?;
    if !fast_info.supports_cpu_single {
        return Err(format!("CPU backend does not support MA '{}'", fast_ma));
    }
    let slow_info = lookup_ma_info(&slow_ma).ok_or_else(|| "unknown slow MA".to_string())?;
    if !slow_info.supports_cpu_single {
        return Err(format!("CPU backend does not support MA '{}'", slow_ma));
    }

    let halftrend_trend = compute_halftrend_trend(candles, &params.halftrend)?;
    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }

    if params.macz.use_volume && !candles.fields.volume {
        return Err("MACZ with volume requires a CSV with volume column.".to_string());
    }
    let macz_hist = compute_macz_hist(candles, ma_source, &params.macz)?;
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }

    let ott_line = compute_ott_line(candles, ma_source, &params.ott)?;
    if ott_line.len() != n {
        return Err("OTT series length mismatch".to_string());
    }

    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", &fast_ma, fast_ma_params)?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", &slow_ma, slow_ma_params)?;

    let fast_periods = [params.fast_len];
    let (fast_mat, cols) = ma_matrix_cpu_f64(
        candles,
        &fast_ma,
        ma_source,
        &fast_periods,
        fast_ma_params,
        fast_alma_offset,
        fast_alma_sigma,
    )?;
    if cols != n {
        return Err("fast MA series length mismatch".to_string());
    }

    let slow_periods = [params.slow_len];
    let (slow_mat, cols2) = ma_matrix_cpu_f64(
        candles,
        &slow_ma,
        ma_source,
        &slow_periods,
        slow_ma_params,
        slow_alma_offset,
        slow_alma_sigma,
    )?;
    if cols2 != n {
        return Err("slow MA series length mismatch".to_string());
    }

    let fv_fast =
        first_finite_idx(&fast_mat).ok_or_else(|| "fast MA series all non-finite".to_string())?;
    let fv_slow =
        first_finite_idx(&slow_mat).ok_or_else(|| "slow MA series all non-finite".to_string())?;
    let fv_ht = first_finite_idx(&halftrend_trend)
        .ok_or_else(|| "halftrend trend series all non-finite".to_string())?;
    let fv_macz =
        first_finite_idx(&macz_hist).ok_or_else(|| "macz series all non-finite".to_string())?;
    let fv_ott =
        first_finite_idx(&ott_line).ok_or_else(|| "ott series all non-finite".to_string())?;

    let mut first_valid = base_first_valid;
    first_valid = first_valid.max(fv_fast);
    first_valid = first_valid.max(fv_slow);
    first_valid = first_valid.max(fv_ht);
    first_valid = first_valid.max(fv_macz);
    first_valid = first_valid.max(fv_ott);

    let trade_offset = if params.strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let max_p = (params.fast_len as usize)
        .max(params.slow_len as usize)
        .max(1);
    let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
    let start_t = t_valid.saturating_add(trade_offset);
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    Ok(CandidateSeries::Composite(CompositeCandidateSeries {
        n,
        lr,
        ott_prices,
        fast: fast_mat,
        slow: slow_mat,
        halftrend_trend,
        macz_hist,
        ott_line,
        start_t,
        signal_mode: params.signal_mode,
        cfg: params.strategy.clone(),
    }))
}

fn build_asr_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    ott_prices: &'a [f64],
    ma_source: &str,
    params: &CompositeParams,
    asr_cfg: &AsrStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;
    validate_asr_params(asr_cfg)?;

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if lr.len() != n || ott_prices.len() != n {
        return Err("series length mismatch".to_string());
    }

    let fast_ma = params.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = params.slow_ma_type.trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("fast/slow MA type cannot be empty".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
    if close.len() != n || ma_prices.len() != n {
        return Err("series length mismatch".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    #[cfg(feature = "cuda")]
    let backend = if prepared_cuda_inputs.is_some() {
        Backend::Auto { device_id: 0 }
    } else {
        Backend::CpuOnly
    };
    #[cfg(not(feature = "cuda"))]
    let backend = Backend::CpuOnly;

    let periods = resolve_asr_periods(asr_cfg, params.fast_len, params.slow_len);
    let fast_signals: AsrFastSignals = compute_asr_fast_signals(
        candles,
        ma_source,
        &backend,
        asr_cfg,
        periods.squeeze,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let slow_signals: AsrSlowSignals = compute_asr_slow_signals(
        candles,
        ma_source,
        &backend,
        asr_cfg,
        periods.trend,
        periods.er,
        periods.z,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let kaufman: AsrKaufmanStops = compute_asr_kaufman_stops(candles, &params.strategy)?;
    let max_period = usize::from(
        periods
            .squeeze
            .max(periods.trend)
            .max(periods.er)
            .max(periods.z),
    )
    .max(1);
    let start_t = compute_asr_start_t(
        first_valid_close,
        first_valid_source,
        fast_signals.first_valid,
        slow_signals.first_valid,
        kaufman.first_valid,
        max_period,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    let desired = build_asr_desired_positions(
        close,
        &fast_signals,
        &slow_signals,
        &kaufman,
        start_t,
        &params.strategy,
        asr_cfg,
    );

    Ok(CandidateSeries::Asr(AsrCandidateSeries {
        candles,
        n,
        lr,
        desired,
        start_t,
        cfg: params.strategy.clone(),
    }))
}

fn build_vcc_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    ott_prices: &'a [f64],
    ma_source: &str,
    params: &CompositeParams,
    vcc_cfg: &VccStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;
    validate_vcc_params(vcc_cfg)?;

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if lr.len() != n || ott_prices.len() != n {
        return Err("series length mismatch".to_string());
    }

    let fast_ma = params.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = params.slow_ma_type.trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("fast/slow MA type cannot be empty".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
    if close.len() != n || ma_prices.len() != n {
        return Err("series length mismatch".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    #[cfg(feature = "cuda")]
    let backend = if prepared_cuda_inputs.is_some() {
        Backend::Auto { device_id: 0 }
    } else {
        Backend::CpuOnly
    };
    #[cfg(not(feature = "cuda"))]
    let backend = Backend::CpuOnly;

    let periods = resolve_vcc_periods(vcc_cfg, params.fast_len, params.slow_len);
    let fast_signals: VccFastSignals = compute_vcc_fast_signals(
        candles,
        ma_source,
        &backend,
        vcc_cfg,
        periods.squeeze,
        periods.momentum,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let trend_signals: VccTrendSignals = compute_vcc_trend_signals(
        candles,
        ma_source,
        &backend,
        periods.trend,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let max_period = usize::from(periods.squeeze.max(periods.momentum).max(periods.trend)).max(1);
    let start_t = compute_vcc_start_t(
        first_valid_close,
        first_valid_source,
        fast_signals.first_valid,
        trend_signals.first_valid,
        max_period,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    let desired = build_vcc_desired_positions(
        close,
        &fast_signals,
        &trend_signals,
        start_t,
        &params.strategy,
        vcc_cfg,
    );

    Ok(CandidateSeries::Vcc(VccCandidateSeries {
        candles,
        n,
        lr,
        desired,
        start_t,
        cfg: params.strategy.clone(),
    }))
}

fn build_vemr_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    ott_prices: &'a [f64],
    ma_source: &str,
    params: &CompositeParams,
    vemr_cfg: &VemrStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;
    validate_vemr_params(vemr_cfg)?;

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if lr.len() != n || ott_prices.len() != n {
        return Err("series length mismatch".to_string());
    }

    let fast_ma = params.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = params.slow_ma_type.trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("fast/slow MA type cannot be empty".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
    if close.len() != n || ma_prices.len() != n {
        return Err("series length mismatch".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    #[cfg(feature = "cuda")]
    let backend = if prepared_cuda_inputs.is_some() {
        Backend::Auto { device_id: 0 }
    } else {
        Backend::CpuOnly
    };
    #[cfg(not(feature = "cuda"))]
    let backend = Backend::CpuOnly;

    let periods = resolve_vemr_periods(vemr_cfg, params.fast_len, params.slow_len);
    let fast_signals: VemrFastSignals = compute_vemr_fast_signals(
        candles,
        ma_source,
        &backend,
        vemr_cfg,
        periods.z,
        periods.bb,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let risk_signals: VemrRiskSignals = compute_vemr_risk_signals(
        candles,
        ma_source,
        &backend,
        periods.atr,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let max_period = usize::from(periods.z.max(periods.bb).max(periods.atr)).max(1);
    let start_t = compute_vemr_start_t(
        first_valid_close,
        first_valid_source,
        fast_signals.first_valid,
        risk_signals.first_valid,
        max_period,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    let desired = build_vemr_desired_positions(
        close,
        &fast_signals,
        &risk_signals,
        start_t,
        &params.strategy,
        vemr_cfg,
    );

    Ok(CandidateSeries::Vemr(VemrCandidateSeries {
        candles,
        n,
        lr,
        desired,
        start_t,
        cfg: params.strategy.clone(),
    }))
}

fn build_rocb_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    ott_prices: &'a [f64],
    ma_source: &str,
    params: &CompositeParams,
    rocb_cfg: &RocbStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;
    validate_rocb_params(rocb_cfg)?;

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if lr.len() != n || ott_prices.len() != n {
        return Err("series length mismatch".to_string());
    }

    let fast_ma = params.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = params.slow_ma_type.trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("fast/slow MA type cannot be empty".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let high = vector_ta::utilities::data_loader::source_type(candles, "high");
    let low = vector_ta::utilities::data_loader::source_type(candles, "low");
    if close.len() != n || high.len() != n || low.len() != n {
        return Err("series length mismatch".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_high =
        first_finite_idx(high).ok_or_else(|| "high series all non-finite".to_string())?;
    let first_valid_low =
        first_finite_idx(low).ok_or_else(|| "low series all non-finite".to_string())?;

    #[cfg(feature = "cuda")]
    let backend = if prepared_cuda_inputs.is_some() {
        Backend::Auto { device_id: 0 }
    } else {
        Backend::CpuOnly
    };
    #[cfg(not(feature = "cuda"))]
    let backend = Backend::CpuOnly;

    let periods = resolve_rocb_periods(rocb_cfg, params.fast_len, params.slow_len);
    let breakout: RocbBreakoutSignals = compute_rocb_breakout_signals(
        candles,
        ma_source,
        &backend,
        periods.channel,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let risk: RocbRiskSignals = compute_rocb_risk_signals(
        candles,
        ma_source,
        &backend,
        periods.atr,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let start_t = compute_rocb_start_t(
        first_valid_close,
        first_valid_high,
        first_valid_low,
        breakout.first_valid,
        risk.first_valid,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    let desired =
        build_rocb_desired_positions(close, &breakout, &risk, start_t, &params.strategy, rocb_cfg);

    Ok(CandidateSeries::Rocb(RocbCandidateSeries {
        candles,
        n,
        lr,
        desired,
        start_t,
        cfg: params.strategy.clone(),
    }))
}

fn eval_metrics_window(series: &CandidateSeries<'_>, start_t: usize, end_t: usize) -> Metrics {
    match series {
        CandidateSeries::Composite(series) => {
            let n = series.n;
            let end_t = end_t.min(n);
            if end_t < 2 || start_t >= end_t {
                return Metrics::default();
            }
            let lr = &series.lr[..end_t];
            let ott_prices = &series.ott_prices[..end_t];
            let fast = &series.fast[..end_t];
            let slow = &series.slow[..end_t];
            let ht = &series.halftrend_trend[..end_t];
            let macz = &series.macz_hist[..end_t];
            let ott_line = &series.ott_line[..end_t];

            ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                lr,
                ott_prices,
                fast,
                slow,
                ht,
                macz,
                ott_line,
                start_t,
                &series.cfg,
                series.signal_mode,
            )
        }
        CandidateSeries::Asr(series) => eval_asr_metrics_window_from_desired(
            series.candles,
            series.lr,
            &series.desired,
            start_t,
            end_t.min(series.n),
            &series.cfg,
        ),
        CandidateSeries::Vcc(series) => eval_asr_metrics_window_from_desired(
            series.candles,
            series.lr,
            &series.desired,
            start_t,
            end_t.min(series.n),
            &series.cfg,
        ),
        CandidateSeries::Vemr(series) => eval_asr_metrics_window_from_desired(
            series.candles,
            series.lr,
            &series.desired,
            start_t,
            end_t.min(series.n),
            &series.cfg,
        ),
        CandidateSeries::Rocb(series) => eval_asr_metrics_window_from_desired(
            series.candles,
            series.lr,
            &series.desired,
            start_t,
            end_t.min(series.n),
            &series.cfg,
        ),
    }
}

fn trades_for_series(series: &CandidateSeries<'_>) -> Vec<Trade> {
    match series {
        CandidateSeries::Composite(series) => {
            let curves = ta_strategies::composite_cpu::eval_composite_curves_from_log_returns_and_mas_f64_with_mode(
                series.lr,
                series.ott_prices,
                &series.fast,
                &series.slow,
                &series.halftrend_trend,
                &series.macz_hist,
                &series.ott_line,
                series.start_t,
                &series.cfg,
                1,
                series.signal_mode,
            );
            curves.trades
        }
        CandidateSeries::Asr(series) => {
            eval_asr_curves_from_desired(
                series.candles,
                series.lr,
                &series.desired,
                series.start_t,
                &series.cfg,
                1,
            )
            .trades
        }
        CandidateSeries::Vcc(series) => {
            eval_asr_curves_from_desired(
                series.candles,
                series.lr,
                &series.desired,
                series.start_t,
                &series.cfg,
                1,
            )
            .trades
        }
        CandidateSeries::Vemr(series) => {
            eval_asr_curves_from_desired(
                series.candles,
                series.lr,
                &series.desired,
                series.start_t,
                &series.cfg,
                1,
            )
            .trades
        }
        CandidateSeries::Rocb(series) => {
            eval_asr_curves_from_desired(
                series.candles,
                series.lr,
                &series.desired,
                series.start_t,
                &series.cfg,
                1,
            )
            .trades
        }
    }
}

fn desired_positions_for_series(
    series: &CandidateSeries<'_>,
    start_t: usize,
    end_t: usize,
) -> Vec<i32> {
    match series {
        CandidateSeries::Composite(series) => {
            let n = series.n;
            let end_t = end_t.min(n);
            let mut out: Vec<i32> = vec![0; n];
            if start_t >= end_t || n == 0 {
                return out;
            }

            let trade_offset = if series.cfg.trade_on_next_bar {
                1usize
            } else {
                0usize
            };
            let mut pos: i32 = 0;

            for t in start_t..end_t {
                let t_sig = t.saturating_sub(trade_offset);
                let sign = ta_strategies::composite_cpu::composite_desired_position(
                    series.fast[t_sig],
                    series.slow[t_sig],
                    series.halftrend_trend[t_sig],
                    series.macz_hist[t_sig],
                    series.ott_prices[t_sig],
                    series.ott_line[t_sig],
                    pos,
                    &series.cfg,
                    series.signal_mode,
                );
                out[t] = sign;
                if sign != pos {
                    if pos == 0 || sign == 0 {
                        pos = sign;
                    } else if !series.cfg.allow_flip {
                        pos = 0;
                    } else {
                        pos = sign;
                    }
                }
            }
            out
        }
        CandidateSeries::Asr(series) => {
            let n = series.n;
            let end_t = end_t.min(n);
            let mut out: Vec<i32> = vec![0; n];
            if start_t >= end_t || n == 0 {
                return out;
            }
            for t in start_t..end_t {
                out[t] = *series.desired.get(t).unwrap_or(&0);
            }
            out
        }
        CandidateSeries::Vcc(series) => {
            let n = series.n;
            let end_t = end_t.min(n);
            let mut out: Vec<i32> = vec![0; n];
            if start_t >= end_t || n == 0 {
                return out;
            }
            for t in start_t..end_t {
                out[t] = *series.desired.get(t).unwrap_or(&0);
            }
            out
        }
        CandidateSeries::Vemr(series) => {
            let n = series.n;
            let end_t = end_t.min(n);
            let mut out: Vec<i32> = vec![0; n];
            if start_t >= end_t || n == 0 {
                return out;
            }
            for t in start_t..end_t {
                out[t] = *series.desired.get(t).unwrap_or(&0);
            }
            out
        }
        CandidateSeries::Rocb(series) => {
            let n = series.n;
            let end_t = end_t.min(n);
            let mut out: Vec<i32> = vec![0; n];
            if start_t >= end_t || n == 0 {
                return out;
            }
            for t in start_t..end_t {
                out[t] = *series.desired.get(t).unwrap_or(&0);
            }
            out
        }
    }
}

fn slice_return_stats_for_series(
    series: &CandidateSeries<'_>,
    start_t: usize,
    slice_bars: usize,
    slices: usize,
) -> Vec<SliceReturnStats> {
    let mut out = vec![SliceReturnStats::default(); slices];
    if slices == 0 || slice_bars == 0 {
        return out;
    }

    match series {
        CandidateSeries::Composite(series) => {
            let end_t = start_t
                .saturating_add(slice_bars.saturating_mul(slices))
                .min(series.n);
            if end_t <= start_t + 1 {
                return out;
            }

            let trade_offset = if series.cfg.trade_on_next_bar {
                1usize
            } else {
                0usize
            };
            let mut pos: i32 = 0;

            for t in start_t..end_t {
                let slice_idx = (t - start_t) / slice_bars;
                if slice_idx >= slices {
                    break;
                }
                let t_sig = t.saturating_sub(trade_offset);
                let sign = ta_strategies::composite_cpu::composite_desired_position(
                    series.fast[t_sig],
                    series.slow[t_sig],
                    series.halftrend_trend[t_sig],
                    series.macz_hist[t_sig],
                    series.ott_prices[t_sig],
                    series.ott_line[t_sig],
                    pos,
                    &series.cfg,
                    series.signal_mode,
                );

                if sign != pos {
                    if pos == 0 || sign == 0 {
                        pos = if sign == 0 { 0 } else { sign };
                    } else if !series.cfg.allow_flip {
                        pos = 0;
                    } else {
                        pos = sign;
                    }
                }

                let lr_t = series.lr[t];
                let step_r = if pos == 0 {
                    0.0
                } else {
                    (pos as f64) * lr_t.exp_m1()
                };
                out[slice_idx].sum += step_r;
                out[slice_idx].sumsq += step_r * step_r;
            }
            out
        }
        CandidateSeries::Asr(series) => {
            let end_t = start_t
                .saturating_add(slice_bars.saturating_mul(slices))
                .min(series.n);
            if end_t <= start_t + 1 {
                return out;
            }

            for t in start_t..end_t {
                let slice_idx = (t - start_t) / slice_bars;
                if slice_idx >= slices {
                    break;
                }
                let pos = series.desired.get(t).copied().unwrap_or(0).clamp(-1, 1);
                let lr_t = series.lr[t];
                let step_r = if pos == 0 {
                    0.0
                } else {
                    (pos as f64) * lr_t.exp_m1()
                };
                out[slice_idx].sum += step_r;
                out[slice_idx].sumsq += step_r * step_r;
            }
            out
        }
        CandidateSeries::Vcc(series) => {
            let end_t = start_t
                .saturating_add(slice_bars.saturating_mul(slices))
                .min(series.n);
            if end_t <= start_t + 1 {
                return out;
            }

            for t in start_t..end_t {
                let slice_idx = (t - start_t) / slice_bars;
                if slice_idx >= slices {
                    break;
                }
                let pos = series.desired.get(t).copied().unwrap_or(0).clamp(-1, 1);
                let lr_t = series.lr[t];
                let step_r = if pos == 0 {
                    0.0
                } else {
                    (pos as f64) * lr_t.exp_m1()
                };
                out[slice_idx].sum += step_r;
                out[slice_idx].sumsq += step_r * step_r;
            }
            out
        }
        CandidateSeries::Vemr(series) => {
            let end_t = start_t
                .saturating_add(slice_bars.saturating_mul(slices))
                .min(series.n);
            if end_t <= start_t + 1 {
                return out;
            }

            for t in start_t..end_t {
                let slice_idx = (t - start_t) / slice_bars;
                if slice_idx >= slices {
                    break;
                }
                let pos = series.desired.get(t).copied().unwrap_or(0).clamp(-1, 1);
                let lr_t = series.lr[t];
                let step_r = if pos == 0 {
                    0.0
                } else {
                    (pos as f64) * lr_t.exp_m1()
                };
                out[slice_idx].sum += step_r;
                out[slice_idx].sumsq += step_r * step_r;
            }
            out
        }
        CandidateSeries::Rocb(series) => {
            let end_t = start_t
                .saturating_add(slice_bars.saturating_mul(slices))
                .min(series.n);
            if end_t <= start_t + 1 {
                return out;
            }

            for t in start_t..end_t {
                let slice_idx = (t - start_t) / slice_bars;
                if slice_idx >= slices {
                    break;
                }
                let pos = series.desired.get(t).copied().unwrap_or(0).clamp(-1, 1);
                let lr_t = series.lr[t];
                let step_r = if pos == 0 {
                    0.0
                } else {
                    (pos as f64) * lr_t.exp_m1()
                };
                out[slice_idx].sum += step_r;
                out[slice_idx].sumsq += step_r * step_r;
            }
            out
        }
    }
}

fn series_n(series: &CandidateSeries<'_>) -> usize {
    match series {
        CandidateSeries::Composite(series) => series.n,
        CandidateSeries::Asr(series) => series.n,
        CandidateSeries::Vcc(series) => series.n,
        CandidateSeries::Vemr(series) => series.n,
        CandidateSeries::Rocb(series) => series.n,
    }
}

fn series_start_t(series: &CandidateSeries<'_>) -> usize {
    match series {
        CandidateSeries::Composite(series) => series.start_t,
        CandidateSeries::Asr(series) => series.start_t,
        CandidateSeries::Vcc(series) => series.start_t,
        CandidateSeries::Vemr(series) => series.start_t,
        CandidateSeries::Rocb(series) => series.start_t,
    }
}

fn series_lr<'a>(series: &'a CandidateSeries<'a>) -> &'a [f64] {
    match series {
        CandidateSeries::Composite(series) => series.lr,
        CandidateSeries::Asr(series) => series.lr,
        CandidateSeries::Vcc(series) => series.lr,
        CandidateSeries::Vemr(series) => series.lr,
        CandidateSeries::Rocb(series) => series.lr,
    }
}

fn series_cfg<'a>(series: &'a CandidateSeries<'a>) -> &'a StrategyConfig {
    match series {
        CandidateSeries::Composite(series) => &series.cfg,
        CandidateSeries::Asr(series) => &series.cfg,
        CandidateSeries::Vcc(series) => &series.cfg,
        CandidateSeries::Vemr(series) => &series.cfg,
        CandidateSeries::Rocb(series) => &series.cfg,
    }
}

fn repair_params(p: &mut CompositeParams) {
    // MA lengths
    if p.fast_len == 0 {
        p.fast_len = 1;
    }
    if p.slow_len == 0 {
        p.slow_len = 1;
    }
    if p.fast_len >= p.slow_len {
        p.slow_len = p.fast_len.saturating_add(1).max(2);
    }

    // HalfTrend
    if p.halftrend.amplitude == 0 {
        p.halftrend.amplitude = 1;
    }
    if p.halftrend.atr_period == 0 {
        p.halftrend.atr_period = 1;
    }
    if !p.halftrend.channel_deviation.is_finite() || p.halftrend.channel_deviation <= 0.0 {
        p.halftrend.channel_deviation = 2.0;
    }

    // MACZ
    if p.macz.fast_length == 0 {
        p.macz.fast_length = 1;
    }
    if p.macz.slow_length == 0 {
        p.macz.slow_length = 2;
    }
    if p.macz.fast_length >= p.macz.slow_length {
        p.macz.slow_length = p.macz.fast_length.saturating_add(1).max(2);
    }
    if p.macz.signal_length == 0 {
        p.macz.signal_length = 1;
    }
    if p.macz.lengthz == 0 {
        p.macz.lengthz = 1;
    }
    if p.macz.length_stdev == 0 {
        p.macz.length_stdev = 1;
    }
    if !p.macz.a.is_finite() {
        p.macz.a = 0.0;
    }
    if !p.macz.b.is_finite() {
        p.macz.b = 0.0;
    }
    p.macz.a = p.macz.a.clamp(-2.0, 2.0);
    p.macz.b = p.macz.b.clamp(-2.0, 2.0);
    if !p.macz.gamma.is_finite() {
        p.macz.gamma = 0.0;
    }
    p.macz.gamma = p.macz.gamma.clamp(0.0, 0.999_999);

    // OTT
    if p.ott.period == 0 {
        p.ott.period = 1;
    }
    if !p.ott.percent.is_finite() || p.ott.percent < 0.0 {
        p.ott.percent = 0.0;
    }
    if p.ott.ma_type.trim().is_empty() {
        p.ott.ma_type = "VAR".to_string();
    }

    if let Some(vemr) = p.vemr.as_mut() {
        if vemr.z_period == 0 {
            vemr.z_period = 20;
        }
        if vemr.bb_period == 0 {
            vemr.bb_period = 20;
        }
        if vemr.atr_period == 0 {
            vemr.atr_period = 14;
        }
        if vemr.z_ma_type.trim().is_empty() {
            vemr.z_ma_type = "sma".to_string();
        }
        if !vemr.z_nbdev.is_finite() || vemr.z_nbdev <= 0.0 {
            vemr.z_nbdev = 1.0;
        }
        if !vemr.z_entry.is_finite() || vemr.z_entry <= 0.0 {
            vemr.z_entry = 2.0;
        }
        if !vemr.z_exit.is_finite() || vemr.z_exit < 0.0 {
            vemr.z_exit = 0.5;
        }
        if vemr.z_exit > vemr.z_entry {
            vemr.z_exit = vemr.z_entry;
        }
        if !vemr.bb_mult.is_finite() || vemr.bb_mult <= 0.0 {
            vemr.bb_mult = 2.0;
        }
        if !vemr.atr_stop_mult.is_finite() || vemr.atr_stop_mult <= 0.0 {
            vemr.atr_stop_mult = 2.0;
        }
        if vemr.max_trade_bars == 0 {
            vemr.max_trade_bars = 80;
        }
    }
    if let Some(vcc) = p.vcc.as_mut() {
        if vcc.squeeze_period == 0 {
            vcc.squeeze_period = 20;
        }
        if vcc.momentum_period == 0 {
            vcc.momentum_period = 20;
        }
        if vcc.momentum_bb_period == 0 {
            vcc.momentum_bb_period = 20;
        }
        if vcc.trend_period == 0 {
            vcc.trend_period = 6;
        }
        if !vcc.squeeze_bb_mult.is_finite() || vcc.squeeze_bb_mult <= 0.0 {
            vcc.squeeze_bb_mult = 2.0;
        }
        if !vcc.squeeze_kc_mult.is_finite() || vcc.squeeze_kc_mult <= 0.0 {
            vcc.squeeze_kc_mult = 1.5;
        }
        if vcc.max_trade_bars == 0 {
            vcc.max_trade_bars = 120;
        }
    }
    if let Some(rocb) = p.rocb.as_mut() {
        if rocb.channel_period == 0 {
            rocb.channel_period = 20;
        }
        if rocb.atr_period == 0 {
            rocb.atr_period = 14;
        }
        if !rocb.atr_stop_mult.is_finite() || rocb.atr_stop_mult <= 0.0 {
            rocb.atr_stop_mult = 2.0;
        }
        if rocb.max_trade_bars == 0 {
            rocb.max_trade_bars = 80;
        }
    }

    // Strategy config
    if !p.strategy.fee_bps.is_finite() || p.strategy.fee_bps < 0.0 {
        p.strategy.fee_bps = 0.0;
    }
    if !p.strategy.spread_bps.is_finite() || p.strategy.spread_bps < 0.0 {
        p.strategy.spread_bps = 0.0;
    }
    if !p.strategy.slippage_bps.is_finite() || p.strategy.slippage_bps < 0.0 {
        p.strategy.slippage_bps = 0.0;
    }
    if !p.strategy.commission.is_finite() {
        p.strategy.commission = p.strategy.commission_from_bps();
    }
    p.strategy.commission = p.strategy.commission.clamp(0.0, 0.999_999);
    if !p.strategy.eps_rel.is_finite() || p.strategy.eps_rel < 0.0 {
        p.strategy.eps_rel = 0.0;
    }
}

fn apply_pct_u16(v: u16, mult: f64) -> u16 {
    let x = (v as f64) * mult;
    if !x.is_finite() {
        return v;
    }
    x.round().clamp(1.0, u16::MAX as f64) as u16
}

fn apply_pct_f64(v: f64, mult: f64) -> f64 {
    let x = v * mult;
    if x.is_finite() {
        x
    } else {
        v
    }
}

fn rand_mult(rng: &mut StdRng, pct_min: f64, pct_max: f64) -> f64 {
    let pct = if pct_max > 0.0 {
        rng.gen_range(pct_min..=pct_max)
    } else {
        0.0
    };
    let sign = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
    1.0 + sign * pct
}

fn perturb_params(
    base: &CompositeParams,
    cfg: &PerturbConfig,
    rng: &mut StdRng,
) -> CompositeParams {
    let mut p = base.clone();

    let pct_min = cfg.pct_min.abs();
    let pct_max = cfg.pct_max.abs().max(pct_min);
    let flip_bool_prob = cfg.flip_bool_prob.clamp(0.0, 1.0);

    // MA lengths
    p.fast_len = apply_pct_u16(p.fast_len, rand_mult(rng, pct_min, pct_max));
    p.slow_len = apply_pct_u16(p.slow_len, rand_mult(rng, pct_min, pct_max));

    // HalfTrend
    p.halftrend.amplitude = apply_pct_u16(p.halftrend.amplitude, rand_mult(rng, pct_min, pct_max));
    p.halftrend.atr_period =
        apply_pct_u16(p.halftrend.atr_period, rand_mult(rng, pct_min, pct_max));
    p.halftrend.channel_deviation = apply_pct_f64(
        p.halftrend.channel_deviation,
        rand_mult(rng, pct_min, pct_max),
    );

    // MACZ
    p.macz.fast_length = apply_pct_u16(p.macz.fast_length, rand_mult(rng, pct_min, pct_max));
    p.macz.slow_length = apply_pct_u16(p.macz.slow_length, rand_mult(rng, pct_min, pct_max));
    p.macz.signal_length = apply_pct_u16(p.macz.signal_length, rand_mult(rng, pct_min, pct_max));
    p.macz.lengthz = apply_pct_u16(p.macz.lengthz, rand_mult(rng, pct_min, pct_max));
    p.macz.length_stdev = apply_pct_u16(p.macz.length_stdev, rand_mult(rng, pct_min, pct_max));
    p.macz.a = apply_pct_f64(p.macz.a, rand_mult(rng, pct_min, pct_max));
    p.macz.b = apply_pct_f64(p.macz.b, rand_mult(rng, pct_min, pct_max));
    p.macz.gamma = apply_pct_f64(p.macz.gamma, rand_mult(rng, pct_min, pct_max));
    if rng.gen_bool(flip_bool_prob) {
        p.macz.use_lag = !p.macz.use_lag;
    }
    if rng.gen_bool(flip_bool_prob) {
        p.macz.use_volume = !p.macz.use_volume;
    }

    // OTT
    p.ott.period = apply_pct_u16(p.ott.period, rand_mult(rng, pct_min, pct_max));
    p.ott.percent = apply_pct_f64(p.ott.percent, rand_mult(rng, pct_min, pct_max));
    if rng.gen_bool(flip_bool_prob) {
        // Keep ma_type stable most of the time; flipping is not well-defined here.
    }

    if let Some(vemr) = p.vemr.as_mut() {
        vemr.z_period = apply_pct_u16(vemr.z_period, rand_mult(rng, pct_min, pct_max));
        vemr.bb_period = apply_pct_u16(vemr.bb_period, rand_mult(rng, pct_min, pct_max));
        vemr.atr_period = apply_pct_u16(vemr.atr_period, rand_mult(rng, pct_min, pct_max));
        vemr.z_nbdev = apply_pct_f64(vemr.z_nbdev, rand_mult(rng, pct_min, pct_max));
        vemr.z_entry = apply_pct_f64(vemr.z_entry, rand_mult(rng, pct_min, pct_max));
        vemr.z_exit = apply_pct_f64(vemr.z_exit, rand_mult(rng, pct_min, pct_max));
        vemr.bb_mult = apply_pct_f64(vemr.bb_mult, rand_mult(rng, pct_min, pct_max));
        vemr.atr_stop_mult = apply_pct_f64(vemr.atr_stop_mult, rand_mult(rng, pct_min, pct_max));
        vemr.confirm_bars =
            apply_pct_u16(vemr.confirm_bars.max(1), rand_mult(rng, pct_min, pct_max))
                .saturating_sub(1);
        vemr.max_trade_bars = apply_pct_u16(vemr.max_trade_bars, rand_mult(rng, pct_min, pct_max));
        if rng.gen_bool(flip_bool_prob) {
            vemr.map_fast_to_z_period = !vemr.map_fast_to_z_period;
        }
        if rng.gen_bool(flip_bool_prob) {
            vemr.map_fast_to_bb_period = !vemr.map_fast_to_bb_period;
        }
        if rng.gen_bool(flip_bool_prob) {
            vemr.map_slow_to_atr_period = !vemr.map_slow_to_atr_period;
        }
        if rng.gen_bool(flip_bool_prob) {
            vemr.require_band_touch = !vemr.require_band_touch;
        }
    }
    if let Some(rocb) = p.rocb.as_mut() {
        rocb.channel_period = apply_pct_u16(rocb.channel_period, rand_mult(rng, pct_min, pct_max));
        rocb.atr_period = apply_pct_u16(rocb.atr_period, rand_mult(rng, pct_min, pct_max));
        rocb.atr_stop_mult = apply_pct_f64(rocb.atr_stop_mult, rand_mult(rng, pct_min, pct_max));
        rocb.max_trade_bars = apply_pct_u16(rocb.max_trade_bars, rand_mult(rng, pct_min, pct_max));
        if rng.gen_bool(flip_bool_prob) {
            rocb.map_fast_to_channel_period = !rocb.map_fast_to_channel_period;
        }
        if rng.gen_bool(flip_bool_prob) {
            rocb.map_slow_to_atr_period = !rocb.map_slow_to_atr_period;
        }
    }
    if let Some(vcc) = p.vcc.as_mut() {
        vcc.squeeze_period = apply_pct_u16(vcc.squeeze_period, rand_mult(rng, pct_min, pct_max));
        vcc.momentum_period = apply_pct_u16(vcc.momentum_period, rand_mult(rng, pct_min, pct_max));
        vcc.momentum_bb_period =
            apply_pct_u16(vcc.momentum_bb_period, rand_mult(rng, pct_min, pct_max));
        vcc.trend_period = apply_pct_u16(vcc.trend_period, rand_mult(rng, pct_min, pct_max));
        vcc.squeeze_bb_mult = apply_pct_f64(vcc.squeeze_bb_mult, rand_mult(rng, pct_min, pct_max));
        vcc.squeeze_kc_mult = apply_pct_f64(vcc.squeeze_kc_mult, rand_mult(rng, pct_min, pct_max));
        vcc.confirm_bars = apply_pct_u16(vcc.confirm_bars.max(1), rand_mult(rng, pct_min, pct_max))
            .saturating_sub(1);
        vcc.max_trade_bars = apply_pct_u16(vcc.max_trade_bars, rand_mult(rng, pct_min, pct_max));
        if rng.gen_bool(flip_bool_prob) {
            vcc.map_fast_to_squeeze_period = !vcc.map_fast_to_squeeze_period;
        }
        if rng.gen_bool(flip_bool_prob) {
            vcc.map_fast_to_momentum_period = !vcc.map_fast_to_momentum_period;
        }
        if rng.gen_bool(flip_bool_prob) {
            vcc.map_slow_to_trend_period = !vcc.map_slow_to_trend_period;
        }
        if rng.gen_bool(flip_bool_prob) {
            vcc.require_momentum_accel = !vcc.require_momentum_accel;
        }
    }

    // Strategy config
    p.strategy.fee_bps = apply_pct_f64(p.strategy.fee_bps, rand_mult(rng, pct_min, pct_max)).abs();
    p.strategy.spread_bps =
        apply_pct_f64(p.strategy.spread_bps, rand_mult(rng, pct_min, pct_max)).abs();
    p.strategy.slippage_bps =
        apply_pct_f64(p.strategy.slippage_bps, rand_mult(rng, pct_min, pct_max)).abs();
    p.strategy.commission = apply_pct_f64(p.strategy.commission, rand_mult(rng, pct_min, pct_max));
    p.strategy.eps_rel = apply_pct_f64(p.strategy.eps_rel, rand_mult(rng, pct_min, pct_max)).abs();
    if rng.gen_bool(flip_bool_prob) {
        p.strategy.long_only = !p.strategy.long_only;
    }
    if rng.gen_bool(flip_bool_prob) {
        p.strategy.allow_flip = !p.strategy.allow_flip;
    }

    repair_params(&mut p);
    p
}

pub fn run_composite_validation_blocking_with_candles(
    mut req: CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<CompositeValidationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    let start = std::time::Instant::now();

    if req.max_candidates == 0 {
        req.max_candidates = default_max_candidates();
    }
    if req.candidates.len() > req.max_candidates {
        req.candidates.truncate(req.max_candidates);
    }
    if req.candidates.is_empty() {
        return Err("no candidates to validate".to_string());
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let base_first_valid = fv_close.max(fv_ma);
    let lr = compute_log_returns_f64(close);

    let total = req.candidates.len();
    emit_progress(progress, 0, total, "composite_validate");

    let seed = req.seed;
    let walk_cfg = req.walk_forward.clone();
    let perturb_cfg = req.perturb.clone();
    let mc_cfg = req.trade_bootstrap.clone();
    let ma_source = req.ma_source.clone();
    let fast_ma_params = req.fast_ma_params.clone();
    let slow_ma_params = req.slow_ma_params.clone();
    let cscv_cfg = req.cscv_pbo.clone();
    let validation_engine = req.validation_engine;
    let iterative_cfg = req.iterative.clone();
    let min_psr = crate::validation_stats::sanitize_probability_gate(req.min_psr);
    let min_dsr = crate::validation_stats::sanitize_probability_gate(req.min_dsr);
    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles(candles, &ma_source).ok();

    let (out, cscv_pbo) = crate::validation_core::run_validation_candidates(
        &req.candidates,
        seed,
        cancel,
        progress,
        "composite_validate",
        "composite_validate_cscv_prep",
        min_psr,
        min_dsr,
        cscv_cfg.as_ref(),
        candles.close.len(),
        |_, p, rng| {
            let series = build_candidate_series(
                candles,
                &lr,
                ma_prices,
                base_first_valid,
                &ma_source,
                fast_ma_params.as_ref(),
                slow_ma_params.as_ref(),
                p,
                #[cfg(feature = "cuda")]
                prepared_cuda_inputs.as_ref(),
            )?;
            let start_t0 = series_start_t(&series);
            let n_total = series_n(&series);
            let in_sample = eval_metrics_window(&series, start_t0, n_total);

            let walk_forward = if let Some(cfg) = walk_cfg.as_ref() {
                Some(crate::validation_pipeline::compute_walk_forward_stats(
                    n_total,
                    start_t0,
                    cfg,
                    |t0, end| eval_metrics_window(&series, t0, end),
                ))
            } else {
                None
            };

            let perturb = if let Some(cfg) = perturb_cfg.as_ref() {
                crate::validation_pipeline::compute_perturb_stats(
                    p,
                    cfg,
                    cancel,
                    rng,
                    perturb_params,
                    |pp| {
                        let s2 = build_candidate_series(
                            candles,
                            &lr,
                            ma_prices,
                            base_first_valid,
                            &ma_source,
                            fast_ma_params.as_ref(),
                            slow_ma_params.as_ref(),
                            pp,
                            #[cfg(feature = "cuda")]
                            prepared_cuda_inputs.as_ref(),
                        )?;
                        Ok(eval_metrics_window(&s2, series_start_t(&s2), series_n(&s2)))
                    },
                )?
            } else {
                None
            };

            let trade_bootstrap = if let Some(cfg) = mc_cfg.as_ref() {
                let sims = cfg.simulations.min(50_000);
                let trades = trades_for_series(&series);
                Some(crate::validation_pipeline::compute_trade_bootstrap_stats(
                    &trades, sims, rng,
                ))
            } else {
                None
            };

            let (iterative_in_sample, iterative_delta) =
                crate::validation_pipeline::maybe_run_iterative_validation(
                    validation_engine,
                    &in_sample,
                    series_lr(&series),
                    crate::iterative_sim::OhlcSlice {
                        open: &candles.open,
                        high: &candles.high,
                        low: &candles.low,
                        close: &candles.close,
                    },
                    start_t0,
                    series_cfg(&series),
                    &iterative_cfg,
                    || desired_positions_for_series(&series, start_t0, n_total),
                );

            let robust_base = iterative_in_sample.as_ref().unwrap_or(&in_sample);
            let robust_score = crate::validation_pipeline::robust_score_from_metrics(
                robust_base,
                walk_forward
                    .as_ref()
                    .map(|w| (w.sharpe_median, w.max_dd_worst)),
            );

            Ok((
                CompositeValidationCandidateResult {
                    params: p.clone(),
                    in_sample,
                    iterative_in_sample,
                    iterative_delta,
                    psr: None,
                    dsr: None,
                    walk_forward,
                    perturb,
                    trade_bootstrap,
                    cross_ticker: None,
                    cross_ticker_pareto: None,
                    cscv_pbo: None,
                    robust_score,
                    robust_pareto: false,
                },
                start_t0,
                n_total.saturating_sub(start_t0),
            ))
        },
        |cand, cscv_start_t, slice_bars, slices| {
            let series = build_candidate_series(
                candles,
                &lr,
                ma_prices,
                base_first_valid,
                &ma_source,
                fast_ma_params.as_ref(),
                slow_ma_params.as_ref(),
                &cand.params,
                #[cfg(feature = "cuda")]
                prepared_cuda_inputs.as_ref(),
            )?;
            Ok(slice_return_stats_for_series(
                &series,
                cscv_start_t,
                slice_bars,
                slices,
            ))
        },
    )?;

    let runtime_ms = start.elapsed().as_millis() as u64;
    Ok(CompositeValidationResult {
        data_id: req.data_id,
        runtime_ms,
        candidates: out,
        cross_ticker: None,
        cscv_pbo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite_cpu::{
        run_composite_cpu_optimization_blocking_with_candles, CompositeCpuRequest,
    };
    use crate::composite_mo::{
        run_composite_mo_optimization_blocking_with_candles, CompositeMoObjectives,
        CompositeMoRequest, CompositeMoSampler,
    };
    use ta_optimizer::mo::{ParamDef, ParamDomain, RandomMoConfig, SearchSpace};
    use ta_optimizer::{ObjectiveKind, OptimizationMode};

    fn make_trending_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64 * 0.1;
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.25);
            vol.push(1_000.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    #[test]
    fn composite_validation_smoke_runs_and_returns_stats() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let base = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base.clone()],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 1000,
                step_bars: 1000,
            }),
            perturb: Some(PerturbConfig {
                iterations: 3,
                pct_min: 0.01,
                pct_max: 0.01,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 50 }),
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 32,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");

        assert_eq!(res.candidates.len(), 1);
        let c = &res.candidates[0];
        assert!(c.in_sample.sharpe.is_finite());
        assert!(c.walk_forward.as_ref().unwrap().windows > 0);
        assert_eq!(c.perturb.as_ref().unwrap().iterations, 3);
        assert_eq!(c.trade_bootstrap.as_ref().unwrap().simulations, 50);
        assert!(c.robust_score.is_finite());
    }

    #[test]
    fn asr_validation_smoke_runs_and_returns_stats() {
        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let candidate = CompositeParams {
            fast_ma_type: "alma".to_string(),
            slow_ma_type: "alma".to_string(),
            fast_len: 12,
            slow_len: 48,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: Some(AsrStrategyParams::default()),
            vcc: None,
            vemr: None,
            rocb: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "asr-test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![candidate],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 600,
                step_bars: 300,
            }),
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 24 }),
            cross_ticker: None,
            cscv_pbo: None,
            seed: 7,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("asr validation run");
        assert_eq!(res.candidates.len(), 1);
        let c = &res.candidates[0];
        assert!(c.in_sample.sharpe.is_finite());
        assert!(c.robust_score.is_finite());
        assert!(c.walk_forward.as_ref().map(|w| w.windows).unwrap_or(0) > 0);
    }

    #[test]
    fn vemr_validation_smoke_runs_and_returns_stats() {
        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let candidate = CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len: 12,
            slow_len: 48,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: Some(VemrStrategyParams::default()),
            rocb: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "vemr-test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![candidate],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 600,
                step_bars: 300,
            }),
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 24 }),
            cross_ticker: None,
            cscv_pbo: None,
            seed: 11,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("vemr validation run");
        assert_eq!(res.candidates.len(), 1);
        let c = &res.candidates[0];
        assert!(c.in_sample.sharpe.is_finite());
        assert!(c.robust_score.is_finite());
        assert!(c.walk_forward.as_ref().map(|w| w.windows).unwrap_or(0) > 0);
    }

    #[test]
    fn vcc_validation_smoke_runs_and_returns_stats() {
        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let candidate = CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len: 12,
            slow_len: 48,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: Some(VccStrategyParams::default()),
            vemr: None,
            rocb: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "vcc-test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![candidate],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 600,
                step_bars: 300,
            }),
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 24 }),
            cross_ticker: None,
            cscv_pbo: None,
            seed: 13,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("vcc validation run");
        assert_eq!(res.candidates.len(), 1);
        let c = &res.candidates[0];
        assert!(c.in_sample.sharpe.is_finite());
        assert!(c.robust_score.is_finite());
        assert!(c.walk_forward.as_ref().map(|w| w.windows).unwrap_or(0) > 0);
    }

    #[test]
    fn fold_cscv_into_robust_score_uses_oos_median_sharpe_as_lower_bound() {
        let st_low = CscvCandidatePboStats {
            is_selected_frac: 0.5,
            oos_rank_frac_p50: Some(0.8),
            oos_metric_p50: Some(0.25),
        };
        assert!(
            (crate::validation_pipeline::fold_cscv_metric(1.0, st_low.oos_metric_p50) - 0.25).abs()
                < 1e-12
        );

        let st_high = CscvCandidatePboStats {
            is_selected_frac: 0.5,
            oos_rank_frac_p50: Some(0.8),
            oos_metric_p50: Some(2.0),
        };
        assert!(
            (crate::validation_pipeline::fold_cscv_metric(1.0, st_high.oos_metric_p50) - 1.0).abs()
                < 1e-12
        );

        let st_none = CscvCandidatePboStats {
            is_selected_frac: 0.0,
            oos_rank_frac_p50: None,
            oos_metric_p50: None,
        };
        assert!(
            (crate::validation_pipeline::fold_cscv_metric(1.0, st_none.oos_metric_p50) - 1.0).abs()
                < 1e-12
        );
        assert!((crate::validation_pipeline::fold_cscv_metric(1.0, None) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn composite_validation_rejects_invalid_candidate_params() {
        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let bad = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 0,
            slow_len: 0,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 0,
                channel_deviation: -1.0,
                atr_period: 0,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            macz: MaczGateParams {
                fast_length: 0,
                slow_length: 0,
                signal_length: 0,
                lengthz: 0,
                length_stdev: 0,
                a: f64::NAN,
                b: f64::INFINITY,
                gamma: 2.0,
                use_lag: false,
                use_volume: false,
            },
            ott: OttGateParams {
                period: 0,
                percent: -10.0,
                ma_type: "".to_string(),
            },
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig {
                long_only: true,
                allow_flip: true,
                trade_on_next_bar: true,
                cost_preset: None,
                fee_bps: f64::NAN,
                spread_bps: f64::NAN,
                slippage_bps: f64::NAN,
                stop_loss_pct: None,
                take_profit_pct: None,
                commission: f64::NAN,
                eps_rel: -1.0,
                ..StrategyConfig::default()
            },
        };

        let ok = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![bad, ok],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 1,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let err = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("invalid candidates must fail validation");
        assert!(!err.is_empty());
    }

    #[test]
    fn composite_validation_trade_bootstrap_quantiles_monotonic() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let base = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 200 }),
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 32,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");

        let c = &res.candidates[0];
        let mc = c.trade_bootstrap.as_ref().expect("mc");
        assert_eq!(mc.simulations, 200);
        assert!((0.0..=1.0).contains(&mc.prob_pnl_positive));
        assert!(mc.pnl_p05 <= mc.pnl_p50);
        assert!(mc.pnl_p50 <= mc.pnl_p95);
        assert!(mc.max_dd_p50 <= mc.max_dd_p95);
    }

    #[test]
    fn composite_validation_cscv_pbo_runs_and_populates_fields() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let base = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let alt = CompositeParams {
            fast_len: 12,
            slow_len: 40,
            ..base.clone()
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base, alt],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 4,
                max_combinations: 1000,
                threshold: 0.0,
            }),
            seed: 1,
            max_candidates: 32,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");

        let cscv = res.cscv_pbo.as_ref().expect("cscv");
        assert_eq!(cscv.combinations, 6);
        assert!(cscv.pbo.is_finite());

        assert_eq!(res.candidates.len(), 2);
        for c in &res.candidates {
            assert!(c.cscv_pbo.is_some());
        }
    }

    #[test]
    fn composite_validation_iterative_engine_emits_delta_fields() {
        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let base = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 11,
            max_candidates: 32,
            validation_engine: ValidationEngine::Iterative,
            iterative: IterativeSimConfig {
                slippage_vol_mult: 0.25,
                ..IterativeSimConfig::default()
            },
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        let c = &res.candidates[0];
        assert!(c.iterative_in_sample.is_some());
        let d = c.iterative_delta.as_ref().expect("iterative delta");
        assert!(d.pnl.is_finite());
        assert!(d.sharpe.is_finite());
        assert!(d.max_dd.is_finite());
    }

    #[test]
    fn composite_validation_matches_optimizer_top_candidates() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let strategy = StrategyConfig::default();
        let halftrend = HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        };
        let macz = MaczGateParams::default();
        let ott = OttGateParams::default();
        let signal_mode = CompositeSignalMode::HardGate;

        let opt_req = CompositeCpuRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "opt-comp".to_string(),
            fast_range: (5, 9, 1),
            slow_range: (18, 28, 2),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: strategy.clone(),
            halftrend: halftrend.clone(),
            macz: macz.clone(),
            ott: ott.clone(),
            signal_mode,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            gpu_prescan: None,
            top_k: Some(6),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let opt =
            run_composite_cpu_optimization_blocking_with_candles(opt_req, &candles, &cancel, None)
                .expect("composite optimization run");
        assert!(!opt.top.is_empty());

        let candidates: Vec<CompositeParams> = opt
            .top
            .iter()
            .take(6)
            .map(|(p, _)| CompositeParams {
                fast_ma_type: p.fast_ma_type.clone(),
                slow_ma_type: p.slow_ma_type.clone(),
                fast_len: p.fast_len,
                slow_len: p.slow_len,
                halftrend: halftrend.clone(),
                asr: None,
                vcc: None,
                vemr: None,
                rocb: None,
                macz: macz.clone(),
                ott: ott.clone(),
                signal_mode,
                strategy: strategy.clone(),
            })
            .collect();

        let mk_req = |validation_engine: ValidationEngine| CompositeValidationRequest {
            data_id: "opt-comp-vs-val".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: candidates.clone(),
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 19,
            max_candidates: 32,
            validation_engine,
            iterative: IterativeSimConfig {
                slippage_vol_mult: 0.0,
                ..IterativeSimConfig::default()
            },
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let vec_res = run_composite_validation_blocking_with_candles(
            mk_req(ValidationEngine::Vectorized),
            &candles,
            &cancel,
            None,
        )
        .expect("vectorized validation run");
        let iter_res = run_composite_validation_blocking_with_candles(
            mk_req(ValidationEngine::Iterative),
            &candles,
            &cancel,
            None,
        )
        .expect("iterative validation run");

        assert_eq!(vec_res.candidates.len(), iter_res.candidates.len());

        for vec_cand in &vec_res.candidates {
            let iter_cand = iter_res
                .candidates
                .iter()
                .find(|c| {
                    c.params.fast_ma_type == vec_cand.params.fast_ma_type
                        && c.params.slow_ma_type == vec_cand.params.slow_ma_type
                        && c.params.fast_len == vec_cand.params.fast_len
                        && c.params.slow_len == vec_cand.params.slow_len
                })
                .expect("matching iterative candidate");

            assert!((iter_cand.in_sample.pnl - vec_cand.in_sample.pnl).abs() < 1e-12);
            assert!((iter_cand.in_sample.sharpe - vec_cand.in_sample.sharpe).abs() < 1e-12);
            assert!((iter_cand.in_sample.max_dd - vec_cand.in_sample.max_dd).abs() < 1e-12);

            let iter_m = iter_cand
                .iterative_in_sample
                .as_ref()
                .expect("iterative metrics");
            let delta = iter_cand.iterative_delta.as_ref().expect("iterative delta");

            assert!(iter_m.pnl.is_finite());
            assert!(iter_m.sharpe.is_finite());
            assert!(iter_m.max_dd.is_finite());
            assert!((iter_m.pnl - iter_cand.in_sample.pnl - delta.pnl).abs() < 1e-9);
            assert!((iter_m.sharpe - iter_cand.in_sample.sharpe - delta.sharpe).abs() < 1e-9);
            assert!((iter_m.max_dd - iter_cand.in_sample.max_dd - delta.max_dd).abs() < 1e-9);
        }
    }

    #[test]
    fn composite_mo_candidates_validate_consistently() {
        let candles = make_trending_candles(2200);
        let cancel = AtomicBool::new(false);

        let mo_req = CompositeMoRequest {
            data_id: "mo-stage-parity".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 9,
                            step: 1,
                            log: false,
                        },
                    },
                    ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 18,
                            high: 28,
                            step: 2,
                            log: false,
                        },
                    },
                    ParamDef {
                        name: "ott_ma_type".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["sma".to_string()],
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig {
                    seed: 41,
                    trials: 48,
                },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let mo_res =
            run_composite_mo_optimization_blocking_with_candles(mo_req, &candles, &cancel, None)
                .expect("composite mo run");
        assert!(!mo_res.pareto.is_empty());

        let candidates: Vec<CompositeParams> = mo_res
            .pareto
            .iter()
            .take(6)
            .map(|pt| CompositeParams {
                fast_ma_type: pt.params.fast_ma_type.clone(),
                slow_ma_type: pt.params.slow_ma_type.clone(),
                fast_len: pt.params.fast_len,
                slow_len: pt.params.slow_len,
                halftrend: pt.params.halftrend.clone(),
                asr: None,
                vcc: None,
                vemr: None,
                rocb: None,
                macz: pt.params.macz.clone(),
                ott: pt.params.ott.clone(),
                signal_mode: pt.params.signal_mode,
                strategy: pt.params.strategy.clone(),
            })
            .collect();
        assert!(!candidates.is_empty());

        let mk_req = |validation_engine: ValidationEngine| CompositeValidationRequest {
            data_id: "mo-stage-parity".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: candidates.clone(),
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 23,
            max_candidates: 32,
            validation_engine,
            iterative: IterativeSimConfig {
                slippage_vol_mult: 0.0,
                ..IterativeSimConfig::default()
            },
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let vec_res = run_composite_validation_blocking_with_candles(
            mk_req(ValidationEngine::Vectorized),
            &candles,
            &cancel,
            None,
        )
        .expect("vectorized validation run");
        let iter_res = run_composite_validation_blocking_with_candles(
            mk_req(ValidationEngine::Iterative),
            &candles,
            &cancel,
            None,
        )
        .expect("iterative validation run");

        assert_eq!(vec_res.candidates.len(), iter_res.candidates.len());
        for vec_cand in &vec_res.candidates {
            let iter_cand = iter_res
                .candidates
                .iter()
                .find(|c| {
                    c.params.fast_ma_type == vec_cand.params.fast_ma_type
                        && c.params.slow_ma_type == vec_cand.params.slow_ma_type
                        && c.params.fast_len == vec_cand.params.fast_len
                        && c.params.slow_len == vec_cand.params.slow_len
                        && c.params.halftrend.amplitude == vec_cand.params.halftrend.amplitude
                        && c.params.halftrend.atr_period == vec_cand.params.halftrend.atr_period
                        && c.params.signal_mode == vec_cand.params.signal_mode
                })
                .expect("matching iterative candidate");

            assert!((iter_cand.in_sample.pnl - vec_cand.in_sample.pnl).abs() < 1e-12);
            assert!((iter_cand.in_sample.sharpe - vec_cand.in_sample.sharpe).abs() < 1e-12);
            assert!((iter_cand.in_sample.max_dd - vec_cand.in_sample.max_dd).abs() < 1e-12);

            let iter_m = iter_cand
                .iterative_in_sample
                .as_ref()
                .expect("iterative metrics");
            let delta = iter_cand.iterative_delta.as_ref().expect("iterative delta");
            assert!((iter_m.pnl - iter_cand.in_sample.pnl - delta.pnl).abs() < 1e-9);
            assert!((iter_m.sharpe - iter_cand.in_sample.sharpe - delta.sharpe).abs() < 1e-9);
            assert!((iter_m.max_dd - iter_cand.in_sample.max_dd - delta.max_dd).abs() < 1e-9);
        }
    }

    #[test]
    fn composite_validation_rerun_equivalence_same_seed() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let base = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 12,
            slow_len: 36,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "det".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base.clone()],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 1024,
                step_bars: 512,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: 0.01,
                pct_max: 0.02,
                flip_bool_prob: 0.01,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 64 }),
            cross_ticker: None,
            cscv_pbo: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let a =
            run_composite_validation_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("first run");
        let b = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("second run");

        assert_eq!(a.candidates.len(), b.candidates.len());
        for (ca, cb) in a.candidates.iter().zip(b.candidates.iter()) {
            assert_eq!(
                serde_json::to_string(&ca.params).expect("params a"),
                serde_json::to_string(&cb.params).expect("params b")
            );
            assert!((ca.robust_score - cb.robust_score).abs() < 1e-12);
            assert!((ca.in_sample.pnl - cb.in_sample.pnl).abs() < 1e-12);
            assert!((ca.psr.unwrap_or(0.0) - cb.psr.unwrap_or(0.0)).abs() < 1e-12);
            assert!((ca.dsr.unwrap_or(0.0) - cb.dsr.unwrap_or(0.0)).abs() < 1e-12);
        }
    }

    #[test]
    fn composite_validation_probability_gates_filter_results() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |fast_len: u16, slow_len: u16| CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len,
            slow_len,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let base_req = CompositeValidationRequest {
            data_id: "gate".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mk(8, 24), mk(12, 36), mk(20, 60)],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            validation_context: None,
        };

        let baseline = run_composite_validation_blocking_with_candles(
            base_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("baseline run");

        let mut gated_req = base_req;
        gated_req.min_psr = Some(0.80);
        gated_req.min_dsr = Some(0.60);
        let gated =
            run_composite_validation_blocking_with_candles(gated_req, &candles, &cancel, None)
                .expect("gated run");

        assert!(gated.candidates.len() <= baseline.candidates.len());
        for cand in &gated.candidates {
            assert!(cand.psr.unwrap_or(0.0) >= 0.80);
            assert!(cand.dsr.unwrap_or(0.0) >= 0.60);
        }
    }
}
