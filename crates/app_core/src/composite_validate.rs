use crate::asr::{
    build_asr_desired_positions, compute_asr_fast_signals, compute_asr_kaufman_stops,
    compute_asr_slow_signals, compute_asr_start_t, eval_asr_curves_from_desired,
    eval_asr_metrics_window_from_desired, resolve_asr_periods, validate_asr_params, AsrFastSignals,
    AsrKaufmanStops, AsrSlowSignals, AsrStrategyParams,
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
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::composite_cpu::build_base_composite_hard_gate_regime_f32;
use crate::composite_cpu::{
    compute_halftrend_trend_with_backend, compute_macz_hist_with_backend,
    compute_ott_line_with_backend, MaczGateParams, OttGateParams,
};
use crate::cscv_pbo::{CscvCandidatePboStats, CscvPboConfig, CscvPboResult, SliceReturnStats};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::double_ma::ma_matrix_cpu_f64;
use crate::double_ma::{
    compute_log_returns_f64, first_finite_idx, ma_matrix_with_backend_f64,
    resolve_alma_params_for_side, validate_ma_kind, validate_strategy_cfg, Backend, BackendUsed,
    HalfTrendRegimeFilter,
};
use crate::iterative_sim::{IterativeMetricsDelta, IterativeSimConfig, ValidationEngine};
use crate::mkcs::{
    build_mkcs_desired_positions, compute_mkcs_breakout_signals, compute_mkcs_compression_signals,
    compute_mkcs_start_t, resolve_mkcs_periods, validate_mkcs_params, MkcsBreakoutSignals,
    MkcsCompressionSignals, MkcsStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::mkcs::{
    compute_mkcs_donchian_output_sweep_cuda_device, compute_mkcs_stddev_sweep_cuda_device,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::mkcs_gpu_kernel::{
    eval_mkcs_metrics_gpu_kernel_device_rows, MkcsGpuComboBatch, MkcsGpuDeviceSignalMatrices,
};
use crate::progress::ProgressSink;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::rocb::compute_rocb_atr_sweep_cuda_device;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::rocb::compute_rocb_donchian_output_sweep_cuda_device;
use crate::rocb::{
    build_rocb_desired_positions, compute_rocb_breakout_signals, compute_rocb_risk_signals,
    compute_rocb_start_t, resolve_rocb_periods, validate_rocb_params, RocbBreakoutSignals,
    RocbRiskSignals, RocbStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::rocb_gpu_kernel::{
    eval_rocb_metrics_gpu_kernel_device_rows, RocbGpuComboBatch, RocbGpuDeviceSignalMatrices,
};
use crate::validation_pipeline::{
    build_run_verdict, sanitize_hard_gates, ValidationCandidateVerdict, ValidationPostCandidate,
    ValidationRunVerdict,
};
use crate::vcc::{
    build_vcc_desired_positions, compute_vcc_fast_signals, compute_vcc_start_t,
    compute_vcc_trend_signals, resolve_vcc_periods, validate_vcc_params, VccFastSignals,
    VccStrategyParams, VccTrendSignals,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcc::{
    compute_vcc_fast_dual_sweep_cuda_device, compute_vcc_ttm_squeeze_sweep_cuda_device,
    compute_vcc_ttm_trend_sweep_cuda_device,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcc::{vcc_momentum_roundtrip_is_fp32_stable, vcc_momentum_roundtrip_parity};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcc_gpu_kernel::{
    eval_vcc_metrics_gpu_kernel_device_rows, eval_vcc_metrics_gpu_kernel_device_rows_detailed,
    VccGpuComboBatch, VccGpuDeviceSignalMatrices,
};
use crate::vemr::{
    build_vemr_desired_positions, compute_vemr_fast_signals, compute_vemr_risk_signals,
    compute_vemr_start_t, resolve_vemr_periods, validate_vemr_params, VemrFastSignals,
    VemrRiskSignals, VemrStrategyParams,
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

use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use ta_strategies::composite_cpu::CompositeSignalMode;
use ta_strategies::double_ma::{Metrics, StrategyConfig, Trade};

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::composite_profile_route::composite_profile_from_strategy;
#[cfg(feature = "cuda")]
use crate::dynamic_indicator_adapter::PreparedCudaIndicatorInputs;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::strategy_ir::StrategyProfileKind;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use ta_optimizer::ObjectiveKind;

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
    pub mkcs: Option<MkcsStrategyParams>,
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

fn default_validation_backend() -> Backend {
    Backend::CpuOnly
}

fn validation_backend_label(backend: &Backend) -> &'static str {
    match backend {
        Backend::Auto { .. } => "Auto",
        Backend::CpuOnly => "CpuOnly",
        Backend::GpuOnly { .. } => "GpuOnly",
        Backend::GpuHybridKernel { .. } => "GpuHybridKernel",
        Backend::GpuKernel { .. } => "GpuKernel",
    }
}

fn resolve_validation_backend_runtime(
    backend: &Backend,
    gpu_activated: bool,
    fallback_reason: Option<String>,
) -> (BackendUsed, Option<String>, Option<String>) {
    if gpu_activated {
        let route = match backend {
            Backend::Auto { .. } => Some("Auto->GpuKernel".to_string()),
            _ => Some(validation_backend_label(backend).to_string()),
        };
        return (BackendUsed::Gpu, route, None);
    }

    match backend {
        Backend::CpuOnly => (BackendUsed::Cpu, Some("CpuOnly".to_string()), None),
        Backend::Auto { .. } => (
            BackendUsed::Cpu,
            Some("Auto->CpuOnly".to_string()),
            fallback_reason.or_else(|| {
                Some(
                    "GPU composite validation runtime not implemented yet; using CPU validation."
                        .to_string(),
                )
            }),
        ),
        _ => (
            BackendUsed::Cpu,
            Some(format!("{}->CpuOnly", validation_backend_label(backend))),
            fallback_reason.or_else(|| {
                Some(
                    "GPU composite validation runtime not implemented yet; using CPU validation."
                        .to_string(),
                )
            }),
        ),
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
struct GpuCompositeFixedCandidateValidationEval {
    metrics: Vec<Metrics>,
    start_ts: Vec<usize>,
    candidate_indices: HashMap<Vec<u8>, usize>,
    walk_forward_stats: Option<Vec<WalkForwardStats>>,
    cscv_slices: Option<HashMap<Vec<u8>, Vec<SliceReturnStats>>>,
    spa_slice_sums: Option<HashMap<Vec<u8>, Vec<f64>>>,
    base_composite_ctx: Option<BaseCompositeGpuValidationInputs>,
    backend_used: BackendUsed,
    backend_route: Option<String>,
    backend_fallback_reason: Option<String>,
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
struct BaseCompositeGpuValidationInputs {
    device_id: u32,
    fast_ma: String,
    slow_ma: String,
    fast_periods: Vec<u16>,
    slow_periods: Vec<u16>,
    fast_values_f32: Vec<f32>,
    fast_cols: usize,
    slow_values_f32: Vec<f32>,
    slow_cols: usize,
    device_regime: crate::gpu_backtest_kernel::DeviceRegimeFilter,
    shared_first_valid: usize,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
impl BaseCompositeGpuValidationInputs {
    fn fast_precomputed(&self) -> crate::gpu_backtest_kernel::PrecomputedHostMaMatrix<'_> {
        crate::gpu_backtest_kernel::PrecomputedHostMaMatrix {
            periods: &self.fast_periods,
            cols: self.fast_cols,
            values_row_major: &self.fast_values_f32,
        }
    }

    fn slow_precomputed(&self) -> crate::gpu_backtest_kernel::PrecomputedHostMaMatrix<'_> {
        crate::gpu_backtest_kernel::PrecomputedHostMaMatrix {
            periods: &self.slow_periods,
            cols: self.slow_cols,
            values_row_major: &self.slow_values_f32,
        }
    }

    fn total_pairs(&self) -> usize {
        self.fast_periods
            .len()
            .saturating_mul(self.slow_periods.len())
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn composite_validation_supports_gpu_fixed_candidates(
    req: &CompositeValidationRequest,
) -> Result<u32, String> {
    match req.backend {
        Backend::CpuOnly => Err("CPU validation requested".to_string()),
        Backend::GpuHybridKernel { .. } => Err(
            "fixed-candidate composite GPU validation does not support GpuHybridKernel yet"
                .to_string(),
        ),
        Backend::GpuOnly { .. } => Err(
            "fixed-candidate composite GPU validation requires GpuKernel-compatible execution"
                .to_string(),
        ),
        Backend::GpuKernel { device_id } | Backend::Auto { device_id } => Ok(device_id),
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn composite_gpu_validation_feature_blocker(req: &CompositeValidationRequest) -> Option<String> {
    if req.walk_forward.is_some() {
        let supports_walk_forward = req
            .candidates
            .first()
            .map(|cand| {
                cand.asr.is_some()
                    || cand.vcc.is_some()
                    || cand.rocb.is_some()
                    || cand.vemr.is_some()
                    || cand.mkcs.is_some()
                    || is_plain_base_composite_candidate(cand)
            })
            .unwrap_or(false);
        if !supports_walk_forward {
            return Some(
                "GPU fixed-candidate composite validation does not support walk_forward yet"
                    .to_string(),
            );
        }
    }
    if req.reopt_walk_forward.is_some() {
        let supports_reopt = req
            .candidates
            .first()
            .map(|cand| {
                cand.asr.is_some()
                    || cand.vcc.is_some()
                    || cand.rocb.is_some()
                    || cand.vemr.is_some()
                    || cand.mkcs.is_some()
                    || is_plain_base_composite_candidate(cand)
            })
            .unwrap_or(false);
        if !supports_reopt {
            return Some(
                "GPU fixed-candidate composite validation does not support reopt_walk_forward yet"
                    .to_string(),
            );
        }
    }
    if req.spa.is_some() {
        let supports_spa = req
            .candidates
            .first()
            .map(|cand| {
                cand.asr.is_some()
                    || cand.vcc.is_some()
                    || cand.rocb.is_some()
                    || cand.vemr.is_some()
                    || cand.mkcs.is_some()
                    || is_plain_base_composite_candidate(cand)
            })
            .unwrap_or(false);
        if !supports_spa {
            return Some(
                "GPU fixed-candidate composite validation does not support spa yet".to_string(),
            );
        }
    }
    if req.trade_bootstrap.is_some() {
        let supports_trade_bootstrap = req
            .candidates
            .first()
            .map(|cand| {
                cand.asr.is_some()
                    || cand.vcc.is_some()
                    || cand.rocb.is_some()
                    || cand.vemr.is_some()
                    || cand.mkcs.is_some()
                    || is_plain_base_composite_candidate(cand)
            })
            .unwrap_or(false);
        if !supports_trade_bootstrap {
            return Some(
                "GPU fixed-candidate composite validation does not support trade bootstrap yet"
                    .to_string(),
            );
        }
    }
    if req.cross_ticker.is_some() {
        return Some(
            "GPU fixed-candidate composite validation does not support cross_ticker yet"
                .to_string(),
        );
    }
    if req.cscv_pbo.is_some() {
        let supports_cscv = req
            .candidates
            .first()
            .map(|cand| {
                cand.asr.is_some()
                    || cand.vcc.is_some()
                    || cand.rocb.is_some()
                    || cand.vemr.is_some()
                    || cand.mkcs.is_some()
                    || is_plain_base_composite_candidate(cand)
            })
            .unwrap_or(false);
        if !supports_cscv {
            return Some(
                "GPU fixed-candidate composite validation does not support CSCV/PBO yet"
                    .to_string(),
            );
        }
    }
    if !matches!(req.validation_engine, ValidationEngine::Vectorized) {
        return Some(
            "GPU fixed-candidate composite validation currently supports only the vectorized validation engine"
                .to_string(),
        );
    }
    None
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn serialized_eq<T: Serialize>(a: &T, b: &T) -> Result<bool, String> {
    Ok(serde_json::to_vec(a).map_err(|e| e.to_string())?
        == serde_json::to_vec(b).map_err(|e| e.to_string())?)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn is_plain_base_composite_candidate(cand: &CompositeParams) -> bool {
    cand.asr.is_none()
        && cand.vcc.is_none()
        && cand.vemr.is_none()
        && cand.rocb.is_none()
        && cand.mkcs.is_none()
        && composite_profile_from_strategy(&cand.strategy) == StrategyProfileKind::CompositeCpu
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn serialize_composite_candidate_key(params: &CompositeParams) -> Result<Vec<u8>, String> {
    serde_json::to_vec(params).map_err(|e| e.to_string())
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn composite_candidate_index_map(
    candidates: &[CompositeParams],
) -> Result<HashMap<Vec<u8>, usize>, String> {
    let mut out = HashMap::with_capacity(candidates.len());
    for (idx, cand) in candidates.iter().enumerate() {
        let key = serialize_composite_candidate_key(cand)?;
        out.entry(key).or_insert(idx);
    }
    Ok(out)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn sanitize_gpu_cscv_slices(slices: usize) -> usize {
    let s = slices.clamp(4, 32);
    if (s & 1) == 0 {
        s
    } else {
        s.saturating_sub(1).max(4)
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn slice_return_stats_from_metrics(metric: &Metrics, slice_bars: usize) -> SliceReturnStats {
    let nf = slice_bars as f64;
    let mean = metric.sharpe * metric.volatility;
    let sum = mean * nf;
    let variance = metric.volatility * metric.volatility;
    let sumsq = variance * (nf - 1.0) + (sum * sum) / nf;
    SliceReturnStats { sum, sumsq }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn spa_slice_sums_from_slice_stats(
    slice_stats: &HashMap<Vec<u8>, Vec<SliceReturnStats>>,
) -> HashMap<Vec<u8>, Vec<f64>> {
    let mut out = HashMap::with_capacity(slice_stats.len());
    for (key, stats) in slice_stats {
        out.insert(key.clone(), stats.iter().map(|s| s.sum).collect());
    }
    out
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn try_prepare_asr_gpu_fixed_candidate_validation(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<GpuCompositeFixedCandidateValidationEval, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    if let Some(reason) = composite_gpu_validation_feature_blocker(req) {
        return Err(reason);
    }
    if req.candidates.is_empty() {
        return Err("no candidates to validate".to_string());
    }

    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    let first_asr = first.asr.as_ref().ok_or_else(|| {
        "GPU composite validation currently supports ASR, ROCB, VEMR, and VCC candidates only"
            .to_string()
    })?;
    if first.vcc.is_some() || first.vemr.is_some() || first.rocb.is_some() {
        return Err(
            "GPU composite validation currently supports ASR, ROCB, VEMR, and VCC candidates only"
                .to_string(),
        );
    }
    validate_strategy_cfg(&first.strategy)?;
    validate_asr_params(first_asr)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite ASR validation currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let mut min_sq = u16::MAX;
    let mut max_sq = 0u16;
    let mut min_trend = u16::MAX;
    let mut max_trend = 0u16;
    let mut min_er = u16::MAX;
    let mut max_er = 0u16;
    let mut min_z = u16::MAX;
    let mut max_z = 0u16;
    let mut periods_by_candidate: Vec<(u16, u16, u16, u16)> =
        Vec::with_capacity(req.candidates.len());

    for cand in &req.candidates {
        if cand.asr.is_none() || cand.vcc.is_some() || cand.vemr.is_some() || cand.rocb.is_some() {
            return Err(
                "GPU composite validation currently requires all candidates to use ASR".to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite validation currently requires a uniform StrategyConfig".to_string(),
            );
        }
        if !serialized_eq(cand.asr.as_ref().unwrap(), first_asr)? {
            return Err(
                "GPU composite validation currently requires uniform ASR parameters".to_string(),
            );
        }

        let periods = resolve_asr_periods(first_asr, cand.fast_len, cand.slow_len);
        min_sq = min_sq.min(periods.squeeze);
        max_sq = max_sq.max(periods.squeeze);
        min_trend = min_trend.min(periods.trend);
        max_trend = max_trend.max(periods.trend);
        min_er = min_er.min(periods.er);
        max_er = max_er.max(periods.er);
        min_z = min_z.min(periods.z);
        max_z = max_z.max(periods.z);
        periods_by_candidate.push((periods.squeeze, periods.trend, periods.er, periods.z));
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    let n = close.len();
    if n < 2 || ma_prices.len() != n {
        return Err("not enough candles or mismatched MA source length".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let kaufman =
        compute_asr_kaufman_stops_cuda_device(&prepared_cuda_inputs, &first.strategy, device_id)?;

    let fast_squeeze_sweep = compute_asr_ttm_squeeze_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_asr,
        min_sq,
        max_sq,
        1,
        device_id,
    )?;
    let fast_momentum_sweep = compute_asr_squeeze_momentum_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_asr,
        min_sq,
        max_sq,
        1,
        device_id,
    )?;
    let slow_angle_sweep = compute_asr_linearreg_angle_sweep_cuda_device(
        &prepared_cuda_inputs,
        min_trend,
        max_trend,
        1,
        device_id,
    )?;
    let slow_er_sweep =
        compute_asr_er_sweep_cuda_device(&prepared_cuda_inputs, min_er, max_er, 1, device_id)?;
    let slow_z_sweep = compute_asr_zscore_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_asr,
        min_z,
        max_z,
        1,
        device_id,
    )?;

    let mut fast_squeeze_rows: Vec<u64> = Vec::new();
    let mut fast_momentum_rows: Vec<u64> = Vec::new();
    let mut slow_angle_rows: Vec<u64> = Vec::new();
    let mut slow_er_rows: Vec<u64> = Vec::new();
    let mut slow_z_rows: Vec<u64> = Vec::new();
    let mut fast_lookup: HashMap<u16, i32> = HashMap::new();
    let mut slow_lookup: HashMap<(u16, u16, u16), i32> = HashMap::new();

    let mut fast_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut slow_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut start_ts: Vec<usize> = Vec::with_capacity(req.candidates.len());

    for (sq, trend, er, z) in periods_by_candidate {
        let fast_idx = if let Some(idx) = fast_lookup.get(&sq) {
            *idx
        } else {
            let sq_ptr = fast_squeeze_sweep
                .row_ptr_for_period(sq)
                .ok_or_else(|| format!("ASR GPU validation missing squeeze row for period={sq}"))?;
            let mo_ptr = fast_momentum_sweep.row_ptr_for_period(sq).ok_or_else(|| {
                format!("ASR GPU validation missing momentum row for period={sq}")
            })?;
            let idx = i32::try_from(fast_squeeze_rows.len())
                .map_err(|_| "ASR GPU fast row index exceeds i32 range".to_string())?;
            fast_squeeze_rows.push(sq_ptr);
            fast_momentum_rows.push(mo_ptr);
            fast_lookup.insert(sq, idx);
            idx
        };

        let slow_key = (trend, er, z);
        let slow_idx = if let Some(idx) = slow_lookup.get(&slow_key) {
            *idx
        } else {
            let angle_ptr = slow_angle_sweep.row_ptr_for_period(trend).ok_or_else(|| {
                format!("ASR GPU validation missing angle row for period={trend}")
            })?;
            let er_ptr = slow_er_sweep
                .row_ptr_for_period(er)
                .ok_or_else(|| format!("ASR GPU validation missing ER row for period={er}"))?;
            let z_ptr = slow_z_sweep
                .row_ptr_for_period(z)
                .ok_or_else(|| format!("ASR GPU validation missing zscore row for period={z}"))?;
            let idx = i32::try_from(slow_angle_rows.len())
                .map_err(|_| "ASR GPU slow row index exceeds i32 range".to_string())?;
            slow_angle_rows.push(angle_ptr);
            slow_er_rows.push(er_ptr);
            slow_z_rows.push(z_ptr);
            slow_lookup.insert(slow_key, idx);
            idx
        };

        let fast_first_valid = fast_squeeze_sweep
            .first_valid_for_period(sq)
            .max(fast_momentum_sweep.first_valid_for_period(sq));
        let slow_first_valid = slow_angle_sweep
            .first_valid_for_period(trend)
            .max(slow_er_sweep.first_valid_for_period(er))
            .max(slow_z_sweep.first_valid_for_period(z));
        let max_period = usize::from(sq.max(trend).max(er).max(z)).max(1);
        let start_t = compute_asr_start_t(
            first_valid_close,
            first_valid_source,
            fast_first_valid,
            slow_first_valid,
            kaufman.first_valid,
            max_period,
            first.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after ASR GPU warmup".to_string());
        }

        fast_row_index.push(fast_idx);
        slow_row_index.push(slow_idx);
        combo_start_t.push(
            i32::try_from(start_t).map_err(|_| "ASR GPU start_t exceeds i32 range".to_string())?,
        );
        start_ts.push(start_t);
    }

    let signals = AsrGpuDeviceSignalMatrices {
        close_f64: close,
        cols: n,
        fast_rows: fast_squeeze_rows.len(),
        slow_rows: slow_angle_rows.len(),
        fast_squeeze_rows: &fast_squeeze_rows,
        fast_momentum_rows: &fast_momentum_rows,
        slow_angle_rows: &slow_angle_rows,
        slow_er_rows: &slow_er_rows,
        slow_z_rows: &slow_z_rows,
        kaufman_long_row: kaufman.long.device_ptr,
        kaufman_short_row: kaufman.short.device_ptr,
    };
    let combo_end_t = vec![
        i32::try_from(n)
            .map_err(|_| "ASR GPU end_t exceeds i32 range".to_string())?;
        fast_row_index.len()
    ];
    let combos = AsrGpuComboBatch {
        fast_row_index: &fast_row_index,
        slow_row_index: &slow_row_index,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };
    let metrics = eval_asr_metrics_gpu_kernel_device_rows(
        device_id,
        &signals,
        &combos,
        &first.strategy,
        first_asr,
    )?;
    if metrics.len() != req.candidates.len() {
        return Err("ASR GPU validation output length mismatch".to_string());
    }
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        compute_asr_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &fast_row_index,
            &slow_row_index,
            &combo_start_t,
            &start_ts,
            first_asr,
            device_id,
            cscv_cfg,
        )?
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        let slice_stats = compute_asr_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &fast_row_index,
            &slow_row_index,
            &combo_start_t,
            &start_ts,
            first_asr,
            device_id,
            &CscvPboConfig {
                slices: spa_cfg.slices.max(4),
                max_combinations: usize::MAX,
                threshold: 0.0,
            },
        )?;
        slice_stats.as_ref().map(spa_slice_sums_from_slice_stats)
    } else {
        None
    };

    let backend_route = match req.backend {
        Backend::Auto { .. } => Some("Auto->GpuKernel".to_string()),
        _ => Some(validation_backend_label(&req.backend).to_string()),
    };

    Ok(GpuCompositeFixedCandidateValidationEval {
        metrics,
        start_ts,
        candidate_indices: composite_candidate_index_map(&req.candidates)?,
        walk_forward_stats: None,
        cscv_slices,
        spa_slice_sums,
        base_composite_ctx: None,
        backend_used: BackendUsed::Gpu,
        backend_route,
        backend_fallback_reason: None,
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(clippy::too_many_arguments)]
fn base_composite_gpu_metric_map(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    inputs: &BaseCompositeGpuValidationInputs,
    strategy: &StrategyConfig,
    top_k: usize,
    eval_range: Option<(usize, usize)>,
) -> Result<HashMap<(u16, u16), Metrics>, String> {
    let fast_precomputed = inputs.fast_precomputed();
    let slow_precomputed = inputs.slow_precomputed();
    let res =
        crate::gpu_backtest_kernel::optimize_double_ma_gpu_kernel_with_precomputed_and_regime(
            candles,
            &inputs.fast_periods,
            &inputs.slow_periods,
            &inputs.fast_ma,
            &inputs.slow_ma,
            &req.ma_source,
            req.fast_ma_params.as_ref(),
            req.slow_ma_params.as_ref(),
            Some(crate::gpu_backtest_kernel::RegimeFilterInput::DeviceF32(
                &inputs.device_regime,
            )),
            strategy,
            ObjectiveKind::Sharpe,
            top_k.max(1),
            true,
            crate::gpu_backtest_kernel::KernelConfig {
                device_id: inputs.device_id,
                fast_alma_offset: inputs.fast_alma_offset,
                fast_alma_sigma: inputs.fast_alma_sigma,
                slow_alma_offset: inputs.slow_alma_offset,
                slow_alma_sigma: inputs.slow_alma_sigma,
            },
            Some(&fast_precomputed),
            Some(&slow_precomputed),
            None,
            None,
            inputs.total_pairs(),
            0,
            Some(inputs.shared_first_valid),
            eval_range,
            &AtomicBool::new(false),
            None,
        )?;
    let all = res
        .all
        .as_ref()
        .ok_or_else(|| "GPU base composite validation expected include_all metrics".to_string())?;
    let mut out: HashMap<(u16, u16), Metrics> = HashMap::with_capacity(all.len());
    for (params, metric) in all {
        out.insert((params.fast_len, params.slow_len), metric.clone());
    }
    Ok(out)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_base_composite_gpu_walk_forward_stats(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    inputs: &BaseCompositeGpuValidationInputs,
    strategy: &StrategyConfig,
    candidate_pairs: &[(u16, u16)],
    start_ts: &[usize],
    cfg: &WalkForwardConfigOptional,
) -> Result<Vec<WalkForwardStats>, String> {
    let n = candles.close.len();
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);

    let mut candidate_windows: Vec<Vec<usize>> = Vec::with_capacity(start_ts.len());
    let mut unique_window_starts: BTreeSet<usize> = BTreeSet::new();
    for &start_t in start_ts {
        let mut windows = Vec::new();
        let mut t0 = start_t;
        while t0 + 2 <= n {
            let end = t0.saturating_add(window).min(n);
            if end <= t0 + 1 {
                break;
            }
            windows.push(t0);
            unique_window_starts.insert(t0);
            if end == n {
                break;
            }
            t0 = t0.saturating_add(step);
        }
        candidate_windows.push(windows);
    }

    let mut by_window: HashMap<usize, HashMap<(u16, u16), Metrics>> =
        HashMap::with_capacity(unique_window_starts.len());
    for &t0 in &unique_window_starts {
        let end = t0.saturating_add(window).min(n);
        let metric_map = base_composite_gpu_metric_map(
            req,
            candles,
            inputs,
            strategy,
            candidate_pairs.len(),
            Some((t0, end)),
        )?;
        by_window.insert(t0, metric_map);
    }

    let mut out = Vec::with_capacity(candidate_pairs.len());
    for (idx, pair) in candidate_pairs.iter().enumerate() {
        let mut sharpes = Vec::new();
        let mut dds = Vec::new();
        let mut pnls = Vec::new();
        let mut trades = Vec::new();
        for &t0 in &candidate_windows[idx] {
            let metric = by_window
                .get(&t0)
                .and_then(|m| m.get(pair))
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "GPU base composite walk_forward missing metric for pair ({}, {}) at window start {}",
                        pair.0, pair.1, t0
                    )
                })?;
            sharpes.push(metric.sharpe);
            dds.push(metric.max_dd);
            pnls.push(metric.pnl);
            trades.push(metric.trades);
        }
        sharpes.sort_by(|a, b| a.total_cmp(b));
        dds.sort_by(|a, b| a.total_cmp(b));
        pnls.sort_by(|a, b| a.total_cmp(b));
        out.push(WalkForwardStats {
            windows: sharpes.len(),
            sharpe_median: crate::stats_common::median(&sharpes),
            sharpe_worst: crate::stats_common::worst_min(&sharpes),
            max_dd_median: crate::stats_common::median(&dds),
            max_dd_worst: crate::stats_common::worst_max(&dds),
            pnl_median: crate::stats_common::median(&pnls),
            pnl_worst: crate::stats_common::worst_min(&pnls),
            trades_median: crate::stats_common::median_u32(&trades),
            trades_worst: crate::stats_common::worst_min_u32(&trades),
        });
    }
    Ok(out)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_base_composite_gpu_cscv_slice_stats(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    inputs: &BaseCompositeGpuValidationInputs,
    strategy: &StrategyConfig,
    candidate_keys: &[Vec<u8>],
    candidate_pairs: &[(u16, u16)],
    start_ts: &[usize],
    cfg: &CscvPboConfig,
) -> Result<Option<HashMap<Vec<u8>, Vec<SliceReturnStats>>>, String> {
    let slices = sanitize_gpu_cscv_slices(cfg.slices);
    let common_start_t = start_ts.iter().copied().max().unwrap_or(0);
    let slice_bars = candles.close.len().saturating_sub(common_start_t) / slices;
    if slice_bars < 2 || candidate_pairs.len() < 2 {
        return Ok(None);
    }

    let mut out: HashMap<Vec<u8>, Vec<SliceReturnStats>> =
        HashMap::with_capacity(candidate_keys.len());
    for key in candidate_keys {
        out.entry(key.clone())
            .or_insert_with(|| vec![SliceReturnStats::default(); slices]);
    }

    for slice_idx in 0..slices {
        let t0 = common_start_t + slice_idx * slice_bars;
        let end = t0.saturating_add(slice_bars).min(candles.close.len());
        let metric_map = base_composite_gpu_metric_map(
            req,
            candles,
            inputs,
            strategy,
            candidate_pairs.len(),
            Some((t0, end)),
        )?;
        for (idx, pair) in candidate_pairs.iter().enumerate() {
            let metric = metric_map.get(pair).ok_or_else(|| {
                format!(
                    "GPU base composite CSCV/PBO missing metric for pair ({}, {}) at slice {}",
                    pair.0, pair.1, slice_idx
                )
            })?;
            let key = candidate_keys.get(idx).ok_or_else(|| {
                "GPU base composite CSCV/PBO candidate key index out of bounds".to_string()
            })?;
            if let Some(slice_vec) = out.get_mut(key) {
                slice_vec[slice_idx] = slice_return_stats_from_metrics(metric, slice_bars);
            }
        }
    }

    Ok(Some(out))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_base_composite_gpu_spa_slice_sums(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    inputs: &BaseCompositeGpuValidationInputs,
    strategy: &StrategyConfig,
    candidate_keys: &[Vec<u8>],
    candidate_pairs: &[(u16, u16)],
    start_ts: &[usize],
    cfg: &SpaConfigOptional,
) -> Result<Option<HashMap<Vec<u8>, Vec<f64>>>, String> {
    let slices = cfg.slices.max(4);
    let common_start_t = start_ts.iter().copied().max().unwrap_or(0);
    let slice_bars = candles.close.len().saturating_sub(common_start_t) / slices;
    if slice_bars < 2 || candidate_pairs.is_empty() {
        return Ok(None);
    }

    let mut out: HashMap<Vec<u8>, Vec<f64>> = HashMap::with_capacity(candidate_keys.len());
    for key in candidate_keys {
        out.entry(key.clone()).or_insert_with(|| vec![0.0; slices]);
    }

    let nf = slice_bars as f64;
    for slice_idx in 0..slices {
        let t0 = common_start_t + slice_idx * slice_bars;
        let end = t0.saturating_add(slice_bars).min(candles.close.len());
        let metric_map = base_composite_gpu_metric_map(
            req,
            candles,
            inputs,
            strategy,
            candidate_pairs.len(),
            Some((t0, end)),
        )?;
        for (idx, pair) in candidate_pairs.iter().enumerate() {
            let metric = metric_map.get(pair).ok_or_else(|| {
                format!(
                    "GPU base composite SPA missing metric for pair ({}, {}) at slice {}",
                    pair.0, pair.1, slice_idx
                )
            })?;
            let key = candidate_keys.get(idx).ok_or_else(|| {
                "GPU base composite SPA candidate key index out of bounds".to_string()
            })?;
            if let Some(slice_vec) = out.get_mut(key) {
                slice_vec[slice_idx] = metric.sharpe * metric.volatility * nf;
            }
        }
    }

    Ok(Some(out))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_base_composite_gpu_reopt_walk_forward_stats(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    inputs: &BaseCompositeGpuValidationInputs,
    finalist_params: &[CompositeParams],
    finalist_start_ts: &[usize],
    cfg: &ReoptWalkForwardConfigOptional,
) -> Result<Option<ReoptWalkForwardStats>, String> {
    if finalist_params.is_empty() || candles.close.len() < 3 {
        return Ok(None);
    }

    let top_n = cfg.top_n_candidates.max(1).min(finalist_params.len());
    let finalists = &finalist_params[..top_n];
    let finalist_start_ts = &finalist_start_ts[..top_n];
    let first = finalists
        .first()
        .ok_or_else(|| "GPU base composite reopt_walk_forward requires finalists".to_string())?;

    let mut finalist_entries: Vec<(String, (u16, u16), usize)> =
        Vec::with_capacity(finalists.len());
    for (idx, cand) in finalists.iter().enumerate() {
        if !is_plain_base_composite_candidate(cand) {
            return Err(
                "GPU base composite reopt_walk_forward currently supports plain CompositeCpu only"
                    .to_string(),
            );
        }
        if cand.signal_mode != CompositeSignalMode::HardGate {
            return Err(
                "GPU base composite reopt_walk_forward currently supports signal_mode=HardGate only"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU base composite reopt_walk_forward currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        let key = serde_json::to_string(cand)
            .unwrap_or_else(|_| format!("base:{}:{}", cand.fast_len, cand.slow_len));
        finalist_entries.push((key, (cand.fast_len, cand.slow_len), finalist_start_ts[idx]));
    }

    let n = candles.close.len();
    let common_start_t = finalist_start_ts
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .min(n.saturating_sub(2));
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);

    let mut ranges: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut t0 = common_start_t;
    while t0 + 2 < n {
        let train_end = t0.saturating_add(window).min(n);
        if train_end <= t0 + 1 {
            break;
        }
        let test_end = train_end.saturating_add(step).min(n);
        if test_end <= train_end + 1 {
            break;
        }
        ranges.insert((t0, train_end));
        ranges.insert((train_end, test_end));
        if test_end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }
    if ranges.is_empty() {
        return Ok(None);
    }

    let mut range_metrics: HashMap<(usize, usize), HashMap<(u16, u16), Metrics>> =
        HashMap::with_capacity(ranges.len());
    for (start, end) in ranges {
        let metric_map = base_composite_gpu_metric_map(
            req,
            candles,
            inputs,
            &first.strategy,
            finalists.len(),
            Some((start, end)),
        )?;
        range_metrics.insert((start, end), metric_map);
    }

    crate::validation_pipeline::compute_reopt_walk_forward_stats(
        &finalist_entries,
        n,
        cfg,
        |(_, _, start_t)| *start_t,
        |(_, pair, _), start, end| {
            range_metrics
                .get(&(start, end))
                .and_then(|m| m.get(pair))
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "missing GPU base composite reopt metrics for pair ({}, {}) in range {}..{}",
                        pair.0, pair.1, start, end
                    )
                })
        },
        |(key, _, _)| key.clone(),
    )
    .ok_or_else(|| "GPU base composite reopt_walk_forward did not produce any windows".to_string())
    .map(Some)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_asr_gpu_walk_forward_stats_for_candidate(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    params: &CompositeParams,
    cfg: &WalkForwardConfigOptional,
) -> Result<WalkForwardStats, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    let asr = params
        .asr
        .as_ref()
        .ok_or_else(|| "ASR GPU walk_forward requires ASR candidate params".to_string())?;
    validate_strategy_cfg(&params.strategy)?;
    validate_asr_params(asr)?;
    if params.strategy.uses_execution_overlays()
        || params.strategy.stop_loss_pct().is_some()
        || params.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite ASR walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    let n = close.len();
    if n < 2 || ma_prices.len() != n {
        return Err("not enough candles or mismatched MA source length".to_string());
    }
    if n > i32::MAX as usize {
        return Err("ASR GPU walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let kaufman =
        compute_asr_kaufman_stops_cuda_device(&prepared_cuda_inputs, &params.strategy, device_id)?;
    let periods = resolve_asr_periods(asr, params.fast_len, params.slow_len);
    let fast_squeeze_sweep = compute_asr_ttm_squeeze_sweep_cuda_device(
        &prepared_cuda_inputs,
        asr,
        periods.squeeze,
        periods.squeeze,
        1,
        device_id,
    )?;
    let fast_momentum_sweep = compute_asr_squeeze_momentum_sweep_cuda_device(
        &prepared_cuda_inputs,
        asr,
        periods.squeeze,
        periods.squeeze,
        1,
        device_id,
    )?;
    let slow_angle_sweep = compute_asr_linearreg_angle_sweep_cuda_device(
        &prepared_cuda_inputs,
        periods.trend,
        periods.trend,
        1,
        device_id,
    )?;
    let slow_er_sweep = compute_asr_er_sweep_cuda_device(
        &prepared_cuda_inputs,
        periods.er,
        periods.er,
        1,
        device_id,
    )?;
    let slow_z_sweep = compute_asr_zscore_sweep_cuda_device(
        &prepared_cuda_inputs,
        asr,
        periods.z,
        periods.z,
        1,
        device_id,
    )?;

    let fast_squeeze_row = fast_squeeze_sweep
        .row_ptr_for_period(periods.squeeze)
        .ok_or_else(|| {
            format!(
                "ASR GPU walk_forward missing squeeze row for period={}",
                periods.squeeze
            )
        })?;
    let fast_momentum_row = fast_momentum_sweep
        .row_ptr_for_period(periods.squeeze)
        .ok_or_else(|| {
            format!(
                "ASR GPU walk_forward missing momentum row for period={}",
                periods.squeeze
            )
        })?;
    let slow_angle_row = slow_angle_sweep
        .row_ptr_for_period(periods.trend)
        .ok_or_else(|| {
            format!(
                "ASR GPU walk_forward missing angle row for period={}",
                periods.trend
            )
        })?;
    let slow_er_row = slow_er_sweep
        .row_ptr_for_period(periods.er)
        .ok_or_else(|| {
            format!(
                "ASR GPU walk_forward missing ER row for period={}",
                periods.er
            )
        })?;
    let slow_z_row = slow_z_sweep.row_ptr_for_period(periods.z).ok_or_else(|| {
        format!(
            "ASR GPU walk_forward missing zscore row for period={}",
            periods.z
        )
    })?;

    let fast_first_valid = fast_squeeze_sweep
        .first_valid_for_period(periods.squeeze)
        .max(fast_momentum_sweep.first_valid_for_period(periods.squeeze));
    let slow_first_valid = slow_angle_sweep
        .first_valid_for_period(periods.trend)
        .max(slow_er_sweep.first_valid_for_period(periods.er))
        .max(slow_z_sweep.first_valid_for_period(periods.z));
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
        fast_first_valid,
        slow_first_valid,
        kaufman.first_valid,
        max_period,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after ASR GPU walk_forward warmup".to_string());
    }

    let fast_squeeze_rows = [fast_squeeze_row];
    let fast_momentum_rows = [fast_momentum_row];
    let slow_angle_rows = [slow_angle_row];
    let slow_er_rows = [slow_er_row];
    let slow_z_rows = [slow_z_row];

    let signals = AsrGpuDeviceSignalMatrices {
        close_f64: close,
        cols: n,
        fast_rows: 1,
        slow_rows: 1,
        fast_squeeze_rows: &fast_squeeze_rows,
        fast_momentum_rows: &fast_momentum_rows,
        slow_angle_rows: &slow_angle_rows,
        slow_er_rows: &slow_er_rows,
        slow_z_rows: &slow_z_rows,
        kaufman_long_row: kaufman.long.device_ptr,
        kaufman_short_row: kaufman.short.device_ptr,
    };

    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let mut sharpes: Vec<f64> = Vec::new();
    let mut dds: Vec<f64> = Vec::new();
    let mut pnls: Vec<f64> = Vec::new();
    let mut trades: Vec<u32> = Vec::new();

    let mut t0 = start_t;
    while t0 + 2 <= n {
        let end = t0.saturating_add(window).min(n);
        if end <= t0 + 1 {
            break;
        }
        let combo_fast = [0i32];
        let combo_slow = [0i32];
        let combo_start = [i32::try_from(t0)
            .map_err(|_| "ASR GPU walk_forward start_t exceeds i32 range".to_string())?];
        let combo_end = [i32::try_from(end)
            .map_err(|_| "ASR GPU walk_forward end_t exceeds i32 range".to_string())?];
        let combos = AsrGpuComboBatch {
            fast_row_index: &combo_fast,
            slow_row_index: &combo_slow,
            start_t: &combo_start,
            end_t: &combo_end,
        };
        let metric = eval_asr_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &params.strategy,
            asr,
        )?
        .into_iter()
        .next()
        .ok_or_else(|| "ASR GPU walk_forward returned no metrics".to_string())?;

        sharpes.push(metric.sharpe);
        dds.push(metric.max_dd);
        pnls.push(metric.pnl);
        trades.push(metric.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    sharpes.sort_by(|a, b| a.total_cmp(b));
    dds.sort_by(|a, b| a.total_cmp(b));
    pnls.sort_by(|a, b| a.total_cmp(b));

    Ok(WalkForwardStats {
        windows: sharpes.len(),
        sharpe_median: crate::stats_common::median(&sharpes),
        sharpe_worst: crate::stats_common::worst_min(&sharpes),
        max_dd_median: crate::stats_common::median(&dds),
        max_dd_worst: crate::stats_common::worst_max(&dds),
        pnl_median: crate::stats_common::median(&pnls),
        pnl_worst: crate::stats_common::worst_min(&pnls),
        trades_median: crate::stats_common::median_u32(&trades),
        trades_worst: crate::stats_common::worst_min_u32(&trades),
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_asr_gpu_reopt_walk_forward_stats_for_candidates(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    finalist_params: &[CompositeParams],
    finalist_start_ts: &[usize],
    cfg: &ReoptWalkForwardConfigOptional,
) -> Result<Option<ReoptWalkForwardStats>, String> {
    if finalist_params.is_empty() || candles.close.len() < 3 {
        return Ok(None);
    }

    let top_n = cfg.top_n_candidates.max(1).min(finalist_params.len());
    let finalists = &finalist_params[..top_n];
    let finalist_start_ts = &finalist_start_ts[..top_n];
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    let first = &finalists[0];
    let first_asr = first
        .asr
        .as_ref()
        .ok_or_else(|| "ASR GPU reopt_walk_forward requires ASR candidate params".to_string())?;
    validate_strategy_cfg(&first.strategy)?;
    validate_asr_params(first_asr)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite ASR reopt_walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    let n = close.len();
    if n < 2 || ma_prices.len() != n {
        return Err("not enough candles or mismatched MA source length".to_string());
    }
    if n > i32::MAX as usize {
        return Err("ASR GPU reopt_walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let mut min_sq = u16::MAX;
    let mut max_sq = 0u16;
    let mut min_trend = u16::MAX;
    let mut max_trend = 0u16;
    let mut min_er = u16::MAX;
    let mut max_er = 0u16;
    let mut min_z = u16::MAX;
    let mut max_z = 0u16;
    let mut periods_by_candidate: Vec<(u16, u16, u16, u16)> = Vec::with_capacity(finalists.len());
    let mut finalist_entries: Vec<(String, Vec<u8>, usize)> = Vec::with_capacity(finalists.len());

    for (idx, cand) in finalists.iter().enumerate() {
        let asr = cand.asr.as_ref().ok_or_else(|| {
            "GPU composite reopt_walk_forward currently requires all finalists to use ASR"
                .to_string()
        })?;
        if cand.vcc.is_some() || cand.vemr.is_some() || cand.rocb.is_some() {
            return Err(
                "GPU composite reopt_walk_forward currently requires all finalists to use ASR"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        if !serialized_eq(asr, first_asr)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires uniform ASR parameters"
                    .to_string(),
            );
        }

        let periods = resolve_asr_periods(first_asr, cand.fast_len, cand.slow_len);
        min_sq = min_sq.min(periods.squeeze);
        max_sq = max_sq.max(periods.squeeze);
        min_trend = min_trend.min(periods.trend);
        max_trend = max_trend.max(periods.trend);
        min_er = min_er.min(periods.er);
        max_er = max_er.max(periods.er);
        min_z = min_z.min(periods.z);
        max_z = max_z.max(periods.z);
        periods_by_candidate.push((periods.squeeze, periods.trend, periods.er, periods.z));

        let key = serde_json::to_string(cand)
            .unwrap_or_else(|_| format!("asr:{}:{}", cand.fast_len, cand.slow_len));
        finalist_entries.push((
            key,
            serialize_composite_candidate_key(cand)?,
            finalist_start_ts[idx],
        ));
    }

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let kaufman =
        compute_asr_kaufman_stops_cuda_device(&prepared_cuda_inputs, &first.strategy, device_id)?;
    let fast_squeeze_sweep = compute_asr_ttm_squeeze_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_asr,
        min_sq,
        max_sq,
        1,
        device_id,
    )?;
    let fast_momentum_sweep = compute_asr_squeeze_momentum_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_asr,
        min_sq,
        max_sq,
        1,
        device_id,
    )?;
    let slow_angle_sweep = compute_asr_linearreg_angle_sweep_cuda_device(
        &prepared_cuda_inputs,
        min_trend,
        max_trend,
        1,
        device_id,
    )?;
    let slow_er_sweep =
        compute_asr_er_sweep_cuda_device(&prepared_cuda_inputs, min_er, max_er, 1, device_id)?;
    let slow_z_sweep = compute_asr_zscore_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_asr,
        min_z,
        max_z,
        1,
        device_id,
    )?;

    let mut fast_squeeze_rows: Vec<u64> = Vec::new();
    let mut fast_momentum_rows: Vec<u64> = Vec::new();
    let mut slow_angle_rows: Vec<u64> = Vec::new();
    let mut slow_er_rows: Vec<u64> = Vec::new();
    let mut slow_z_rows: Vec<u64> = Vec::new();
    let mut fast_lookup: HashMap<u16, i32> = HashMap::new();
    let mut slow_lookup: HashMap<(u16, u16, u16), i32> = HashMap::new();

    let mut fast_row_index: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut slow_row_index: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(finalists.len());

    for ((sq, trend, er, z), cand) in periods_by_candidate.into_iter().zip(finalists.iter()) {
        let fast_idx = if let Some(idx) = fast_lookup.get(&sq) {
            *idx
        } else {
            let sq_ptr = fast_squeeze_sweep
                .row_ptr_for_period(sq)
                .ok_or_else(|| format!("ASR GPU reopt missing squeeze row for period={sq}"))?;
            let mo_ptr = fast_momentum_sweep
                .row_ptr_for_period(sq)
                .ok_or_else(|| format!("ASR GPU reopt missing momentum row for period={sq}"))?;
            let idx = i32::try_from(fast_squeeze_rows.len())
                .map_err(|_| "ASR GPU reopt fast row index exceeds i32 range".to_string())?;
            fast_squeeze_rows.push(sq_ptr);
            fast_momentum_rows.push(mo_ptr);
            fast_lookup.insert(sq, idx);
            idx
        };

        let slow_key = (trend, er, z);
        let slow_idx = if let Some(idx) = slow_lookup.get(&slow_key) {
            *idx
        } else {
            let angle_ptr = slow_angle_sweep
                .row_ptr_for_period(trend)
                .ok_or_else(|| format!("ASR GPU reopt missing angle row for period={trend}"))?;
            let er_ptr = slow_er_sweep
                .row_ptr_for_period(er)
                .ok_or_else(|| format!("ASR GPU reopt missing ER row for period={er}"))?;
            let z_ptr = slow_z_sweep
                .row_ptr_for_period(z)
                .ok_or_else(|| format!("ASR GPU reopt missing zscore row for period={z}"))?;
            let idx = i32::try_from(slow_angle_rows.len())
                .map_err(|_| "ASR GPU reopt slow row index exceeds i32 range".to_string())?;
            slow_angle_rows.push(angle_ptr);
            slow_er_rows.push(er_ptr);
            slow_z_rows.push(z_ptr);
            slow_lookup.insert(slow_key, idx);
            idx
        };

        let fast_first_valid = fast_squeeze_sweep
            .first_valid_for_period(sq)
            .max(fast_momentum_sweep.first_valid_for_period(sq));
        let slow_first_valid = slow_angle_sweep
            .first_valid_for_period(trend)
            .max(slow_er_sweep.first_valid_for_period(er))
            .max(slow_z_sweep.first_valid_for_period(z));
        let max_period = usize::from(sq.max(trend).max(er).max(z)).max(1);
        let start_t = compute_asr_start_t(
            first_valid_close,
            first_valid_source,
            fast_first_valid,
            slow_first_valid,
            kaufman.first_valid,
            max_period,
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after ASR GPU reopt warmup".to_string());
        }
        fast_row_index.push(fast_idx);
        slow_row_index.push(slow_idx);
        combo_start_t.push(
            i32::try_from(start_t)
                .map_err(|_| "ASR GPU reopt start_t exceeds i32 range".to_string())?,
        );
    }

    let signals = AsrGpuDeviceSignalMatrices {
        close_f64: close,
        cols: n,
        fast_rows: fast_squeeze_rows.len(),
        slow_rows: slow_angle_rows.len(),
        fast_squeeze_rows: &fast_squeeze_rows,
        fast_momentum_rows: &fast_momentum_rows,
        slow_angle_rows: &slow_angle_rows,
        slow_er_rows: &slow_er_rows,
        slow_z_rows: &slow_z_rows,
        kaufman_long_row: kaufman.long.device_ptr,
        kaufman_short_row: kaufman.short.device_ptr,
    };

    let common_start_t = finalist_start_ts
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .min(n.saturating_sub(2));
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let mut ranges: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut t0 = common_start_t;
    while t0 + 2 < n {
        let train_end = t0.saturating_add(window).min(n);
        if train_end <= t0 + 1 {
            break;
        }
        let test_end = train_end.saturating_add(step).min(n);
        if test_end <= train_end + 1 {
            break;
        }
        ranges.insert((t0, train_end));
        ranges.insert((train_end, test_end));
        if test_end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }
    if ranges.is_empty() {
        return Ok(None);
    }

    let candidate_keys: Vec<Vec<u8>> = finalist_entries
        .iter()
        .map(|(_, key, _)| key.clone())
        .collect();
    let mut range_metrics: HashMap<(usize, usize), HashMap<Vec<u8>, Metrics>> =
        HashMap::with_capacity(ranges.len());
    for (start, end) in ranges {
        let end_i =
            i32::try_from(end).map_err(|_| "ASR GPU reopt end_t exceeds i32 range".to_string())?;
        let slice_start: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start_i = usize::try_from(st).unwrap_or(0).max(start);
                i32::try_from(start_i)
                    .map_err(|_| "ASR GPU reopt start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let slice_end = vec![end_i; slice_start.len()];
        let combos = AsrGpuComboBatch {
            fast_row_index: &fast_row_index,
            slow_row_index: &slow_row_index,
            start_t: &slice_start,
            end_t: &slice_end,
        };
        let metrics = eval_asr_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &first.strategy,
            first_asr,
        )?;
        let mut keyed = HashMap::with_capacity(metrics.len());
        for (key, metric) in candidate_keys.iter().zip(metrics.into_iter()) {
            keyed.insert(key.clone(), metric);
        }
        range_metrics.insert((start, end), keyed);
    }

    crate::validation_pipeline::compute_reopt_walk_forward_stats(
        &finalist_entries,
        n,
        cfg,
        |(_, _, start_t)| *start_t,
        |(_, key, _), start, end| {
            range_metrics
                .get(&(start, end))
                .and_then(|m| m.get(key))
                .cloned()
                .unwrap_or_else(|| {
                    panic!("missing ASR GPU reopt metrics for range {}..{}", start, end)
                })
        },
        |(key, _, _)| key.clone(),
    )
    .ok_or_else(|| "ASR GPU reopt_walk_forward did not produce any windows".to_string())
    .map(Some)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_asr_gpu_cscv_slice_stats_for_candidates(
    req: &CompositeValidationRequest,
    signals: &AsrGpuDeviceSignalMatrices<'_>,
    fast_row_index: &[i32],
    slow_row_index: &[i32],
    combo_start_t: &[i32],
    start_ts: &[usize],
    asr: &AsrStrategyParams,
    device_id: u32,
    cfg: &CscvPboConfig,
) -> Result<Option<HashMap<Vec<u8>, Vec<SliceReturnStats>>>, String> {
    let slices = sanitize_gpu_cscv_slices(cfg.slices);
    let common_start_t = start_ts.iter().copied().max().unwrap_or(0);
    let slice_bars = signals.cols.saturating_sub(common_start_t) / slices;
    if slice_bars < 2 || req.candidates.len() < 2 {
        return Ok(None);
    }

    let candidate_keys: Vec<Vec<u8>> = req
        .candidates
        .iter()
        .map(serialize_composite_candidate_key)
        .collect::<Result<_, _>>()?;
    let mut out: HashMap<Vec<u8>, Vec<SliceReturnStats>> =
        HashMap::with_capacity(candidate_keys.len());
    for key in &candidate_keys {
        out.insert(key.clone(), vec![SliceReturnStats::default(); slices]);
    }

    for slice_idx in 0..slices {
        let t0 = common_start_t + slice_idx * slice_bars;
        let end = t0.saturating_add(slice_bars).min(signals.cols);
        let end_i =
            i32::try_from(end).map_err(|_| "ASR GPU CSCV end_t exceeds i32 range".to_string())?;
        let slice_start: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start = usize::try_from(st).unwrap_or(0).max(t0);
                i32::try_from(start)
                    .map_err(|_| "ASR GPU CSCV start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let slice_end = vec![end_i; slice_start.len()];
        let combos = AsrGpuComboBatch {
            fast_row_index,
            slow_row_index,
            start_t: &slice_start,
            end_t: &slice_end,
        };
        let metrics = eval_asr_metrics_gpu_kernel_device_rows(
            device_id,
            signals,
            &combos,
            &req.candidates[0].strategy,
            asr,
        )?;
        for ((key, metric), _) in candidate_keys
            .iter()
            .zip(metrics.iter())
            .zip(req.candidates.iter())
        {
            if let Some(slice_vec) = out.get_mut(key) {
                slice_vec[slice_idx] = slice_return_stats_from_metrics(metric, slice_bars);
            }
        }
    }

    Ok(Some(out))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn try_prepare_rocb_gpu_fixed_candidate_validation(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<GpuCompositeFixedCandidateValidationEval, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    if let Some(reason) = composite_gpu_validation_feature_blocker(req) {
        return Err(reason);
    }
    if req.candidates.is_empty() {
        return Err("no candidates to validate".to_string());
    }

    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    let first_rocb = first.rocb.as_ref().ok_or_else(|| {
        "GPU composite validation currently supports ROCB candidates only".to_string()
    })?;
    if first.asr.is_some() || first.vcc.is_some() || first.vemr.is_some() {
        return Err("GPU composite validation currently supports ROCB candidates only".to_string());
    }
    validate_strategy_cfg(&first.strategy)?;
    validate_rocb_params(first_rocb)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite ROCB validation currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let mut min_channel = u16::MAX;
    let mut max_channel = 0u16;
    let mut min_atr = u16::MAX;
    let mut max_atr = 0u16;
    let mut periods_by_candidate: Vec<(u16, u16)> = Vec::with_capacity(req.candidates.len());

    for cand in &req.candidates {
        if cand.rocb.is_none() || cand.asr.is_some() || cand.vcc.is_some() || cand.vemr.is_some() {
            return Err(
                "GPU composite validation currently requires all candidates to use ROCB"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite validation currently requires a uniform StrategyConfig".to_string(),
            );
        }
        if !serialized_eq(cand.rocb.as_ref().unwrap(), first_rocb)? {
            return Err(
                "GPU composite validation currently requires uniform ROCB parameters".to_string(),
            );
        }

        let periods = resolve_rocb_periods(first_rocb, cand.fast_len, cand.slow_len);
        min_channel = min_channel.min(periods.channel);
        max_channel = max_channel.max(periods.channel);
        min_atr = min_atr.min(periods.atr);
        max_atr = max_atr.max(periods.atr);
        periods_by_candidate.push((periods.channel, periods.atr));
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let high = vector_ta::utilities::data_loader::source_type(candles, "high");
    let low = vector_ta::utilities::data_loader::source_type(candles, "low");
    let n = close.len();
    if n < 2 || high.len() != n || low.len() != n {
        return Err("not enough candles or mismatched OHLC series lengths".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_high =
        first_finite_idx(high).ok_or_else(|| "high series all non-finite".to_string())?;
    let first_valid_low =
        first_finite_idx(low).ok_or_else(|| "low series all non-finite".to_string())?;
    let close_f32: Vec<f32> = close.iter().map(|v| *v as f32).collect();

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let upper_sweep = compute_rocb_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["upper_band", "upper", "up"],
        min_channel,
        max_channel,
        1,
    )?;
    let lower_sweep = compute_rocb_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["lower_band", "lower", "down"],
        min_channel,
        max_channel,
        1,
    )?;
    let atr_sweep = compute_rocb_atr_sweep_cuda_device(&prepared_cuda_inputs, min_atr, max_atr, 1)?;

    let mut channel_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut atr_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut start_ts: Vec<usize> = Vec::with_capacity(req.candidates.len());

    for (channel_period, atr_period) in periods_by_candidate {
        let channel_row = upper_sweep
            .row_index_for_period(channel_period)
            .ok_or_else(|| {
                format!("ROCB GPU validation missing channel row for period={channel_period}")
            })?;
        let atr_row = atr_sweep.row_index_for_period(atr_period).ok_or_else(|| {
            format!("ROCB GPU validation missing ATR row for period={atr_period}")
        })?;
        let start_t = compute_rocb_start_t(
            first_valid_close,
            first_valid_high,
            first_valid_low,
            upper_sweep
                .first_valid_for_period(channel_period)
                .max(lower_sweep.first_valid_for_period(channel_period)),
            atr_sweep.first_valid_for_period(atr_period),
            first.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after ROCB GPU warmup".to_string());
        }

        channel_row_index.push(
            i32::try_from(channel_row)
                .map_err(|_| "ROCB GPU channel row index exceeds i32 range".to_string())?,
        );
        atr_row_index.push(
            i32::try_from(atr_row)
                .map_err(|_| "ROCB GPU ATR row index exceeds i32 range".to_string())?,
        );
        combo_start_t.push(
            i32::try_from(start_t).map_err(|_| "ROCB GPU start_t exceeds i32 range".to_string())?,
        );
        start_ts.push(start_t);
    }

    let signals = RocbGpuDeviceSignalMatrices {
        close_f32: &close_f32,
        cols: n,
        upper_row_major_ptr: upper_sweep.matrix.device_ptr,
        lower_row_major_ptr: lower_sweep.matrix.device_ptr,
        channel_rows: upper_sweep.matrix.rows,
        channel_stride: upper_sweep.matrix.cols,
        atr_row_major_ptr: atr_sweep.matrix.device_ptr,
        atr_rows: atr_sweep.matrix.rows,
        atr_stride: atr_sweep.matrix.cols,
    };
    let combo_end_t = vec![
        i32::try_from(n)
            .map_err(|_| "ROCB GPU end_t exceeds i32 range".to_string())?;
        channel_row_index.len()
    ];
    let combos = RocbGpuComboBatch {
        channel_row_index: &channel_row_index,
        atr_row_index: &atr_row_index,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };
    let metrics = eval_rocb_metrics_gpu_kernel_device_rows(
        device_id,
        &signals,
        &combos,
        &first.strategy,
        first_rocb,
    )?;
    if metrics.len() != req.candidates.len() {
        return Err("ROCB GPU validation output length mismatch".to_string());
    }
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        compute_rocb_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &channel_row_index,
            &atr_row_index,
            &combo_start_t,
            &start_ts,
            first_rocb,
            device_id,
            cscv_cfg,
        )?
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        let slice_stats = compute_rocb_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &channel_row_index,
            &atr_row_index,
            &combo_start_t,
            &start_ts,
            first_rocb,
            device_id,
            &CscvPboConfig {
                slices: spa_cfg.slices.max(4),
                max_combinations: usize::MAX,
                threshold: 0.0,
            },
        )?;
        slice_stats.as_ref().map(spa_slice_sums_from_slice_stats)
    } else {
        None
    };

    let backend_route = match req.backend {
        Backend::Auto { .. } => Some("Auto->GpuKernel".to_string()),
        _ => Some(validation_backend_label(&req.backend).to_string()),
    };

    Ok(GpuCompositeFixedCandidateValidationEval {
        metrics,
        start_ts,
        candidate_indices: composite_candidate_index_map(&req.candidates)?,
        walk_forward_stats: None,
        cscv_slices,
        spa_slice_sums,
        base_composite_ctx: None,
        backend_used: BackendUsed::Gpu,
        backend_route,
        backend_fallback_reason: None,
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn try_prepare_mkcs_gpu_fixed_candidate_validation(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<GpuCompositeFixedCandidateValidationEval, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    if let Some(reason) = composite_gpu_validation_feature_blocker(req) {
        return Err(reason);
    }
    if req.candidates.is_empty() {
        return Err("no candidates to validate".to_string());
    }

    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    let first_mkcs = first.mkcs.as_ref().ok_or_else(|| {
        "GPU composite validation currently supports MKCS candidates only".to_string()
    })?;
    if first.asr.is_some() || first.vcc.is_some() || first.vemr.is_some() || first.rocb.is_some() {
        return Err("GPU composite validation currently supports MKCS candidates only".to_string());
    }
    validate_strategy_cfg(&first.strategy)?;
    validate_mkcs_params(first_mkcs)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite MKCS validation currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let mut min_stddev = u16::MAX;
    let mut max_stddev = 0u16;
    let mut min_breakout = u16::MAX;
    let mut max_breakout = 0u16;
    let mut periods_by_candidate: Vec<(u16, u16)> = Vec::with_capacity(req.candidates.len());

    for cand in &req.candidates {
        if cand.mkcs.is_none()
            || cand.asr.is_some()
            || cand.vcc.is_some()
            || cand.vemr.is_some()
            || cand.rocb.is_some()
        {
            return Err(
                "GPU composite validation currently requires all candidates to use MKCS"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite validation currently requires a uniform StrategyConfig".to_string(),
            );
        }
        if !serialized_eq(cand.mkcs.as_ref().unwrap(), first_mkcs)? {
            return Err(
                "GPU composite validation currently requires uniform MKCS parameters".to_string(),
            );
        }

        let periods = resolve_mkcs_periods(first_mkcs, cand.fast_len, cand.slow_len);
        min_stddev = min_stddev.min(periods.stddev);
        max_stddev = max_stddev.max(periods.stddev);
        min_breakout = min_breakout.min(periods.breakout);
        max_breakout = max_breakout.max(periods.breakout);
        periods_by_candidate.push((periods.stddev, periods.breakout));
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let high = vector_ta::utilities::data_loader::source_type(candles, "high");
    let low = vector_ta::utilities::data_loader::source_type(candles, "low");
    let n = close.len();
    if n < 2 || high.len() != n || low.len() != n {
        return Err("not enough candles or mismatched OHLC series lengths".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_high =
        first_finite_idx(high).ok_or_else(|| "high series all non-finite".to_string())?;
    let first_valid_low =
        first_finite_idx(low).ok_or_else(|| "low series all non-finite".to_string())?;
    let close_f32: Vec<f32> = close.iter().map(|v| *v as f32).collect();

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let stddev_sweep =
        compute_mkcs_stddev_sweep_cuda_device(&prepared_cuda_inputs, min_stddev, max_stddev, 1)?;
    let upper_sweep = compute_mkcs_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["upper_band", "upper", "up"],
        min_breakout,
        max_breakout,
        1,
    )?;
    let lower_sweep = compute_mkcs_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["lower_band", "lower", "down"],
        min_breakout,
        max_breakout,
        1,
    )?;

    let mut stddev_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut breakout_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut start_ts: Vec<usize> = Vec::with_capacity(req.candidates.len());

    for (stddev_period, breakout_period) in periods_by_candidate {
        let stddev_row = stddev_sweep
            .row_index_for_period(stddev_period)
            .ok_or_else(|| {
                format!("MKCS GPU validation missing stddev row for period={stddev_period}")
            })?;
        let breakout_row = upper_sweep
            .row_index_for_period(breakout_period)
            .ok_or_else(|| {
                format!("MKCS GPU validation missing breakout row for period={breakout_period}")
            })?;
        let start_t = compute_mkcs_start_t(
            first_valid_close,
            first_valid_high,
            first_valid_low,
            stddev_sweep.first_valid_for_period(stddev_period),
            upper_sweep
                .first_valid_for_period(breakout_period)
                .max(lower_sweep.first_valid_for_period(breakout_period)),
            first.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after MKCS GPU warmup".to_string());
        }

        stddev_row_index.push(
            i32::try_from(stddev_row)
                .map_err(|_| "MKCS GPU stddev row index exceeds i32 range".to_string())?,
        );
        breakout_row_index.push(
            i32::try_from(breakout_row)
                .map_err(|_| "MKCS GPU breakout row index exceeds i32 range".to_string())?,
        );
        combo_start_t.push(
            i32::try_from(start_t).map_err(|_| "MKCS GPU start_t exceeds i32 range".to_string())?,
        );
        start_ts.push(start_t);
    }

    let signals = MkcsGpuDeviceSignalMatrices {
        close_f32: &close_f32,
        cols: n,
        stddev_row_major_ptr: stddev_sweep.matrix.device_ptr,
        stddev_rows: stddev_sweep.matrix.rows,
        stddev_stride: stddev_sweep.matrix.cols,
        upper_row_major_ptr: upper_sweep.matrix.device_ptr,
        lower_row_major_ptr: lower_sweep.matrix.device_ptr,
        breakout_rows: upper_sweep.matrix.rows,
        breakout_stride: upper_sweep.matrix.cols,
    };
    let combo_end_t = vec![
        i32::try_from(n)
            .map_err(|_| "MKCS GPU end_t exceeds i32 range".to_string())?;
        stddev_row_index.len()
    ];
    let combos = MkcsGpuComboBatch {
        stddev_row_index: &stddev_row_index,
        breakout_row_index: &breakout_row_index,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };
    let metrics = eval_mkcs_metrics_gpu_kernel_device_rows(
        device_id,
        &signals,
        &combos,
        &first.strategy,
        first_mkcs,
    )?;
    if metrics.len() != req.candidates.len() {
        return Err("MKCS GPU validation output length mismatch".to_string());
    }
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        compute_mkcs_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &stddev_row_index,
            &breakout_row_index,
            &combo_start_t,
            &start_ts,
            first_mkcs,
            device_id,
            cscv_cfg,
        )?
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        let slice_stats = compute_mkcs_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &stddev_row_index,
            &breakout_row_index,
            &combo_start_t,
            &start_ts,
            first_mkcs,
            device_id,
            &CscvPboConfig {
                slices: spa_cfg.slices.max(4),
                max_combinations: usize::MAX,
                threshold: 0.0,
            },
        )?;
        slice_stats.as_ref().map(spa_slice_sums_from_slice_stats)
    } else {
        None
    };

    let backend_route = match req.backend {
        Backend::Auto { .. } => Some("Auto->GpuKernel".to_string()),
        _ => Some(validation_backend_label(&req.backend).to_string()),
    };

    Ok(GpuCompositeFixedCandidateValidationEval {
        metrics,
        start_ts,
        candidate_indices: composite_candidate_index_map(&req.candidates)?,
        walk_forward_stats: None,
        cscv_slices,
        spa_slice_sums,
        base_composite_ctx: None,
        backend_used: BackendUsed::Gpu,
        backend_route,
        backend_fallback_reason: None,
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_mkcs_gpu_walk_forward_stats_for_candidate(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    params: &CompositeParams,
    cfg: &WalkForwardConfigOptional,
) -> Result<WalkForwardStats, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    let mkcs = params
        .mkcs
        .as_ref()
        .ok_or_else(|| "MKCS GPU walk_forward requires MKCS candidate params".to_string())?;
    validate_strategy_cfg(&params.strategy)?;
    validate_mkcs_params(mkcs)?;
    if params.strategy.uses_execution_overlays()
        || params.strategy.stop_loss_pct().is_some()
        || params.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite MKCS walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let high = vector_ta::utilities::data_loader::source_type(candles, "high");
    let low = vector_ta::utilities::data_loader::source_type(candles, "low");
    let n = close.len();
    if n < 2 || high.len() != n || low.len() != n {
        return Err("not enough candles or mismatched OHLC series lengths".to_string());
    }
    if n > i32::MAX as usize {
        return Err("MKCS GPU walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_high =
        first_finite_idx(high).ok_or_else(|| "high series all non-finite".to_string())?;
    let first_valid_low =
        first_finite_idx(low).ok_or_else(|| "low series all non-finite".to_string())?;
    let close_f32: Vec<f32> = close.iter().map(|v| *v as f32).collect();

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let periods = resolve_mkcs_periods(mkcs, params.fast_len, params.slow_len);
    let stddev_sweep = compute_mkcs_stddev_sweep_cuda_device(
        &prepared_cuda_inputs,
        periods.stddev,
        periods.stddev,
        1,
    )?;
    let upper_sweep = compute_mkcs_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["upper_band", "upper", "up"],
        periods.breakout,
        periods.breakout,
        1,
    )?;
    let lower_sweep = compute_mkcs_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["lower_band", "lower", "down"],
        periods.breakout,
        periods.breakout,
        1,
    )?;

    let stddev_row = stddev_sweep
        .row_index_for_period(periods.stddev)
        .ok_or_else(|| {
            format!(
                "MKCS GPU walk_forward missing stddev row for period={}",
                periods.stddev
            )
        })?;
    let breakout_row = upper_sweep
        .row_index_for_period(periods.breakout)
        .ok_or_else(|| {
            format!(
                "MKCS GPU walk_forward missing breakout row for period={}",
                periods.breakout
            )
        })?;
    let start_t = compute_mkcs_start_t(
        first_valid_close,
        first_valid_high,
        first_valid_low,
        stddev_sweep.first_valid_for_period(periods.stddev),
        upper_sweep
            .first_valid_for_period(periods.breakout)
            .max(lower_sweep.first_valid_for_period(periods.breakout)),
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after MKCS GPU walk_forward warmup".to_string());
    }

    let stddev_row_i = i32::try_from(stddev_row)
        .map_err(|_| "MKCS GPU walk_forward stddev row index exceeds i32 range".to_string())?;
    let breakout_row_i = i32::try_from(breakout_row)
        .map_err(|_| "MKCS GPU walk_forward breakout row index exceeds i32 range".to_string())?;

    let signals = MkcsGpuDeviceSignalMatrices {
        close_f32: &close_f32,
        cols: n,
        stddev_row_major_ptr: stddev_sweep.matrix.device_ptr,
        stddev_rows: stddev_sweep.matrix.rows,
        stddev_stride: stddev_sweep.matrix.cols,
        upper_row_major_ptr: upper_sweep.matrix.device_ptr,
        lower_row_major_ptr: lower_sweep.matrix.device_ptr,
        breakout_rows: upper_sweep.matrix.rows,
        breakout_stride: upper_sweep.matrix.cols,
    };

    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let mut sharpes: Vec<f64> = Vec::new();
    let mut dds: Vec<f64> = Vec::new();
    let mut pnls: Vec<f64> = Vec::new();
    let mut trades: Vec<u32> = Vec::new();

    let mut t0 = start_t;
    while t0 + 2 <= n {
        let end = t0.saturating_add(window).min(n);
        if end <= t0 + 1 {
            break;
        }
        let start_i = i32::try_from(t0)
            .map_err(|_| "MKCS GPU walk_forward start_t exceeds i32 range".to_string())?;
        let end_i = i32::try_from(end)
            .map_err(|_| "MKCS GPU walk_forward end_t exceeds i32 range".to_string())?;
        let combo_stddev = [stddev_row_i];
        let combo_breakout = [breakout_row_i];
        let combo_start = [start_i];
        let combo_end = [end_i];
        let combos = MkcsGpuComboBatch {
            stddev_row_index: &combo_stddev,
            breakout_row_index: &combo_breakout,
            start_t: &combo_start,
            end_t: &combo_end,
        };
        let metric = eval_mkcs_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &params.strategy,
            mkcs,
        )?
        .into_iter()
        .next()
        .ok_or_else(|| "MKCS GPU walk_forward returned no metrics".to_string())?;

        sharpes.push(metric.sharpe);
        dds.push(metric.max_dd);
        pnls.push(metric.pnl);
        trades.push(metric.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    sharpes.sort_by(|a, b| a.total_cmp(b));
    dds.sort_by(|a, b| a.total_cmp(b));
    pnls.sort_by(|a, b| a.total_cmp(b));

    Ok(WalkForwardStats {
        windows: sharpes.len(),
        sharpe_median: crate::stats_common::median(&sharpes),
        sharpe_worst: crate::stats_common::worst_min(&sharpes),
        max_dd_median: crate::stats_common::median(&dds),
        max_dd_worst: crate::stats_common::worst_max(&dds),
        pnl_median: crate::stats_common::median(&pnls),
        pnl_worst: crate::stats_common::worst_min(&pnls),
        trades_median: crate::stats_common::median_u32(&trades),
        trades_worst: crate::stats_common::worst_min_u32(&trades),
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_mkcs_gpu_reopt_walk_forward_stats_for_candidates(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    finalist_params: &[CompositeParams],
    finalist_start_ts: &[usize],
    cfg: &ReoptWalkForwardConfigOptional,
) -> Result<Option<ReoptWalkForwardStats>, String> {
    if finalist_params.is_empty() || candles.close.len() < 3 {
        return Ok(None);
    }

    let top_n = cfg.top_n_candidates.max(1).min(finalist_params.len());
    let finalists = &finalist_params[..top_n];
    let finalist_start_ts = &finalist_start_ts[..top_n];
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    let first = &finalists[0];
    let first_mkcs = first
        .mkcs
        .as_ref()
        .ok_or_else(|| "MKCS GPU reopt_walk_forward requires MKCS candidate params".to_string())?;
    validate_strategy_cfg(&first.strategy)?;
    validate_mkcs_params(first_mkcs)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite MKCS reopt_walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let high = vector_ta::utilities::data_loader::source_type(candles, "high");
    let low = vector_ta::utilities::data_loader::source_type(candles, "low");
    let n = close.len();
    if n < 2 || high.len() != n || low.len() != n {
        return Err("not enough candles or mismatched OHLC series lengths".to_string());
    }
    if n > i32::MAX as usize {
        return Err("MKCS GPU reopt_walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_high =
        first_finite_idx(high).ok_or_else(|| "high series all non-finite".to_string())?;
    let first_valid_low =
        first_finite_idx(low).ok_or_else(|| "low series all non-finite".to_string())?;
    let close_f32: Vec<f32> = close.iter().map(|v| *v as f32).collect();

    let mut min_stddev = u16::MAX;
    let mut max_stddev = 0u16;
    let mut min_breakout = u16::MAX;
    let mut max_breakout = 0u16;
    let mut periods_by_candidate: Vec<(u16, u16)> = Vec::with_capacity(finalists.len());
    let mut finalist_entries: Vec<(String, Vec<u8>, usize)> = Vec::with_capacity(finalists.len());

    for (idx, cand) in finalists.iter().enumerate() {
        let mkcs = cand.mkcs.as_ref().ok_or_else(|| {
            "GPU composite reopt_walk_forward currently requires all finalists to use MKCS"
                .to_string()
        })?;
        if cand.asr.is_some() || cand.vcc.is_some() || cand.vemr.is_some() || cand.rocb.is_some() {
            return Err(
                "GPU composite reopt_walk_forward currently requires all finalists to use MKCS"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        if !serialized_eq(mkcs, first_mkcs)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires uniform MKCS parameters"
                    .to_string(),
            );
        }

        let periods = resolve_mkcs_periods(first_mkcs, cand.fast_len, cand.slow_len);
        min_stddev = min_stddev.min(periods.stddev);
        max_stddev = max_stddev.max(periods.stddev);
        min_breakout = min_breakout.min(periods.breakout);
        max_breakout = max_breakout.max(periods.breakout);
        periods_by_candidate.push((periods.stddev, periods.breakout));

        let key = serde_json::to_string(cand)
            .unwrap_or_else(|_| format!("mkcs:{}:{}", cand.fast_len, cand.slow_len));
        finalist_entries.push((
            key,
            serialize_composite_candidate_key(cand)?,
            finalist_start_ts[idx],
        ));
    }

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let stddev_sweep =
        compute_mkcs_stddev_sweep_cuda_device(&prepared_cuda_inputs, min_stddev, max_stddev, 1)?;
    let upper_sweep = compute_mkcs_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["upper_band", "upper", "up"],
        min_breakout,
        max_breakout,
        1,
    )?;
    let lower_sweep = compute_mkcs_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["lower_band", "lower", "down"],
        min_breakout,
        max_breakout,
        1,
    )?;

    let mut stddev_row_index: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut breakout_row_index: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(finalists.len());

    for ((stddev, breakout), cand) in periods_by_candidate.into_iter().zip(finalists.iter()) {
        let stddev_row = stddev_sweep
            .row_index_for_period(stddev)
            .ok_or_else(|| format!("MKCS GPU reopt missing stddev row for period={stddev}"))?;
        let breakout_row = upper_sweep
            .row_index_for_period(breakout)
            .ok_or_else(|| format!("MKCS GPU reopt missing breakout row for period={breakout}"))?;
        let start_t = compute_mkcs_start_t(
            first_valid_close,
            first_valid_high,
            first_valid_low,
            stddev_sweep.first_valid_for_period(stddev),
            upper_sweep
                .first_valid_for_period(breakout)
                .max(lower_sweep.first_valid_for_period(breakout)),
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after MKCS GPU reopt warmup".to_string());
        }
        stddev_row_index.push(
            i32::try_from(stddev_row)
                .map_err(|_| "MKCS GPU reopt stddev row index exceeds i32 range".to_string())?,
        );
        breakout_row_index.push(
            i32::try_from(breakout_row)
                .map_err(|_| "MKCS GPU reopt breakout row index exceeds i32 range".to_string())?,
        );
        combo_start_t.push(
            i32::try_from(start_t)
                .map_err(|_| "MKCS GPU reopt start_t exceeds i32 range".to_string())?,
        );
    }

    let signals = MkcsGpuDeviceSignalMatrices {
        close_f32: &close_f32,
        cols: n,
        stddev_row_major_ptr: stddev_sweep.matrix.device_ptr,
        stddev_rows: stddev_sweep.matrix.rows,
        stddev_stride: stddev_sweep.matrix.cols,
        upper_row_major_ptr: upper_sweep.matrix.device_ptr,
        lower_row_major_ptr: lower_sweep.matrix.device_ptr,
        breakout_rows: upper_sweep.matrix.rows,
        breakout_stride: upper_sweep.matrix.cols,
    };

    let common_start_t = finalist_start_ts
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .min(n.saturating_sub(2));
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let mut ranges: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut t0 = common_start_t;
    while t0 + 2 < n {
        let train_end = t0.saturating_add(window).min(n);
        if train_end <= t0 + 1 {
            break;
        }
        let test_end = train_end.saturating_add(step).min(n);
        if test_end <= train_end + 1 {
            break;
        }
        ranges.insert((t0, train_end));
        ranges.insert((train_end, test_end));
        if test_end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }
    if ranges.is_empty() {
        return Ok(None);
    }

    let candidate_keys: Vec<Vec<u8>> = finalist_entries
        .iter()
        .map(|(_, key, _)| key.clone())
        .collect();
    let mut range_metrics: HashMap<(usize, usize), HashMap<Vec<u8>, Metrics>> =
        HashMap::with_capacity(ranges.len());
    for (start, end) in ranges {
        let start_vec: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start_i = usize::try_from(st).unwrap_or(0).max(start);
                i32::try_from(start_i)
                    .map_err(|_| "MKCS GPU reopt start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let end_i =
            i32::try_from(end).map_err(|_| "MKCS GPU reopt end_t exceeds i32 range".to_string())?;
        let end_vec = vec![end_i; start_vec.len()];
        let combos = MkcsGpuComboBatch {
            stddev_row_index: &stddev_row_index,
            breakout_row_index: &breakout_row_index,
            start_t: &start_vec,
            end_t: &end_vec,
        };
        let metrics = eval_mkcs_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &first.strategy,
            first_mkcs,
        )?;
        let mut keyed = HashMap::with_capacity(metrics.len());
        for (key, metric) in candidate_keys.iter().zip(metrics.into_iter()) {
            keyed.insert(key.clone(), metric);
        }
        range_metrics.insert((start, end), keyed);
    }

    crate::validation_pipeline::compute_reopt_walk_forward_stats(
        &finalist_entries,
        n,
        cfg,
        |(_, _, start_t)| *start_t,
        |(_, key, _), start, end| {
            range_metrics
                .get(&(start, end))
                .and_then(|m| m.get(key))
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "missing MKCS GPU reopt metrics for range {}..{}",
                        start, end
                    )
                })
        },
        |(key, _, _)| key.clone(),
    )
    .ok_or_else(|| "MKCS GPU reopt_walk_forward did not produce any windows".to_string())
    .map(Some)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_mkcs_gpu_cscv_slice_stats_for_candidates(
    req: &CompositeValidationRequest,
    signals: &MkcsGpuDeviceSignalMatrices<'_>,
    stddev_row_index: &[i32],
    breakout_row_index: &[i32],
    combo_start_t: &[i32],
    start_ts: &[usize],
    mkcs: &MkcsStrategyParams,
    device_id: u32,
    cfg: &CscvPboConfig,
) -> Result<Option<HashMap<Vec<u8>, Vec<SliceReturnStats>>>, String> {
    let slices = sanitize_gpu_cscv_slices(cfg.slices);
    let common_start_t = start_ts.iter().copied().max().unwrap_or(0);
    let slice_bars = signals.cols.saturating_sub(common_start_t) / slices;
    if slice_bars < 2 || req.candidates.len() < 2 {
        return Ok(None);
    }

    let candidate_keys: Vec<Vec<u8>> = req
        .candidates
        .iter()
        .map(serialize_composite_candidate_key)
        .collect::<Result<_, _>>()?;
    let mut out: HashMap<Vec<u8>, Vec<SliceReturnStats>> =
        HashMap::with_capacity(candidate_keys.len());
    for key in &candidate_keys {
        out.insert(key.clone(), vec![SliceReturnStats::default(); slices]);
    }

    for slice_idx in 0..slices {
        let t0 = common_start_t + slice_idx * slice_bars;
        let end = t0.saturating_add(slice_bars).min(signals.cols);
        let end_i =
            i32::try_from(end).map_err(|_| "MKCS GPU CSCV end_t exceeds i32 range".to_string())?;
        let slice_start: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start = usize::try_from(st).unwrap_or(0).max(t0);
                i32::try_from(start)
                    .map_err(|_| "MKCS GPU CSCV start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let slice_end = vec![end_i; slice_start.len()];
        let combos = MkcsGpuComboBatch {
            stddev_row_index,
            breakout_row_index,
            start_t: &slice_start,
            end_t: &slice_end,
        };
        let metrics = eval_mkcs_metrics_gpu_kernel_device_rows(
            device_id,
            signals,
            &combos,
            &req.candidates[0].strategy,
            mkcs,
        )?;
        for (key, metric) in candidate_keys.iter().zip(metrics.iter()) {
            if let Some(slice_vec) = out.get_mut(key) {
                slice_vec[slice_idx] = slice_return_stats_from_metrics(metric, slice_bars);
            }
        }
    }

    Ok(Some(out))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_rocb_gpu_walk_forward_stats_for_candidate(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    params: &CompositeParams,
    cfg: &WalkForwardConfigOptional,
) -> Result<WalkForwardStats, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    let rocb = params
        .rocb
        .as_ref()
        .ok_or_else(|| "ROCB GPU walk_forward requires ROCB candidate params".to_string())?;
    validate_strategy_cfg(&params.strategy)?;
    validate_rocb_params(rocb)?;
    if params.strategy.uses_execution_overlays()
        || params.strategy.stop_loss_pct().is_some()
        || params.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite ROCB walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let high = vector_ta::utilities::data_loader::source_type(candles, "high");
    let low = vector_ta::utilities::data_loader::source_type(candles, "low");
    let n = close.len();
    if n < 2 || high.len() != n || low.len() != n {
        return Err("not enough candles or mismatched OHLC series lengths".to_string());
    }
    if n > i32::MAX as usize {
        return Err("ROCB GPU walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_high =
        first_finite_idx(high).ok_or_else(|| "high series all non-finite".to_string())?;
    let first_valid_low =
        first_finite_idx(low).ok_or_else(|| "low series all non-finite".to_string())?;
    let close_f32: Vec<f32> = close.iter().map(|v| *v as f32).collect();

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let periods = resolve_rocb_periods(rocb, params.fast_len, params.slow_len);
    let upper_sweep = compute_rocb_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["upper_band", "upper", "up"],
        periods.channel,
        periods.channel,
        1,
    )?;
    let lower_sweep = compute_rocb_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["lower_band", "lower", "down"],
        periods.channel,
        periods.channel,
        1,
    )?;
    let atr_sweep =
        compute_rocb_atr_sweep_cuda_device(&prepared_cuda_inputs, periods.atr, periods.atr, 1)?;

    let channel_row = upper_sweep
        .row_index_for_period(periods.channel)
        .ok_or_else(|| {
            format!(
                "ROCB GPU walk_forward missing channel row for period={}",
                periods.channel
            )
        })?;
    let atr_row = atr_sweep.row_index_for_period(periods.atr).ok_or_else(|| {
        format!(
            "ROCB GPU walk_forward missing ATR row for period={}",
            periods.atr
        )
    })?;
    let start_t = compute_rocb_start_t(
        first_valid_close,
        first_valid_high,
        first_valid_low,
        upper_sweep
            .first_valid_for_period(periods.channel)
            .max(lower_sweep.first_valid_for_period(periods.channel)),
        atr_sweep.first_valid_for_period(periods.atr),
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after ROCB GPU walk_forward warmup".to_string());
    }

    let channel_row_i = i32::try_from(channel_row)
        .map_err(|_| "ROCB GPU walk_forward channel row index exceeds i32 range".to_string())?;
    let atr_row_i = i32::try_from(atr_row)
        .map_err(|_| "ROCB GPU walk_forward ATR row index exceeds i32 range".to_string())?;

    let signals = RocbGpuDeviceSignalMatrices {
        close_f32: &close_f32,
        cols: n,
        upper_row_major_ptr: upper_sweep.matrix.device_ptr,
        lower_row_major_ptr: lower_sweep.matrix.device_ptr,
        channel_rows: upper_sweep.matrix.rows,
        channel_stride: upper_sweep.matrix.cols,
        atr_row_major_ptr: atr_sweep.matrix.device_ptr,
        atr_rows: atr_sweep.matrix.rows,
        atr_stride: atr_sweep.matrix.cols,
    };

    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let mut sharpes: Vec<f64> = Vec::new();
    let mut dds: Vec<f64> = Vec::new();
    let mut pnls: Vec<f64> = Vec::new();
    let mut trades: Vec<u32> = Vec::new();

    let mut t0 = start_t;
    while t0 + 2 <= n {
        let end = t0.saturating_add(window).min(n);
        if end <= t0 + 1 {
            break;
        }
        let start_i = i32::try_from(t0)
            .map_err(|_| "ROCB GPU walk_forward start_t exceeds i32 range".to_string())?;
        let end_i = i32::try_from(end)
            .map_err(|_| "ROCB GPU walk_forward end_t exceeds i32 range".to_string())?;
        let combo_channel = [channel_row_i];
        let combo_atr = [atr_row_i];
        let combo_start = [start_i];
        let combo_end = [end_i];
        let combos = RocbGpuComboBatch {
            channel_row_index: &combo_channel,
            atr_row_index: &combo_atr,
            start_t: &combo_start,
            end_t: &combo_end,
        };
        let metric = eval_rocb_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &params.strategy,
            rocb,
        )?
        .into_iter()
        .next()
        .ok_or_else(|| "ROCB GPU walk_forward returned no metrics".to_string())?;

        sharpes.push(metric.sharpe);
        dds.push(metric.max_dd);
        pnls.push(metric.pnl);
        trades.push(metric.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    sharpes.sort_by(|a, b| a.total_cmp(b));
    dds.sort_by(|a, b| a.total_cmp(b));
    pnls.sort_by(|a, b| a.total_cmp(b));

    Ok(WalkForwardStats {
        windows: sharpes.len(),
        sharpe_median: crate::stats_common::median(&sharpes),
        sharpe_worst: crate::stats_common::worst_min(&sharpes),
        max_dd_median: crate::stats_common::median(&dds),
        max_dd_worst: crate::stats_common::worst_max(&dds),
        pnl_median: crate::stats_common::median(&pnls),
        pnl_worst: crate::stats_common::worst_min(&pnls),
        trades_median: crate::stats_common::median_u32(&trades),
        trades_worst: crate::stats_common::worst_min_u32(&trades),
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_rocb_gpu_reopt_walk_forward_stats_for_candidates(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    finalist_params: &[CompositeParams],
    finalist_start_ts: &[usize],
    cfg: &ReoptWalkForwardConfigOptional,
) -> Result<Option<ReoptWalkForwardStats>, String> {
    if finalist_params.is_empty() || candles.close.len() < 3 {
        return Ok(None);
    }

    let top_n = cfg.top_n_candidates.max(1).min(finalist_params.len());
    let finalists = &finalist_params[..top_n];
    let finalist_start_ts = &finalist_start_ts[..top_n];
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    let first = &finalists[0];
    let first_rocb = first
        .rocb
        .as_ref()
        .ok_or_else(|| "ROCB GPU reopt_walk_forward requires ROCB candidate params".to_string())?;
    validate_strategy_cfg(&first.strategy)?;
    validate_rocb_params(first_rocb)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite ROCB reopt_walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let high = vector_ta::utilities::data_loader::source_type(candles, "high");
    let low = vector_ta::utilities::data_loader::source_type(candles, "low");
    let n = close.len();
    if n < 2 || high.len() != n || low.len() != n {
        return Err("not enough candles or mismatched OHLC series lengths".to_string());
    }
    if n > i32::MAX as usize {
        return Err("ROCB GPU reopt_walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_high =
        first_finite_idx(high).ok_or_else(|| "high series all non-finite".to_string())?;
    let first_valid_low =
        first_finite_idx(low).ok_or_else(|| "low series all non-finite".to_string())?;
    let close_f32: Vec<f32> = close.iter().map(|v| *v as f32).collect();

    let mut min_channel = u16::MAX;
    let mut max_channel = 0u16;
    let mut min_atr = u16::MAX;
    let mut max_atr = 0u16;
    let mut periods_by_candidate: Vec<(u16, u16)> = Vec::with_capacity(finalists.len());
    let mut finalist_entries: Vec<(String, Vec<u8>, usize)> = Vec::with_capacity(finalists.len());

    for (idx, cand) in finalists.iter().enumerate() {
        let rocb = cand.rocb.as_ref().ok_or_else(|| {
            "GPU composite reopt_walk_forward currently requires all finalists to use ROCB"
                .to_string()
        })?;
        if cand.asr.is_some() || cand.vcc.is_some() || cand.vemr.is_some() {
            return Err(
                "GPU composite reopt_walk_forward currently requires all finalists to use ROCB"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        if !serialized_eq(rocb, first_rocb)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires uniform ROCB parameters"
                    .to_string(),
            );
        }

        let periods = resolve_rocb_periods(first_rocb, cand.fast_len, cand.slow_len);
        min_channel = min_channel.min(periods.channel);
        max_channel = max_channel.max(periods.channel);
        min_atr = min_atr.min(periods.atr);
        max_atr = max_atr.max(periods.atr);
        periods_by_candidate.push((periods.channel, periods.atr));

        let key = serde_json::to_string(cand)
            .unwrap_or_else(|_| format!("rocb:{}:{}", cand.fast_len, cand.slow_len));
        finalist_entries.push((
            key,
            serialize_composite_candidate_key(cand)?,
            finalist_start_ts[idx],
        ));
    }

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let upper_sweep = compute_rocb_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["upper_band", "upper", "up"],
        min_channel,
        max_channel,
        1,
    )?;
    let lower_sweep = compute_rocb_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["lower_band", "lower", "down"],
        min_channel,
        max_channel,
        1,
    )?;
    let atr_sweep = compute_rocb_atr_sweep_cuda_device(&prepared_cuda_inputs, min_atr, max_atr, 1)?;

    let mut channel_row_index: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut atr_row_index: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(finalists.len());

    for ((channel, atr), cand) in periods_by_candidate.into_iter().zip(finalists.iter()) {
        let channel_row = upper_sweep
            .row_index_for_period(channel)
            .ok_or_else(|| format!("ROCB GPU reopt missing channel row for period={channel}"))?;
        let atr_row = atr_sweep
            .row_index_for_period(atr)
            .ok_or_else(|| format!("ROCB GPU reopt missing ATR row for period={atr}"))?;
        let start_t = compute_rocb_start_t(
            first_valid_close,
            first_valid_high,
            first_valid_low,
            upper_sweep
                .first_valid_for_period(channel)
                .max(lower_sweep.first_valid_for_period(channel)),
            atr_sweep.first_valid_for_period(atr),
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after ROCB GPU reopt warmup".to_string());
        }
        channel_row_index.push(
            i32::try_from(channel_row)
                .map_err(|_| "ROCB GPU reopt channel row index exceeds i32 range".to_string())?,
        );
        atr_row_index.push(
            i32::try_from(atr_row)
                .map_err(|_| "ROCB GPU reopt ATR row index exceeds i32 range".to_string())?,
        );
        combo_start_t.push(
            i32::try_from(start_t)
                .map_err(|_| "ROCB GPU reopt start_t exceeds i32 range".to_string())?,
        );
    }

    let signals = RocbGpuDeviceSignalMatrices {
        close_f32: &close_f32,
        cols: n,
        upper_row_major_ptr: upper_sweep.matrix.device_ptr,
        lower_row_major_ptr: lower_sweep.matrix.device_ptr,
        channel_rows: upper_sweep.matrix.rows,
        channel_stride: upper_sweep.matrix.cols,
        atr_row_major_ptr: atr_sweep.matrix.device_ptr,
        atr_rows: atr_sweep.matrix.rows,
        atr_stride: atr_sweep.matrix.cols,
    };

    let common_start_t = finalist_start_ts
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .min(n.saturating_sub(2));
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let mut ranges: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut t0 = common_start_t;
    while t0 + 2 < n {
        let train_end = t0.saturating_add(window).min(n);
        if train_end <= t0 + 1 {
            break;
        }
        let test_end = train_end.saturating_add(step).min(n);
        if test_end <= train_end + 1 {
            break;
        }
        ranges.insert((t0, train_end));
        ranges.insert((train_end, test_end));
        if test_end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }
    if ranges.is_empty() {
        return Ok(None);
    }

    let candidate_keys: Vec<Vec<u8>> = finalist_entries
        .iter()
        .map(|(_, key, _)| key.clone())
        .collect();
    let mut range_metrics: HashMap<(usize, usize), HashMap<Vec<u8>, Metrics>> =
        HashMap::with_capacity(ranges.len());
    for (start, end) in ranges {
        let start_vec: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start_i = usize::try_from(st).unwrap_or(0).max(start);
                i32::try_from(start_i)
                    .map_err(|_| "ROCB GPU reopt start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let end_i =
            i32::try_from(end).map_err(|_| "ROCB GPU reopt end_t exceeds i32 range".to_string())?;
        let end_vec = vec![end_i; start_vec.len()];
        let combos = RocbGpuComboBatch {
            channel_row_index: &channel_row_index,
            atr_row_index: &atr_row_index,
            start_t: &start_vec,
            end_t: &end_vec,
        };
        let metrics = eval_rocb_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &first.strategy,
            first_rocb,
        )?;
        let mut keyed = HashMap::with_capacity(metrics.len());
        for (key, metric) in candidate_keys.iter().zip(metrics.into_iter()) {
            keyed.insert(key.clone(), metric);
        }
        range_metrics.insert((start, end), keyed);
    }

    crate::validation_pipeline::compute_reopt_walk_forward_stats(
        &finalist_entries,
        n,
        cfg,
        |(_, _, start_t)| *start_t,
        |(_, key, _), start, end| {
            range_metrics
                .get(&(start, end))
                .and_then(|m| m.get(key))
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "missing ROCB GPU reopt metrics for range {}..{}",
                        start, end
                    )
                })
        },
        |(key, _, _)| key.clone(),
    )
    .ok_or_else(|| "ROCB GPU reopt_walk_forward did not produce any windows".to_string())
    .map(Some)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_rocb_gpu_cscv_slice_stats_for_candidates(
    req: &CompositeValidationRequest,
    signals: &RocbGpuDeviceSignalMatrices<'_>,
    channel_row_index: &[i32],
    atr_row_index: &[i32],
    combo_start_t: &[i32],
    start_ts: &[usize],
    rocb: &RocbStrategyParams,
    device_id: u32,
    cfg: &CscvPboConfig,
) -> Result<Option<HashMap<Vec<u8>, Vec<SliceReturnStats>>>, String> {
    let slices = sanitize_gpu_cscv_slices(cfg.slices);
    let common_start_t = start_ts.iter().copied().max().unwrap_or(0);
    let slice_bars = signals.cols.saturating_sub(common_start_t) / slices;
    if slice_bars < 2 || req.candidates.len() < 2 {
        return Ok(None);
    }

    let candidate_keys: Vec<Vec<u8>> = req
        .candidates
        .iter()
        .map(serialize_composite_candidate_key)
        .collect::<Result<_, _>>()?;
    let mut out: HashMap<Vec<u8>, Vec<SliceReturnStats>> =
        HashMap::with_capacity(candidate_keys.len());
    for key in &candidate_keys {
        out.insert(key.clone(), vec![SliceReturnStats::default(); slices]);
    }

    for slice_idx in 0..slices {
        let t0 = common_start_t + slice_idx * slice_bars;
        let end = t0.saturating_add(slice_bars).min(signals.cols);
        let end_i =
            i32::try_from(end).map_err(|_| "ROCB GPU CSCV end_t exceeds i32 range".to_string())?;
        let slice_start: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start = usize::try_from(st).unwrap_or(0).max(t0);
                i32::try_from(start)
                    .map_err(|_| "ROCB GPU CSCV start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let slice_end = vec![end_i; slice_start.len()];
        let combos = RocbGpuComboBatch {
            channel_row_index,
            atr_row_index,
            start_t: &slice_start,
            end_t: &slice_end,
        };
        let metrics = eval_rocb_metrics_gpu_kernel_device_rows(
            device_id,
            signals,
            &combos,
            &req.candidates[0].strategy,
            rocb,
        )?;
        for (key, metric) in candidate_keys.iter().zip(metrics.iter()) {
            if let Some(slice_vec) = out.get_mut(key) {
                slice_vec[slice_idx] = slice_return_stats_from_metrics(metric, slice_bars);
            }
        }
    }

    Ok(Some(out))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn try_prepare_vemr_gpu_fixed_candidate_validation(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<GpuCompositeFixedCandidateValidationEval, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    if let Some(reason) = composite_gpu_validation_feature_blocker(req) {
        return Err(reason);
    }
    if req.candidates.is_empty() {
        return Err("no candidates to validate".to_string());
    }

    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    let first_vemr = first.vemr.as_ref().ok_or_else(|| {
        "GPU composite validation currently supports ROCB and VEMR candidates only".to_string()
    })?;
    if first.asr.is_some() || first.vcc.is_some() || first.rocb.is_some() {
        return Err(
            "GPU composite validation currently supports ROCB and VEMR candidates only".to_string(),
        );
    }
    validate_strategy_cfg(&first.strategy)?;
    validate_vemr_params(first_vemr)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite VEMR validation currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let mut min_z = u16::MAX;
    let mut max_z = 0u16;
    let mut min_bb = u16::MAX;
    let mut max_bb = 0u16;
    let mut min_atr = u16::MAX;
    let mut max_atr = 0u16;
    let mut periods_by_candidate: Vec<(u16, u16, u16)> = Vec::with_capacity(req.candidates.len());

    for cand in &req.candidates {
        if cand.vemr.is_none() || cand.asr.is_some() || cand.vcc.is_some() || cand.rocb.is_some() {
            return Err(
                "GPU composite validation currently requires all candidates to use VEMR"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite validation currently requires a uniform StrategyConfig".to_string(),
            );
        }
        if !serialized_eq(cand.vemr.as_ref().unwrap(), first_vemr)? {
            return Err(
                "GPU composite validation currently requires uniform VEMR parameters".to_string(),
            );
        }

        let periods = resolve_vemr_periods(first_vemr, cand.fast_len, cand.slow_len);
        min_z = min_z.min(periods.z);
        max_z = max_z.max(periods.z);
        min_bb = min_bb.min(periods.bb);
        max_bb = max_bb.max(periods.bb);
        min_atr = min_atr.min(periods.atr);
        max_atr = max_atr.max(periods.atr);
        periods_by_candidate.push((periods.z, periods.bb, periods.atr));
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    let n = close.len();
    if n < 2 || ma_prices.len() != n {
        return Err("not enough candles or mismatched MA source length".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let z_sweep = compute_vemr_zscore_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_vemr,
        min_z,
        max_z,
        1,
        device_id,
    )?;
    let bb_upper_sweep = compute_vemr_bollinger_upper_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_vemr,
        min_bb,
        max_bb,
        1,
        device_id,
    )?;
    let bb_middle_sweep = compute_vemr_bollinger_middle_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_vemr,
        min_bb,
        max_bb,
        1,
        device_id,
    )?;
    let bb_lower_sweep = compute_vemr_bollinger_lower_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_vemr,
        min_bb,
        max_bb,
        1,
        device_id,
    )?;
    let atr_sweep =
        compute_vemr_atr_sweep_cuda_device(&prepared_cuda_inputs, min_atr, max_atr, 1, device_id)?;

    let mut z_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut bb_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut atr_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut start_ts: Vec<usize> = Vec::with_capacity(req.candidates.len());

    for (z_period, bb_period, atr_period) in periods_by_candidate {
        let z_row = z_sweep.row_index_for_period(z_period).ok_or_else(|| {
            format!("VEMR GPU validation missing zscore row for period={z_period}")
        })?;
        let bb_row = bb_upper_sweep
            .row_index_for_period(bb_period)
            .ok_or_else(|| {
                format!("VEMR GPU validation missing bollinger row for period={bb_period}")
            })?;
        if bb_middle_sweep.row_index_for_period(bb_period).is_none()
            || bb_lower_sweep.row_index_for_period(bb_period).is_none()
        {
            return Err(format!(
                "VEMR GPU validation missing one or more bollinger rows for period={bb_period}"
            ));
        }
        let atr_row = atr_sweep.row_index_for_period(atr_period).ok_or_else(|| {
            format!("VEMR GPU validation missing ATR row for period={atr_period}")
        })?;
        let fast_first_valid = z_sweep
            .first_valid_for_period(z_period)
            .max(bb_upper_sweep.first_valid_for_period(bb_period))
            .max(bb_middle_sweep.first_valid_for_period(bb_period))
            .max(bb_lower_sweep.first_valid_for_period(bb_period));
        let risk_first_valid = atr_sweep.first_valid_for_period(atr_period);
        let max_period = usize::from(z_period.max(bb_period).max(atr_period)).max(1);
        let start_t = compute_vemr_start_t(
            first_valid_close,
            first_valid_source,
            fast_first_valid,
            risk_first_valid,
            max_period,
            first.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after VEMR GPU warmup".to_string());
        }

        z_row_index.push(
            i32::try_from(z_row)
                .map_err(|_| "VEMR GPU zscore row index exceeds i32 range".to_string())?,
        );
        bb_row_index.push(
            i32::try_from(bb_row)
                .map_err(|_| "VEMR GPU bollinger row index exceeds i32 range".to_string())?,
        );
        atr_row_index.push(
            i32::try_from(atr_row)
                .map_err(|_| "VEMR GPU ATR row index exceeds i32 range".to_string())?,
        );
        combo_start_t.push(
            i32::try_from(start_t).map_err(|_| "VEMR GPU start_t exceeds i32 range".to_string())?,
        );
        start_ts.push(start_t);
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
    let combo_end_t = vec![
        i32::try_from(n)
            .map_err(|_| "VEMR GPU end_t exceeds i32 range".to_string())?;
        z_row_index.len()
    ];
    let combos = VemrGpuComboBatch {
        z_row_index: &z_row_index,
        bb_row_index: &bb_row_index,
        atr_row_index: &atr_row_index,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };

    let metrics = match eval_vemr_metrics_gpu_kernel_device_rows(
        device_id,
        &signals,
        &combos,
        &first.strategy,
        first_vemr,
        ObjectiveKind::Sharpe,
        req.candidates.len().max(1),
        true,
        0,
    )? {
        VemrGpuEvalResult::Full(metrics) => metrics,
        VemrGpuEvalResult::TopK(_) => {
            return Err(
                "VEMR GPU validation unexpectedly returned TopK output in include_all mode"
                    .to_string(),
            )
        }
    };
    if metrics.len() != req.candidates.len() {
        return Err("VEMR GPU validation output length mismatch".to_string());
    }
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        compute_vemr_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &z_row_index,
            &bb_row_index,
            &atr_row_index,
            &combo_start_t,
            &start_ts,
            first_vemr,
            device_id,
            cscv_cfg,
        )?
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        let slice_stats = compute_vemr_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &z_row_index,
            &bb_row_index,
            &atr_row_index,
            &combo_start_t,
            &start_ts,
            first_vemr,
            device_id,
            &CscvPboConfig {
                slices: spa_cfg.slices.max(4),
                max_combinations: usize::MAX,
                threshold: 0.0,
            },
        )?;
        slice_stats.as_ref().map(spa_slice_sums_from_slice_stats)
    } else {
        None
    };

    let backend_route = match req.backend {
        Backend::Auto { .. } => Some("Auto->GpuKernel".to_string()),
        _ => Some(validation_backend_label(&req.backend).to_string()),
    };

    Ok(GpuCompositeFixedCandidateValidationEval {
        metrics,
        start_ts,
        candidate_indices: composite_candidate_index_map(&req.candidates)?,
        walk_forward_stats: None,
        cscv_slices,
        spa_slice_sums,
        base_composite_ctx: None,
        backend_used: BackendUsed::Gpu,
        backend_route,
        backend_fallback_reason: None,
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_vemr_gpu_walk_forward_stats_for_candidate(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    params: &CompositeParams,
    cfg: &WalkForwardConfigOptional,
) -> Result<WalkForwardStats, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    let vemr = params
        .vemr
        .as_ref()
        .ok_or_else(|| "VEMR GPU walk_forward requires VEMR candidate params".to_string())?;
    validate_strategy_cfg(&params.strategy)?;
    validate_vemr_params(vemr)?;
    if params.strategy.uses_execution_overlays()
        || params.strategy.stop_loss_pct().is_some()
        || params.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite VEMR walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    let n = close.len();
    if n < 2 || ma_prices.len() != n {
        return Err("not enough candles or mismatched MA source length".to_string());
    }
    if n > i32::MAX as usize {
        return Err("VEMR GPU walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let periods = resolve_vemr_periods(vemr, params.fast_len, params.slow_len);
    let z_sweep = compute_vemr_zscore_sweep_cuda_device(
        &prepared_cuda_inputs,
        vemr,
        periods.z,
        periods.z,
        1,
        device_id,
    )?;
    let bb_upper_sweep = compute_vemr_bollinger_upper_sweep_cuda_device(
        &prepared_cuda_inputs,
        vemr,
        periods.bb,
        periods.bb,
        1,
        device_id,
    )?;
    let bb_middle_sweep = compute_vemr_bollinger_middle_sweep_cuda_device(
        &prepared_cuda_inputs,
        vemr,
        periods.bb,
        periods.bb,
        1,
        device_id,
    )?;
    let bb_lower_sweep = compute_vemr_bollinger_lower_sweep_cuda_device(
        &prepared_cuda_inputs,
        vemr,
        periods.bb,
        periods.bb,
        1,
        device_id,
    )?;
    let atr_sweep = compute_vemr_atr_sweep_cuda_device(
        &prepared_cuda_inputs,
        periods.atr,
        periods.atr,
        1,
        device_id,
    )?;

    let z_row = z_sweep.row_index_for_period(periods.z).ok_or_else(|| {
        format!(
            "VEMR GPU walk_forward missing zscore row for period={}",
            periods.z
        )
    })?;
    let bb_row = bb_upper_sweep
        .row_index_for_period(periods.bb)
        .ok_or_else(|| {
            format!(
                "VEMR GPU walk_forward missing bollinger row for period={}",
                periods.bb
            )
        })?;
    if bb_middle_sweep.row_index_for_period(periods.bb).is_none()
        || bb_lower_sweep.row_index_for_period(periods.bb).is_none()
    {
        return Err(format!(
            "VEMR GPU walk_forward missing one or more bollinger rows for period={}",
            periods.bb
        ));
    }
    let atr_row = atr_sweep.row_index_for_period(periods.atr).ok_or_else(|| {
        format!(
            "VEMR GPU walk_forward missing ATR row for period={}",
            periods.atr
        )
    })?;
    let fast_first_valid = z_sweep
        .first_valid_for_period(periods.z)
        .max(bb_upper_sweep.first_valid_for_period(periods.bb))
        .max(bb_middle_sweep.first_valid_for_period(periods.bb))
        .max(bb_lower_sweep.first_valid_for_period(periods.bb));
    let risk_first_valid = atr_sweep.first_valid_for_period(periods.atr);
    let max_period = usize::from(periods.z.max(periods.bb).max(periods.atr)).max(1);
    let start_t = compute_vemr_start_t(
        first_valid_close,
        first_valid_source,
        fast_first_valid,
        risk_first_valid,
        max_period,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after VEMR GPU walk_forward warmup".to_string());
    }

    let z_row_i = i32::try_from(z_row)
        .map_err(|_| "VEMR GPU walk_forward zscore row index exceeds i32 range".to_string())?;
    let bb_row_i = i32::try_from(bb_row)
        .map_err(|_| "VEMR GPU walk_forward bollinger row index exceeds i32 range".to_string())?;
    let atr_row_i = i32::try_from(atr_row)
        .map_err(|_| "VEMR GPU walk_forward ATR row index exceeds i32 range".to_string())?;

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

    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let mut sharpes: Vec<f64> = Vec::new();
    let mut dds: Vec<f64> = Vec::new();
    let mut pnls: Vec<f64> = Vec::new();
    let mut trades: Vec<u32> = Vec::new();

    let mut t0 = start_t;
    while t0 + 2 <= n {
        let end = t0.saturating_add(window).min(n);
        if end <= t0 + 1 {
            break;
        }
        let start_i = i32::try_from(t0)
            .map_err(|_| "VEMR GPU walk_forward start_t exceeds i32 range".to_string())?;
        let end_i = i32::try_from(end)
            .map_err(|_| "VEMR GPU walk_forward end_t exceeds i32 range".to_string())?;
        let combo_z = [z_row_i];
        let combo_bb = [bb_row_i];
        let combo_atr = [atr_row_i];
        let combo_start = [start_i];
        let combo_end = [end_i];
        let combos = VemrGpuComboBatch {
            z_row_index: &combo_z,
            bb_row_index: &combo_bb,
            atr_row_index: &combo_atr,
            start_t: &combo_start,
            end_t: &combo_end,
        };
        let metric =
            match eval_vemr_metrics_gpu_kernel_device_rows(
                device_id,
                &signals,
                &combos,
                &params.strategy,
                vemr,
                ObjectiveKind::Sharpe,
                1,
                true,
                0,
            )? {
                VemrGpuEvalResult::Full(metrics) => metrics
                    .into_iter()
                    .next()
                    .ok_or_else(|| "VEMR GPU walk_forward returned no metrics".to_string())?,
                VemrGpuEvalResult::TopK(_) => return Err(
                    "VEMR GPU walk_forward unexpectedly returned TopK output in include_all mode"
                        .to_string(),
                ),
            };

        sharpes.push(metric.sharpe);
        dds.push(metric.max_dd);
        pnls.push(metric.pnl);
        trades.push(metric.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    sharpes.sort_by(|a, b| a.total_cmp(b));
    dds.sort_by(|a, b| a.total_cmp(b));
    pnls.sort_by(|a, b| a.total_cmp(b));

    Ok(WalkForwardStats {
        windows: sharpes.len(),
        sharpe_median: crate::stats_common::median(&sharpes),
        sharpe_worst: crate::stats_common::worst_min(&sharpes),
        max_dd_median: crate::stats_common::median(&dds),
        max_dd_worst: crate::stats_common::worst_max(&dds),
        pnl_median: crate::stats_common::median(&pnls),
        pnl_worst: crate::stats_common::worst_min(&pnls),
        trades_median: crate::stats_common::median_u32(&trades),
        trades_worst: crate::stats_common::worst_min_u32(&trades),
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_vemr_gpu_reopt_walk_forward_stats_for_candidates(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    finalist_params: &[CompositeParams],
    finalist_start_ts: &[usize],
    cfg: &ReoptWalkForwardConfigOptional,
) -> Result<Option<ReoptWalkForwardStats>, String> {
    if finalist_params.is_empty() || candles.close.len() < 3 {
        return Ok(None);
    }

    let top_n = cfg.top_n_candidates.max(1).min(finalist_params.len());
    let finalists = &finalist_params[..top_n];
    let finalist_start_ts = &finalist_start_ts[..top_n];
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    let first = &finalists[0];
    let first_vemr = first
        .vemr
        .as_ref()
        .ok_or_else(|| "VEMR GPU reopt_walk_forward requires VEMR candidate params".to_string())?;
    validate_strategy_cfg(&first.strategy)?;
    validate_vemr_params(first_vemr)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite VEMR reopt_walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    let n = close.len();
    if n < 2 || ma_prices.len() != n {
        return Err("not enough candles or mismatched MA source length".to_string());
    }
    if n > i32::MAX as usize {
        return Err("VEMR GPU reopt_walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let mut min_z = u16::MAX;
    let mut max_z = 0u16;
    let mut min_bb = u16::MAX;
    let mut max_bb = 0u16;
    let mut min_atr = u16::MAX;
    let mut max_atr = 0u16;
    let mut periods_by_candidate: Vec<(u16, u16, u16)> = Vec::with_capacity(finalists.len());
    let mut finalist_entries: Vec<(String, Vec<u8>, usize)> = Vec::with_capacity(finalists.len());

    for (idx, cand) in finalists.iter().enumerate() {
        let vemr = cand.vemr.as_ref().ok_or_else(|| {
            "GPU composite reopt_walk_forward currently requires all finalists to use VEMR"
                .to_string()
        })?;
        if cand.asr.is_some() || cand.vcc.is_some() || cand.rocb.is_some() {
            return Err(
                "GPU composite reopt_walk_forward currently requires all finalists to use VEMR"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        if !serialized_eq(vemr, first_vemr)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires uniform VEMR parameters"
                    .to_string(),
            );
        }

        let periods = resolve_vemr_periods(first_vemr, cand.fast_len, cand.slow_len);
        min_z = min_z.min(periods.z);
        max_z = max_z.max(periods.z);
        min_bb = min_bb.min(periods.bb);
        max_bb = max_bb.max(periods.bb);
        min_atr = min_atr.min(periods.atr);
        max_atr = max_atr.max(periods.atr);
        periods_by_candidate.push((periods.z, periods.bb, periods.atr));

        let key = serde_json::to_string(cand)
            .unwrap_or_else(|_| format!("vemr:{}:{}", cand.fast_len, cand.slow_len));
        finalist_entries.push((
            key,
            serialize_composite_candidate_key(cand)?,
            finalist_start_ts[idx],
        ));
    }

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let z_sweep = compute_vemr_zscore_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_vemr,
        min_z,
        max_z,
        1,
        device_id,
    )?;
    let bb_upper_sweep = compute_vemr_bollinger_upper_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_vemr,
        min_bb,
        max_bb,
        1,
        device_id,
    )?;
    let bb_middle_sweep = compute_vemr_bollinger_middle_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_vemr,
        min_bb,
        max_bb,
        1,
        device_id,
    )?;
    let bb_lower_sweep = compute_vemr_bollinger_lower_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_vemr,
        min_bb,
        max_bb,
        1,
        device_id,
    )?;
    let atr_sweep =
        compute_vemr_atr_sweep_cuda_device(&prepared_cuda_inputs, min_atr, max_atr, 1, device_id)?;

    let mut z_row_index: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut bb_row_index: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut atr_row_index: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(finalists.len());

    for ((z, bb, atr), cand) in periods_by_candidate.into_iter().zip(finalists.iter()) {
        let z_row = z_sweep
            .row_index_for_period(z)
            .ok_or_else(|| format!("VEMR GPU reopt missing zscore row for period={z}"))?;
        let bb_row = bb_upper_sweep
            .row_index_for_period(bb)
            .ok_or_else(|| format!("VEMR GPU reopt missing bollinger row for period={bb}"))?;
        if bb_middle_sweep.row_index_for_period(bb).is_none()
            || bb_lower_sweep.row_index_for_period(bb).is_none()
        {
            return Err(format!(
                "VEMR GPU reopt missing one or more bollinger rows for period={bb}"
            ));
        }
        let atr_row = atr_sweep
            .row_index_for_period(atr)
            .ok_or_else(|| format!("VEMR GPU reopt missing ATR row for period={atr}"))?;
        let fast_first_valid = z_sweep
            .first_valid_for_period(z)
            .max(bb_upper_sweep.first_valid_for_period(bb))
            .max(bb_middle_sweep.first_valid_for_period(bb))
            .max(bb_lower_sweep.first_valid_for_period(bb));
        let risk_first_valid = atr_sweep.first_valid_for_period(atr);
        let max_period = usize::from(z.max(bb).max(atr)).max(1);
        let start_t = compute_vemr_start_t(
            first_valid_close,
            first_valid_source,
            fast_first_valid,
            risk_first_valid,
            max_period,
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after VEMR GPU reopt warmup".to_string());
        }
        z_row_index.push(
            i32::try_from(z_row)
                .map_err(|_| "VEMR GPU reopt zscore row index exceeds i32 range".to_string())?,
        );
        bb_row_index.push(
            i32::try_from(bb_row)
                .map_err(|_| "VEMR GPU reopt bollinger row index exceeds i32 range".to_string())?,
        );
        atr_row_index.push(
            i32::try_from(atr_row)
                .map_err(|_| "VEMR GPU reopt ATR row index exceeds i32 range".to_string())?,
        );
        combo_start_t.push(
            i32::try_from(start_t)
                .map_err(|_| "VEMR GPU reopt start_t exceeds i32 range".to_string())?,
        );
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

    let common_start_t = finalist_start_ts
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .min(n.saturating_sub(2));
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let mut ranges: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut t0 = common_start_t;
    while t0 + 2 < n {
        let train_end = t0.saturating_add(window).min(n);
        if train_end <= t0 + 1 {
            break;
        }
        let test_end = train_end.saturating_add(step).min(n);
        if test_end <= train_end + 1 {
            break;
        }
        ranges.insert((t0, train_end));
        ranges.insert((train_end, test_end));
        if test_end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }
    if ranges.is_empty() {
        return Ok(None);
    }

    let candidate_keys: Vec<Vec<u8>> = finalist_entries
        .iter()
        .map(|(_, key, _)| key.clone())
        .collect();
    let mut range_metrics: HashMap<(usize, usize), HashMap<Vec<u8>, Metrics>> =
        HashMap::with_capacity(ranges.len());
    for (start, end) in ranges {
        let start_vec: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start_i = usize::try_from(st).unwrap_or(0).max(start);
                i32::try_from(start_i)
                    .map_err(|_| "VEMR GPU reopt start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let end_i =
            i32::try_from(end).map_err(|_| "VEMR GPU reopt end_t exceeds i32 range".to_string())?;
        let end_vec = vec![end_i; start_vec.len()];
        let combos = VemrGpuComboBatch {
            z_row_index: &z_row_index,
            bb_row_index: &bb_row_index,
            atr_row_index: &atr_row_index,
            start_t: &start_vec,
            end_t: &end_vec,
        };
        let metrics = match eval_vemr_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &first.strategy,
            first_vemr,
            ObjectiveKind::Sharpe,
            finalists.len().max(1),
            true,
            0,
        )? {
            VemrGpuEvalResult::Full(metrics) => metrics,
            VemrGpuEvalResult::TopK(_) => return Err(
                "VEMR GPU reopt_walk_forward unexpectedly returned TopK output in include_all mode"
                    .to_string(),
            ),
        };
        let mut keyed = HashMap::with_capacity(metrics.len());
        for (key, metric) in candidate_keys.iter().zip(metrics.into_iter()) {
            keyed.insert(key.clone(), metric);
        }
        range_metrics.insert((start, end), keyed);
    }

    crate::validation_pipeline::compute_reopt_walk_forward_stats(
        &finalist_entries,
        n,
        cfg,
        |(_, _, start_t)| *start_t,
        |(_, key, _), start, end| {
            range_metrics
                .get(&(start, end))
                .and_then(|m| m.get(key))
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "missing VEMR GPU reopt metrics for range {}..{}",
                        start, end
                    )
                })
        },
        |(key, _, _)| key.clone(),
    )
    .ok_or_else(|| "VEMR GPU reopt_walk_forward did not produce any windows".to_string())
    .map(Some)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_vemr_gpu_cscv_slice_stats_for_candidates(
    req: &CompositeValidationRequest,
    signals: &VemrGpuDeviceSignalMatrices<'_>,
    z_row_index: &[i32],
    bb_row_index: &[i32],
    atr_row_index: &[i32],
    combo_start_t: &[i32],
    start_ts: &[usize],
    vemr: &VemrStrategyParams,
    device_id: u32,
    cfg: &CscvPboConfig,
) -> Result<Option<HashMap<Vec<u8>, Vec<SliceReturnStats>>>, String> {
    let slices = sanitize_gpu_cscv_slices(cfg.slices);
    let common_start_t = start_ts.iter().copied().max().unwrap_or(0);
    let slice_bars = signals.cols.saturating_sub(common_start_t) / slices;
    if slice_bars < 2 || req.candidates.len() < 2 {
        return Ok(None);
    }

    let candidate_keys: Vec<Vec<u8>> = req
        .candidates
        .iter()
        .map(serialize_composite_candidate_key)
        .collect::<Result<_, _>>()?;
    let mut out: HashMap<Vec<u8>, Vec<SliceReturnStats>> =
        HashMap::with_capacity(candidate_keys.len());
    for key in &candidate_keys {
        out.insert(key.clone(), vec![SliceReturnStats::default(); slices]);
    }

    for slice_idx in 0..slices {
        let t0 = common_start_t + slice_idx * slice_bars;
        let end = t0.saturating_add(slice_bars).min(signals.cols);
        let end_i =
            i32::try_from(end).map_err(|_| "VEMR GPU CSCV end_t exceeds i32 range".to_string())?;
        let slice_start: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start = usize::try_from(st).unwrap_or(0).max(t0);
                i32::try_from(start)
                    .map_err(|_| "VEMR GPU CSCV start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let slice_end = vec![end_i; slice_start.len()];
        let combos = VemrGpuComboBatch {
            z_row_index,
            bb_row_index,
            atr_row_index,
            start_t: &slice_start,
            end_t: &slice_end,
        };
        let metrics = match eval_vemr_metrics_gpu_kernel_device_rows(
            device_id,
            signals,
            &combos,
            &req.candidates[0].strategy,
            vemr,
            ObjectiveKind::Sharpe,
            req.candidates.len().max(1),
            true,
            0,
        )? {
            VemrGpuEvalResult::Full(metrics) => metrics,
            VemrGpuEvalResult::TopK(_) => {
                return Err(
                    "VEMR GPU CSCV unexpectedly returned TopK output in include_all mode"
                        .to_string(),
                )
            }
        };
        for (key, metric) in candidate_keys.iter().zip(metrics.iter()) {
            if let Some(slice_vec) = out.get_mut(key) {
                slice_vec[slice_idx] = slice_return_stats_from_metrics(metric, slice_bars);
            }
        }
    }

    Ok(Some(out))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(dead_code)]
fn try_prepare_vcc_gpu_fixed_candidate_validation(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<GpuCompositeFixedCandidateValidationEval, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    if let Some(reason) = composite_gpu_validation_feature_blocker(req) {
        return Err(reason);
    }
    if req.candidates.is_empty() {
        return Err("no candidates to validate".to_string());
    }

    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    let first_vcc = first.vcc.as_ref().ok_or_else(|| {
        "GPU composite validation currently supports ROCB, VEMR, and VCC candidates only"
            .to_string()
    })?;
    if first.asr.is_some() || first.vemr.is_some() || first.rocb.is_some() {
        return Err(
            "GPU composite validation currently supports ROCB, VEMR, and VCC candidates only"
                .to_string(),
        );
    }
    validate_strategy_cfg(&first.strategy)?;
    validate_vcc_params(first_vcc)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite VCC validation currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let mut min_sq = u16::MAX;
    let mut max_sq = 0u16;
    let mut min_mo = u16::MAX;
    let mut max_mo = 0u16;
    let mut min_trend = u16::MAX;
    let mut max_trend = 0u16;
    let mut periods_by_candidate: Vec<(u16, u16, u16)> = Vec::with_capacity(req.candidates.len());

    for cand in &req.candidates {
        if cand.vcc.is_none() || cand.asr.is_some() || cand.vemr.is_some() || cand.rocb.is_some() {
            return Err(
                "GPU composite validation currently requires all candidates to use VCC".to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite validation currently requires a uniform StrategyConfig".to_string(),
            );
        }
        if !serialized_eq(cand.vcc.as_ref().unwrap(), first_vcc)? {
            return Err(
                "GPU composite validation currently requires uniform VCC parameters".to_string(),
            );
        }

        let periods = resolve_vcc_periods(first_vcc, cand.fast_len, cand.slow_len);
        min_sq = min_sq.min(periods.squeeze);
        max_sq = max_sq.max(periods.squeeze);
        min_mo = min_mo.min(periods.momentum);
        max_mo = max_mo.max(periods.momentum);
        min_trend = min_trend.min(periods.trend);
        max_trend = max_trend.max(periods.trend);
        periods_by_candidate.push((periods.squeeze, periods.momentum, periods.trend));
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    let n = close.len();
    if n < 2 || ma_prices.len() != n {
        return Err("not enough candles or mismatched MA source length".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let (fast_squeeze_sweep, fast_momentum_sweep) = compute_vcc_fast_dual_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_vcc,
        min_sq,
        max_sq,
        min_mo,
        max_mo,
        1,
        device_id,
    )?;
    let trend_sweep = compute_vcc_ttm_trend_sweep_cuda_device(
        &prepared_cuda_inputs,
        min_trend,
        max_trend,
        1,
        device_id,
    )?;

    let mut fast_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut trend_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut start_ts: Vec<usize> = Vec::with_capacity(req.candidates.len());

    let mut fast_squeeze_rows: Vec<u64> = Vec::new();
    let mut fast_momentum_rows: Vec<u64> = Vec::new();
    let mut fast_squeeze_host_row_major: Vec<f32> = Vec::with_capacity(req.candidates.len() * n);
    let mut fast_momentum_host_row_major: Vec<f32> = Vec::with_capacity(req.candidates.len() * n);
    let mut trend_rows_ptr: Vec<u64> = Vec::new();
    let mut trend_host_row_major: Vec<f32> = Vec::with_capacity(req.candidates.len() * n);
    let mut fast_lookup: HashMap<(u16, u16), i32> = HashMap::new();
    let mut trend_lookup: HashMap<u16, i32> = HashMap::new();
    let mut fast_first_valid_lookup: HashMap<(u16, u16), usize> = HashMap::new();
    let mut trend_first_valid_lookup: HashMap<u16, usize> = HashMap::new();

    for (sq, mo, trend) in periods_by_candidate {
        let fast_key = (sq, mo);
        let fast_idx = if let Some(idx) = fast_lookup.get(&fast_key) {
            *idx
        } else {
            let sq_ptr = fast_squeeze_sweep
                .row_ptr_for_period(sq)
                .ok_or_else(|| format!("VCC GPU validation missing squeeze row for period={sq}"))?;
            let mo_ptr = fast_momentum_sweep.row_ptr_for_period(mo).ok_or_else(|| {
                format!("VCC GPU validation missing momentum row for period={mo}")
            })?;
            let idx = i32::try_from(fast_squeeze_rows.len())
                .map_err(|_| "VCC GPU fast row index exceeds i32 range".to_string())?;
            fast_squeeze_rows.push(sq_ptr);
            fast_momentum_rows.push(mo_ptr);
            fast_lookup.insert(fast_key, idx);
            let fast_signals = compute_vcc_fast_signals(
                candles,
                &req.ma_source,
                &Backend::CpuOnly,
                first_vcc,
                sq,
                mo,
                Some(&prepared_cuda_inputs),
            )?;
            let parity = vcc_momentum_roundtrip_parity(&fast_signals.momentum);
            if !vcc_momentum_roundtrip_is_fp32_stable(&fast_signals.momentum) {
                return Err(format!(
                    "GPU composite VCC validation falls back to CPU because the requested momentum rows are not fp32-stable (sign mismatches={}, delta-sign mismatches={}).",
                    parity.sign_mismatch_count,
                    parity.delta_sign_mismatch_count
                ));
            }
            fast_squeeze_host_row_major
                .extend(fast_signals.squeeze_state.iter().map(|v| *v as f32));
            fast_momentum_host_row_major.extend(fast_signals.momentum.iter().map(|v| *v as f32));
            fast_first_valid_lookup.insert(fast_key, fast_signals.first_valid);
            idx
        };
        let trend_idx = if let Some(idx) = trend_lookup.get(&trend) {
            *idx
        } else {
            let tr_ptr = trend_sweep.row_ptr_for_period(trend).ok_or_else(|| {
                format!("VCC GPU validation missing trend row for period={trend}")
            })?;
            let idx = i32::try_from(trend_rows_ptr.len())
                .map_err(|_| "VCC GPU trend row index exceeds i32 range".to_string())?;
            trend_rows_ptr.push(tr_ptr);
            trend_lookup.insert(trend, idx);
            let trend_signals = compute_vcc_trend_signals(
                candles,
                &req.ma_source,
                &Backend::CpuOnly,
                trend,
                Some(&prepared_cuda_inputs),
            )?;
            trend_host_row_major.extend(trend_signals.trend_state.iter().map(|v| *v as f32));
            trend_first_valid_lookup.insert(trend, trend_signals.first_valid);
            idx
        };
        let fast_first_valid = *fast_first_valid_lookup
            .get(&fast_key)
            .ok_or_else(|| "VCC GPU validation fast first_valid lookup miss".to_string())?;
        let trend_first_valid = *trend_first_valid_lookup
            .get(&trend)
            .ok_or_else(|| "VCC GPU validation trend first_valid lookup miss".to_string())?;
        let max_period = usize::from(sq.max(mo).max(trend)).max(1);
        let start_t = compute_vcc_start_t(
            first_valid_close,
            first_valid_source,
            fast_first_valid,
            trend_first_valid,
            max_period,
            first.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after VCC GPU warmup".to_string());
        }

        fast_row_index.push(fast_idx);
        trend_row_index.push(trend_idx);
        combo_start_t.push(
            i32::try_from(start_t).map_err(|_| "VCC GPU start_t exceeds i32 range".to_string())?,
        );
        start_ts.push(start_t);
    }

    let signals = VccGpuDeviceSignalMatrices {
        close_f64: close,
        cols: n,
        fast_rows: fast_squeeze_rows.len(),
        trend_rows: trend_rows_ptr.len(),
        fast_squeeze_rows: &fast_squeeze_rows,
        fast_squeeze_host_row_major: Some(&fast_squeeze_host_row_major),
        fast_momentum_rows: &fast_momentum_rows,
        fast_momentum_host_row_major: Some(&fast_momentum_host_row_major),
        trend_rows_ptr: &trend_rows_ptr,
        trend_host_row_major: Some(&trend_host_row_major),
    };
    let combo_end_t = vec![
        i32::try_from(n)
            .map_err(|_| "VCC GPU end_t exceeds i32 range".to_string())?;
        fast_row_index.len()
    ];
    let combos = VccGpuComboBatch {
        fast_row_index: &fast_row_index,
        trend_row_index: &trend_row_index,
        start_t: &combo_start_t,
        metric_start_t: &combo_start_t,
        end_t: &combo_end_t,
    };
    let metrics = eval_vcc_metrics_gpu_kernel_device_rows(
        device_id,
        &signals,
        &combos,
        &first.strategy,
        first_vcc,
    )?;
    if metrics.len() != req.candidates.len() {
        return Err("VCC GPU validation output length mismatch".to_string());
    }
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        compute_vcc_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &fast_row_index,
            &trend_row_index,
            &combo_start_t,
            &start_ts,
            first_vcc,
            device_id,
            cscv_cfg,
        )?
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        let slice_stats = compute_vcc_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &fast_row_index,
            &trend_row_index,
            &combo_start_t,
            &start_ts,
            first_vcc,
            device_id,
            &CscvPboConfig {
                slices: spa_cfg.slices.max(4),
                max_combinations: 0,
                threshold: 0.0,
            },
        )?;
        slice_stats.as_ref().map(spa_slice_sums_from_slice_stats)
    } else {
        None
    };

    let backend_route = match req.backend {
        Backend::Auto { .. } => Some("Auto->GpuKernel".to_string()),
        _ => Some(validation_backend_label(&req.backend).to_string()),
    };

    Ok(GpuCompositeFixedCandidateValidationEval {
        metrics,
        start_ts,
        candidate_indices: composite_candidate_index_map(&req.candidates)?,
        walk_forward_stats: None,
        cscv_slices,
        spa_slice_sums,
        base_composite_ctx: None,
        backend_used: BackendUsed::Gpu,
        backend_route,
        backend_fallback_reason: None,
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(dead_code)]
fn compute_vcc_gpu_cscv_slice_stats_for_candidates(
    req: &CompositeValidationRequest,
    signals: &VccGpuDeviceSignalMatrices<'_>,
    fast_row_index: &[i32],
    trend_row_index: &[i32],
    combo_start_t: &[i32],
    start_ts: &[usize],
    vcc: &VccStrategyParams,
    device_id: u32,
    cfg: &CscvPboConfig,
) -> Result<Option<HashMap<Vec<u8>, Vec<SliceReturnStats>>>, String> {
    let slices = sanitize_gpu_cscv_slices(cfg.slices);
    let common_start_t = start_ts.iter().copied().max().unwrap_or(0);
    let slice_bars = signals.cols.saturating_sub(common_start_t) / slices;
    if slice_bars < 2 || req.candidates.len() < 2 {
        return Ok(None);
    }

    let candidate_keys: Vec<Vec<u8>> = req
        .candidates
        .iter()
        .map(serialize_composite_candidate_key)
        .collect::<Result<_, _>>()?;
    let mut out: HashMap<Vec<u8>, Vec<SliceReturnStats>> =
        HashMap::with_capacity(candidate_keys.len());
    for key in &candidate_keys {
        out.insert(key.clone(), vec![SliceReturnStats::default(); slices]);
    }

    for slice_idx in 0..slices {
        let t0 = common_start_t + slice_idx * slice_bars;
        let end = t0.saturating_add(slice_bars).min(signals.cols);
        let end_i =
            i32::try_from(end).map_err(|_| "VCC GPU CSCV end_t exceeds i32 range".to_string())?;
        let slice_metric_start: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start = usize::try_from(st).unwrap_or(0).max(t0);
                i32::try_from(start)
                    .map_err(|_| "VCC GPU CSCV start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let slice_end = vec![end_i; slice_metric_start.len()];
        let combos = VccGpuComboBatch {
            fast_row_index,
            trend_row_index,
            start_t: combo_start_t,
            metric_start_t: &slice_metric_start,
            end_t: &slice_end,
        };
        let metrics = eval_vcc_metrics_gpu_kernel_device_rows_detailed(
            device_id,
            signals,
            &combos,
            &req.candidates[0].strategy,
            vcc,
        )?;
        for (key, metric) in candidate_keys.iter().zip(metrics.iter()) {
            if let Some(slice_vec) = out.get_mut(key) {
                let _ = slice_bars;
                slice_vec[slice_idx] = SliceReturnStats {
                    sum: metric.step_return_sum,
                    sumsq: metric.step_return_sumsq,
                };
            }
        }
    }

    Ok(Some(out))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(dead_code)]
fn compute_vcc_gpu_walk_forward_stats_for_candidate(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    params: &CompositeParams,
    cfg: &WalkForwardConfigOptional,
) -> Result<WalkForwardStats, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    let vcc = params
        .vcc
        .as_ref()
        .ok_or_else(|| "VCC GPU walk_forward requires VCC candidate params".to_string())?;
    validate_strategy_cfg(&params.strategy)?;
    validate_vcc_params(vcc)?;
    if params.strategy.uses_execution_overlays()
        || params.strategy.stop_loss_pct().is_some()
        || params.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite VCC walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    let n = close.len();
    if n < 2 || ma_prices.len() != n {
        return Err("not enough candles or mismatched MA source length".to_string());
    }
    if n > i32::MAX as usize {
        return Err("VCC GPU walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let periods = resolve_vcc_periods(vcc, params.fast_len, params.slow_len);
    let fast_squeeze_sweep = compute_vcc_ttm_squeeze_sweep_cuda_device(
        &prepared_cuda_inputs,
        vcc,
        periods.squeeze,
        periods.squeeze,
        1,
        device_id,
    )?;
    let trend_sweep = compute_vcc_ttm_trend_sweep_cuda_device(
        &prepared_cuda_inputs,
        periods.trend,
        periods.trend,
        1,
        device_id,
    )?;
    let fast_row = fast_squeeze_sweep
        .row_ptr_for_period(periods.squeeze)
        .ok_or_else(|| {
            format!(
                "VCC GPU walk_forward missing squeeze row for period={}",
                periods.squeeze
            )
        })?;
    let trend_row = trend_sweep
        .row_ptr_for_period(periods.trend)
        .ok_or_else(|| {
            format!(
                "VCC GPU walk_forward missing trend row for period={}",
                periods.trend
            )
        })?;
    let fast_signals = compute_vcc_fast_signals(
        candles,
        &req.ma_source,
        &Backend::CpuOnly,
        vcc,
        periods.squeeze,
        periods.momentum,
        Some(&prepared_cuda_inputs),
    )?;
    let parity = vcc_momentum_roundtrip_parity(&fast_signals.momentum);
    if !vcc_momentum_roundtrip_is_fp32_stable(&fast_signals.momentum) {
        return Err(format!(
            "GPU composite VCC validation falls back to CPU because the requested momentum rows are not fp32-stable (sign mismatches={}, delta-sign mismatches={}).",
            parity.sign_mismatch_count,
            parity.delta_sign_mismatch_count
        ));
    }
    let trend_signals = compute_vcc_trend_signals(
        candles,
        &req.ma_source,
        &Backend::CpuOnly,
        periods.trend,
        Some(&prepared_cuda_inputs),
    )?;
    let start_t = compute_vcc_start_t(
        first_valid_close,
        first_valid_source,
        fast_signals.first_valid,
        trend_signals.first_valid,
        usize::from(periods.squeeze.max(periods.momentum).max(periods.trend)).max(1),
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after VCC GPU walk_forward warmup".to_string());
    }

    let fast_squeeze_rows = vec![fast_row];
    let fast_squeeze_host_row_major: Vec<f32> = fast_signals
        .squeeze_state
        .iter()
        .map(|v| *v as f32)
        .collect();
    let trend_rows_ptr = vec![trend_row];
    let trend_host_row_major: Vec<f32> = trend_signals
        .trend_state
        .iter()
        .map(|v| *v as f32)
        .collect();
    let fast_momentum_host_row_major: Vec<f32> =
        fast_signals.momentum.iter().map(|v| *v as f32).collect();
    let signals = VccGpuDeviceSignalMatrices {
        close_f64: close,
        cols: n,
        fast_rows: fast_squeeze_rows.len(),
        trend_rows: trend_rows_ptr.len(),
        fast_squeeze_rows: &fast_squeeze_rows,
        fast_squeeze_host_row_major: Some(&fast_squeeze_host_row_major),
        fast_momentum_rows: &[],
        fast_momentum_host_row_major: Some(&fast_momentum_host_row_major),
        trend_rows_ptr: &trend_rows_ptr,
        trend_host_row_major: Some(&trend_host_row_major),
    };

    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let mut sharpes: Vec<f64> = Vec::new();
    let mut dds: Vec<f64> = Vec::new();
    let mut pnls: Vec<f64> = Vec::new();
    let mut trades: Vec<u32> = Vec::new();

    let mut t0 = start_t;
    while t0 + 2 <= n {
        let end = t0.saturating_add(window).min(n);
        if end <= t0 + 1 {
            break;
        }
        let combo_fast = [0];
        let combo_trend = [0];
        let combo_start = [i32::try_from(start_t)
            .map_err(|_| "VCC GPU walk_forward canonical start_t exceeds i32 range".to_string())?];
        let combo_metric_start = [i32::try_from(t0)
            .map_err(|_| "VCC GPU walk_forward start_t exceeds i32 range".to_string())?];
        let combo_end = [i32::try_from(end)
            .map_err(|_| "VCC GPU walk_forward end_t exceeds i32 range".to_string())?];
        let combos = VccGpuComboBatch {
            fast_row_index: &combo_fast,
            trend_row_index: &combo_trend,
            start_t: &combo_start,
            metric_start_t: &combo_metric_start,
            end_t: &combo_end,
        };
        let metric = eval_vcc_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &params.strategy,
            vcc,
        )?
        .into_iter()
        .next()
        .ok_or_else(|| "VCC GPU walk_forward returned no metrics".to_string())?;

        sharpes.push(metric.sharpe);
        dds.push(metric.max_dd);
        pnls.push(metric.pnl);
        trades.push(metric.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    sharpes.sort_by(|a, b| a.total_cmp(b));
    dds.sort_by(|a, b| a.total_cmp(b));
    pnls.sort_by(|a, b| a.total_cmp(b));

    Ok(WalkForwardStats {
        windows: sharpes.len(),
        sharpe_median: crate::stats_common::median(&sharpes),
        sharpe_worst: crate::stats_common::worst_min(&sharpes),
        max_dd_median: crate::stats_common::median(&dds),
        max_dd_worst: crate::stats_common::worst_max(&dds),
        pnl_median: crate::stats_common::median(&pnls),
        pnl_worst: crate::stats_common::worst_min(&pnls),
        trades_median: crate::stats_common::median_u32(&trades),
        trades_worst: crate::stats_common::worst_min_u32(&trades),
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(dead_code)]
fn compute_vcc_gpu_reopt_walk_forward_stats_for_candidates(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    finalist_params: &[CompositeParams],
    finalist_start_ts: &[usize],
    cfg: &ReoptWalkForwardConfigOptional,
) -> Result<Option<ReoptWalkForwardStats>, String> {
    if finalist_params.is_empty() || candles.close.len() < 3 {
        return Ok(None);
    }

    let top_n = cfg.top_n_candidates.max(1).min(finalist_params.len());
    let finalists = &finalist_params[..top_n];
    let finalist_start_ts = &finalist_start_ts[..top_n];
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    let first = &finalists[0];
    let first_vcc = first
        .vcc
        .as_ref()
        .ok_or_else(|| "VCC GPU reopt_walk_forward requires VCC candidate params".to_string())?;
    validate_strategy_cfg(&first.strategy)?;
    validate_vcc_params(first_vcc)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite VCC reopt_walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    let n = close.len();
    if n < 2 || ma_prices.len() != n {
        return Err("not enough candles or mismatched MA source length".to_string());
    }
    if n > i32::MAX as usize {
        return Err("VCC GPU reopt_walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let mut min_sq = u16::MAX;
    let mut max_sq = 0u16;
    let mut min_mo = u16::MAX;
    let mut max_mo = 0u16;
    let mut min_trend = u16::MAX;
    let mut max_trend = 0u16;
    let mut periods_by_candidate: Vec<(u16, u16, u16)> = Vec::with_capacity(finalists.len());
    let mut finalist_entries: Vec<(String, Vec<u8>, usize)> = Vec::with_capacity(finalists.len());

    for (idx, cand) in finalists.iter().enumerate() {
        let vcc = cand.vcc.as_ref().ok_or_else(|| {
            "GPU composite reopt_walk_forward currently requires all finalists to use VCC"
                .to_string()
        })?;
        if cand.asr.is_some() || cand.vemr.is_some() || cand.rocb.is_some() || cand.mkcs.is_some() {
            return Err(
                "GPU composite reopt_walk_forward currently requires all finalists to use VCC"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        if !serialized_eq(vcc, first_vcc)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires uniform VCC parameters"
                    .to_string(),
            );
        }

        let periods = resolve_vcc_periods(first_vcc, cand.fast_len, cand.slow_len);
        min_sq = min_sq.min(periods.squeeze);
        max_sq = max_sq.max(periods.squeeze);
        min_mo = min_mo.min(periods.momentum);
        max_mo = max_mo.max(periods.momentum);
        min_trend = min_trend.min(periods.trend);
        max_trend = max_trend.max(periods.trend);
        periods_by_candidate.push((periods.squeeze, periods.momentum, periods.trend));

        let key = serde_json::to_string(cand)
            .unwrap_or_else(|_| format!("vcc:{}:{}", cand.fast_len, cand.slow_len));
        finalist_entries.push((
            key,
            serialize_composite_candidate_key(cand)?,
            finalist_start_ts[idx],
        ));
    }

    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )
    .map_err(|e| e.to_string())?;
    let fast_squeeze_sweep = compute_vcc_ttm_squeeze_sweep_cuda_device(
        &prepared_cuda_inputs,
        first_vcc,
        min_sq,
        max_sq,
        1,
        device_id,
    )?;
    let trend_sweep = compute_vcc_ttm_trend_sweep_cuda_device(
        &prepared_cuda_inputs,
        min_trend,
        max_trend,
        1,
        device_id,
    )?;

    let mut fast_row_index: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut trend_row_index: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut fast_squeeze_rows: Vec<u64> = Vec::new();
    let mut fast_squeeze_host_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut fast_momentum_host_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut trend_rows_ptr: Vec<u64> = Vec::new();
    let mut trend_host_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut fast_lookup: HashMap<(u16, u16), i32> = HashMap::new();
    let mut trend_lookup: HashMap<u16, i32> = HashMap::new();
    let mut fast_first_valid_lookup: HashMap<(u16, u16), usize> = HashMap::new();
    let mut trend_first_valid_lookup: HashMap<u16, usize> = HashMap::new();

    for ((sq, mo, trend), cand) in periods_by_candidate.into_iter().zip(finalists.iter()) {
        let fast_key = (sq, mo);
        let fast_idx = if let Some(idx) = fast_lookup.get(&fast_key) {
            *idx
        } else {
            let sq_ptr = fast_squeeze_sweep
                .row_ptr_for_period(sq)
                .ok_or_else(|| format!("VCC GPU reopt missing squeeze row for period={sq}"))?;
            let idx = i32::try_from(fast_squeeze_rows.len())
                .map_err(|_| "VCC GPU reopt fast row index exceeds i32 range".to_string())?;
            fast_squeeze_rows.push(sq_ptr);
            fast_lookup.insert(fast_key, idx);
            let fast_signals = compute_vcc_fast_signals(
                candles,
                &req.ma_source,
                &Backend::CpuOnly,
                first_vcc,
                sq,
                mo,
                Some(&prepared_cuda_inputs),
            )?;
            let parity = vcc_momentum_roundtrip_parity(&fast_signals.momentum);
            if !vcc_momentum_roundtrip_is_fp32_stable(&fast_signals.momentum) {
                return Err(format!(
                    "GPU composite VCC validation falls back to CPU because the requested momentum rows are not fp32-stable (sign mismatches={}, delta-sign mismatches={}).",
                    parity.sign_mismatch_count,
                    parity.delta_sign_mismatch_count
                ));
            }
            fast_squeeze_host_row_major
                .extend(fast_signals.squeeze_state.iter().map(|v| *v as f32));
            fast_momentum_host_row_major.extend(fast_signals.momentum.iter().map(|v| *v as f32));
            fast_first_valid_lookup.insert(fast_key, fast_signals.first_valid);
            idx
        };
        let trend_idx = if let Some(idx) = trend_lookup.get(&trend) {
            *idx
        } else {
            let tr_ptr = trend_sweep
                .row_ptr_for_period(trend)
                .ok_or_else(|| format!("VCC GPU reopt missing trend row for period={trend}"))?;
            let idx = i32::try_from(trend_rows_ptr.len())
                .map_err(|_| "VCC GPU reopt trend row index exceeds i32 range".to_string())?;
            trend_rows_ptr.push(tr_ptr);
            trend_lookup.insert(trend, idx);
            let trend_signals = compute_vcc_trend_signals(
                candles,
                &req.ma_source,
                &Backend::CpuOnly,
                trend,
                Some(&prepared_cuda_inputs),
            )?;
            trend_host_row_major.extend(trend_signals.trend_state.iter().map(|v| *v as f32));
            trend_first_valid_lookup.insert(trend, trend_signals.first_valid);
            idx
        };
        let fast_first_valid = *fast_first_valid_lookup
            .get(&fast_key)
            .ok_or_else(|| "VCC GPU reopt fast first_valid lookup miss".to_string())?;
        let trend_first_valid = *trend_first_valid_lookup
            .get(&trend)
            .ok_or_else(|| "VCC GPU reopt trend first_valid lookup miss".to_string())?;
        let start_t = compute_vcc_start_t(
            first_valid_close,
            first_valid_source,
            fast_first_valid,
            trend_first_valid,
            usize::from(sq.max(mo).max(trend)).max(1),
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after VCC GPU reopt warmup".to_string());
        }
        fast_row_index.push(fast_idx);
        trend_row_index.push(trend_idx);
        combo_start_t.push(
            i32::try_from(start_t)
                .map_err(|_| "VCC GPU reopt start_t exceeds i32 range".to_string())?,
        );
    }

    let signals = VccGpuDeviceSignalMatrices {
        close_f64: close,
        cols: n,
        fast_rows: fast_squeeze_rows.len(),
        trend_rows: trend_rows_ptr.len(),
        fast_squeeze_rows: &fast_squeeze_rows,
        fast_squeeze_host_row_major: Some(&fast_squeeze_host_row_major),
        fast_momentum_rows: &[],
        fast_momentum_host_row_major: Some(&fast_momentum_host_row_major),
        trend_rows_ptr: &trend_rows_ptr,
        trend_host_row_major: Some(&trend_host_row_major),
    };

    let common_start_t = finalist_start_ts
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .min(n.saturating_sub(2));
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let mut ranges: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut t0 = common_start_t;
    while t0 + 2 < n {
        let train_end = t0.saturating_add(window).min(n);
        if train_end <= t0 + 1 {
            break;
        }
        let test_end = train_end.saturating_add(step).min(n);
        if test_end <= train_end + 1 {
            break;
        }
        ranges.insert((t0, train_end));
        ranges.insert((train_end, test_end));
        if test_end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }
    if ranges.is_empty() {
        return Ok(None);
    }

    let candidate_keys: Vec<Vec<u8>> = finalist_entries
        .iter()
        .map(|(_, key, _)| key.clone())
        .collect();
    let mut range_metrics: HashMap<(usize, usize), HashMap<Vec<u8>, Metrics>> =
        HashMap::with_capacity(ranges.len());
    for (start, end) in ranges {
        let metric_start_vec: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start_i = usize::try_from(st).unwrap_or(0).max(start);
                i32::try_from(start_i)
                    .map_err(|_| "VCC GPU reopt start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let end_i =
            i32::try_from(end).map_err(|_| "VCC GPU reopt end_t exceeds i32 range".to_string())?;
        let end_vec = vec![end_i; metric_start_vec.len()];
        let combos = VccGpuComboBatch {
            fast_row_index: &fast_row_index,
            trend_row_index: &trend_row_index,
            start_t: &combo_start_t,
            metric_start_t: &metric_start_vec,
            end_t: &end_vec,
        };
        let metrics = eval_vcc_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &first.strategy,
            first_vcc,
        )?;
        let mut keyed = HashMap::with_capacity(metrics.len());
        for (key, metric) in candidate_keys.iter().zip(metrics.into_iter()) {
            keyed.insert(key.clone(), metric);
        }
        range_metrics.insert((start, end), keyed);
    }

    crate::validation_pipeline::compute_reopt_walk_forward_stats(
        &finalist_entries,
        n,
        cfg,
        |(_, _, start_t)| *start_t,
        |(_, key, _), start, end| {
            range_metrics
                .get(&(start, end))
                .and_then(|m| m.get(key))
                .cloned()
                .unwrap_or_else(|| {
                    panic!("missing VCC GPU reopt metrics for range {}..{}", start, end)
                })
        },
        |(key, _, _)| key.clone(),
    )
    .ok_or_else(|| "VCC GPU reopt_walk_forward did not produce any windows".to_string())
    .map(Some)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn try_prepare_base_composite_gpu_fixed_candidate_validation(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<GpuCompositeFixedCandidateValidationEval, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    if let Some(reason) = composite_gpu_validation_feature_blocker(req) {
        return Err(reason);
    }
    if req.candidates.is_empty() {
        return Err("no candidates to validate".to_string());
    }

    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    if first.asr.is_some()
        || first.vcc.is_some()
        || first.vemr.is_some()
        || first.rocb.is_some()
        || first.mkcs.is_some()
    {
        return Err(
            "GPU base composite validation requires plain CompositeCpu candidates only".to_string(),
        );
    }
    if composite_profile_from_strategy(&first.strategy) != StrategyProfileKind::CompositeCpu {
        return Err(
            "GPU base composite validation currently supports plain CompositeCpu only".to_string(),
        );
    }
    if first.signal_mode != CompositeSignalMode::HardGate {
        return Err(
            "GPU base composite validation currently supports signal_mode=HardGate only"
                .to_string(),
        );
    }
    validate_strategy_cfg(&first.strategy)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU base composite validation currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let fast_ma = first.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = first.slow_ma_type.trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("fast/slow MA type cannot be empty".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let mut fast_periods: BTreeSet<u16> = BTreeSet::new();
    let mut slow_periods: BTreeSet<u16> = BTreeSet::new();
    let mut candidate_pairs: Vec<(u16, u16)> = Vec::with_capacity(req.candidates.len());

    for cand in &req.candidates {
        if cand.asr.is_some()
            || cand.vcc.is_some()
            || cand.vemr.is_some()
            || cand.rocb.is_some()
            || cand.mkcs.is_some()
        {
            return Err(
                "GPU base composite validation currently requires all candidates to use plain CompositeCpu"
                    .to_string(),
            );
        }
        if composite_profile_from_strategy(&cand.strategy) != StrategyProfileKind::CompositeCpu {
            return Err(
                "GPU base composite validation currently supports plain CompositeCpu only"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU base composite validation currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.halftrend, &first.halftrend)? {
            return Err(
                "GPU base composite validation currently requires uniform HalfTrend parameters"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.macz, &first.macz)? {
            return Err(
                "GPU base composite validation currently requires uniform MACZ parameters"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.ott, &first.ott)? {
            return Err(
                "GPU base composite validation currently requires uniform OTT parameters"
                    .to_string(),
            );
        }
        if cand.signal_mode != first.signal_mode {
            return Err(
                "GPU base composite validation currently requires uniform signal_mode".to_string(),
            );
        }
        if cand.fast_ma_type.trim().to_ascii_lowercase() != fast_ma
            || cand.slow_ma_type.trim().to_ascii_lowercase() != slow_ma
        {
            return Err(
                "GPU base composite validation currently requires uniform fast/slow MA types"
                    .to_string(),
            );
        }

        fast_periods.insert(cand.fast_len);
        slow_periods.insert(cand.slow_len);
        candidate_pairs.push((cand.fast_len, cand.slow_len));
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    let n = close.len();
    if n < 2 || ma_prices.len() != n {
        return Err("not enough candles or mismatched MA source length".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let base_first_valid = first_valid_close.max(first_valid_source);

    let halftrend_trend =
        compute_halftrend_trend_with_backend(candles, &first.halftrend, &Backend::CpuOnly, None)?;
    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }
    let macz_hist = compute_macz_hist_with_backend(
        candles,
        &req.ma_source,
        &first.macz,
        &Backend::CpuOnly,
        None,
    )?;
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }
    let ott_line = compute_ott_line_with_backend(
        candles,
        &req.ma_source,
        &first.ott,
        &Backend::CpuOnly,
        None,
    )?;
    if ott_line.len() != n {
        return Err("OTT series length mismatch".to_string());
    }

    let fv_ht = first_finite_idx(&halftrend_trend)
        .ok_or_else(|| "halftrend trend series all non-finite".to_string())?;
    let fv_macz =
        first_finite_idx(&macz_hist).ok_or_else(|| "macz series all non-finite".to_string())?;
    let fv_ott =
        first_finite_idx(&ott_line).ok_or_else(|| "ott line series all non-finite".to_string())?;
    let shared_first_valid = base_first_valid.max(fv_ht).max(fv_macz).max(fv_ott);

    let regime = build_base_composite_hard_gate_regime_f32(
        &halftrend_trend,
        &macz_hist,
        ma_prices,
        &ott_line,
    )?;
    let device_regime = crate::gpu_backtest_kernel::upload_host_regime_filter(
        device_id,
        &regime,
        shared_first_valid,
    )?;

    let fast_periods: Vec<u16> = fast_periods.into_iter().collect();
    let slow_periods: Vec<u16> = slow_periods.into_iter().collect();
    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", &fast_ma, req.fast_ma_params.as_ref())?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", &slow_ma, req.slow_ma_params.as_ref())?;

    let (fast_values_f64, fast_cols) = ma_matrix_cpu_f64(
        candles,
        &fast_ma,
        &req.ma_source,
        &fast_periods,
        req.fast_ma_params.as_ref(),
        fast_alma_offset,
        fast_alma_sigma,
    )?;
    let (slow_values_f64, slow_cols) = ma_matrix_cpu_f64(
        candles,
        &slow_ma,
        &req.ma_source,
        &slow_periods,
        req.slow_ma_params.as_ref(),
        slow_alma_offset,
        slow_alma_sigma,
    )?;
    if fast_cols != n || slow_cols != n {
        return Err("base composite GPU validation MA matrix column mismatch".to_string());
    }

    let fast_values_f32: Vec<f32> = fast_values_f64.iter().map(|&v| v as f32).collect();
    let slow_values_f32: Vec<f32> = slow_values_f64.iter().map(|&v| v as f32).collect();

    let inputs = BaseCompositeGpuValidationInputs {
        device_id,
        fast_ma: fast_ma.clone(),
        slow_ma: slow_ma.clone(),
        fast_periods: fast_periods.clone(),
        slow_periods: slow_periods.clone(),
        fast_values_f32,
        fast_cols,
        slow_values_f32,
        slow_cols,
        device_regime,
        shared_first_valid,
        fast_alma_offset,
        fast_alma_sigma,
        slow_alma_offset,
        slow_alma_sigma,
    };
    let metric_by_pair = base_composite_gpu_metric_map(
        req,
        candles,
        &inputs,
        &first.strategy,
        req.candidates.len(),
        None,
    )?;

    let mut fast_first_valid_lookup: HashMap<u16, usize> =
        HashMap::with_capacity(fast_periods.len());
    for (row, &period) in fast_periods.iter().enumerate() {
        let start = row.saturating_mul(n);
        let end = start.saturating_add(n);
        let slice = &fast_values_f64[start..end];
        fast_first_valid_lookup.insert(
            period,
            first_finite_idx(slice)
                .ok_or_else(|| format!("fast MA row all non-finite for period={period}"))?,
        );
    }
    let mut slow_first_valid_lookup: HashMap<u16, usize> =
        HashMap::with_capacity(slow_periods.len());
    for (row, &period) in slow_periods.iter().enumerate() {
        let start = row.saturating_mul(n);
        let end = start.saturating_add(n);
        let slice = &slow_values_f64[start..end];
        slow_first_valid_lookup.insert(
            period,
            first_finite_idx(slice)
                .ok_or_else(|| format!("slow MA row all non-finite for period={period}"))?,
        );
    }

    let trade_offset = if first.strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let mut metrics: Vec<Metrics> = Vec::with_capacity(req.candidates.len());
    let mut start_ts: Vec<usize> = Vec::with_capacity(req.candidates.len());
    for &(fast_len, slow_len) in &candidate_pairs {
        let candidate_first_valid = base_first_valid
            .max(fv_ht)
            .max(fv_macz)
            .max(fv_ott)
            .max(
                *fast_first_valid_lookup
                    .get(&fast_len)
                    .ok_or_else(|| format!("missing fast first_valid for period={fast_len}"))?,
            )
            .max(
                *slow_first_valid_lookup
                    .get(&slow_len)
                    .ok_or_else(|| format!("missing slow first_valid for period={slow_len}"))?,
            );
        let start_t = candidate_first_valid
            .saturating_add(usize::from(fast_len.max(slow_len)).saturating_sub(1))
            .saturating_add(trade_offset);
        if start_t >= n {
            return Err("not enough candles after base composite GPU warmup".to_string());
        }
        let metric = metric_by_pair
            .get(&(fast_len, slow_len))
            .cloned()
            .ok_or_else(|| {
                format!(
                    "GPU base composite validation missing metric for pair ({fast_len}, {slow_len})"
                )
            })?;
        metrics.push(metric);
        start_ts.push(start_t);
    }

    let candidate_keys: Vec<Vec<u8>> = req
        .candidates
        .iter()
        .map(serialize_composite_candidate_key)
        .collect::<Result<_, _>>()?;
    let walk_forward_stats = if let Some(cfg) = req.walk_forward.as_ref() {
        Some(compute_base_composite_gpu_walk_forward_stats(
            req,
            candles,
            &inputs,
            &first.strategy,
            &candidate_pairs,
            &start_ts,
            cfg,
        )?)
    } else {
        None
    };
    let cscv_slices = if let Some(cfg) = req.cscv_pbo.as_ref() {
        compute_base_composite_gpu_cscv_slice_stats(
            req,
            candles,
            &inputs,
            &first.strategy,
            &candidate_keys,
            &candidate_pairs,
            &start_ts,
            cfg,
        )?
    } else {
        None
    };
    let spa_slice_sums = if let Some(cfg) = req.spa.as_ref() {
        compute_base_composite_gpu_spa_slice_sums(
            req,
            candles,
            &inputs,
            &first.strategy,
            &candidate_keys,
            &candidate_pairs,
            &start_ts,
            cfg,
        )?
    } else {
        None
    };

    let backend_route = match req.backend {
        Backend::Auto { .. } => Some("Auto->GpuKernel".to_string()),
        _ => Some(validation_backend_label(&req.backend).to_string()),
    };

    Ok(GpuCompositeFixedCandidateValidationEval {
        metrics,
        start_ts,
        candidate_indices: composite_candidate_index_map(&req.candidates)?,
        walk_forward_stats,
        cscv_slices,
        spa_slice_sums,
        base_composite_ctx: Some(inputs),
        backend_used: BackendUsed::Gpu,
        backend_route,
        backend_fallback_reason: None,
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn try_prepare_composite_gpu_fixed_candidate_validation(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<GpuCompositeFixedCandidateValidationEval, String> {
    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    if first.asr.is_some() {
        return try_prepare_asr_gpu_fixed_candidate_validation(req, candles);
    }
    if first.rocb.is_some() {
        return try_prepare_rocb_gpu_fixed_candidate_validation(req, candles);
    }
    if first.mkcs.is_some() {
        return try_prepare_mkcs_gpu_fixed_candidate_validation(req, candles);
    }
    if first.vemr.is_some() {
        return try_prepare_vemr_gpu_fixed_candidate_validation(req, candles);
    }
    if first.vcc.is_some() {
        return try_prepare_vcc_gpu_fixed_candidate_validation(req, candles);
    }
    try_prepare_base_composite_gpu_fixed_candidate_validation(req, candles)
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

fn default_reopt_walk_forward_top_n() -> usize {
    8
}

fn default_spa_slices() -> usize {
    16
}

fn default_spa_bootstrap_samples() -> usize {
    500
}

fn default_spa_block_length() -> usize {
    4
}

fn default_spa_benchmark() -> SpaBenchmarkKind {
    SpaBenchmarkKind::BuyAndHold
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WalkForwardConfigOptional {
    #[serde(default = "default_walk_forward_window_bars")]
    pub window_bars: usize,
    #[serde(default = "default_walk_forward_step_bars")]
    pub step_bars: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReoptWalkForwardConfigOptional {
    #[serde(default = "default_walk_forward_window_bars")]
    pub window_bars: usize,
    #[serde(default = "default_walk_forward_step_bars")]
    pub step_bars: usize,
    #[serde(default = "default_reopt_walk_forward_top_n")]
    pub top_n_candidates: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaBenchmarkKind {
    Flat,
    BuyAndHold,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaConfigOptional {
    #[serde(default = "default_spa_slices")]
    pub slices: usize,
    #[serde(default = "default_spa_bootstrap_samples")]
    pub bootstrap_samples: usize,
    #[serde(default = "default_spa_block_length")]
    pub block_length: usize,
    #[serde(default = "default_spa_benchmark")]
    pub benchmark: SpaBenchmarkKind,
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

#[derive(Clone, Debug, Serialize)]
pub struct ReoptWalkForwardStats {
    pub windows: usize,
    pub top_n_candidates: usize,
    pub unique_winners: usize,
    pub winner_changes: usize,
    pub dominant_winner_frac: f64,
    pub winner_param_drift_mean: f64,
    pub winner_param_drift_max: f64,
    pub train_score_median: f64,
    pub forward_positive_pnl_frac: f64,
    pub forward_sharpe_median: f64,
    pub forward_sharpe_worst: f64,
    pub forward_max_dd_median: f64,
    pub forward_max_dd_worst: f64,
    pub forward_pnl_median: f64,
    pub forward_pnl_worst: f64,
    pub forward_trades_median: u32,
    pub forward_trades_worst: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub winner_sequence: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SpaValidationStats {
    pub scope: String,
    pub candidate_count: usize,
    pub slice_count: usize,
    pub bootstrap_samples: usize,
    pub block_length: usize,
    pub benchmark: SpaBenchmarkKind,
    pub best_candidate_key: String,
    pub best_candidate_mean_diff: f64,
    pub best_candidate_t_stat: f64,
    pub p_value: f64,
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
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
    #[serde(default = "default_validation_backend")]
    pub backend: Backend,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    pub candidates: Vec<CompositeParams>,
    #[serde(default)]
    pub walk_forward: Option<WalkForwardConfigOptional>,
    #[serde(default)]
    pub reopt_walk_forward: Option<ReoptWalkForwardConfigOptional>,
    #[serde(default)]
    pub spa: Option<SpaConfigOptional>,
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
    pub min_cscv_oos_rank_frac: Option<f64>,
    #[serde(default)]
    pub min_cscv_oos_metric: Option<f64>,
    #[serde(default)]
    pub max_pbo: Option<f64>,
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
    pub track_record_bars: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_track_record_length_bars: Option<usize>,
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
    #[serde(default)]
    pub validation_verdict: ValidationCandidateVerdict,
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

    fn cscv_oos_rank_frac_p50(&self) -> Option<f64> {
        self.cscv_pbo.as_ref().and_then(|c| c.oos_rank_frac_p50)
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

    fn track_record_bars(&self) -> usize {
        self.track_record_bars
    }

    fn min_track_record_length_bars(&self) -> Option<usize> {
        self.min_track_record_length_bars
    }

    fn set_track_record_bars(&mut self, v: usize) {
        self.track_record_bars = v;
    }

    fn set_min_track_record_length_bars(&mut self, v: Option<usize>) {
        self.min_track_record_length_bars = v;
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

    fn validation_verdict(&self) -> &ValidationCandidateVerdict {
        &self.validation_verdict
    }

    fn set_validation_verdict(&mut self, v: ValidationCandidateVerdict) {
        self.validation_verdict = v;
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeValidationResult {
    pub data_id: String,
    pub runtime_ms: u64,
    pub backend_used: BackendUsed,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_fallback_reason: Option<String>,
    pub candidates: Vec<CompositeValidationCandidateResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reopt_walk_forward: Option<ReoptWalkForwardStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spa: Option<SpaValidationStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_ticker: Option<CrossTickerValidationSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cscv_pbo: Option<CscvPboResult>,
    pub run_verdict: ValidationRunVerdict,
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

struct MkcsCandidateSeries<'a> {
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
    Mkcs(MkcsCandidateSeries<'a>),
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
    #[cfg(feature = "cuda")] backend: &Backend,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    let custom_strategy_count = usize::from(params.asr.is_some())
        + usize::from(params.vcc.is_some())
        + usize::from(params.vemr.is_some())
        + usize::from(params.rocb.is_some())
        + usize::from(params.mkcs.is_some());
    if custom_strategy_count > 1 {
        return Err(
            "Select only one of ASR, VCC, VEMR, ROCB, or MKCS strategy config.".to_string(),
        );
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
    if let Some(mkcs_cfg) = params.mkcs.as_ref() {
        return build_mkcs_candidate_series(
            candles,
            lr,
            ott_prices,
            ma_source,
            params,
            mkcs_cfg,
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
        #[cfg(feature = "cuda")]
        backend,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
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
    #[cfg(feature = "cuda")] backend: &Backend,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
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

    let halftrend_trend = compute_halftrend_trend_with_backend(
        candles,
        &params.halftrend,
        #[cfg(feature = "cuda")]
        backend,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }

    if params.macz.use_volume && !candles.fields.volume {
        return Err("MACZ with volume requires a CSV with volume column.".to_string());
    }
    let macz_hist = compute_macz_hist_with_backend(
        candles,
        ma_source,
        &params.macz,
        #[cfg(feature = "cuda")]
        backend,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }

    let ott_line = compute_ott_line_with_backend(
        candles,
        ma_source,
        &params.ott,
        #[cfg(feature = "cuda")]
        backend,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    if ott_line.len() != n {
        return Err("OTT series length mismatch".to_string());
    }

    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", &fast_ma, fast_ma_params)?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", &slow_ma, slow_ma_params)?;

    #[cfg(feature = "cuda")]
    let ma_source_f32 = prepared_cuda_inputs.map(|inputs| inputs.source());
    let fast_periods = [params.fast_len];
    let (fast_mat, cols) = ma_matrix_with_backend_f64(
        candles,
        &fast_ma,
        ma_source,
        &fast_periods,
        fast_ma_params,
        fast_alma_offset,
        fast_alma_sigma,
        #[cfg(feature = "cuda")]
        Some(backend),
        #[cfg(not(feature = "cuda"))]
        None,
        #[cfg(feature = "cuda")]
        ma_source_f32,
        #[cfg(not(feature = "cuda"))]
        None,
    )?;
    if cols != n {
        return Err("fast MA series length mismatch".to_string());
    }

    let slow_periods = [params.slow_len];
    let (slow_mat, cols2) = ma_matrix_with_backend_f64(
        candles,
        &slow_ma,
        ma_source,
        &slow_periods,
        slow_ma_params,
        slow_alma_offset,
        slow_alma_sigma,
        #[cfg(feature = "cuda")]
        Some(backend),
        #[cfg(not(feature = "cuda"))]
        None,
        #[cfg(feature = "cuda")]
        ma_source_f32,
        #[cfg(not(feature = "cuda"))]
        None,
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

fn build_mkcs_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    ott_prices: &'a [f64],
    ma_source: &str,
    params: &CompositeParams,
    mkcs_cfg: &MkcsStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;
    validate_mkcs_params(mkcs_cfg)?;

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

    let backend = Backend::CpuOnly;
    let periods = resolve_mkcs_periods(mkcs_cfg, params.fast_len, params.slow_len);
    let compression: MkcsCompressionSignals = compute_mkcs_compression_signals(
        candles,
        ma_source,
        &backend,
        periods.stddev,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let breakout: MkcsBreakoutSignals = compute_mkcs_breakout_signals(
        candles,
        ma_source,
        &backend,
        periods.breakout,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let start_t = compute_mkcs_start_t(
        first_valid_close,
        first_valid_high,
        first_valid_low,
        compression.first_valid,
        breakout.first_valid,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    let desired = build_mkcs_desired_positions(
        close,
        &compression,
        &breakout,
        start_t,
        &params.strategy,
        mkcs_cfg,
    );

    Ok(CandidateSeries::Mkcs(MkcsCandidateSeries {
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
        CandidateSeries::Mkcs(series) => eval_asr_metrics_window_from_desired(
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
        CandidateSeries::Mkcs(series) => {
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
        CandidateSeries::Mkcs(series) => {
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
        CandidateSeries::Mkcs(series) => {
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
        CandidateSeries::Mkcs(series) => series.n,
    }
}

fn series_start_t(series: &CandidateSeries<'_>) -> usize {
    match series {
        CandidateSeries::Composite(series) => series.start_t,
        CandidateSeries::Asr(series) => series.start_t,
        CandidateSeries::Vcc(series) => series.start_t,
        CandidateSeries::Vemr(series) => series.start_t,
        CandidateSeries::Rocb(series) => series.start_t,
        CandidateSeries::Mkcs(series) => series.start_t,
    }
}

fn series_lr<'a>(series: &'a CandidateSeries<'a>) -> &'a [f64] {
    match series {
        CandidateSeries::Composite(series) => series.lr,
        CandidateSeries::Asr(series) => series.lr,
        CandidateSeries::Vcc(series) => series.lr,
        CandidateSeries::Vemr(series) => series.lr,
        CandidateSeries::Rocb(series) => series.lr,
        CandidateSeries::Mkcs(series) => series.lr,
    }
}

fn series_cfg<'a>(series: &'a CandidateSeries<'a>) -> &'a StrategyConfig {
    match series {
        CandidateSeries::Composite(series) => &series.cfg,
        CandidateSeries::Asr(series) => &series.cfg,
        CandidateSeries::Vcc(series) => &series.cfg,
        CandidateSeries::Vemr(series) => &series.cfg,
        CandidateSeries::Rocb(series) => &series.cfg,
        CandidateSeries::Mkcs(series) => &series.cfg,
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

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(clippy::too_many_arguments)]
fn compute_gpu_perturb_stats_for_composite_candidate(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    lr: &[f64],
    ma_prices: &[f64],
    base_first_valid: usize,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
    base: &CompositeParams,
    cfg: &PerturbConfig,
    cancel: &AtomicBool,
    rng: &mut StdRng,
) -> Result<Option<PerturbStats>, String> {
    let iters = cfg.iterations.min(1024);
    if iters == 0 {
        return Ok(None);
    }

    let mut sharpes: Vec<f64> = Vec::with_capacity(iters);
    let mut dds: Vec<f64> = Vec::with_capacity(iters);
    let mut pnls: Vec<f64> = Vec::with_capacity(iters);
    let mut pnl_pos: usize = 0;

    for _ in 0..iters {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let pp = perturb_params(base, cfg, rng);
        let temp_req = CompositeValidationRequest {
            data_id: req.data_id.clone(),
            backend: req.backend.clone(),
            ma_source: ma_source.to_string(),
            fast_ma_params: fast_ma_params.cloned(),
            slow_ma_params: slow_ma_params.cloned(),
            candidates: vec![pp.clone()],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: req.seed,
            max_candidates: 1,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let metric = match try_prepare_composite_gpu_fixed_candidate_validation(&temp_req, candles)
        {
            Ok(eval) => eval.metrics.into_iter().next().ok_or_else(|| {
                "GPU composite perturb validation returned no metrics for perturbed candidate"
                    .to_string()
            })?,
            Err(_) => {
                let s2 = build_candidate_series(
                    candles,
                    lr,
                    ma_prices,
                    base_first_valid,
                    ma_source,
                    fast_ma_params,
                    slow_ma_params,
                    &pp,
                    &req.backend,
                    prepared_cuda_inputs,
                )?;
                eval_metrics_window(&s2, series_start_t(&s2), series_n(&s2))
            }
        };

        if metric.pnl > 0.0 {
            pnl_pos += 1;
        }
        sharpes.push(metric.sharpe);
        dds.push(metric.max_dd);
        pnls.push(metric.pnl);
    }

    Ok(Some(crate::validation_pipeline::summarize_perturb_stats(
        cfg, iters, &sharpes, &dds, &pnls, pnl_pos,
    )))
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
    let reopt_walk_cfg = req.reopt_walk_forward.clone();
    let spa_cfg = req.spa.clone();
    let hard_gates = sanitize_hard_gates(
        req.min_psr,
        req.min_dsr,
        req.min_cscv_oos_rank_frac,
        req.min_cscv_oos_metric,
        req.max_pbo,
    );
    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles(candles, &ma_source).ok();

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let mut gpu_backend_fallback_reason = None::<String>;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let gpu_fixed_eval = match composite_validation_supports_gpu_fixed_candidates(&req) {
        Ok(_) => match try_prepare_composite_gpu_fixed_candidate_validation(&req, candles) {
            Ok(eval) => Some(eval),
            Err(reason) => {
                gpu_backend_fallback_reason = Some(reason);
                None
            }
        },
        Err(reason) => {
            if !matches!(req.backend, Backend::CpuOnly) {
                gpu_backend_fallback_reason = Some(reason);
            }
            None
        }
    };
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let _gpu_fixed_eval = None::<()>;

    let (out, cscv_pbo) = crate::validation_core::run_validation_candidates(
        &req.candidates,
        seed,
        cancel,
        progress,
        "composite_validate",
        "composite_validate_cscv_prep",
        &hard_gates,
        cscv_cfg.as_ref(),
        candles.close.len(),
        |_, p, rng| {
            #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
            if let Some(gpu_eval) = gpu_fixed_eval.as_ref() {
                let key = serialize_composite_candidate_key(p)?;
                let idx = *gpu_eval.candidate_indices.get(&key).ok_or_else(|| {
                    "GPU composite validation could not match candidate key back to request order"
                        .to_string()
                })?;
                let in_sample = gpu_eval.metrics.get(idx).cloned().ok_or_else(|| {
                    "GPU composite validation metrics index out of bounds".to_string()
                })?;
                let start_t0 = *gpu_eval.start_ts.get(idx).ok_or_else(|| {
                    "GPU composite validation warmup index out of bounds".to_string()
                })?;
                let walk_forward = if let Some(cfg) = walk_cfg.as_ref() {
                    if let Some(stats) = gpu_eval.walk_forward_stats.as_ref() {
                        Some(stats.get(idx).cloned().ok_or_else(|| {
                            "GPU composite validation walk_forward index out of bounds".to_string()
                        })?)
                    } else if p.asr.is_some() {
                        Some(compute_asr_gpu_walk_forward_stats_for_candidate(
                            &req, candles, p, cfg,
                        )?)
                    } else if p.vcc.is_some() {
                        Some(compute_vcc_gpu_walk_forward_stats_for_candidate(
                            &req, candles, p, cfg,
                        )?)
                    } else if p.mkcs.is_some() {
                        Some(compute_mkcs_gpu_walk_forward_stats_for_candidate(
                            &req, candles, p, cfg,
                        )?)
                    } else if p.rocb.is_some() {
                        Some(compute_rocb_gpu_walk_forward_stats_for_candidate(
                            &req, candles, p, cfg,
                        )?)
                    } else if p.vemr.is_some() {
                        Some(compute_vemr_gpu_walk_forward_stats_for_candidate(
                            &req, candles, p, cfg,
                        )?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let trade_bootstrap = if let Some(cfg) = mc_cfg.as_ref() {
                    let sims = cfg.simulations.min(50_000);
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
                        &req.backend,
                        #[cfg(feature = "cuda")]
                        prepared_cuda_inputs.as_ref(),
                    )?;
                    let trades = trades_for_series(&series);
                    Some(crate::validation_pipeline::compute_trade_bootstrap_stats(
                        &trades, sims, rng,
                    ))
                } else {
                    None
                };
                let robust_score = crate::validation_pipeline::robust_score_from_metrics(
                    &in_sample,
                    walk_forward
                        .as_ref()
                        .map(|w| (w.sharpe_median, w.max_dd_worst)),
                );
                let perturb = if let Some(cfg) = perturb_cfg.as_ref() {
                    compute_gpu_perturb_stats_for_composite_candidate(
                        &req,
                        candles,
                        &lr,
                        ma_prices,
                        base_first_valid,
                        &ma_source,
                        fast_ma_params.as_ref(),
                        slow_ma_params.as_ref(),
                        prepared_cuda_inputs.as_ref(),
                        p,
                        cfg,
                        cancel,
                        rng,
                    )?
                } else {
                    None
                };
                return Ok((
                    CompositeValidationCandidateResult {
                        params: p.clone(),
                        in_sample,
                        iterative_in_sample: None,
                        iterative_delta: None,
                        psr: None,
                        dsr: None,
                        track_record_bars: 0,
                        min_track_record_length_bars: None,
                        walk_forward,
                        perturb,
                        trade_bootstrap,
                        cross_ticker: None,
                        cross_ticker_pareto: None,
                        cscv_pbo: None,
                        validation_verdict: ValidationCandidateVerdict::default(),
                        robust_score,
                        robust_pareto: false,
                    },
                    start_t0,
                    candles.close.len().saturating_sub(start_t0),
                ));
            }

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
                &req.backend,
                #[cfg(feature = "cuda")]
                prepared_cuda_inputs.as_ref(),
            )?;
            let start_t0 = series_start_t(&series);
            let n_total = series_n(&series);
            let in_sample = eval_metrics_window(&series, start_t0, n_total);

            let walk_forward = if let Some(cfg) = walk_cfg.as_ref() {
                #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
                if gpu_fixed_eval.is_some() && p.asr.is_some() {
                    Some(compute_asr_gpu_walk_forward_stats_for_candidate(
                        &req, candles, p, cfg,
                    )?)
                } else if gpu_fixed_eval.is_some() && p.vcc.is_some() {
                    Some(compute_vcc_gpu_walk_forward_stats_for_candidate(
                        &req, candles, p, cfg,
                    )?)
                } else if gpu_fixed_eval.is_some() && p.mkcs.is_some() {
                    Some(compute_mkcs_gpu_walk_forward_stats_for_candidate(
                        &req, candles, p, cfg,
                    )?)
                } else if gpu_fixed_eval.is_some() && p.rocb.is_some() {
                    Some(compute_rocb_gpu_walk_forward_stats_for_candidate(
                        &req, candles, p, cfg,
                    )?)
                } else if gpu_fixed_eval.is_some() && p.vemr.is_some() {
                    Some(compute_vemr_gpu_walk_forward_stats_for_candidate(
                        &req, candles, p, cfg,
                    )?)
                } else {
                    Some(crate::validation_pipeline::compute_walk_forward_stats(
                        n_total,
                        start_t0,
                        cfg,
                        |t0, end| eval_metrics_window(&series, t0, end),
                    ))
                }
                #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
                {
                    Some(crate::validation_pipeline::compute_walk_forward_stats(
                        n_total,
                        start_t0,
                        cfg,
                        |t0, end| eval_metrics_window(&series, t0, end),
                    ))
                }
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
                            &req.backend,
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
                    track_record_bars: 0,
                    min_track_record_length_bars: None,
                    walk_forward,
                    perturb,
                    trade_bootstrap,
                    cross_ticker: None,
                    cross_ticker_pareto: None,
                    cscv_pbo: None,
                    validation_verdict: ValidationCandidateVerdict::default(),
                    robust_score,
                    robust_pareto: false,
                },
                start_t0,
                n_total.saturating_sub(start_t0),
            ))
        },
        |cand, cscv_start_t, slice_bars, slices| {
            #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
            if gpu_fixed_eval.is_some() {
                if let Some(slice_map) =
                    gpu_fixed_eval.as_ref().and_then(|g| g.cscv_slices.as_ref())
                {
                    let key = serialize_composite_candidate_key(&cand.params)?;
                    if let Some(slice_stats) = slice_map.get(&key) {
                        return Ok(slice_stats.clone());
                    }
                }
            }
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
                &req.backend,
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

    let reopt_walk_forward = if let Some(cfg) = reopt_walk_cfg.as_ref() {
        let finalist_params: Vec<CompositeParams> = out
            .iter()
            .take(cfg.top_n_candidates.max(1))
            .map(|cand| cand.params.clone())
            .collect();
        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        {
            if gpu_fixed_eval.is_some() {
                let finalist_start_ts: Vec<usize> = out
                    .iter()
                    .take(cfg.top_n_candidates.max(1))
                    .map(|cand| candles.close.len().saturating_sub(cand.track_record_bars))
                    .collect();
                if let Some(first_params) = finalist_params.first() {
                    if let Some(inputs) = gpu_fixed_eval
                        .as_ref()
                        .and_then(|eval| eval.base_composite_ctx.as_ref())
                    {
                        compute_base_composite_gpu_reopt_walk_forward_stats(
                            &req,
                            candles,
                            inputs,
                            &finalist_params,
                            &finalist_start_ts,
                            cfg,
                        )?
                    } else if first_params.asr.is_some() {
                        compute_asr_gpu_reopt_walk_forward_stats_for_candidates(
                            &req,
                            candles,
                            &finalist_params,
                            &finalist_start_ts,
                            cfg,
                        )?
                    } else if first_params.vcc.is_some() {
                        compute_vcc_gpu_reopt_walk_forward_stats_for_candidates(
                            &req,
                            candles,
                            &finalist_params,
                            &finalist_start_ts,
                            cfg,
                        )?
                    } else if first_params.mkcs.is_some() {
                        compute_mkcs_gpu_reopt_walk_forward_stats_for_candidates(
                            &req,
                            candles,
                            &finalist_params,
                            &finalist_start_ts,
                            cfg,
                        )?
                    } else if first_params.rocb.is_some() {
                        compute_rocb_gpu_reopt_walk_forward_stats_for_candidates(
                            &req,
                            candles,
                            &finalist_params,
                            &finalist_start_ts,
                            cfg,
                        )?
                    } else if first_params.vemr.is_some() {
                        compute_vemr_gpu_reopt_walk_forward_stats_for_candidates(
                            &req,
                            candles,
                            &finalist_params,
                            &finalist_start_ts,
                            cfg,
                        )?
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                let mut finalist_series: Vec<(String, CandidateSeries<'_>)> =
                    Vec::with_capacity(finalist_params.len());
                for params in finalist_params {
                    let key = serde_json::to_string(&params).unwrap_or_else(|_| {
                        format!(
                            "{}:{}:{}:{}",
                            params.fast_ma_type,
                            params.fast_len,
                            params.slow_ma_type,
                            params.slow_len
                        )
                    });
                    finalist_series.push((
                        key,
                        build_candidate_series(
                            candles,
                            &lr,
                            ma_prices,
                            base_first_valid,
                            &ma_source,
                            fast_ma_params.as_ref(),
                            slow_ma_params.as_ref(),
                            &params,
                            #[cfg(feature = "cuda")]
                            &req.backend,
                            #[cfg(feature = "cuda")]
                            prepared_cuda_inputs.as_ref(),
                        )?,
                    ));
                }
                crate::validation_pipeline::compute_reopt_walk_forward_stats(
                    &finalist_series,
                    candles.close.len(),
                    cfg,
                    |(_, series)| series_start_t(series),
                    |(_, series), t0, end| eval_metrics_window(series, t0, end),
                    |(key, _)| key.clone(),
                )
            }
        }
        #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
        {
            let mut finalist_series: Vec<(String, CandidateSeries<'_>)> =
                Vec::with_capacity(finalist_params.len());
            for params in finalist_params {
                let key = serde_json::to_string(&params).unwrap_or_else(|_| {
                    format!(
                        "{}:{}:{}:{}",
                        params.fast_ma_type, params.fast_len, params.slow_ma_type, params.slow_len
                    )
                });
                finalist_series.push((
                    key,
                    build_candidate_series(
                        candles,
                        &lr,
                        ma_prices,
                        base_first_valid,
                        &ma_source,
                        fast_ma_params.as_ref(),
                        slow_ma_params.as_ref(),
                        &params,
                        #[cfg(feature = "cuda")]
                        &req.backend,
                        #[cfg(feature = "cuda")]
                        prepared_cuda_inputs.as_ref(),
                    )?,
                ));
            }
            crate::validation_pipeline::compute_reopt_walk_forward_stats(
                &finalist_series,
                candles.close.len(),
                cfg,
                |(_, series)| series_start_t(series),
                |(_, series), t0, end| eval_metrics_window(series, t0, end),
                |(key, _)| key.clone(),
            )
        }
    } else {
        None
    };
    let spa = if let Some(cfg) = spa_cfg.as_ref() {
        let common_start_t = out
            .iter()
            .map(|cand| candles.close.len().saturating_sub(cand.track_record_bars))
            .max()
            .unwrap_or(0);
        let slices = cfg.slices.max(4);
        let slice_bars = candles
            .close
            .len()
            .saturating_sub(common_start_t)
            .checked_div(slices)
            .unwrap_or(0);
        if slice_bars >= 2 {
            let benchmark =
                benchmark_slice_returns(&lr, common_start_t, slice_bars, slices, &cfg.benchmark);
            let mut candidate_keys: Vec<String> = Vec::with_capacity(out.len());
            let mut candidate_slice_returns: Vec<Vec<f64>> = Vec::with_capacity(out.len());
            for cand in &out {
                let params = cand.params.clone();
                let key = serde_json::to_string(&params).unwrap_or_else(|_| {
                    format!(
                        "{}:{}:{}:{}",
                        params.fast_ma_type, params.fast_len, params.slow_ma_type, params.slow_len
                    )
                });
                candidate_keys.push(key);
                #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
                if let Some(gpu_eval) = gpu_fixed_eval.as_ref() {
                    if let Some(slice_map) = gpu_eval.spa_slice_sums.as_ref() {
                        if let Ok(serialized_key) = serialize_composite_candidate_key(&params) {
                            if let Some(slice_sums) = slice_map.get(&serialized_key) {
                                candidate_slice_returns.push(slice_sums.clone());
                                continue;
                            }
                        }
                    }
                }
                let series = build_candidate_series(
                    candles,
                    &lr,
                    ma_prices,
                    base_first_valid,
                    &ma_source,
                    fast_ma_params.as_ref(),
                    slow_ma_params.as_ref(),
                    &params,
                    #[cfg(feature = "cuda")]
                    &req.backend,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                candidate_slice_returns.push(slice_return_sums_for_series(
                    &series,
                    common_start_t,
                    slice_bars,
                    slices,
                ));
            }
            crate::validation_pipeline::compute_spa_validation_stats(
                &candidate_keys,
                &candidate_slice_returns,
                &benchmark,
                cfg,
                req.seed,
                cancel,
            )
        } else {
            Some(SpaValidationStats {
                scope: "validation_candidates".to_string(),
                candidate_count: out.len(),
                slice_count: slices,
                bootstrap_samples: cfg.bootstrap_samples.max(1),
                block_length: cfg.block_length.max(1),
                benchmark: cfg.benchmark.clone(),
                best_candidate_key: String::new(),
                best_candidate_mean_diff: 0.0,
                best_candidate_t_stat: 0.0,
                p_value: 1.0,
                passed: false,
                skipped_reason: Some("spa_skipped_not_enough_bars_after_warmup".to_string()),
            })
        }
    } else {
        None
    };

    let runtime_ms = start.elapsed().as_millis() as u64;
    let run_verdict = build_run_verdict(
        &out,
        req.candidates.len(),
        &hard_gates,
        cscv_pbo.as_ref(),
        spa.as_ref(),
    );
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let (backend_used, backend_route, backend_fallback_reason) =
        if let Some(gpu_eval) = gpu_fixed_eval.as_ref() {
            (
                gpu_eval.backend_used.clone(),
                gpu_eval.backend_route.clone(),
                gpu_eval.backend_fallback_reason.clone(),
            )
        } else {
            resolve_validation_backend_runtime(&req.backend, false, gpu_backend_fallback_reason)
        };
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let (backend_used, backend_route, backend_fallback_reason) =
        resolve_validation_backend_runtime(&req.backend, false, None);
    Ok(CompositeValidationResult {
        data_id: req.data_id,
        runtime_ms,
        backend_used,
        backend_route,
        backend_fallback_reason,
        candidates: out,
        reopt_walk_forward,
        spa,
        cross_ticker: None,
        cscv_pbo,
        run_verdict,
    })
}

fn slice_return_sums_for_series(
    series: &CandidateSeries<'_>,
    start_t: usize,
    slice_bars: usize,
    slices: usize,
) -> Vec<f64> {
    slice_return_stats_for_series(series, start_t, slice_bars, slices)
        .into_iter()
        .map(|entry| entry.sum)
        .collect()
}

fn benchmark_slice_returns(
    lr: &[f64],
    start_t: usize,
    slice_bars: usize,
    slices: usize,
    benchmark: &SpaBenchmarkKind,
) -> Vec<f64> {
    let mut out = vec![0.0; slices];
    if slices == 0 || slice_bars == 0 {
        return out;
    }
    let end_t = start_t
        .saturating_add(slice_bars.saturating_mul(slices))
        .min(lr.len());
    for t in start_t..end_t {
        let slice_idx = (t - start_t) / slice_bars;
        if slice_idx >= slices {
            break;
        }
        out[slice_idx] += match benchmark {
            SpaBenchmarkKind::Flat => 0.0,
            SpaBenchmarkKind::BuyAndHold => lr[t].exp_m1(),
        };
    }
    out
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

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn make_noisy_trending_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let x = i as f64;
            let trend = 100.0 + x * 0.08;
            let osc = (x / 19.0).sin() * 1.4 + (x / 7.0).cos() * 0.45;
            let o = trend + osc * 0.8;
            let c = trend + osc;
            let h = o.max(c) + 1.0 + (x / 11.0).sin().abs() * 0.25;
            let l = o.min(c) - 1.0 - (x / 13.0).cos().abs() * 0.25;
            open.push(o);
            high.push(h);
            low.push(l);
            close.push(c);
            vol.push(1_000.0 + (x / 5.0).sin().abs() * 250.0);
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base.clone()],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 1000,
                step_bars: 1000,
            }),
            reopt_walk_forward: None,
            perturb: Some(PerturbConfig {
                iterations: 3,
                pct_min: 0.01,
                pct_max: 0.01,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 50 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 32,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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
    fn composite_validation_cpu_request_reports_backend_route() {
        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let candidate = CompositeParams {
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "cpu-route".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![candidate],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite validation run");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("CpuOnly"));
        assert!(res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn composite_validation_gpu_request_reports_gpu_route_for_base_composite_in_sample() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let candidate = CompositeParams {
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "gpu-route".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![candidate],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite gpu-route in-sample");
        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn composite_validation_gpu_request_reports_gpu_route_for_base_composite_walk_forward() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let candidate = CompositeParams {
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "gpu-route-base-wf".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![candidate],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite gpu-route walk-forward");
        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
        assert_eq!(res.candidates.len(), 1);
        assert!(res.candidates[0].walk_forward.is_some());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_validation_gpu_kernel_smoke_in_sample() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "base-composite-gpu-validate".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
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
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "sma".to_string(),
                    slow_ma_type: "sma".to_string(),
                    fast_len: 14,
                    slow_len: 42,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 11,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu base composite validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
        assert_eq!(gpu_res.candidates.len(), 2);

        for gpu in &gpu_res.candidates {
            assert!(gpu.in_sample.pnl.is_finite());
            assert!(gpu.in_sample.sharpe.is_finite());
            assert!(gpu.in_sample.max_dd.is_finite());
            assert!(gpu.track_record_bars > 0);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_validation_gpu_kernel_smoke_walk_forward() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "base-composite-gpu-walk-forward".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
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
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "sma".to_string(),
                    slow_ma_type: "sma".to_string(),
                    fast_len: 14,
                    slow_len: 42,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 11,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu base composite walk_forward validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(gpu_res.candidates.len(), 2);

        for gpu in &gpu_res.candidates {
            let gpu_wf = gpu.walk_forward.as_ref().expect("gpu walk_forward");
            assert!(gpu_wf.windows > 0);
            assert!(gpu_wf.sharpe_median.is_finite());
            assert!(gpu_wf.sharpe_worst.is_finite());
            assert!(gpu_wf.max_dd_median.is_finite());
            assert!(gpu_wf.max_dd_worst.is_finite());
            assert!(gpu_wf.pnl_median.is_finite());
            assert!(gpu_wf.pnl_worst.is_finite());
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_validation_gpu_kernel_smoke_trade_bootstrap() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let req = CompositeValidationRequest {
            data_id: "base-composite-gpu-trade-bootstrap".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![CompositeParams {
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
                mkcs: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 24 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 13,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("base composite gpu trade_bootstrap validation");

        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(res.candidates.len(), 1);
        let mc = res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("gpu trade_bootstrap");
        assert_eq!(mc.simulations, 24);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_validation_gpu_kernel_smoke_perturb() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let req = CompositeValidationRequest {
            data_id: "base-composite-gpu-perturb".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![CompositeParams {
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
                mkcs: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: 0.05,
                pct_max: 0.05,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 17,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("base composite gpu perturb validation");

        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(res.candidates.len(), 1);
        let perturb = res.candidates[0].perturb.as_ref().expect("gpu perturb");
        assert_eq!(perturb.iterations, 8);
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "asr-test".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![candidate],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 600,
                step_bars: 300,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 24 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 7,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "vemr-test".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![candidate],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 600,
                step_bars: 300,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 24 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 11,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "vcc-test".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![candidate],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 600,
                step_bars: 300,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 24 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 13,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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
            mkcs: None,
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![bad, ok],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 1,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 200 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 32,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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
            mkcs: None,
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
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base, alt],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 4,
                max_combinations: 1000,
                threshold: 0.0,
            }),
            spa: None,
            seed: 1,
            max_candidates: 32,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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
    fn composite_validation_reopt_walk_forward_smoke() {
        let candles = make_trending_candles(8192);
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "reopt".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mk(10, 30), mk(14, 42)],
            walk_forward: None,
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 2000,
                step_bars: 1000,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 5,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("reopt validation run");
        let stats = res.reopt_walk_forward.as_ref().expect("reopt stats");
        assert!(stats.windows > 0);
        assert!(stats.unique_winners > 0);
        assert!(stats.dominant_winner_frac.is_finite());
        assert!(stats.forward_positive_pnl_frac.is_finite());
        assert!(stats.winner_param_drift_mean.is_finite());
        assert!(stats.winner_param_drift_max.is_finite());
        assert_eq!(stats.windows, stats.winner_sequence.len());
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 11,
            max_candidates: 32,
            validation_engine: ValidationEngine::Iterative,
            iterative: IterativeSimConfig {
                slippage_vol_mult: 0.25,
                ..IterativeSimConfig::default()
            },
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
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
                mkcs: None,
                macz: macz.clone(),
                ott: ott.clone(),
                signal_mode,
                strategy: strategy.clone(),
            })
            .collect();

        let mk_req = |validation_engine: ValidationEngine| CompositeValidationRequest {
            data_id: "opt-comp-vs-val".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: candidates.clone(),
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 19,
            max_candidates: 32,
            validation_engine,
            iterative: IterativeSimConfig {
                slippage_vol_mult: 0.0,
                ..IterativeSimConfig::default()
            },
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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
                mkcs: None,
                macz: pt.params.macz.clone(),
                ott: pt.params.ott.clone(),
                signal_mode: pt.params.signal_mode,
                strategy: pt.params.strategy.clone(),
            })
            .collect();
        assert!(!candidates.is_empty());

        let mk_req = |validation_engine: ValidationEngine| CompositeValidationRequest {
            data_id: "mo-stage-parity".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: candidates.clone(),
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 23,
            max_candidates: 32,
            validation_engine,
            iterative: IterativeSimConfig {
                slippage_vol_mult: 0.0,
                ..IterativeSimConfig::default()
            },
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "det".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base.clone()],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 1024,
                step_bars: 512,
            }),
            reopt_walk_forward: None,
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: 0.01,
                pct_max: 0.02,
                flip_bool_prob: 0.01,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 64 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let base_req = CompositeValidationRequest {
            data_id: "gate".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mk(8, 24), mk(12, 36), mk(20, 60)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
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

        assert_eq!(gated.candidates.len(), baseline.candidates.len());
        assert!(
            gated.run_verdict.candidates_survived_hard_gates <= gated.candidates.len(),
            "survivor count must not exceed total candidates"
        );
        for cand in &gated.candidates {
            let passed = cand.validation_verdict.passed;
            let psr_ok = cand.psr.unwrap_or(0.0) >= 0.80;
            let dsr_ok = cand.dsr.unwrap_or(0.0) >= 0.60;
            assert_eq!(passed, psr_ok && dsr_ok);
        }
    }

    #[test]
    fn asr_validation_probability_gates_filter_results() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |fast_len: u16, slow_len: u16| CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: Some(AsrStrategyParams::default()),
            vcc: None,
            vemr: None,
            rocb: None,
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "asr-gate".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mk(8, 24), mk(12, 36), mk(20, 60)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: Some(0.80),
            min_dsr: Some(0.60),
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let gated = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("asr gated run");

        for cand in &gated.candidates {
            let passed = cand.validation_verdict.passed;
            let psr_ok = cand.psr.unwrap_or(0.0) >= 0.80;
            let dsr_ok = cand.dsr.unwrap_or(0.0) >= 0.60;
            assert_eq!(passed, psr_ok && dsr_ok);
        }
    }

    #[test]
    fn vemr_validation_probability_gates_filter_results() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |fast_len: u16, slow_len: u16| CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: Some(VemrStrategyParams::default()),
            rocb: None,
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "vemr-gate".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mk(8, 24), mk(12, 36), mk(20, 60)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: Some(0.80),
            min_dsr: Some(0.60),
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let gated = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("vemr gated run");

        for cand in &gated.candidates {
            let passed = cand.validation_verdict.passed;
            let psr_ok = cand.psr.unwrap_or(0.0) >= 0.80;
            let dsr_ok = cand.dsr.unwrap_or(0.0) >= 0.60;
            assert_eq!(passed, psr_ok && dsr_ok);
        }
    }

    #[test]
    fn vcc_validation_probability_gates_filter_results() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |fast_len: u16, slow_len: u16| CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: Some(VccStrategyParams::default()),
            vemr: None,
            rocb: None,
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "vcc-gate".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mk(8, 24), mk(12, 36), mk(20, 60)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: Some(0.80),
            min_dsr: Some(0.60),
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let gated = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("vcc gated run");

        for cand in &gated.candidates {
            let passed = cand.validation_verdict.passed;
            let psr_ok = cand.psr.unwrap_or(0.0) >= 0.80;
            let dsr_ok = cand.dsr.unwrap_or(0.0) >= 0.60;
            assert_eq!(passed, psr_ok && dsr_ok);
        }
    }

    #[test]
    fn rocb_validation_probability_gates_filter_results() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |fast_len: u16, slow_len: u16| CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
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
            rocb: Some(RocbStrategyParams::default()),
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "rocb-gate".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mk(8, 24), mk(12, 36), mk(20, 60)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: Some(0.80),
            min_dsr: Some(0.60),
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let gated = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("rocb gated run");

        for cand in &gated.candidates {
            let passed = cand.validation_verdict.passed;
            let psr_ok = cand.psr.unwrap_or(0.0) >= 0.80;
            let dsr_ok = cand.dsr.unwrap_or(0.0) >= 0.60;
            assert_eq!(passed, psr_ok && dsr_ok);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_matches_cpu_metrics() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "asr-gpu-validate".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: Some(AsrStrategyParams::default()),
                    vcc: None,
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 12,
                    slow_len: 36,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: Some(AsrStrategyParams::default()),
                    vcc: None,
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 5,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu asr validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu asr validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());

        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!(
                (cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4,
                "pnl cpu={} gpu={}",
                cpu.in_sample.pnl,
                gpu.in_sample.pnl
            );
            assert!(
                (cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 1e-4,
                "sharpe cpu={} gpu={}",
                cpu.in_sample.sharpe,
                gpu.in_sample.sharpe
            );
            assert!(
                (cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-4,
                "max_dd cpu={} gpu={}",
                cpu.in_sample.max_dd,
                gpu.in_sample.max_dd
            );
            assert_eq!(
                cpu.in_sample.trades, gpu.in_sample.trades,
                "trades cpu={} gpu={}",
                cpu.in_sample.trades, gpu.in_sample.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_walk_forward_matches_cpu_metrics() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "asr-gpu-walk-forward".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 8,
                slow_len: 24,
                halftrend: HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: Some(AsrStrategyParams::default()),
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 29,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu asr walk_forward validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu asr walk_forward validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));

        let cpu = cpu_res.candidates.first().expect("cpu candidate");
        let gpu = gpu_res.candidates.first().expect("gpu candidate");
        let cpu_wf = cpu.walk_forward.as_ref().expect("cpu walk_forward");
        let gpu_wf = gpu.walk_forward.as_ref().expect("gpu walk_forward");

        assert_eq!(cpu_wf.windows, gpu_wf.windows);
        assert!(
            (cpu_wf.sharpe_median - gpu_wf.sharpe_median).abs() < 1e-4,
            "sharpe_median cpu={} gpu={}",
            cpu_wf.sharpe_median,
            gpu_wf.sharpe_median
        );
        assert!(
            (cpu_wf.sharpe_worst - gpu_wf.sharpe_worst).abs() < 1e-4,
            "sharpe_worst cpu={} gpu={}",
            cpu_wf.sharpe_worst,
            gpu_wf.sharpe_worst
        );
        assert!(
            (cpu_wf.max_dd_median - gpu_wf.max_dd_median).abs() < 1e-4,
            "max_dd_median cpu={} gpu={}",
            cpu_wf.max_dd_median,
            gpu_wf.max_dd_median
        );
        assert!(
            (cpu_wf.max_dd_worst - gpu_wf.max_dd_worst).abs() < 1e-4,
            "max_dd_worst cpu={} gpu={}",
            cpu_wf.max_dd_worst,
            gpu_wf.max_dd_worst
        );
        assert!(
            (cpu_wf.pnl_median - gpu_wf.pnl_median).abs() < 1e-4,
            "pnl_median cpu={} gpu={}",
            cpu_wf.pnl_median,
            gpu_wf.pnl_median
        );
        assert!(
            (cpu_wf.pnl_worst - gpu_wf.pnl_worst).abs() < 1e-4,
            "pnl_worst cpu={} gpu={}",
            cpu_wf.pnl_worst,
            gpu_wf.pnl_worst
        );
        assert_eq!(cpu_wf.trades_median, gpu_wf.trades_median);
        assert_eq!(cpu_wf.trades_worst, gpu_wf.trades_worst);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_cscv_matches_cpu() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "asr-gpu-cscv".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: Some(AsrStrategyParams::default()),
                    vcc: None,
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 12,
                    slow_len: 36,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: Some(AsrStrategyParams::default()),
                    vcc: None,
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 4,
                max_combinations: 1000,
                threshold: 0.0,
            }),
            spa: None,
            seed: 17,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu asr cscv validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu asr cscv validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_cscv = cpu_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_res.cscv_pbo.as_ref().expect("gpu cscv");
        assert_eq!(cpu_cscv.slices, gpu_cscv.slices);
        assert_eq!(cpu_cscv.slice_bars, gpu_cscv.slice_bars);
        assert!((cpu_cscv.pbo - gpu_cscv.pbo).abs() < 1e-3);
        assert!((cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs() < 1e-3);
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            assert!((cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs() < 1e-6);
            assert!(
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
            assert!(
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_spa_matches_cpu() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "asr-gpu-spa".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: Some(AsrStrategyParams::default()),
                    vcc: None,
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 12,
                    slow_len: 36,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: Some(AsrStrategyParams::default()),
                    vcc: None,
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: Some(SpaConfigOptional {
                slices: 8,
                bootstrap_samples: 128,
                block_length: 3,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 17,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu asr spa validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu asr spa validation");

        let cpu_spa = cpu_res.spa.as_ref().expect("cpu spa");
        let gpu_spa = gpu_res.spa.as_ref().expect("gpu spa");
        assert_eq!(cpu_spa.slice_count, gpu_spa.slice_count);
        assert_eq!(cpu_spa.candidate_count, gpu_spa.candidate_count);
        assert_eq!(cpu_spa.best_candidate_key, gpu_spa.best_candidate_key);
        assert!((cpu_spa.best_candidate_mean_diff - gpu_spa.best_candidate_mean_diff).abs() < 1e-3);
        assert!((cpu_spa.best_candidate_t_stat - gpu_spa.best_candidate_t_stat).abs() < 1e-3);
        assert!((cpu_spa.p_value - gpu_spa.p_value).abs() < 1e-9);
        assert_eq!(cpu_spa.passed, gpu_spa.passed);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_reopt_walk_forward_matches_cpu() {
        let candles = make_trending_candles(8192);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "asr-gpu-reopt".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: Some(AsrStrategyParams::default()),
                    vcc: None,
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 12,
                    slow_len: 36,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: Some(AsrStrategyParams::default()),
                    vcc: None,
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 2000,
                step_bars: 1000,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 31,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu asr reopt validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu asr reopt validation");

        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_eq!(cpu_stats.windows, gpu_stats.windows);
        assert_eq!(cpu_stats.top_n_candidates, gpu_stats.top_n_candidates);
        assert_eq!(cpu_stats.unique_winners, gpu_stats.unique_winners);
        assert_eq!(cpu_stats.winner_changes, gpu_stats.winner_changes);
        assert_eq!(cpu_stats.winner_sequence, gpu_stats.winner_sequence);
        assert!((cpu_stats.dominant_winner_frac - gpu_stats.dominant_winner_frac).abs() < 1e-6);
        assert!(
            (cpu_stats.winner_param_drift_mean - gpu_stats.winner_param_drift_mean).abs() < 1e-6
        );
        assert!((cpu_stats.winner_param_drift_max - gpu_stats.winner_param_drift_max).abs() < 1e-6);
        assert!((cpu_stats.train_score_median - gpu_stats.train_score_median).abs() < 5e-2);
        assert!(
            (cpu_stats.forward_positive_pnl_frac - gpu_stats.forward_positive_pnl_frac).abs()
                < 1e-6
        );
        assert!((cpu_stats.forward_sharpe_median - gpu_stats.forward_sharpe_median).abs() < 5e-2);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_trade_bootstrap_matches_cpu() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "asr-gpu-bootstrap".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 8,
                slow_len: 24,
                halftrend: HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: Some(AsrStrategyParams::default()),
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 64 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 43,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu asr bootstrap validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu asr bootstrap validation");

        let cpu_bs = cpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("cpu bootstrap");
        let gpu_bs = gpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("gpu bootstrap");
        assert_eq!(cpu_bs.simulations, gpu_bs.simulations);
        assert_eq!(cpu_bs.trade_count, gpu_bs.trade_count);
        assert!((cpu_bs.prob_pnl_positive - gpu_bs.prob_pnl_positive).abs() < 1e-12);
        assert!((cpu_bs.pnl_p05 - gpu_bs.pnl_p05).abs() < 1e-12);
        assert!((cpu_bs.pnl_p50 - gpu_bs.pnl_p50).abs() < 1e-12);
        assert!((cpu_bs.pnl_p95 - gpu_bs.pnl_p95).abs() < 1e-12);
        assert!((cpu_bs.max_dd_p50 - gpu_bs.max_dd_p50).abs() < 1e-12);
        assert!((cpu_bs.max_dd_p95 - gpu_bs.max_dd_p95).abs() < 1e-12);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn rocb_validation_gpu_kernel_matches_cpu_metrics() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "rocb-gpu-validate".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: None,
                    rocb: Some(RocbStrategyParams::default()),
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
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
                    rocb: Some(RocbStrategyParams::default()),
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 7,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu rocb validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu rocb validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());

        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!(
                (cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4,
                "pnl cpu={} gpu={}",
                cpu.in_sample.pnl,
                gpu.in_sample.pnl
            );
            assert!(
                (cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 1e-4,
                "sharpe cpu={} gpu={}",
                cpu.in_sample.sharpe,
                gpu.in_sample.sharpe
            );
            assert!(
                (cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-4,
                "max_dd cpu={} gpu={}",
                cpu.in_sample.max_dd,
                gpu.in_sample.max_dd
            );
            assert_eq!(
                cpu.in_sample.trades, gpu.in_sample.trades,
                "trades cpu={} gpu={}",
                cpu.in_sample.trades, gpu.in_sample.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn rocb_validation_gpu_kernel_walk_forward_matches_cpu_metrics() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "rocb-gpu-walk-forward".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 8,
                slow_len: 24,
                halftrend: HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: None,
                rocb: Some(RocbStrategyParams::default()),
                mkcs: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 7,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu rocb walk_forward validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu rocb walk_forward validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu = cpu_res.candidates.first().expect("cpu candidate");
        let gpu = gpu_res.candidates.first().expect("gpu candidate");
        let cpu_wf = cpu.walk_forward.as_ref().expect("cpu walk_forward");
        let gpu_wf = gpu.walk_forward.as_ref().expect("gpu walk_forward");

        assert_eq!(cpu_wf.windows, gpu_wf.windows);
        assert!(
            (cpu_wf.sharpe_median - gpu_wf.sharpe_median).abs() < 1e-4,
            "sharpe_median cpu={} gpu={}",
            cpu_wf.sharpe_median,
            gpu_wf.sharpe_median
        );
        assert!(
            (cpu_wf.sharpe_worst - gpu_wf.sharpe_worst).abs() < 1e-4,
            "sharpe_worst cpu={} gpu={}",
            cpu_wf.sharpe_worst,
            gpu_wf.sharpe_worst
        );
        assert!(
            (cpu_wf.max_dd_median - gpu_wf.max_dd_median).abs() < 1e-4,
            "max_dd_median cpu={} gpu={}",
            cpu_wf.max_dd_median,
            gpu_wf.max_dd_median
        );
        assert!(
            (cpu_wf.max_dd_worst - gpu_wf.max_dd_worst).abs() < 1e-4,
            "max_dd_worst cpu={} gpu={}",
            cpu_wf.max_dd_worst,
            gpu_wf.max_dd_worst
        );
        assert!(
            (cpu_wf.pnl_median - gpu_wf.pnl_median).abs() < 1e-4,
            "pnl_median cpu={} gpu={}",
            cpu_wf.pnl_median,
            gpu_wf.pnl_median
        );
        assert!(
            (cpu_wf.pnl_worst - gpu_wf.pnl_worst).abs() < 1e-4,
            "pnl_worst cpu={} gpu={}",
            cpu_wf.pnl_worst,
            gpu_wf.pnl_worst
        );
        assert_eq!(cpu_wf.trades_median, gpu_wf.trades_median);
        assert_eq!(cpu_wf.trades_worst, gpu_wf.trades_worst);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn rocb_validation_gpu_kernel_cscv_matches_cpu() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "rocb-gpu-cscv".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: None,
                    rocb: Some(RocbStrategyParams::default()),
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
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
                    rocb: Some(RocbStrategyParams::default()),
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 4,
                max_combinations: 1000,
                threshold: 0.0,
            }),
            spa: None,
            seed: 19,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu rocb cscv validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu rocb cscv validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_cscv = cpu_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_res.cscv_pbo.as_ref().expect("gpu cscv");
        assert_eq!(cpu_cscv.slices, gpu_cscv.slices);
        assert_eq!(cpu_cscv.slice_bars, gpu_cscv.slice_bars);
        assert!((cpu_cscv.pbo - gpu_cscv.pbo).abs() < 1e-3);
        assert!((cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs() < 1e-3);
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            assert!((cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs() < 1e-6);
            assert!(
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
            assert!(
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn rocb_validation_gpu_kernel_spa_matches_cpu() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "rocb-gpu-spa".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: None,
                    rocb: Some(RocbStrategyParams::default()),
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
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
                    rocb: Some(RocbStrategyParams::default()),
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: Some(SpaConfigOptional {
                slices: 8,
                bootstrap_samples: 128,
                block_length: 3,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 19,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu rocb spa validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu rocb spa validation");

        let cpu_spa = cpu_res.spa.as_ref().expect("cpu spa");
        let gpu_spa = gpu_res.spa.as_ref().expect("gpu spa");
        assert_eq!(cpu_spa.slice_count, gpu_spa.slice_count);
        assert_eq!(cpu_spa.candidate_count, gpu_spa.candidate_count);
        assert_eq!(cpu_spa.best_candidate_key, gpu_spa.best_candidate_key);
        assert!((cpu_spa.best_candidate_mean_diff - gpu_spa.best_candidate_mean_diff).abs() < 1e-3);
        assert!((cpu_spa.best_candidate_t_stat - gpu_spa.best_candidate_t_stat).abs() < 1e-3);
        assert!((cpu_spa.p_value - gpu_spa.p_value).abs() < 1e-9);
        assert_eq!(cpu_spa.passed, gpu_spa.passed);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn rocb_validation_gpu_kernel_reopt_walk_forward_matches_cpu() {
        let candles = make_trending_candles(8192);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "rocb-gpu-reopt".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: None,
                    rocb: Some(RocbStrategyParams::default()),
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
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
                    rocb: Some(RocbStrategyParams::default()),
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 2000,
                step_bars: 1000,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 37,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu rocb reopt validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu rocb reopt validation");

        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_eq!(cpu_stats.windows, gpu_stats.windows);
        assert_eq!(cpu_stats.top_n_candidates, gpu_stats.top_n_candidates);
        assert_eq!(cpu_stats.unique_winners, gpu_stats.unique_winners);
        assert_eq!(cpu_stats.winner_changes, gpu_stats.winner_changes);
        assert_eq!(cpu_stats.winner_sequence, gpu_stats.winner_sequence);
        assert!((cpu_stats.dominant_winner_frac - gpu_stats.dominant_winner_frac).abs() < 1e-6);
        assert!(
            (cpu_stats.winner_param_drift_mean - gpu_stats.winner_param_drift_mean).abs() < 1e-6
        );
        assert!((cpu_stats.winner_param_drift_max - gpu_stats.winner_param_drift_max).abs() < 1e-6);
        assert!((cpu_stats.train_score_median - gpu_stats.train_score_median).abs() < 5e-2);
        assert!(
            (cpu_stats.forward_positive_pnl_frac - gpu_stats.forward_positive_pnl_frac).abs()
                < 1e-6
        );
        assert!((cpu_stats.forward_sharpe_median - gpu_stats.forward_sharpe_median).abs() < 5e-2);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn rocb_validation_gpu_kernel_trade_bootstrap_matches_cpu() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "rocb-gpu-bootstrap".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 8,
                slow_len: 24,
                halftrend: HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: None,
                rocb: Some(RocbStrategyParams::default()),
                mkcs: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 64 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 47,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu rocb bootstrap validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu rocb bootstrap validation");

        let cpu_bs = cpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("cpu bootstrap");
        let gpu_bs = gpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("gpu bootstrap");
        assert_eq!(cpu_bs.simulations, gpu_bs.simulations);
        assert_eq!(cpu_bs.trade_count, gpu_bs.trade_count);
        assert!((cpu_bs.prob_pnl_positive - gpu_bs.prob_pnl_positive).abs() < 1e-12);
        assert!((cpu_bs.pnl_p05 - gpu_bs.pnl_p05).abs() < 1e-12);
        assert!((cpu_bs.pnl_p50 - gpu_bs.pnl_p50).abs() < 1e-12);
        assert!((cpu_bs.pnl_p95 - gpu_bs.pnl_p95).abs() < 1e-12);
        assert!((cpu_bs.max_dd_p50 - gpu_bs.max_dd_p50).abs() < 1e-12);
        assert!((cpu_bs.max_dd_p95 - gpu_bs.max_dd_p95).abs() < 1e-12);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_validation_gpu_kernel_matches_cpu_metrics() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vemr-gpu-validate".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: Some(VemrStrategyParams::default()),
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 12,
                    slow_len: 36,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: Some(VemrStrategyParams::default()),
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 11,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vemr validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vemr validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());

        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!(
                (cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4,
                "pnl cpu={} gpu={}",
                cpu.in_sample.pnl,
                gpu.in_sample.pnl
            );
            assert!(
                (cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 1e-4,
                "sharpe cpu={} gpu={}",
                cpu.in_sample.sharpe,
                gpu.in_sample.sharpe
            );
            assert!(
                (cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-4,
                "max_dd cpu={} gpu={}",
                cpu.in_sample.max_dd,
                gpu.in_sample.max_dd
            );
            assert_eq!(
                cpu.in_sample.trades, gpu.in_sample.trades,
                "trades cpu={} gpu={}",
                cpu.in_sample.trades, gpu.in_sample.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_validation_gpu_kernel_walk_forward_matches_cpu_metrics() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vemr-gpu-walk-forward".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 8,
                slow_len: 24,
                halftrend: HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: Some(VemrStrategyParams::default()),
                rocb: None,
                mkcs: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 23,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vemr walk_forward validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vemr walk_forward validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));

        let cpu = cpu_res.candidates.first().expect("cpu candidate");
        let gpu = gpu_res.candidates.first().expect("gpu candidate");
        let cpu_wf = cpu.walk_forward.as_ref().expect("cpu walk_forward");
        let gpu_wf = gpu.walk_forward.as_ref().expect("gpu walk_forward");

        assert_eq!(cpu_wf.windows, gpu_wf.windows);
        assert!(
            (cpu_wf.sharpe_median - gpu_wf.sharpe_median).abs() < 1e-4,
            "sharpe_median cpu={} gpu={}",
            cpu_wf.sharpe_median,
            gpu_wf.sharpe_median
        );
        assert!(
            (cpu_wf.sharpe_worst - gpu_wf.sharpe_worst).abs() < 1e-4,
            "sharpe_worst cpu={} gpu={}",
            cpu_wf.sharpe_worst,
            gpu_wf.sharpe_worst
        );
        assert!(
            (cpu_wf.max_dd_median - gpu_wf.max_dd_median).abs() < 1e-4,
            "max_dd_median cpu={} gpu={}",
            cpu_wf.max_dd_median,
            gpu_wf.max_dd_median
        );
        assert!(
            (cpu_wf.max_dd_worst - gpu_wf.max_dd_worst).abs() < 1e-4,
            "max_dd_worst cpu={} gpu={}",
            cpu_wf.max_dd_worst,
            gpu_wf.max_dd_worst
        );
        assert!(
            (cpu_wf.pnl_median - gpu_wf.pnl_median).abs() < 1e-4,
            "pnl_median cpu={} gpu={}",
            cpu_wf.pnl_median,
            gpu_wf.pnl_median
        );
        assert!(
            (cpu_wf.pnl_worst - gpu_wf.pnl_worst).abs() < 1e-4,
            "pnl_worst cpu={} gpu={}",
            cpu_wf.pnl_worst,
            gpu_wf.pnl_worst
        );
        assert_eq!(cpu_wf.trades_median, gpu_wf.trades_median);
        assert_eq!(cpu_wf.trades_worst, gpu_wf.trades_worst);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_validation_gpu_kernel_cscv_matches_cpu() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vemr-gpu-cscv".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: Some(VemrStrategyParams::default()),
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 12,
                    slow_len: 36,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: Some(VemrStrategyParams::default()),
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 4,
                max_combinations: 1000,
                threshold: 0.0,
            }),
            spa: None,
            seed: 23,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vemr cscv validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vemr cscv validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_cscv = cpu_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_res.cscv_pbo.as_ref().expect("gpu cscv");
        assert_eq!(cpu_cscv.slices, gpu_cscv.slices);
        assert_eq!(cpu_cscv.slice_bars, gpu_cscv.slice_bars);
        assert!((cpu_cscv.pbo - gpu_cscv.pbo).abs() < 1e-3);
        assert!((cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs() < 1e-3);
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            assert!((cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs() < 1e-6);
            assert!(
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
            assert!(
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_validation_gpu_kernel_spa_matches_cpu() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vemr-gpu-spa".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: Some(VemrStrategyParams::default()),
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 12,
                    slow_len: 36,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: Some(VemrStrategyParams::default()),
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: Some(SpaConfigOptional {
                slices: 8,
                bootstrap_samples: 128,
                block_length: 3,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 23,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vemr spa validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vemr spa validation");

        let cpu_spa = cpu_res.spa.as_ref().expect("cpu spa");
        let gpu_spa = gpu_res.spa.as_ref().expect("gpu spa");
        assert_eq!(cpu_spa.slice_count, gpu_spa.slice_count);
        assert_eq!(cpu_spa.candidate_count, gpu_spa.candidate_count);
        assert_eq!(cpu_spa.best_candidate_key, gpu_spa.best_candidate_key);
        assert!((cpu_spa.best_candidate_mean_diff - gpu_spa.best_candidate_mean_diff).abs() < 1e-3);
        assert!((cpu_spa.best_candidate_t_stat - gpu_spa.best_candidate_t_stat).abs() < 1e-3);
        assert!((cpu_spa.p_value - gpu_spa.p_value).abs() < 1e-9);
        assert_eq!(cpu_spa.passed, gpu_spa.passed);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_validation_gpu_kernel_reopt_walk_forward_matches_cpu() {
        let candles = make_trending_candles(8192);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vemr-gpu-reopt".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: Some(VemrStrategyParams::default()),
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 12,
                    slow_len: 36,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: None,
                    vemr: Some(VemrStrategyParams::default()),
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 2000,
                step_bars: 1000,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 41,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vemr reopt validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vemr reopt validation");

        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_eq!(cpu_stats.windows, gpu_stats.windows);
        assert_eq!(cpu_stats.top_n_candidates, gpu_stats.top_n_candidates);
        assert_eq!(cpu_stats.unique_winners, gpu_stats.unique_winners);
        assert_eq!(cpu_stats.winner_changes, gpu_stats.winner_changes);
        assert_eq!(cpu_stats.winner_sequence, gpu_stats.winner_sequence);
        assert!((cpu_stats.dominant_winner_frac - gpu_stats.dominant_winner_frac).abs() < 1e-6);
        assert!(
            (cpu_stats.winner_param_drift_mean - gpu_stats.winner_param_drift_mean).abs() < 1e-6
        );
        assert!((cpu_stats.winner_param_drift_max - gpu_stats.winner_param_drift_max).abs() < 1e-6);
        assert!((cpu_stats.train_score_median - gpu_stats.train_score_median).abs() < 5e-2);
        assert!(
            (cpu_stats.forward_positive_pnl_frac - gpu_stats.forward_positive_pnl_frac).abs()
                < 1e-6
        );
        assert!((cpu_stats.forward_sharpe_median - gpu_stats.forward_sharpe_median).abs() < 5e-2);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_validation_gpu_kernel_trade_bootstrap_matches_cpu() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vemr-gpu-bootstrap".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 8,
                slow_len: 24,
                halftrend: HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: Some(VemrStrategyParams::default()),
                rocb: None,
                mkcs: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 64 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 53,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vemr bootstrap validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vemr bootstrap validation");

        let cpu_bs = cpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("cpu bootstrap");
        let gpu_bs = gpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("gpu bootstrap");
        assert_eq!(cpu_bs.simulations, gpu_bs.simulations);
        assert_eq!(cpu_bs.trade_count, gpu_bs.trade_count);
        assert!((cpu_bs.prob_pnl_positive - gpu_bs.prob_pnl_positive).abs() < 1e-12);
        assert!((cpu_bs.pnl_p05 - gpu_bs.pnl_p05).abs() < 1e-12);
        assert!((cpu_bs.pnl_p50 - gpu_bs.pnl_p50).abs() < 1e-12);
        assert!((cpu_bs.pnl_p95 - gpu_bs.pnl_p95).abs() < 1e-12);
        assert!((cpu_bs.max_dd_p50 - gpu_bs.max_dd_p50).abs() < 1e-12);
        assert!((cpu_bs.max_dd_p95 - gpu_bs.max_dd_p95).abs() < 1e-12);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_matches_cpu_metrics_when_fp32_stable() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcc-gpu-validate".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: Some(VccStrategyParams::default()),
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 12,
                    slow_len: 36,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: Some(VccStrategyParams::default()),
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 13,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vcc validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcc validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(
            gpu_res.backend_fallback_reason.is_none(),
            "unexpected fallback reason: {:?}",
            gpu_res.backend_fallback_reason
        );
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());

        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!(
                (cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4,
                "pnl cpu={} gpu={}",
                cpu.in_sample.pnl,
                gpu.in_sample.pnl
            );
            assert!(
                (cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 1e-4,
                "sharpe cpu={} gpu={}",
                cpu.in_sample.sharpe,
                gpu.in_sample.sharpe
            );
            assert!(
                (cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-4,
                "max_dd cpu={} gpu={}",
                cpu.in_sample.max_dd,
                gpu.in_sample.max_dd
            );
            assert_eq!(
                cpu.in_sample.trades, gpu.in_sample.trades,
                "trades cpu={} gpu={}",
                cpu.in_sample.trades, gpu.in_sample.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_reports_cpu_fallback_when_momentum_is_not_fp32_stable() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let req = CompositeValidationRequest {
            data_id: "vcc-gpu-validate-monotonic".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 8,
                    slow_len: 24,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: Some(VccStrategyParams::default()),
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 12,
                    slow_len: 36,
                    halftrend: HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                    asr: None,
                    vcc: Some(VccStrategyParams::default()),
                    vemr: None,
                    rocb: None,
                    mkcs: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 13,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let gpu_res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu vcc validation monotonic");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Cpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
        assert!(
            gpu_res
                .backend_fallback_reason
                .as_deref()
                .unwrap_or_default()
                .contains("not fp32-stable"),
            "unexpected fallback reason: {:?}",
            gpu_res.backend_fallback_reason
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn vcc_test_candidate(fast_len: u16, slow_len: u16) -> CompositeParams {
        CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: Some(VccStrategyParams::default()),
            vemr: None,
            rocb: None,
            mkcs: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_walk_forward_matches_cpu_metrics_when_fp32_stable() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcc-gpu-walk-forward".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcc_test_candidate(8, 24)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 67,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vcc walk_forward validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcc walk_forward validation");

        let cpu_wf = cpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("cpu walk_forward");
        let gpu_wf = gpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("gpu walk_forward");
        assert_eq!(cpu_wf.windows, gpu_wf.windows);
        assert!((cpu_wf.sharpe_median - gpu_wf.sharpe_median).abs() < 1e-3);
        assert!((cpu_wf.sharpe_worst - gpu_wf.sharpe_worst).abs() < 1e-3);
        assert!((cpu_wf.max_dd_median - gpu_wf.max_dd_median).abs() < 1e-3);
        assert!((cpu_wf.max_dd_worst - gpu_wf.max_dd_worst).abs() < 1e-3);
        assert!((cpu_wf.pnl_median - gpu_wf.pnl_median).abs() < 1e-3);
        assert!((cpu_wf.pnl_worst - gpu_wf.pnl_worst).abs() < 1e-3);
        assert_eq!(cpu_wf.trades_median, gpu_wf.trades_median);
        assert_eq!(cpu_wf.trades_worst, gpu_wf.trades_worst);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_cscv_matches_cpu_when_fp32_stable() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcc-gpu-cscv".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcc_test_candidate(8, 24), vcc_test_candidate(12, 36)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 4,
                max_combinations: 1000,
                threshold: 0.0,
            }),
            spa: None,
            seed: 71,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vcc cscv validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcc cscv validation");

        let cpu_cscv = cpu_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_res.cscv_pbo.as_ref().expect("gpu cscv");
        assert_eq!(cpu_cscv.slices, gpu_cscv.slices);
        assert_eq!(cpu_cscv.slice_bars, gpu_cscv.slice_bars);
        assert!((cpu_cscv.pbo - gpu_cscv.pbo).abs() < 1e-3);
        assert!((cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs() < 1e-3);
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            assert!((cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs() < 1e-6);
            assert!(
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
            assert!(
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
        }
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_spa_matches_cpu_when_fp32_stable() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcc-gpu-spa".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcc_test_candidate(8, 24), vcc_test_candidate(12, 36)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: Some(SpaConfigOptional {
                slices: 8,
                bootstrap_samples: 128,
                block_length: 3,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 73,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vcc spa validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcc spa validation");

        let cpu_spa = cpu_res.spa.as_ref().expect("cpu spa");
        let gpu_spa = gpu_res.spa.as_ref().expect("gpu spa");
        assert_eq!(cpu_spa.slice_count, gpu_spa.slice_count);
        assert_eq!(cpu_spa.candidate_count, gpu_spa.candidate_count);
        assert_eq!(cpu_spa.best_candidate_key, gpu_spa.best_candidate_key);
        assert!((cpu_spa.best_candidate_mean_diff - gpu_spa.best_candidate_mean_diff).abs() < 1e-3);
        assert!((cpu_spa.best_candidate_t_stat - gpu_spa.best_candidate_t_stat).abs() < 1e-3);
        assert!((cpu_spa.p_value - gpu_spa.p_value).abs() < 1e-9);
        assert_eq!(cpu_spa.passed, gpu_spa.passed);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_reopt_walk_forward_matches_cpu_when_fp32_stable() {
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcc-gpu-reopt".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcc_test_candidate(8, 24), vcc_test_candidate(12, 36)],
            walk_forward: None,
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 2000,
                step_bars: 1000,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 79,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vcc reopt validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcc reopt validation");

        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_eq!(cpu_stats.windows, gpu_stats.windows);
        assert_eq!(cpu_stats.top_n_candidates, gpu_stats.top_n_candidates);
        assert_eq!(cpu_stats.unique_winners, gpu_stats.unique_winners);
        assert_eq!(cpu_stats.winner_changes, gpu_stats.winner_changes);
        assert_eq!(cpu_stats.winner_sequence, gpu_stats.winner_sequence);
        assert!((cpu_stats.dominant_winner_frac - gpu_stats.dominant_winner_frac).abs() < 1e-6);
        assert!(
            (cpu_stats.winner_param_drift_mean - gpu_stats.winner_param_drift_mean).abs() < 1e-6
        );
        assert!((cpu_stats.winner_param_drift_max - gpu_stats.winner_param_drift_max).abs() < 1e-6);
        assert!((cpu_stats.train_score_median - gpu_stats.train_score_median).abs() < 5e-2);
        assert!(
            (cpu_stats.forward_positive_pnl_frac - gpu_stats.forward_positive_pnl_frac).abs()
                < 1e-6
        );
        assert!((cpu_stats.forward_sharpe_median - gpu_stats.forward_sharpe_median).abs() < 5e-2);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_trade_bootstrap_matches_cpu_when_fp32_stable() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcc-gpu-bootstrap".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcc_test_candidate(8, 24)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 64 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 83,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu vcc bootstrap validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcc bootstrap validation");

        let cpu_bs = cpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("cpu bootstrap");
        let gpu_bs = gpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("gpu bootstrap");
        assert_eq!(cpu_bs.simulations, gpu_bs.simulations);
        assert_eq!(cpu_bs.trade_count, gpu_bs.trade_count);
        assert!((cpu_bs.prob_pnl_positive - gpu_bs.prob_pnl_positive).abs() < 1e-12);
        assert!((cpu_bs.pnl_p05 - gpu_bs.pnl_p05).abs() < 1e-12);
        assert!((cpu_bs.pnl_p50 - gpu_bs.pnl_p50).abs() < 1e-12);
        assert!((cpu_bs.pnl_p95 - gpu_bs.pnl_p95).abs() < 1e-12);
        assert!((cpu_bs.max_dd_p50 - gpu_bs.max_dd_p50).abs() < 1e-12);
        assert!((cpu_bs.max_dd_p95 - gpu_bs.max_dd_p95).abs() < 1e-12);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn mkcs_test_candidate(fast_len: u16, slow_len: u16) -> CompositeParams {
        CompositeParams {
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
            mkcs: Some(MkcsStrategyParams::default()),
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_validation_gpu_kernel_matches_cpu_metrics() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "mkcs-gpu-validate".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mkcs_test_candidate(10, 30), mkcs_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 41,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu mkcs validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu mkcs validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());

        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!(
                (cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4,
                "pnl cpu={} gpu={}",
                cpu.in_sample.pnl,
                gpu.in_sample.pnl
            );
            assert!(
                (cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 1e-4,
                "sharpe cpu={} gpu={}",
                cpu.in_sample.sharpe,
                gpu.in_sample.sharpe
            );
            assert!(
                (cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-4,
                "max_dd cpu={} gpu={}",
                cpu.in_sample.max_dd,
                gpu.in_sample.max_dd
            );
            assert_eq!(
                cpu.in_sample.trades, gpu.in_sample.trades,
                "trades cpu={} gpu={}",
                cpu.in_sample.trades, gpu.in_sample.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_validation_gpu_kernel_walk_forward_matches_cpu_metrics() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "mkcs-gpu-walk-forward".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mkcs_test_candidate(10, 30)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 43,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu mkcs walk_forward validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu mkcs walk_forward validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu = cpu_res.candidates.first().expect("cpu candidate");
        let gpu = gpu_res.candidates.first().expect("gpu candidate");
        let cpu_wf = cpu.walk_forward.as_ref().expect("cpu walk_forward");
        let gpu_wf = gpu.walk_forward.as_ref().expect("gpu walk_forward");

        assert_eq!(cpu_wf.windows, gpu_wf.windows);
        assert!(
            (cpu_wf.sharpe_median - gpu_wf.sharpe_median).abs() < 1e-4,
            "sharpe_median cpu={} gpu={}",
            cpu_wf.sharpe_median,
            gpu_wf.sharpe_median
        );
        assert!(
            (cpu_wf.sharpe_worst - gpu_wf.sharpe_worst).abs() < 1e-4,
            "sharpe_worst cpu={} gpu={}",
            cpu_wf.sharpe_worst,
            gpu_wf.sharpe_worst
        );
        assert!(
            (cpu_wf.max_dd_median - gpu_wf.max_dd_median).abs() < 1e-4,
            "max_dd_median cpu={} gpu={}",
            cpu_wf.max_dd_median,
            gpu_wf.max_dd_median
        );
        assert!(
            (cpu_wf.max_dd_worst - gpu_wf.max_dd_worst).abs() < 1e-4,
            "max_dd_worst cpu={} gpu={}",
            cpu_wf.max_dd_worst,
            gpu_wf.max_dd_worst
        );
        assert!(
            (cpu_wf.pnl_median - gpu_wf.pnl_median).abs() < 1e-4,
            "pnl_median cpu={} gpu={}",
            cpu_wf.pnl_median,
            gpu_wf.pnl_median
        );
        assert!(
            (cpu_wf.pnl_worst - gpu_wf.pnl_worst).abs() < 1e-4,
            "pnl_worst cpu={} gpu={}",
            cpu_wf.pnl_worst,
            gpu_wf.pnl_worst
        );
        assert_eq!(cpu_wf.trades_median, gpu_wf.trades_median);
        assert_eq!(cpu_wf.trades_worst, gpu_wf.trades_worst);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_validation_gpu_kernel_cscv_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "mkcs-gpu-cscv".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mkcs_test_candidate(10, 30), mkcs_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 4,
                max_combinations: 1000,
                threshold: 0.0,
            }),
            spa: None,
            seed: 47,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu mkcs cscv validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu mkcs cscv validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_cscv = cpu_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_res.cscv_pbo.as_ref().expect("gpu cscv");
        assert_eq!(cpu_cscv.slices, gpu_cscv.slices);
        assert_eq!(cpu_cscv.slice_bars, gpu_cscv.slice_bars);
        assert!((cpu_cscv.pbo - gpu_cscv.pbo).abs() < 1e-3);
        assert!((cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs() < 1e-3);
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            assert!((cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs() < 1e-6);
            assert!(
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
            assert!(
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_validation_gpu_kernel_spa_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "mkcs-gpu-spa".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mkcs_test_candidate(10, 30), mkcs_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: Some(SpaConfigOptional {
                slices: 4,
                benchmark: SpaBenchmarkKind::BuyAndHold,
                bootstrap_samples: 64,
                block_length: 1,
            }),
            seed: 49,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu mkcs spa validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu mkcs spa validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_spa = cpu_res.spa.as_ref().expect("cpu spa");
        let gpu_spa = gpu_res.spa.as_ref().expect("gpu spa");
        assert_eq!(cpu_spa.slice_count, gpu_spa.slice_count);
        assert_eq!(cpu_spa.bootstrap_samples, gpu_spa.bootstrap_samples);
        assert_eq!(cpu_spa.best_candidate_key, gpu_spa.best_candidate_key);
        assert!((cpu_spa.best_candidate_mean_diff - gpu_spa.best_candidate_mean_diff).abs() < 1e-3);
        assert!((cpu_spa.best_candidate_t_stat - gpu_spa.best_candidate_t_stat).abs() < 1e-3);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_validation_gpu_kernel_reopt_walk_forward_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "mkcs-gpu-reopt".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mkcs_test_candidate(10, 30), mkcs_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 51,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu mkcs reopt validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu mkcs reopt validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_eq!(cpu_stats.windows, gpu_stats.windows);
        assert_eq!(
            cpu_stats.forward_positive_pnl_frac,
            gpu_stats.forward_positive_pnl_frac
        );
        assert_eq!(cpu_stats.winner_sequence, gpu_stats.winner_sequence);
        assert!(
            (cpu_stats.winner_param_drift_mean - gpu_stats.winner_param_drift_mean).abs() < 1e-6
        );
        assert!((cpu_stats.winner_param_drift_max - gpu_stats.winner_param_drift_max).abs() < 1e-6);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_validation_gpu_kernel_trade_bootstrap_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "mkcs-gpu-bootstrap".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mkcs_test_candidate(10, 30)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 64 }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 53,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu mkcs bootstrap validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu mkcs bootstrap validation");

        let cpu_bs = cpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("cpu bootstrap");
        let gpu_bs = gpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("gpu bootstrap");
        assert_eq!(cpu_bs.simulations, gpu_bs.simulations);
        assert_eq!(cpu_bs.trade_count, gpu_bs.trade_count);
        assert!((cpu_bs.prob_pnl_positive - gpu_bs.prob_pnl_positive).abs() < 1e-12);
        assert!((cpu_bs.pnl_p05 - gpu_bs.pnl_p05).abs() < 1e-12);
        assert!((cpu_bs.pnl_p50 - gpu_bs.pnl_p50).abs() < 1e-12);
        assert!((cpu_bs.pnl_p95 - gpu_bs.pnl_p95).abs() < 1e-12);
        assert!((cpu_bs.max_dd_p50 - gpu_bs.max_dd_p50).abs() < 1e-12);
        assert!((cpu_bs.max_dd_p95 - gpu_bs.max_dd_p95).abs() < 1e-12);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_validation_gpu_kernel_perturb_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "mkcs-gpu-perturb".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mkcs_test_candidate(10, 30)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: Some(PerturbConfig {
                iterations: 32,
                pct_min: 0.05,
                pct_max: 0.10,
                flip_bool_prob: 0.1,
            }),
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 61,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu mkcs perturb validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu mkcs perturb validation");

        let cpu_pt = cpu_res.candidates[0].perturb.as_ref().expect("cpu perturb");
        let gpu_pt = gpu_res.candidates[0].perturb.as_ref().expect("gpu perturb");
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(cpu_pt.iterations, gpu_pt.iterations);
        assert!((cpu_pt.sharpe_mean - gpu_pt.sharpe_mean).abs() < 1e-4);
        assert!((cpu_pt.sharpe_worst - gpu_pt.sharpe_worst).abs() < 1e-4);
        assert!((cpu_pt.max_dd_mean - gpu_pt.max_dd_mean).abs() < 1e-4);
        assert!((cpu_pt.max_dd_worst - gpu_pt.max_dd_worst).abs() < 1e-4);
        assert!((cpu_pt.pnl_mean - gpu_pt.pnl_mean).abs() < 1e-4);
        assert!((cpu_pt.pnl_worst - gpu_pt.pnl_worst).abs() < 1e-4);
        assert!((cpu_pt.pnl_positive_frac - gpu_pt.pnl_positive_frac).abs() < 1e-12);
    }
}
