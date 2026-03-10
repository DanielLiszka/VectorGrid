use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use ta_optimizer::stream::StreamAggregator;
use ta_optimizer::{ObjectiveKind, OptimizationMode, OptimizationResult};
use ta_strategies::composite_cpu::CompositeSignalMode;
use ta_strategies::double_ma::{DoubleMaCurves, DoubleMaParams, Metrics, StrategyConfig};

use crate::asr::{
    build_asr_desired_positions, compute_asr_fast_signals, compute_asr_kaufman_stops,
    compute_asr_slow_signals, compute_asr_start_t, eval_asr_curves_from_desired,
    eval_asr_metrics_from_desired, resolve_asr_periods, validate_asr_params, AsrStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::asr::{
    compute_asr_er_sweep_cuda_device, compute_asr_kaufman_stops_cuda_device,
    compute_asr_linearreg_angle_sweep_cuda_device, compute_asr_squeeze_momentum_sweep_cuda_device,
    compute_asr_ttm_squeeze_sweep_cuda_device, compute_asr_zscore_sweep_cuda_device,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::asr_gpu_kernel::{
    eval_asr_metrics_gpu_kernel_device_rows, AsrGpuComboBatch, AsrGpuDeviceSignalMatrices,
};
use crate::composite_mo::CompositeGpuPrescanConfig;
use crate::double_ma::{
    build_indicator_bars_for_chart, compute_halftrend_trend, compute_log_returns_f64,
    cpu_tile_sizes, estimate_pairs_fast_lt_slow, expand_u16_range, first_finite_idx,
    lookup_ma_info, ma_matrix_cpu_f64, resolve_alma_params_for_side,
    run_double_ma_optimization_blocking_with_candles, strategy_cfg_for_optimization,
    validate_ma_kind, validate_strategy_cfg_for_optimization, Backend, BackendOptimizationResult,
    BackendUsed, DoubleMaParamsResolved, DoubleMaRequest, HalfTrendRegimeFilter,
    LocalStabilityPoint, LocalStabilityResult, LocalStabilitySummary, OptimizationModeResolved,
};
use crate::matrix_index::DoubleMaFastLtSlowIndex;
use crate::prescan_predictive::{predictive_prescan_seed_pairs, PredictivePrescanParams};
use crate::progress::ProgressSink;
use crate::request_normalization::{normalize_heatmap_bins, normalize_topk_include_all};
use crate::rocb::{
    build_rocb_desired_positions, compute_rocb_breakout_signals, compute_rocb_risk_signals,
    compute_rocb_start_t, resolve_rocb_periods, validate_rocb_params, RocbStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::rocb::{
    compute_rocb_atr_sweep_cuda_device, compute_rocb_donchian_output_sweep_cuda_device,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::rocb_gpu_kernel::{
    eval_rocb_metrics_gpu_kernel_device_rows, RocbGpuComboBatch, RocbGpuDeviceSignalMatrices,
};
use crate::vcc::{
    build_vcc_desired_positions, compute_vcc_fast_signals, compute_vcc_start_t,
    compute_vcc_trend_signals, resolve_vcc_periods, validate_vcc_params, VccStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcc::{
    compute_vcc_fast_dual_sweep_cuda_device, compute_vcc_ttm_trend_sweep_cuda_device,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcc_gpu_kernel::{
    eval_vcc_metrics_gpu_kernel_device_rows, VccGpuComboBatch, VccGpuDeviceSignalMatrices,
};
use crate::vemr::{
    build_vemr_desired_positions, compute_vemr_fast_signals, compute_vemr_risk_signals,
    compute_vemr_start_t, resolve_vemr_periods, validate_vemr_params, VemrStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vemr::{
    compute_vemr_atr_sweep_cuda_device, compute_vemr_bollinger_lower_sweep_cuda_device,
    compute_vemr_bollinger_middle_sweep_cuda_device,
    compute_vemr_bollinger_upper_sweep_cuda_device, compute_vemr_zscore_sweep_cuda_device,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vemr_gpu_kernel::{
    eval_vemr_metrics_gpu_kernel_device_rows, VemrGpuComboBatch, VemrGpuDeviceSignalMatrices,
    VemrGpuEvalResult,
};
#[cfg(feature = "cuda")]
use crate::PreparedCudaIndicatorInputs;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MaczGateParams {
    #[serde(default = "default_macz_fast_length")]
    pub fast_length: u16,
    #[serde(default = "default_macz_slow_length")]
    pub slow_length: u16,
    #[serde(default = "default_macz_signal_length")]
    pub signal_length: u16,
    #[serde(default = "default_macz_lengthz")]
    pub lengthz: u16,
    #[serde(default = "default_macz_length_stdev")]
    pub length_stdev: u16,
    #[serde(default = "default_macz_a")]
    pub a: f64,
    #[serde(default = "default_macz_b")]
    pub b: f64,
    #[serde(default)]
    pub use_lag: bool,
    #[serde(default = "default_macz_gamma")]
    pub gamma: f64,
    #[serde(default)]
    pub use_volume: bool,
}

fn default_macz_fast_length() -> u16 {
    12
}
fn default_macz_slow_length() -> u16 {
    25
}
fn default_macz_signal_length() -> u16 {
    9
}
fn default_macz_lengthz() -> u16 {
    20
}
fn default_macz_length_stdev() -> u16 {
    25
}
fn default_macz_a() -> f64 {
    1.0
}
fn default_macz_b() -> f64 {
    1.0
}
fn default_macz_gamma() -> f64 {
    0.02
}

impl Default for MaczGateParams {
    fn default() -> Self {
        Self {
            fast_length: default_macz_fast_length(),
            slow_length: default_macz_slow_length(),
            signal_length: default_macz_signal_length(),
            lengthz: default_macz_lengthz(),
            length_stdev: default_macz_length_stdev(),
            a: default_macz_a(),
            b: default_macz_b(),
            use_lag: false,
            gamma: default_macz_gamma(),
            use_volume: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OttGateParams {
    #[serde(default = "default_ott_period")]
    pub period: u16,
    #[serde(default = "default_ott_percent")]
    pub percent: f64,
    #[serde(default = "default_ott_ma_type")]
    pub ma_type: String,
}

fn default_ott_period() -> u16 {
    2
}
fn default_ott_percent() -> f64 {
    1.4
}
fn default_ott_ma_type() -> String {
    "VAR".to_string()
}

impl Default for OttGateParams {
    fn default() -> Self {
        Self {
            period: default_ott_period(),
            percent: default_ott_percent(),
            ma_type: default_ott_ma_type(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositeCpuRequest {
    #[serde(default = "default_composite_backend")]
    pub backend: Backend,
    pub data_id: String,
    pub fast_range: (u32, u32, u32),
    pub slow_range: (u32, u32, u32),
    pub fast_ma_types: Vec<String>,
    pub slow_ma_types: Vec<String>,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    pub halftrend: HalfTrendRegimeFilter,
    #[serde(default)]
    pub macz: MaczGateParams,
    #[serde(default)]
    pub ott: OttGateParams,
    #[serde(default)]
    pub signal_mode: CompositeSignalMode,
    #[serde(default)]
    pub asr: Option<AsrStrategyParams>,
    #[serde(default)]
    pub vcc: Option<VccStrategyParams>,
    #[serde(default)]
    pub vemr: Option<VemrStrategyParams>,
    #[serde(default)]
    pub rocb: Option<RocbStrategyParams>,
    pub objective: ObjectiveKind,
    pub mode: OptimizationMode,
    /// Optional stage-0 prescan (double-MA optimization) to narrow MA-length ranges before running
    /// the composite grid search.
    #[serde(default)]
    pub gpu_prescan: Option<CompositeGpuPrescanConfig>,
    pub top_k: Option<usize>,
    pub include_all: Option<bool>,
    pub heatmap_bins: Option<u16>,
}

fn default_ma_source() -> String {
    "close".to_string()
}

fn default_composite_backend() -> Backend {
    Backend::CpuOnly
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositeCpuDrilldownRequest {
    #[serde(default = "default_composite_backend")]
    pub backend: Backend,
    pub data_id: String,
    pub params: DoubleMaParamsResolved,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    pub halftrend: HalfTrendRegimeFilter,
    #[serde(default)]
    pub macz: MaczGateParams,
    #[serde(default)]
    pub ott: OttGateParams,
    #[serde(default)]
    pub signal_mode: CompositeSignalMode,
    #[serde(default)]
    pub asr: Option<AsrStrategyParams>,
    #[serde(default)]
    pub vcc: Option<VccStrategyParams>,
    #[serde(default)]
    pub vemr: Option<VemrStrategyParams>,
    #[serde(default)]
    pub rocb: Option<RocbStrategyParams>,
    pub bins: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompositeIndicatorBar {
    pub time: i64,
    pub from_index: u32,
    pub to_index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halftrend: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macz_hist: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ott_line: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ott_price: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompositeCpuDrilldownResult {
    #[serde(flatten)]
    pub curves: DoubleMaCurves,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub composite_indicator_bars: Vec<CompositeIndicatorBar>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositeCpuLocalStabilityRequest {
    #[serde(default = "default_composite_backend")]
    pub backend: Backend,
    pub data_id: String,
    pub params: DoubleMaParamsResolved,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    pub halftrend: HalfTrendRegimeFilter,
    #[serde(default)]
    pub macz: MaczGateParams,
    #[serde(default)]
    pub ott: OttGateParams,
    #[serde(default)]
    pub signal_mode: CompositeSignalMode,
    #[serde(default)]
    pub asr: Option<AsrStrategyParams>,
    #[serde(default)]
    pub vcc: Option<VccStrategyParams>,
    #[serde(default)]
    pub vemr: Option<VemrStrategyParams>,
    #[serde(default)]
    pub rocb: Option<RocbStrategyParams>,
    pub objective: ObjectiveKind,
    pub radius: Option<u16>,
    pub step: Option<u16>,
}

fn objective_score(metrics: &Metrics, objective: ObjectiveKind) -> f64 {
    match objective {
        ObjectiveKind::Sharpe => metrics.sharpe,
        ObjectiveKind::Sortino => metrics.sortino,
        ObjectiveKind::Calmar => metrics.calmar,
        ObjectiveKind::Omega => metrics.omega,
        ObjectiveKind::Volatility => -metrics.volatility,
        ObjectiveKind::Pnl => metrics.pnl,
        ObjectiveKind::MaxDrawdown => -metrics.max_dd,
    }
}

fn objective_needs_return_stats(objective: ObjectiveKind) -> bool {
    matches!(
        objective,
        ObjectiveKind::Sharpe
            | ObjectiveKind::Sortino
            | ObjectiveKind::Omega
            | ObjectiveKind::Volatility
    )
}

fn resolve_composite_grid_mode(
    mode: &OptimizationMode,
    strategy_label: &str,
) -> Result<OptimizationModeResolved, String> {
    if matches!(
        mode,
        OptimizationMode::CoarseToFine | OptimizationMode::GpuVdps
    ) {
        return Err(format!(
            "{strategy_label} strategy currently supports mode=Grid only."
        ));
    }
    Ok(OptimizationModeResolved::Grid)
}

fn period_window_slice(periods: &[u16], center: u16, window: u16) -> &[u16] {
    let lo = center.saturating_sub(window);
    let hi = center.saturating_add(window);
    let start = periods.partition_point(|&x| x < lo);
    let end = periods.partition_point(|&x| x <= hi);
    &periods[start..end]
}

#[cfg(feature = "cuda")]
fn asr_backend_label(backend: &Backend) -> &'static str {
    match backend {
        Backend::Auto { .. } => "Auto",
        Backend::CpuOnly => "CpuOnly",
        Backend::GpuOnly { .. } => "GpuOnly",
        Backend::GpuHybridKernel { .. } => "GpuHybridKernel",
        Backend::GpuKernel { .. } => "GpuKernel",
    }
}

fn run_asr_optimization(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let normalized_backend = req.backend.clone().with_preferred_cuda_device()?;
        if !matches!(normalized_backend, Backend::CpuOnly) {
            let backend_label = asr_backend_label(&req.backend).to_string();
            if vector_ta::cuda::cuda_available() {
                let device_id = match &normalized_backend {
                    Backend::Auto { device_id }
                    | Backend::GpuOnly { device_id }
                    | Backend::GpuHybridKernel { device_id }
                    | Backend::GpuKernel { device_id } => *device_id,
                    Backend::CpuOnly => 0,
                };
                match run_asr_optimization_gpu(&req, candles, cancel, progress, device_id) {
                    Ok(mut gpu_res) => {
                        if matches!(req.backend, Backend::Auto { .. }) {
                            gpu_res.backend_route = Some("Auto->GpuKernel".to_string());
                        } else if !matches!(req.backend, Backend::GpuKernel { .. }) {
                            gpu_res.backend_route = Some(format!("{backend_label}->GpuKernel"));
                        }
                        return Ok(gpu_res);
                    }
                    Err(err) => {
                        let mut cpu_res = run_asr_optimization_cpu(req, candles, cancel, progress)?;
                        cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
                        cpu_res.backend_fallback_reason = Some(err);
                        return Ok(cpu_res);
                    }
                }
            }

            let mut cpu_res = run_asr_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason =
                Some("CUDA unavailable; falling back to CPU ASR optimization.".to_string());
            return Ok(cpu_res);
        }
    }

    #[cfg(all(feature = "cuda", not(feature = "cuda-backtest-kernel")))]
    {
        if !matches!(req.backend, Backend::CpuOnly) {
            let backend_label = asr_backend_label(&req.backend).to_string();
            let mut cpu_res = run_asr_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason = Some(
                "ASR GPU kernel backend is disabled (build without `cuda-backtest-kernel`)."
                    .to_string(),
            );
            return Ok(cpu_res);
        }
    }

    run_asr_optimization_cpu(req, candles, cancel, progress)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn run_asr_optimization_gpu(
    req: &CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    device_id: u32,
) -> Result<BackendOptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let asr_cfg = req.asr.clone().unwrap_or_default();
    validate_asr_params(&asr_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "ASR")?;

    let objective = req.objective;
    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);

    if req.fast_ma_types.len() != 1 || req.slow_ma_types.len() != 1 {
        return Err("Select exactly one fast MA and one slow MA.".to_string());
    }
    let fast_ma = req.fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = req.slow_ma_types[0].trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("Fast/slow MA type cannot be empty.".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_periods = expand_u16_range(req.fast_range)?;
    let slow_periods = expand_u16_range(req.slow_range)?;
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty period range".to_string());
    }
    let total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);
    if total_pairs == 0 {
        return Err("no valid parameter combinations (fast must be < slow)".to_string());
    }
    if include_all && total_pairs > 2_000_000 {
        return Err(format!(
            "include_all=true would return {} rows; disable it or reduce ranges.",
            total_pairs
        ));
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let ma_source = req.ma_source.clone();
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &ma_source);
    if ma_prices.len() != close.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let kaufman =
        compute_asr_kaufman_stops_cuda_device(&prepared_cuda_inputs, &strategy, device_id)?;
    if kaufman.long.rows != 1
        || kaufman.short.rows != 1
        || kaufman.long.cols != n
        || kaufman.short.cols != n
    {
        return Err("ASR kaufman-stop CUDA device shape mismatch".to_string());
    }

    let mut fast_keys: Vec<u16> = fast_periods
        .iter()
        .map(|&fast_len| resolve_asr_periods(&asr_cfg, fast_len, 1).squeeze)
        .collect();
    fast_keys.sort_unstable();
    fast_keys.dedup();
    let fast_rows = fast_keys.len();
    if fast_rows == 0 {
        return Err("ASR fast period set is empty".to_string());
    }
    let fast_min = *fast_keys.first().unwrap_or(&1);
    let fast_max = *fast_keys.last().unwrap_or(&fast_min);

    let fast_squeeze_sweep = compute_asr_ttm_squeeze_sweep_cuda_device(
        &prepared_cuda_inputs,
        &asr_cfg,
        fast_min,
        fast_max,
        1,
        device_id,
    )?;
    let fast_momentum_sweep = compute_asr_squeeze_momentum_sweep_cuda_device(
        &prepared_cuda_inputs,
        &asr_cfg,
        fast_min,
        fast_max,
        1,
        device_id,
    )?;
    if fast_squeeze_sweep.matrix.cols != n || fast_momentum_sweep.matrix.cols != n {
        return Err("ASR fast sweep column mismatch".to_string());
    }

    let mut fast_squeeze_rows: Vec<u64> = Vec::with_capacity(fast_rows);
    let mut fast_momentum_rows: Vec<u64> = Vec::with_capacity(fast_rows);
    let mut fast_row_lookup: HashMap<u16, i32> = HashMap::with_capacity(fast_rows);
    let mut fast_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(fast_rows);
    for (row, key) in fast_keys.iter().copied().enumerate() {
        let sq_ptr = fast_squeeze_sweep
            .row_ptr_for_period(key)
            .ok_or_else(|| format!("ASR fast squeeze row missing for period={key}"))?;
        let mo_ptr = fast_momentum_sweep
            .row_ptr_for_period(key)
            .ok_or_else(|| format!("ASR fast momentum row missing for period={key}"))?;
        fast_squeeze_rows.push(sq_ptr);
        fast_momentum_rows.push(mo_ptr);
        fast_row_lookup.insert(key, row as i32);
        fast_first_valid_lookup.insert(key, fast_squeeze_sweep.first_valid_for_period(key));
    }

    let mut slow_keys: Vec<(u16, u16, u16)> = slow_periods
        .iter()
        .map(|&slow_len| {
            let p = resolve_asr_periods(&asr_cfg, 1, slow_len);
            (p.trend, p.er, p.z)
        })
        .collect();
    slow_keys.sort_unstable();
    slow_keys.dedup();
    let slow_rows = slow_keys.len();
    if slow_rows == 0 {
        return Err("ASR slow period set is empty".to_string());
    }

    let trend_min = slow_keys.iter().map(|k| k.0).min().unwrap_or(1);
    let trend_max = slow_keys.iter().map(|k| k.0).max().unwrap_or(trend_min);
    let er_min = slow_keys.iter().map(|k| k.1).min().unwrap_or(1);
    let er_max = slow_keys.iter().map(|k| k.1).max().unwrap_or(er_min);
    let z_min = slow_keys.iter().map(|k| k.2).min().unwrap_or(1);
    let z_max = slow_keys.iter().map(|k| k.2).max().unwrap_or(z_min);

    let slow_angle_sweep = compute_asr_linearreg_angle_sweep_cuda_device(
        &prepared_cuda_inputs,
        trend_min,
        trend_max,
        1,
        device_id,
    )?;
    let slow_er_sweep =
        compute_asr_er_sweep_cuda_device(&prepared_cuda_inputs, er_min, er_max, 1, device_id)?;
    let slow_z_sweep = compute_asr_zscore_sweep_cuda_device(
        &prepared_cuda_inputs,
        &asr_cfg,
        z_min,
        z_max,
        1,
        device_id,
    )?;
    if slow_angle_sweep.matrix.cols != n
        || slow_er_sweep.matrix.cols != n
        || slow_z_sweep.matrix.cols != n
    {
        return Err("ASR slow sweep column mismatch".to_string());
    }

    let mut slow_angle_rows: Vec<u64> = Vec::with_capacity(slow_rows);
    let mut slow_er_rows: Vec<u64> = Vec::with_capacity(slow_rows);
    let mut slow_z_rows: Vec<u64> = Vec::with_capacity(slow_rows);
    let mut slow_row_lookup: HashMap<(u16, u16, u16), i32> = HashMap::with_capacity(slow_rows);
    let mut slow_first_valid_lookup: HashMap<(u16, u16, u16), usize> =
        HashMap::with_capacity(slow_rows);

    for (row, key) in slow_keys.iter().copied().enumerate() {
        let angle_ptr = slow_angle_sweep
            .row_ptr_for_period(key.0)
            .ok_or_else(|| format!("ASR angle row missing for period={}", key.0))?;
        let er_ptr = slow_er_sweep
            .row_ptr_for_period(key.1)
            .ok_or_else(|| format!("ASR ER row missing for period={}", key.1))?;
        let z_ptr = slow_z_sweep
            .row_ptr_for_period(key.2)
            .ok_or_else(|| format!("ASR zscore row missing for period={}", key.2))?;
        slow_angle_rows.push(angle_ptr);
        slow_er_rows.push(er_ptr);
        slow_z_rows.push(z_ptr);
        slow_row_lookup.insert(key, row as i32);
        let fv = slow_angle_sweep
            .first_valid_for_period(key.0)
            .max(slow_er_sweep.first_valid_for_period(key.1))
            .max(slow_z_sweep.first_valid_for_period(key.2));
        slow_first_valid_lookup.insert(key, fv);
    }

    let mut combo_fast_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_slow_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_params: Vec<DoubleMaParams> = Vec::with_capacity(total_pairs);

    emit_progress(progress, 0, total_pairs, "asr_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_asr_periods(&asr_cfg, fast_len, slow_len);
            let fast_first_valid = *fast_first_valid_lookup
                .get(&periods.squeeze)
                .ok_or_else(|| "ASR fast first_valid lookup miss".to_string())?;
            let slow_first_valid = *slow_first_valid_lookup
                .get(&(periods.trend, periods.er, periods.z))
                .ok_or_else(|| "ASR slow first_valid lookup miss".to_string())?;
            let max_period = usize::from(
                periods
                    .squeeze
                    .max(periods.trend)
                    .max(periods.er)
                    .max(periods.z),
            )
            .max(1);
            let start_t = compute_asr_start_t(
                fv_close,
                fv_ma,
                fast_first_valid,
                slow_first_valid,
                kaufman.first_valid,
                max_period,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            let fast_row = *fast_row_lookup
                .get(&periods.squeeze)
                .ok_or_else(|| "ASR fast row lookup miss".to_string())?;
            let slow_row = *slow_row_lookup
                .get(&(periods.trend, periods.er, periods.z))
                .ok_or_else(|| "ASR slow row lookup miss".to_string())?;
            let start_i32 =
                i32::try_from(start_t).map_err(|_| "ASR start_t exceeds i32 range".to_string())?;

            combo_fast_row_idx.push(fast_row);
            combo_slow_row_idx.push(slow_row);
            combo_start_t.push(start_i32);
            combo_params.push(DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            });

            if (combo_params.len() & 0x03ff) == 0 {
                emit_progress(progress, combo_params.len(), total_pairs, "asr_optimize");
            }
        }
    }
    emit_progress(progress, combo_params.len(), total_pairs, "asr_optimize");

    if combo_params.is_empty() {
        return Err("ASR grid search produced no valid parameter combinations".to_string());
    }

    let signals = AsrGpuDeviceSignalMatrices {
        close_f64: close,
        cols: n,
        fast_rows,
        slow_rows,
        fast_squeeze_rows: &fast_squeeze_rows,
        fast_momentum_rows: &fast_momentum_rows,
        slow_angle_rows: &slow_angle_rows,
        slow_er_rows: &slow_er_rows,
        slow_z_rows: &slow_z_rows,
        kaufman_long_row: kaufman.long.device_ptr,
        kaufman_short_row: kaufman.short.device_ptr,
    };
    let combos = AsrGpuComboBatch {
        fast_row_index: &combo_fast_row_idx,
        slow_row_index: &combo_slow_row_idx,
        start_t: &combo_start_t,
    };

    let start = std::time::Instant::now();
    let metrics =
        eval_asr_metrics_gpu_kernel_device_rows(device_id, &signals, &combos, &strategy, &asr_cfg)?;
    if metrics.len() != combo_params.len() {
        return Err("ASR GPU kernel output length mismatch".to_string());
    }

    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    if heatmap_bins > 0 {
        agg = agg.with_heatmap(
            heatmap_bins,
            heatmap_bins,
            *fast_periods.iter().min().unwrap_or(&0),
            *fast_periods.iter().max().unwrap_or(&0),
            *slow_periods.iter().min().unwrap_or(&0),
            *slow_periods.iter().max().unwrap_or(&0),
        );
    }
    for (i, (params, m)) in combo_params
        .into_iter()
        .zip(metrics.into_iter())
        .enumerate()
    {
        agg.push(params, m);
        if (i & 0x03ff) == 0 {
            emit_progress(progress, i + 1, total_pairs, "asr_optimize");
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "asr_optimize");

    let result = agg
        .finalize()
        .ok_or_else(|| "ASR grid search produced no result".to_string())?;
    let runtime_ms = start.elapsed().as_millis() as u64;

    let resolve_params = |p: &DoubleMaParams| -> DoubleMaParamsResolved {
        DoubleMaParamsResolved {
            fast_len: p.fast_len,
            slow_len: p.slow_len,
            fast_ma_type: fast_ma.clone(),
            slow_ma_type: slow_ma.clone(),
            fast_ma_params: req.fast_ma_params.clone(),
            slow_ma_params: req.slow_ma_params.clone(),
        }
    };
    let best_params = resolve_params(&result.best_params);
    let top: Vec<(DoubleMaParamsResolved, Metrics)> = result
        .top
        .iter()
        .map(|(p, m)| (resolve_params(p), m.clone()))
        .collect();
    let all = result.all.as_ref().map(|rows| {
        rows.iter()
            .map(|(p, m)| (resolve_params(p), m.clone()))
            .collect::<Vec<_>>()
    });
    let heatmap = result.heatmap.clone();

    let (matrix_index, best_col, top_cols) =
        match DoubleMaFastLtSlowIndex::new(fast_periods, slow_periods) {
            Ok(idx) => {
                let best_col = idx.col_id(best_params.fast_len, best_params.slow_len);
                let mut top_cols: Vec<u32> = Vec::with_capacity(top.len());
                for (p, _) in &top {
                    match idx.col_id(p.fast_len, p.slow_len) {
                        Some(c) => top_cols.push(c),
                        None => {
                            top_cols.clear();
                            break;
                        }
                    }
                }
                let top_cols = if top_cols.is_empty() {
                    None
                } else {
                    Some(top_cols)
                };
                (Some(idx), best_col, top_cols)
            }
            Err(_) => (None, None, None),
        };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        mode_used,
        backend_used: BackendUsed::Gpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
    })
}

fn run_asr_optimization_cpu(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let asr_cfg = req.asr.clone().unwrap_or_default();
    validate_asr_params(&asr_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "ASR")?;

    let objective = req.objective;
    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);

    if req.fast_ma_types.len() != 1 || req.slow_ma_types.len() != 1 {
        return Err("Select exactly one fast MA and one slow MA.".to_string());
    }
    let fast_ma = req.fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = req.slow_ma_types[0].trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("Fast/slow MA type cannot be empty.".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_periods = expand_u16_range(req.fast_range)?;
    let slow_periods = expand_u16_range(req.slow_range)?;
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty period range".to_string());
    }
    let total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);
    if total_pairs == 0 {
        return Err("no valid parameter combinations (fast must be < slow)".to_string());
    }
    if include_all && total_pairs > 2_000_000 {
        return Err(format!(
            "include_all=true would return {} rows; disable it or reduce ranges.",
            total_pairs
        ));
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let ma_source = req.ma_source.clone();
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &ma_source);
    if ma_prices.len() != close.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let lr = compute_log_returns_f64(close);
    let kaufman = compute_asr_kaufman_stops(candles, &strategy)?;

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(req.backend, Backend::CpuOnly) {
        None
    } else {
        PreparedCudaIndicatorInputs::from_candles(candles, &ma_source).ok()
    };

    let mut fast_cache: HashMap<u16, crate::asr::AsrFastSignals> = HashMap::new();
    for &fast_len in &fast_periods {
        let squeeze_period = resolve_asr_periods(&asr_cfg, fast_len, 1).squeeze;
        if fast_cache.contains_key(&squeeze_period) {
            continue;
        }
        let fast_signals = compute_asr_fast_signals(
            candles,
            &ma_source,
            &req.backend,
            &asr_cfg,
            squeeze_period,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs.as_ref(),
        )?;
        fast_cache.insert(squeeze_period, fast_signals);
    }

    let mut slow_cache: HashMap<(u16, u16, u16), crate::asr::AsrSlowSignals> = HashMap::new();
    for &slow_len in &slow_periods {
        let periods = resolve_asr_periods(&asr_cfg, 1, slow_len);
        let key = (periods.trend, periods.er, periods.z);
        if slow_cache.contains_key(&key) {
            continue;
        }
        let slow_signals = compute_asr_slow_signals(
            candles,
            &ma_source,
            &req.backend,
            &asr_cfg,
            periods.trend,
            periods.er,
            periods.z,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs.as_ref(),
        )?;
        slow_cache.insert(key, slow_signals);
    }

    let start = std::time::Instant::now();
    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    if heatmap_bins > 0 {
        agg = agg.with_heatmap(
            heatmap_bins,
            heatmap_bins,
            *fast_periods.iter().min().unwrap_or(&0),
            *fast_periods.iter().max().unwrap_or(&0),
            *slow_periods.iter().min().unwrap_or(&0),
            *slow_periods.iter().max().unwrap_or(&0),
        );
    }

    let mut processed = 0usize;
    emit_progress(progress, processed, total_pairs, "asr_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }
            let periods = resolve_asr_periods(&asr_cfg, fast_len, slow_len);
            let fast_signals = fast_cache
                .get(&periods.squeeze)
                .ok_or_else(|| "ASR fast signal cache miss".to_string())?;
            let slow_signals = slow_cache
                .get(&(periods.trend, periods.er, periods.z))
                .ok_or_else(|| "ASR slow signal cache miss".to_string())?;
            let max_period = usize::from(
                periods
                    .squeeze
                    .max(periods.trend)
                    .max(periods.er)
                    .max(periods.z),
            )
            .max(1);
            let start_t = compute_asr_start_t(
                fv_close,
                fv_ma,
                fast_signals.first_valid,
                slow_signals.first_valid,
                kaufman.first_valid,
                max_period,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            let desired = build_asr_desired_positions(
                close,
                fast_signals,
                slow_signals,
                &kaufman,
                start_t,
                &strategy,
                &asr_cfg,
            );
            let metrics = eval_asr_metrics_from_desired(candles, &lr, &desired, start_t, &strategy);
            agg.push(
                DoubleMaParams {
                    fast_len,
                    slow_len,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                metrics,
            );

            processed = processed.saturating_add(1);
            if (processed & 0x03ff) == 0 {
                emit_progress(progress, processed, total_pairs, "asr_optimize");
            }
        }
    }
    emit_progress(progress, processed, total_pairs, "asr_optimize");

    let result = agg
        .finalize()
        .ok_or_else(|| "ASR grid search produced no result".to_string())?;
    let runtime_ms = start.elapsed().as_millis() as u64;

    let resolve_params = |p: &DoubleMaParams| -> DoubleMaParamsResolved {
        DoubleMaParamsResolved {
            fast_len: p.fast_len,
            slow_len: p.slow_len,
            fast_ma_type: fast_ma.clone(),
            slow_ma_type: slow_ma.clone(),
            fast_ma_params: req.fast_ma_params.clone(),
            slow_ma_params: req.slow_ma_params.clone(),
        }
    };
    let best_params = resolve_params(&result.best_params);
    let top: Vec<(DoubleMaParamsResolved, Metrics)> = result
        .top
        .iter()
        .map(|(p, m)| (resolve_params(p), m.clone()))
        .collect();
    let all = result.all.as_ref().map(|rows| {
        rows.iter()
            .map(|(p, m)| (resolve_params(p), m.clone()))
            .collect::<Vec<_>>()
    });
    let heatmap = result.heatmap.clone();

    let (matrix_index, best_col, top_cols) =
        match DoubleMaFastLtSlowIndex::new(fast_periods, slow_periods) {
            Ok(idx) => {
                let best_col = idx.col_id(best_params.fast_len, best_params.slow_len);
                let mut top_cols: Vec<u32> = Vec::with_capacity(top.len());
                for (p, _) in &top {
                    match idx.col_id(p.fast_len, p.slow_len) {
                        Some(c) => top_cols.push(c),
                        None => {
                            top_cols.clear();
                            break;
                        }
                    }
                }
                let top_cols = if top_cols.is_empty() {
                    None
                } else {
                    Some(top_cols)
                };
                (Some(idx), best_col, top_cols)
            }
            Err(_) => (None, None, None),
        };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        mode_used,
        backend_used: BackendUsed::Cpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
    })
}

fn run_vcc_optimization(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let normalized_backend = req.backend.clone().with_preferred_cuda_device()?;
        if !matches!(normalized_backend, Backend::CpuOnly) {
            let backend_label = asr_backend_label(&req.backend).to_string();
            if vector_ta::cuda::cuda_available() {
                let device_id = match &normalized_backend {
                    Backend::Auto { device_id }
                    | Backend::GpuOnly { device_id }
                    | Backend::GpuHybridKernel { device_id }
                    | Backend::GpuKernel { device_id } => *device_id,
                    Backend::CpuOnly => 0,
                };
                match run_vcc_optimization_gpu(&req, candles, cancel, progress, device_id) {
                    Ok(mut gpu_res) => {
                        if matches!(req.backend, Backend::Auto { .. }) {
                            gpu_res.backend_route = Some("Auto->GpuKernel".to_string());
                        } else if !matches!(req.backend, Backend::GpuKernel { .. }) {
                            gpu_res.backend_route = Some(format!("{backend_label}->GpuKernel"));
                        }
                        return Ok(gpu_res);
                    }
                    Err(err) => {
                        let mut cpu_res = run_vcc_optimization_cpu(req, candles, cancel, progress)?;
                        cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
                        cpu_res.backend_fallback_reason = Some(err);
                        return Ok(cpu_res);
                    }
                }
            }

            let mut cpu_res = run_vcc_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason =
                Some("CUDA unavailable; falling back to CPU VCC optimization.".to_string());
            return Ok(cpu_res);
        }
    }

    #[cfg(all(feature = "cuda", not(feature = "cuda-backtest-kernel")))]
    {
        if !matches!(req.backend, Backend::CpuOnly) {
            let backend_label = asr_backend_label(&req.backend).to_string();
            let mut cpu_res = run_vcc_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason = Some(
                "VCC GPU kernel backend is disabled (build without `cuda-backtest-kernel`)."
                    .to_string(),
            );
            return Ok(cpu_res);
        }
    }

    run_vcc_optimization_cpu(req, candles, cancel, progress)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn run_vcc_optimization_gpu(
    req: &CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    device_id: u32,
) -> Result<BackendOptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let vcc_cfg = req.vcc.clone().unwrap_or_default();
    validate_vcc_params(&vcc_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "VCC")?;

    let objective = req.objective;
    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);

    if req.fast_ma_types.len() != 1 || req.slow_ma_types.len() != 1 {
        return Err("Select exactly one fast MA and one slow MA.".to_string());
    }
    let fast_ma = req.fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = req.slow_ma_types[0].trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("Fast/slow MA type cannot be empty.".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_periods = expand_u16_range(req.fast_range)?;
    let slow_periods = expand_u16_range(req.slow_range)?;
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty period range".to_string());
    }
    let total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);
    if total_pairs == 0 {
        return Err("no valid parameter combinations (fast must be < slow)".to_string());
    }
    if include_all && total_pairs > 2_000_000 {
        return Err(format!(
            "include_all=true would return {} rows; disable it or reduce ranges.",
            total_pairs
        ));
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let ma_source = req.ma_source.clone();
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &ma_source);
    if ma_prices.len() != close.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;

    let mut fast_keys: Vec<(u16, u16)> = fast_periods
        .iter()
        .map(|&fast_len| {
            let p = resolve_vcc_periods(&vcc_cfg, fast_len, 1);
            (p.squeeze, p.momentum)
        })
        .collect();
    fast_keys.sort_unstable();
    fast_keys.dedup();
    let fast_rows = fast_keys.len();
    if fast_rows == 0 {
        return Err("VCC fast period set is empty".to_string());
    }

    let squeeze_min = fast_keys.iter().map(|k| k.0).min().unwrap_or(1);
    let squeeze_max = fast_keys.iter().map(|k| k.0).max().unwrap_or(squeeze_min);
    let momentum_min = fast_keys.iter().map(|k| k.1).min().unwrap_or(1);
    let momentum_max = fast_keys.iter().map(|k| k.1).max().unwrap_or(momentum_min);

    let (fast_squeeze_sweep, fast_momentum_sweep) = compute_vcc_fast_dual_sweep_cuda_device(
        &prepared_cuda_inputs,
        &vcc_cfg,
        squeeze_min,
        squeeze_max,
        momentum_min,
        momentum_max,
        1,
        device_id,
    )?;
    if fast_squeeze_sweep.matrix.cols != n || fast_momentum_sweep.matrix.cols != n {
        return Err("VCC fast sweep column mismatch".to_string());
    }

    let mut fast_squeeze_rows: Vec<u64> = Vec::with_capacity(fast_rows);
    let mut fast_momentum_rows: Vec<u64> = Vec::with_capacity(fast_rows);
    let mut fast_row_lookup: HashMap<(u16, u16), i32> = HashMap::with_capacity(fast_rows);
    let mut fast_first_valid_lookup: HashMap<(u16, u16), usize> = HashMap::with_capacity(fast_rows);
    for (row, key) in fast_keys.iter().copied().enumerate() {
        let sq_ptr = fast_squeeze_sweep
            .row_ptr_for_period(key.0)
            .ok_or_else(|| format!("VCC squeeze row missing for period={}", key.0))?;
        let mo_ptr = fast_momentum_sweep
            .row_ptr_for_period(key.1)
            .ok_or_else(|| format!("VCC momentum row missing for period={}", key.1))?;
        fast_squeeze_rows.push(sq_ptr);
        fast_momentum_rows.push(mo_ptr);
        fast_row_lookup.insert(key, row as i32);
        let fv = fast_squeeze_sweep
            .first_valid_for_period(key.0)
            .max(fast_momentum_sweep.first_valid_for_period(key.1));
        fast_first_valid_lookup.insert(key, fv);
    }

    let mut trend_keys: Vec<u16> = slow_periods
        .iter()
        .map(|&slow_len| resolve_vcc_periods(&vcc_cfg, 1, slow_len).trend)
        .collect();
    trend_keys.sort_unstable();
    trend_keys.dedup();
    let trend_rows = trend_keys.len();
    if trend_rows == 0 {
        return Err("VCC trend period set is empty".to_string());
    }

    let trend_min = *trend_keys.first().unwrap_or(&1);
    let trend_max = *trend_keys.last().unwrap_or(&trend_min);
    let trend_sweep = compute_vcc_ttm_trend_sweep_cuda_device(
        &prepared_cuda_inputs,
        trend_min,
        trend_max,
        1,
        device_id,
    )?;
    if trend_sweep.matrix.cols != n {
        return Err("VCC trend sweep column mismatch".to_string());
    }

    let mut trend_rows_ptr: Vec<u64> = Vec::with_capacity(trend_rows);
    let mut trend_row_lookup: HashMap<u16, i32> = HashMap::with_capacity(trend_rows);
    let mut trend_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(trend_rows);
    for (row, key) in trend_keys.iter().copied().enumerate() {
        let tr_ptr = trend_sweep
            .row_ptr_for_period(key)
            .ok_or_else(|| format!("VCC trend row missing for period={key}"))?;
        trend_rows_ptr.push(tr_ptr);
        trend_row_lookup.insert(key, row as i32);
        trend_first_valid_lookup.insert(key, trend_sweep.first_valid_for_period(key));
    }

    let mut combo_fast_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_trend_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_params: Vec<DoubleMaParams> = Vec::with_capacity(total_pairs);

    emit_progress(progress, 0, total_pairs, "vcc_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_vcc_periods(&vcc_cfg, fast_len, slow_len);
            let fast_first_valid = *fast_first_valid_lookup
                .get(&(periods.squeeze, periods.momentum))
                .ok_or_else(|| "VCC fast first_valid lookup miss".to_string())?;
            let trend_first_valid = *trend_first_valid_lookup
                .get(&periods.trend)
                .ok_or_else(|| "VCC trend first_valid lookup miss".to_string())?;
            let max_period =
                usize::from(periods.squeeze.max(periods.momentum).max(periods.trend)).max(1);
            let start_t = compute_vcc_start_t(
                fv_close,
                fv_ma,
                fast_first_valid,
                trend_first_valid,
                max_period,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            let fast_row = *fast_row_lookup
                .get(&(periods.squeeze, periods.momentum))
                .ok_or_else(|| "VCC fast row lookup miss".to_string())?;
            let trend_row = *trend_row_lookup
                .get(&periods.trend)
                .ok_or_else(|| "VCC trend row lookup miss".to_string())?;
            let start_i32 =
                i32::try_from(start_t).map_err(|_| "VCC start_t exceeds i32 range".to_string())?;

            combo_fast_row_idx.push(fast_row);
            combo_trend_row_idx.push(trend_row);
            combo_start_t.push(start_i32);
            combo_params.push(DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            });

            if (combo_params.len() & 0x03ff) == 0 {
                emit_progress(progress, combo_params.len(), total_pairs, "vcc_optimize");
            }
        }
    }
    emit_progress(progress, combo_params.len(), total_pairs, "vcc_optimize");

    if combo_params.is_empty() {
        return Err("VCC grid search produced no valid parameter combinations".to_string());
    }

    let signals = VccGpuDeviceSignalMatrices {
        close_f64: close,
        cols: n,
        fast_rows,
        trend_rows,
        fast_squeeze_rows: &fast_squeeze_rows,
        fast_momentum_rows: &fast_momentum_rows,
        trend_rows_ptr: &trend_rows_ptr,
    };
    let combos = VccGpuComboBatch {
        fast_row_index: &combo_fast_row_idx,
        trend_row_index: &combo_trend_row_idx,
        start_t: &combo_start_t,
    };

    let start = std::time::Instant::now();
    let metrics =
        eval_vcc_metrics_gpu_kernel_device_rows(device_id, &signals, &combos, &strategy, &vcc_cfg)?;
    if metrics.len() != combo_params.len() {
        return Err("VCC GPU kernel output length mismatch".to_string());
    }

    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    if heatmap_bins > 0 {
        agg = agg.with_heatmap(
            heatmap_bins,
            heatmap_bins,
            *fast_periods.iter().min().unwrap_or(&0),
            *fast_periods.iter().max().unwrap_or(&0),
            *slow_periods.iter().min().unwrap_or(&0),
            *slow_periods.iter().max().unwrap_or(&0),
        );
    }
    for (i, (params, m)) in combo_params
        .into_iter()
        .zip(metrics.into_iter())
        .enumerate()
    {
        agg.push(params, m);
        if (i & 0x03ff) == 0 {
            emit_progress(progress, i + 1, total_pairs, "vcc_optimize");
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "vcc_optimize");

    let result = agg
        .finalize()
        .ok_or_else(|| "VCC grid search produced no result".to_string())?;
    let runtime_ms = start.elapsed().as_millis() as u64;

    let resolve_params = |p: &DoubleMaParams| -> DoubleMaParamsResolved {
        DoubleMaParamsResolved {
            fast_len: p.fast_len,
            slow_len: p.slow_len,
            fast_ma_type: fast_ma.clone(),
            slow_ma_type: slow_ma.clone(),
            fast_ma_params: req.fast_ma_params.clone(),
            slow_ma_params: req.slow_ma_params.clone(),
        }
    };
    let best_params = resolve_params(&result.best_params);
    let top: Vec<(DoubleMaParamsResolved, Metrics)> = result
        .top
        .iter()
        .map(|(p, m)| (resolve_params(p), m.clone()))
        .collect();
    let all = result.all.as_ref().map(|rows| {
        rows.iter()
            .map(|(p, m)| (resolve_params(p), m.clone()))
            .collect::<Vec<_>>()
    });
    let heatmap = result.heatmap.clone();

    let (matrix_index, best_col, top_cols) =
        match DoubleMaFastLtSlowIndex::new(fast_periods, slow_periods) {
            Ok(idx) => {
                let best_col = idx.col_id(best_params.fast_len, best_params.slow_len);
                let mut top_cols: Vec<u32> = Vec::with_capacity(top.len());
                for (p, _) in &top {
                    match idx.col_id(p.fast_len, p.slow_len) {
                        Some(c) => top_cols.push(c),
                        None => {
                            top_cols.clear();
                            break;
                        }
                    }
                }
                let top_cols = if top_cols.is_empty() {
                    None
                } else {
                    Some(top_cols)
                };
                (Some(idx), best_col, top_cols)
            }
            Err(_) => (None, None, None),
        };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        mode_used,
        backend_used: BackendUsed::Gpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
    })
}

fn run_vcc_optimization_cpu(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let vcc_cfg = req.vcc.clone().unwrap_or_default();
    validate_vcc_params(&vcc_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "VCC")?;
    let objective = req.objective;
    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);

    if req.fast_ma_types.len() != 1 || req.slow_ma_types.len() != 1 {
        return Err("Select exactly one fast MA and one slow MA.".to_string());
    }
    let fast_ma = req.fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = req.slow_ma_types[0].trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("Fast/slow MA type cannot be empty.".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_periods = expand_u16_range(req.fast_range)?;
    let slow_periods = expand_u16_range(req.slow_range)?;
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty period range".to_string());
    }
    let total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);
    if total_pairs == 0 {
        return Err("no valid parameter combinations (fast must be < slow)".to_string());
    }
    if include_all && total_pairs > 2_000_000 {
        return Err(format!(
            "include_all=true would return {} rows; disable it or reduce ranges.",
            total_pairs
        ));
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let ma_source = req.ma_source.clone();
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let lr = compute_log_returns_f64(close);

    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    if heatmap_bins > 0 {
        agg = agg.with_heatmap(
            heatmap_bins,
            heatmap_bins,
            *fast_periods.iter().min().unwrap_or(&0),
            *fast_periods.iter().max().unwrap_or(&0),
            *slow_periods.iter().min().unwrap_or(&0),
            *slow_periods.iter().max().unwrap_or(&0),
        );
    }

    let mut processed = 0usize;
    let mut fast_cache: HashMap<(u16, u16), crate::vcc::VccFastSignals> = HashMap::new();
    let mut trend_cache: HashMap<u16, crate::vcc::VccTrendSignals> = HashMap::new();

    let start = std::time::Instant::now();
    emit_progress(progress, processed, total_pairs, "vcc_optimize");

    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_vcc_periods(&vcc_cfg, fast_len, slow_len);
            let fast_key = (periods.squeeze, periods.momentum);
            if !fast_cache.contains_key(&fast_key) {
                let computed = compute_vcc_fast_signals(
                    candles,
                    &ma_source,
                    &Backend::CpuOnly,
                    &vcc_cfg,
                    periods.squeeze,
                    periods.momentum,
                    #[cfg(feature = "cuda")]
                    None,
                )?;
                fast_cache.insert(fast_key, computed);
            }
            if !trend_cache.contains_key(&periods.trend) {
                let computed = compute_vcc_trend_signals(
                    candles,
                    &ma_source,
                    &Backend::CpuOnly,
                    periods.trend,
                    #[cfg(feature = "cuda")]
                    None,
                )?;
                trend_cache.insert(periods.trend, computed);
            }
            let fast_signals = fast_cache
                .get(&fast_key)
                .ok_or_else(|| "VCC fast signal cache miss".to_string())?;
            let trend_signals = trend_cache
                .get(&periods.trend)
                .ok_or_else(|| "VCC trend signal cache miss".to_string())?;

            let max_period =
                usize::from(periods.squeeze.max(periods.momentum).max(periods.trend)).max(1);
            let start_t = compute_vcc_start_t(
                fv_close,
                fv_ma,
                fast_signals.first_valid,
                trend_signals.first_valid,
                max_period,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            let desired = build_vcc_desired_positions(
                close,
                fast_signals,
                trend_signals,
                start_t,
                &strategy,
                &vcc_cfg,
            );
            let metrics = eval_asr_metrics_from_desired(candles, &lr, &desired, start_t, &strategy);
            agg.push(
                DoubleMaParams {
                    fast_len,
                    slow_len,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                metrics,
            );

            processed = processed.saturating_add(1);
            if (processed & 0x03ff) == 0 {
                emit_progress(progress, processed, total_pairs, "vcc_optimize");
            }
        }
    }
    emit_progress(progress, processed, total_pairs, "vcc_optimize");

    let result = agg
        .finalize()
        .ok_or_else(|| "VCC grid search produced no result".to_string())?;
    let runtime_ms = start.elapsed().as_millis() as u64;

    let resolve_params = |p: &DoubleMaParams| -> DoubleMaParamsResolved {
        DoubleMaParamsResolved {
            fast_len: p.fast_len,
            slow_len: p.slow_len,
            fast_ma_type: fast_ma.clone(),
            slow_ma_type: slow_ma.clone(),
            fast_ma_params: req.fast_ma_params.clone(),
            slow_ma_params: req.slow_ma_params.clone(),
        }
    };
    let best_params = resolve_params(&result.best_params);
    let top: Vec<(DoubleMaParamsResolved, Metrics)> = result
        .top
        .iter()
        .map(|(p, m)| (resolve_params(p), m.clone()))
        .collect();
    let all = result.all.as_ref().map(|rows| {
        rows.iter()
            .map(|(p, m)| (resolve_params(p), m.clone()))
            .collect::<Vec<_>>()
    });
    let heatmap = result.heatmap.clone();

    let (matrix_index, best_col, top_cols) =
        match DoubleMaFastLtSlowIndex::new(fast_periods, slow_periods) {
            Ok(idx) => {
                let best_col = idx.col_id(best_params.fast_len, best_params.slow_len);
                let mut top_cols: Vec<u32> = Vec::with_capacity(top.len());
                for (p, _) in &top {
                    match idx.col_id(p.fast_len, p.slow_len) {
                        Some(c) => top_cols.push(c),
                        None => {
                            top_cols.clear();
                            break;
                        }
                    }
                }
                let top_cols = if top_cols.is_empty() {
                    None
                } else {
                    Some(top_cols)
                };
                (Some(idx), best_col, top_cols)
            }
            Err(_) => (None, None, None),
        };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        mode_used,
        backend_used: BackendUsed::Cpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
    })
}

fn run_vemr_optimization(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let normalized_backend = req.backend.clone().with_preferred_cuda_device()?;
        if !matches!(normalized_backend, Backend::CpuOnly) {
            let backend_label = asr_backend_label(&req.backend).to_string();
            if vector_ta::cuda::cuda_available() {
                let device_id = match &normalized_backend {
                    Backend::Auto { device_id }
                    | Backend::GpuOnly { device_id }
                    | Backend::GpuHybridKernel { device_id }
                    | Backend::GpuKernel { device_id } => *device_id,
                    Backend::CpuOnly => 0,
                };
                match run_vemr_optimization_gpu(&req, candles, cancel, progress, device_id) {
                    Ok(mut gpu_res) => {
                        if matches!(req.backend, Backend::Auto { .. }) {
                            gpu_res.backend_route = Some("Auto->GpuKernel".to_string());
                        } else if !matches!(req.backend, Backend::GpuKernel { .. }) {
                            gpu_res.backend_route = Some(format!("{backend_label}->GpuKernel"));
                        }
                        return Ok(gpu_res);
                    }
                    Err(err) => {
                        let mut cpu_res =
                            run_vemr_optimization_cpu(req, candles, cancel, progress)?;
                        cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
                        cpu_res.backend_fallback_reason = Some(err);
                        return Ok(cpu_res);
                    }
                }
            }

            let mut cpu_res = run_vemr_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason =
                Some("CUDA unavailable; falling back to CPU VEMR optimization.".to_string());
            return Ok(cpu_res);
        }
    }

    #[cfg(all(feature = "cuda", not(feature = "cuda-backtest-kernel")))]
    {
        if !matches!(req.backend, Backend::CpuOnly) {
            let backend_label = asr_backend_label(&req.backend).to_string();
            let mut cpu_res = run_vemr_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason = Some(
                "VEMR GPU kernel backend is disabled (build without `cuda-backtest-kernel`)."
                    .to_string(),
            );
            return Ok(cpu_res);
        }
    }

    run_vemr_optimization_cpu(req, candles, cancel, progress)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn run_vemr_optimization_gpu(
    req: &CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    device_id: u32,
) -> Result<BackendOptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let vemr_cfg = req.vemr.clone().unwrap_or_default();
    validate_vemr_params(&vemr_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "VEMR")?;
    let objective = req.objective;
    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);

    if req.fast_ma_types.len() != 1 || req.slow_ma_types.len() != 1 {
        return Err("Select exactly one fast MA and one slow MA.".to_string());
    }
    let fast_ma = req.fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = req.slow_ma_types[0].trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("Fast/slow MA type cannot be empty.".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_periods = expand_u16_range(req.fast_range)?;
    let slow_periods = expand_u16_range(req.slow_range)?;
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty period range".to_string());
    }
    let total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);
    if total_pairs == 0 {
        return Err("no valid parameter combinations (fast must be < slow)".to_string());
    }
    if include_all && total_pairs > 2_000_000 {
        return Err(format!(
            "include_all=true would return {} rows; disable it or reduce ranges.",
            total_pairs
        ));
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let ma_source = req.ma_source.clone();
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;

    let mut fast_keys: Vec<(u16, u16)> = fast_periods
        .iter()
        .map(|&fast_len| {
            let p = resolve_vemr_periods(&vemr_cfg, fast_len, 1);
            (p.z, p.bb)
        })
        .collect();
    fast_keys.sort_unstable();
    fast_keys.dedup();
    let fast_rows = fast_keys.len();
    if fast_rows == 0 {
        return Err("VEMR fast period set is empty".to_string());
    }

    let z_min = fast_keys.iter().map(|k| k.0).min().unwrap_or(1);
    let z_max = fast_keys.iter().map(|k| k.0).max().unwrap_or(z_min);
    let bb_min = fast_keys.iter().map(|k| k.1).min().unwrap_or(1);
    let bb_max = fast_keys.iter().map(|k| k.1).max().unwrap_or(bb_min);

    let z_sweep = compute_vemr_zscore_sweep_cuda_device(
        &prepared_cuda_inputs,
        &vemr_cfg,
        z_min,
        z_max,
        1,
        device_id,
    )?;
    let bb_upper_sweep = compute_vemr_bollinger_upper_sweep_cuda_device(
        &prepared_cuda_inputs,
        &vemr_cfg,
        bb_min,
        bb_max,
        1,
        device_id,
    )?;
    let bb_middle_sweep = compute_vemr_bollinger_middle_sweep_cuda_device(
        &prepared_cuda_inputs,
        &vemr_cfg,
        bb_min,
        bb_max,
        1,
        device_id,
    )?;
    let bb_lower_sweep = compute_vemr_bollinger_lower_sweep_cuda_device(
        &prepared_cuda_inputs,
        &vemr_cfg,
        bb_min,
        bb_max,
        1,
        device_id,
    )?;
    if z_sweep.matrix.cols != n
        || bb_upper_sweep.matrix.cols != n
        || bb_middle_sweep.matrix.cols != n
        || bb_lower_sweep.matrix.cols != n
    {
        return Err("VEMR fast sweep column mismatch".to_string());
    }
    if bb_upper_sweep.matrix.rows != bb_middle_sweep.matrix.rows
        || bb_upper_sweep.matrix.rows != bb_lower_sweep.matrix.rows
    {
        return Err("VEMR bollinger sweep row mismatch".to_string());
    }

    let mut fast_row_lookup: HashMap<(u16, u16), (i32, i32)> = HashMap::with_capacity(fast_rows);
    let mut fast_first_valid_lookup: HashMap<(u16, u16), usize> = HashMap::with_capacity(fast_rows);
    for key in fast_keys.iter().copied() {
        let z_row = z_sweep
            .row_index_for_period(key.0)
            .ok_or_else(|| format!("VEMR zscore row missing for period={}", key.0))?;
        let bb_row = bb_upper_sweep
            .row_index_for_period(key.1)
            .ok_or_else(|| format!("VEMR bollinger upper row missing for period={}", key.1))?;
        if bb_middle_sweep.row_index_for_period(key.1).is_none() {
            return Err(format!(
                "VEMR bollinger middle row missing for period={}",
                key.1
            ));
        }
        if bb_lower_sweep.row_index_for_period(key.1).is_none() {
            return Err(format!(
                "VEMR bollinger lower row missing for period={}",
                key.1
            ));
        }
        let z_row_i =
            i32::try_from(z_row).map_err(|_| "VEMR z row exceeds i32 range".to_string())?;
        let bb_row_i =
            i32::try_from(bb_row).map_err(|_| "VEMR bb row exceeds i32 range".to_string())?;
        fast_row_lookup.insert(key, (z_row_i, bb_row_i));

        let fv = z_sweep
            .first_valid_for_period(key.0)
            .max(bb_upper_sweep.first_valid_for_period(key.1))
            .max(bb_middle_sweep.first_valid_for_period(key.1))
            .max(bb_lower_sweep.first_valid_for_period(key.1));
        fast_first_valid_lookup.insert(key, fv);
    }

    let mut risk_keys: Vec<u16> = slow_periods
        .iter()
        .map(|&slow_len| resolve_vemr_periods(&vemr_cfg, 1, slow_len).atr)
        .collect();
    risk_keys.sort_unstable();
    risk_keys.dedup();
    let risk_rows = risk_keys.len();
    if risk_rows == 0 {
        return Err("VEMR ATR period set is empty".to_string());
    }
    let atr_min = *risk_keys.first().unwrap_or(&1);
    let atr_max = *risk_keys.last().unwrap_or(&atr_min);
    let atr_sweep =
        compute_vemr_atr_sweep_cuda_device(&prepared_cuda_inputs, atr_min, atr_max, 1, device_id)?;
    if atr_sweep.matrix.cols != n {
        return Err("VEMR ATR sweep column mismatch".to_string());
    }

    let mut risk_row_lookup: HashMap<u16, i32> = HashMap::with_capacity(risk_rows);
    let mut risk_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(risk_rows);
    for key in risk_keys.iter().copied() {
        let row = atr_sweep
            .row_index_for_period(key)
            .ok_or_else(|| format!("VEMR ATR row missing for period={key}"))?;
        let row_i = i32::try_from(row).map_err(|_| "VEMR ATR row exceeds i32 range".to_string())?;
        risk_row_lookup.insert(key, row_i);
        risk_first_valid_lookup.insert(key, atr_sweep.first_valid_for_period(key));
    }

    let mut combo_z_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_bb_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_atr_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_params: Vec<DoubleMaParams> = Vec::with_capacity(total_pairs);

    emit_progress(progress, 0, total_pairs, "vemr_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_vemr_periods(&vemr_cfg, fast_len, slow_len);
            let (z_row, bb_row) = *fast_row_lookup
                .get(&(periods.z, periods.bb))
                .ok_or_else(|| "VEMR fast row lookup miss".to_string())?;
            let atr_row = *risk_row_lookup
                .get(&periods.atr)
                .ok_or_else(|| "VEMR risk row lookup miss".to_string())?;
            let fast_first_valid = *fast_first_valid_lookup
                .get(&(periods.z, periods.bb))
                .ok_or_else(|| "VEMR fast first_valid lookup miss".to_string())?;
            let risk_first_valid = *risk_first_valid_lookup
                .get(&periods.atr)
                .ok_or_else(|| "VEMR risk first_valid lookup miss".to_string())?;
            let max_period = usize::from(periods.z.max(periods.bb).max(periods.atr)).max(1);
            let start_t = compute_vemr_start_t(
                fv_close,
                fv_ma,
                fast_first_valid,
                risk_first_valid,
                max_period,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            combo_z_row_idx.push(z_row);
            combo_bb_row_idx.push(bb_row);
            combo_atr_row_idx.push(atr_row);
            combo_start_t.push(
                i32::try_from(start_t).map_err(|_| "VEMR start_t exceeds i32 range".to_string())?,
            );
            combo_params.push(DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            });

            if (combo_params.len() & 0x03ff) == 0 {
                emit_progress(progress, combo_params.len(), total_pairs, "vemr_optimize");
            }
        }
    }
    emit_progress(progress, combo_params.len(), total_pairs, "vemr_optimize");

    if combo_params.is_empty() {
        return Err("VEMR grid search produced no valid parameter combinations".to_string());
    }

    let signals = VemrGpuDeviceSignalMatrices {
        close_f32: prepared_cuda_inputs.close(),
        cols: n,
        zscore_row_major_ptr: z_sweep.matrix.device_ptr,
        zscore_rows: z_sweep.matrix.rows,
        zscore_stride: z_sweep.matrix.cols,
        bb_upper_row_major_ptr: bb_upper_sweep.matrix.device_ptr,
        bb_middle_row_major_ptr: bb_middle_sweep.matrix.device_ptr,
        bb_lower_row_major_ptr: bb_lower_sweep.matrix.device_ptr,
        bb_rows: bb_upper_sweep.matrix.rows,
        bb_stride: bb_upper_sweep.matrix.cols,
        atr_row_major_ptr: atr_sweep.matrix.device_ptr,
        atr_rows: atr_sweep.matrix.rows,
        atr_stride: atr_sweep.matrix.cols,
    };
    let combos = VemrGpuComboBatch {
        z_row_index: &combo_z_row_idx,
        bb_row_index: &combo_bb_row_idx,
        atr_row_index: &combo_atr_row_idx,
        start_t: &combo_start_t,
    };
    let num_combos_total = combo_params.len();

    let start = std::time::Instant::now();
    let gpu_eval = eval_vemr_metrics_gpu_kernel_device_rows(
        device_id,
        &signals,
        &combos,
        &strategy,
        &vemr_cfg,
        objective,
        top_k,
        include_all,
        heatmap_bins,
    )?;

    let resolve_params = |p: &DoubleMaParams| -> DoubleMaParamsResolved {
        DoubleMaParamsResolved {
            fast_len: p.fast_len,
            slow_len: p.slow_len,
            fast_ma_type: fast_ma.clone(),
            slow_ma_type: slow_ma.clone(),
            fast_ma_params: req.fast_ma_params.clone(),
            slow_ma_params: req.slow_ma_params.clone(),
        }
    };
    let (best_params, best_metrics, top, all, heatmap, num_combos) = match gpu_eval {
        VemrGpuEvalResult::Full(metrics) => {
            if metrics.len() != combo_params.len() {
                return Err("VEMR GPU kernel output length mismatch".to_string());
            }

            let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
            if heatmap_bins > 0 {
                agg = agg.with_heatmap(
                    heatmap_bins,
                    heatmap_bins,
                    *fast_periods.iter().min().unwrap_or(&0),
                    *fast_periods.iter().max().unwrap_or(&0),
                    *slow_periods.iter().min().unwrap_or(&0),
                    *slow_periods.iter().max().unwrap_or(&0),
                );
            }
            for (i, (params, m)) in combo_params
                .into_iter()
                .zip(metrics.into_iter())
                .enumerate()
            {
                agg.push(params, m);
                if (i & 0x03ff) == 0 {
                    emit_progress(progress, i + 1, total_pairs, "vemr_optimize");
                }
            }
            emit_progress(progress, total_pairs, total_pairs, "vemr_optimize");

            let result = agg
                .finalize()
                .ok_or_else(|| "VEMR grid search produced no result".to_string())?;
            let best_params = resolve_params(&result.best_params);
            let top: Vec<(DoubleMaParamsResolved, Metrics)> = result
                .top
                .iter()
                .map(|(p, m)| (resolve_params(p), m.clone()))
                .collect();
            let all = result.all.as_ref().map(|rows| {
                rows.iter()
                    .map(|(p, m)| (resolve_params(p), m.clone()))
                    .collect::<Vec<_>>()
            });
            (
                best_params,
                result.best_metrics,
                top,
                all,
                result.heatmap.clone(),
                result.num_combos,
            )
        }
        VemrGpuEvalResult::TopK(mut top_rows) => {
            if top_rows.is_empty() {
                return Err("VEMR GPU top-k reduction produced no rows".to_string());
            }
            top_rows.sort_by(|a, b| {
                objective_score(&b.metrics, objective)
                    .total_cmp(&objective_score(&a.metrics, objective))
            });

            let best_combo = combo_params
                .get(top_rows[0].combo_index)
                .ok_or_else(|| "VEMR GPU best combo index out of range".to_string())?;
            let best_params = resolve_params(best_combo);
            let best_metrics = top_rows[0].metrics.clone();

            let mut top: Vec<(DoubleMaParamsResolved, Metrics)> = Vec::with_capacity(top_k);
            if top_k > 0 {
                for row in top_rows.into_iter().take(top_k) {
                    let params = combo_params
                        .get(row.combo_index)
                        .ok_or_else(|| "VEMR GPU top combo index out of range".to_string())?;
                    top.push((resolve_params(params), row.metrics));
                }
            }
            emit_progress(progress, total_pairs, total_pairs, "vemr_optimize");

            (best_params, best_metrics, top, None, None, num_combos_total)
        }
    };
    let runtime_ms = start.elapsed().as_millis() as u64;

    let (matrix_index, best_col, top_cols) =
        match DoubleMaFastLtSlowIndex::new(fast_periods, slow_periods) {
            Ok(idx) => {
                let best_col = idx.col_id(best_params.fast_len, best_params.slow_len);
                let mut top_cols: Vec<u32> = Vec::with_capacity(top.len());
                for (p, _) in &top {
                    match idx.col_id(p.fast_len, p.slow_len) {
                        Some(c) => top_cols.push(c),
                        None => {
                            top_cols.clear();
                            break;
                        }
                    }
                }
                let top_cols = if top_cols.is_empty() {
                    None
                } else {
                    Some(top_cols)
                };
                (Some(idx), best_col, top_cols)
            }
            Err(_) => (None, None, None),
        };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics,
        mode_used,
        backend_used: BackendUsed::Gpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos,
        num_candles: n,
        runtime_ms,
        top,
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
    })
}

fn run_vemr_optimization_cpu(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    run_vemr_optimization_impl(
        &req,
        candles,
        cancel,
        progress,
        &Backend::CpuOnly,
        BackendUsed::Cpu,
    )
}

fn run_vemr_optimization_impl(
    req: &CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    indicator_backend: &Backend,
    backend_used: BackendUsed,
) -> Result<BackendOptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let vemr_cfg = req.vemr.clone().unwrap_or_default();
    validate_vemr_params(&vemr_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "VEMR")?;
    let objective = req.objective;
    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);

    if req.fast_ma_types.len() != 1 || req.slow_ma_types.len() != 1 {
        return Err("Select exactly one fast MA and one slow MA.".to_string());
    }
    let fast_ma = req.fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = req.slow_ma_types[0].trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("Fast/slow MA type cannot be empty.".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_periods = expand_u16_range(req.fast_range)?;
    let slow_periods = expand_u16_range(req.slow_range)?;
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty period range".to_string());
    }
    let total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);
    if total_pairs == 0 {
        return Err("no valid parameter combinations (fast must be < slow)".to_string());
    }
    if include_all && total_pairs > 2_000_000 {
        return Err(format!(
            "include_all=true would return {} rows; disable it or reduce ranges.",
            total_pairs
        ));
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let ma_source = req.ma_source.clone();
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let lr = compute_log_returns_f64(close);

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(indicator_backend, Backend::CpuOnly) {
        None
    } else {
        PreparedCudaIndicatorInputs::from_candles(candles, &ma_source).ok()
    };

    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    if heatmap_bins > 0 {
        agg = agg.with_heatmap(
            heatmap_bins,
            heatmap_bins,
            *fast_periods.iter().min().unwrap_or(&0),
            *fast_periods.iter().max().unwrap_or(&0),
            *slow_periods.iter().min().unwrap_or(&0),
            *slow_periods.iter().max().unwrap_or(&0),
        );
    }

    let mut processed = 0usize;
    let mut fast_cache: HashMap<(u16, u16), crate::vemr::VemrFastSignals> = HashMap::new();
    let mut risk_cache: HashMap<u16, crate::vemr::VemrRiskSignals> = HashMap::new();

    let start = std::time::Instant::now();
    emit_progress(progress, processed, total_pairs, "vemr_optimize");

    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_vemr_periods(&vemr_cfg, fast_len, slow_len);
            let fast_key = (periods.z, periods.bb);
            if !fast_cache.contains_key(&fast_key) {
                let computed = compute_vemr_fast_signals(
                    candles,
                    &ma_source,
                    indicator_backend,
                    &vemr_cfg,
                    periods.z,
                    periods.bb,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                fast_cache.insert(fast_key, computed);
            }
            if !risk_cache.contains_key(&periods.atr) {
                let computed = compute_vemr_risk_signals(
                    candles,
                    &ma_source,
                    indicator_backend,
                    periods.atr,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                risk_cache.insert(periods.atr, computed);
            }
            let fast_signals = fast_cache
                .get(&fast_key)
                .ok_or_else(|| "VEMR fast signal cache miss".to_string())?;
            let risk_signals = risk_cache
                .get(&periods.atr)
                .ok_or_else(|| "VEMR risk signal cache miss".to_string())?;

            let max_period = usize::from(periods.z.max(periods.bb).max(periods.atr)).max(1);
            let start_t = compute_vemr_start_t(
                fv_close,
                fv_ma,
                fast_signals.first_valid,
                risk_signals.first_valid,
                max_period,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            let desired = build_vemr_desired_positions(
                close,
                fast_signals,
                risk_signals,
                start_t,
                &strategy,
                &vemr_cfg,
            );
            let metrics = eval_asr_metrics_from_desired(candles, &lr, &desired, start_t, &strategy);
            agg.push(
                DoubleMaParams {
                    fast_len,
                    slow_len,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                metrics,
            );

            processed = processed.saturating_add(1);
            if (processed & 0x03ff) == 0 {
                emit_progress(progress, processed, total_pairs, "vemr_optimize");
            }
        }
    }
    emit_progress(progress, processed, total_pairs, "vemr_optimize");

    let result = agg
        .finalize()
        .ok_or_else(|| "VEMR grid search produced no result".to_string())?;
    let runtime_ms = start.elapsed().as_millis() as u64;

    let resolve_params = |p: &DoubleMaParams| -> DoubleMaParamsResolved {
        DoubleMaParamsResolved {
            fast_len: p.fast_len,
            slow_len: p.slow_len,
            fast_ma_type: fast_ma.clone(),
            slow_ma_type: slow_ma.clone(),
            fast_ma_params: req.fast_ma_params.clone(),
            slow_ma_params: req.slow_ma_params.clone(),
        }
    };
    let best_params = resolve_params(&result.best_params);
    let top: Vec<(DoubleMaParamsResolved, Metrics)> = result
        .top
        .iter()
        .map(|(p, m)| (resolve_params(p), m.clone()))
        .collect();
    let all = result.all.as_ref().map(|rows| {
        rows.iter()
            .map(|(p, m)| (resolve_params(p), m.clone()))
            .collect::<Vec<_>>()
    });
    let heatmap = result.heatmap.clone();

    let (matrix_index, best_col, top_cols) =
        match DoubleMaFastLtSlowIndex::new(fast_periods, slow_periods) {
            Ok(idx) => {
                let best_col = idx.col_id(best_params.fast_len, best_params.slow_len);
                let mut top_cols: Vec<u32> = Vec::with_capacity(top.len());
                for (p, _) in &top {
                    match idx.col_id(p.fast_len, p.slow_len) {
                        Some(c) => top_cols.push(c),
                        None => {
                            top_cols.clear();
                            break;
                        }
                    }
                }
                let top_cols = if top_cols.is_empty() {
                    None
                } else {
                    Some(top_cols)
                };
                (Some(idx), best_col, top_cols)
            }
            Err(_) => (None, None, None),
        };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        mode_used,
        backend_used,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
    })
}

fn run_asr_drilldown(
    req: CompositeCpuDrilldownRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<CompositeCpuDrilldownResult, String> {
    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_ma = req.params.fast_ma_type.trim();
    let slow_ma = req.params.slow_ma_type.trim();
    validate_ma_kind(fast_ma)?;
    validate_ma_kind(slow_ma)?;

    let fast_len = req.params.fast_len;
    let slow_len = req.params.slow_len;
    if fast_len == 0 || slow_len == 0 {
        return Err("periods must be > 0".to_string());
    }
    if fast_len >= slow_len {
        return Err("invalid params: fast_len must be < slow_len".to_string());
    }
    if fast_len as usize > n || slow_len as usize > n {
        return Err(format!(
            "period exceeds series length: fast_len={fast_len} slow_len={slow_len} n={n}"
        ));
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let asr_cfg = req.asr.clone().unwrap_or_default();
    validate_asr_params(&asr_cfg)?;

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(req.backend, Backend::CpuOnly) {
        None
    } else {
        PreparedCudaIndicatorInputs::from_candles(candles, &req.ma_source).ok()
    };

    let periods = resolve_asr_periods(&asr_cfg, fast_len, slow_len);
    let fast_signals = compute_asr_fast_signals(
        candles,
        &req.ma_source,
        &req.backend,
        &asr_cfg,
        periods.squeeze,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs.as_ref(),
    )?;
    let slow_signals = compute_asr_slow_signals(
        candles,
        &req.ma_source,
        &req.backend,
        &asr_cfg,
        periods.trend,
        periods.er,
        periods.z,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs.as_ref(),
    )?;
    let kaufman = compute_asr_kaufman_stops(candles, &strategy)?;

    let max_period = usize::from(
        periods
            .squeeze
            .max(periods.trend)
            .max(periods.er)
            .max(periods.z),
    )
    .max(1);
    let start_t = compute_asr_start_t(
        fv_close,
        fv_ma,
        fast_signals.first_valid,
        slow_signals.first_valid,
        kaufman.first_valid,
        max_period,
        strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup for analysis".to_string());
    }

    let lr = compute_log_returns_f64(close);
    let bins = req.bins.unwrap_or(1024).clamp(1, 8192);
    let desired = build_asr_desired_positions(
        close,
        &fast_signals,
        &slow_signals,
        &kaufman,
        start_t,
        &strategy,
        &asr_cfg,
    );
    let curves = eval_asr_curves_from_desired(candles, &lr, &desired, start_t, &strategy, bins);
    Ok(CompositeCpuDrilldownResult {
        curves,
        composite_indicator_bars: Vec::new(),
    })
}

fn run_vemr_drilldown(
    req: CompositeCpuDrilldownRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<CompositeCpuDrilldownResult, String> {
    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_ma = req.params.fast_ma_type.trim();
    let slow_ma = req.params.slow_ma_type.trim();
    validate_ma_kind(fast_ma)?;
    validate_ma_kind(slow_ma)?;

    let fast_len = req.params.fast_len;
    let slow_len = req.params.slow_len;
    if fast_len == 0 || slow_len == 0 {
        return Err("periods must be > 0".to_string());
    }
    if fast_len >= slow_len {
        return Err("invalid params: fast_len must be < slow_len".to_string());
    }
    if fast_len as usize > n || slow_len as usize > n {
        return Err(format!(
            "period exceeds series length: fast_len={fast_len} slow_len={slow_len} n={n}"
        ));
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let vemr_cfg = req.vemr.clone().unwrap_or_default();
    validate_vemr_params(&vemr_cfg)?;

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(req.backend, Backend::CpuOnly) {
        None
    } else {
        PreparedCudaIndicatorInputs::from_candles(candles, &req.ma_source).ok()
    };

    let periods = resolve_vemr_periods(&vemr_cfg, fast_len, slow_len);
    let fast_signals = compute_vemr_fast_signals(
        candles,
        &req.ma_source,
        &req.backend,
        &vemr_cfg,
        periods.z,
        periods.bb,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs.as_ref(),
    )?;
    let risk_signals = compute_vemr_risk_signals(
        candles,
        &req.ma_source,
        &req.backend,
        periods.atr,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs.as_ref(),
    )?;

    let max_period = usize::from(periods.z.max(periods.bb).max(periods.atr)).max(1);
    let start_t = compute_vemr_start_t(
        fv_close,
        fv_ma,
        fast_signals.first_valid,
        risk_signals.first_valid,
        max_period,
        strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup for analysis".to_string());
    }

    let lr = compute_log_returns_f64(close);
    let bins = req.bins.unwrap_or(1024).clamp(1, 8192);
    let desired = build_vemr_desired_positions(
        close,
        &fast_signals,
        &risk_signals,
        start_t,
        &strategy,
        &vemr_cfg,
    );
    let curves = eval_asr_curves_from_desired(candles, &lr, &desired, start_t, &strategy, bins);
    Ok(CompositeCpuDrilldownResult {
        curves,
        composite_indicator_bars: Vec::new(),
    })
}

fn run_asr_local_stability(
    req: CompositeCpuLocalStabilityRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<LocalStabilityResult, String> {
    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_ma = req.params.fast_ma_type.trim();
    let slow_ma = req.params.slow_ma_type.trim();
    validate_ma_kind(fast_ma)?;
    validate_ma_kind(slow_ma)?;

    let base_fast = req.params.fast_len;
    let base_slow = req.params.slow_len;
    if base_fast == 0 || base_slow == 0 {
        return Err("periods must be > 0".to_string());
    }
    if base_fast >= base_slow {
        return Err("invalid params: fast_len must be < slow_len".to_string());
    }
    if base_fast as usize > n || base_slow as usize > n {
        return Err(format!(
            "period exceeds series length: fast_len={base_fast} slow_len={base_slow} n={n}"
        ));
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let asr_cfg = req.asr.clone().unwrap_or_default();
    validate_asr_params(&asr_cfg)?;

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(req.backend, Backend::CpuOnly) {
        None
    } else {
        PreparedCudaIndicatorInputs::from_candles(candles, &req.ma_source).ok()
    };

    let radius = req.radius.unwrap_or(1).min(32);
    let step = req.step.unwrap_or(1).max(1);

    let mut fast_periods: Vec<u16> = Vec::new();
    let mut df = -(radius as i32);
    while df <= radius as i32 {
        let v = base_fast as i32 + df;
        if v > 0 {
            fast_periods.push(v as u16);
        }
        df += step as i32;
    }
    let mut slow_periods: Vec<u16> = Vec::new();
    let mut ds = -(radius as i32);
    while ds <= radius as i32 {
        let v = base_slow as i32 + ds;
        if v > 0 {
            slow_periods.push(v as u16);
        }
        ds += step as i32;
    }
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty neighborhood".to_string());
    }
    fast_periods.sort_unstable();
    slow_periods.sort_unstable();

    let mut fast_cache: HashMap<u16, crate::asr::AsrFastSignals> = HashMap::new();
    for &fast_len in &fast_periods {
        let squeeze_period = resolve_asr_periods(&asr_cfg, fast_len, base_slow).squeeze;
        if fast_cache.contains_key(&squeeze_period) {
            continue;
        }
        let fast_signals = compute_asr_fast_signals(
            candles,
            &req.ma_source,
            &req.backend,
            &asr_cfg,
            squeeze_period,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs.as_ref(),
        )?;
        fast_cache.insert(squeeze_period, fast_signals);
    }

    let mut slow_cache: HashMap<(u16, u16, u16), crate::asr::AsrSlowSignals> = HashMap::new();
    for &slow_len in &slow_periods {
        let periods = resolve_asr_periods(&asr_cfg, base_fast, slow_len);
        let key = (periods.trend, periods.er, periods.z);
        if slow_cache.contains_key(&key) {
            continue;
        }
        let slow_signals = compute_asr_slow_signals(
            candles,
            &req.ma_source,
            &req.backend,
            &asr_cfg,
            periods.trend,
            periods.er,
            periods.z,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs.as_ref(),
        )?;
        slow_cache.insert(key, slow_signals);
    }

    let lr = compute_log_returns_f64(close);
    let kaufman = compute_asr_kaufman_stops(candles, &strategy)?;
    let objective = req.objective;

    let mut max_period = 1usize;
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            let p = resolve_asr_periods(&asr_cfg, fast_len, slow_len);
            let cur_max = usize::from(p.squeeze.max(p.trend).max(p.er).max(p.z)).max(1);
            if cur_max > max_period {
                max_period = cur_max;
            }
        }
    }
    let first_valid_fast = fast_cache
        .values()
        .map(|row| row.first_valid)
        .max()
        .unwrap_or(fv_ma);
    let first_valid_slow = slow_cache
        .values()
        .map(|row| row.first_valid)
        .max()
        .unwrap_or(fv_ma);
    let start_t = compute_asr_start_t(
        fv_close,
        fv_ma,
        first_valid_fast,
        first_valid_slow,
        kaufman.first_valid,
        max_period,
        strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup for analysis".to_string());
    }

    let mut points: Vec<LocalStabilityPoint> = Vec::new();
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            let periods = resolve_asr_periods(&asr_cfg, fast_len, slow_len);
            let fast_signals = fast_cache
                .get(&periods.squeeze)
                .ok_or_else(|| "ASR fast signal cache miss".to_string())?;
            let slow_signals = slow_cache
                .get(&(periods.trend, periods.er, periods.z))
                .ok_or_else(|| "ASR slow signal cache miss".to_string())?;

            let desired = build_asr_desired_positions(
                close,
                fast_signals,
                slow_signals,
                &kaufman,
                start_t,
                &strategy,
                &asr_cfg,
            );
            let metrics = eval_asr_metrics_from_desired(candles, &lr, &desired, start_t, &strategy);
            let score = objective_score(&metrics, objective);
            points.push(LocalStabilityPoint {
                fast_len,
                slow_len,
                metrics,
                score,
            });
        }
    }
    if points.is_empty() {
        return Err("empty neighborhood".to_string());
    }

    let base_score = points
        .iter()
        .find(|p| p.fast_len == base_fast && p.slow_len == base_slow)
        .map(|p| p.score)
        .unwrap_or(0.0);

    let mut scores: Vec<f64> = points.iter().map(|p| p.score).collect();
    scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let min = *scores.first().unwrap_or(&0.0);
    let max = *scores.last().unwrap_or(&0.0);
    let median = if scores.is_empty() {
        0.0
    } else {
        scores[scores.len() / 2]
    };

    Ok(LocalStabilityResult {
        base: DoubleMaParamsResolved {
            fast_len: base_fast,
            slow_len: base_slow,
            fast_ma_type: req.params.fast_ma_type,
            slow_ma_type: req.params.slow_ma_type,
            fast_ma_params: req.fast_ma_params,
            slow_ma_params: req.slow_ma_params,
        },
        objective,
        radius,
        step,
        start_t,
        points,
        summary: LocalStabilitySummary {
            base: base_score,
            min,
            median,
            max,
        },
    })
}

fn run_vemr_local_stability(
    req: CompositeCpuLocalStabilityRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<LocalStabilityResult, String> {
    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_ma = req.params.fast_ma_type.trim();
    let slow_ma = req.params.slow_ma_type.trim();
    validate_ma_kind(fast_ma)?;
    validate_ma_kind(slow_ma)?;

    let base_fast = req.params.fast_len;
    let base_slow = req.params.slow_len;
    if base_fast == 0 || base_slow == 0 {
        return Err("periods must be > 0".to_string());
    }
    if base_fast >= base_slow {
        return Err("invalid params: fast_len must be < slow_len".to_string());
    }
    if base_fast as usize > n || base_slow as usize > n {
        return Err(format!(
            "period exceeds series length: fast_len={base_fast} slow_len={base_slow} n={n}"
        ));
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let vemr_cfg = req.vemr.clone().unwrap_or_default();
    validate_vemr_params(&vemr_cfg)?;

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(req.backend, Backend::CpuOnly) {
        None
    } else {
        PreparedCudaIndicatorInputs::from_candles(candles, &req.ma_source).ok()
    };

    let radius = req.radius.unwrap_or(1).min(32);
    let step = req.step.unwrap_or(1).max(1);

    let mut fast_periods: Vec<u16> = Vec::new();
    let mut df = -(radius as i32);
    while df <= radius as i32 {
        let v = base_fast as i32 + df;
        if v > 0 {
            fast_periods.push(v as u16);
        }
        df += step as i32;
    }
    let mut slow_periods: Vec<u16> = Vec::new();
    let mut ds = -(radius as i32);
    while ds <= radius as i32 {
        let v = base_slow as i32 + ds;
        if v > 0 {
            slow_periods.push(v as u16);
        }
        ds += step as i32;
    }
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty neighborhood".to_string());
    }
    fast_periods.sort_unstable();
    slow_periods.sort_unstable();

    let mut fast_cache: HashMap<(u16, u16), crate::vemr::VemrFastSignals> = HashMap::new();
    let mut risk_cache: HashMap<u16, crate::vemr::VemrRiskSignals> = HashMap::new();
    let mut max_period = 1usize;
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            let periods = resolve_vemr_periods(&vemr_cfg, fast_len, slow_len);
            let fast_key = (periods.z, periods.bb);
            if !fast_cache.contains_key(&fast_key) {
                let fast_signals = compute_vemr_fast_signals(
                    candles,
                    &req.ma_source,
                    &req.backend,
                    &vemr_cfg,
                    periods.z,
                    periods.bb,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                fast_cache.insert(fast_key, fast_signals);
            }
            if !risk_cache.contains_key(&periods.atr) {
                let risk_signals = compute_vemr_risk_signals(
                    candles,
                    &req.ma_source,
                    &req.backend,
                    periods.atr,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                risk_cache.insert(periods.atr, risk_signals);
            }
            let cur_max = usize::from(periods.z.max(periods.bb).max(periods.atr)).max(1);
            if cur_max > max_period {
                max_period = cur_max;
            }
        }
    }

    let first_valid_fast = fast_cache
        .values()
        .map(|row| row.first_valid)
        .max()
        .unwrap_or(fv_ma);
    let first_valid_risk = risk_cache
        .values()
        .map(|row| row.first_valid)
        .max()
        .unwrap_or(fv_ma);
    let start_t = compute_vemr_start_t(
        fv_close,
        fv_ma,
        first_valid_fast,
        first_valid_risk,
        max_period,
        strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup for analysis".to_string());
    }

    let lr = compute_log_returns_f64(close);
    let objective = req.objective;
    let mut points: Vec<LocalStabilityPoint> = Vec::new();
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            let periods = resolve_vemr_periods(&vemr_cfg, fast_len, slow_len);
            let fast_signals = fast_cache
                .get(&(periods.z, periods.bb))
                .ok_or_else(|| "VEMR fast signal cache miss".to_string())?;
            let risk_signals = risk_cache
                .get(&periods.atr)
                .ok_or_else(|| "VEMR risk signal cache miss".to_string())?;

            let desired = build_vemr_desired_positions(
                close,
                fast_signals,
                risk_signals,
                start_t,
                &strategy,
                &vemr_cfg,
            );
            let metrics = eval_asr_metrics_from_desired(candles, &lr, &desired, start_t, &strategy);
            let score = objective_score(&metrics, objective);
            points.push(LocalStabilityPoint {
                fast_len,
                slow_len,
                metrics,
                score,
            });
        }
    }
    if points.is_empty() {
        return Err("empty neighborhood".to_string());
    }

    let base_score = points
        .iter()
        .find(|p| p.fast_len == base_fast && p.slow_len == base_slow)
        .map(|p| p.score)
        .unwrap_or(0.0);

    let mut scores: Vec<f64> = points.iter().map(|p| p.score).collect();
    scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let min = *scores.first().unwrap_or(&0.0);
    let max = *scores.last().unwrap_or(&0.0);
    let median = if scores.is_empty() {
        0.0
    } else {
        scores[scores.len() / 2]
    };

    Ok(LocalStabilityResult {
        base: DoubleMaParamsResolved {
            fast_len: base_fast,
            slow_len: base_slow,
            fast_ma_type: req.params.fast_ma_type,
            slow_ma_type: req.params.slow_ma_type,
            fast_ma_params: req.fast_ma_params,
            slow_ma_params: req.slow_ma_params,
        },
        objective,
        radius,
        step,
        start_t,
        points,
        summary: LocalStabilitySummary {
            base: base_score,
            min,
            median,
            max,
        },
    })
}

fn run_rocb_optimization(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let normalized_backend = req.backend.clone().with_preferred_cuda_device()?;
        if !matches!(normalized_backend, Backend::CpuOnly) {
            let backend_label = asr_backend_label(&req.backend).to_string();
            if vector_ta::cuda::cuda_available() {
                let device_id = match &normalized_backend {
                    Backend::Auto { device_id }
                    | Backend::GpuOnly { device_id }
                    | Backend::GpuHybridKernel { device_id }
                    | Backend::GpuKernel { device_id } => *device_id,
                    Backend::CpuOnly => 0,
                };
                match run_rocb_optimization_gpu(&req, candles, cancel, progress, device_id) {
                    Ok(mut gpu_res) => {
                        if matches!(req.backend, Backend::Auto { .. }) {
                            gpu_res.backend_route = Some("Auto->GpuKernel".to_string());
                        } else if !matches!(req.backend, Backend::GpuKernel { .. }) {
                            gpu_res.backend_route = Some(format!("{backend_label}->GpuKernel"));
                        }
                        return Ok(gpu_res);
                    }
                    Err(err) => {
                        let mut cpu_res =
                            run_rocb_optimization_cpu(req, candles, cancel, progress)?;
                        cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
                        cpu_res.backend_fallback_reason = Some(err);
                        return Ok(cpu_res);
                    }
                }
            }

            let mut cpu_res = run_rocb_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason =
                Some("CUDA unavailable; falling back to CPU ROCB optimization.".to_string());
            return Ok(cpu_res);
        }
    }

    #[cfg(all(feature = "cuda", not(feature = "cuda-backtest-kernel")))]
    {
        if !matches!(req.backend, Backend::CpuOnly) {
            let backend_label = asr_backend_label(&req.backend).to_string();
            let mut cpu_res = run_rocb_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason = Some(
                "ROCB GPU kernel backend is disabled (build without `cuda-backtest-kernel`)."
                    .to_string(),
            );
            return Ok(cpu_res);
        }
    }

    run_rocb_optimization_cpu(req, candles, cancel, progress)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn run_rocb_optimization_gpu(
    req: &CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    device_id: u32,
) -> Result<BackendOptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let rocb_cfg = req.rocb.clone().unwrap_or_default();
    validate_rocb_params(&rocb_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "ROCB")?;
    let objective = req.objective;
    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);

    if req.fast_ma_types.len() != 1 || req.slow_ma_types.len() != 1 {
        return Err("Select exactly one fast MA and one slow MA.".to_string());
    }
    let fast_ma = req.fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = req.slow_ma_types[0].trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("Fast/slow MA type cannot be empty.".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_periods = expand_u16_range(req.fast_range)?;
    let slow_periods = expand_u16_range(req.slow_range)?;
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty period range".to_string());
    }
    let total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);
    if total_pairs == 0 {
        return Err("no valid parameter combinations (fast must be < slow)".to_string());
    }
    if include_all && total_pairs > 2_000_000 {
        return Err(format!(
            "include_all=true would return {} rows; disable it or reduce ranges.",
            total_pairs
        ));
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let high = vector_ta::utilities::data_loader::source_type(candles, "high");
    let low = vector_ta::utilities::data_loader::source_type(candles, "low");
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_high = first_finite_idx(high).ok_or_else(|| "high series all non-finite".to_string())?;
    let fv_low = first_finite_idx(low).ok_or_else(|| "low series all non-finite".to_string())?;

    let prepared_cuda_inputs =
        PreparedCudaIndicatorInputs::from_candles_on_device(candles, "close", device_id as usize)
            .map_err(|e| e.to_string())?;

    let mut channel_keys: Vec<u16> = fast_periods
        .iter()
        .map(|&fast_len| resolve_rocb_periods(&rocb_cfg, fast_len, 1).channel)
        .collect();
    channel_keys.sort_unstable();
    channel_keys.dedup();
    if channel_keys.is_empty() {
        return Err("ROCB channel period set is empty".to_string());
    }

    let channel_min = *channel_keys.first().unwrap_or(&1);
    let channel_max = *channel_keys.last().unwrap_or(&channel_min);
    let upper_sweep = compute_rocb_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["upper", "upperband", "value"],
        channel_min,
        channel_max,
        1,
    )?;
    let lower_sweep = compute_rocb_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["lower", "lowerband"],
        channel_min,
        channel_max,
        1,
    )?;
    if upper_sweep.matrix.cols != n || lower_sweep.matrix.cols != n {
        return Err("ROCB donchian sweep column mismatch".to_string());
    }
    if upper_sweep.matrix.rows != lower_sweep.matrix.rows {
        return Err("ROCB donchian sweep row mismatch".to_string());
    }

    let mut channel_row_lookup: HashMap<u16, i32> = HashMap::with_capacity(channel_keys.len());
    let mut channel_first_valid_lookup: HashMap<u16, usize> =
        HashMap::with_capacity(channel_keys.len());
    for period in channel_keys.iter().copied() {
        let upper_row = upper_sweep
            .row_index_for_period(period)
            .ok_or_else(|| format!("ROCB upper donchian row missing for period={period}"))?;
        if lower_sweep.row_index_for_period(period).is_none() {
            return Err(format!(
                "ROCB lower donchian row missing for period={period}"
            ));
        }
        let row_i = i32::try_from(upper_row)
            .map_err(|_| "ROCB channel row exceeds i32 range".to_string())?;
        channel_row_lookup.insert(period, row_i);
        channel_first_valid_lookup.insert(period, upper_sweep.first_valid_for_period(period));
    }

    let mut atr_keys: Vec<u16> = slow_periods
        .iter()
        .map(|&slow_len| resolve_rocb_periods(&rocb_cfg, 1, slow_len).atr)
        .collect();
    atr_keys.sort_unstable();
    atr_keys.dedup();
    if atr_keys.is_empty() {
        return Err("ROCB ATR period set is empty".to_string());
    }
    let atr_min = *atr_keys.first().unwrap_or(&1);
    let atr_max = *atr_keys.last().unwrap_or(&atr_min);
    let atr_sweep = compute_rocb_atr_sweep_cuda_device(&prepared_cuda_inputs, atr_min, atr_max, 1)?;
    if atr_sweep.matrix.cols != n {
        return Err("ROCB ATR sweep column mismatch".to_string());
    }

    let mut atr_row_lookup: HashMap<u16, i32> = HashMap::with_capacity(atr_keys.len());
    let mut atr_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(atr_keys.len());
    for period in atr_keys.iter().copied() {
        let row = atr_sweep
            .row_index_for_period(period)
            .ok_or_else(|| format!("ROCB ATR row missing for period={period}"))?;
        let row_i = i32::try_from(row).map_err(|_| "ROCB ATR row exceeds i32 range".to_string())?;
        atr_row_lookup.insert(period, row_i);
        atr_first_valid_lookup.insert(period, atr_sweep.first_valid_for_period(period));
    }

    let mut combo_channel_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_atr_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_params: Vec<DoubleMaParams> = Vec::with_capacity(total_pairs);

    emit_progress(progress, 0, total_pairs, "rocb_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_rocb_periods(&rocb_cfg, fast_len, slow_len);
            let channel_row = *channel_row_lookup
                .get(&periods.channel)
                .ok_or_else(|| "ROCB channel row lookup miss".to_string())?;
            let atr_row = *atr_row_lookup
                .get(&periods.atr)
                .ok_or_else(|| "ROCB ATR row lookup miss".to_string())?;
            let channel_first_valid = *channel_first_valid_lookup
                .get(&periods.channel)
                .ok_or_else(|| "ROCB channel first_valid lookup miss".to_string())?;
            let atr_first_valid = *atr_first_valid_lookup
                .get(&periods.atr)
                .ok_or_else(|| "ROCB ATR first_valid lookup miss".to_string())?;
            let start_t = compute_rocb_start_t(
                fv_close,
                fv_high,
                fv_low,
                channel_first_valid,
                atr_first_valid,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            combo_channel_row_idx.push(channel_row);
            combo_atr_row_idx.push(atr_row);
            combo_start_t.push(
                i32::try_from(start_t).map_err(|_| "ROCB start_t exceeds i32 range".to_string())?,
            );
            combo_params.push(DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            });

            if (combo_params.len() & 0x03ff) == 0 {
                emit_progress(progress, combo_params.len(), total_pairs, "rocb_optimize");
            }
        }
    }
    emit_progress(progress, combo_params.len(), total_pairs, "rocb_optimize");

    if combo_params.is_empty() {
        return Err("ROCB grid search produced no valid parameter combinations".to_string());
    }

    let signals = RocbGpuDeviceSignalMatrices {
        close_f32: prepared_cuda_inputs.close(),
        cols: n,
        upper_row_major_ptr: upper_sweep.matrix.device_ptr,
        lower_row_major_ptr: lower_sweep.matrix.device_ptr,
        channel_rows: upper_sweep.matrix.rows,
        channel_stride: upper_sweep.matrix.cols,
        atr_row_major_ptr: atr_sweep.matrix.device_ptr,
        atr_rows: atr_sweep.matrix.rows,
        atr_stride: atr_sweep.matrix.cols,
    };
    let combos = RocbGpuComboBatch {
        channel_row_index: &combo_channel_row_idx,
        atr_row_index: &combo_atr_row_idx,
        start_t: &combo_start_t,
    };

    let start = std::time::Instant::now();
    let metrics = eval_rocb_metrics_gpu_kernel_device_rows(
        device_id, &signals, &combos, &strategy, &rocb_cfg,
    )?;
    if metrics.len() != combo_params.len() {
        return Err("ROCB GPU kernel output length mismatch".to_string());
    }

    let resolve_params = |p: &DoubleMaParams| -> DoubleMaParamsResolved {
        DoubleMaParamsResolved {
            fast_len: p.fast_len,
            slow_len: p.slow_len,
            fast_ma_type: fast_ma.clone(),
            slow_ma_type: slow_ma.clone(),
            fast_ma_params: req.fast_ma_params.clone(),
            slow_ma_params: req.slow_ma_params.clone(),
        }
    };

    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    if heatmap_bins > 0 {
        agg = agg.with_heatmap(
            heatmap_bins,
            heatmap_bins,
            *fast_periods.iter().min().unwrap_or(&0),
            *fast_periods.iter().max().unwrap_or(&0),
            *slow_periods.iter().min().unwrap_or(&0),
            *slow_periods.iter().max().unwrap_or(&0),
        );
    }
    for (i, (params, m)) in combo_params
        .into_iter()
        .zip(metrics.into_iter())
        .enumerate()
    {
        agg.push(params, m);
        if (i & 0x03ff) == 0 {
            emit_progress(progress, i + 1, total_pairs, "rocb_optimize");
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "rocb_optimize");

    let result = agg
        .finalize()
        .ok_or_else(|| "ROCB grid search produced no result".to_string())?;
    let best_params = resolve_params(&result.best_params);
    let best_metrics = result.best_metrics.clone();
    let top: Vec<(DoubleMaParamsResolved, Metrics)> = result
        .top
        .iter()
        .map(|(p, m)| (resolve_params(p), m.clone()))
        .collect();
    let all = result.all.as_ref().map(|rows| {
        rows.iter()
            .map(|(p, m)| (resolve_params(p), m.clone()))
            .collect::<Vec<_>>()
    });
    let runtime_ms = start.elapsed().as_millis() as u64;

    let (matrix_index, best_col, top_cols) =
        match DoubleMaFastLtSlowIndex::new(fast_periods, slow_periods) {
            Ok(idx) => {
                let best_col = idx.col_id(best_params.fast_len, best_params.slow_len);
                let mut top_cols: Vec<u32> = Vec::with_capacity(top.len());
                for (p, _) in &top {
                    match idx.col_id(p.fast_len, p.slow_len) {
                        Some(c) => top_cols.push(c),
                        None => {
                            top_cols.clear();
                            break;
                        }
                    }
                }
                let top_cols = if top_cols.is_empty() {
                    None
                } else {
                    Some(top_cols)
                };
                (Some(idx), best_col, top_cols)
            }
            Err(_) => (None, None, None),
        };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics,
        mode_used,
        backend_used: BackendUsed::Gpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        all,
        heatmap: result.heatmap.clone(),
        vdps: None,
        hybrid_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
    })
}

fn run_rocb_optimization_cpu(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    run_rocb_optimization_impl(
        &req,
        candles,
        cancel,
        progress,
        &Backend::CpuOnly,
        BackendUsed::Cpu,
    )
}

fn run_rocb_optimization_impl(
    req: &CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    indicator_backend: &Backend,
    backend_used: BackendUsed,
) -> Result<BackendOptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());
    let rocb_cfg = req.rocb.clone().unwrap_or_default();
    validate_rocb_params(&rocb_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "ROCB")?;
    let objective = req.objective;
    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);

    if req.fast_ma_types.len() != 1 || req.slow_ma_types.len() != 1 {
        return Err("Select exactly one fast MA and one slow MA.".to_string());
    }
    let fast_ma = req.fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = req.slow_ma_types[0].trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("Fast/slow MA type cannot be empty.".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_periods = expand_u16_range(req.fast_range)?;
    let slow_periods = expand_u16_range(req.slow_range)?;
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty period range".to_string());
    }
    let total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);
    if total_pairs == 0 {
        return Err("no valid parameter combinations (fast must be < slow)".to_string());
    }
    if include_all && total_pairs > 2_000_000 {
        return Err(format!(
            "include_all=true would return {} rows; disable it or reduce ranges.",
            total_pairs
        ));
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let high = vector_ta::utilities::data_loader::source_type(candles, "high");
    let low = vector_ta::utilities::data_loader::source_type(candles, "low");
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_high = first_finite_idx(high).ok_or_else(|| "high series all non-finite".to_string())?;
    let fv_low = first_finite_idx(low).ok_or_else(|| "low series all non-finite".to_string())?;
    let lr = compute_log_returns_f64(close);

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if !matches!(indicator_backend, Backend::CpuOnly) {
        Some(
            PreparedCudaIndicatorInputs::from_candles_on_device(
                candles,
                "close",
                match indicator_backend {
                    Backend::Auto { device_id }
                    | Backend::GpuOnly { device_id }
                    | Backend::GpuHybridKernel { device_id }
                    | Backend::GpuKernel { device_id } => *device_id as usize,
                    Backend::CpuOnly => 0,
                },
            )
            .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

    let mut breakout_cache: HashMap<u16, crate::rocb::RocbBreakoutSignals> = HashMap::new();
    let mut risk_cache: HashMap<u16, crate::rocb::RocbRiskSignals> = HashMap::new();
    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    if heatmap_bins > 0 {
        agg = agg.with_heatmap(
            heatmap_bins,
            heatmap_bins,
            *fast_periods.iter().min().unwrap_or(&0),
            *fast_periods.iter().max().unwrap_or(&0),
            *slow_periods.iter().min().unwrap_or(&0),
            *slow_periods.iter().max().unwrap_or(&0),
        );
    }

    let mut processed = 0usize;
    emit_progress(progress, 0, total_pairs, "rocb_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_rocb_periods(&rocb_cfg, fast_len, slow_len);
            let breakout = if let Some(v) = breakout_cache.get(&periods.channel) {
                v
            } else {
                let built = compute_rocb_breakout_signals(
                    candles,
                    "close",
                    indicator_backend,
                    periods.channel,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                breakout_cache.insert(periods.channel, built);
                breakout_cache
                    .get(&periods.channel)
                    .ok_or_else(|| "ROCB breakout cache insert failed".to_string())?
            };
            let risk = if let Some(v) = risk_cache.get(&periods.atr) {
                v
            } else {
                let built = compute_rocb_risk_signals(
                    candles,
                    "close",
                    indicator_backend,
                    periods.atr,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                risk_cache.insert(periods.atr, built);
                risk_cache
                    .get(&periods.atr)
                    .ok_or_else(|| "ROCB risk cache insert failed".to_string())?
            };

            let start_t = compute_rocb_start_t(
                fv_close,
                fv_high,
                fv_low,
                breakout.first_valid,
                risk.first_valid,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            let desired =
                build_rocb_desired_positions(close, breakout, risk, start_t, &strategy, &rocb_cfg);
            let metrics = eval_asr_metrics_from_desired(candles, &lr, &desired, start_t, &strategy);
            agg.push(
                DoubleMaParams {
                    fast_len,
                    slow_len,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                metrics,
            );
            processed += 1;
            if (processed & 0x03ff) == 0 {
                emit_progress(progress, processed, total_pairs, "rocb_optimize");
            }
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "rocb_optimize");

    let result = agg
        .finalize()
        .ok_or_else(|| "ROCB grid search produced no result".to_string())?;
    let best_params = DoubleMaParamsResolved {
        fast_len: result.best_params.fast_len,
        slow_len: result.best_params.slow_len,
        fast_ma_type: fast_ma.clone(),
        slow_ma_type: slow_ma.clone(),
        fast_ma_params: req.fast_ma_params.clone(),
        slow_ma_params: req.slow_ma_params.clone(),
    };
    let top: Vec<(DoubleMaParamsResolved, Metrics)> = result
        .top
        .iter()
        .map(|(p, m)| {
            (
                DoubleMaParamsResolved {
                    fast_len: p.fast_len,
                    slow_len: p.slow_len,
                    fast_ma_type: fast_ma.clone(),
                    slow_ma_type: slow_ma.clone(),
                    fast_ma_params: req.fast_ma_params.clone(),
                    slow_ma_params: req.slow_ma_params.clone(),
                },
                m.clone(),
            )
        })
        .collect();
    let all = result.all.as_ref().map(|rows| {
        rows.iter()
            .map(|(p, m)| {
                (
                    DoubleMaParamsResolved {
                        fast_len: p.fast_len,
                        slow_len: p.slow_len,
                        fast_ma_type: fast_ma.clone(),
                        slow_ma_type: slow_ma.clone(),
                        fast_ma_params: req.fast_ma_params.clone(),
                        slow_ma_params: req.slow_ma_params.clone(),
                    },
                    m.clone(),
                )
            })
            .collect::<Vec<_>>()
    });

    let (matrix_index, best_col, top_cols) =
        match DoubleMaFastLtSlowIndex::new(fast_periods, slow_periods) {
            Ok(idx) => {
                let best_col = idx.col_id(best_params.fast_len, best_params.slow_len);
                let mut top_cols: Vec<u32> = Vec::with_capacity(top.len());
                for (p, _) in &top {
                    match idx.col_id(p.fast_len, p.slow_len) {
                        Some(c) => top_cols.push(c),
                        None => {
                            top_cols.clear();
                            break;
                        }
                    }
                }
                let top_cols = if top_cols.is_empty() {
                    None
                } else {
                    Some(top_cols)
                };
                (Some(idx), best_col, top_cols)
            }
            Err(_) => (None, None, None),
        };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        mode_used,
        backend_used,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms: 0,
        top,
        all,
        heatmap: result.heatmap.clone(),
        vdps: None,
        hybrid_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
    })
}

pub fn run_composite_cpu_optimization_blocking_with_candles(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    let custom_strategy_count = usize::from(req.asr.is_some())
        + usize::from(req.vcc.is_some())
        + usize::from(req.vemr.is_some())
        + usize::from(req.rocb.is_some());
    if custom_strategy_count > 1 {
        return Err("Select only one of ASR, VCC, VEMR, or ROCB strategy config.".to_string());
    }
    if req.asr.is_some() {
        return run_asr_optimization(req, candles, cancel, progress);
    }
    if req.vcc.is_some() {
        return run_vcc_optimization(req, candles, cancel, progress);
    }
    if req.vemr.is_some() {
        return run_vemr_optimization(req, candles, cancel, progress);
    }
    if req.rocb.is_some() {
        return run_rocb_optimization(req, candles, cancel, progress);
    }

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    let strategy = strategy_cfg_for_optimization(req.strategy.clone());

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;

    let mode_used = resolve_composite_grid_mode(&req.mode, "Composite")?;

    let objective = req.objective;
    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);

    if req.fast_ma_types.len() != 1 || req.slow_ma_types.len() != 1 {
        return Err("Select exactly one fast MA and one slow MA.".to_string());
    }

    let fast_ma = req.fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = req.slow_ma_types[0].trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("Fast/slow MA type cannot be empty.".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", &fast_ma, req.fast_ma_params.as_ref())?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", &slow_ma, req.slow_ma_params.as_ref())?;

    let mut fast_periods = expand_u16_range(req.fast_range)?;
    let mut slow_periods = expand_u16_range(req.slow_range)?;
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty period range".to_string());
    }
    let mut total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);
    if total_pairs == 0 {
        return Err("no valid parameter combinations (fast must be < slow)".to_string());
    }

    validate_macz_params(&req.macz)?;
    validate_ott_params(&req.ott)?;
    if req.macz.use_volume && !candles.fields.volume {
        return Err("MACZ with volume requires a CSV with volume column.".to_string());
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    // Optional stage-0 prescan: use the fast double-MA optimizer (GPU-kernel if available) to
    // narrow the (fast_len, slow_len) search space for the composite grid search.
    const AUTO_PRESCAN_MIN_PAIRS: usize = 250_000;
    const AUTO_PRESCAN_TOP_K: usize = 128;
    const AUTO_PRESCAN_WINDOW: u16 = 8;
    const AUTO_PRESCAN_MAX_PERIODS: usize = 256;

    let mut allowed_pairs: Option<HashSet<(u16, u16)>> = None;
    if req.gpu_prescan.is_some() || total_pairs > AUTO_PRESCAN_MIN_PAIRS {
        let cfg = req.gpu_prescan.as_ref();
        let prescan_top_k = cfg.map(|c| c.top_k).unwrap_or(AUTO_PRESCAN_TOP_K);
        let prescan_window = cfg.map(|c| c.window).unwrap_or(AUTO_PRESCAN_WINDOW);
        let predictive_top_k = cfg.map(|c| c.predictive_top_k).unwrap_or(0);
        let predictive_stride = cfg.map(|c| c.predictive_stride).unwrap_or(8);
        let device_id = cfg.and_then(|c| c.device_id).unwrap_or(0);
        let require_gpu = cfg.map(|c| c.require_gpu).unwrap_or(false);
        let prescan_mode = cfg.map(|c| c.mode).unwrap_or(OptimizationMode::Auto);

        let dm_req = DoubleMaRequest {
            backend: Backend::Auto { device_id },
            data_id: req.data_id.clone(),
            fast_range: req.fast_range,
            slow_range: req.slow_range,
            fast_ma_types: vec![fast_ma.clone()],
            slow_ma_types: vec![slow_ma.clone()],
            ma_source: req.ma_source.clone(),
            export_all_csv_path: None,
            fast_ma_params: req.fast_ma_params.clone(),
            slow_ma_params: req.slow_ma_params.clone(),
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: strategy.clone(),
            halftrend_filter: None,
            objective,
            mode: prescan_mode,
            top_k: Some(prescan_top_k),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
        };

        let dm_res = run_double_ma_optimization_blocking_with_candles(
            dm_req, candles, cancel,
            // Forward progress so the UI shows the prescan phase too.
            progress,
        )?;
        if require_gpu && matches!(dm_res.backend_used, BackendUsed::Cpu) {
            return Err(
                "gpu_prescan require_gpu=true but GPU backend was not available (CPU used)"
                    .to_string(),
            );
        }

        let mut pairs: Vec<(u16, u16)> = Vec::with_capacity(dm_res.top.len().saturating_add(1));
        pairs.push((dm_res.best_params.fast_len, dm_res.best_params.slow_len));
        for (p, _) in &dm_res.top {
            pairs.push((p.fast_len, p.slow_len));
        }
        if predictive_top_k > 0 {
            let rescue = predictive_prescan_seed_pairs(PredictivePrescanParams {
                candles,
                ma_source: &req.ma_source,
                fast_periods: &fast_periods,
                slow_periods: &slow_periods,
                fast_ma_type: &fast_ma,
                slow_ma_type: &slow_ma,
                fast_ma_params: req.fast_ma_params.as_ref(),
                slow_ma_params: req.slow_ma_params.as_ref(),
                fast_alma_offset,
                fast_alma_sigma,
                slow_alma_offset,
                slow_alma_sigma,
                long_only: strategy.long_only,
                top_k: predictive_top_k,
                period_stride: predictive_stride,
            })?;
            pairs.extend(rescue);
        }
        pairs.sort();
        pairs.dedup();

        let mut fast_set: BTreeSet<u16> = BTreeSet::new();
        let mut slow_set: BTreeSet<u16> = BTreeSet::new();
        let mut pair_set: HashSet<(u16, u16)> = HashSet::new();

        for (f, s) in pairs {
            let fast_window = period_window_slice(&fast_periods, f, prescan_window);
            let slow_window = period_window_slice(&slow_periods, s, prescan_window);
            for &fast in fast_window {
                for &slow in slow_window {
                    if fast >= slow {
                        continue;
                    }
                    fast_set.insert(fast);
                    slow_set.insert(slow);
                    pair_set.insert((fast, slow));
                }
            }
        }

        let best_fast = dm_res.best_params.fast_len;
        let best_slow = dm_res.best_params.slow_len;

        let mut fast_candidates: Vec<u16> = fast_set.into_iter().collect();
        let mut slow_candidates: Vec<u16> = slow_set.into_iter().collect();

        // Cap candidate set sizes to keep memory use and total work bounded.
        if fast_candidates.len() > AUTO_PRESCAN_MAX_PERIODS {
            fast_candidates.sort_by_key(|&p| (p.abs_diff(best_fast), p));
            fast_candidates.truncate(AUTO_PRESCAN_MAX_PERIODS);
            fast_candidates.sort_unstable();
        }
        if slow_candidates.len() > AUTO_PRESCAN_MAX_PERIODS {
            slow_candidates.sort_by_key(|&p| (p.abs_diff(best_slow), p));
            slow_candidates.truncate(AUTO_PRESCAN_MAX_PERIODS);
            slow_candidates.sort_unstable();
        }

        if fast_candidates.is_empty() || slow_candidates.is_empty() {
            return Err("gpu_prescan produced empty candidate set".to_string());
        }

        let fast_allowed: HashSet<u16> = fast_candidates.iter().copied().collect();
        let slow_allowed: HashSet<u16> = slow_candidates.iter().copied().collect();
        pair_set.retain(|(f, s)| fast_allowed.contains(f) && slow_allowed.contains(s) && f < s);
        if pair_set.is_empty() {
            return Err("gpu_prescan produced no valid parameter combinations".to_string());
        }

        fast_periods = fast_candidates;
        slow_periods = slow_candidates;
        total_pairs = pair_set.len();
        allowed_pairs = Some(pair_set);
    }

    let ma_source = req.ma_source;
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &ma_source);
    if ma_prices.len() != close.len() {
        return Err("ma source length mismatch".to_string());
    }

    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let mut first_valid = fv_close.max(fv_ma);

    let halftrend_trend = compute_halftrend_trend(candles, &req.halftrend)?;
    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }
    let fv_ht = first_finite_idx(&halftrend_trend)
        .ok_or_else(|| "halftrend trend series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_ht);

    let macz_hist = compute_macz_hist(candles, &ma_source, &req.macz)?;
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }
    let fv_macz =
        first_finite_idx(&macz_hist).ok_or_else(|| "macz series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_macz);

    let ott_line = compute_ott_line(candles, &ma_source, &req.ott)?;
    if ott_line.len() != n {
        return Err("OTT series length mismatch".to_string());
    }
    let fv_ott =
        first_finite_idx(&ott_line).ok_or_else(|| "ott series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_ott);

    let start = std::time::Instant::now();
    if include_all && total_pairs > 2_000_000 {
        return Err(format!(
            "include_all=true would return {} rows; disable it or reduce ranges.",
            total_pairs
        ));
    }

    let result = optimize_composite_cpu_tiled(
        candles,
        &fast_periods,
        &slow_periods,
        &fast_ma,
        &slow_ma,
        &ma_source,
        req.fast_ma_params.as_ref(),
        req.slow_ma_params.as_ref(),
        &req.halftrend,
        &req.macz,
        &req.ott,
        &halftrend_trend,
        &macz_hist,
        ma_prices,
        &ott_line,
        first_valid,
        &strategy,
        req.signal_mode,
        objective,
        top_k,
        include_all,
        fast_alma_offset,
        fast_alma_sigma,
        slow_alma_offset,
        slow_alma_sigma,
        total_pairs,
        allowed_pairs.as_ref(),
        heatmap_bins,
        cancel,
        progress,
    )?;

    let runtime_ms = start.elapsed().as_millis() as u64;

    let resolve_params = |p: &DoubleMaParams| -> DoubleMaParamsResolved {
        DoubleMaParamsResolved {
            fast_len: p.fast_len,
            slow_len: p.slow_len,
            fast_ma_type: fast_ma.clone(),
            slow_ma_type: slow_ma.clone(),
            fast_ma_params: None,
            slow_ma_params: None,
        }
    };

    let best_params = resolve_params(&result.best_params);
    let top: Vec<(DoubleMaParamsResolved, Metrics)> = result
        .top
        .iter()
        .map(|(p, m)| (resolve_params(p), m.clone()))
        .collect();
    let all = result.all.as_ref().map(|rows| {
        rows.iter()
            .map(|(p, m)| (resolve_params(p), m.clone()))
            .collect::<Vec<_>>()
    });
    let heatmap = result.heatmap.clone();

    let (matrix_index, best_col, top_cols) =
        match DoubleMaFastLtSlowIndex::new(fast_periods, slow_periods) {
            Ok(idx) => {
                let best_col = idx.col_id(best_params.fast_len, best_params.slow_len);
                let mut top_cols: Vec<u32> = Vec::with_capacity(top.len());
                for (p, _) in &top {
                    match idx.col_id(p.fast_len, p.slow_len) {
                        Some(c) => top_cols.push(c),
                        None => {
                            top_cols.clear();
                            break;
                        }
                    }
                }
                let top_cols = if top_cols.is_empty() {
                    None
                } else {
                    Some(top_cols)
                };
                (Some(idx), best_col, top_cols)
            }
            Err(_) => (None, None, None),
        };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        mode_used,
        backend_used: BackendUsed::Cpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
    })
}

pub fn compute_composite_cpu_drilldown_blocking_with_candles(
    req: CompositeCpuDrilldownRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<CompositeCpuDrilldownResult, String> {
    let custom_strategy_count = usize::from(req.asr.is_some())
        + usize::from(req.vcc.is_some())
        + usize::from(req.vemr.is_some())
        + usize::from(req.rocb.is_some());
    if custom_strategy_count > 1 {
        return Err("Select only one of ASR, VCC, VEMR, or ROCB strategy config.".to_string());
    }
    if req.vemr.is_some() {
        return run_vemr_drilldown(req, candles);
    }
    if req.vcc.is_some() {
        return Err("VCC drilldown is not implemented yet.".to_string());
    }
    if req.asr.is_some() {
        return run_asr_drilldown(req, candles);
    }
    if req.rocb.is_some() {
        return Err("ROCB drilldown is not implemented yet.".to_string());
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_ma = req.params.fast_ma_type.trim();
    let slow_ma = req.params.slow_ma_type.trim();
    validate_ma_kind(fast_ma)?;
    validate_ma_kind(slow_ma)?;

    let fast_len = req.params.fast_len as usize;
    let slow_len = req.params.slow_len as usize;
    if fast_len == 0 || slow_len == 0 {
        return Err("periods must be > 0".to_string());
    }
    if fast_len > n || slow_len > n {
        return Err(format!(
            "period exceeds series length: fast_len={fast_len} slow_len={slow_len} n={n}"
        ));
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    validate_macz_params(&req.macz)?;
    validate_ott_params(&req.ott)?;
    if req.macz.use_volume && !candles.fields.volume {
        return Err("MACZ with volume requires a CSV with volume column.".to_string());
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
    let mut first_valid = fv_close.max(fv_ma);

    let halftrend_trend = compute_halftrend_trend(candles, &req.halftrend)?;
    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }
    let fv_ht = first_finite_idx(&halftrend_trend)
        .ok_or_else(|| "halftrend trend series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_ht);

    let macz_hist = compute_macz_hist(candles, &req.ma_source, &req.macz)?;
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }
    let fv_macz =
        first_finite_idx(&macz_hist).ok_or_else(|| "macz series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_macz);

    let ott_line = compute_ott_line(candles, &req.ma_source, &req.ott)?;
    if ott_line.len() != n {
        return Err("OTT series length mismatch".to_string());
    }
    let fv_ott =
        first_finite_idx(&ott_line).ok_or_else(|| "ott series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_ott);

    let trade_offset = if req.strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let max_p = fast_len.max(slow_len).max(1);
    let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
    let start_t = t_valid.saturating_add(trade_offset);
    if start_t >= n {
        return Err("not enough candles after warmup for analysis".to_string());
    }

    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", fast_ma, req.fast_ma_params.as_ref())?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", slow_ma, req.slow_ma_params.as_ref())?;

    let fast_periods = [req.params.fast_len];
    let (fast_mat, cols) = ma_matrix_cpu_f64(
        candles,
        fast_ma,
        &req.ma_source,
        &fast_periods,
        req.fast_ma_params.as_ref(),
        fast_alma_offset,
        fast_alma_sigma,
    )?;
    let slow_periods = [req.params.slow_len];
    let (slow_mat, cols2) = ma_matrix_cpu_f64(
        candles,
        slow_ma,
        &req.ma_source,
        &slow_periods,
        req.slow_ma_params.as_ref(),
        slow_alma_offset,
        slow_alma_sigma,
    )?;
    if cols2 != cols {
        return Err("MA matrix column mismatch".to_string());
    }

    let lr = compute_log_returns_f64(close);
    let bins = req.bins.unwrap_or(1024);
    let mut curves =
        ta_strategies::composite_cpu::eval_composite_curves_from_log_returns_and_mas_f64_with_mode(
            &lr,
            ma_prices,
            &fast_mat,
            &slow_mat,
            &halftrend_trend,
            &macz_hist,
            &ott_line,
            start_t,
            &req.strategy,
            bins,
            req.signal_mode,
        );
    curves.indicator_bars = build_indicator_bars_for_chart(candles, &fast_mat, &slow_mat, 8_000)?;
    let composite_indicator_bars = build_composite_indicator_bars_for_chart(
        candles,
        &halftrend_trend,
        &macz_hist,
        ma_prices,
        &ott_line,
        8_000,
    )?;
    Ok(CompositeCpuDrilldownResult {
        curves,
        composite_indicator_bars,
    })
}

fn build_composite_indicator_bars_for_chart(
    candles: &vector_ta::utilities::data_loader::Candles,
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_price: &[f64],
    ott_line: &[f64],
    max_points: usize,
) -> Result<Vec<CompositeIndicatorBar>, String> {
    let n = candles.timestamp.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    if halftrend_trend.len() != n
        || macz_hist.len() != n
        || ott_price.len() != n
        || ott_line.len() != n
    {
        return Err("composite indicator series length mismatch".to_string());
    }

    let cap = max_points.clamp(300, 20_000);
    let step = if n <= cap { 1usize } else { n.div_ceil(cap) };
    let mut out: Vec<CompositeIndicatorBar> = Vec::with_capacity(n.div_ceil(step));
    let mut prev_time: Option<i64> = None;

    let mut from = 0usize;
    while from < n {
        let to_exclusive = n.min(from.saturating_add(step));
        let to = to_exclusive.saturating_sub(1);
        let time = candles.timestamp[from];
        if prev_time.is_some_and(|p| time <= p) {
            from = to_exclusive;
            continue;
        }

        let halftrend = (from..to_exclusive).rev().find_map(|i| {
            let v = halftrend_trend[i];
            if v.is_finite() {
                Some(v)
            } else {
                None
            }
        });
        let macz_hist = (from..to_exclusive).rev().find_map(|i| {
            let v = macz_hist[i];
            if v.is_finite() {
                Some(v)
            } else {
                None
            }
        });
        let ott_line = (from..to_exclusive).rev().find_map(|i| {
            let v = ott_line[i];
            if v.is_finite() {
                Some(v)
            } else {
                None
            }
        });
        let ott_price = (from..to_exclusive).rev().find_map(|i| {
            let v = ott_price[i];
            if v.is_finite() {
                Some(v)
            } else {
                None
            }
        });

        if halftrend.is_none() && macz_hist.is_none() && ott_line.is_none() && ott_price.is_none() {
            from = to_exclusive;
            continue;
        }

        out.push(CompositeIndicatorBar {
            time,
            from_index: u32::try_from(from)
                .map_err(|_| "dataset too large for indicator index".to_string())?,
            to_index: u32::try_from(to)
                .map_err(|_| "dataset too large for indicator index".to_string())?,
            halftrend,
            macz_hist,
            ott_line,
            ott_price,
        });
        prev_time = Some(time);
        from = to_exclusive;
    }

    Ok(out)
}

pub fn compute_composite_cpu_local_stability_blocking_with_candles(
    req: CompositeCpuLocalStabilityRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<LocalStabilityResult, String> {
    let custom_strategy_count = usize::from(req.asr.is_some())
        + usize::from(req.vcc.is_some())
        + usize::from(req.vemr.is_some())
        + usize::from(req.rocb.is_some());
    if custom_strategy_count > 1 {
        return Err("Select only one of ASR, VCC, VEMR, or ROCB strategy config.".to_string());
    }
    if req.vemr.is_some() {
        return run_vemr_local_stability(req, candles);
    }
    if req.vcc.is_some() {
        return Err("VCC local stability is not implemented yet.".to_string());
    }
    if req.asr.is_some() {
        return run_asr_local_stability(req, candles);
    }
    if req.rocb.is_some() {
        return Err("ROCB local stability is not implemented yet.".to_string());
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_ma = req.params.fast_ma_type.trim();
    let slow_ma = req.params.slow_ma_type.trim();
    validate_ma_kind(fast_ma)?;
    validate_ma_kind(slow_ma)?;

    let base_fast = req.params.fast_len;
    let base_slow = req.params.slow_len;
    if base_fast == 0 || base_slow == 0 {
        return Err("periods must be > 0".to_string());
    }
    if base_fast >= base_slow {
        return Err("invalid params: fast_len must be < slow_len".to_string());
    }
    if base_fast as usize > n || base_slow as usize > n {
        return Err(format!(
            "period exceeds series length: fast_len={base_fast} slow_len={base_slow} n={n}"
        ));
    }

    validate_strategy_cfg_for_optimization(&req.strategy)?;
    validate_macz_params(&req.macz)?;
    validate_ott_params(&req.ott)?;
    if req.macz.use_volume && !candles.fields.volume {
        return Err("MACZ with volume requires a CSV with volume column.".to_string());
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
    let mut first_valid = fv_close.max(fv_ma);

    let halftrend_trend = compute_halftrend_trend(candles, &req.halftrend)?;
    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }
    let fv_ht = first_finite_idx(&halftrend_trend)
        .ok_or_else(|| "halftrend trend series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_ht);

    let macz_hist = compute_macz_hist(candles, &req.ma_source, &req.macz)?;
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }
    let fv_macz =
        first_finite_idx(&macz_hist).ok_or_else(|| "macz series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_macz);

    let ott_line = compute_ott_line(candles, &req.ma_source, &req.ott)?;
    if ott_line.len() != n {
        return Err("OTT series length mismatch".to_string());
    }
    let fv_ott =
        first_finite_idx(&ott_line).ok_or_else(|| "ott series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_ott);

    let radius = req.radius.unwrap_or(1).min(32);
    let step = req.step.unwrap_or(1).max(1);

    let mut fast_periods: Vec<u16> = Vec::new();
    let mut df = -(radius as i32);
    while df <= radius as i32 {
        let v = base_fast as i32 + df;
        if v > 0 {
            fast_periods.push(v as u16);
        }
        df += step as i32;
    }
    let mut slow_periods: Vec<u16> = Vec::new();
    let mut ds = -(radius as i32);
    while ds <= radius as i32 {
        let v = base_slow as i32 + ds;
        if v > 0 {
            slow_periods.push(v as u16);
        }
        ds += step as i32;
    }

    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty neighborhood".to_string());
    }
    fast_periods.sort_unstable();
    slow_periods.sort_unstable();

    // Use a single start_t based on the maximum period in the neighborhood so all points are comparable.
    let trade_offset = if req.strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let max_p = (fast_periods[fast_periods.len() - 1] as usize)
        .max(slow_periods[slow_periods.len() - 1] as usize)
        .max(1);
    let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
    let start_t = t_valid.saturating_add(trade_offset);
    if start_t >= n {
        return Err("not enough candles after warmup for analysis".to_string());
    }

    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", fast_ma, req.fast_ma_params.as_ref())?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", slow_ma, req.slow_ma_params.as_ref())?;

    let (fast_mat, cols) = ma_matrix_cpu_f64(
        candles,
        fast_ma,
        &req.ma_source,
        &fast_periods,
        req.fast_ma_params.as_ref(),
        fast_alma_offset,
        fast_alma_sigma,
    )?;
    if cols != n {
        return Err("fast MA series length mismatch".to_string());
    }
    let (slow_mat, cols2) = ma_matrix_cpu_f64(
        candles,
        slow_ma,
        &req.ma_source,
        &slow_periods,
        req.slow_ma_params.as_ref(),
        slow_alma_offset,
        slow_alma_sigma,
    )?;
    if cols2 != n {
        return Err("slow MA series length mismatch".to_string());
    }

    let lr = compute_log_returns_f64(close);
    let objective = req.objective;

    let mut points: Vec<LocalStabilityPoint> = Vec::new();
    for (fi, &fast_len) in fast_periods.iter().enumerate() {
        let fast_off = fi.saturating_mul(n);
        let fast = fast_mat
            .get(fast_off..fast_off + n)
            .ok_or_else(|| "fast MA matrix indexing error".to_string())?;

        for (si, &slow_len) in slow_periods.iter().enumerate() {
            if fast_len >= slow_len {
                continue;
            }
            let slow_off = si.saturating_mul(n);
            let slow = slow_mat
                .get(slow_off..slow_off + n)
                .ok_or_else(|| "slow MA matrix indexing error".to_string())?;

            let metrics =
                ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                    &lr,
                    ma_prices,
                    fast,
                    slow,
                    &halftrend_trend,
                    &macz_hist,
                    &ott_line,
                    start_t,
                    &req.strategy,
                    req.signal_mode,
                );
            let score = objective_score(&metrics, objective);
            points.push(LocalStabilityPoint {
                fast_len,
                slow_len,
                metrics,
                score,
            });
        }
    }

    let base_score = points
        .iter()
        .find(|p| p.fast_len == base_fast && p.slow_len == base_slow)
        .map(|p| p.score)
        .unwrap_or(0.0);

    let mut scores: Vec<f64> = points.iter().map(|p| p.score).collect();
    scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let min = *scores.first().unwrap_or(&0.0);
    let max = *scores.last().unwrap_or(&0.0);
    let median = if scores.is_empty() {
        0.0
    } else {
        scores[scores.len() / 2]
    };

    Ok(LocalStabilityResult {
        base: DoubleMaParamsResolved {
            fast_len: base_fast,
            slow_len: base_slow,
            fast_ma_type: req.params.fast_ma_type,
            slow_ma_type: req.params.slow_ma_type,
            fast_ma_params: req.fast_ma_params,
            slow_ma_params: req.slow_ma_params,
        },
        objective,
        radius,
        step,
        start_t,
        points,
        summary: LocalStabilitySummary {
            base: base_score,
            min,
            median,
            max,
        },
    })
}

fn validate_macz_params(p: &MaczGateParams) -> Result<(), String> {
    if p.fast_length == 0
        || p.slow_length == 0
        || p.signal_length == 0
        || p.lengthz == 0
        || p.length_stdev == 0
    {
        return Err("MACZ lengths must be > 0".to_string());
    }
    if p.fast_length >= p.slow_length {
        return Err("MACZ requires fast_length < slow_length".to_string());
    }
    if !(-2.0..=2.0).contains(&p.a) || !p.a.is_finite() {
        return Err("MACZ a must be finite and in [-2, 2]".to_string());
    }
    if !(-2.0..=2.0).contains(&p.b) || !p.b.is_finite() {
        return Err("MACZ b must be finite and in [-2, 2]".to_string());
    }
    if !(0.0..1.0).contains(&p.gamma) {
        return Err("MACZ gamma must be in [0, 1)".to_string());
    }
    Ok(())
}

pub(crate) fn compute_macz_hist(
    candles: &vector_ta::utilities::data_loader::Candles,
    source: &str,
    p: &MaczGateParams,
) -> Result<Vec<f64>, String> {
    validate_macz_params(p)?;
    let data = vector_ta::utilities::data_loader::source_type(candles, source);

    let params = vector_ta::indicators::macz::MaczParams {
        fast_length: Some(p.fast_length as usize),
        slow_length: Some(p.slow_length as usize),
        signal_length: Some(p.signal_length as usize),
        lengthz: Some(p.lengthz as usize),
        length_stdev: Some(p.length_stdev as usize),
        a: Some(p.a),
        b: Some(p.b),
        use_lag: Some(p.use_lag),
        gamma: Some(p.gamma),
    };

    let input = if p.use_volume && candles.fields.volume {
        vector_ta::indicators::macz::MaczInput::from_slice_with_volume(
            data,
            &candles.volume,
            params,
        )
    } else {
        vector_ta::indicators::macz::MaczInput::from_slice(data, params)
    };

    let out = vector_ta::indicators::macz::macz(&input).map_err(|e| e.to_string())?;
    Ok(out.values)
}

fn validate_ott_params(p: &OttGateParams) -> Result<(), String> {
    if p.period == 0 {
        return Err("OTT period must be > 0".to_string());
    }
    if !p.percent.is_finite() || p.percent < 0.0 {
        return Err("OTT percent must be finite and >= 0".to_string());
    }
    if p.ma_type.trim().is_empty() {
        return Err("OTT ma_type cannot be empty".to_string());
    }
    Ok(())
}

pub(crate) fn compute_ott_line(
    candles: &vector_ta::utilities::data_loader::Candles,
    source: &str,
    p: &OttGateParams,
) -> Result<Vec<f64>, String> {
    validate_ott_params(p)?;
    let data = vector_ta::utilities::data_loader::source_type(candles, source);
    let input = vector_ta::indicators::ott::OttInput::from_slice(
        data,
        vector_ta::indicators::ott::OttParams {
            period: Some(p.period as usize),
            percent: Some(p.percent),
            ma_type: Some(p.ma_type.clone()),
        },
    );
    let out = vector_ta::indicators::ott::ott(&input).map_err(|e| e.to_string())?;
    Ok(out.values)
}

fn emit_progress(
    progress: Option<&dyn ProgressSink>,
    processed_pairs: usize,
    total_pairs: usize,
    phase: &'static str,
) {
    crate::progress::emit_double_ma_progress(progress, processed_pairs, total_pairs, phase);
}

#[allow(clippy::too_many_arguments)]
fn optimize_composite_cpu_tiled(
    candles: &vector_ta::utilities::data_loader::Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    halftrend_cfg: &HalfTrendRegimeFilter,
    macz_cfg: &MaczGateParams,
    ott_cfg: &OttGateParams,
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_prices: &[f64],
    ott_line: &[f64],
    first_valid: usize,
    strategy: &StrategyConfig,
    signal_mode: CompositeSignalMode,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    total_pairs: usize,
    allowed_pairs: Option<&HashSet<(u16, u16)>>,
    heatmap_bins: usize,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    let run_start = std::time::Instant::now();
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_info = lookup_ma_info(fast_ma_type).ok_or_else(|| "unknown fast MA".to_string())?;
    if !fast_info.supports_cpu_single {
        return Err(format!(
            "CPU backend does not support MA '{}'",
            fast_ma_type
        ));
    }
    let slow_info = lookup_ma_info(slow_ma_type).ok_or_else(|| "unknown slow MA".to_string())?;
    if !slow_info.supports_cpu_single {
        return Err(format!(
            "CPU backend does not support MA '{}'",
            slow_ma_type
        ));
    }

    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }
    if ott_prices.len() != n || ott_line.len() != n {
        return Err("OTT series length mismatch".to_string());
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let lr = compute_log_returns_f64(close);
    let trade_offset = if strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };

    let dynamic_plan_ctx = if crate::strategy_dynamic::dynamic_strategy_opt_experimental_enabled() {
        let plan = crate::strategy_dynamic::compile_dynamic_plan_for_composite_cpu_config(
            ma_source,
            fast_ma_type,
            slow_ma_type,
            fast_ma_params,
            slow_ma_params,
            strategy,
            halftrend_cfg,
            macz_cfg,
            ott_cfg,
            signal_mode,
            crate::strategy_plan::StrategyPlanBackend::Cpu,
        )?;
        let ctx = crate::strategy_dynamic::build_dynamic_cpu_context_from_precomputed(
            &lr,
            close,
            ott_prices,
            None,
            Some(halftrend_trend),
            Some(macz_hist),
            Some(ott_line),
            None,
        );
        Some((plan, ctx))
    } else {
        None
    };
    let dynamic_shadow_enabled = dynamic_plan_ctx.is_some()
        && crate::strategy_dynamic::dynamic_strategy_opt_shadow_enabled();
    let dynamic_shadow_every = if dynamic_shadow_enabled {
        crate::strategy_dynamic::dynamic_strategy_opt_shadow_every().max(1)
    } else {
        0
    };
    let dynamic_shadow_sampled = AtomicUsize::new(0);
    let dynamic_shadow_mismatches = AtomicUsize::new(0);

    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    let heatmap_cfg = if heatmap_bins > 0 {
        Some((
            *fast_periods.iter().min().unwrap_or(&0),
            *fast_periods.iter().max().unwrap_or(&0),
            *slow_periods.iter().min().unwrap_or(&0),
            *slow_periods.iter().max().unwrap_or(&0),
        ))
    } else {
        None
    };
    if heatmap_bins > 0 {
        let (f_min, f_max, s_min, s_max) = heatmap_cfg.unwrap();
        agg = agg.with_heatmap(heatmap_bins, heatmap_bins, f_min, f_max, s_min, s_max);
    }

    let (pf_tile, ps_tile) = cpu_tile_sizes(fast_periods.len(), slow_periods.len(), n);
    let mut processed = 0usize;
    emit_progress(progress, processed, total_pairs, "composite_cpu");

    for f_chunk in fast_periods.chunks(pf_tile) {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let (fast_mat, cols) = ma_matrix_cpu_f64(
            candles,
            fast_ma_type,
            ma_source,
            f_chunk,
            fast_ma_params,
            fast_alma_offset,
            fast_alma_sigma,
        )?;
        for s_chunk in slow_periods.chunks(ps_tile) {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }
            let (slow_mat, cols2) = ma_matrix_cpu_f64(
                candles,
                slow_ma_type,
                ma_source,
                s_chunk,
                slow_ma_params,
                slow_alma_offset,
                slow_alma_sigma,
            )?;
            if cols2 != cols {
                return Err("MA matrix column mismatch".to_string());
            }

            if include_all {
                for (i, &fast_len) in f_chunk.iter().enumerate() {
                    let fast_row = &fast_mat[i * cols..(i + 1) * cols];
                    for (j, &slow_len) in s_chunk.iter().enumerate() {
                        if fast_len >= slow_len {
                            continue;
                        }
                        if allowed_pairs.is_some_and(|pairs| !pairs.contains(&(fast_len, slow_len)))
                        {
                            continue;
                        }
                        let slow_row = &slow_mat[j * cols..(j + 1) * cols];
                        let max_p = (fast_len as usize).max(slow_len as usize).max(1);
                        let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                        let start_t = t_valid.saturating_add(trade_offset);
                        let metrics = if let Some((plan, ctx)) = dynamic_plan_ctx.as_ref() {
                            let dynamic_metrics =
                                crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu(
                                    plan, ctx, fast_row, slow_row, start_t, true,
                                )?;
                            if dynamic_shadow_enabled {
                                let sampled = dynamic_shadow_sampled
                                    .fetch_add(1, Ordering::Relaxed)
                                    .saturating_add(1);
                                if sampled % dynamic_shadow_every == 0 {
                                    let static_metrics = ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                                        &lr,
                                        ott_prices,
                                        fast_row,
                                        slow_row,
                                        halftrend_trend,
                                        macz_hist,
                                        ott_line,
                                        start_t,
                                        strategy,
                                        signal_mode,
                                    );
                                    if !crate::strategy_dynamic::metrics_shadow_equivalent(
                                        &dynamic_metrics,
                                        &static_metrics,
                                    ) {
                                        dynamic_shadow_mismatches.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                            dynamic_metrics
                        } else {
                            ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                                &lr,
                                ott_prices,
                                fast_row,
                                slow_row,
                                halftrend_trend,
                                macz_hist,
                                ott_line,
                                start_t,
                                strategy,
                                signal_mode,
                            )
                        };
                        agg.push(
                            DoubleMaParams {
                                fast_len,
                                slow_len,
                                fast_ma_id: 0,
                                slow_ma_id: 0,
                            },
                            metrics,
                        );
                        processed = processed.saturating_add(1);
                        if (processed & 0x1fff) == 0 && cancel.load(Ordering::Relaxed) {
                            return Err("cancelled".to_string());
                        }
                    }
                }
            } else {
                let compute_sharpe = objective_needs_return_stats(objective);
                let pf = f_chunk.len();
                let ps = s_chunk.len();
                let tile_pairs = pf
                    .checked_mul(ps)
                    .ok_or_else(|| "tile pairs overflow".to_string())?;

                let init = || {
                    let mut local = StreamAggregator::new(objective, top_k, false, n);
                    if let Some((f_min, f_max, s_min, s_max)) = heatmap_cfg {
                        local = local.with_heatmap(
                            heatmap_bins,
                            heatmap_bins,
                            f_min,
                            f_max,
                            s_min,
                            s_max,
                        );
                    }
                    local
                };

                let tile = (0..tile_pairs)
                    .into_par_iter()
                    .try_fold(init, |mut local, idx| {
                        if cancel.load(Ordering::Relaxed) {
                            return Err("cancelled".to_string());
                        }
                        let i = idx / ps;
                        let j = idx - i * ps;
                        let fast_len = f_chunk[i];
                        let slow_len = s_chunk[j];
                        if fast_len >= slow_len {
                            return Ok(local);
                        }
                        if allowed_pairs.is_some_and(|pairs| !pairs.contains(&(fast_len, slow_len)))
                        {
                            return Ok(local);
                        }

                        let fast_row = &fast_mat[i * cols..(i + 1) * cols];
                        let slow_row = &slow_mat[j * cols..(j + 1) * cols];

                        let max_p = (fast_len as usize).max(slow_len as usize).max(1);
                        let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                        let start_t = t_valid.saturating_add(trade_offset);
                        let metrics = if let Some((plan, ctx)) = dynamic_plan_ctx.as_ref() {
                            let dynamic_metrics =
                                crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu(
                                    plan,
                                    ctx,
                                    fast_row,
                                    slow_row,
                                    start_t,
                                    compute_sharpe,
                                )?;
                            if dynamic_shadow_enabled {
                                let sampled = dynamic_shadow_sampled
                                    .fetch_add(1, Ordering::Relaxed)
                                    .saturating_add(1);
                                if sampled % dynamic_shadow_every == 0 {
                                    let static_metrics = if compute_sharpe {
                                        ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                                            &lr,
                                            ott_prices,
                                            fast_row,
                                            slow_row,
                                            halftrend_trend,
                                            macz_hist,
                                            ott_line,
                                            start_t,
                                            strategy,
                                            signal_mode,
                                        )
                                    } else {
                                        ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_fast_with_mode(
                                            &lr,
                                            ott_prices,
                                            fast_row,
                                            slow_row,
                                            halftrend_trend,
                                            macz_hist,
                                            ott_line,
                                            start_t,
                                            strategy,
                                            signal_mode,
                                        )
                                    };
                                    if !crate::strategy_dynamic::metrics_shadow_equivalent(
                                        &dynamic_metrics,
                                        &static_metrics,
                                    ) {
                                        dynamic_shadow_mismatches.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                            dynamic_metrics
                        } else {
                            if compute_sharpe {
                                ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                                    &lr,
                                    ott_prices,
                                    fast_row,
                                    slow_row,
                                    halftrend_trend,
                                    macz_hist,
                                    ott_line,
                                    start_t,
                                    strategy,
                                    signal_mode,
                                )
                            } else {
                                ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_fast_with_mode(
                                    &lr,
                                    ott_prices,
                                    fast_row,
                                    slow_row,
                                    halftrend_trend,
                                    macz_hist,
                                    ott_line,
                                    start_t,
                                    strategy,
                                    signal_mode,
                                )
                            }
                        };
                        local.push(
                            DoubleMaParams {
                                fast_len,
                                slow_len,
                                fast_ma_id: 0,
                                slow_ma_id: 0,
                            },
                            metrics,
                        );
                        Ok(local)
                    })
                    .try_reduce(init, |mut a, b| {
                        a.merge(b);
                        Ok(a)
                    })?;

                processed = processed.saturating_add(tile.num_combos());
                agg.merge(tile);
            }

            emit_progress(progress, processed, total_pairs, "composite_cpu");
        }
    }

    let mut result = agg
        .finalize()
        .ok_or_else(|| "grid search produced no result".to_string())?;

    if !objective_needs_return_stats(objective) && !include_all {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let mut fast_cache: std::collections::HashMap<u16, Vec<f64>> =
            std::collections::HashMap::new();
        let mut slow_cache: std::collections::HashMap<u16, Vec<f64>> =
            std::collections::HashMap::new();

        let cache_ma = |cache: &mut std::collections::HashMap<u16, Vec<f64>>,
                        period: u16,
                        ma_type: &str,
                        ma_params: Option<&HashMap<String, f64>>,
                        alma_offset: f64,
                        alma_sigma: f64|
         -> Result<(), String> {
            if cache.contains_key(&period) {
                return Ok(());
            }
            let periods = [period];
            let (values, _) = ma_matrix_cpu_f64(
                candles,
                ma_type,
                ma_source,
                &periods,
                ma_params,
                alma_offset,
                alma_sigma,
            )?;
            cache.insert(period, values);
            Ok(())
        };

        let recompute = |fast_len: u16,
                         slow_len: u16,
                         fast_cache: &mut std::collections::HashMap<u16, Vec<f64>>,
                         slow_cache: &mut std::collections::HashMap<u16, Vec<f64>>|
         -> Result<Metrics, String> {
            cache_ma(
                fast_cache,
                fast_len,
                fast_ma_type,
                fast_ma_params,
                fast_alma_offset,
                fast_alma_sigma,
            )?;
            cache_ma(
                slow_cache,
                slow_len,
                slow_ma_type,
                slow_ma_params,
                slow_alma_offset,
                slow_alma_sigma,
            )?;

            let fast = fast_cache
                .get(&fast_len)
                .ok_or_else(|| "fast MA cache miss".to_string())?;
            let slow = slow_cache
                .get(&slow_len)
                .ok_or_else(|| "slow MA cache miss".to_string())?;

            let max_p = (fast_len as usize).max(slow_len as usize).max(1);
            let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
            let start_t = t_valid.saturating_add(trade_offset);
            if let Some((plan, ctx)) = dynamic_plan_ctx.as_ref() {
                crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu(
                    plan, ctx, fast, slow, start_t, true,
                )
            } else {
                Ok(
                    ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                        &lr,
                        ott_prices,
                        fast,
                        slow,
                        halftrend_trend,
                        macz_hist,
                        ott_line,
                        start_t,
                        strategy,
                        signal_mode,
                    ),
                )
            }
        };

        result.best_metrics = recompute(
            result.best_params.fast_len,
            result.best_params.slow_len,
            &mut fast_cache,
            &mut slow_cache,
        )?;

        for (p, m) in result.top.iter_mut() {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }
            *m = recompute(p.fast_len, p.slow_len, &mut fast_cache, &mut slow_cache)?;
        }
    }

    let shadow_sampled = if dynamic_shadow_enabled {
        dynamic_shadow_sampled.load(Ordering::Relaxed)
    } else {
        0
    };
    let shadow_mismatches = if dynamic_shadow_enabled {
        dynamic_shadow_mismatches.load(Ordering::Relaxed)
    } else {
        0
    };
    if dynamic_shadow_enabled && shadow_mismatches > 0 {
        eprintln!(
            "[dynamic-shadow][composite_cpu] sampled={} mismatches={} every={}",
            shadow_sampled, shadow_mismatches, dynamic_shadow_every
        );
    }
    crate::strategy_dynamic::maybe_append_dynamic_strategy_artifact(
        crate::strategy_dynamic::DynamicStrategyArtifactRecord {
            scope: "composite_cpu",
            profile: "CompositeCpu",
            backend: "Cpu",
            dynamic_enabled: dynamic_plan_ctx.is_some(),
            shadow_enabled: dynamic_shadow_enabled,
            shadow_every: dynamic_shadow_every,
            shadow_sampled,
            shadow_mismatches,
            runtime_ms: run_start.elapsed().as_millis() as u64,
            work_items: processed,
        },
    );

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        use std::sync::{Mutex, OnceLock};
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    fn with_dynamic_strategy_opt_env<T>(enabled: bool, f: impl FnOnce() -> T) -> T {
        let _guard = lock_env();
        let key = "VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL";
        let prev = std::env::var(key).ok();
        if enabled {
            std::env::set_var(key, "1");
        } else {
            std::env::remove_var(key);
        }
        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        match out {
            Ok(v) => v,
            Err(panic_payload) => std::panic::resume_unwind(panic_payload),
        }
    }

    fn with_dynamic_strategy_shadow_env<T>(every: usize, f: impl FnOnce() -> T) -> T {
        let _guard = lock_env();
        let key_exp = "VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL";
        let key_shadow = "VECTORBT_DYNAMIC_STRATEGY_OPT_SHADOW";
        let key_every = "VECTORBT_DYNAMIC_STRATEGY_OPT_SHADOW_EVERY";
        let prev_exp = std::env::var(key_exp).ok();
        let prev_shadow = std::env::var(key_shadow).ok();
        let prev_every = std::env::var(key_every).ok();
        std::env::set_var(key_exp, "1");
        std::env::set_var(key_shadow, "1");
        std::env::set_var(key_every, every.max(1).to_string());

        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        match prev_exp {
            Some(v) => std::env::set_var(key_exp, v),
            None => std::env::remove_var(key_exp),
        }
        match prev_shadow {
            Some(v) => std::env::set_var(key_shadow, v),
            None => std::env::remove_var(key_shadow),
        }
        match prev_every {
            Some(v) => std::env::set_var(key_every, v),
            None => std::env::remove_var(key_every),
        }

        match out {
            Ok(v) => v,
            Err(panic_payload) => std::panic::resume_unwind(panic_payload),
        }
    }

    fn assert_composite_cpu_env_parity(
        req: CompositeCpuRequest,
        candles: &vector_ta::utilities::data_loader::Candles,
        cancel: &AtomicBool,
    ) {
        let legacy = with_dynamic_strategy_opt_env(false, || {
            run_composite_cpu_optimization_blocking_with_candles(req.clone(), candles, cancel, None)
                .expect("legacy composite cpu run")
        });
        let dynamic = with_dynamic_strategy_opt_env(true, || {
            run_composite_cpu_optimization_blocking_with_candles(req, candles, cancel, None)
                .expect("dynamic composite cpu run")
        });

        assert_eq!(legacy.num_combos, dynamic.num_combos);
        assert_eq!(legacy.best_params.fast_len, dynamic.best_params.fast_len);
        assert_eq!(legacy.best_params.slow_len, dynamic.best_params.slow_len);
        assert!((legacy.best_metrics.sharpe - dynamic.best_metrics.sharpe).abs() < 1e-12);
        assert!((legacy.best_metrics.pnl - dynamic.best_metrics.pnl).abs() < 1e-12);
        assert!((legacy.best_metrics.max_dd - dynamic.best_metrics.max_dd).abs() < 1e-12);
        assert_eq!(legacy.best_metrics.trades, dynamic.best_metrics.trades);
        assert_eq!(legacy.top.len(), dynamic.top.len());

        let heatmap_legacy = legacy.heatmap.expect("legacy heatmap");
        let heatmap_dynamic = dynamic.heatmap.expect("dynamic heatmap");
        assert_eq!(heatmap_legacy.values, heatmap_dynamic.values);
    }

    fn make_base_composite_req() -> CompositeCpuRequest {
        CompositeCpuRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            gpu_prescan: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
        }
    }

    fn make_trending_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64;
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.5);
            vol.push(1_000.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    #[test]
    fn composite_cpu_smoke_small_grid_counts_and_shapes() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            gpu_prescan: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("composite cpu run");

        assert_eq!(res.num_combos, 6);
        assert!(res.best_params.fast_len < res.best_params.slow_len);
        assert!(res.top.len() <= 5);
        assert!(res.heatmap.is_some());
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert!(matches!(res.mode_used, OptimizationModeResolved::Grid));
    }

    #[test]
    fn composite_cpu_dynamic_strategy_env_path_matches_legacy_results() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let req = make_base_composite_req();
        assert_composite_cpu_env_parity(req, &candles, &cancel);
    }

    #[test]
    fn composite_cpu_dynamic_strategy_shadow_mode_matches_legacy_results() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let req = make_base_composite_req();

        let legacy = with_dynamic_strategy_opt_env(false, || {
            run_composite_cpu_optimization_blocking_with_candles(
                req.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("legacy composite cpu run")
        });
        let dynamic = with_dynamic_strategy_shadow_env(1, || {
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("dynamic shadow composite cpu run")
        });

        assert_eq!(legacy.best_params.fast_len, dynamic.best_params.fast_len);
        assert_eq!(legacy.best_params.slow_len, dynamic.best_params.slow_len);
        assert!((legacy.best_metrics.sharpe - dynamic.best_metrics.sharpe).abs() < 1e-12);
        assert!((legacy.best_metrics.pnl - dynamic.best_metrics.pnl).abs() < 1e-12);
        assert!((legacy.best_metrics.max_dd - dynamic.best_metrics.max_dd).abs() < 1e-12);
        assert_eq!(legacy.best_metrics.trades, dynamic.best_metrics.trades);
    }

    #[test]
    fn composite_trend_dynamic_strategy_env_path_matches_legacy_results() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.strategy.trend_quality_enable = true;
        assert_composite_cpu_env_parity(req, &candles, &cancel);
    }

    #[test]
    fn regime_hybrid_dynamic_strategy_env_path_matches_legacy_results() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.strategy.regime_hybrid_enable = true;
        req.strategy.trend_quality_enable = false;
        assert_composite_cpu_env_parity(req, &candles, &cancel);
    }

    #[test]
    fn breakout_momentum_dynamic_strategy_env_path_matches_legacy_results() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.strategy.breakout_momentum_enable = true;
        req.strategy.trend_quality_enable = false;
        req.strategy.regime_hybrid_enable = false;
        assert_composite_cpu_env_parity(req, &candles, &cancel);
    }

    #[test]
    fn composite_cpu_entry_core_exit_overlay_mode_runs() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::EntryCoreExitOverlay,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            gpu_prescan: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("composite cpu run");
        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn composite_grid_mode_helper_allows_grid_and_auto_only() {
        assert!(matches!(
            resolve_composite_grid_mode(&OptimizationMode::Grid, "Composite"),
            Ok(OptimizationModeResolved::Grid)
        ));
        assert!(matches!(
            resolve_composite_grid_mode(&OptimizationMode::Auto, "Composite"),
            Ok(OptimizationModeResolved::Grid)
        ));

        let err = resolve_composite_grid_mode(&OptimizationMode::GpuVdps, "Composite")
            .expect_err("GpuVdps should be rejected");
        assert_eq!(err, "Composite strategy currently supports mode=Grid only.");
    }

    #[test]
    fn composite_cpu_rejects_gpu_vdps_mode() {
        let candles = make_trending_candles(2000);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::GpuVdps,
            gpu_prescan: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let err =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect_err("GpuVdps mode should be rejected for composite CPU optimization");
        assert!(
            err.to_ascii_lowercase().contains("mode=grid"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn composite_cpu_trend_pullback_reversion_mode_runs() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::TrendPullbackReversion,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            gpu_prescan: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("composite cpu run");
        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn composite_cpu_prescan_can_narrow_ranges() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "test".to_string(),
            fast_range: (5, 50, 1),
            slow_range: (20, 200, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            gpu_prescan: Some(crate::composite_mo::CompositeGpuPrescanConfig {
                top_k: 0,
                window: 0,
                predictive_top_k: 0,
                predictive_stride: 8,
                device_id: None,
                require_gpu: false,
                objectives: crate::composite_mo::CompositeGpuPrescanObjectives::Both,
                ma_pairs: None,
                mode: OptimizationMode::Auto,
            }),
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("composite cpu run");

        assert_eq!(res.num_combos, 1);
        assert!(res.best_params.fast_len < res.best_params.slow_len);
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
    }

    #[test]
    fn composite_cpu_prescan_predictive_rescue_can_expand_candidates() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "test".to_string(),
            fast_range: (5, 50, 1),
            slow_range: (20, 200, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            gpu_prescan: Some(crate::composite_mo::CompositeGpuPrescanConfig {
                top_k: 0,
                window: 0,
                predictive_top_k: 8,
                predictive_stride: 8,
                device_id: None,
                require_gpu: false,
                objectives: crate::composite_mo::CompositeGpuPrescanObjectives::Both,
                ma_pairs: None,
                mode: OptimizationMode::Auto,
            }),
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("composite cpu run");

        assert!(res.num_combos > 1);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn composite_cpu_drilldown_smoke_has_bins_and_values() {
        let candles = make_trending_candles(5000);

        let req = CompositeCpuDrilldownRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "test".to_string(),
            params: DoubleMaParamsResolved {
                fast_len: 10,
                slow_len: 30,
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
            },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            bins: Some(256),
        };

        let drill = compute_composite_cpu_drilldown_blocking_with_candles(req, &candles)
            .expect("composite drilldown");
        let curves = &drill.curves;

        assert!(curves.equity.bins > 0);
        assert_eq!(curves.equity.bins, curves.drawdown.bins);
        assert_eq!(
            curves.equity.minmax.len(),
            curves.equity.bins.saturating_mul(2)
        );
        assert_eq!(
            curves.drawdown.minmax.len(),
            curves.drawdown.bins.saturating_mul(2)
        );

        assert!(curves.equity.minmax.iter().all(|v| v.is_finite()));
        assert!(curves
            .drawdown
            .minmax
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0 && *v <= 1.0));
        assert!(!drill.composite_indicator_bars.is_empty());
    }

    #[test]
    fn composite_cpu_local_stability_smoke_has_base_point_and_summary() {
        let candles = make_trending_candles(4096);

        let req = CompositeCpuLocalStabilityRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "test".to_string(),
            params: DoubleMaParamsResolved {
                fast_len: 9,
                slow_len: 21,
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
            },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            objective: ObjectiveKind::Sharpe,
            radius: Some(1),
            step: Some(1),
        };

        let res = compute_composite_cpu_local_stability_blocking_with_candles(req, &candles)
            .expect("composite local stability");

        assert_eq!(res.base.fast_len, 9);
        assert_eq!(res.base.slow_len, 21);
        assert_eq!(res.radius, 1);
        assert_eq!(res.step, 1);
        assert!(!res.points.is_empty());
        assert!(res
            .points
            .iter()
            .any(|p| p.fast_len == 9 && p.slow_len == 21));
        assert!(res.summary.min <= res.summary.median);
        assert!(res.summary.median <= res.summary.max);
        assert!(res.summary.base.is_finite());
    }

    #[test]
    fn asr_optimization_smoke_runs() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::CpuOnly;
        req.asr = Some(AsrStrategyParams::default());
        req.fast_range = (5, 9, 1);
        req.slow_range = (20, 30, 2);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("asr optimization run");
        assert!(res.num_combos > 0);
        assert!(res.best_params.fast_len < res.best_params.slow_len);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn vcc_optimization_smoke_runs() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::CpuOnly;
        req.vcc = Some(VccStrategyParams::default());
        req.fast_range = (5, 9, 1);
        req.slow_range = (20, 30, 2);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("vcc optimization run");
        assert!(res.num_combos > 0);
        assert!(res.best_params.fast_len < res.best_params.slow_len);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn vemr_optimization_smoke_runs() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::CpuOnly;
        req.vemr = Some(VemrStrategyParams::default());
        req.fast_range = (5, 9, 1);
        req.slow_range = (20, 30, 2);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("vemr optimization run");
        assert!(res.num_combos > 0);
        assert!(res.best_params.fast_len < res.best_params.slow_len);
        assert!(!res.top.is_empty());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_optimization_gpu_kernel_smoke_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        req.vemr = Some(VemrStrategyParams::default());
        req.fast_range = (5, 9, 1);
        req.slow_range = (20, 30, 2);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("vemr gpu-kernel optimization run");
        assert!(res.num_combos > 0);
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "expected VEMR GPU path, got backend={:?}, route={:?}, fallback={:?}",
            res.backend_used,
            res.backend_route,
            res.backend_fallback_reason
        );
        assert!(res.best_params.fast_len < res.best_params.slow_len);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_optimization_gpu_kernel_sanity_matches_cpu_small_grid() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mut cpu_req = make_base_composite_req();
        cpu_req.backend = Backend::CpuOnly;
        cpu_req.vemr = Some(VemrStrategyParams::default());
        cpu_req.fast_range = (5, 7, 1);
        cpu_req.slow_range = (12, 16, 1);
        cpu_req.top_k = Some(128);
        cpu_req.include_all = Some(true);

        let cpu_res = run_composite_cpu_optimization_blocking_with_candles(
            cpu_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("vemr cpu optimization run");

        let mut gpu_req = cpu_req;
        gpu_req.backend = Backend::GpuKernel { device_id: 0 };
        let gpu_res =
            run_composite_cpu_optimization_blocking_with_candles(gpu_req, &candles, &cancel, None)
                .expect("vemr gpu-kernel optimization run");

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "expected VEMR GPU backend, got {:?}",
            gpu_res.backend_used
        );
        assert_eq!(cpu_res.num_combos, gpu_res.num_combos);

        let cpu_all = cpu_res
            .all
            .expect("cpu include_all result should contain full combo rows");
        let gpu_all = gpu_res
            .all
            .expect("gpu include_all result should contain full combo rows");
        assert_eq!(cpu_all.len(), gpu_all.len());

        let mut cpu_map: std::collections::HashMap<(u16, u16), Metrics> =
            std::collections::HashMap::with_capacity(cpu_all.len());
        for (params, metrics) in cpu_all {
            cpu_map.insert((params.fast_len, params.slow_len), metrics);
        }

        let tol = 1.0e-2;
        for (params, gpu_metrics) in gpu_all {
            let key = (params.fast_len, params.slow_len);
            let cpu_metrics = cpu_map
                .get(&key)
                .unwrap_or_else(|| panic!("cpu metrics missing for combo {:?}", key));
            assert!(
                (gpu_metrics.pnl - cpu_metrics.pnl).abs() <= tol,
                "pnl mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.pnl,
                cpu_metrics.pnl
            );
            assert!(
                (gpu_metrics.max_dd - cpu_metrics.max_dd).abs() <= tol,
                "max_dd mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.max_dd,
                cpu_metrics.max_dd
            );
            assert!(
                (gpu_metrics.exposure - cpu_metrics.exposure).abs() <= tol,
                "exposure mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.exposure,
                cpu_metrics.exposure
            );
            assert!(
                (gpu_metrics.net_exposure - cpu_metrics.net_exposure).abs() <= tol,
                "net_exposure mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.net_exposure,
                cpu_metrics.net_exposure
            );
            assert_eq!(
                gpu_metrics.trades, cpu_metrics.trades,
                "trades mismatch for {:?}: gpu={} cpu={}",
                key, gpu_metrics.trades, cpu_metrics.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_optimization_gpu_kernel_topk_matches_cpu_small_grid() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mut cpu_req = make_base_composite_req();
        cpu_req.backend = Backend::CpuOnly;
        cpu_req.vemr = Some(VemrStrategyParams::default());
        cpu_req.fast_range = (5, 7, 1);
        cpu_req.slow_range = (12, 16, 1);
        cpu_req.objective = ObjectiveKind::Omega;
        cpu_req.top_k = Some(4);
        cpu_req.include_all = Some(false);
        cpu_req.heatmap_bins = Some(0);

        let cpu_res = run_composite_cpu_optimization_blocking_with_candles(
            cpu_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("vemr cpu optimization run");

        let mut gpu_req = cpu_req;
        gpu_req.backend = Backend::GpuKernel { device_id: 0 };
        let gpu_res =
            run_composite_cpu_optimization_blocking_with_candles(gpu_req, &candles, &cancel, None)
                .expect("vemr gpu-kernel optimization run");

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "expected VEMR GPU backend, got {:?}",
            gpu_res.backend_used
        );
        assert_eq!(cpu_res.num_combos, gpu_res.num_combos);
        assert_eq!(cpu_res.top.len(), gpu_res.top.len());

        let cpu_best_score = objective_score(&cpu_res.best_metrics, ObjectiveKind::Omega);
        let gpu_best_score = objective_score(&gpu_res.best_metrics, ObjectiveKind::Omega);
        let tol = 1.0e-2;
        assert!(
            (gpu_best_score - cpu_best_score).abs() <= tol,
            "best omega mismatch: gpu={} cpu={}",
            gpu_best_score,
            cpu_best_score
        );

        let mut cpu_top: std::collections::HashMap<(u16, u16), Metrics> =
            std::collections::HashMap::with_capacity(cpu_res.top.len());
        for (params, metrics) in cpu_res.top {
            cpu_top.insert((params.fast_len, params.slow_len), metrics);
        }

        for (params, gpu_metrics) in gpu_res.top {
            let key = (params.fast_len, params.slow_len);
            let cpu_metrics = cpu_top
                .get(&key)
                .unwrap_or_else(|| panic!("cpu top metrics missing for combo {:?}", key));
            assert!(
                (gpu_metrics.omega - cpu_metrics.omega).abs() <= tol,
                "omega mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.omega,
                cpu_metrics.omega
            );
            assert!(
                (gpu_metrics.pnl - cpu_metrics.pnl).abs() <= tol,
                "pnl mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.pnl,
                cpu_metrics.pnl
            );
            assert_eq!(
                gpu_metrics.trades, cpu_metrics.trades,
                "trades mismatch for {:?}: gpu={} cpu={}",
                key, gpu_metrics.trades, cpu_metrics.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_optimization_gpu_kernel_smoke_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        req.vcc = Some(VccStrategyParams::default());
        req.fast_range = (5, 9, 1);
        req.slow_range = (20, 30, 2);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("vcc gpu-kernel optimization run");
        assert!(res.num_combos > 0);
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "expected VCC GPU path, got backend={:?}, route={:?}, fallback={:?}",
            res.backend_used,
            res.backend_route,
            res.backend_fallback_reason
        );
        assert!(res.best_params.fast_len < res.best_params.slow_len);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_optimization_gpu_kernel_sanity_matches_cpu_small_grid() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mut cpu_req = make_base_composite_req();
        cpu_req.backend = Backend::CpuOnly;
        cpu_req.vcc = Some(VccStrategyParams::default());
        cpu_req.fast_range = (5, 7, 1);
        cpu_req.slow_range = (12, 16, 1);
        cpu_req.top_k = Some(128);
        cpu_req.include_all = Some(true);

        let cpu_res = run_composite_cpu_optimization_blocking_with_candles(
            cpu_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("vcc cpu optimization run");

        let mut gpu_req = cpu_req;
        gpu_req.backend = Backend::GpuKernel { device_id: 0 };
        let gpu_res =
            run_composite_cpu_optimization_blocking_with_candles(gpu_req, &candles, &cancel, None)
                .expect("vcc gpu-kernel optimization run");

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "expected VCC GPU backend, got {:?}",
            gpu_res.backend_used
        );
        assert_eq!(cpu_res.num_combos, gpu_res.num_combos);

        let cpu_all = cpu_res
            .all
            .expect("cpu include_all result should contain full combo rows");
        let gpu_all = gpu_res
            .all
            .expect("gpu include_all result should contain full combo rows");
        assert_eq!(cpu_all.len(), gpu_all.len());

        let mut cpu_map: std::collections::HashMap<(u16, u16), Metrics> =
            std::collections::HashMap::with_capacity(cpu_all.len());
        for (params, metrics) in cpu_all {
            cpu_map.insert((params.fast_len, params.slow_len), metrics);
        }

        let tol = 1.0e-2;
        for (params, gpu_metrics) in gpu_all {
            let key = (params.fast_len, params.slow_len);
            let cpu_metrics = cpu_map
                .get(&key)
                .unwrap_or_else(|| panic!("cpu metrics missing for combo {:?}", key));
            assert!(
                (gpu_metrics.pnl - cpu_metrics.pnl).abs() <= tol,
                "pnl mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.pnl,
                cpu_metrics.pnl
            );
            assert!(
                (gpu_metrics.max_dd - cpu_metrics.max_dd).abs() <= tol,
                "max_dd mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.max_dd,
                cpu_metrics.max_dd
            );
            assert!(
                (gpu_metrics.exposure - cpu_metrics.exposure).abs() <= tol,
                "exposure mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.exposure,
                cpu_metrics.exposure
            );
            assert!(
                (gpu_metrics.net_exposure - cpu_metrics.net_exposure).abs() <= tol,
                "net_exposure mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.net_exposure,
                cpu_metrics.net_exposure
            );
            assert_eq!(
                gpu_metrics.trades, cpu_metrics.trades,
                "trades mismatch for {:?}: gpu={} cpu={}",
                key, gpu_metrics.trades, cpu_metrics.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_optimization_gpu_kernel_smoke_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        req.asr = Some(AsrStrategyParams::default());
        req.fast_range = (5, 9, 1);
        req.slow_range = (20, 30, 2);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("asr gpu-kernel optimization run");
        assert!(res.num_combos > 0);
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "expected ASR GPU path, got backend={:?}, route={:?}, fallback={:?}",
            res.backend_used,
            res.backend_route,
            res.backend_fallback_reason
        );
        assert!(res.best_params.fast_len < res.best_params.slow_len);
    }

    #[test]
    fn asr_drilldown_smoke_runs() {
        let candles = make_trending_candles(5000);
        let req = CompositeCpuDrilldownRequest {
            backend: Backend::CpuOnly,
            asr: Some(AsrStrategyParams::default()),
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "test".to_string(),
            params: DoubleMaParamsResolved {
                fast_len: 8,
                slow_len: 24,
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "alma".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
            },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            bins: Some(256),
        };

        let res = compute_composite_cpu_drilldown_blocking_with_candles(req, &candles)
            .expect("asr drilldown run");
        assert!(res.curves.equity.bins > 0);
        assert_eq!(
            res.curves.equity.minmax.len(),
            res.curves.equity.bins.saturating_mul(2)
        );
    }

    #[test]
    fn asr_local_stability_smoke_runs() {
        let candles = make_trending_candles(5000);
        let req = CompositeCpuLocalStabilityRequest {
            backend: Backend::CpuOnly,
            asr: Some(AsrStrategyParams::default()),
            vcc: None,
            vemr: None,
            rocb: None,
            data_id: "test".to_string(),
            params: DoubleMaParamsResolved {
                fast_len: 8,
                slow_len: 24,
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "alma".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
            },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            objective: ObjectiveKind::Sharpe,
            radius: Some(1),
            step: Some(1),
        };

        let res = compute_composite_cpu_local_stability_blocking_with_candles(req, &candles)
            .expect("asr local stability run");
        assert!(!res.points.is_empty());
        assert!(res.summary.base.is_finite());
    }

    #[test]
    fn vemr_drilldown_smoke_runs() {
        let candles = make_trending_candles(5000);
        let req = CompositeCpuDrilldownRequest {
            backend: Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: Some(VemrStrategyParams::default()),
            rocb: None,
            data_id: "test".to_string(),
            params: DoubleMaParamsResolved {
                fast_len: 8,
                slow_len: 24,
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "alma".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
            },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            bins: Some(256),
        };

        let res = compute_composite_cpu_drilldown_blocking_with_candles(req, &candles)
            .expect("vemr drilldown run");
        assert!(res.curves.equity.bins > 0);
        assert_eq!(
            res.curves.equity.minmax.len(),
            res.curves.equity.bins.saturating_mul(2)
        );
    }

    #[test]
    fn vemr_local_stability_smoke_runs() {
        let candles = make_trending_candles(5000);
        let req = CompositeCpuLocalStabilityRequest {
            backend: Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: Some(VemrStrategyParams::default()),
            rocb: None,
            data_id: "test".to_string(),
            params: DoubleMaParamsResolved {
                fast_len: 8,
                slow_len: 24,
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "alma".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
            },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            objective: ObjectiveKind::Sharpe,
            radius: Some(1),
            step: Some(1),
        };

        let res = compute_composite_cpu_local_stability_blocking_with_candles(req, &candles)
            .expect("vemr local stability run");
        assert!(!res.points.is_empty());
        assert!(res.summary.base.is_finite());
    }

    #[test]
    fn composite_cpu_rejects_multiple_custom_strategy_configs() {
        let candles = make_trending_candles(1024);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.asr = Some(AsrStrategyParams::default());
        req.vcc = Some(VccStrategyParams::default());
        req.vemr = Some(VemrStrategyParams::default());

        let err =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect_err("multiple custom strategy configs should be rejected");
        assert!(err.to_ascii_lowercase().contains("only one"));
    }
}
