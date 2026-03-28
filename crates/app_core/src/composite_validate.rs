use crate::asr::{
    build_asr_desired_positions, compute_asr_fast_signals, compute_asr_kaufman_stops,
    compute_asr_slow_signals, compute_asr_start_t, eval_asr_curves_from_desired,
    eval_asr_metrics_from_desired, eval_asr_metrics_window_from_desired, resolve_asr_periods,
    validate_asr_params, AsrFastSignals, AsrKaufmanStops, AsrSlowSignals, AsrStrategyParams,
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
use crate::carm::{
    build_carm_desired_positions, compute_carm_signals, compute_carm_start_t, resolve_carm_periods,
    validate_carm_params, CarmSignals, CarmStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::carm::{
    compute_carm_mixed_signals, compute_carm_otto_signals, compute_carm_rocr_signal,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::carm_gpu_kernel::{
    eval_carm_metrics_gpu_kernel_device_rows, CarmGpuComboBatch, CarmGpuDeviceSignalMatrices,
};
use crate::cdmc::{
    build_cdmc_desired_positions, compute_cdmc_signals, compute_cdmc_start_t, resolve_cdmc_periods,
    validate_cdmc_params, CdmcSignals, CdmcStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::cdmc_gpu_kernel::{
    eval_cdmc_metrics_gpu_kernel_device_rows, CdmcGpuComboBatch, CdmcGpuDeviceSignalMatrices,
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
use crate::double_ma_validate::{
    FrictionStressStats, RegimeBucketStats, RegimeSliceRobustnessStats,
};
use crate::iterative_sim::{IterativeMetricsDelta, IterativeSimConfig, ValidationEngine};
use crate::kqsr::{
    build_kqsr_desired_positions, compute_kqsr_signals, compute_kqsr_start_t, resolve_kqsr_periods,
    validate_kqsr_params, KqsrSignals, KqsrStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::kqsr::{compute_kqsr_fast_signals_f32, compute_kqsr_slow_signals_f32};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::kqsr_gpu_kernel::{
    eval_kqsr_metrics_gpu_kernel_device_rows, KqsrGpuComboBatch, KqsrGpuDeviceSignalMatrices,
};
use crate::kvdf::{
    build_kvdf_desired_positions, compute_kvdf_signals, compute_kvdf_start_t, resolve_kvdf_periods,
    validate_kvdf_params, KvdfSignals, KvdfStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::kvdf::{
    compute_kvdf_dti_signal_f32, compute_kvdf_fast_signals_f32, compute_kvdf_shared_signals_f32,
    compute_kvdf_slow_signals_f32,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::kvdf_gpu_kernel::{
    eval_kvdf_metrics_gpu_kernel_device_rows, KvdfGpuComboBatch, KvdfGpuDeviceSignalMatrices,
};
use crate::mkcs::{
    build_mkcs_desired_positions, compute_mkcs_breakout_signals, compute_mkcs_compression_signals,
    compute_mkcs_start_t, resolve_mkcs_periods, validate_mkcs_params, MkcsBreakoutSignals,
    MkcsCompressionSignals, MkcsStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::mkcs::{
    compute_mkcs_donchian_sweeps_cuda_device, compute_mkcs_stddev_sweep_cuda_device,
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
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcc::vcc_momentum_roundtrip_parity;
use crate::vcc::{
    build_vcc_desired_positions, clear_vcc_route_diagnostics, compute_vcc_fast_signals,
    compute_vcc_start_t, compute_vcc_trend_signals, resolve_vcc_periods, take_vcc_route_diagnostics,
    validate_vcc_params, VccFastSignals, VccStrategyParams, VccTrendSignals,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcc::{
    compute_vcc_fast_dual_sweep_cuda_device, compute_vcc_ttm_squeeze_sweep_cuda_device,
    compute_vcc_ttm_trend_sweep_cuda_device,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcc_gpu_kernel::{
    eval_vcc_metrics_gpu_kernel_device_rows, eval_vcc_metrics_gpu_kernel_device_rows_detailed,
    VccGpuComboBatch, VccGpuDeviceSignalMatrices,
};
use crate::vcet::{
    build_vcet_desired_positions, clear_vcet_route_diagnostics, compute_vcet_chandelier_signals,
    compute_vcet_start_t, compute_vcet_vidya_signals, resolve_vcet_periods, take_vcet_route_diagnostics,
    validate_vcet_params, VcetChandelierSignals, VcetStrategyParams, VcetVidyaSignals,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcet::{
    compute_vcet_chandelier_signal_cuda_device, compute_vcet_vidya_signal_cuda_device,
    VcetOwnedDeviceMatrix,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcet_gpu_kernel::{
    eval_vcet_metrics_gpu_kernel_device_rows, VcetGpuComboBatch, VcetGpuDeviceSignalMatrices,
    VcetGpuOwnedSignalTables,
};
use crate::vemr::{
    build_vemr_desired_positions, clear_vemr_route_diagnostics, compute_vemr_fast_signals,
    compute_vemr_risk_signals, compute_vemr_start_t, resolve_vemr_periods, take_vemr_route_diagnostics,
    validate_vemr_params, VemrFastSignals, VemrRiskSignals, VemrStrategyParams,
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

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn prepared_cuda_inputs_for_source_hlc_on_device(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    device_id: usize,
) -> Result<PreparedCudaIndicatorInputs, String> {
    PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(
        candles,
        ma_source,
        device_id,
        &[crate::indicator_catalog::IndicatorInputKind::Ohlc],
    )
    .map_err(|e| e.to_string())
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn prepared_cuda_inputs_for_source_ohlcv_on_device(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    device_id: usize,
) -> Result<PreparedCudaIndicatorInputs, String> {
    PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(
        candles,
        ma_source,
        device_id,
        &[crate::indicator_catalog::IndicatorInputKind::Ohlcv],
    )
    .map_err(|e| e.to_string())
}

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
    pub vcet: Option<VcetStrategyParams>,
    #[serde(default)]
    pub mkcs: Option<MkcsStrategyParams>,
    #[serde(default)]
    pub kqsr: Option<KqsrStrategyParams>,
    #[serde(default)]
    pub carm: Option<CarmStrategyParams>,
    #[serde(default)]
    pub kvdf: Option<KvdfStrategyParams>,
    #[serde(default)]
    pub cdmc: Option<CdmcStrategyParams>,
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

fn clear_validation_strategy_route_diagnostics(candidates: &[CompositeParams]) {
    let Some(first) = candidates.first() else {
        return;
    };
    if first.vcc.is_some() {
        clear_vcc_route_diagnostics();
    } else if first.vemr.is_some() {
        clear_vemr_route_diagnostics();
    } else if first.vcet.is_some() {
        clear_vcet_route_diagnostics();
    }
}

fn take_validation_strategy_route_diagnostics(
    candidates: &[CompositeParams],
) -> Option<Vec<String>> {
    let first = candidates.first()?;
    if first.vcc.is_some() {
        take_vcc_route_diagnostics()
    } else if first.vemr.is_some() {
        take_vemr_route_diagnostics()
    } else if first.vcet.is_some() {
        take_vcet_route_diagnostics()
    } else {
        None
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
fn composite_auto_cpu_preference_reason(req: &CompositeValidationRequest) -> Option<&'static str> {
    if !matches!(req.backend, Backend::Auto { .. }) {
        return None;
    }
    let first = req.candidates.first()?;
    if first.asr.is_some() {
        return Some(
            "Auto composite validation currently prefers CpuOnly for ASR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.",
        );
    }
    if first.vemr.is_some() {
        return Some(
            "Auto composite validation currently prefers CpuOnly for VEMR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.",
        );
    }
    if first.rocb.is_some() {
        return Some(
            "Auto composite validation currently prefers CpuOnly for ROCB because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.",
        );
    }
    if first.mkcs.is_some() {
        return Some(
            "Auto composite validation currently prefers CpuOnly for MKCS because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.",
        );
    }
    if first.carm.is_some() {
        return Some(
            "Auto composite validation currently prefers CpuOnly for CARM because current advanced-stack benches show CPU faster than GPU on measured workloads.",
        );
    }
    if first.kqsr.is_some() {
        return Some(
            "Auto composite validation currently prefers CpuOnly for KQSR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.",
        );
    }
    if first.kvdf.is_some() {
        return Some(
            "Auto composite validation currently prefers CpuOnly for KVDF because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.",
        );
    }
    None
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
        Backend::GpuKernel { device_id } => Ok(device_id),
        Backend::Auto { device_id } => {
            if let Some(reason) = composite_auto_cpu_preference_reason(req) {
                Err(reason.to_string())
            } else {
                Ok(device_id)
            }
        }
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
                    || cand.vcet.is_some()
                    || cand.vemr.is_some()
                    || cand.mkcs.is_some()
                    || cand.kqsr.is_some()
                    || cand.carm.is_some()
                    || cand.kvdf.is_some()
                    || cand.cdmc.is_some()
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
                    || cand.vcet.is_some()
                    || cand.vemr.is_some()
                    || cand.mkcs.is_some()
                    || cand.kqsr.is_some()
                    || cand.carm.is_some()
                    || cand.kvdf.is_some()
                    || cand.cdmc.is_some()
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
                    || cand.vcet.is_some()
                    || cand.vemr.is_some()
                    || cand.mkcs.is_some()
                    || cand.kqsr.is_some()
                    || cand.carm.is_some()
                    || cand.kvdf.is_some()
                    || cand.cdmc.is_some()
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
                    || cand.vcet.is_some()
                    || cand.vemr.is_some()
                    || cand.mkcs.is_some()
                    || cand.kqsr.is_some()
                    || cand.carm.is_some()
                    || cand.kvdf.is_some()
                    || cand.cdmc.is_some()
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
                    || cand.vcet.is_some()
                    || cand.vemr.is_some()
                    || cand.mkcs.is_some()
                    || cand.kqsr.is_some()
                    || cand.carm.is_some()
                    || cand.kvdf.is_some()
                    || cand.cdmc.is_some()
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
        && cand.vcet.is_none()
        && cand.mkcs.is_none()
        && cand.kqsr.is_none()
        && cand.carm.is_none()
        && cand.kvdf.is_none()
        && cand.cdmc.is_none()
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
fn summarize_walk_forward_metric_samples_full(
    mut sharpes: Vec<f64>,
    mut dds: Vec<f64>,
    mut pnls: Vec<f64>,
    mut downside_ratios: Vec<f64>,
    mut ulcers: Vec<f64>,
    mut martins: Vec<f64>,
    trades: Vec<u32>,
) -> WalkForwardStats {
    sharpes.sort_by(|a, b| a.total_cmp(b));
    dds.sort_by(|a, b| a.total_cmp(b));
    pnls.sort_by(|a, b| a.total_cmp(b));
    downside_ratios.sort_by(|a, b| a.total_cmp(b));
    ulcers.sort_by(|a, b| a.total_cmp(b));
    martins.sort_by(|a, b| a.total_cmp(b));

    WalkForwardStats {
        windows: sharpes.len(),
        sharpe_median: crate::stats_common::median(&sharpes),
        sharpe_worst: crate::stats_common::worst_min(&sharpes),
        max_dd_median: crate::stats_common::median(&dds),
        max_dd_worst: crate::stats_common::worst_max(&dds),
        pnl_median: crate::stats_common::median(&pnls),
        pnl_worst: crate::stats_common::worst_min(&pnls),
        downside_vol_ratio_median: crate::stats_common::median(&downside_ratios),
        downside_vol_ratio_worst: crate::stats_common::worst_max(&downside_ratios),
        ulcer_index_median: crate::stats_common::median(&ulcers),
        ulcer_index_worst: crate::stats_common::worst_max(&ulcers),
        martin_ratio_median: crate::stats_common::median(&martins),
        martin_ratio_worst: crate::stats_common::worst_min(&martins),
        trades_median: crate::stats_common::median_u32(&trades),
        trades_worst: crate::stats_common::worst_min_u32(&trades),
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn summarize_walk_forward_metric_samples(
    sharpes: Vec<f64>,
    dds: Vec<f64>,
    pnls: Vec<f64>,
    trades: Vec<u32>,
) -> WalkForwardStats {
    summarize_walk_forward_metric_samples_full(
        sharpes,
        dds,
        pnls,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        trades,
    )
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn count_trade_events_from_desired_window(
    desired_pos: &[i32],
    start_t: usize,
    end_t: usize,
    strategy: &StrategyConfig,
) -> u32 {
    let end_t = end_t.min(desired_pos.len());
    if start_t >= end_t {
        return 0;
    }
    let mut pos = 0i32;
    let mut trades = 0u32;
    for &desired in &desired_pos[start_t..end_t] {
        let desired = desired.clamp(-1, 1);
        if desired != pos {
            if pos == 0 || desired == 0 {
                trades = trades.saturating_add(1);
                pos = desired;
            } else if !strategy.allow_flip {
                trades = trades.saturating_add(1);
                pos = 0;
            } else {
                trades = trades.saturating_add(2);
                pos = desired;
            }
        }
    }
    trades
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)
            .map_err(|e| format!("ASR prepared_cuda_inputs: {e}"))?;
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
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
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
            None,
            None,
            None,
            None,
            None,
            None,
            top_k.max(1),
            true,
            crate::gpu_backtest_kernel::KernelConfig {
                device_id: inputs.device_id,
                fast_alma_offset: inputs.fast_alma_offset,
                fast_alma_sigma: inputs.fast_alma_sigma,
                slow_alma_offset: inputs.slow_alma_offset,
                slow_alma_sigma: inputs.slow_alma_sigma,
                hurdle_tau: 0.0,
                include_trade_metrics: false,
                metric_contract: None,
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
        let mut downside_ratios = Vec::new();
        let mut ulcers = Vec::new();
        let mut martins = Vec::new();
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
            if let Some(ratio) =
                crate::validation_pipeline::downside_vol_ratio_from_metrics(&metric)
            {
                downside_ratios.push(ratio);
            }
            ulcers.push(metric.ulcer_index);
            martins.push(metric.martin_ratio);
            trades.push(metric.trades);
        }
        out.push(summarize_walk_forward_metric_samples_full(
            sharpes,
            dds,
            pnls,
            downside_ratios,
            ulcers,
            martins,
            trades,
        ));
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
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
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
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
    let mut downside_ratios: Vec<f64> = Vec::new();
    let mut ulcers: Vec<f64> = Vec::new();
    let mut martins: Vec<f64> = Vec::new();
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
        if let Some(ratio) = crate::validation_pipeline::downside_vol_ratio_from_metrics(&metric) {
            downside_ratios.push(ratio);
        }
        ulcers.push(metric.ulcer_index);
        martins.push(metric.martin_ratio);
        trades.push(metric.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    Ok(summarize_walk_forward_metric_samples_full(
        sharpes,
        dds,
        pnls,
        downside_ratios,
        ulcers,
        martins,
        trades,
    ))
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
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
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
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
    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
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
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
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
    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
    let stddev_sweep =
        compute_mkcs_stddev_sweep_cuda_device(&prepared_cuda_inputs, min_stddev, max_stddev, 1)?;
    let donchian_sweeps = compute_mkcs_donchian_sweeps_cuda_device(
        &prepared_cuda_inputs,
        min_breakout,
        max_breakout,
        1,
    )?;
    let upper_sweep = donchian_sweeps.upper;
    let lower_sweep = donchian_sweeps.lower;

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
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
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
    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
    let periods = resolve_mkcs_periods(mkcs, params.fast_len, params.slow_len);
    let stddev_sweep = compute_mkcs_stddev_sweep_cuda_device(
        &prepared_cuda_inputs,
        periods.stddev,
        periods.stddev,
        1,
    )?;
    let donchian_sweeps = compute_mkcs_donchian_sweeps_cuda_device(
        &prepared_cuda_inputs,
        periods.breakout,
        periods.breakout,
        1,
    )?;
    let upper_sweep = donchian_sweeps.upper;
    let lower_sweep = donchian_sweeps.lower;

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
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
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
    let mut downside_ratios: Vec<f64> = Vec::new();
    let mut ulcers: Vec<f64> = Vec::new();
    let mut martins: Vec<f64> = Vec::new();
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
        if let Some(ratio) = crate::validation_pipeline::downside_vol_ratio_from_metrics(&metric) {
            downside_ratios.push(ratio);
        }
        ulcers.push(metric.ulcer_index);
        martins.push(metric.martin_ratio);
        trades.push(metric.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    Ok(summarize_walk_forward_metric_samples_full(
        sharpes,
        dds,
        pnls,
        downside_ratios,
        ulcers,
        martins,
        trades,
    ))
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
    let stddev_sweep =
        compute_mkcs_stddev_sweep_cuda_device(&prepared_cuda_inputs, min_stddev, max_stddev, 1)?;
    let donchian_sweeps = compute_mkcs_donchian_sweeps_cuda_device(
        &prepared_cuda_inputs,
        min_breakout,
        max_breakout,
        1,
    )?;
    let upper_sweep = donchian_sweeps.upper;
    let lower_sweep = donchian_sweeps.lower;

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
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
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
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
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
    let mut downside_ratios: Vec<f64> = Vec::new();
    let mut ulcers: Vec<f64> = Vec::new();
    let mut martins: Vec<f64> = Vec::new();
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
        if let Some(ratio) = crate::validation_pipeline::downside_vol_ratio_from_metrics(&metric) {
            downside_ratios.push(ratio);
        }
        ulcers.push(metric.ulcer_index);
        martins.push(metric.martin_ratio);
        trades.push(metric.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    Ok(summarize_walk_forward_metric_samples_full(
        sharpes,
        dds,
        pnls,
        downside_ratios,
        ulcers,
        martins,
        trades,
    ))
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
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
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
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
fn compute_vcet_gpu_cscv_slice_stats_for_candidates(
    req: &CompositeValidationRequest,
    signals: &VcetGpuDeviceSignalMatrices<'_>,
    vidya_row_index: &[i32],
    ce_row_index: &[i32],
    combo_start_t: &[i32],
    start_ts: &[usize],
    vcet: &VcetStrategyParams,
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
            i32::try_from(end).map_err(|_| "VCET GPU CSCV end_t exceeds i32 range".to_string())?;
        let slice_start: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start = usize::try_from(st).unwrap_or(0).max(t0);
                i32::try_from(start)
                    .map_err(|_| "VCET GPU CSCV start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let slice_end = vec![end_i; slice_start.len()];
        let combos = VcetGpuComboBatch {
            vidya_row_index,
            ce_row_index,
            start_t: &slice_start,
            end_t: &slice_end,
        };
        let metrics = eval_vcet_metrics_gpu_kernel_device_rows(
            device_id,
            signals,
            &combos,
            &req.candidates[0].strategy,
            vcet,
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
fn try_prepare_vcet_gpu_fixed_candidate_validation(
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
    let first_vcet = first.vcet.as_ref().ok_or_else(|| {
        "GPU composite validation currently supports VCET candidates only".to_string()
    })?;
    if first.asr.is_some() || first.vcc.is_some() || first.vemr.is_some() || first.rocb.is_some() {
        return Err("GPU composite validation currently supports VCET candidates only".to_string());
    }
    validate_strategy_cfg(&first.strategy)?;
    validate_vcet_params(first_vcet)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite VCET validation currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let mut vidya_keys: Vec<(u16, u16)> = Vec::with_capacity(req.candidates.len());
    let mut ce_keys: Vec<u16> = Vec::with_capacity(req.candidates.len());
    let mut periods_by_candidate: Vec<((u16, u16), u16)> = Vec::with_capacity(req.candidates.len());

    for cand in &req.candidates {
        if cand.vcet.is_none()
            || cand.asr.is_some()
            || cand.vcc.is_some()
            || cand.vemr.is_some()
            || cand.rocb.is_some()
        {
            return Err(
                "GPU composite validation currently requires all candidates to use VCET"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite validation currently requires a uniform StrategyConfig".to_string(),
            );
        }
        if !serialized_eq(cand.vcet.as_ref().unwrap(), first_vcet)? {
            return Err(
                "GPU composite validation currently requires uniform VCET parameters".to_string(),
            );
        }

        let periods = resolve_vcet_periods(first_vcet, cand.fast_len, cand.slow_len);
        let vidya_key = (periods.vidya_short, periods.vidya_long);
        vidya_keys.push(vidya_key);
        ce_keys.push(periods.ce);
        periods_by_candidate.push((vidya_key, periods.ce));
    }
    vidya_keys.sort_unstable();
    vidya_keys.dedup();
    ce_keys.sort_unstable();
    ce_keys.dedup();

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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
    let close_f32: Vec<f32> = close.iter().map(|v| *v as f32).collect();

    let mut vidya_lookup: HashMap<(u16, u16), (i32, usize)> =
        HashMap::with_capacity(vidya_keys.len());
    let mut ce_lookup: HashMap<u16, (i32, usize)> = HashMap::with_capacity(ce_keys.len());
    let mut vidya_keepalive: Vec<VcetOwnedDeviceMatrix> = Vec::with_capacity(vidya_keys.len());
    let mut ce_long_keepalive: Vec<VcetOwnedDeviceMatrix> = Vec::with_capacity(ce_keys.len());
    let mut ce_short_keepalive: Vec<VcetOwnedDeviceMatrix> = Vec::with_capacity(ce_keys.len());

    for &(vidya_short, vidya_long) in &vidya_keys {
        let matrix = compute_vcet_vidya_signal_cuda_device(
            &prepared_cuda_inputs,
            crate::vcet::VcetResolvedPeriods {
                vidya_short,
                vidya_long,
                ce: 1,
            },
            first_vcet.vidya_alpha,
        )?;
        if matrix.rows != 1 || matrix.cols != n {
            return Err("VCET GPU VIDYA matrix shape mismatch".to_string());
        }
        let idx = i32::try_from(vidya_keepalive.len())
            .map_err(|_| "VCET GPU VIDYA row index exceeds i32 range".to_string())?;
        vidya_lookup.insert(
            (vidya_short, vidya_long),
            (idx, usize::from(vidya_long.saturating_sub(1))),
        );
        vidya_keepalive.push(matrix);
    }

    for &ce_period in &ce_keys {
        let (long_matrix, short_matrix) = compute_vcet_chandelier_signal_cuda_device(
            &prepared_cuda_inputs,
            ce_period,
            first_vcet.ce_mult,
            first_vcet.ce_use_close,
        )?;
        if long_matrix.rows != 1
            || short_matrix.rows != 1
            || long_matrix.cols != n
            || short_matrix.cols != n
        {
            return Err("VCET GPU chandelier matrix shape mismatch".to_string());
        }
        let idx = i32::try_from(ce_long_keepalive.len())
            .map_err(|_| "VCET GPU chandelier row index exceeds i32 range".to_string())?;
        ce_lookup.insert(ce_period, (idx, usize::from(ce_period.saturating_sub(1))));
        ce_long_keepalive.push(long_matrix);
        ce_short_keepalive.push(short_matrix);
    }

    let mut vidya_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut ce_row_index: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut start_ts: Vec<usize> = Vec::with_capacity(req.candidates.len());

    for (vidya_key, ce_period) in periods_by_candidate {
        let (vidya_row, vidya_first_valid) = *vidya_lookup
            .get(&vidya_key)
            .ok_or_else(|| "VCET GPU VIDYA row lookup miss".to_string())?;
        let (ce_row, ce_first_valid) = *ce_lookup
            .get(&ce_period)
            .ok_or_else(|| "VCET GPU chandelier row lookup miss".to_string())?;
        let start_t = compute_vcet_start_t(
            first_valid_close,
            first_valid_high,
            first_valid_low,
            vidya_first_valid,
            ce_first_valid,
            first.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after VCET GPU warmup".to_string());
        }

        vidya_row_index.push(vidya_row);
        ce_row_index.push(ce_row);
        combo_start_t.push(
            i32::try_from(start_t).map_err(|_| "VCET GPU start_t exceeds i32 range".to_string())?,
        );
        start_ts.push(start_t);
    }

    let signal_tables = VcetGpuOwnedSignalTables::from_owned_rows(
        n,
        vidya_keepalive,
        ce_long_keepalive,
        ce_short_keepalive,
    )?;
    let signals = signal_tables.as_signal_matrices(
        &close_f32,
        prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
    );
    let combo_end_t = vec![
        i32::try_from(n)
            .map_err(|_| "VCET GPU end_t exceeds i32 range".to_string())?;
        vidya_row_index.len()
    ];
    let combos = VcetGpuComboBatch {
        vidya_row_index: &vidya_row_index,
        ce_row_index: &ce_row_index,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };
    let metrics = eval_vcet_metrics_gpu_kernel_device_rows(
        device_id,
        &signals,
        &combos,
        &first.strategy,
        first_vcet,
    )?;
    if metrics.len() != req.candidates.len() {
        return Err("VCET GPU validation output length mismatch".to_string());
    }
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        compute_vcet_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &vidya_row_index,
            &ce_row_index,
            &combo_start_t,
            &start_ts,
            first_vcet,
            device_id,
            cscv_cfg,
        )?
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        let slice_stats = compute_vcet_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &vidya_row_index,
            &ce_row_index,
            &combo_start_t,
            &start_ts,
            first_vcet,
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
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
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
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
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
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
    let mut downside_ratios: Vec<f64> = Vec::new();
    let mut ulcers: Vec<f64> = Vec::new();
    let mut martins: Vec<f64> = Vec::new();
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
        if let Some(ratio) = crate::validation_pipeline::downside_vol_ratio_from_metrics(&metric) {
            downside_ratios.push(ratio);
        }
        ulcers.push(metric.ulcer_index);
        martins.push(metric.martin_ratio);
        trades.push(metric.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    Ok(summarize_walk_forward_metric_samples_full(
        sharpes,
        dds,
        pnls,
        downside_ratios,
        ulcers,
        martins,
        trades,
    ))
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
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
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
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
fn vcc_roundtripped_momentum_preserves_desired_positions(
    close: &[f64],
    first_valid_close: usize,
    first_valid_source: usize,
    fast_signals: &VccFastSignals,
    trend_signals: &VccTrendSignals,
    max_period: usize,
    strategy: &StrategyConfig,
    vcc: &VccStrategyParams,
) -> bool {
    let start_t = compute_vcc_start_t(
        first_valid_close,
        first_valid_source,
        fast_signals.first_valid,
        trend_signals.first_valid,
        max_period,
        strategy.trade_on_next_bar,
    );
    let desired_cpu =
        build_vcc_desired_positions(close, fast_signals, trend_signals, start_t, strategy, vcc);
    let fast_roundtrip = VccFastSignals {
        squeeze_state: fast_signals.squeeze_state.clone(),
        momentum: fast_signals
            .momentum
            .iter()
            .map(|v| (*v as f32) as f64)
            .collect(),
        first_valid: fast_signals.first_valid,
    };
    let desired_roundtrip = build_vcc_desired_positions(
        close,
        &fast_roundtrip,
        trend_signals,
        start_t,
        strategy,
        vcc,
    );
    desired_cpu == desired_roundtrip
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
    let lr = compute_log_returns_f64(close);
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let first_valid_source =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
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
    let mut fast_signal_lookup: HashMap<(u16, u16), VccFastSignals> = HashMap::new();
    let mut trend_signal_lookup: HashMap<u16, VccTrendSignals> = HashMap::new();
    let mut exact_metrics: Vec<Metrics> = Vec::with_capacity(req.candidates.len());
    let mut exact_series: Vec<CandidateSeries<'_>> = Vec::with_capacity(req.candidates.len());

    for ((sq, mo, trend), cand) in periods_by_candidate.into_iter().zip(req.candidates.iter()) {
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
            fast_squeeze_host_row_major
                .extend(fast_signals.squeeze_state.iter().map(|v| *v as f32));
            fast_momentum_host_row_major.extend(fast_signals.momentum.iter().map(|v| *v as f32));
            fast_first_valid_lookup.insert(fast_key, fast_signals.first_valid);
            fast_signal_lookup.insert(fast_key, fast_signals);
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
            trend_signal_lookup.insert(trend, trend_signals);
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

        let fast_signals = fast_signal_lookup
            .get(&fast_key)
            .ok_or_else(|| "VCC GPU validation fast signal lookup miss".to_string())?;
        let trend_signals = trend_signal_lookup
            .get(&trend)
            .ok_or_else(|| "VCC GPU validation trend signal lookup miss".to_string())?;
        let desired = build_vcc_desired_positions(
            close,
            fast_signals,
            trend_signals,
            start_t,
            &cand.strategy,
            first_vcc,
        );
        exact_metrics.push(eval_asr_metrics_from_desired(
            candles,
            &lr,
            &desired,
            start_t,
            &cand.strategy,
        ));
        exact_series.push(CandidateSeries::Vcc(VccCandidateSeries {
            candles,
            n,
            lr: &lr,
            desired,
            start_t,
            cfg: cand.strategy.clone(),
        }));

        fast_row_index.push(fast_idx);
        trend_row_index.push(trend_idx);
        combo_start_t.push(
            i32::try_from(start_t).map_err(|_| "VCC GPU start_t exceeds i32 range".to_string())?,
        );
        start_ts.push(start_t);
    }

    let signals = VccGpuDeviceSignalMatrices {
        close_f64: close,
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
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
    let metrics = exact_metrics;
    let candidate_keys: Vec<Vec<u8>> = req
        .candidates
        .iter()
        .map(serialize_composite_candidate_key)
        .collect::<Result<_, _>>()?;
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        exact_slice_stats_for_candidate_series(&candidate_keys, &exact_series, cscv_cfg)
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        let slice_stats = exact_slice_stats_for_candidate_series(
            &candidate_keys,
            &exact_series,
            &CscvPboConfig {
                slices: spa_cfg.slices.max(4),
                max_combinations: 0,
                threshold: 0.0,
            },
        );
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
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
    let max_period = usize::from(periods.squeeze.max(periods.momentum).max(periods.trend)).max(1);
    let trend_signals = compute_vcc_trend_signals(
        candles,
        &req.ma_source,
        &Backend::CpuOnly,
        periods.trend,
        Some(&prepared_cuda_inputs),
    )?;
    if !vcc_roundtripped_momentum_preserves_desired_positions(
        close,
        first_valid_close,
        first_valid_source,
        &fast_signals,
        &trend_signals,
        max_period,
        &params.strategy,
        vcc,
    ) {
        return Err(format!(
            "GPU composite VCC validation falls back to CPU because the requested momentum rows changes desired positions after f32 roundtrip (sign mismatches={}, delta-sign mismatches={}).",
            parity.sign_mismatch_count,
            parity.delta_sign_mismatch_count
        ));
    }
    let start_t = compute_vcc_start_t(
        first_valid_close,
        first_valid_source,
        fast_signals.first_valid,
        trend_signals.first_valid,
        max_period,
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
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
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
    let mut downside_ratios: Vec<f64> = Vec::new();
    let mut ulcers: Vec<f64> = Vec::new();
    let mut martins: Vec<f64> = Vec::new();
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
        if let Some(ratio) = crate::validation_pipeline::downside_vol_ratio_from_metrics(&metric) {
            downside_ratios.push(ratio);
        }
        ulcers.push(metric.ulcer_index);
        martins.push(metric.martin_ratio);
        trades.push(metric.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    Ok(summarize_walk_forward_metric_samples_full(
        sharpes,
        dds,
        pnls,
        downside_ratios,
        ulcers,
        martins,
        trades,
    ))
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
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
            let max_period = usize::from(sq.max(mo).max(trend)).max(1);
            if !vcc_roundtripped_momentum_preserves_desired_positions(
                close,
                first_valid_close,
                first_valid_source,
                &fast_signals,
                &compute_vcc_trend_signals(
                    candles,
                    &req.ma_source,
                    &Backend::CpuOnly,
                    trend,
                    Some(&prepared_cuda_inputs),
                )?,
                max_period,
                &cand.strategy,
                first_vcc,
            ) {
                return Err(format!(
                    "GPU composite VCC validation falls back to CPU because the requested momentum rows changes desired positions after f32 roundtrip (sign mismatches={}, delta-sign mismatches={}).",
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
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
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
fn compute_kqsr_gpu_walk_forward_stats_batch(
    signals: &KqsrGpuDeviceSignalMatrices<'_>,
    fast_row_index: &[i32],
    slow_row_index: &[i32],
    combo_start_t: &[i32],
    strategy: &StrategyConfig,
    kqsr: &KqsrStrategyParams,
    device_id: u32,
    cfg: &WalkForwardConfigOptional,
) -> Result<Vec<WalkForwardStats>, String> {
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let n = signals.cols;
    let mut out = Vec::with_capacity(fast_row_index.len());
    for idx in 0..fast_row_index.len() {
        let mut sharpes = Vec::new();
        let mut dds = Vec::new();
        let mut pnls = Vec::new();
        let mut trades = Vec::new();
        let mut t0 = usize::try_from(combo_start_t[idx]).unwrap_or(0);
        while t0 + 2 <= n {
            let end = t0.saturating_add(window).min(n);
            if end <= t0 + 1 {
                break;
            }
            let start_i = i32::try_from(t0)
                .map_err(|_| "KQSR GPU walk_forward start_t exceeds i32 range".to_string())?;
            let end_i = i32::try_from(end)
                .map_err(|_| "KQSR GPU walk_forward end_t exceeds i32 range".to_string())?;
            let combos = KqsrGpuComboBatch {
                fast_row_index: &fast_row_index[idx..idx + 1],
                slow_row_index: &slow_row_index[idx..idx + 1],
                start_t: &[start_i],
                end_t: &[end_i],
            };
            let metric = eval_kqsr_metrics_gpu_kernel_device_rows(
                device_id, signals, &combos, strategy, kqsr,
            )?
            .into_iter()
            .next()
            .ok_or_else(|| "KQSR GPU walk_forward returned no metrics".to_string())?;
            sharpes.push(metric.sharpe);
            dds.push(metric.max_dd);
            pnls.push(metric.pnl);
            trades.push(metric.trades);
            if end == n {
                break;
            }
            t0 = t0.saturating_add(step);
        }
        out.push(summarize_walk_forward_metric_samples(
            sharpes, dds, pnls, trades,
        ));
    }
    Ok(out)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_kqsr_gpu_cscv_slice_stats_for_candidates(
    req: &CompositeValidationRequest,
    signals: &KqsrGpuDeviceSignalMatrices<'_>,
    fast_row_index: &[i32],
    slow_row_index: &[i32],
    combo_start_t: &[i32],
    start_ts: &[usize],
    kqsr: &KqsrStrategyParams,
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
            i32::try_from(end).map_err(|_| "KQSR GPU CSCV end_t exceeds i32 range".to_string())?;
        let slice_start: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start = usize::try_from(st).unwrap_or(0).max(t0);
                i32::try_from(start)
                    .map_err(|_| "KQSR GPU CSCV start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let slice_end = vec![end_i; slice_start.len()];
        let combos = KqsrGpuComboBatch {
            fast_row_index,
            slow_row_index,
            start_t: &slice_start,
            end_t: &slice_end,
        };
        let metrics = eval_kqsr_metrics_gpu_kernel_device_rows(
            device_id,
            signals,
            &combos,
            &req.candidates[0].strategy,
            kqsr,
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
fn try_prepare_kqsr_gpu_fixed_candidate_validation(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<GpuCompositeFixedCandidateValidationEval, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    if let Some(reason) = composite_gpu_validation_feature_blocker(req) {
        return Err(reason);
    }
    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    let first_kqsr = first.kqsr.as_ref().ok_or_else(|| {
        "GPU composite validation currently supports KQSR candidates only".to_string()
    })?;
    if first.asr.is_some()
        || first.vcc.is_some()
        || first.vemr.is_some()
        || first.rocb.is_some()
        || first.mkcs.is_some()
        || first.carm.is_some()
        || first.kvdf.is_some()
        || first.cdmc.is_some()
    {
        return Err("GPU composite validation currently supports KQSR candidates only".to_string());
    }
    validate_strategy_cfg(&first.strategy)?;
    validate_kqsr_params(first_kqsr)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite KQSR validation currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;

    let mut fast_periods: Vec<u16> = req.candidates.iter().map(|cand| cand.fast_len).collect();
    let mut slow_periods: Vec<u16> = req.candidates.iter().map(|cand| cand.slow_len).collect();
    fast_periods.sort_unstable();
    fast_periods.dedup();
    slow_periods.sort_unstable();
    slow_periods.dedup();
    let slow_seed = *slow_periods
        .first()
        .ok_or_else(|| "KQSR validation slow period set is empty".to_string())?;
    let fast_seed = *fast_periods
        .first()
        .ok_or_else(|| "KQSR validation fast period set is empty".to_string())?;

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;

    let fast_rows = fast_periods.len();
    let slow_rows = slow_periods.len();
    let row_major_len_fast = fast_rows
        .checked_mul(n)
        .ok_or_else(|| "KQSR validation fast signal matrix overflow".to_string())?;
    let row_major_len_slow = slow_rows
        .checked_mul(n)
        .ok_or_else(|| "KQSR validation slow signal matrix overflow".to_string())?;

    let mut j_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut srsi_k_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut srsi_d_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut ift_rsi_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut fast_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(fast_rows);
    for &fast_len in &fast_periods {
        let periods = resolve_kqsr_periods(first_kqsr, fast_len, slow_seed);
        let signals = compute_kqsr_fast_signals_f32(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            first_kqsr,
            Some(&prepared_cuda_inputs),
        )
        .map_err(|e| format!("KQSR fast signals for fast_len={fast_len}: {e}"))?;
        fast_first_valid_lookup.insert(fast_len, signals.first_valid);
        j_row_major.extend_from_slice(&signals.j);
        srsi_k_row_major.extend_from_slice(&signals.srsi_k);
        srsi_d_row_major.extend_from_slice(&signals.srsi_d);
        ift_rsi_row_major.extend_from_slice(&signals.ift_rsi);
    }

    let mut qqe_fast_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut qqe_slow_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut prb_upper_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut prb_lower_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut slow_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(slow_rows);
    for &slow_len in &slow_periods {
        let periods = resolve_kqsr_periods(first_kqsr, fast_seed, slow_len);
        let signals = compute_kqsr_slow_signals_f32(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            Some(&prepared_cuda_inputs),
        )
        .map_err(|e| format!("KQSR slow signals for slow_len={slow_len}: {e}"))?;
        slow_first_valid_lookup.insert(slow_len, signals.first_valid);
        qqe_fast_row_major.extend_from_slice(&signals.qqe_fast);
        qqe_slow_row_major.extend_from_slice(&signals.qqe_slow);
        prb_upper_row_major.extend_from_slice(&signals.prb_upper);
        prb_lower_row_major.extend_from_slice(&signals.prb_lower);
    }

    let runtime = prepared_cuda_inputs.runtime();
    let d_j = runtime
        .upload_f32(j_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_srsi_k = runtime
        .upload_f32(srsi_k_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_srsi_d = runtime
        .upload_f32(srsi_d_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_ift_rsi = runtime
        .upload_f32(ift_rsi_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_qqe_fast = runtime
        .upload_f32(qqe_fast_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_qqe_slow = runtime
        .upload_f32(qqe_slow_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_prb_upper = runtime
        .upload_f32(prb_upper_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_prb_lower = runtime
        .upload_f32(prb_lower_row_major.as_slice())
        .map_err(|e| e.to_string())?;

    let fast_row_lookup: HashMap<u16, i32> = fast_periods
        .iter()
        .enumerate()
        .map(|(idx, &period)| {
            Ok((
                period,
                i32::try_from(idx)
                    .map_err(|_| "KQSR fast row index exceeds i32 range".to_string())?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let slow_row_lookup: HashMap<u16, i32> = slow_periods
        .iter()
        .enumerate()
        .map(|(idx, &period)| {
            Ok((
                period,
                i32::try_from(idx)
                    .map_err(|_| "KQSR slow row index exceeds i32 range".to_string())?,
            ))
        })
        .collect::<Result<_, String>>()?;

    let mut combo_fast_row_idx: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut combo_slow_row_idx: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(req.candidates.len());
    let mut start_ts: Vec<usize> = Vec::with_capacity(req.candidates.len());
    for cand in &req.candidates {
        if cand.kqsr.is_none()
            || cand.asr.is_some()
            || cand.vcc.is_some()
            || cand.vemr.is_some()
            || cand.rocb.is_some()
            || cand.mkcs.is_some()
            || cand.carm.is_some()
            || cand.kvdf.is_some()
            || cand.cdmc.is_some()
        {
            return Err(
                "GPU composite validation currently requires all candidates to use KQSR"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite validation currently requires a uniform StrategyConfig".to_string(),
            );
        }
        if !serialized_eq(cand.kqsr.as_ref().unwrap(), first_kqsr)? {
            return Err(
                "GPU composite validation currently requires uniform KQSR parameters".to_string(),
            );
        }

        let signals_first_valid =
            (*fast_first_valid_lookup.get(&cand.fast_len).ok_or_else(|| {
                format!(
                    "missing KQSR fast first_valid for fast_len={}",
                    cand.fast_len
                )
            })?)
            .max(*slow_first_valid_lookup.get(&cand.slow_len).ok_or_else(|| {
                format!(
                    "missing KQSR slow first_valid for slow_len={}",
                    cand.slow_len
                )
            })?);
        let start_t = compute_kqsr_start_t(
            fv_close,
            signals_first_valid,
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after KQSR GPU warmup".to_string());
        }
        combo_fast_row_idx.push(*fast_row_lookup.get(&cand.fast_len).ok_or_else(|| {
            format!("missing KQSR fast row index for fast_len={}", cand.fast_len)
        })?);
        combo_slow_row_idx.push(*slow_row_lookup.get(&cand.slow_len).ok_or_else(|| {
            format!("missing KQSR slow row index for slow_len={}", cand.slow_len)
        })?);
        combo_start_t.push(
            i32::try_from(start_t).map_err(|_| "KQSR start_t exceeds i32 range".to_string())?,
        );
        start_ts.push(start_t);
    }

    let signals = KqsrGpuDeviceSignalMatrices {
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
        cols: n,
        j_row_major_ptr: d_j.buffer().as_device_ptr().as_raw() as u64,
        fast_rows,
        fast_stride: n,
        srsi_k_row_major_ptr: d_srsi_k.buffer().as_device_ptr().as_raw() as u64,
        srsi_d_row_major_ptr: d_srsi_d.buffer().as_device_ptr().as_raw() as u64,
        ift_rsi_row_major_ptr: d_ift_rsi.buffer().as_device_ptr().as_raw() as u64,
        qqe_fast_row_major_ptr: d_qqe_fast.buffer().as_device_ptr().as_raw() as u64,
        qqe_slow_row_major_ptr: d_qqe_slow.buffer().as_device_ptr().as_raw() as u64,
        slow_rows,
        slow_stride: n,
        prb_upper_row_major_ptr: d_prb_upper.buffer().as_device_ptr().as_raw() as u64,
        prb_lower_row_major_ptr: d_prb_lower.buffer().as_device_ptr().as_raw() as u64,
    };
    let combo_end_t = vec![
        i32::try_from(n)
            .map_err(|_| "KQSR end_t exceeds i32 range".to_string())?;
        combo_fast_row_idx.len()
    ];
    let combos = KqsrGpuComboBatch {
        fast_row_index: &combo_fast_row_idx,
        slow_row_index: &combo_slow_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };
    let metrics = eval_kqsr_metrics_gpu_kernel_device_rows(
        device_id,
        &signals,
        &combos,
        &first.strategy,
        first_kqsr,
    )
    .map_err(|e| format!("KQSR in-sample GPU eval: {e}"))?;
    if metrics.len() != req.candidates.len() {
        return Err("KQSR GPU validation output length mismatch".to_string());
    }
    let walk_forward_stats = if let Some(walk_cfg) = req.walk_forward.as_ref() {
        Some(compute_kqsr_gpu_walk_forward_stats_batch(
            &signals,
            &combo_fast_row_idx,
            &combo_slow_row_idx,
            &combo_start_t,
            &first.strategy,
            first_kqsr,
            device_id,
            walk_cfg,
        )?)
    } else {
        None
    };
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        compute_kqsr_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &combo_fast_row_idx,
            &combo_slow_row_idx,
            &combo_start_t,
            &start_ts,
            first_kqsr,
            device_id,
            cscv_cfg,
        )?
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        let slice_stats = compute_kqsr_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &combo_fast_row_idx,
            &combo_slow_row_idx,
            &combo_start_t,
            &start_ts,
            first_kqsr,
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
        walk_forward_stats,
        cscv_slices,
        spa_slice_sums,
        base_composite_ctx: None,
        backend_used: BackendUsed::Gpu,
        backend_route,
        backend_fallback_reason: None,
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_kqsr_gpu_reopt_walk_forward_stats_for_candidates(
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
    let first_kqsr = first
        .kqsr
        .as_ref()
        .ok_or_else(|| "KQSR GPU reopt_walk_forward requires KQSR candidate params".to_string())?;
    validate_strategy_cfg(&first.strategy)?;
    validate_kqsr_params(first_kqsr)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite KQSR reopt_walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if n > i32::MAX as usize {
        return Err("KQSR GPU reopt_walk_forward length exceeds i32 range".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;

    let mut fast_periods: Vec<u16> = finalists.iter().map(|cand| cand.fast_len).collect();
    let mut slow_periods: Vec<u16> = finalists.iter().map(|cand| cand.slow_len).collect();
    fast_periods.sort_unstable();
    fast_periods.dedup();
    slow_periods.sort_unstable();
    slow_periods.dedup();
    let fast_rows = fast_periods.len();
    let slow_rows = slow_periods.len();
    let row_major_len_fast = fast_rows
        .checked_mul(n)
        .ok_or_else(|| "KQSR reopt fast signal matrix overflow".to_string())?;
    let row_major_len_slow = slow_rows
        .checked_mul(n)
        .ok_or_else(|| "KQSR reopt slow signal matrix overflow".to_string())?;
    let slow_seed = *slow_periods
        .first()
        .ok_or_else(|| "KQSR reopt slow period set is empty".to_string())?;
    let fast_seed = *fast_periods
        .first()
        .ok_or_else(|| "KQSR reopt fast period set is empty".to_string())?;

    let mut j_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut srsi_k_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut srsi_d_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut ift_rsi_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut fast_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(fast_rows);
    for &fast_len in &fast_periods {
        let periods = resolve_kqsr_periods(first_kqsr, fast_len, slow_seed);
        let fast = compute_kqsr_fast_signals_f32(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            first_kqsr,
            Some(&prepared_cuda_inputs),
        )?;
        fast_first_valid_lookup.insert(fast_len, fast.first_valid);
        j_row_major.extend_from_slice(&fast.j);
        srsi_k_row_major.extend_from_slice(&fast.srsi_k);
        srsi_d_row_major.extend_from_slice(&fast.srsi_d);
        ift_rsi_row_major.extend_from_slice(&fast.ift_rsi);
    }

    let mut qqe_fast_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut qqe_slow_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut prb_upper_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut prb_lower_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut slow_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(slow_rows);
    for &slow_len in &slow_periods {
        let periods = resolve_kqsr_periods(first_kqsr, fast_seed, slow_len);
        let slow = compute_kqsr_slow_signals_f32(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            Some(&prepared_cuda_inputs),
        )?;
        slow_first_valid_lookup.insert(slow_len, slow.first_valid);
        qqe_fast_row_major.extend_from_slice(&slow.qqe_fast);
        qqe_slow_row_major.extend_from_slice(&slow.qqe_slow);
        prb_upper_row_major.extend_from_slice(&slow.prb_upper);
        prb_lower_row_major.extend_from_slice(&slow.prb_lower);
    }

    let runtime = prepared_cuda_inputs.runtime();
    let d_j = runtime
        .upload_f32(j_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_srsi_k = runtime
        .upload_f32(srsi_k_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_srsi_d = runtime
        .upload_f32(srsi_d_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_ift_rsi = runtime
        .upload_f32(ift_rsi_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_qqe_fast = runtime
        .upload_f32(qqe_fast_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_qqe_slow = runtime
        .upload_f32(qqe_slow_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_prb_upper = runtime
        .upload_f32(prb_upper_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_prb_lower = runtime
        .upload_f32(prb_lower_row_major.as_slice())
        .map_err(|e| e.to_string())?;

    let fast_row_lookup: HashMap<u16, i32> = fast_periods
        .iter()
        .enumerate()
        .map(|(idx, &period)| {
            Ok((
                period,
                i32::try_from(idx)
                    .map_err(|_| "KQSR reopt fast row index exceeds i32 range".to_string())?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let slow_row_lookup: HashMap<u16, i32> = slow_periods
        .iter()
        .enumerate()
        .map(|(idx, &period)| {
            Ok((
                period,
                i32::try_from(idx)
                    .map_err(|_| "KQSR reopt slow row index exceeds i32 range".to_string())?,
            ))
        })
        .collect::<Result<_, String>>()?;

    let mut combo_fast_row_idx: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut combo_slow_row_idx: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut finalist_entries: Vec<(String, Vec<u8>, usize)> = Vec::with_capacity(finalists.len());
    for (idx, cand) in finalists.iter().enumerate() {
        let cand_kqsr = cand.kqsr.as_ref().ok_or_else(|| {
            "GPU composite reopt_walk_forward currently requires all finalists to use KQSR"
                .to_string()
        })?;
        if cand.asr.is_some()
            || cand.vcc.is_some()
            || cand.vemr.is_some()
            || cand.rocb.is_some()
            || cand.mkcs.is_some()
            || cand.carm.is_some()
            || cand.kvdf.is_some()
            || cand.cdmc.is_some()
        {
            return Err(
                "GPU composite reopt_walk_forward currently requires all finalists to use KQSR"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        if !serialized_eq(cand_kqsr, first_kqsr)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires uniform KQSR parameters"
                    .to_string(),
            );
        }

        let signals_first_valid =
            (*fast_first_valid_lookup.get(&cand.fast_len).ok_or_else(|| {
                format!(
                    "missing KQSR fast first_valid for fast_len={}",
                    cand.fast_len
                )
            })?)
            .max(*slow_first_valid_lookup.get(&cand.slow_len).ok_or_else(|| {
                format!(
                    "missing KQSR slow first_valid for slow_len={}",
                    cand.slow_len
                )
            })?);
        let start_t = compute_kqsr_start_t(
            fv_close,
            signals_first_valid,
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after KQSR GPU reopt warmup".to_string());
        }
        combo_fast_row_idx.push(*fast_row_lookup.get(&cand.fast_len).ok_or_else(|| {
            format!("missing KQSR fast row index for fast_len={}", cand.fast_len)
        })?);
        combo_slow_row_idx.push(*slow_row_lookup.get(&cand.slow_len).ok_or_else(|| {
            format!("missing KQSR slow row index for slow_len={}", cand.slow_len)
        })?);
        combo_start_t.push(
            i32::try_from(start_t)
                .map_err(|_| "KQSR reopt start_t exceeds i32 range".to_string())?,
        );
        finalist_entries.push((
            serde_json::to_string(cand)
                .unwrap_or_else(|_| format!("kqsr:{}:{}", cand.fast_len, cand.slow_len)),
            serialize_composite_candidate_key(cand)?,
            finalist_start_ts[idx],
        ));
    }

    let signals = KqsrGpuDeviceSignalMatrices {
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
        cols: n,
        j_row_major_ptr: d_j.buffer().as_device_ptr().as_raw() as u64,
        fast_rows,
        fast_stride: n,
        srsi_k_row_major_ptr: d_srsi_k.buffer().as_device_ptr().as_raw() as u64,
        srsi_d_row_major_ptr: d_srsi_d.buffer().as_device_ptr().as_raw() as u64,
        ift_rsi_row_major_ptr: d_ift_rsi.buffer().as_device_ptr().as_raw() as u64,
        qqe_fast_row_major_ptr: d_qqe_fast.buffer().as_device_ptr().as_raw() as u64,
        qqe_slow_row_major_ptr: d_qqe_slow.buffer().as_device_ptr().as_raw() as u64,
        slow_rows,
        slow_stride: n,
        prb_upper_row_major_ptr: d_prb_upper.buffer().as_device_ptr().as_raw() as u64,
        prb_lower_row_major_ptr: d_prb_lower.buffer().as_device_ptr().as_raw() as u64,
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
                    .map_err(|_| "KQSR reopt start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let end_i =
            i32::try_from(end).map_err(|_| "KQSR reopt end_t exceeds i32 range".to_string())?;
        let end_vec = vec![end_i; start_vec.len()];
        let combos = KqsrGpuComboBatch {
            fast_row_index: &combo_fast_row_idx,
            slow_row_index: &combo_slow_row_idx,
            start_t: &start_vec,
            end_t: &end_vec,
        };
        let metrics = eval_kqsr_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &first.strategy,
            first_kqsr,
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
                        "missing KQSR GPU reopt metrics for range {}..{}",
                        start, end
                    )
                })
        },
        |(key, _, _)| key.clone(),
    )
    .ok_or_else(|| "KQSR GPU reopt_walk_forward did not produce any windows".to_string())
    .map(Some)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_carm_gpu_walk_forward_stats_batch(
    signals: &CarmGpuDeviceSignalMatrices<'_>,
    fast_row_index: &[i32],
    otto_row_index: &[i32],
    combo_start_t: &[i32],
    desired_row_major: &[i32],
    strategy: &StrategyConfig,
    carm: &CarmStrategyParams,
    device_id: u32,
    cfg: &WalkForwardConfigOptional,
) -> Result<Vec<WalkForwardStats>, String> {
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let n = signals.cols;
    let inactive_start = n.saturating_sub(2);
    let inactive_start_i = i32::try_from(inactive_start)
        .map_err(|_| "CARM GPU walk_forward inactive start exceeds i32 range".to_string())?;
    let inactive_end_i = i32::try_from(n)
        .map_err(|_| "CARM GPU walk_forward inactive end exceeds i32 range".to_string())?;
    let mut current_t0: Vec<Option<usize>> = combo_start_t
        .iter()
        .map(|&st| Some(usize::try_from(st).unwrap_or(0)))
        .collect();
    let mut sharpes: Vec<Vec<f64>> = vec![Vec::new(); fast_row_index.len()];
    let mut dds: Vec<Vec<f64>> = vec![Vec::new(); fast_row_index.len()];
    let mut pnls: Vec<Vec<f64>> = vec![Vec::new(); fast_row_index.len()];
    let mut trades: Vec<Vec<u32>> = vec![Vec::new(); fast_row_index.len()];

    loop {
        let mut any_active = false;
        let mut active_mask = vec![false; fast_row_index.len()];
        let mut next_t0 = current_t0.clone();
        let mut start_vec = Vec::with_capacity(fast_row_index.len());
        let mut end_vec = Vec::with_capacity(fast_row_index.len());

        for idx in 0..fast_row_index.len() {
            if let Some(t0) = current_t0[idx] {
                let end = t0.saturating_add(window).min(n);
                if t0 + 2 <= n && end > t0 + 1 {
                    any_active = true;
                    active_mask[idx] = true;
                    start_vec.push(i32::try_from(t0).map_err(|_| {
                        "CARM GPU walk_forward start_t exceeds i32 range".to_string()
                    })?);
                    end_vec.push(i32::try_from(end).map_err(|_| {
                        "CARM GPU walk_forward end_t exceeds i32 range".to_string()
                    })?);
                    next_t0[idx] = if end == n {
                        None
                    } else {
                        Some(t0.saturating_add(step))
                    };
                    continue;
                }
            }
            start_vec.push(inactive_start_i);
            end_vec.push(inactive_end_i);
            next_t0[idx] = None;
        }

        if !any_active {
            break;
        }

        let combos = CarmGpuComboBatch {
            fast_row_index,
            otto_row_index,
            start_t: &start_vec,
            end_t: &end_vec,
        };
        let metrics =
            eval_carm_metrics_gpu_kernel_device_rows(device_id, signals, &combos, strategy, carm)?;
        for (idx, metric) in metrics.into_iter().enumerate() {
            if active_mask[idx] {
                let start = usize::try_from(start_vec[idx]).unwrap_or(0);
                let end = usize::try_from(end_vec[idx]).unwrap_or(n);
                let desired = &desired_row_major[idx * n..(idx + 1) * n];
                sharpes[idx].push(metric.sharpe);
                dds[idx].push(metric.max_dd);
                pnls[idx].push(metric.pnl);
                trades[idx].push(count_trade_events_from_desired_window(
                    desired, start, end, strategy,
                ));
            }
        }
        current_t0 = next_t0;
    }

    let mut out = Vec::with_capacity(fast_row_index.len());
    for idx in 0..fast_row_index.len() {
        out.push(summarize_walk_forward_metric_samples(
            std::mem::take(&mut sharpes[idx]),
            std::mem::take(&mut dds[idx]),
            std::mem::take(&mut pnls[idx]),
            std::mem::take(&mut trades[idx]),
        ));
    }
    Ok(out)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(dead_code)]
fn compute_carm_gpu_cscv_slice_stats_for_candidates(
    req: &CompositeValidationRequest,
    signals: &CarmGpuDeviceSignalMatrices<'_>,
    fast_row_index: &[i32],
    otto_row_index: &[i32],
    combo_start_t: &[i32],
    start_ts: &[usize],
    carm: &CarmStrategyParams,
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
            i32::try_from(end).map_err(|_| "CARM GPU CSCV end_t exceeds i32 range".to_string())?;
        let slice_start: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start = usize::try_from(st).unwrap_or(0).max(t0);
                i32::try_from(start)
                    .map_err(|_| "CARM GPU CSCV start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let slice_end = vec![end_i; slice_start.len()];
        let combos = CarmGpuComboBatch {
            fast_row_index,
            otto_row_index,
            start_t: &slice_start,
            end_t: &slice_end,
        };
        let metrics = eval_carm_metrics_gpu_kernel_device_rows(
            device_id,
            signals,
            &combos,
            &req.candidates[0].strategy,
            carm,
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
fn try_prepare_carm_gpu_fixed_candidate_validation(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<GpuCompositeFixedCandidateValidationEval, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    if let Some(reason) = composite_gpu_validation_feature_blocker(req) {
        return Err(reason);
    }
    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    let first_carm = first.carm.as_ref().ok_or_else(|| {
        "GPU composite validation currently supports CARM candidates only".to_string()
    })?;
    if first.asr.is_some()
        || first.vcc.is_some()
        || first.vemr.is_some()
        || first.rocb.is_some()
        || first.mkcs.is_some()
        || first.kqsr.is_some()
        || first.kvdf.is_some()
        || first.cdmc.is_some()
    {
        return Err("GPU composite validation currently supports CARM candidates only".to_string());
    }
    validate_strategy_cfg(&first.strategy)?;
    validate_carm_params(first_carm)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite CARM validation currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    let lr = compute_log_returns_f64(close);
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;

    let mut fast_periods: Vec<u16> = req.candidates.iter().map(|cand| cand.fast_len).collect();
    fast_periods.sort_unstable();
    fast_periods.dedup();
    let fast_rows = fast_periods.len();
    let row_major_len_fast = fast_rows
        .checked_mul(n)
        .ok_or_else(|| "CARM validation fast signal matrix overflow".to_string())?;
    let slow_seed = req
        .candidates
        .iter()
        .map(|cand| cand.slow_len)
        .min()
        .ok_or_else(|| "CARM validation slow period set is empty".to_string())?;
    let mut rocr_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut rocr_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(fast_rows);
    for &fast_len in &fast_periods {
        let periods = resolve_carm_periods(first_carm, fast_len, slow_seed);
        let rocr = compute_carm_rocr_signal(
            candles,
            &req.ma_source,
            &Backend::CpuOnly,
            periods,
            Some(&prepared_cuda_inputs),
        )?;
        rocr_first_valid_lookup.insert(
            fast_len,
            rocr.iter().position(|v| v.is_finite()).unwrap_or(n),
        );
        rocr_row_major.extend(rocr.into_iter().map(|value| value as f32));
    }

    let total = req.candidates.len();
    let mut coppock_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut ao_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut apo_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut otto_hott_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut otto_lott_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let fast_row_lookup: HashMap<u16, i32> = fast_periods
        .iter()
        .enumerate()
        .map(|(idx, &period)| {
            Ok((
                period,
                i32::try_from(idx)
                    .map_err(|_| "CARM fast row index exceeds i32 range".to_string())?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let mut otto_first_valid_by_row: Vec<usize> = Vec::new();
    let mut otto_row_lookup: HashMap<(u16, u16, u16), i32> = HashMap::new();
    let mut combo_fast_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_otto_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total);
    let mut start_ts: Vec<usize> = Vec::with_capacity(total);
    let mut desired_row_major: Vec<i32> = Vec::with_capacity(total * n);
    let mut exact_metrics: Vec<Metrics> = Vec::with_capacity(total);
    let mut exact_series: Vec<CandidateSeries<'_>> = Vec::with_capacity(total);
    for cand in &req.candidates {
        if cand.carm.is_none()
            || cand.asr.is_some()
            || cand.vcc.is_some()
            || cand.vemr.is_some()
            || cand.rocb.is_some()
            || cand.mkcs.is_some()
            || cand.kqsr.is_some()
            || cand.kvdf.is_some()
            || cand.cdmc.is_some()
        {
            return Err(
                "GPU composite validation currently requires all candidates to use CARM"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite validation currently requires a uniform StrategyConfig".to_string(),
            );
        }
        if !serialized_eq(cand.carm.as_ref().unwrap(), first_carm)? {
            return Err(
                "GPU composite validation currently requires uniform CARM parameters".to_string(),
            );
        }

        let periods = resolve_carm_periods(first_carm, cand.fast_len, cand.slow_len);
        let otto_key = (
            periods.otto,
            periods.coppock_short.max(2),
            periods.ao_slow.max(periods.otto),
        );
        let rocr_for_desired = compute_carm_rocr_signal(
            candles,
            &req.ma_source,
            &Backend::CpuOnly,
            periods,
            Some(&prepared_cuda_inputs),
        )?;
        let mixed = compute_carm_mixed_signals(
            candles,
            &req.ma_source,
            &Backend::CpuOnly,
            periods,
            Some(&prepared_cuda_inputs),
        )?;
        let otto_for_desired = compute_carm_otto_signals(
            candles,
            &req.ma_source,
            &Backend::CpuOnly,
            periods,
            first_carm,
            Some(&prepared_cuda_inputs),
        )?;
        let otto_row = if let Some(&row) = otto_row_lookup.get(&otto_key) {
            row
        } else {
            let row = i32::try_from(otto_first_valid_by_row.len())
                .map_err(|_| "CARM OTTO row index exceeds i32 range".to_string())?;
            otto_first_valid_by_row.push(otto_for_desired.first_valid);
            otto_hott_row_major.extend(otto_for_desired.hott.iter().map(|value| *value as f32));
            otto_lott_row_major.extend(otto_for_desired.lott.iter().map(|value| *value as f32));
            otto_row_lookup.insert(otto_key, row);
            row
        };
        let otto_first_valid = otto_first_valid_by_row
            .get(usize::try_from(otto_row).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or(n);
        let signals_first_valid = mixed
            .first_valid
            .max(*rocr_first_valid_lookup.get(&cand.fast_len).ok_or_else(|| {
                format!(
                    "missing CARM ROCR first_valid for fast_len={}",
                    cand.fast_len
                )
            })?)
            .max(otto_first_valid);
        let start_t = compute_carm_start_t(
            fv_close,
            signals_first_valid,
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after CARM GPU warmup".to_string());
        }

        let desired = build_carm_desired_positions(
            close,
            &CarmSignals {
                coppock: mixed.coppock.clone(),
                ao: mixed.ao.clone(),
                rocr: rocr_for_desired,
                apo: mixed.apo.clone(),
                otto_hott: otto_for_desired.hott.clone(),
                otto_lott: otto_for_desired.lott.clone(),
                first_valid: signals_first_valid,
            },
            start_t,
            &cand.strategy,
            first_carm,
        );
        exact_metrics.push(eval_asr_metrics_from_desired(
            candles,
            &lr,
            &desired,
            start_t,
            &cand.strategy,
        ));
        desired_row_major.extend(desired.iter().copied());
        exact_series.push(CandidateSeries::Carm(CarmCandidateSeries {
            candles,
            n,
            lr: &lr,
            desired,
            start_t,
            cfg: cand.strategy.clone(),
        }));

        coppock_row_major.extend(mixed.coppock.into_iter().map(|value| value as f32));
        ao_row_major.extend(mixed.ao.into_iter().map(|value| value as f32));
        apo_row_major.extend(mixed.apo.into_iter().map(|value| value as f32));
        combo_fast_row_idx.push(*fast_row_lookup.get(&cand.fast_len).ok_or_else(|| {
            format!("missing CARM fast row index for fast_len={}", cand.fast_len)
        })?);
        combo_otto_row_idx.push(otto_row);
        combo_start_t.push(
            i32::try_from(start_t).map_err(|_| "CARM start_t exceeds i32 range".to_string())?,
        );
        start_ts.push(start_t);
    }

    let runtime = prepared_cuda_inputs.runtime();
    let d_coppock = runtime
        .upload_f32(coppock_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_ao = runtime
        .upload_f32(ao_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_rocr = runtime
        .upload_f32(rocr_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_apo = runtime
        .upload_f32(apo_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_otto_hott = runtime
        .upload_f32(otto_hott_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_otto_lott = runtime
        .upload_f32(otto_lott_row_major.as_slice())
        .map_err(|e| e.to_string())?;

    let signals = CarmGpuDeviceSignalMatrices {
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
        cols: n,
        mixed_rows: total,
        mixed_stride: n,
        coppock_row_major_ptr: d_coppock.buffer().as_device_ptr().as_raw() as u64,
        ao_row_major_ptr: d_ao.buffer().as_device_ptr().as_raw() as u64,
        rocr_row_major_ptr: d_rocr.buffer().as_device_ptr().as_raw() as u64,
        fast_rows,
        fast_stride: n,
        apo_row_major_ptr: d_apo.buffer().as_device_ptr().as_raw() as u64,
        otto_hott_row_major_ptr: d_otto_hott.buffer().as_device_ptr().as_raw() as u64,
        otto_lott_row_major_ptr: d_otto_lott.buffer().as_device_ptr().as_raw() as u64,
        otto_rows: otto_first_valid_by_row.len(),
        otto_stride: n,
    };
    let combo_end_t =
        vec![i32::try_from(n).map_err(|_| "CARM end_t exceeds i32 range".to_string())?; total];
    let combos = CarmGpuComboBatch {
        fast_row_index: &combo_fast_row_idx,
        otto_row_index: &combo_otto_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };
    let gpu_metrics = eval_carm_metrics_gpu_kernel_device_rows(
        device_id,
        &signals,
        &combos,
        &first.strategy,
        first_carm,
    )?;
    if gpu_metrics.len() != total {
        return Err("CARM GPU validation output length mismatch".to_string());
    }
    let metrics = exact_metrics;
    let walk_forward_stats = if let Some(walk_cfg) = req.walk_forward.as_ref() {
        Some(compute_carm_gpu_walk_forward_stats_batch(
            &signals,
            &combo_fast_row_idx,
            &combo_otto_row_idx,
            &combo_start_t,
            &desired_row_major,
            &first.strategy,
            first_carm,
            device_id,
            walk_cfg,
        )?)
    } else {
        None
    };
    let candidate_keys: Vec<Vec<u8>> = req
        .candidates
        .iter()
        .map(serialize_composite_candidate_key)
        .collect::<Result<_, _>>()?;
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        exact_slice_stats_for_candidate_series(&candidate_keys, &exact_series, cscv_cfg)
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        let slice_stats = exact_slice_stats_for_candidate_series(
            &candidate_keys,
            &exact_series,
            &CscvPboConfig {
                slices: spa_cfg.slices.max(4),
                max_combinations: usize::MAX,
                threshold: 0.0,
            },
        );
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
        walk_forward_stats,
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
fn compute_carm_gpu_reopt_walk_forward_stats_for_candidates(
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
    let first_carm = first
        .carm
        .as_ref()
        .ok_or_else(|| "CARM GPU reopt_walk_forward requires CARM candidate params".to_string())?;
    validate_strategy_cfg(&first.strategy)?;
    validate_carm_params(first_carm)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite CARM reopt_walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if n > i32::MAX as usize {
        return Err("CARM GPU reopt_walk_forward length exceeds i32 range".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;

    let mut fast_periods: Vec<u16> = finalists.iter().map(|cand| cand.fast_len).collect();
    fast_periods.sort_unstable();
    fast_periods.dedup();
    let fast_rows = fast_periods.len();
    let row_major_len_fast = fast_rows
        .checked_mul(n)
        .ok_or_else(|| "CARM reopt fast signal matrix overflow".to_string())?;
    let slow_seed = finalists
        .iter()
        .map(|cand| cand.slow_len)
        .min()
        .ok_or_else(|| "CARM reopt slow period set is empty".to_string())?;

    let mut rocr_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut rocr_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(fast_rows);
    for &fast_len in &fast_periods {
        let periods = resolve_carm_periods(first_carm, fast_len, slow_seed);
        let rocr = compute_carm_rocr_signal(
            candles,
            &req.ma_source,
            &Backend::CpuOnly,
            periods,
            Some(&prepared_cuda_inputs),
        )?;
        rocr_first_valid_lookup.insert(
            fast_len,
            rocr.iter().position(|v| v.is_finite()).unwrap_or(n),
        );
        rocr_row_major.extend(rocr.into_iter().map(|value| value as f32));
    }

    let total = finalists.len();
    let mut coppock_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut ao_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut apo_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut otto_hott_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut otto_lott_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let fast_row_lookup: HashMap<u16, i32> = fast_periods
        .iter()
        .enumerate()
        .map(|(idx, &period)| {
            Ok((
                period,
                i32::try_from(idx)
                    .map_err(|_| "CARM reopt fast row index exceeds i32 range".to_string())?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let mut otto_first_valid_by_row: Vec<usize> = Vec::new();
    let mut otto_row_lookup: HashMap<(u16, u16, u16), i32> = HashMap::new();
    let mut combo_fast_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_otto_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total);
    let mut finalist_entries: Vec<(String, Vec<u8>, usize)> = Vec::with_capacity(total);
    for (idx, cand) in finalists.iter().enumerate() {
        let cand_carm = cand.carm.as_ref().ok_or_else(|| {
            "GPU composite reopt_walk_forward currently requires all finalists to use CARM"
                .to_string()
        })?;
        if cand.asr.is_some()
            || cand.vcc.is_some()
            || cand.vemr.is_some()
            || cand.rocb.is_some()
            || cand.mkcs.is_some()
            || cand.kqsr.is_some()
            || cand.kvdf.is_some()
            || cand.cdmc.is_some()
        {
            return Err(
                "GPU composite reopt_walk_forward currently requires all finalists to use CARM"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        if !serialized_eq(cand_carm, first_carm)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires uniform CARM parameters"
                    .to_string(),
            );
        }

        let periods = resolve_carm_periods(first_carm, cand.fast_len, cand.slow_len);
        let otto_key = (
            periods.otto,
            periods.coppock_short.max(2),
            periods.ao_slow.max(periods.otto),
        );
        let mixed = compute_carm_mixed_signals(
            candles,
            &req.ma_source,
            &Backend::CpuOnly,
            periods,
            Some(&prepared_cuda_inputs),
        )?;
        let otto_row = if let Some(&row) = otto_row_lookup.get(&otto_key) {
            row
        } else {
            let otto = compute_carm_otto_signals(
                candles,
                &req.ma_source,
                &Backend::CpuOnly,
                periods,
                first_carm,
                Some(&prepared_cuda_inputs),
            )?;
            let row = i32::try_from(otto_first_valid_by_row.len())
                .map_err(|_| "CARM GPU reopt OTTO row index exceeds i32 range".to_string())?;
            otto_first_valid_by_row.push(otto.first_valid);
            otto_hott_row_major.extend(otto.hott.into_iter().map(|value| value as f32));
            otto_lott_row_major.extend(otto.lott.into_iter().map(|value| value as f32));
            otto_row_lookup.insert(otto_key, row);
            row
        };
        let otto_first_valid = otto_first_valid_by_row
            .get(usize::try_from(otto_row).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or(n);
        let signals_first_valid = mixed
            .first_valid
            .max(*rocr_first_valid_lookup.get(&cand.fast_len).ok_or_else(|| {
                format!(
                    "missing CARM ROCR first_valid for fast_len={}",
                    cand.fast_len
                )
            })?)
            .max(otto_first_valid);
        let start_t = compute_carm_start_t(
            fv_close,
            signals_first_valid,
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after CARM GPU reopt warmup".to_string());
        }

        coppock_row_major.extend(mixed.coppock.into_iter().map(|value| value as f32));
        ao_row_major.extend(mixed.ao.into_iter().map(|value| value as f32));
        apo_row_major.extend(mixed.apo.into_iter().map(|value| value as f32));
        combo_fast_row_idx.push(*fast_row_lookup.get(&cand.fast_len).ok_or_else(|| {
            format!("missing CARM fast row index for fast_len={}", cand.fast_len)
        })?);
        combo_otto_row_idx.push(otto_row);
        combo_start_t.push(
            i32::try_from(start_t)
                .map_err(|_| "CARM reopt start_t exceeds i32 range".to_string())?,
        );
        finalist_entries.push((
            serde_json::to_string(cand)
                .unwrap_or_else(|_| format!("carm:{}:{}", cand.fast_len, cand.slow_len)),
            serialize_composite_candidate_key(cand)?,
            finalist_start_ts[idx],
        ));
    }

    let runtime = prepared_cuda_inputs.runtime();
    let d_coppock = runtime
        .upload_f32(coppock_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_ao = runtime
        .upload_f32(ao_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_rocr = runtime
        .upload_f32(rocr_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_apo = runtime
        .upload_f32(apo_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_otto_hott = runtime
        .upload_f32(otto_hott_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_otto_lott = runtime
        .upload_f32(otto_lott_row_major.as_slice())
        .map_err(|e| e.to_string())?;

    let signals = CarmGpuDeviceSignalMatrices {
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
        cols: n,
        mixed_rows: total,
        mixed_stride: n,
        coppock_row_major_ptr: d_coppock.buffer().as_device_ptr().as_raw() as u64,
        ao_row_major_ptr: d_ao.buffer().as_device_ptr().as_raw() as u64,
        rocr_row_major_ptr: d_rocr.buffer().as_device_ptr().as_raw() as u64,
        fast_rows,
        fast_stride: n,
        apo_row_major_ptr: d_apo.buffer().as_device_ptr().as_raw() as u64,
        otto_hott_row_major_ptr: d_otto_hott.buffer().as_device_ptr().as_raw() as u64,
        otto_lott_row_major_ptr: d_otto_lott.buffer().as_device_ptr().as_raw() as u64,
        otto_rows: otto_first_valid_by_row.len(),
        otto_stride: n,
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
                    .map_err(|_| "CARM reopt start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let end_i =
            i32::try_from(end).map_err(|_| "CARM reopt end_t exceeds i32 range".to_string())?;
        let end_vec = vec![end_i; start_vec.len()];
        let combos = CarmGpuComboBatch {
            fast_row_index: &combo_fast_row_idx,
            otto_row_index: &combo_otto_row_idx,
            start_t: &start_vec,
            end_t: &end_vec,
        };
        let metrics = eval_carm_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &first.strategy,
            first_carm,
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
                        "missing CARM GPU reopt metrics for range {}..{}",
                        start, end
                    )
                })
        },
        |(key, _, _)| key.clone(),
    )
    .ok_or_else(|| "CARM GPU reopt_walk_forward did not produce any windows".to_string())
    .map(Some)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_kvdf_gpu_walk_forward_stats_batch(
    signals: &KvdfGpuDeviceSignalMatrices<'_>,
    fast_row_index: &[i32],
    slow_row_index: &[i32],
    dti_row_index: &[i32],
    combo_start_t: &[i32],
    strategy: &StrategyConfig,
    kvdf: &KvdfStrategyParams,
    device_id: u32,
    cfg: &WalkForwardConfigOptional,
) -> Result<Vec<WalkForwardStats>, String> {
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let n = signals.cols;
    let inactive_start = n.saturating_sub(2);
    let inactive_start_i = i32::try_from(inactive_start)
        .map_err(|_| "KVDF GPU walk_forward inactive start exceeds i32 range".to_string())?;
    let inactive_end_i = i32::try_from(n)
        .map_err(|_| "KVDF GPU walk_forward inactive end exceeds i32 range".to_string())?;
    let mut current_t0: Vec<Option<usize>> = combo_start_t
        .iter()
        .map(|&st| Some(usize::try_from(st).unwrap_or(0)))
        .collect();
    let mut sharpes: Vec<Vec<f64>> = vec![Vec::new(); fast_row_index.len()];
    let mut dds: Vec<Vec<f64>> = vec![Vec::new(); fast_row_index.len()];
    let mut pnls: Vec<Vec<f64>> = vec![Vec::new(); fast_row_index.len()];
    let mut trades: Vec<Vec<u32>> = vec![Vec::new(); fast_row_index.len()];

    loop {
        let mut any_active = false;
        let mut active_mask = vec![false; fast_row_index.len()];
        let mut next_t0 = current_t0.clone();
        let mut start_vec = Vec::with_capacity(fast_row_index.len());
        let mut end_vec = Vec::with_capacity(fast_row_index.len());

        for idx in 0..fast_row_index.len() {
            if let Some(t0) = current_t0[idx] {
                let end = t0.saturating_add(window).min(n);
                if t0 + 2 <= n && end > t0 + 1 {
                    any_active = true;
                    active_mask[idx] = true;
                    start_vec.push(i32::try_from(t0).map_err(|_| {
                        "KVDF GPU walk_forward start_t exceeds i32 range".to_string()
                    })?);
                    end_vec.push(i32::try_from(end).map_err(|_| {
                        "KVDF GPU walk_forward end_t exceeds i32 range".to_string()
                    })?);
                    next_t0[idx] = if end == n {
                        None
                    } else {
                        Some(t0.saturating_add(step))
                    };
                    continue;
                }
            }
            start_vec.push(inactive_start_i);
            end_vec.push(inactive_end_i);
            next_t0[idx] = None;
        }

        if !any_active {
            break;
        }

        let combos = KvdfGpuComboBatch {
            fast_row_index,
            slow_row_index,
            dti_row_index,
            start_t: &start_vec,
            end_t: &end_vec,
        };
        let metrics =
            eval_kvdf_metrics_gpu_kernel_device_rows(device_id, signals, &combos, strategy, kvdf)?;
        for (idx, metric) in metrics.into_iter().enumerate() {
            if active_mask[idx] {
                sharpes[idx].push(metric.sharpe);
                dds[idx].push(metric.max_dd);
                pnls[idx].push(metric.pnl);
                trades[idx].push(metric.trades);
            }
        }
        current_t0 = next_t0;
    }

    let mut out = Vec::with_capacity(fast_row_index.len());
    for idx in 0..fast_row_index.len() {
        out.push(summarize_walk_forward_metric_samples(
            std::mem::take(&mut sharpes[idx]),
            std::mem::take(&mut dds[idx]),
            std::mem::take(&mut pnls[idx]),
            std::mem::take(&mut trades[idx]),
        ));
    }
    Ok(out)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_kvdf_gpu_cscv_slice_stats_for_candidates(
    req: &CompositeValidationRequest,
    signals: &KvdfGpuDeviceSignalMatrices<'_>,
    fast_row_index: &[i32],
    slow_row_index: &[i32],
    dti_row_index: &[i32],
    combo_start_t: &[i32],
    start_ts: &[usize],
    kvdf: &KvdfStrategyParams,
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
            i32::try_from(end).map_err(|_| "KVDF GPU CSCV end_t exceeds i32 range".to_string())?;
        let slice_start: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start = usize::try_from(st).unwrap_or(0).max(t0);
                i32::try_from(start)
                    .map_err(|_| "KVDF GPU CSCV start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let slice_end = vec![end_i; slice_start.len()];
        let combos = KvdfGpuComboBatch {
            fast_row_index,
            slow_row_index,
            dti_row_index,
            start_t: &slice_start,
            end_t: &slice_end,
        };
        let metrics = eval_kvdf_metrics_gpu_kernel_device_rows(
            device_id,
            signals,
            &combos,
            &req.candidates[0].strategy,
            kvdf,
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
fn compute_kvdf_gpu_reopt_walk_forward_stats_for_candidates(
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
    let first_kvdf = first
        .kvdf
        .as_ref()
        .ok_or_else(|| "KVDF GPU reopt_walk_forward requires KVDF candidate params".to_string())?;
    validate_strategy_cfg(&first.strategy)?;
    validate_kvdf_params(first_kvdf)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite KVDF reopt_walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if n > i32::MAX as usize {
        return Err("KVDF GPU reopt_walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let prepared_cuda_inputs = prepared_cuda_inputs_for_source_ohlcv_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )?;
    let runtime = prepared_cuda_inputs.runtime();

    let mut keltner_upper_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut keltner_middle_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut keltner_lower_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut vwap_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut dti_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut dti_first_valid_by_row: Vec<usize> = Vec::new();
    let mut dti_row_lookup: HashMap<(u16, u16, u16), i32> = HashMap::new();
    let mut devstop_long_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut devstop_short_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut fvg_upper_ts_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut fvg_lower_ts_row_major: Vec<f32> = Vec::with_capacity(finalists.len() * n);
    let mut combo_fast_row_idx: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut combo_slow_row_idx: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut combo_dti_row_idx: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(finalists.len());
    let mut finalist_entries: Vec<(String, Vec<u8>, usize)> = Vec::with_capacity(finalists.len());

    for (idx, cand) in finalists.iter().enumerate() {
        let kvdf = cand.kvdf.as_ref().ok_or_else(|| {
            "GPU composite reopt_walk_forward currently requires all finalists to use KVDF"
                .to_string()
        })?;
        if cand.asr.is_some()
            || cand.vcc.is_some()
            || cand.vemr.is_some()
            || cand.rocb.is_some()
            || cand.mkcs.is_some()
            || cand.kqsr.is_some()
            || cand.carm.is_some()
            || cand.cdmc.is_some()
        {
            return Err(
                "GPU composite reopt_walk_forward currently requires all finalists to use KVDF"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        if !serialized_eq(kvdf, first_kvdf)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires uniform KVDF parameters"
                    .to_string(),
            );
        }

        let periods = resolve_kvdf_periods(first_kvdf, cand.fast_len, cand.slow_len);
        let slow = compute_kvdf_slow_signals_f32(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            first_kvdf,
            Some(&prepared_cuda_inputs),
        )?;
        let shared = compute_kvdf_shared_signals_f32(
            candles,
            &req.ma_source,
            &req.backend,
            first_kvdf,
            Some(&prepared_cuda_inputs),
        )?;
        let fast = compute_kvdf_fast_signals_f32(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            Some(&prepared_cuda_inputs),
        )?;
        let dti_key = (periods.dti_r, periods.dti_s, periods.dti_u);
        let dti_row = if let Some(&row) = dti_row_lookup.get(&dti_key) {
            row
        } else {
            let dti = compute_kvdf_dti_signal_f32(
                candles,
                &req.ma_source,
                &req.backend,
                periods,
                Some(&prepared_cuda_inputs),
            )?;
            let row = i32::try_from(dti_first_valid_by_row.len())
                .map_err(|_| "KVDF GPU reopt DTI row index exceeds i32 range".to_string())?;
            dti_first_valid_by_row.push(dti.iter().position(|v| v.is_finite()).unwrap_or(n));
            dti_row_major.extend_from_slice(dti.as_slice());
            dti_row_lookup.insert(dti_key, row);
            row
        };
        let dti_first_valid = dti_first_valid_by_row
            .get(usize::try_from(dti_row).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or(n);
        let start_t = compute_kvdf_start_t(
            first_valid_close,
            slow.first_valid
                .max(shared.first_valid)
                .max(fast.first_valid)
                .max(dti_first_valid),
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after KVDF GPU reopt warmup".to_string());
        }

        keltner_upper_row_major.extend_from_slice(slow.keltner_upper.as_slice());
        keltner_middle_row_major.extend_from_slice(slow.keltner_middle.as_slice());
        keltner_lower_row_major.extend_from_slice(slow.keltner_lower.as_slice());
        vwap_row_major.extend_from_slice(shared.vwap.as_slice());
        devstop_long_row_major.extend_from_slice(slow.devstop_long.as_slice());
        devstop_short_row_major.extend_from_slice(slow.devstop_short.as_slice());
        fvg_upper_ts_row_major.extend_from_slice(fast.fvg_upper_ts.as_slice());
        fvg_lower_ts_row_major.extend_from_slice(fast.fvg_lower_ts.as_slice());
        combo_fast_row_idx.push(
            i32::try_from(idx)
                .map_err(|_| "KVDF GPU reopt fast row index exceeds i32 range".to_string())?,
        );
        combo_slow_row_idx.push(
            i32::try_from(idx)
                .map_err(|_| "KVDF GPU reopt slow row index exceeds i32 range".to_string())?,
        );
        combo_dti_row_idx.push(dti_row);
        combo_start_t.push(
            i32::try_from(start_t)
                .map_err(|_| "KVDF GPU reopt start_t exceeds i32 range".to_string())?,
        );

        let key = serde_json::to_string(cand)
            .unwrap_or_else(|_| format!("kvdf:{}:{}", cand.fast_len, cand.slow_len));
        finalist_entries.push((
            key,
            serialize_composite_candidate_key(cand)?,
            finalist_start_ts[idx],
        ));
    }

    let d_keltner_upper = runtime
        .upload_f32(keltner_upper_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_keltner_middle = runtime
        .upload_f32(keltner_middle_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_keltner_lower = runtime
        .upload_f32(keltner_lower_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_vwap = runtime
        .upload_f32(vwap_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_dti = runtime
        .upload_f32(dti_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_devstop_long = runtime
        .upload_f32(devstop_long_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_devstop_short = runtime
        .upload_f32(devstop_short_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_fvg_upper_ts = runtime
        .upload_f32(fvg_upper_ts_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_fvg_lower_ts = runtime
        .upload_f32(fvg_lower_ts_row_major.as_slice())
        .map_err(|e| e.to_string())?;

    let signals = KvdfGpuDeviceSignalMatrices {
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
        cols: n,
        dti_rows: dti_first_valid_by_row.len(),
        dti_stride: n,
        keltner_upper_row_major_ptr: d_keltner_upper.buffer().as_device_ptr().as_raw() as u64,
        keltner_middle_row_major_ptr: d_keltner_middle.buffer().as_device_ptr().as_raw() as u64,
        keltner_lower_row_major_ptr: d_keltner_lower.buffer().as_device_ptr().as_raw() as u64,
        vwap_row_major_ptr: d_vwap.buffer().as_device_ptr().as_raw() as u64,
        dti_row_major_ptr: d_dti.buffer().as_device_ptr().as_raw() as u64,
        slow_rows: finalists.len(),
        slow_stride: n,
        devstop_long_row_major_ptr: d_devstop_long.buffer().as_device_ptr().as_raw() as u64,
        devstop_short_row_major_ptr: d_devstop_short.buffer().as_device_ptr().as_raw() as u64,
        fvg_upper_ts_row_major_ptr: d_fvg_upper_ts.buffer().as_device_ptr().as_raw() as u64,
        fvg_lower_ts_row_major_ptr: d_fvg_lower_ts.buffer().as_device_ptr().as_raw() as u64,
        fast_rows: finalists.len(),
        fast_stride: n,
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
                    .map_err(|_| "KVDF GPU reopt start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let end_i =
            i32::try_from(end).map_err(|_| "KVDF GPU reopt end_t exceeds i32 range".to_string())?;
        let end_vec = vec![end_i; start_vec.len()];
        let combos = KvdfGpuComboBatch {
            fast_row_index: &combo_fast_row_idx,
            slow_row_index: &combo_slow_row_idx,
            dti_row_index: &combo_dti_row_idx,
            start_t: &start_vec,
            end_t: &end_vec,
        };
        let metrics = eval_kvdf_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &first.strategy,
            first_kvdf,
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
                        "missing KVDF GPU reopt metrics for range {}..{}",
                        start, end
                    )
                })
        },
        |(key, _, _)| key.clone(),
    )
    .ok_or_else(|| "KVDF GPU reopt_walk_forward did not produce any windows".to_string())
    .map(Some)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn try_prepare_kvdf_gpu_fixed_candidate_validation(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<GpuCompositeFixedCandidateValidationEval, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    if let Some(reason) = composite_gpu_validation_feature_blocker(req) {
        return Err(reason);
    }
    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    let first_kvdf = first.kvdf.as_ref().ok_or_else(|| {
        "GPU composite validation currently supports KVDF candidates only".to_string()
    })?;
    if first.asr.is_some()
        || first.vcc.is_some()
        || first.vemr.is_some()
        || first.rocb.is_some()
        || first.mkcs.is_some()
        || first.kqsr.is_some()
        || first.carm.is_some()
        || first.cdmc.is_some()
    {
        return Err("GPU composite validation currently supports KVDF candidates only".to_string());
    }
    validate_strategy_cfg(&first.strategy)?;
    validate_kvdf_params(first_kvdf)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite KVDF validation currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let prepared_cuda_inputs = prepared_cuda_inputs_for_source_ohlcv_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )?;

    let shared = compute_kvdf_shared_signals_f32(
        candles,
        &req.ma_source,
        &req.backend,
        first_kvdf,
        Some(&prepared_cuda_inputs),
    )?;
    let shared_vwap_f32 = shared.vwap.clone();

    let mut fast_periods: Vec<u16> = req.candidates.iter().map(|cand| cand.fast_len).collect();
    let mut slow_periods: Vec<u16> = req.candidates.iter().map(|cand| cand.slow_len).collect();
    fast_periods.sort_unstable();
    fast_periods.dedup();
    slow_periods.sort_unstable();
    slow_periods.dedup();
    let fast_rows = fast_periods.len();
    let slow_rows = slow_periods.len();
    let row_major_len_fast = fast_rows
        .checked_mul(n)
        .ok_or_else(|| "KVDF validation fast signal matrix overflow".to_string())?;
    let row_major_len_slow = slow_rows
        .checked_mul(n)
        .ok_or_else(|| "KVDF validation slow signal matrix overflow".to_string())?;
    let slow_seed = *slow_periods
        .first()
        .ok_or_else(|| "KVDF validation slow period set is empty".to_string())?;
    let fast_seed = *fast_periods
        .first()
        .ok_or_else(|| "KVDF validation fast period set is empty".to_string())?;

    let mut fvg_upper_ts_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut fvg_lower_ts_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut fast_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(fast_rows);
    for &fast_len in &fast_periods {
        let periods = resolve_kvdf_periods(first_kvdf, fast_len, slow_seed);
        let fast = compute_kvdf_fast_signals_f32(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            Some(&prepared_cuda_inputs),
        )?;
        fast_first_valid_lookup.insert(fast_len, fast.first_valid);
        fvg_upper_ts_row_major.extend_from_slice(fast.fvg_upper_ts.as_slice());
        fvg_lower_ts_row_major.extend_from_slice(fast.fvg_lower_ts.as_slice());
    }

    let mut keltner_upper_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut keltner_middle_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut keltner_lower_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut devstop_long_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut devstop_short_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut slow_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(slow_rows);
    for &slow_len in &slow_periods {
        let periods = resolve_kvdf_periods(first_kvdf, fast_seed, slow_len);
        let slow = compute_kvdf_slow_signals_f32(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            first_kvdf,
            Some(&prepared_cuda_inputs),
        )?;
        slow_first_valid_lookup.insert(slow_len, slow.first_valid);
        keltner_upper_row_major.extend_from_slice(slow.keltner_upper.as_slice());
        keltner_middle_row_major.extend_from_slice(slow.keltner_middle.as_slice());
        keltner_lower_row_major.extend_from_slice(slow.keltner_lower.as_slice());
        devstop_long_row_major.extend_from_slice(slow.devstop_long.as_slice());
        devstop_short_row_major.extend_from_slice(slow.devstop_short.as_slice());
    }

    let fast_row_lookup: HashMap<u16, i32> = fast_periods
        .iter()
        .enumerate()
        .map(|(idx, &period)| {
            Ok((
                period,
                i32::try_from(idx)
                    .map_err(|_| "KVDF fast row index exceeds i32 range".to_string())?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let slow_row_lookup: HashMap<u16, i32> = slow_periods
        .iter()
        .enumerate()
        .map(|(idx, &period)| {
            Ok((
                period,
                i32::try_from(idx)
                    .map_err(|_| "KVDF slow row index exceeds i32 range".to_string())?,
            ))
        })
        .collect::<Result<_, String>>()?;

    let total = req.candidates.len();
    let mut dti_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut dti_first_valid_by_row: Vec<usize> = Vec::new();
    let mut dti_row_lookup: HashMap<(u16, u16, u16), i32> = HashMap::new();
    let mut combo_fast_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_slow_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_dti_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total);
    let mut start_ts: Vec<usize> = Vec::with_capacity(total);
    for cand in &req.candidates {
        if cand.kvdf.is_none()
            || cand.asr.is_some()
            || cand.vcc.is_some()
            || cand.vemr.is_some()
            || cand.rocb.is_some()
            || cand.mkcs.is_some()
            || cand.kqsr.is_some()
            || cand.carm.is_some()
            || cand.cdmc.is_some()
        {
            return Err(
                "GPU composite validation currently requires all candidates to use KVDF"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite validation currently requires a uniform StrategyConfig".to_string(),
            );
        }
        if !serialized_eq(cand.kvdf.as_ref().unwrap(), first_kvdf)? {
            return Err(
                "GPU composite validation currently requires uniform KVDF parameters".to_string(),
            );
        }

        let periods = resolve_kvdf_periods(first_kvdf, cand.fast_len, cand.slow_len);
        let dti_key = (periods.dti_r, periods.dti_s, periods.dti_u);
        let dti_row = if let Some(&row) = dti_row_lookup.get(&dti_key) {
            row
        } else {
            let dti = compute_kvdf_dti_signal_f32(
                candles,
                &req.ma_source,
                &req.backend,
                periods,
                Some(&prepared_cuda_inputs),
            )?;
            let row = i32::try_from(dti_first_valid_by_row.len())
                .map_err(|_| "KVDF DTI row index exceeds i32 range".to_string())?;
            dti_first_valid_by_row.push(dti.iter().position(|v| v.is_finite()).unwrap_or(n));
            dti_row_major.extend_from_slice(dti.as_slice());
            dti_row_lookup.insert(dti_key, row);
            row
        };
        let dti_first_valid = dti_first_valid_by_row
            .get(usize::try_from(dti_row).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or(n);
        let signals_first_valid = shared
            .first_valid
            .max(*fast_first_valid_lookup.get(&cand.fast_len).ok_or_else(|| {
                format!(
                    "missing KVDF fast first_valid for fast_len={}",
                    cand.fast_len
                )
            })?)
            .max(*slow_first_valid_lookup.get(&cand.slow_len).ok_or_else(|| {
                format!(
                    "missing KVDF slow first_valid for slow_len={}",
                    cand.slow_len
                )
            })?)
            .max(dti_first_valid);
        let start_t = compute_kvdf_start_t(
            fv_close,
            signals_first_valid,
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after KVDF GPU warmup".to_string());
        }

        combo_fast_row_idx.push(*fast_row_lookup.get(&cand.fast_len).ok_or_else(|| {
            format!("missing KVDF fast row index for fast_len={}", cand.fast_len)
        })?);
        combo_slow_row_idx.push(*slow_row_lookup.get(&cand.slow_len).ok_or_else(|| {
            format!("missing KVDF slow row index for slow_len={}", cand.slow_len)
        })?);
        combo_dti_row_idx.push(dti_row);
        combo_start_t.push(
            i32::try_from(start_t).map_err(|_| "KVDF start_t exceeds i32 range".to_string())?,
        );
        start_ts.push(start_t);
    }

    let runtime = prepared_cuda_inputs.runtime();
    let d_keltner_upper = runtime
        .upload_f32(keltner_upper_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_keltner_middle = runtime
        .upload_f32(keltner_middle_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_keltner_lower = runtime
        .upload_f32(keltner_lower_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_vwap = runtime
        .upload_f32(shared_vwap_f32.as_slice())
        .map_err(|e| e.to_string())?;
    let d_dti = runtime
        .upload_f32(dti_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_devstop_long = runtime
        .upload_f32(devstop_long_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_devstop_short = runtime
        .upload_f32(devstop_short_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_fvg_upper_ts = runtime
        .upload_f32(fvg_upper_ts_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_fvg_lower_ts = runtime
        .upload_f32(fvg_lower_ts_row_major.as_slice())
        .map_err(|e| e.to_string())?;

    let signals = KvdfGpuDeviceSignalMatrices {
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
        cols: n,
        dti_rows: dti_first_valid_by_row.len(),
        dti_stride: n,
        keltner_upper_row_major_ptr: d_keltner_upper.buffer().as_device_ptr().as_raw() as u64,
        keltner_middle_row_major_ptr: d_keltner_middle.buffer().as_device_ptr().as_raw() as u64,
        keltner_lower_row_major_ptr: d_keltner_lower.buffer().as_device_ptr().as_raw() as u64,
        vwap_row_major_ptr: d_vwap.buffer().as_device_ptr().as_raw() as u64,
        dti_row_major_ptr: d_dti.buffer().as_device_ptr().as_raw() as u64,
        slow_rows,
        slow_stride: n,
        devstop_long_row_major_ptr: d_devstop_long.buffer().as_device_ptr().as_raw() as u64,
        devstop_short_row_major_ptr: d_devstop_short.buffer().as_device_ptr().as_raw() as u64,
        fvg_upper_ts_row_major_ptr: d_fvg_upper_ts.buffer().as_device_ptr().as_raw() as u64,
        fvg_lower_ts_row_major_ptr: d_fvg_lower_ts.buffer().as_device_ptr().as_raw() as u64,
        fast_rows,
        fast_stride: n,
    };
    let combo_end_t =
        vec![i32::try_from(n).map_err(|_| "KVDF end_t exceeds i32 range".to_string())?; total];
    let combos = KvdfGpuComboBatch {
        fast_row_index: &combo_fast_row_idx,
        slow_row_index: &combo_slow_row_idx,
        dti_row_index: &combo_dti_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };
    let metrics = eval_kvdf_metrics_gpu_kernel_device_rows(
        device_id,
        &signals,
        &combos,
        &first.strategy,
        first_kvdf,
    )?;
    if metrics.len() != total {
        return Err("KVDF GPU validation output length mismatch".to_string());
    }
    let walk_forward_stats = if let Some(walk_cfg) = req.walk_forward.as_ref() {
        Some(compute_kvdf_gpu_walk_forward_stats_batch(
            &signals,
            &combo_fast_row_idx,
            &combo_slow_row_idx,
            &combo_dti_row_idx,
            &combo_start_t,
            &first.strategy,
            first_kvdf,
            device_id,
            walk_cfg,
        )?)
    } else {
        None
    };
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        compute_kvdf_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &combo_fast_row_idx,
            &combo_slow_row_idx,
            &combo_dti_row_idx,
            &combo_start_t,
            &start_ts,
            first_kvdf,
            device_id,
            cscv_cfg,
        )?
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        let slice_stats = compute_kvdf_gpu_cscv_slice_stats_for_candidates(
            req,
            &signals,
            &combo_fast_row_idx,
            &combo_slow_row_idx,
            &combo_dti_row_idx,
            &combo_start_t,
            &start_ts,
            first_kvdf,
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
        walk_forward_stats,
        cscv_slices,
        spa_slice_sums,
        base_composite_ctx: None,
        backend_used: BackendUsed::Gpu,
        backend_route,
        backend_fallback_reason: None,
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_cdmc_gpu_walk_forward_stats_batch(
    signals: &CdmcGpuDeviceSignalMatrices<'_>,
    dpo_row_index: &[i32],
    cycle_row_index: &[i32],
    decycler_row_index: &[i32],
    combo_start_t: &[i32],
    desired_row_major: &[i32],
    strategy: &StrategyConfig,
    cdmc: &CdmcStrategyParams,
    device_id: u32,
    cfg: &WalkForwardConfigOptional,
) -> Result<Vec<WalkForwardStats>, String> {
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let n = signals.cols;
    let mut out = Vec::with_capacity(dpo_row_index.len());
    for idx in 0..dpo_row_index.len() {
        let mut sharpes = Vec::new();
        let mut dds = Vec::new();
        let mut pnls = Vec::new();
        let mut trades = Vec::new();
        let mut t0 = usize::try_from(combo_start_t[idx]).unwrap_or(0);
        while t0 + 2 <= n {
            let end = t0.saturating_add(window).min(n);
            if end <= t0 + 1 {
                break;
            }
            let start_i = i32::try_from(t0)
                .map_err(|_| "CDMC GPU walk_forward start_t exceeds i32 range".to_string())?;
            let end_i = i32::try_from(end)
                .map_err(|_| "CDMC GPU walk_forward end_t exceeds i32 range".to_string())?;
            let combos = CdmcGpuComboBatch {
                dpo_row_index: &dpo_row_index[idx..idx + 1],
                cycle_row_index: &cycle_row_index[idx..idx + 1],
                decycler_row_index: &decycler_row_index[idx..idx + 1],
                start_t: &[start_i],
                end_t: &[end_i],
            };
            let metric = eval_cdmc_metrics_gpu_kernel_device_rows(
                device_id, signals, &combos, strategy, cdmc,
            )?
            .into_iter()
            .next()
            .ok_or_else(|| "CDMC GPU walk_forward returned no metrics".to_string())?;
            sharpes.push(metric.sharpe);
            dds.push(metric.max_dd);
            pnls.push(metric.pnl);
            let desired = &desired_row_major[idx * n..(idx + 1) * n];
            trades.push(count_trade_events_from_desired_window(
                desired, t0, end, strategy,
            ));
            if end == n {
                break;
            }
            t0 = t0.saturating_add(step);
        }
        out.push(summarize_walk_forward_metric_samples(
            sharpes, dds, pnls, trades,
        ));
    }
    Ok(out)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(dead_code)]
fn compute_cdmc_gpu_cscv_slice_stats_for_candidates(
    req: &CompositeValidationRequest,
    signals: &CdmcGpuDeviceSignalMatrices<'_>,
    dpo_row_index: &[i32],
    cycle_row_index: &[i32],
    decycler_row_index: &[i32],
    combo_start_t: &[i32],
    start_ts: &[usize],
    cdmc: &CdmcStrategyParams,
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
            i32::try_from(end).map_err(|_| "CDMC GPU CSCV end_t exceeds i32 range".to_string())?;
        let slice_start: Vec<i32> = combo_start_t
            .iter()
            .map(|&st| {
                let start = usize::try_from(st).unwrap_or(0).max(t0);
                i32::try_from(start)
                    .map_err(|_| "CDMC GPU CSCV start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let slice_end = vec![end_i; slice_start.len()];
        let combos = CdmcGpuComboBatch {
            dpo_row_index,
            cycle_row_index,
            decycler_row_index,
            start_t: &slice_start,
            end_t: &slice_end,
        };
        let metrics = eval_cdmc_metrics_gpu_kernel_device_rows(
            device_id,
            signals,
            &combos,
            &req.candidates[0].strategy,
            cdmc,
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
#[allow(dead_code)]
fn compute_cdmc_gpu_reopt_walk_forward_stats_for_candidates(
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
    let first_cdmc = first
        .cdmc
        .as_ref()
        .ok_or_else(|| "CDMC GPU reopt_walk_forward requires CDMC candidate params".to_string())?;
    validate_strategy_cfg(&first.strategy)?;
    validate_cdmc_params(first_cdmc)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite CDMC reopt_walk_forward currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if n > i32::MAX as usize {
        return Err("CDMC GPU reopt_walk_forward length exceeds i32 range".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(
        candles,
        "close",
        device_id as usize,
        &[crate::indicator_catalog::IndicatorInputKind::Slice],
    )
    .map_err(|e| e.to_string())?;

    let total = finalists.len();
    let mut dpo_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut cycle_angle_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut cycle_state_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut decycler_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut mama_row_major: Option<Vec<f32>> = None;
    let mut fama_row_major: Option<Vec<f32>> = None;
    let mut combo_dpo_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_cycle_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_decycler_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total);
    let mut finalist_entries: Vec<(String, Vec<u8>, usize)> = Vec::with_capacity(total);

    for (idx, cand) in finalists.iter().enumerate() {
        let cdmc = cand.cdmc.as_ref().ok_or_else(|| {
            "GPU composite reopt_walk_forward currently requires all finalists to use CDMC"
                .to_string()
        })?;
        if cand.asr.is_some()
            || cand.vcc.is_some()
            || cand.vemr.is_some()
            || cand.rocb.is_some()
            || cand.mkcs.is_some()
            || cand.kqsr.is_some()
            || cand.carm.is_some()
            || cand.kvdf.is_some()
        {
            return Err(
                "GPU composite reopt_walk_forward currently requires all finalists to use CDMC"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        if !serialized_eq(cdmc, first_cdmc)? {
            return Err(
                "GPU composite reopt_walk_forward currently requires uniform CDMC parameters"
                    .to_string(),
            );
        }

        let periods = resolve_cdmc_periods(first_cdmc, cand.fast_len, cand.slow_len);
        let signals = compute_cdmc_signals(
            candles,
            &req.ma_source,
            &Backend::CpuOnly,
            periods,
            first_cdmc,
            Some(&prepared_cuda_inputs),
        )?;
        if mama_row_major.is_none() {
            mama_row_major = Some(signals.mama.iter().map(|value| *value as f32).collect());
            fama_row_major = Some(signals.fama.iter().map(|value| *value as f32).collect());
        }
        let signals_first_valid = signals.first_valid;
        let start_t = compute_cdmc_start_t(
            first_valid_close,
            signals_first_valid,
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after CDMC GPU reopt warmup".to_string());
        }

        dpo_row_major.extend(signals.dpo.iter().map(|value| *value as f32));
        cycle_angle_row_major.extend(signals.cycle_angle.iter().map(|value| *value as f32));
        cycle_state_row_major.extend(signals.cycle_state.iter().map(|value| *value as f32));
        decycler_row_major.extend(signals.decycler.iter().map(|value| *value as f32));
        let row_idx = i32::try_from(idx)
            .map_err(|_| "CDMC GPU reopt row index exceeds i32 range".to_string())?;
        combo_dpo_row_idx.push(row_idx);
        combo_cycle_row_idx.push(row_idx);
        combo_decycler_row_idx.push(row_idx);
        combo_start_t.push(
            i32::try_from(start_t)
                .map_err(|_| "CDMC GPU reopt start_t exceeds i32 range".to_string())?,
        );

        let key = serde_json::to_string(cand)
            .unwrap_or_else(|_| format!("cdmc:{}:{}", cand.fast_len, cand.slow_len));
        finalist_entries.push((
            key,
            serialize_composite_candidate_key(cand)?,
            finalist_start_ts[idx],
        ));
    }

    let runtime = prepared_cuda_inputs.runtime();
    let d_dpo = runtime
        .upload_f32(dpo_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_cycle_angle = runtime
        .upload_f32(cycle_angle_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_cycle_state = runtime
        .upload_f32(cycle_state_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_mama = runtime
        .upload_f32(
            mama_row_major
                .as_deref()
                .ok_or_else(|| "CDMC GPU reopt missing MAMA row".to_string())?,
        )
        .map_err(|e| e.to_string())?;
    let d_fama = runtime
        .upload_f32(
            fama_row_major
                .as_deref()
                .ok_or_else(|| "CDMC GPU reopt missing FAMA row".to_string())?,
        )
        .map_err(|e| e.to_string())?;
    let d_decycler = runtime
        .upload_f32(decycler_row_major.as_slice())
        .map_err(|e| e.to_string())?;

    let signals = CdmcGpuDeviceSignalMatrices {
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
        cols: n,
        dpo_row_major_ptr: d_dpo.buffer().as_device_ptr().as_raw() as u64,
        dpo_rows: total,
        dpo_stride: n,
        cycle_angle_row_major_ptr: d_cycle_angle.buffer().as_device_ptr().as_raw() as u64,
        cycle_state_row_major_ptr: d_cycle_state.buffer().as_device_ptr().as_raw() as u64,
        cycle_rows: total,
        cycle_stride: n,
        mama_ptr: d_mama.buffer().as_device_ptr().as_raw() as u64,
        fama_ptr: d_fama.buffer().as_device_ptr().as_raw() as u64,
        decycler_row_major_ptr: d_decycler.buffer().as_device_ptr().as_raw() as u64,
        decycler_rows: total,
        decycler_stride: n,
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
                    .map_err(|_| "CDMC GPU reopt start_t exceeds i32 range".to_string())
            })
            .collect::<Result<_, _>>()?;
        let end_i =
            i32::try_from(end).map_err(|_| "CDMC GPU reopt end_t exceeds i32 range".to_string())?;
        let end_vec = vec![end_i; start_vec.len()];
        let combos = CdmcGpuComboBatch {
            dpo_row_index: &combo_dpo_row_idx,
            cycle_row_index: &combo_cycle_row_idx,
            decycler_row_index: &combo_decycler_row_idx,
            start_t: &start_vec,
            end_t: &end_vec,
        };
        let metrics = eval_cdmc_metrics_gpu_kernel_device_rows(
            device_id,
            &signals,
            &combos,
            &first.strategy,
            first_cdmc,
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
                        "missing CDMC GPU reopt metrics for range {}..{}",
                        start, end
                    )
                })
        },
        |(key, _, _)| key.clone(),
    )
    .ok_or_else(|| "CDMC GPU reopt_walk_forward did not produce any windows".to_string())
    .map(Some)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn try_prepare_cdmc_gpu_fixed_candidate_validation(
    req: &CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<GpuCompositeFixedCandidateValidationEval, String> {
    let device_id = composite_validation_supports_gpu_fixed_candidates(req)?;
    if let Some(reason) = composite_gpu_validation_feature_blocker(req) {
        return Err(reason);
    }
    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    let first_cdmc = first.cdmc.as_ref().ok_or_else(|| {
        "GPU composite validation currently supports CDMC candidates only".to_string()
    })?;
    if first.asr.is_some()
        || first.vcc.is_some()
        || first.vemr.is_some()
        || first.rocb.is_some()
        || first.mkcs.is_some()
        || first.kqsr.is_some()
        || first.carm.is_some()
        || first.kvdf.is_some()
    {
        return Err("GPU composite validation currently supports CDMC candidates only".to_string());
    }
    validate_strategy_cfg(&first.strategy)?;
    validate_cdmc_params(first_cdmc)?;
    if first.strategy.uses_execution_overlays()
        || first.strategy.stop_loss_pct().is_some()
        || first.strategy.take_profit_pct().is_some()
    {
        return Err(
            "GPU composite CDMC validation currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    let lr = compute_log_returns_f64(close);
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(
        candles,
        "close",
        device_id as usize,
        &[crate::indicator_catalog::IndicatorInputKind::Slice],
    )
    .map_err(|e| e.to_string())?;

    let total = req.candidates.len();
    let mut dpo_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut cycle_angle_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut cycle_state_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut decycler_row_major: Vec<f32> = Vec::with_capacity(total * n);
    let mut desired_row_major: Vec<i32> = Vec::with_capacity(total * n);
    let mut mama_row_major: Option<Vec<f32>> = None;
    let mut fama_row_major: Option<Vec<f32>> = None;
    let mut combo_dpo_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_cycle_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_decycler_row_idx: Vec<i32> = Vec::with_capacity(total);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total);
    let mut start_ts: Vec<usize> = Vec::with_capacity(total);
    let mut exact_metrics: Vec<Metrics> = Vec::with_capacity(total);
    let mut exact_series: Vec<CandidateSeries<'_>> = Vec::with_capacity(total);
    for (idx, cand) in req.candidates.iter().enumerate() {
        if cand.cdmc.is_none()
            || cand.asr.is_some()
            || cand.vcc.is_some()
            || cand.vemr.is_some()
            || cand.rocb.is_some()
            || cand.mkcs.is_some()
            || cand.kqsr.is_some()
            || cand.carm.is_some()
            || cand.kvdf.is_some()
        {
            return Err(
                "GPU composite validation currently requires all candidates to use CDMC"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, &first.strategy)? {
            return Err(
                "GPU composite validation currently requires a uniform StrategyConfig".to_string(),
            );
        }
        if !serialized_eq(cand.cdmc.as_ref().unwrap(), first_cdmc)? {
            return Err(
                "GPU composite validation currently requires uniform CDMC parameters".to_string(),
            );
        }

        let periods = resolve_cdmc_periods(first_cdmc, cand.fast_len, cand.slow_len);
        let signals = compute_cdmc_signals(
            candles,
            &req.ma_source,
            &Backend::CpuOnly,
            periods,
            first_cdmc,
            Some(&prepared_cuda_inputs),
        )?;
        if mama_row_major.is_none() {
            mama_row_major = Some(signals.mama.iter().map(|value| *value as f32).collect());
            fama_row_major = Some(signals.fama.iter().map(|value| *value as f32).collect());
        }
        let signals_first_valid = signals.first_valid;
        let start_t = compute_cdmc_start_t(
            fv_close,
            signals_first_valid,
            cand.strategy.trade_on_next_bar,
        );
        if start_t >= n {
            return Err("not enough candles after CDMC GPU warmup".to_string());
        }

        dpo_row_major.extend(signals.dpo.iter().map(|value| *value as f32));
        cycle_angle_row_major.extend(signals.cycle_angle.iter().map(|value| *value as f32));
        cycle_state_row_major.extend(signals.cycle_state.iter().map(|value| *value as f32));
        decycler_row_major.extend(signals.decycler.iter().map(|value| *value as f32));
        let desired =
            build_cdmc_desired_positions(close, &signals, start_t, &cand.strategy, first_cdmc);
        exact_metrics.push(eval_asr_metrics_from_desired(
            candles,
            &lr,
            &desired,
            start_t,
            &cand.strategy,
        ));
        desired_row_major.extend(desired.iter().copied());
        exact_series.push(CandidateSeries::Cdmc(CdmcCandidateSeries {
            candles,
            n,
            lr: &lr,
            desired,
            start_t,
            cfg: cand.strategy.clone(),
        }));
        let row_idx =
            i32::try_from(idx).map_err(|_| "CDMC row index exceeds i32 range".to_string())?;
        combo_dpo_row_idx.push(row_idx);
        combo_cycle_row_idx.push(row_idx);
        combo_decycler_row_idx.push(row_idx);
        combo_start_t.push(
            i32::try_from(start_t).map_err(|_| "CDMC start_t exceeds i32 range".to_string())?,
        );
        start_ts.push(start_t);
    }

    let runtime = prepared_cuda_inputs.runtime();
    let d_dpo = runtime
        .upload_f32(dpo_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_cycle_angle = runtime
        .upload_f32(cycle_angle_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_cycle_state = runtime
        .upload_f32(cycle_state_row_major.as_slice())
        .map_err(|e| e.to_string())?;
    let d_mama = runtime
        .upload_f32(
            mama_row_major
                .as_deref()
                .ok_or_else(|| "CDMC GPU validation missing MAMA row".to_string())?,
        )
        .map_err(|e| e.to_string())?;
    let d_fama = runtime
        .upload_f32(
            fama_row_major
                .as_deref()
                .ok_or_else(|| "CDMC GPU validation missing FAMA row".to_string())?,
        )
        .map_err(|e| e.to_string())?;
    let d_decycler = runtime
        .upload_f32(decycler_row_major.as_slice())
        .map_err(|e| e.to_string())?;

    let signals = CdmcGpuDeviceSignalMatrices {
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
        cols: n,
        dpo_row_major_ptr: d_dpo.buffer().as_device_ptr().as_raw() as u64,
        dpo_rows: total,
        dpo_stride: n,
        cycle_angle_row_major_ptr: d_cycle_angle.buffer().as_device_ptr().as_raw() as u64,
        cycle_state_row_major_ptr: d_cycle_state.buffer().as_device_ptr().as_raw() as u64,
        cycle_rows: total,
        cycle_stride: n,
        mama_ptr: d_mama.buffer().as_device_ptr().as_raw() as u64,
        fama_ptr: d_fama.buffer().as_device_ptr().as_raw() as u64,
        decycler_row_major_ptr: d_decycler.buffer().as_device_ptr().as_raw() as u64,
        decycler_rows: total,
        decycler_stride: n,
    };
    let combo_end_t =
        vec![i32::try_from(n).map_err(|_| "CDMC end_t exceeds i32 range".to_string())?; total];
    let combos = CdmcGpuComboBatch {
        dpo_row_index: &combo_dpo_row_idx,
        cycle_row_index: &combo_cycle_row_idx,
        decycler_row_index: &combo_decycler_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };
    let gpu_metrics = eval_cdmc_metrics_gpu_kernel_device_rows(
        device_id,
        &signals,
        &combos,
        &first.strategy,
        first_cdmc,
    )?;
    if gpu_metrics.len() != total {
        return Err("CDMC GPU validation output length mismatch".to_string());
    }
    let metrics = exact_metrics;
    let walk_forward_stats = if let Some(walk_cfg) = req.walk_forward.as_ref() {
        Some(compute_cdmc_gpu_walk_forward_stats_batch(
            &signals,
            &combo_dpo_row_idx,
            &combo_cycle_row_idx,
            &combo_decycler_row_idx,
            &combo_start_t,
            &desired_row_major,
            &first.strategy,
            first_cdmc,
            device_id,
            walk_cfg,
        )?)
    } else {
        None
    };
    let candidate_keys: Vec<Vec<u8>> = req
        .candidates
        .iter()
        .map(serialize_composite_candidate_key)
        .collect::<Result<_, _>>()?;
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        exact_slice_stats_for_candidate_series(&candidate_keys, &exact_series, cscv_cfg)
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        let slice_stats = exact_slice_stats_for_candidate_series(
            &candidate_keys,
            &exact_series,
            &CscvPboConfig {
                slices: spa_cfg.slices.max(4),
                max_combinations: usize::MAX,
                threshold: 0.0,
            },
        );
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
        walk_forward_stats,
        cscv_slices,
        spa_slice_sums,
        base_composite_ctx: None,
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
    if first.vcet.is_some() {
        return try_prepare_vcet_gpu_fixed_candidate_validation(req, candles);
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
    if first.kqsr.is_some() {
        return try_prepare_kqsr_gpu_fixed_candidate_validation(req, candles);
    }
    if first.carm.is_some() {
        return try_prepare_carm_gpu_fixed_candidate_validation(req, candles);
    }
    if first.kvdf.is_some() {
        return try_prepare_kvdf_gpu_fixed_candidate_validation(req, candles);
    }
    if first.cdmc.is_some() {
        return try_prepare_cdmc_gpu_fixed_candidate_validation(req, candles);
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
    pub downside_vol_ratio_median: f64,
    pub downside_vol_ratio_worst: f64,
    pub ulcer_index_median: f64,
    pub ulcer_index_worst: f64,
    pub martin_ratio_median: f64,
    pub martin_ratio_worst: f64,
    pub trades_median: u32,
    pub trades_worst: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReoptWalkForwardWindowStats {
    pub train_start: usize,
    pub train_end: usize,
    pub test_end: usize,
    pub winner_key: String,
    pub train_score: f64,
    pub train_pnl: f64,
    pub train_sharpe: f64,
    pub train_ulcer_index: f64,
    pub train_martin_ratio: f64,
    pub forward_pnl: f64,
    pub forward_sharpe: f64,
    pub forward_max_dd: f64,
    pub forward_ulcer_index: f64,
    pub forward_martin_ratio: f64,
    pub forward_trades: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pnl_efficiency: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharpe_efficiency: Option<f64>,
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
    pub forward_ulcer_index_median: f64,
    pub forward_ulcer_index_worst: f64,
    pub forward_martin_ratio_median: f64,
    pub forward_martin_ratio_worst: f64,
    pub forward_trades_median: u32,
    pub forward_trades_worst: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_pnl_efficiency_median: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_pnl_efficiency_worst: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_sharpe_efficiency_median: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_sharpe_efficiency_worst: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_sharpe_efficiency_std: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub winner_sequence: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub window_stats: Vec<ReoptWalkForwardWindowStats>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stationary_mean_block_len: Option<usize>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_mean_block_len: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_prob_pnl_positive: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_pnl_p05: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_pnl_p50: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_pnl_p95: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_max_dd_p50: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_max_dd_p95: Option<f64>,
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

fn default_cross_ticker_mode() -> CrossTickerMode {
    CrossTickerMode::Lightweight
}

#[derive(Clone, Debug, Deserialize)]
pub struct CrossTickerDataset {
    pub csv_path: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CrossTickerMode {
    Lightweight,
    PreservedFinalists,
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
    #[serde(default = "default_cross_ticker_mode")]
    pub mode: CrossTickerMode,
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
    pub mode: CrossTickerMode,
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
    pub regime_slice_robustness: Option<RegimeSliceRobustnessStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friction_stress: Option<FrictionStressStats>,
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

    fn walk_forward_stats(&self) -> Option<&WalkForwardStats> {
        self.walk_forward.as_ref()
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_diagnostics: Option<Vec<String>>,
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

struct VcetCandidateSeries<'a> {
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

struct KqsrCandidateSeries<'a> {
    candles: &'a vector_ta::utilities::data_loader::Candles,
    n: usize,
    lr: &'a [f64],
    desired: Vec<i32>,
    start_t: usize,
    cfg: StrategyConfig,
}

struct CarmCandidateSeries<'a> {
    candles: &'a vector_ta::utilities::data_loader::Candles,
    n: usize,
    lr: &'a [f64],
    desired: Vec<i32>,
    start_t: usize,
    cfg: StrategyConfig,
}

struct KvdfCandidateSeries<'a> {
    candles: &'a vector_ta::utilities::data_loader::Candles,
    n: usize,
    lr: &'a [f64],
    desired: Vec<i32>,
    start_t: usize,
    cfg: StrategyConfig,
}

struct CdmcCandidateSeries<'a> {
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
    Vcet(VcetCandidateSeries<'a>),
    Mkcs(MkcsCandidateSeries<'a>),
    Kqsr(KqsrCandidateSeries<'a>),
    Carm(CarmCandidateSeries<'a>),
    Kvdf(KvdfCandidateSeries<'a>),
    Cdmc(CdmcCandidateSeries<'a>),
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
        + usize::from(params.vcet.is_some())
        + usize::from(params.mkcs.is_some())
        + usize::from(params.kqsr.is_some())
        + usize::from(params.carm.is_some())
        + usize::from(params.kvdf.is_some())
        + usize::from(params.cdmc.is_some());
    if custom_strategy_count > 1 {
        return Err(
            "Select only one of ASR, VCC, VEMR, ROCB, VCET, MKCS, KQSR, CARM, KVDF, or CDMC strategy config."
                .to_string(),
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
    if let Some(vcet_cfg) = params.vcet.as_ref() {
        return build_vcet_candidate_series(
            candles,
            lr,
            ott_prices,
            ma_source,
            params,
            vcet_cfg,
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
    if let Some(kqsr_cfg) = params.kqsr.as_ref() {
        return build_kqsr_candidate_series(
            candles,
            lr,
            ott_prices,
            ma_source,
            params,
            kqsr_cfg,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs,
        );
    }
    if let Some(carm_cfg) = params.carm.as_ref() {
        return build_carm_candidate_series(
            candles,
            lr,
            ott_prices,
            ma_source,
            params,
            carm_cfg,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs,
        );
    }
    if let Some(kvdf_cfg) = params.kvdf.as_ref() {
        return build_kvdf_candidate_series(
            candles,
            lr,
            ott_prices,
            ma_source,
            params,
            kvdf_cfg,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs,
        );
    }
    if let Some(cdmc_cfg) = params.cdmc.as_ref() {
        return build_cdmc_candidate_series(
            candles,
            lr,
            ott_prices,
            ma_source,
            params,
            cdmc_cfg,
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

fn build_vcet_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    ott_prices: &'a [f64],
    ma_source: &str,
    params: &CompositeParams,
    vcet_cfg: &VcetStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;
    validate_vcet_params(vcet_cfg)?;

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

    let periods = resolve_vcet_periods(vcet_cfg, params.fast_len, params.slow_len);
    let vidya: VcetVidyaSignals = compute_vcet_vidya_signals(
        candles,
        ma_source,
        &backend,
        periods,
        vcet_cfg.vidya_alpha,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let ce: VcetChandelierSignals = compute_vcet_chandelier_signals(
        candles,
        &backend,
        periods.ce,
        vcet_cfg.ce_mult,
        vcet_cfg.ce_use_close,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let start_t = compute_vcet_start_t(
        first_valid_close,
        first_valid_high,
        first_valid_low,
        vidya.first_valid,
        ce.first_valid,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    let desired =
        build_vcet_desired_positions(close, &vidya, &ce, start_t, &params.strategy, vcet_cfg);

    Ok(CandidateSeries::Vcet(VcetCandidateSeries {
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

fn build_kqsr_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    _ott_prices: &'a [f64],
    ma_source: &str,
    params: &CompositeParams,
    kqsr_cfg: &KqsrStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;
    validate_kqsr_params(kqsr_cfg)?;

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if lr.len() != n {
        return Err("series length mismatch".to_string());
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    if close.len() != n {
        return Err("series length mismatch".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;

    let backend = Backend::CpuOnly;
    let periods = resolve_kqsr_periods(kqsr_cfg, params.fast_len, params.slow_len);
    let signals: KqsrSignals = compute_kqsr_signals(
        candles,
        ma_source,
        &backend,
        periods,
        kqsr_cfg,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let start_t = compute_kqsr_start_t(
        first_valid_close,
        signals.first_valid,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    let desired =
        build_kqsr_desired_positions(close, &signals, start_t, &params.strategy, kqsr_cfg);

    Ok(CandidateSeries::Kqsr(KqsrCandidateSeries {
        candles,
        n,
        lr,
        desired,
        start_t,
        cfg: params.strategy.clone(),
    }))
}

fn build_carm_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    _ott_prices: &'a [f64],
    ma_source: &str,
    params: &CompositeParams,
    carm_cfg: &CarmStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;
    validate_carm_params(carm_cfg)?;

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if lr.len() != n {
        return Err("series length mismatch".to_string());
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    if close.len() != n {
        return Err("series length mismatch".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;

    let backend = Backend::CpuOnly;
    let periods = resolve_carm_periods(carm_cfg, params.fast_len, params.slow_len);
    let signals: CarmSignals = compute_carm_signals(
        candles,
        ma_source,
        &backend,
        periods,
        carm_cfg,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let start_t = compute_carm_start_t(
        first_valid_close,
        signals.first_valid,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    let desired =
        build_carm_desired_positions(close, &signals, start_t, &params.strategy, carm_cfg);

    Ok(CandidateSeries::Carm(CarmCandidateSeries {
        candles,
        n,
        lr,
        desired,
        start_t,
        cfg: params.strategy.clone(),
    }))
}

fn build_kvdf_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    _ott_prices: &'a [f64],
    ma_source: &str,
    params: &CompositeParams,
    kvdf_cfg: &KvdfStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;
    validate_kvdf_params(kvdf_cfg)?;

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if lr.len() != n {
        return Err("series length mismatch".to_string());
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    if close.len() != n {
        return Err("series length mismatch".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;

    let backend = Backend::CpuOnly;
    let periods = resolve_kvdf_periods(kvdf_cfg, params.fast_len, params.slow_len);
    let signals: KvdfSignals = compute_kvdf_signals(
        candles,
        ma_source,
        &backend,
        periods,
        kvdf_cfg,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let start_t = compute_kvdf_start_t(
        first_valid_close,
        signals.first_valid,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    let desired =
        build_kvdf_desired_positions(close, &signals, start_t, &params.strategy, kvdf_cfg);

    Ok(CandidateSeries::Kvdf(KvdfCandidateSeries {
        candles,
        n,
        lr,
        desired,
        start_t,
        cfg: params.strategy.clone(),
    }))
}

fn build_cdmc_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    _ott_prices: &'a [f64],
    ma_source: &str,
    params: &CompositeParams,
    cdmc_cfg: &CdmcStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;
    validate_cdmc_params(cdmc_cfg)?;

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if lr.len() != n {
        return Err("series length mismatch".to_string());
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    if close.len() != n {
        return Err("series length mismatch".to_string());
    }
    let first_valid_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;

    let backend = Backend::CpuOnly;
    let periods = resolve_cdmc_periods(cdmc_cfg, params.fast_len, params.slow_len);
    let signals: CdmcSignals = compute_cdmc_signals(
        candles,
        ma_source,
        &backend,
        periods,
        cdmc_cfg,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let start_t = compute_cdmc_start_t(
        first_valid_close,
        signals.first_valid,
        params.strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    let desired =
        build_cdmc_desired_positions(close, &signals, start_t, &params.strategy, cdmc_cfg);

    Ok(CandidateSeries::Cdmc(CdmcCandidateSeries {
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
        CandidateSeries::Vcet(series) => eval_asr_metrics_window_from_desired(
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
        CandidateSeries::Kqsr(series) => eval_asr_metrics_window_from_desired(
            series.candles,
            series.lr,
            &series.desired,
            start_t,
            end_t.min(series.n),
            &series.cfg,
        ),
        CandidateSeries::Carm(series) => eval_asr_metrics_window_from_desired(
            series.candles,
            series.lr,
            &series.desired,
            start_t,
            end_t.min(series.n),
            &series.cfg,
        ),
        CandidateSeries::Kvdf(series) => eval_asr_metrics_window_from_desired(
            series.candles,
            series.lr,
            &series.desired,
            start_t,
            end_t.min(series.n),
            &series.cfg,
        ),
        CandidateSeries::Cdmc(series) => eval_asr_metrics_window_from_desired(
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
        CandidateSeries::Vcet(series) => {
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
        CandidateSeries::Kqsr(series) => {
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
        CandidateSeries::Carm(series) => {
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
        CandidateSeries::Kvdf(series) => {
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
        CandidateSeries::Cdmc(series) => {
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
        CandidateSeries::Vcet(series) => {
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
        CandidateSeries::Kqsr(series) => {
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
        CandidateSeries::Carm(series) => {
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
        CandidateSeries::Kvdf(series) => {
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
        CandidateSeries::Cdmc(series) => {
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
        CandidateSeries::Vcet(series) => {
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
        CandidateSeries::Kqsr(series) => {
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
        CandidateSeries::Carm(series) => {
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
        CandidateSeries::Kvdf(series) => {
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
        CandidateSeries::Cdmc(series) => {
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

fn exact_slice_stats_for_candidate_series(
    candidate_keys: &[Vec<u8>],
    series_list: &[CandidateSeries<'_>],
    cfg: &CscvPboConfig,
) -> Option<HashMap<Vec<u8>, Vec<SliceReturnStats>>> {
    let slices = sanitize_gpu_cscv_slices(cfg.slices);
    let common_start_t = series_list.iter().map(series_start_t).max().unwrap_or(0);
    let n = series_list.iter().map(series_n).min().unwrap_or(0);
    let slice_bars = n.saturating_sub(common_start_t) / slices;
    if slice_bars < 2 || series_list.len() < 2 {
        return None;
    }

    let mut out: HashMap<Vec<u8>, Vec<SliceReturnStats>> =
        HashMap::with_capacity(candidate_keys.len());
    for (key, series) in candidate_keys.iter().zip(series_list.iter()) {
        out.insert(
            key.clone(),
            slice_return_stats_for_series(series, common_start_t, slice_bars, slices),
        );
    }
    Some(out)
}

fn series_n(series: &CandidateSeries<'_>) -> usize {
    match series {
        CandidateSeries::Composite(series) => series.n,
        CandidateSeries::Asr(series) => series.n,
        CandidateSeries::Vcc(series) => series.n,
        CandidateSeries::Vemr(series) => series.n,
        CandidateSeries::Rocb(series) => series.n,
        CandidateSeries::Vcet(series) => series.n,
        CandidateSeries::Mkcs(series) => series.n,
        CandidateSeries::Kqsr(series) => series.n,
        CandidateSeries::Carm(series) => series.n,
        CandidateSeries::Kvdf(series) => series.n,
        CandidateSeries::Cdmc(series) => series.n,
    }
}

fn series_start_t(series: &CandidateSeries<'_>) -> usize {
    match series {
        CandidateSeries::Composite(series) => series.start_t,
        CandidateSeries::Asr(series) => series.start_t,
        CandidateSeries::Vcc(series) => series.start_t,
        CandidateSeries::Vemr(series) => series.start_t,
        CandidateSeries::Rocb(series) => series.start_t,
        CandidateSeries::Vcet(series) => series.start_t,
        CandidateSeries::Mkcs(series) => series.start_t,
        CandidateSeries::Kqsr(series) => series.start_t,
        CandidateSeries::Carm(series) => series.start_t,
        CandidateSeries::Kvdf(series) => series.start_t,
        CandidateSeries::Cdmc(series) => series.start_t,
    }
}

fn series_lr<'a>(series: &'a CandidateSeries<'a>) -> &'a [f64] {
    match series {
        CandidateSeries::Composite(series) => series.lr,
        CandidateSeries::Asr(series) => series.lr,
        CandidateSeries::Vcc(series) => series.lr,
        CandidateSeries::Vemr(series) => series.lr,
        CandidateSeries::Rocb(series) => series.lr,
        CandidateSeries::Vcet(series) => series.lr,
        CandidateSeries::Mkcs(series) => series.lr,
        CandidateSeries::Kqsr(series) => series.lr,
        CandidateSeries::Carm(series) => series.lr,
        CandidateSeries::Kvdf(series) => series.lr,
        CandidateSeries::Cdmc(series) => series.lr,
    }
}

fn series_cfg<'a>(series: &'a CandidateSeries<'a>) -> &'a StrategyConfig {
    match series {
        CandidateSeries::Composite(series) => &series.cfg,
        CandidateSeries::Asr(series) => &series.cfg,
        CandidateSeries::Vcc(series) => &series.cfg,
        CandidateSeries::Vemr(series) => &series.cfg,
        CandidateSeries::Rocb(series) => &series.cfg,
        CandidateSeries::Vcet(series) => &series.cfg,
        CandidateSeries::Mkcs(series) => &series.cfg,
        CandidateSeries::Kqsr(series) => &series.cfg,
        CandidateSeries::Carm(series) => &series.cfg,
        CandidateSeries::Kvdf(series) => &series.cfg,
        CandidateSeries::Cdmc(series) => &series.cfg,
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

        let metric = if pp.vcet.is_some() || pp.carm.is_some() || pp.cdmc.is_some() {
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
        } else {
            match try_prepare_composite_gpu_fixed_candidate_validation(&temp_req, candles) {
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

    clear_validation_strategy_route_diagnostics(&req.candidates);

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

    let (mut out, cscv_pbo) = crate::validation_core::run_validation_candidates(
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
                    if p.vcc.is_some() || p.vcet.is_some() || p.carm.is_some() || p.cdmc.is_some() {
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
                        Some(crate::validation_pipeline::compute_walk_forward_stats(
                            series_n(&series),
                            series_start_t(&series),
                            cfg,
                            |t0, end| eval_metrics_window(&series, t0, end),
                        ))
                    } else if let Some(stats) = gpu_eval.walk_forward_stats.as_ref() {
                        Some(stats.get(idx).cloned().ok_or_else(|| {
                            "GPU composite validation walk_forward index out of bounds".to_string()
                        })?)
                    } else if p.asr.is_some() {
                        Some(compute_asr_gpu_walk_forward_stats_for_candidate(
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
                        &trades, cfg, rng,
                    ))
                } else {
                    None
                };
                let robust_score = crate::validation_pipeline::robust_score_from_metrics(
                    &in_sample,
                    walk_forward.as_ref(),
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
                        regime_slice_robustness: None,
                        friction_stress: None,
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
                } else if gpu_fixed_eval.is_some()
                    && (p.vcc.is_some() || p.vcet.is_some() || p.carm.is_some() || p.cdmc.is_some())
                {
                    Some(crate::validation_pipeline::compute_walk_forward_stats(
                        n_total,
                        start_t0,
                        cfg,
                        |t0, end| eval_metrics_window(&series, t0, end),
                    ))
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
                let trades = trades_for_series(&series);
                Some(crate::validation_pipeline::compute_trade_bootstrap_stats(
                    &trades, cfg, rng,
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
                walk_forward.as_ref(),
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
                    regime_slice_robustness: None,
                    friction_stress: None,
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
            if gpu_fixed_eval.is_some()
                && cand.params.vcet.is_none()
                && cand.params.carm.is_none()
                && cand.params.cdmc.is_none()
            {
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

    let regime_eval_count = DEFAULT_COMPOSITE_REGIME_SLICE_FINALISTS.min(out.len());
    for cand in out.iter_mut().take(regime_eval_count) {
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
            &Backend::CpuOnly,
            #[cfg(feature = "cuda")]
            None,
        )?;
        cand.regime_slice_robustness = compute_composite_regime_slice_robustness_stats(&series);
    }

    let friction_eval_count = DEFAULT_COMPOSITE_FRICTION_STRESS_FINALISTS.min(out.len());
    if friction_eval_count > 0 {
        let finalist_params: Vec<CompositeParams> = out
            .iter()
            .take(friction_eval_count)
            .map(|cand| cand.params.clone())
            .collect();
        let summaries = compute_composite_friction_stress_summaries(
            candles,
            &lr,
            ma_prices,
            base_first_valid,
            &ma_source,
            fast_ma_params.as_ref(),
            slow_ma_params.as_ref(),
            &finalist_params,
        )?;
        for (cand, summary) in out
            .iter_mut()
            .take(friction_eval_count)
            .zip(summaries.into_iter())
        {
            cand.friction_stress = summary;
        }
    }

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
                    } else if first_params.kqsr.is_some() {
                        compute_kqsr_gpu_reopt_walk_forward_stats_for_candidates(
                            &req,
                            candles,
                            &finalist_params,
                            &finalist_start_ts,
                            cfg,
                        )?
                    } else if first_params.vcc.is_some()
                        || first_params.carm.is_some()
                        || first_params.vcet.is_some()
                        || first_params.cdmc.is_some()
                    {
                        let mut finalist_series: Vec<(String, CandidateSeries<'_>)> =
                            Vec::with_capacity(finalist_params.len());
                        for params in &finalist_params {
                            let key = serde_json::to_string(params).unwrap_or_else(|_| {
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
                                    params,
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
                    } else if first_params.kvdf.is_some() {
                        compute_kvdf_gpu_reopt_walk_forward_stats_for_candidates(
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
                    if params.carm.is_some() || params.cdmc.is_some() {
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
                        continue;
                    }
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
    let route_diagnostics = take_validation_strategy_route_diagnostics(&req.candidates)
        .filter(|entries| !entries.is_empty());
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
        route_diagnostics,
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

const DEFAULT_COMPOSITE_REGIME_SLICE_VOL_WINDOW: usize = 48;
const DEFAULT_COMPOSITE_REGIME_SLICE_FINALISTS: usize = 5;
const DEFAULT_COMPOSITE_FRICTION_STRESS_FINALISTS: usize = 4;
const DEFAULT_COMPOSITE_FRICTION_FEE_FLOOR_BPS: f64 = 2.0;
const DEFAULT_COMPOSITE_FRICTION_SPREAD_FLOOR_BPS: f64 = 4.0;
const DEFAULT_COMPOSITE_FRICTION_SLIPPAGE_FLOOR_BPS: f64 = 2.0;

fn compute_step_returns_for_candidate_series(
    series: &CandidateSeries<'_>,
) -> (Vec<f64>, Vec<bool>, Vec<bool>) {
    let n = series_n(series);
    let start_t = series_start_t(series);
    let lr = series_lr(series);
    let desired = desired_positions_for_series(series, start_t, n);
    let mut step_returns = vec![0.0; n];
    let mut active_flags = vec![false; n];
    let mut trade_entries = vec![false; n];
    let mut prev_pos = 0i32;

    for t in start_t..n {
        let pos = *desired.get(t).unwrap_or(&0);
        if pos != 0 {
            active_flags[t] = true;
            step_returns[t] = (pos as f64) * lr[t].exp_m1();
        }
        if pos != 0 && pos != prev_pos {
            trade_entries[t] = true;
        }
        prev_pos = pos;
    }

    (step_returns, active_flags, trade_entries)
}

fn rolling_volatility_tertiles_generic(lr: &[f64], start_t: usize, window: usize) -> Vec<u8> {
    let n = lr.len();
    let mut states = vec![u8::MAX; n];
    if n == 0 || start_t >= n {
        return states;
    }

    let mut prefix = vec![0.0; n + 1];
    let mut prefix_sq = vec![0.0; n + 1];
    for (idx, value) in lr.iter().copied().enumerate() {
        prefix[idx + 1] = prefix[idx] + value;
        prefix_sq[idx + 1] = prefix_sq[idx] + value * value;
    }

    let mut vols = Vec::new();
    let mut raw = vec![f64::NAN; n];
    for t in start_t..n {
        let begin = t.saturating_add(1).saturating_sub(window.max(1));
        let count = t.saturating_sub(begin).saturating_add(1);
        if count < window.max(1) {
            continue;
        }
        let sum = prefix[t + 1] - prefix[begin];
        let sumsq = prefix_sq[t + 1] - prefix_sq[begin];
        let mean = sum / count as f64;
        let variance = (sumsq / count as f64 - mean * mean).max(0.0);
        let vol = variance.sqrt();
        raw[t] = vol;
        vols.push(vol);
    }
    if vols.is_empty() {
        return states;
    }
    vols.sort_by(|a, b| a.total_cmp(b));
    let low_thr = crate::stats_common::quantile(&vols, 1.0 / 3.0);
    let high_thr = crate::stats_common::quantile(&vols, 2.0 / 3.0);
    for t in start_t..n {
        let vol = raw[t];
        if !vol.is_finite() {
            continue;
        }
        states[t] = if vol <= low_thr {
            0
        } else if vol <= high_thr {
            1
        } else {
            2
        };
    }
    states
}

fn rolling_trend_efficiency_scores(lr: &[f64], start_t: usize, window: usize) -> Vec<f64> {
    let n = lr.len();
    let mut out = vec![f64::NAN; n];
    if n == 0 || start_t >= n {
        return out;
    }
    let mut prefix = vec![0.0; n + 1];
    let mut abs_prefix = vec![0.0; n + 1];
    for (idx, value) in lr.iter().copied().enumerate() {
        prefix[idx + 1] = prefix[idx] + value;
        abs_prefix[idx + 1] = abs_prefix[idx] + value.abs();
    }
    for t in start_t..n {
        let begin = t.saturating_add(1).saturating_sub(window.max(1));
        let count = t.saturating_sub(begin).saturating_add(1);
        if count < window.max(1) {
            continue;
        }
        let net = prefix[t + 1] - prefix[begin];
        let gross = abs_prefix[t + 1] - abs_prefix[begin];
        if gross > 0.0 {
            out[t] = net.abs() / gross;
        }
    }
    out
}

fn summarize_regime_bucket_generic(
    step_returns: &[f64],
    active_flags: &[bool],
    trade_entries: &[bool],
    membership: &[bool],
) -> RegimeBucketStats {
    let mut bars = 0usize;
    let mut active_bars = 0usize;
    let mut trades = 0usize;
    let mut eq = 1.0_f64;
    let mut peak = 1.0_f64;
    let mut max_dd = 0.0_f64;
    let mut ulcer_sum_sq = 0.0_f64;
    let mut sum_neg_sq = 0.0_f64;
    let mut samples = Vec::new();

    for idx in 0..membership.len() {
        if !membership[idx] {
            continue;
        }
        bars += 1;
        if active_flags[idx] {
            active_bars += 1;
        }
        if trade_entries[idx] {
            trades += 1;
        }
        let r = step_returns[idx];
        samples.push(r);
        eq *= (1.0 + r).max(1.0e-7);
        if eq > peak {
            peak = eq;
        }
        let dd = (peak - eq) / peak.max(1.0e-7);
        if dd > max_dd {
            max_dd = dd;
        }
        ulcer_sum_sq += dd * dd;
        if r < 0.0 {
            sum_neg_sq += r * r;
        }
    }

    if bars == 0 {
        return RegimeBucketStats {
            bars: 0,
            active_frac: 0.0,
            pnl: 0.0,
            sharpe: 0.0,
            downside_deviation: 0.0,
            ulcer_index: 0.0,
            martin_ratio: 0.0,
            max_dd: 0.0,
            trades: 0,
        };
    }

    let mean = if samples.is_empty() {
        0.0
    } else {
        samples.iter().copied().sum::<f64>() / samples.len() as f64
    };
    let std = if samples.len() <= 1 {
        0.0
    } else {
        let sumsq = samples
            .iter()
            .map(|value| {
                let d = *value - mean;
                d * d
            })
            .sum::<f64>();
        (sumsq / (samples.len() - 1) as f64).sqrt()
    };
    let sharpe = if std > 0.0 { mean / std } else { 0.0 };
    let downside_deviation = (sum_neg_sq / bars as f64).sqrt();
    let ulcer_index = (ulcer_sum_sq / bars as f64).sqrt();
    let pnl = eq - 1.0;
    let martin_ratio = if ulcer_index > 1.0e-12 {
        (pnl / ulcer_index).clamp(-1.0e6, 1.0e6)
    } else if pnl > 0.0 {
        1.0e6
    } else if pnl < 0.0 {
        -1.0e6
    } else {
        0.0
    };
    RegimeBucketStats {
        bars,
        active_frac: active_bars as f64 / bars as f64,
        pnl,
        sharpe,
        downside_deviation,
        ulcer_index,
        martin_ratio,
        max_dd,
        trades,
    }
}

fn derive_regime_support_label_generic(
    trend_pass: bool,
    range_pass: bool,
    stable_bucket_count: usize,
) -> String {
    match (trend_pass, range_pass) {
        (true, true) => "broad".to_string(),
        (true, false) => "trend_only".to_string(),
        (false, true) => "range_only".to_string(),
        (false, false) if stable_bucket_count >= 2 => "vol_selective".to_string(),
        _ => "weak".to_string(),
    }
}

fn compute_composite_regime_slice_robustness_stats(
    series: &CandidateSeries<'_>,
) -> Option<RegimeSliceRobustnessStats> {
    let n = series_n(series);
    let start_t = series_start_t(series);
    let lr = series_lr(series);
    if n.saturating_sub(start_t) < DEFAULT_COMPOSITE_REGIME_SLICE_VOL_WINDOW.max(24) {
        return None;
    }

    let (step_returns, active_flags, trade_entries) =
        compute_step_returns_for_candidate_series(series);
    let vol_states =
        rolling_volatility_tertiles_generic(lr, start_t, DEFAULT_COMPOSITE_REGIME_SLICE_VOL_WINDOW);
    let trend_eff =
        rolling_trend_efficiency_scores(lr, start_t, DEFAULT_COMPOSITE_REGIME_SLICE_VOL_WINDOW);
    let mut eff_values: Vec<f64> = trend_eff[start_t..]
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    if eff_values.is_empty() {
        return None;
    }
    eff_values.sort_by(|a, b| a.total_cmp(b));
    let trend_threshold = crate::stats_common::median(&eff_values);

    let mut low_mask = vec![false; n];
    let mut mid_mask = vec![false; n];
    let mut high_mask = vec![false; n];
    let mut trend_mask = vec![false; n];
    let mut range_mask = vec![false; n];
    for t in start_t..n {
        match vol_states[t] {
            0 => low_mask[t] = true,
            1 => mid_mask[t] = true,
            2 => high_mask[t] = true,
            _ => {}
        }
        let eff = trend_eff[t];
        if eff.is_finite() {
            if eff >= trend_threshold {
                trend_mask[t] = true;
            } else {
                range_mask[t] = true;
            }
        }
    }

    let low_vol =
        summarize_regime_bucket_generic(&step_returns, &active_flags, &trade_entries, &low_mask);
    let mid_vol =
        summarize_regime_bucket_generic(&step_returns, &active_flags, &trade_entries, &mid_mask);
    let high_vol =
        summarize_regime_bucket_generic(&step_returns, &active_flags, &trade_entries, &high_mask);
    let trend =
        summarize_regime_bucket_generic(&step_returns, &active_flags, &trade_entries, &trend_mask);
    let range =
        summarize_regime_bucket_generic(&step_returns, &active_flags, &trade_entries, &range_mask);
    let trend_pass = trend.sharpe >= 0.0 && trend.pnl >= 0.0;
    let range_pass = range.sharpe >= 0.0 && range.pnl >= 0.0;

    let buckets = [
        ("low_vol", &low_vol),
        ("mid_vol", &mid_vol),
        ("high_vol", &high_vol),
        ("trend", &trend),
        ("range", &range),
    ];
    let mut worst_bucket_label = String::new();
    let mut worst_sharpe = f64::INFINITY;
    let mut worst_ulcer_index = 0.0_f64;
    let mut worst_martin_ratio = f64::INFINITY;
    let mut worst_max_dd = 0.0_f64;
    let mut stable_bucket_count = 0usize;
    for (label, stats) in buckets {
        if stats.bars == 0 {
            continue;
        }
        if stats.sharpe < worst_sharpe {
            worst_sharpe = stats.sharpe;
            worst_bucket_label = label.to_string();
        }
        if stats.max_dd > worst_max_dd {
            worst_max_dd = stats.max_dd;
        }
        if stats.ulcer_index > worst_ulcer_index {
            worst_ulcer_index = stats.ulcer_index;
        }
        if stats.martin_ratio < worst_martin_ratio {
            worst_martin_ratio = stats.martin_ratio;
        }
        if stats.sharpe >= 0.0 && stats.pnl >= 0.0 {
            stable_bucket_count += 1;
        }
    }
    if !worst_sharpe.is_finite() {
        return None;
    }
    if !worst_martin_ratio.is_finite() {
        worst_martin_ratio = 0.0;
    }
    let stable_bucket_frac = stable_bucket_count as f64 / 5.0;
    let support_label =
        derive_regime_support_label_generic(trend_pass, range_pass, stable_bucket_count);

    Some(RegimeSliceRobustnessStats {
        vol_window: DEFAULT_COMPOSITE_REGIME_SLICE_VOL_WINDOW,
        low_vol,
        mid_vol,
        high_vol,
        trend,
        range,
        worst_bucket_label,
        worst_sharpe,
        worst_ulcer_index,
        worst_martin_ratio,
        worst_max_dd,
        stable_bucket_count,
        stable_bucket_frac,
        trend_pass,
        range_pass,
        support_label,
    })
}

#[derive(Clone, Copy)]
struct CompositeFrictionScenarioSpec {
    label: &'static str,
    fee_bps: f64,
    spread_bps: f64,
    slippage_bps: f64,
}

fn composite_friction_stress_scenarios(
    base: &StrategyConfig,
) -> [CompositeFrictionScenarioSpec; 4] {
    let fee_stress = (base.fee_bps * 2.0)
        .max(base.fee_bps + 1.0)
        .max(DEFAULT_COMPOSITE_FRICTION_FEE_FLOOR_BPS);
    let spread_stress = (base.spread_bps * 2.0)
        .max(base.spread_bps + 2.0)
        .max(DEFAULT_COMPOSITE_FRICTION_SPREAD_FLOOR_BPS);
    let slippage_stress = (base.slippage_bps * 2.0)
        .max(base.slippage_bps + 1.0)
        .max(DEFAULT_COMPOSITE_FRICTION_SLIPPAGE_FLOOR_BPS);
    [
        CompositeFrictionScenarioSpec {
            label: "fee_stress",
            fee_bps: fee_stress,
            spread_bps: base.spread_bps,
            slippage_bps: base.slippage_bps,
        },
        CompositeFrictionScenarioSpec {
            label: "spread_stress",
            fee_bps: base.fee_bps,
            spread_bps: spread_stress,
            slippage_bps: base.slippage_bps,
        },
        CompositeFrictionScenarioSpec {
            label: "slippage_stress",
            fee_bps: base.fee_bps,
            spread_bps: base.spread_bps,
            slippage_bps: slippage_stress,
        },
        CompositeFrictionScenarioSpec {
            label: "combined_stress",
            fee_bps: fee_stress,
            spread_bps: spread_stress,
            slippage_bps: slippage_stress,
        },
    ]
}

fn compute_composite_friction_stress_summaries(
    candles: &vector_ta::utilities::data_loader::Candles,
    lr: &[f64],
    ott_prices: &[f64],
    base_first_valid: usize,
    ma_source: &str,
    fast_ma_params_req: Option<&HashMap<String, f64>>,
    slow_ma_params_req: Option<&HashMap<String, f64>>,
    finalists: &[CompositeParams],
) -> Result<Vec<Option<FrictionStressStats>>, String> {
    if finalists.is_empty() {
        return Ok(Vec::new());
    }

    let scenario_labels = composite_friction_stress_scenarios(&StrategyConfig::default())
        .map(|scenario| scenario.label);
    let scenario_count = scenario_labels.len();
    let mut base_ranks = vec![0usize; finalists.len()];
    let mut scenario_ranks = vec![vec![0usize; finalists.len()]; scenario_count];
    let mut scenario_metrics: Vec<Vec<Metrics>> =
        vec![Vec::with_capacity(finalists.len()); scenario_count];
    let mut scenario_scores = vec![vec![0.0_f64; finalists.len()]; scenario_count];

    let sort_scored = |scored: &mut Vec<(usize, Metrics, f64)>| {
        scored.sort_by(|a, b| {
            b.2.total_cmp(&a.2)
                .then_with(|| b.1.sharpe.total_cmp(&a.1.sharpe))
                .then_with(|| a.1.max_dd.total_cmp(&b.1.max_dd))
        });
    };

    let mut base_scored = Vec::with_capacity(finalists.len());
    for (idx, params) in finalists.iter().enumerate() {
        let series = build_candidate_series(
            candles,
            lr,
            ott_prices,
            base_first_valid,
            ma_source,
            fast_ma_params_req,
            slow_ma_params_req,
            params,
            #[cfg(feature = "cuda")]
            &Backend::CpuOnly,
            #[cfg(feature = "cuda")]
            None,
        )?;
        let start_t = series_start_t(&series);
        let metrics = eval_metrics_window(&series, start_t, series_n(&series));
        let score = crate::validation_pipeline::robust_score_from_metrics(&metrics, None);
        base_scored.push((idx, metrics, score));
    }
    sort_scored(&mut base_scored);
    for (rank_idx, (finalist_idx, _, _)) in base_scored.into_iter().enumerate() {
        base_ranks[finalist_idx] = rank_idx + 1;
    }

    for scenario_idx in 0..scenario_count {
        let mut scored = Vec::with_capacity(finalists.len());
        for (idx, params) in finalists.iter().enumerate() {
            let scenario = composite_friction_stress_scenarios(&params.strategy)[scenario_idx];
            let mut stressed = params.clone();
            stressed.strategy.fee_bps = scenario.fee_bps;
            stressed.strategy.spread_bps = scenario.spread_bps;
            stressed.strategy.slippage_bps = scenario.slippage_bps;
            let series = build_candidate_series(
                candles,
                lr,
                ott_prices,
                base_first_valid,
                ma_source,
                fast_ma_params_req,
                slow_ma_params_req,
                &stressed,
                #[cfg(feature = "cuda")]
                &Backend::CpuOnly,
                #[cfg(feature = "cuda")]
                None,
            )?;
            let start_t = series_start_t(&series);
            let metrics = eval_metrics_window(&series, start_t, series_n(&series));
            let score = crate::validation_pipeline::robust_score_from_metrics(&metrics, None);
            scored.push((idx, metrics, score));
        }
        sort_scored(&mut scored);
        let mut metrics_by_finalist: Vec<Option<Metrics>> = vec![None; finalists.len()];
        let mut scores_by_finalist = vec![0.0_f64; finalists.len()];
        for (rank_idx, (finalist_idx, metrics, score)) in scored.into_iter().enumerate() {
            scenario_ranks[scenario_idx][finalist_idx] = rank_idx + 1;
            metrics_by_finalist[finalist_idx] = Some(metrics);
            scores_by_finalist[finalist_idx] = score;
        }
        scenario_metrics[scenario_idx] = metrics_by_finalist
            .into_iter()
            .map(|metrics| metrics.expect("friction metrics for each finalist"))
            .collect();
        scenario_scores[scenario_idx] = scores_by_finalist;
    }

    let mut out = Vec::with_capacity(finalists.len());
    for finalist_idx in 0..finalists.len() {
        let mut pass_count = 0usize;
        let mut worst_label = String::new();
        let mut worst_score = f64::INFINITY;
        let mut worst_pnl = 0.0_f64;
        let mut worst_sharpe = 0.0_f64;
        let mut worst_downside_deviation = 0.0_f64;
        let mut worst_ulcer_index = 0.0_f64;
        let mut worst_martin_ratio = 0.0_f64;
        let mut worst_max_dd = 0.0_f64;
        let mut rank_sum = 0usize;
        let mut max_rank_drift = 0usize;
        for scenario_idx in 0..scenario_count {
            let metrics = &scenario_metrics[scenario_idx][finalist_idx];
            if metrics.pnl >= 0.0 && metrics.sharpe >= 0.0 {
                pass_count += 1;
            }
            let scenario_score = scenario_scores[scenario_idx][finalist_idx];
            if scenario_score < worst_score {
                worst_score = scenario_score;
                worst_pnl = metrics.pnl;
                worst_sharpe = metrics.sharpe;
                worst_downside_deviation = metrics.downside_deviation;
                worst_ulcer_index = metrics.ulcer_index;
                worst_martin_ratio = metrics.martin_ratio;
                worst_max_dd = metrics.max_dd;
                worst_label = scenario_labels[scenario_idx].to_string();
            }
            let rank = scenario_ranks[scenario_idx][finalist_idx];
            rank_sum += rank;
            let drift = rank.abs_diff(base_ranks[finalist_idx]);
            if drift > max_rank_drift {
                max_rank_drift = drift;
            }
        }
        out.push(Some(FrictionStressStats {
            scenario_count,
            pass_count,
            pass_frac: pass_count as f64 / scenario_count as f64,
            worst_scenario_label: worst_label,
            worst_pnl,
            worst_sharpe,
            worst_downside_deviation,
            worst_ulcer_index,
            worst_martin_ratio,
            worst_max_dd,
            exact_fee_stress_pnl: None,
            mean_rank: rank_sum as f64 / scenario_count as f64,
            max_rank_drift,
        }));
    }
    Ok(out)
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

    struct TestEnvGuard {
        _env: std::sync::MutexGuard<'static, ()>,
        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        _gpu: crate::gpu_backtest_kernel::GpuTestLockGuard,
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn lock_env() -> TestEnvGuard {
        let env = crate::test_support::lock_test_env();
        let gpu = crate::gpu_backtest_kernel::lock_gpu_test();
        crate::gpu_backtest_kernel::reset_test_runtime();
        crate::dynamic_indicator_adapter::reset_test_cuda_input_cache();
        TestEnvGuard {
            _env: env,
            _gpu: gpu,
        }
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    fn lock_env() -> TestEnvGuard {
        let env = crate::test_support::lock_test_env();
        TestEnvGuard { _env: env }
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
            let base = 100.0 + i as f64 * 0.1;
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.25);
            vol.push(1_000.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    fn make_strong_trending_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
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

    fn mkcs_validation_test_candidate(fast_len: u16, slow_len: u16) -> CompositeParams {
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    fn asr_validation_test_candidate(fast_len: u16, slow_len: u16) -> CompositeParams {
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
            asr: Some(AsrStrategyParams::default()),
            vcc: None,
            vemr: None,
            rocb: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    fn rocb_validation_test_candidate(fast_len: u16, slow_len: u16) -> CompositeParams {
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
            vcc: None,
            vemr: None,
            rocb: Some(RocbStrategyParams::default()),
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    fn vemr_validation_test_candidate(fast_len: u16, slow_len: u16) -> CompositeParams {
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
            vcc: None,
            vemr: Some(VemrStrategyParams::default()),
            rocb: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    fn base_composite_validation_test_candidate(
        fast_len: u16,
        slow_len: u16,
        halftrend: HalfTrendRegimeFilter,
    ) -> CompositeParams {
        CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len,
            slow_len,
            halftrend,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 50,
                stationary_mean_block_len: None,
            }),
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
    fn composite_validation_populates_regime_slice_robustness_for_top_finalists() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = CompositeValidationRequest {
            data_id: "composite-regime-robustness".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                mkcs_validation_test_candidate(8, 24),
                mkcs_validation_test_candidate(12, 36),
            ],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 1500,
                step_bars: 750,
            }),
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

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        assert_eq!(res.candidates.len(), 2);
        for cand in &res.candidates {
            let regime = cand
                .regime_slice_robustness
                .as_ref()
                .expect("top finalists should have regime robustness stats");
            assert!(regime.vol_window >= DEFAULT_COMPOSITE_REGIME_SLICE_VOL_WINDOW);
            assert!(regime.stable_bucket_count <= 5);
            assert!((0.0..=1.0).contains(&regime.stable_bucket_frac));
            assert!(!regime.worst_bucket_label.is_empty());
            assert!(regime.worst_sharpe.is_finite());
            assert!(regime.worst_max_dd.is_finite());
            assert!(!regime.support_label.is_empty());
        }
    }

    #[test]
    fn composite_validation_populates_friction_stress_for_top_finalists() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = CompositeValidationRequest {
            data_id: "composite-friction-stress".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                mkcs_validation_test_candidate(8, 24),
                mkcs_validation_test_candidate(12, 36),
            ],
            walk_forward: None,
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

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        assert_eq!(res.candidates.len(), 2);
        for cand in &res.candidates {
            let stress = cand
                .friction_stress
                .as_ref()
                .expect("top finalists should have friction stress stats");
            assert_eq!(stress.scenario_count, 4);
            assert!(stress.pass_count <= stress.scenario_count);
            assert!((0.0..=1.0).contains(&stress.pass_frac));
            assert!(!stress.worst_scenario_label.is_empty());
            assert!(stress.worst_pnl.is_finite());
            assert!(stress.worst_sharpe.is_finite());
            assert!(stress.worst_max_dd.is_finite());
            assert!(stress.mean_rank.is_finite());
        }
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "backend_used={:?} route={:?} fallback={:?}",
            res.backend_used,
            res.backend_route,
            res.backend_fallback_reason
        );
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "backend_used={:?} route={:?} fallback={:?}",
            res.backend_used,
            res.backend_route,
            res.backend_fallback_reason
        );
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
        assert_eq!(res.candidates.len(), 1);
        assert!(res.candidates[0].walk_forward.is_some());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_validation_auto_keeps_gpu_route_when_uniform() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let halftrend = HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        };
        let req = CompositeValidationRequest {
            data_id: "base-composite-auto-gpu-policy".to_string(),
            backend: Backend::Auto { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                base_composite_validation_test_candidate(10, 30, halftrend.clone()),
                base_composite_validation_test_candidate(14, 42, halftrend),
            ],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 301,
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
            .expect("base composite auto gpu policy validation");
        if !matches!(res.backend_used, BackendUsed::Gpu) {
            panic!(
                "backend_used={:?} route={:?} fallback={:?}",
                res.backend_used, res.backend_route, res.backend_fallback_reason
            );
        }
        assert_eq!(res.backend_route.as_deref(), Some("Auto->GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_validation_auto_falls_back_to_cpu_when_halftrend_differs() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "base-composite-auto-cpu-policy".to_string(),
            backend: Backend::Auto { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                base_composite_validation_test_candidate(
                    10,
                    30,
                    HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                ),
                base_composite_validation_test_candidate(
                    14,
                    42,
                    HalfTrendRegimeFilter {
                        amplitude: 3,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                ),
            ],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 302,
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
            .expect("base composite auto cpu policy validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("Auto->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("")
            .contains("uniform HalfTrend parameters"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_validation_gpu_kernel_falls_back_to_cpu_when_halftrend_differs() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "base-composite-gpu-fallback-halftrend".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                base_composite_validation_test_candidate(
                    10,
                    30,
                    HalfTrendRegimeFilter {
                        amplitude: 2,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                ),
                base_composite_validation_test_candidate(
                    14,
                    42,
                    HalfTrendRegimeFilter {
                        amplitude: 3,
                        channel_deviation: 2.0,
                        atr_period: 100,
                    },
                ),
            ],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 303,
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
            .expect("base composite gpu fallback validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("")
            .contains("uniform HalfTrend parameters"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_validation_auto_falls_back_to_cpu_when_signal_mode_is_not_hard_gate() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let halftrend = HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        };
        let mut first = base_composite_validation_test_candidate(10, 30, halftrend.clone());
        let mut second = base_composite_validation_test_candidate(14, 42, halftrend);
        first.signal_mode = CompositeSignalMode::EntryCoreExitOverlay;
        second.signal_mode = CompositeSignalMode::EntryCoreExitOverlay;

        let req = CompositeValidationRequest {
            data_id: "base-composite-auto-cpu-policy-soft-vote".to_string(),
            backend: Backend::Auto { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![first, second],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 304,
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
            .expect("base composite auto signal-mode fallback validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("Auto->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("")
            .contains("supports signal_mode=HardGate only"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_validation_gpu_kernel_falls_back_to_cpu_when_signal_mode_is_not_hard_gate() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let halftrend = HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        };
        let mut first = base_composite_validation_test_candidate(10, 30, halftrend.clone());
        let mut second = base_composite_validation_test_candidate(14, 42, halftrend);
        first.signal_mode = CompositeSignalMode::EntryCoreExitOverlay;
        second.signal_mode = CompositeSignalMode::EntryCoreExitOverlay;

        let req = CompositeValidationRequest {
            data_id: "base-composite-gpu-fallback-soft-vote".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![first, second],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 305,
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
            .expect("base composite gpu signal-mode fallback validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("")
            .contains("supports signal_mode=HardGate only"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_validation_gpu_only_falls_back_to_cpu_with_reason() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let halftrend = HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        };
        let req = CompositeValidationRequest {
            data_id: "base-composite-gpu-only-fallback".to_string(),
            backend: Backend::GpuOnly { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                base_composite_validation_test_candidate(10, 30, halftrend.clone()),
                base_composite_validation_test_candidate(14, 42, halftrend),
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
            seed: 306,
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
            .expect("base composite gpu-only fallback validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuOnly->CpuOnly"));
        assert!(
            res.backend_fallback_reason
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
                == false
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_validation_gpu_hybrid_kernel_falls_back_to_cpu_with_reason() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let halftrend = HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        };
        let req = CompositeValidationRequest {
            data_id: "base-composite-gpu-hybrid-fallback".to_string(),
            backend: Backend::GpuHybridKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                base_composite_validation_test_candidate(10, 30, halftrend.clone()),
                base_composite_validation_test_candidate(14, 42, halftrend),
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
            seed: 307,
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
            .expect("base composite gpu-hybrid fallback validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(
            res.backend_route.as_deref(),
            Some("GpuHybridKernel->CpuOnly")
        );
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("")
            .contains("does not support GpuHybridKernel yet"));
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "backend_used={:?} route={:?} fallback={:?}",
            gpu_res.backend_used,
            gpu_res.backend_route,
            gpu_res.backend_fallback_reason
        );
        assert_eq!(
            gpu_res.backend_route.as_deref(),
            Some("GpuKernel"),
            "backend_used={:?} route={:?} fallback={:?}",
            gpu_res.backend_used,
            gpu_res.backend_route,
            gpu_res.backend_fallback_reason
        );
        assert!(
            gpu_res.backend_fallback_reason.is_none(),
            "backend_used={:?} route={:?} fallback={:?}",
            gpu_res.backend_used,
            gpu_res.backend_route,
            gpu_res.backend_fallback_reason
        );
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 24,
                stationary_mean_block_len: None,
            }),
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
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 24,
                stationary_mean_block_len: None,
            }),
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 24,
                stationary_mean_block_len: None,
            }),
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 24,
                stationary_mean_block_len: None,
            }),
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
    fn vcet_validation_smoke_runs_and_returns_stats() {
        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let candidate = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 12,
            slow_len: 48,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: Some(VcetStrategyParams::default()),
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "vcet-test".to_string(),
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
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 24,
                stationary_mean_block_len: None,
            }),
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
            .expect("vcet validation run");
        assert_eq!(res.candidates.len(), 1);
        let c = &res.candidates[0];
        assert!(c.in_sample.sharpe.is_finite());
        assert!(c.robust_score.is_finite());
        assert!(c.walk_forward.as_ref().map(|w| w.windows).unwrap_or(0) > 0);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcet_validation_gpu_kernel_smoke_runs_and_returns_stats() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let candidate = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 12,
            slow_len: 48,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: Some(VcetStrategyParams::default()),
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "vcet-gpu-test".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
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
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 24,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 16,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 32,
                block_length: 8,
                benchmark: SpaBenchmarkKind::Flat,
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

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("vcet gpu validation run");
        assert_eq!(res.candidates.len(), 1);
        if !matches!(res.backend_used, BackendUsed::Gpu) {
            panic!(
                "backend_used={:?} route={:?} fallback={:?}",
                res.backend_used, res.backend_route, res.backend_fallback_reason
            );
        }
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        let c = &res.candidates[0];
        assert!(c.in_sample.sharpe.is_finite());
        assert!(c.robust_score.is_finite());
        assert!(c.walk_forward.as_ref().map(|w| w.windows).unwrap_or(0) > 0);
        assert!(res.spa.is_some());
        assert!(res.cscv_pbo.is_some());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcet_validation_gpu_kernel_falls_back_to_cpu_with_stop_loss_overlay() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let candidate = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 12,
            slow_len: 48,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: Some(VcetStrategyParams::default()),
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig {
                stop_loss_pct: Some(0.02),
                ..StrategyConfig::default()
            },
        };

        let req = CompositeValidationRequest {
            data_id: "vcet-gpu-fallback-test".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
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
            .expect("vcet gpu fallback validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or_default()
            .contains("without overlays/stop-loss/take-profit"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcet_validation_gpu_kernel_matches_cpu_metrics() {
        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcet-gpu-validate".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcet_test_candidate(12, 48), vcet_test_candidate(16, 64)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 18,
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
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu carm validation");
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
        assert_eq!(gpu_res.candidates.len(), 2);
        assert!(gpu_res
            .candidates
            .iter()
            .all(|cand| cand.in_sample.sharpe.is_finite()));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcet_validation_gpu_kernel_walk_forward_matches_cpu_metrics() {
        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcet-gpu-walk-forward".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcet_test_candidate(12, 48)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 600,
                step_bars: 300,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
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
        .expect("cpu vcet walk_forward validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcet walk_forward validation");

        let cpu_wf = cpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("cpu walk_forward");
        let gpu_wf = gpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("gpu walk_forward");
        assert_walk_forward_stats_match("vcet walk_forward", cpu_wf, gpu_wf, 1e-4, 1e-4);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcet_validation_gpu_kernel_cscv_matches_cpu() {
        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcet-gpu-cscv".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcet_test_candidate(12, 48), vcet_test_candidate(16, 64)],
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
            seed: 20,
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
        .expect("cpu vcet cscv validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcet cscv validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_cscv = cpu_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_res.cscv_pbo.as_ref().expect("gpu cscv");
        assert_eq!(cpu_cscv.slices, gpu_cscv.slices);
        assert_eq!(cpu_cscv.slice_bars, gpu_cscv.slice_bars);
        assert!((cpu_cscv.pbo - gpu_cscv.pbo).abs() < 1e-3);
        assert!(
            (cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs() < 1e-3,
            "vcet cscv lambda_p50 cpu={} gpu={}",
            cpu_cscv.lambda_p50,
            gpu_cscv.lambda_p50
        );
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
    fn vcet_validation_gpu_kernel_spa_matches_cpu() {
        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcet-gpu-spa".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcet_test_candidate(12, 48), vcet_test_candidate(16, 64)],
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
            seed: 21,
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
        .expect("cpu vcet spa validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcet spa validation");

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
    fn vcet_validation_gpu_kernel_reopt_walk_forward_matches_cpu() {
        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcet-gpu-reopt".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcet_test_candidate(12, 48), vcet_test_candidate(16, 64)],
            walk_forward: None,
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 600,
                step_bars: 300,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 22,
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
        .expect("cpu vcet reopt validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcet reopt validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_reopt_walk_forward_stats_match(
            "vcet reopt_walk_forward",
            cpu_stats,
            gpu_stats,
            5e-2,
            5e-2,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcet_validation_gpu_kernel_trade_bootstrap_matches_cpu() {
        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcet-gpu-bootstrap".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcet_test_candidate(12, 48)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
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
        .expect("cpu vcet bootstrap validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcet bootstrap validation");

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
    fn vcet_validation_gpu_kernel_perturb_matches_cpu() {
        assert_single_candidate_gpu_perturb_matches_cpu(
            "vcet-gpu-perturb",
            vcet_test_candidate(12, 48),
            24,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcet_validation_gpu_kernel_advanced_routes_smoke() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "vcet-gpu-advanced".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcet_test_candidate(12, 48), vcet_test_candidate(16, 64)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 600,
                step_bars: 300,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 600,
                step_bars: 300,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 25,
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
            .expect("vcet advanced gpu validation");
        assert_gpu_native_advanced_validation_surfaces(&res);
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 200,
                stationary_mean_block_len: None,
            }),
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
        assert!(mc.stationary_mean_block_len.unwrap_or(0) >= 1);
        assert!(mc.stationary_pnl_p05.unwrap_or(0.0) <= mc.stationary_pnl_p50.unwrap_or(0.0));
        assert!(mc.stationary_pnl_p50.unwrap_or(0.0) <= mc.stationary_pnl_p95.unwrap_or(0.0));
        assert!(mc.stationary_max_dd_p50.unwrap_or(0.0) <= mc.stationary_max_dd_p95.unwrap_or(0.0));
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
    fn composite_validation_cpu_full_stack_smoke_covers_current_profiles() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let halftrend = HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        };

        let mut composite_mo_first =
            base_composite_validation_test_candidate(10, 30, halftrend.clone());
        composite_mo_first.signal_mode = CompositeSignalMode::EntryCoreExitOverlay;
        let mut composite_mo_second =
            base_composite_validation_test_candidate(14, 42, halftrend.clone());
        composite_mo_second.signal_mode = CompositeSignalMode::EntryCoreExitOverlay;

        let profile_cases: [(&str, Vec<CompositeParams>); 11] = [
            (
                "CompositeCpu",
                vec![
                    base_composite_validation_test_candidate(10, 30, halftrend.clone()),
                    base_composite_validation_test_candidate(14, 42, halftrend.clone()),
                ],
            ),
            ("CompositeMo", vec![composite_mo_first, composite_mo_second]),
            (
                "Asr",
                vec![
                    asr_validation_test_candidate(8, 24),
                    asr_validation_test_candidate(12, 36),
                ],
            ),
            (
                "Vcc",
                vec![vcc_test_candidate(10, 30), vcc_test_candidate(14, 42)],
            ),
            (
                "Vemr",
                vec![
                    vemr_validation_test_candidate(8, 24),
                    vemr_validation_test_candidate(12, 36),
                ],
            ),
            (
                "Rocb",
                vec![
                    rocb_validation_test_candidate(8, 24),
                    rocb_validation_test_candidate(12, 36),
                ],
            ),
            (
                "Mkcs",
                vec![mkcs_test_candidate(10, 30), mkcs_test_candidate(14, 42)],
            ),
            (
                "Kqsr",
                vec![kqsr_test_candidate(10, 30), kqsr_test_candidate(14, 42)],
            ),
            (
                "Carm",
                vec![carm_test_candidate(10, 30), carm_test_candidate(14, 42)],
            ),
            (
                "Kvdf",
                vec![kvdf_test_candidate(10, 30), kvdf_test_candidate(14, 42)],
            ),
            (
                "Cdmc",
                vec![cdmc_test_candidate(10, 30), cdmc_test_candidate(14, 42)],
            ),
        ];

        for (label, candidates) in profile_cases {
            let req = CompositeValidationRequest {
                data_id: format!("cpu-full-stack-{label}"),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward: Some(WalkForwardConfigOptional {
                    window_bars: 800,
                    step_bars: 400,
                }),
                reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                    window_bars: 800,
                    step_bars: 400,
                    top_n_candidates: 2,
                }),
                perturb: Some(PerturbConfig {
                    iterations: 8,
                    pct_min: 0.05,
                    pct_max: 0.10,
                    flip_bool_prob: 0.1,
                }),
                trade_bootstrap: Some(TradeBootstrapConfig {
                    simulations: 64,
                    stationary_mean_block_len: Some(6),
                }),
                cross_ticker: None,
                cscv_pbo: Some(CscvPboConfig {
                    slices: 6,
                    max_combinations: 128,
                    threshold: 0.0,
                }),
                spa: Some(SpaConfigOptional {
                    slices: 6,
                    bootstrap_samples: 32,
                    block_length: 2,
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

            let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
                .unwrap_or_else(|err| panic!("{label} cpu full-stack validation failed: {err}"));

            assert!(matches!(res.backend_used, BackendUsed::Cpu), "{label}");
            assert!(
                res.backend_route
                    .as_deref()
                    .map(|route| route.contains("CpuOnly"))
                    .unwrap_or(false),
                "{label}"
            );
            assert!(res.reopt_walk_forward.is_some(), "{label}");
            assert!(res.cscv_pbo.is_some(), "{label}");
            assert!(res.spa.is_some(), "{label}");
            assert!(!res.candidates.is_empty(), "{label}");
            assert!(
                res.candidates
                    .iter()
                    .all(|cand| cand.walk_forward.is_some()),
                "{label}"
            );
            assert!(
                res.candidates.iter().all(|cand| cand.perturb.is_some()),
                "{label}"
            );
            assert!(
                res.candidates
                    .iter()
                    .all(|cand| cand.trade_bootstrap.is_some()),
                "{label}"
            );
            assert!(
                res.candidates.iter().all(|cand| cand.cscv_pbo.is_some()),
                "{label}"
            );
            assert!(
                res.candidates
                    .iter()
                    .all(|cand| cand.validation_verdict.passed
                        || !cand.validation_verdict.fail_reasons.is_empty()
                        || !cand.validation_verdict.warning_flags.is_empty()),
                "{label}"
            );
            assert!(
                res.candidates
                    .iter()
                    .take(DEFAULT_COMPOSITE_REGIME_SLICE_FINALISTS.min(res.candidates.len()))
                    .all(|cand| cand.regime_slice_robustness.is_some()),
                "{label}"
            );
            assert!(
                res.candidates
                    .iter()
                    .take(DEFAULT_COMPOSITE_FRICTION_STRESS_FINALISTS.min(res.candidates.len()))
                    .all(|cand| cand.friction_stress.is_some()),
                "{label}"
            );
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            vcet: None,
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
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
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
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
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
    fn asr_validation_gpu_kernel_falls_back_to_cpu_for_nondefault_zscore_devtype() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let mut candidate = asr_validation_test_candidate(8, 24);
        candidate.asr.as_mut().expect("asr params").z_devtype = 1;

        let req = CompositeValidationRequest {
            data_id: "asr-gpu-fallback-zscore-devtype".to_string(),
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

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("asr gpu fallback validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or_default()
            .contains("unsupported devtype 1"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_matches_cpu_metrics() {
        let _guard = lock_env();
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
            assert!(
                (cpu.in_sample.downside_deviation - gpu.in_sample.downside_deviation).abs() < 1e-4,
                "downside_deviation cpu={} gpu={}",
                cpu.in_sample.downside_deviation,
                gpu.in_sample.downside_deviation
            );
            assert!(
                (cpu.in_sample.ulcer_index - gpu.in_sample.ulcer_index).abs() < 1e-4,
                "ulcer_index cpu={} gpu={}",
                cpu.in_sample.ulcer_index,
                gpu.in_sample.ulcer_index
            );
            assert_close_metric_scaled(
                "asr in_sample",
                "martin_ratio",
                cpu.in_sample.martin_ratio,
                gpu.in_sample.martin_ratio,
                1e-4,
                1e-4,
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
    fn asr_validation_gpu_kernel_matches_cpu_metrics_with_ema_zscore() {
        let _guard = lock_env();
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "asr-gpu-validate-ema-zscore".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: {
                let mut first = asr_validation_test_candidate(8, 24);
                first.asr.as_mut().expect("asr params").z_ma_type = "ema".to_string();
                let mut second = asr_validation_test_candidate(12, 36);
                second.asr.as_mut().expect("asr params").z_ma_type = "ema".to_string();
                vec![first, second]
            },
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
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
        .expect("cpu asr ema validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu asr ema validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());

        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!(
                (cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4,
                "ema zscore pnl cpu={} gpu={}",
                cpu.in_sample.pnl,
                gpu.in_sample.pnl
            );
            assert!(
                (cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 1e-4,
                "ema zscore sharpe cpu={} gpu={}",
                cpu.in_sample.sharpe,
                gpu.in_sample.sharpe
            );
            assert!(
                (cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-4,
                "ema zscore max_dd cpu={} gpu={}",
                cpu.in_sample.max_dd,
                gpu.in_sample.max_dd
            );
            assert_eq!(
                cpu.in_sample.trades, gpu.in_sample.trades,
                "ema zscore trades cpu={} gpu={}",
                cpu.in_sample.trades, gpu.in_sample.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_walk_forward_matches_cpu_metrics() {
        let _guard = lock_env();
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
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
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
        assert_walk_forward_stats_match("asr walk_forward", cpu_wf, gpu_wf, 1e-4, 1e-4);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_cscv_matches_cpu() {
        let _guard = lock_env();
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
        let _guard = lock_env();
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "backend_used={:?} route={:?} fallback={:?}",
            gpu_res.backend_used,
            gpu_res.backend_route,
            gpu_res.backend_fallback_reason
        );
        assert_eq!(
            gpu_res.backend_route.as_deref(),
            Some("GpuKernel"),
            "backend_used={:?} route={:?} fallback={:?}",
            gpu_res.backend_used,
            gpu_res.backend_route,
            gpu_res.backend_fallback_reason
        );
        assert!(
            gpu_res.backend_fallback_reason.is_none(),
            "backend_used={:?} route={:?} fallback={:?}",
            gpu_res.backend_used,
            gpu_res.backend_route,
            gpu_res.backend_fallback_reason
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_reopt_walk_forward_matches_cpu() {
        let _guard = lock_env();
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
        assert_reopt_walk_forward_stats_match(
            "asr reopt_walk_forward",
            cpu_stats,
            gpu_stats,
            5e-2,
            5e-2,
        );
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_trade_bootstrap_matches_cpu() {
        let _guard = lock_env();
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
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
            assert!(
                (cpu.in_sample.downside_deviation - gpu.in_sample.downside_deviation).abs() < 1e-4,
                "downside_deviation cpu={} gpu={}",
                cpu.in_sample.downside_deviation,
                gpu.in_sample.downside_deviation
            );
            assert!(
                (cpu.in_sample.ulcer_index - gpu.in_sample.ulcer_index).abs() < 1e-4,
                "ulcer_index cpu={} gpu={}",
                cpu.in_sample.ulcer_index,
                gpu.in_sample.ulcer_index
            );
            assert_close_metric_scaled(
                "rocb in_sample",
                "martin_ratio",
                cpu.in_sample.martin_ratio,
                gpu.in_sample.martin_ratio,
                1e-4,
                1e-4,
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
    fn rocb_validation_gpu_kernel_matches_cpu_metrics_short_periods() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "rocb-gpu-validate-short".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 5,
                    slow_len: 12,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 5,
                    slow_len: 16,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 7,
                    slow_len: 12,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
                    macz: MaczGateParams::default(),
                    ott: OttGateParams::default(),
                    signal_mode: CompositeSignalMode::HardGate,
                    strategy: StrategyConfig::default(),
                },
                CompositeParams {
                    fast_ma_type: "ema".to_string(),
                    slow_ma_type: "ema".to_string(),
                    fast_len: 7,
                    slow_len: 16,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
        .expect("cpu rocb validation short periods");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu rocb validation short periods");

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
            assert!(
                (cpu.in_sample.downside_deviation - gpu.in_sample.downside_deviation).abs() < 1e-4,
                "downside_deviation cpu={} gpu={}",
                cpu.in_sample.downside_deviation,
                gpu.in_sample.downside_deviation
            );
            assert!(
                (cpu.in_sample.ulcer_index - gpu.in_sample.ulcer_index).abs() < 1e-4,
                "ulcer_index cpu={} gpu={}",
                cpu.in_sample.ulcer_index,
                gpu.in_sample.ulcer_index
            );
            assert_close_metric_scaled(
                "rocb short in_sample",
                "martin_ratio",
                cpu.in_sample.martin_ratio,
                gpu.in_sample.martin_ratio,
                1e-4,
                1e-4,
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
    fn rocb_validation_gpu_kernel_matches_cpu_metrics_small_grid() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "rocb-gpu-validate-grid".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: {
                let mut out = Vec::new();
                for fast_len in 5..=7 {
                    for slow_len in 12..=16 {
                        if fast_len >= slow_len {
                            continue;
                        }
                        out.push(CompositeParams {
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
                            kqsr: None,
                            carm: None,
                            kvdf: None,
                            cdmc: None,
                            vcet: None,
                            macz: MaczGateParams::default(),
                            ott: OttGateParams::default(),
                            signal_mode: CompositeSignalMode::HardGate,
                            strategy: StrategyConfig::default(),
                        });
                    }
                }
                out
            },
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 7,
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

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu rocb validation small grid");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu rocb validation small grid");

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
            assert!(
                (cpu.in_sample.downside_deviation - gpu.in_sample.downside_deviation).abs() < 1e-4,
                "downside_deviation cpu={} gpu={}",
                cpu.in_sample.downside_deviation,
                gpu.in_sample.downside_deviation
            );
            assert!(
                (cpu.in_sample.ulcer_index - gpu.in_sample.ulcer_index).abs() < 1e-4,
                "ulcer_index cpu={} gpu={}",
                cpu.in_sample.ulcer_index,
                gpu.in_sample.ulcer_index
            );
            assert_close_metric_scaled(
                "rocb grid in_sample",
                "martin_ratio",
                cpu.in_sample.martin_ratio,
                gpu.in_sample.martin_ratio,
                1e-4,
                1e-4,
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
    fn rocb_validation_gpu_kernel_matches_cpu_metrics_small_grid_3000_bars() {
        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "rocb-gpu-validate-grid-3000".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: {
                let mut out = Vec::new();
                for fast_len in 5..=7 {
                    for slow_len in 12..=16 {
                        if fast_len >= slow_len {
                            continue;
                        }
                        out.push(CompositeParams {
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
                            kqsr: None,
                            carm: None,
                            kvdf: None,
                            cdmc: None,
                            vcet: None,
                            macz: MaczGateParams::default(),
                            ott: OttGateParams::default(),
                            signal_mode: CompositeSignalMode::HardGate,
                            strategy: StrategyConfig::default(),
                        });
                    }
                }
                out
            },
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 7,
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

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu rocb validation small grid 3000");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu rocb validation small grid 3000");

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
            assert!(
                (cpu.in_sample.downside_deviation - gpu.in_sample.downside_deviation).abs() < 1e-4,
                "downside_deviation cpu={} gpu={}",
                cpu.in_sample.downside_deviation,
                gpu.in_sample.downside_deviation
            );
            assert!(
                (cpu.in_sample.ulcer_index - gpu.in_sample.ulcer_index).abs() < 1e-4,
                "ulcer_index cpu={} gpu={}",
                cpu.in_sample.ulcer_index,
                gpu.in_sample.ulcer_index
            );
            assert_close_metric_scaled(
                "rocb grid 3000 in_sample",
                "martin_ratio",
                cpu.in_sample.martin_ratio,
                gpu.in_sample.martin_ratio,
                1e-4,
                1e-4,
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
    fn rocb_validation_gpu_kernel_matches_cpu_metrics_small_grid_strong_trend_3000_bars() {
        let candles = make_strong_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "rocb-gpu-validate-grid-strong-3000".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: {
                let mut out = Vec::new();
                for fast_len in 5..=7 {
                    for slow_len in 12..=16 {
                        if fast_len >= slow_len {
                            continue;
                        }
                        out.push(CompositeParams {
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
                            kqsr: None,
                            carm: None,
                            kvdf: None,
                            cdmc: None,
                            vcet: None,
                            macz: MaczGateParams::default(),
                            ott: OttGateParams::default(),
                            signal_mode: CompositeSignalMode::HardGate,
                            strategy: StrategyConfig::default(),
                        });
                    }
                }
                out
            },
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 7,
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

        let cpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu rocb validation strong grid 3000");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu rocb validation strong grid 3000");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());

        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert_close_metric_scaled(
                "rocb grid strong 3000 in_sample",
                "pnl",
                cpu.in_sample.pnl,
                gpu.in_sample.pnl,
                1e-4,
                1e-5,
            );
            assert_close_metric_scaled(
                "rocb grid strong 3000 in_sample",
                "sharpe",
                cpu.in_sample.sharpe,
                gpu.in_sample.sharpe,
                1e-4,
                1e-5,
            );
            assert_close_metric_scaled(
                "rocb grid strong 3000 in_sample",
                "max_dd",
                cpu.in_sample.max_dd,
                gpu.in_sample.max_dd,
                1e-4,
                1e-5,
            );
            assert_close_metric_scaled(
                "rocb grid strong 3000 in_sample",
                "downside_deviation",
                cpu.in_sample.downside_deviation,
                gpu.in_sample.downside_deviation,
                1e-4,
                1e-5,
            );
            assert_close_metric_scaled(
                "rocb grid strong 3000 in_sample",
                "ulcer_index",
                cpu.in_sample.ulcer_index,
                gpu.in_sample.ulcer_index,
                1e-4,
                1e-5,
            );
            assert_close_metric_scaled(
                "rocb grid strong 3000 in_sample",
                "martin_ratio",
                cpu.in_sample.martin_ratio,
                gpu.in_sample.martin_ratio,
                1e-4,
                1e-4,
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
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
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

        assert_walk_forward_stats_match("rocb walk_forward", cpu_wf, gpu_wf, 1e-4, 1e-4);
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
        assert_reopt_walk_forward_stats_match(
            "rocb reopt_walk_forward",
            cpu_stats,
            gpu_stats,
            5e-2,
            5e-2,
        );
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
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
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
    fn vemr_validation_gpu_kernel_falls_back_to_cpu_with_stop_loss_overlay() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        struct ResetVemrCpuFallback;

        impl Drop for ResetVemrCpuFallback {
            fn drop(&mut self) {
                crate::vemr::set_force_vemr_cuda_cpu_fallback_for_test(false);
            }
        }

        crate::vemr::set_force_vemr_cuda_cpu_fallback_for_test(true);
        let _reset = ResetVemrCpuFallback;

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let candidate = CompositeParams {
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig {
                stop_loss_pct: Some(0.02),
                ..StrategyConfig::default()
            },
        };

        let req = CompositeValidationRequest {
            data_id: "vemr-gpu-fallback-test".to_string(),
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
            .expect("vemr gpu fallback validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or_default()
            .contains("without overlays/stop-loss/take-profit"));
        let route_diagnostics = res.route_diagnostics.unwrap_or_default();
        assert!(
            route_diagnostics.iter().any(|entry| {
                entry.contains("VEMR indicator") && entry.contains("forced test fallback")
            }),
            "expected VEMR CUDA helper fallback diagnostic, got {:?}",
            route_diagnostics
        );
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
            assert!(
                (cpu.in_sample.downside_deviation - gpu.in_sample.downside_deviation).abs() < 1e-4,
                "downside_deviation cpu={} gpu={}",
                cpu.in_sample.downside_deviation,
                gpu.in_sample.downside_deviation
            );
            assert!(
                (cpu.in_sample.ulcer_index - gpu.in_sample.ulcer_index).abs() < 1e-4,
                "ulcer_index cpu={} gpu={}",
                cpu.in_sample.ulcer_index,
                gpu.in_sample.ulcer_index
            );
            assert_close_metric_scaled(
                "vemr in_sample",
                "martin_ratio",
                cpu.in_sample.martin_ratio,
                gpu.in_sample.martin_ratio,
                1e-4,
                1e-4,
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
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
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

        assert_walk_forward_stats_match("vemr walk_forward", cpu_wf, gpu_wf, 1e-4, 1e-4);
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
        assert_reopt_walk_forward_stats_match(
            "vemr reopt_walk_forward",
            cpu_stats,
            gpu_stats,
            5e-2,
            5e-2,
        );
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
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: MaczGateParams::default(),
                ott: OttGateParams::default(),
                signal_mode: CompositeSignalMode::HardGate,
                strategy: StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
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
    fn vcc_validation_gpu_kernel_matches_cpu_metrics() {
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
            assert!(
                (cpu.in_sample.downside_deviation - gpu.in_sample.downside_deviation).abs() < 1e-4,
                "downside_deviation cpu={} gpu={}",
                cpu.in_sample.downside_deviation,
                gpu.in_sample.downside_deviation
            );
            assert!(
                (cpu.in_sample.ulcer_index - gpu.in_sample.ulcer_index).abs() < 1e-4,
                "ulcer_index cpu={} gpu={}",
                cpu.in_sample.ulcer_index,
                gpu.in_sample.ulcer_index
            );
            assert_close_metric_scaled(
                "vcc in_sample",
                "martin_ratio",
                cpu.in_sample.martin_ratio,
                gpu.in_sample.martin_ratio,
                1e-4,
                1e-4,
            );
            assert_eq!(
                cpu.in_sample.trades, gpu.in_sample.trades,
                "trades cpu={} gpu={}",
                cpu.in_sample.trades, gpu.in_sample.trades
            );
        }
    }

    #[test]
    fn vcc_validation_gpu_kernel_matches_cpu_metrics_on_roundtrip_unstable_fixture() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "vcc-gpu-validate-monotonic".to_string(),
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
                    kqsr: None,
                    carm: None,
                    kvdf: None,
                    cdmc: None,
                    vcet: None,
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
        .expect("cpu vcc monotonic validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcc monotonic validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!((cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-9);
            assert!((cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 1e-9);
            assert!((cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-9);
            assert!(
                (cpu.in_sample.downside_deviation - gpu.in_sample.downside_deviation).abs() < 1e-9
            );
            assert!((cpu.in_sample.ulcer_index - gpu.in_sample.ulcer_index).abs() < 1e-9);
            assert!((cpu.in_sample.martin_ratio - gpu.in_sample.martin_ratio).abs() < 1e-9);
            assert_eq!(cpu.in_sample.trades, gpu.in_sample.trades);
        }
    }

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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_walk_forward_matches_cpu_metrics() {
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
        assert_walk_forward_stats_match("vcc walk_forward", cpu_wf, gpu_wf, 1e-3, 1e-3);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_cscv_matches_cpu() {
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
        assert!((cpu_cscv.pbo - gpu_cscv.pbo).abs() < 1e-9);
        assert!((cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs() < 1e-9);
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            assert!((cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs() < 1e-9);
            assert!(
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs()
                    < 1e-9
            );
            assert!(
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs()
                    < 1e-9
            );
        }
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_spa_matches_cpu() {
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
        assert!((cpu_spa.best_candidate_mean_diff - gpu_spa.best_candidate_mean_diff).abs() < 1e-9);
        assert!((cpu_spa.best_candidate_t_stat - gpu_spa.best_candidate_t_stat).abs() < 1e-9);
        assert!((cpu_spa.p_value - gpu_spa.p_value).abs() < 1e-9);
        assert_eq!(cpu_spa.passed, gpu_spa.passed);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_reopt_walk_forward_matches_cpu() {
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
        assert_reopt_walk_forward_stats_match(
            "vcc reopt_walk_forward",
            cpu_stats,
            gpu_stats,
            5e-2,
            5e-2,
        );
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_trade_bootstrap_matches_cpu() {
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
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
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
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    fn kqsr_test_candidate(fast_len: u16, slow_len: u16) -> CompositeParams {
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
            vcc: None,
            vemr: None,
            rocb: None,
            mkcs: None,
            kqsr: Some(KqsrStrategyParams::default()),
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    fn vcet_test_candidate(fast_len: u16, slow_len: u16) -> CompositeParams {
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: Some(VcetStrategyParams::default()),
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    fn carm_test_candidate(fast_len: u16, slow_len: u16) -> CompositeParams {
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
            vcc: None,
            vemr: None,
            rocb: None,
            mkcs: None,
            kqsr: None,
            carm: Some(CarmStrategyParams::default()),
            kvdf: None,
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    fn kvdf_test_candidate(fast_len: u16, slow_len: u16) -> CompositeParams {
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
            vcc: None,
            vemr: None,
            rocb: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: Some(KvdfStrategyParams::default()),
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    fn cdmc_test_candidate(fast_len: u16, slow_len: u16) -> CompositeParams {
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
            vcc: None,
            vemr: None,
            rocb: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: Some(CdmcStrategyParams::default()),
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[allow(dead_code)]
    fn assert_gpu_in_sample_matches_cpu(
        req: impl Fn(Backend) -> CompositeValidationRequest,
        candles: &vector_ta::utilities::data_loader::Candles,
        cancel: &AtomicBool,
    ) {
        let cpu_res = run_composite_validation_blocking_with_candles(
            req(Backend::CpuOnly),
            candles,
            cancel,
            None,
        )
        .expect("cpu validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            req(Backend::GpuKernel { device_id: 0 }),
            candles,
            cancel,
            None,
        )
        .expect("gpu validation");

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "backend_used={:?} route={:?} fallback={:?}",
            gpu_res.backend_used,
            gpu_res.backend_route,
            gpu_res.backend_fallback_reason
        );
        assert_eq!(
            gpu_res.backend_route.as_deref(),
            Some("GpuKernel"),
            "backend_used={:?} route={:?} fallback={:?}",
            gpu_res.backend_used,
            gpu_res.backend_route,
            gpu_res.backend_fallback_reason
        );
        assert!(
            gpu_res.backend_fallback_reason.is_none(),
            "backend_used={:?} route={:?} fallback={:?}",
            gpu_res.backend_used,
            gpu_res.backend_route,
            gpu_res.backend_fallback_reason
        );
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!((cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-2);
            assert!((cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 1e-2);
            assert!((cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-2);
            assert!(
                (cpu.in_sample.downside_deviation - gpu.in_sample.downside_deviation).abs() < 1e-2
            );
            assert!((cpu.in_sample.ulcer_index - gpu.in_sample.ulcer_index).abs() < 1e-2);
            assert!((cpu.in_sample.martin_ratio - gpu.in_sample.martin_ratio).abs() < 1e-2);
            assert_eq!(cpu.in_sample.trades, gpu.in_sample.trades);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn assert_close_metric(label: &str, metric: &str, cpu: f64, gpu: f64, tol: f64) {
        assert!(
            (cpu - gpu).abs() < tol,
            "{label} {metric} cpu={cpu} gpu={gpu} tol={tol}"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn assert_close_metric_scaled(
        label: &str,
        metric: &str,
        cpu: f64,
        gpu: f64,
        abs_tol: f64,
        rel_tol: f64,
    ) {
        let scale = cpu.abs().max(gpu.abs()).max(1.0);
        let tol = abs_tol.max(rel_tol * scale);
        assert!(
            (cpu - gpu).abs() < tol,
            "{label} {metric} cpu={cpu} gpu={gpu} tol={tol} (abs_tol={abs_tol} rel_tol={rel_tol})"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn assert_walk_forward_stats_match(
        label: &str,
        cpu_wf: &WalkForwardStats,
        gpu_wf: &WalkForwardStats,
        tol: f64,
        martin_tol: f64,
    ) {
        assert_eq!(cpu_wf.windows, gpu_wf.windows, "{label} windows");
        assert_close_metric(
            label,
            "sharpe_median",
            cpu_wf.sharpe_median,
            gpu_wf.sharpe_median,
            tol,
        );
        assert_close_metric(
            label,
            "sharpe_worst",
            cpu_wf.sharpe_worst,
            gpu_wf.sharpe_worst,
            tol,
        );
        assert_close_metric(
            label,
            "max_dd_median",
            cpu_wf.max_dd_median,
            gpu_wf.max_dd_median,
            tol,
        );
        assert_close_metric(
            label,
            "max_dd_worst",
            cpu_wf.max_dd_worst,
            gpu_wf.max_dd_worst,
            tol,
        );
        assert_close_metric(
            label,
            "pnl_median",
            cpu_wf.pnl_median,
            gpu_wf.pnl_median,
            tol,
        );
        assert_close_metric(label, "pnl_worst", cpu_wf.pnl_worst, gpu_wf.pnl_worst, tol);
        assert_close_metric(
            label,
            "ulcer_index_median",
            cpu_wf.ulcer_index_median,
            gpu_wf.ulcer_index_median,
            tol,
        );
        assert_close_metric(
            label,
            "ulcer_index_worst",
            cpu_wf.ulcer_index_worst,
            gpu_wf.ulcer_index_worst,
            tol,
        );
        assert_close_metric_scaled(
            label,
            "martin_ratio_median",
            cpu_wf.martin_ratio_median,
            gpu_wf.martin_ratio_median,
            martin_tol,
            martin_tol,
        );
        assert_close_metric_scaled(
            label,
            "martin_ratio_worst",
            cpu_wf.martin_ratio_worst,
            gpu_wf.martin_ratio_worst,
            martin_tol,
            martin_tol,
        );
        assert_eq!(
            cpu_wf.trades_median, gpu_wf.trades_median,
            "{label} trades_median"
        );
        assert_eq!(
            cpu_wf.trades_worst, gpu_wf.trades_worst,
            "{label} trades_worst"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn assert_reopt_walk_forward_stats_match(
        label: &str,
        cpu_stats: &ReoptWalkForwardStats,
        gpu_stats: &ReoptWalkForwardStats,
        tol: f64,
        martin_tol: f64,
    ) {
        assert_eq!(cpu_stats.windows, gpu_stats.windows, "{label} windows");
        assert_eq!(
            cpu_stats.top_n_candidates, gpu_stats.top_n_candidates,
            "{label} top_n_candidates"
        );
        assert_eq!(
            cpu_stats.unique_winners, gpu_stats.unique_winners,
            "{label} unique_winners"
        );
        assert_eq!(
            cpu_stats.winner_changes, gpu_stats.winner_changes,
            "{label} winner_changes"
        );
        assert_eq!(
            cpu_stats.winner_sequence, gpu_stats.winner_sequence,
            "{label} winner_sequence"
        );
        assert_close_metric(
            label,
            "dominant_winner_frac",
            cpu_stats.dominant_winner_frac,
            gpu_stats.dominant_winner_frac,
            1e-6,
        );
        assert_close_metric(
            label,
            "winner_param_drift_mean",
            cpu_stats.winner_param_drift_mean,
            gpu_stats.winner_param_drift_mean,
            1e-6,
        );
        assert_close_metric(
            label,
            "winner_param_drift_max",
            cpu_stats.winner_param_drift_max,
            gpu_stats.winner_param_drift_max,
            1e-6,
        );
        assert_close_metric(
            label,
            "train_score_median",
            cpu_stats.train_score_median,
            gpu_stats.train_score_median,
            tol,
        );
        assert_close_metric(
            label,
            "forward_positive_pnl_frac",
            cpu_stats.forward_positive_pnl_frac,
            gpu_stats.forward_positive_pnl_frac,
            1e-6,
        );
        assert_close_metric(
            label,
            "forward_sharpe_median",
            cpu_stats.forward_sharpe_median,
            gpu_stats.forward_sharpe_median,
            tol,
        );
        assert_close_metric(
            label,
            "forward_sharpe_worst",
            cpu_stats.forward_sharpe_worst,
            gpu_stats.forward_sharpe_worst,
            tol,
        );
        assert_close_metric(
            label,
            "forward_max_dd_median",
            cpu_stats.forward_max_dd_median,
            gpu_stats.forward_max_dd_median,
            tol,
        );
        assert_close_metric(
            label,
            "forward_max_dd_worst",
            cpu_stats.forward_max_dd_worst,
            gpu_stats.forward_max_dd_worst,
            tol,
        );
        assert_close_metric(
            label,
            "forward_pnl_median",
            cpu_stats.forward_pnl_median,
            gpu_stats.forward_pnl_median,
            tol,
        );
        assert_close_metric(
            label,
            "forward_pnl_worst",
            cpu_stats.forward_pnl_worst,
            gpu_stats.forward_pnl_worst,
            tol,
        );
        assert_close_metric(
            label,
            "forward_ulcer_index_median",
            cpu_stats.forward_ulcer_index_median,
            gpu_stats.forward_ulcer_index_median,
            tol,
        );
        assert_close_metric(
            label,
            "forward_ulcer_index_worst",
            cpu_stats.forward_ulcer_index_worst,
            gpu_stats.forward_ulcer_index_worst,
            tol,
        );
        assert_close_metric_scaled(
            label,
            "forward_martin_ratio_median",
            cpu_stats.forward_martin_ratio_median,
            gpu_stats.forward_martin_ratio_median,
            martin_tol,
            martin_tol,
        );
        assert_close_metric_scaled(
            label,
            "forward_martin_ratio_worst",
            cpu_stats.forward_martin_ratio_worst,
            gpu_stats.forward_martin_ratio_worst,
            martin_tol,
            martin_tol,
        );
        assert_eq!(
            cpu_stats.forward_trades_median, gpu_stats.forward_trades_median,
            "{label} forward_trades_median"
        );
        assert_eq!(
            cpu_stats.forward_trades_worst, gpu_stats.forward_trades_worst,
            "{label} forward_trades_worst"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn assert_gpu_native_advanced_validation_surfaces(res: &CompositeValidationResult) {
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "backend_used={:?} route={:?} fallback={:?}",
            res.backend_used,
            res.backend_route,
            res.backend_fallback_reason
        );
        assert_eq!(
            res.backend_route.as_deref(),
            Some("GpuKernel"),
            "backend_used={:?} route={:?} fallback={:?}",
            res.backend_used,
            res.backend_route,
            res.backend_fallback_reason
        );
        assert!(res.backend_fallback_reason.is_none());
        assert!(res.cscv_pbo.is_some());
        assert!(res.spa.is_some());
        let reopt = res.reopt_walk_forward.as_ref().expect("reopt stats");
        assert!(reopt.windows > 0);
        assert!(reopt.unique_winners > 0);
        assert_eq!(res.candidates.len(), 2);
        assert!(res.candidates.iter().all(|cand| cand
            .walk_forward
            .as_ref()
            .map(|w| w.windows)
            .unwrap_or(0)
            > 0));
        assert!(res
            .candidates
            .iter()
            .all(|cand| cand.trade_bootstrap.is_some()));
        assert!(res.candidates.iter().all(|cand| cand
            .perturb
            .as_ref()
            .map(|stats| stats.iterations)
            .unwrap_or(0)
            > 0));
        assert!(res.candidates.iter().all(|cand| cand.cscv_pbo.is_some()));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn assert_single_candidate_gpu_perturb_matches_cpu(
        data_id: &str,
        candidate: CompositeParams,
        seed: u64,
    ) {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: data_id.to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![candidate.clone()],
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
            seed,
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
        .expect("cpu perturb validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu perturb validation");

        let cpu_pt = cpu_res.candidates[0].perturb.as_ref().expect("cpu perturb");
        let gpu_pt = gpu_res.candidates[0].perturb.as_ref().expect("gpu perturb");
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(cpu_pt.iterations, gpu_pt.iterations);
        assert!(
            (cpu_pt.sharpe_mean - gpu_pt.sharpe_mean).abs() < 1e-4,
            "{data_id} perturb sharpe_mean cpu={} gpu={}",
            cpu_pt.sharpe_mean,
            gpu_pt.sharpe_mean
        );
        assert!(
            (cpu_pt.sharpe_worst - gpu_pt.sharpe_worst).abs() < 1e-4,
            "{data_id} perturb sharpe_worst cpu={} gpu={}",
            cpu_pt.sharpe_worst,
            gpu_pt.sharpe_worst
        );
        assert!(
            (cpu_pt.max_dd_mean - gpu_pt.max_dd_mean).abs() < 1e-4,
            "{data_id} perturb max_dd_mean cpu={} gpu={}",
            cpu_pt.max_dd_mean,
            gpu_pt.max_dd_mean
        );
        assert!(
            (cpu_pt.max_dd_worst - gpu_pt.max_dd_worst).abs() < 1e-4,
            "{data_id} perturb max_dd_worst cpu={} gpu={}",
            cpu_pt.max_dd_worst,
            gpu_pt.max_dd_worst
        );
        assert!(
            (cpu_pt.pnl_mean - gpu_pt.pnl_mean).abs() < 1e-4,
            "{data_id} perturb pnl_mean cpu={} gpu={}",
            cpu_pt.pnl_mean,
            gpu_pt.pnl_mean
        );
        assert!(
            (cpu_pt.pnl_worst - gpu_pt.pnl_worst).abs() < 1e-4,
            "{data_id} perturb pnl_worst cpu={} gpu={}",
            cpu_pt.pnl_worst,
            gpu_pt.pnl_worst
        );
        assert!(
            (cpu_pt.pnl_positive_frac - gpu_pt.pnl_positive_frac).abs() < 1e-12,
            "{data_id} perturb pnl_positive_frac cpu={} gpu={}",
            cpu_pt.pnl_positive_frac,
            gpu_pt.pnl_positive_frac
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_advanced_routes_smoke() {
        let _guard = lock_env();
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "asr-gpu-advanced".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                asr_validation_test_candidate(8, 24),
                asr_validation_test_candidate(12, 36),
            ],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 128,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 57,
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
            .expect("asr advanced gpu validation");
        assert_gpu_native_advanced_validation_surfaces(&res);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_validation_gpu_kernel_advanced_routes_smoke_with_ema_zscore() {
        let _guard = lock_env();
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let candidates = {
            let mut first = asr_validation_test_candidate(8, 24);
            first.asr.as_mut().expect("asr params").z_ma_type = "ema".to_string();
            let mut second = asr_validation_test_candidate(12, 36);
            second.asr.as_mut().expect("asr params").z_ma_type = "ema".to_string();
            vec![first, second]
        };
        let req = CompositeValidationRequest {
            data_id: "asr-gpu-advanced-ema-zscore".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates,
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 128,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
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
        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("asr advanced gpu validation with ema zscore");
        assert_gpu_native_advanced_validation_surfaces(&res);
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn rocb_validation_gpu_kernel_advanced_routes_smoke() {
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "rocb-gpu-advanced".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                rocb_validation_test_candidate(8, 24),
                rocb_validation_test_candidate(12, 36),
            ],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 128,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 58,
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
            .expect("rocb advanced gpu validation");
        assert_gpu_native_advanced_validation_surfaces(&res);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_validation_gpu_kernel_advanced_routes_smoke() {
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "vemr-gpu-advanced".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                vemr_validation_test_candidate(8, 24),
                vemr_validation_test_candidate(12, 36),
            ],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 128,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 59,
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
            .expect("vemr advanced gpu validation");
        assert_gpu_native_advanced_validation_surfaces(&res);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_advanced_routes_smoke() {
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "vcc-gpu-advanced".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcc_test_candidate(10, 30), vcc_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 128,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 60,
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
            .expect("vcc advanced gpu validation");
        assert_gpu_native_advanced_validation_surfaces(&res);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn assert_composite_validation_outcome_parity(
        cpu_res: &CompositeValidationResult,
        gpu_res: &CompositeValidationResult,
    ) {
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());
        assert_eq!(cpu_res.run_verdict.passed, gpu_res.run_verdict.passed);
        assert_eq!(cpu_res.run_verdict.approved, gpu_res.run_verdict.approved);
        assert_eq!(
            cpu_res.run_verdict.candidates_tested,
            gpu_res.run_verdict.candidates_tested
        );
        assert_eq!(
            cpu_res.run_verdict.candidates_survived_hard_gates,
            gpu_res.run_verdict.candidates_survived_hard_gates
        );
        assert_eq!(
            cpu_res.run_verdict.approved_candidate_count,
            gpu_res.run_verdict.approved_candidate_count
        );
        assert_eq!(
            cpu_res.run_verdict.final_recommendation,
            gpu_res.run_verdict.final_recommendation
        );
        assert_eq!(
            cpu_res.run_verdict.first_approved_rank,
            gpu_res.run_verdict.first_approved_rank
        );
        assert_eq!(
            cpu_res.run_verdict.warning_flags,
            gpu_res.run_verdict.warning_flags
        );
        assert_eq!(
            cpu_res.run_verdict.fail_reasons,
            gpu_res.run_verdict.fail_reasons
        );

        let cpu_keys: Vec<Vec<u8>> = cpu_res
            .candidates
            .iter()
            .map(|cand| serialize_composite_candidate_key(&cand.params).expect("cpu candidate key"))
            .collect();
        let gpu_keys: Vec<Vec<u8>> = gpu_res
            .candidates
            .iter()
            .map(|cand| serialize_composite_candidate_key(&cand.params).expect("gpu candidate key"))
            .collect();
        assert_eq!(cpu_keys, gpu_keys);

        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert_eq!(cpu.validation_verdict.passed, gpu.validation_verdict.passed);
            assert_eq!(
                cpu.validation_verdict.warning_flags,
                gpu.validation_verdict.warning_flags
            );
            assert_eq!(
                cpu.validation_verdict.fail_reasons,
                gpu.validation_verdict.fail_reasons
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kqsr_validation_gpu_kernel_matches_cpu_metrics() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kqsr-gpu-validate".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kqsr_test_candidate(10, 30), kqsr_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: None,
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
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cdmc validation");
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
        assert_eq!(gpu_res.candidates.len(), 2);
        assert!(gpu_res
            .candidates
            .iter()
            .all(|cand| cand.in_sample.sharpe.is_finite()));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kqsr_validation_gpu_kernel_walk_forward_matches_cpu_metrics() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kqsr-gpu-walk-forward".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kqsr_test_candidate(10, 30)],
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
            seed: 58,
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
        .expect("cpu kqsr walk_forward validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu kqsr walk_forward validation");

        let cpu_wf = cpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("cpu walk_forward");
        let gpu_wf = gpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("gpu walk_forward");
        assert_walk_forward_stats_match("kqsr walk_forward", cpu_wf, gpu_wf, 1e-3, 1e-3);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kqsr_validation_gpu_kernel_cscv_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kqsr-gpu-cscv".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kqsr_test_candidate(10, 30), kqsr_test_candidate(14, 42)],
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
            seed: 63,
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
        .expect("cpu kqsr cscv validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu kqsr cscv validation");

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
    fn kqsr_validation_gpu_kernel_spa_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kqsr-gpu-spa".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kqsr_test_candidate(10, 30), kqsr_test_candidate(14, 42)],
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
            seed: 64,
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
        .expect("cpu kqsr spa validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu kqsr spa validation");

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
    fn kqsr_validation_gpu_kernel_reopt_walk_forward_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kqsr-gpu-reopt".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kqsr_test_candidate(10, 30), kqsr_test_candidate(14, 42)],
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
            seed: 65,
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
        .expect("cpu kqsr reopt validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu kqsr reopt validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_reopt_walk_forward_stats_match(
            "kqsr reopt_walk_forward",
            cpu_stats,
            gpu_stats,
            5e-2,
            5e-2,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kqsr_validation_gpu_kernel_trade_bootstrap_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kqsr-gpu-bootstrap".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kqsr_test_candidate(10, 30)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 66,
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
        .expect("cpu kqsr bootstrap validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu kqsr bootstrap validation");

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
    fn kqsr_validation_gpu_kernel_perturb_matches_cpu() {
        assert_single_candidate_gpu_perturb_matches_cpu(
            "kqsr-gpu-perturb",
            kqsr_test_candidate(10, 30),
            67,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_auto_uses_gpu_mixed_route_on_current_policy() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "vcc-auto-cpu-policy".to_string(),
            backend: Backend::Auto { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![vcc_test_candidate(8, 24), vcc_test_candidate(10, 30)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 256,
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
            .expect("vcc auto mixed-route validation");
        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert_eq!(res.backend_route.as_deref(), Some("Auto->GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_validation_auto_prefers_cpu_route_on_current_policy() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "mkcs-auto-cpu-policy".to_string(),
            backend: Backend::Auto { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mkcs_test_candidate(10, 30), mkcs_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 257,
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
            .expect("mkcs auto cpu policy validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("Auto->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("")
            .contains("Auto composite validation currently prefers CpuOnly for MKCS"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kqsr_validation_auto_prefers_cpu_route_on_current_policy() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "kqsr-auto-cpu-policy".to_string(),
            backend: Backend::Auto { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kqsr_test_candidate(10, 30), kqsr_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 258,
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
            .expect("kqsr auto cpu policy validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("Auto->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("")
            .contains("Auto composite validation currently prefers CpuOnly for KQSR"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_validation_gpu_kernel_matches_cpu_metrics() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "carm-gpu-validate".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30), carm_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 52,
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
        assert_gpu_in_sample_matches_cpu(mk, &candles, &cancel);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_validation_gpu_kernel_falls_back_to_cpu_with_stop_loss_overlay() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_noisy_trending_candles(3000);
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
            kqsr: None,
            carm: Some(CarmStrategyParams::default()),
            kvdf: None,
            cdmc: None,
            vcet: None,
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            strategy: StrategyConfig {
                stop_loss_pct: Some(0.02),
                ..StrategyConfig::default()
            },
        };

        let req = CompositeValidationRequest {
            data_id: "carm-gpu-fallback-test".to_string(),
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

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("carm gpu fallback validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or_default()
            .contains("without overlays/stop-loss/take-profit"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_validation_gpu_kernel_start_t_matches_cpu_series() {
        let candles = make_noisy_trending_candles(4096);
        let candidate = carm_test_candidate(10, 30);
        let close = vector_ta::utilities::data_loader::source_type(&candles, "close");
        let lr = compute_log_returns_f64(close);
        let prepared_cuda_inputs =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0)
                .expect("prepared cuda inputs");
        let cpu_series = build_carm_candidate_series(
            &candles,
            &lr,
            &[],
            "close",
            &candidate,
            candidate.carm.as_ref().expect("carm params"),
            Some(&prepared_cuda_inputs),
        )
        .expect("cpu carm series");

        let req = CompositeValidationRequest {
            data_id: "carm-gpu-start-t".to_string(),
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
            seed: 81,
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
        let gpu_eval =
            try_prepare_carm_gpu_fixed_candidate_validation(&req, &candles).expect("gpu carm eval");
        assert_eq!(gpu_eval.start_ts, vec![series_start_t(&cpu_series)]);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_validation_gpu_kernel_reopt_walk_forward_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "carm-gpu-reopt".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30), carm_test_candidate(14, 42)],
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
            seed: 86,
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
        .expect("cpu carm reopt validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu carm reopt validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_reopt_walk_forward_stats_match(
            "carm reopt_walk_forward",
            cpu_stats,
            gpu_stats,
            1e-4,
            1e-4,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_validation_gpu_kernel_trade_bootstrap_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "carm-gpu-bootstrap".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 91,
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
        .expect("cpu carm bootstrap validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu carm bootstrap validation");

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
    fn carm_validation_gpu_kernel_perturb_matches_cpu() {
        assert_single_candidate_gpu_perturb_matches_cpu(
            "carm-gpu-perturb",
            carm_test_candidate(10, 30),
            92,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_validation_gpu_kernel_walk_forward_matches_cpu_metrics() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "carm-gpu-walk-forward".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30)],
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
        .expect("cpu carm walk_forward validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu carm walk_forward validation");

        let cpu_wf = cpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("cpu walk_forward");
        let gpu_wf = gpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("gpu walk_forward");
        assert_walk_forward_stats_match("carm walk_forward", cpu_wf, gpu_wf, 1e-4, 1e-4);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_validation_gpu_kernel_cscv_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "carm-gpu-cscv".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30), carm_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: None,
            seed: 84,
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
        .expect("cpu carm cscv validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu carm cscv validation");

        let cpu_cscv = cpu_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_res.cscv_pbo.as_ref().expect("gpu cscv");
        assert_eq!(cpu_cscv.slices, gpu_cscv.slices);
        assert_eq!(cpu_cscv.slice_bars, gpu_cscv.slice_bars);
        assert!((cpu_cscv.pbo - gpu_cscv.pbo).abs() < 1e-9);
        assert!((cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs() < 1e-9);
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            assert!((cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs() < 1e-9);
            assert!(
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs()
                    < 1e-9
            );
            assert!(
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs()
                    < 1e-9
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_validation_gpu_kernel_spa_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "carm-gpu-spa".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30), carm_test_candidate(14, 42)],
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
            seed: 85,
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
        .expect("cpu carm spa validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu carm spa validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_spa = cpu_res.spa.as_ref().expect("cpu spa");
        let gpu_spa = gpu_res.spa.as_ref().expect("gpu spa");
        assert_eq!(cpu_spa.slice_count, gpu_spa.slice_count);
        assert_eq!(cpu_spa.bootstrap_samples, gpu_spa.bootstrap_samples);
        assert_eq!(cpu_spa.best_candidate_key, gpu_spa.best_candidate_key);
        assert!((cpu_spa.best_candidate_mean_diff - gpu_spa.best_candidate_mean_diff).abs() < 1e-9);
        assert!((cpu_spa.best_candidate_t_stat - gpu_spa.best_candidate_t_stat).abs() < 1e-9);
        assert!((cpu_spa.p_value - gpu_spa.p_value).abs() < 1e-9);
        assert_eq!(cpu_spa.passed, gpu_spa.passed);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic drift characterization"]
    fn carm_validation_gpu_kernel_advanced_route_drift_diagnostic() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let wf_req = |backend: Backend| CompositeValidationRequest {
            data_id: "carm-gpu-walk-forward-drift".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30)],
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
            seed: 183,
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
        let cpu_wf_res = run_composite_validation_blocking_with_candles(
            wf_req(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu carm walk_forward diagnostic");
        let gpu_wf_res = run_composite_validation_blocking_with_candles(
            wf_req(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu carm walk_forward diagnostic");
        let cpu_wf = cpu_wf_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("cpu wf");
        let gpu_wf = gpu_wf_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("gpu wf");
        println!(
            "CARM WF drift: windows_cpu={} windows_gpu={} trades_median_cpu={} trades_median_gpu={} trades_worst_cpu={} trades_worst_gpu={} sharpe_median={} sharpe_worst={} max_dd_median={} max_dd_worst={} pnl_median={} pnl_worst={}",
            cpu_wf.windows,
            gpu_wf.windows,
            cpu_wf.trades_median,
            gpu_wf.trades_median,
            cpu_wf.trades_worst,
            gpu_wf.trades_worst,
            (cpu_wf.sharpe_median - gpu_wf.sharpe_median).abs(),
            (cpu_wf.sharpe_worst - gpu_wf.sharpe_worst).abs(),
            (cpu_wf.max_dd_median - gpu_wf.max_dd_median).abs(),
            (cpu_wf.max_dd_worst - gpu_wf.max_dd_worst).abs(),
            (cpu_wf.pnl_median - gpu_wf.pnl_median).abs(),
            (cpu_wf.pnl_worst - gpu_wf.pnl_worst).abs(),
        );

        let cscv_req = |backend: Backend| CompositeValidationRequest {
            data_id: "carm-gpu-cscv-drift".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30), carm_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: None,
            seed: 184,
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
        let cpu_cscv_res = run_composite_validation_blocking_with_candles(
            cscv_req(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu carm cscv diagnostic");
        let gpu_cscv_res = run_composite_validation_blocking_with_candles(
            cscv_req(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu carm cscv diagnostic");
        let cpu_cscv = cpu_cscv_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_cscv_res.cscv_pbo.as_ref().expect("gpu cscv");
        println!(
            "CARM CSCV run drift: pbo={} lambda_p50={}",
            (cpu_cscv.pbo - gpu_cscv.pbo).abs(),
            (cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs(),
        );
        for (idx, (cpu, gpu)) in cpu_cscv_res
            .candidates
            .iter()
            .zip(gpu_cscv_res.candidates.iter())
            .enumerate()
        {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            println!(
                "CARM CSCV cand#{idx} drift: selected_frac={} oos_rank={} oos_metric={}",
                (cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs(),
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs(),
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs(),
            );
        }

        let spa_req = |backend: Backend| CompositeValidationRequest {
            data_id: "carm-gpu-spa-drift".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30), carm_test_candidate(14, 42)],
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
            seed: 185,
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
        let cpu_spa_res = run_composite_validation_blocking_with_candles(
            spa_req(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu carm spa diagnostic");
        let gpu_spa_res = run_composite_validation_blocking_with_candles(
            spa_req(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu carm spa diagnostic");
        let cpu_spa = cpu_spa_res.spa.as_ref().expect("cpu spa");
        let gpu_spa = gpu_spa_res.spa.as_ref().expect("gpu spa");
        println!(
            "CARM SPA drift: mean_diff={} t_stat={} p_value={} passed={} cpu_key={} gpu_key={}",
            (cpu_spa.best_candidate_mean_diff - gpu_spa.best_candidate_mean_diff).abs(),
            (cpu_spa.best_candidate_t_stat - gpu_spa.best_candidate_t_stat).abs(),
            (cpu_spa.p_value - gpu_spa.p_value).abs(),
            cpu_spa.passed == gpu_spa.passed,
            cpu_spa.best_candidate_key,
            gpu_spa.best_candidate_key,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_validation_gpu_kernel_start_t_matches_cpu_series() {
        let candles = make_noisy_trending_candles(4096);
        let candidate = cdmc_test_candidate(10, 30);
        let close = vector_ta::utilities::data_loader::source_type(&candles, "close");
        let lr = compute_log_returns_f64(close);
        let prepared_cuda_inputs =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0)
                .expect("prepared cuda inputs");
        let cpu_series = build_cdmc_candidate_series(
            &candles,
            &lr,
            &[],
            "close",
            &candidate,
            candidate.cdmc.as_ref().expect("cdmc params"),
            Some(&prepared_cuda_inputs),
        )
        .expect("cpu cdmc series");

        let req = CompositeValidationRequest {
            data_id: "cdmc-gpu-start-t".to_string(),
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
            seed: 82,
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
        let gpu_eval =
            try_prepare_cdmc_gpu_fixed_candidate_validation(&req, &candles).expect("gpu cdmc eval");
        assert_eq!(gpu_eval.start_ts, vec![series_start_t(&cpu_series)]);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kvdf_validation_gpu_kernel_matches_cpu_metrics() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kvdf-gpu-validate".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kvdf_test_candidate(10, 30), kvdf_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
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
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cdmc validation");
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
        assert_eq!(gpu_res.candidates.len(), 2);
        assert!(gpu_res
            .candidates
            .iter()
            .all(|cand| cand.in_sample.sharpe.is_finite()));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kvdf_validation_gpu_kernel_walk_forward_matches_cpu_metrics() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kvdf-gpu-walk-forward".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kvdf_test_candidate(10, 30)],
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
            seed: 60,
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
        .expect("cpu kvdf walk_forward validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu kvdf walk_forward validation");

        let cpu_wf = cpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("cpu walk_forward");
        let gpu_wf = gpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("gpu walk_forward");
        assert_walk_forward_stats_match("kvdf walk_forward", cpu_wf, gpu_wf, 1e-3, 1e-3);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kvdf_validation_gpu_kernel_cscv_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kvdf-gpu-cscv".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kvdf_test_candidate(10, 30), kvdf_test_candidate(14, 42)],
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
            seed: 68,
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
        .expect("cpu kvdf cscv validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu kvdf cscv validation");

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
    fn kvdf_validation_gpu_kernel_spa_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kvdf-gpu-spa".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kvdf_test_candidate(10, 30), kvdf_test_candidate(14, 42)],
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
            seed: 69,
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
        .expect("cpu kvdf spa validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu kvdf spa validation");

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
    fn kvdf_validation_gpu_kernel_reopt_walk_forward_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kvdf-gpu-reopt".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kvdf_test_candidate(10, 30), kvdf_test_candidate(14, 42)],
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
            seed: 70,
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
        .expect("cpu kvdf reopt validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu kvdf reopt validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_reopt_walk_forward_stats_match(
            "kvdf reopt_walk_forward",
            cpu_stats,
            gpu_stats,
            5e-2,
            5e-2,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kvdf_validation_gpu_kernel_trade_bootstrap_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "kvdf-gpu-bootstrap".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kvdf_test_candidate(10, 30)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: None,
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
        .expect("cpu kvdf bootstrap validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu kvdf bootstrap validation");

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
    fn kvdf_validation_gpu_kernel_perturb_matches_cpu() {
        assert_single_candidate_gpu_perturb_matches_cpu(
            "kvdf-gpu-perturb",
            kvdf_test_candidate(10, 30),
            72,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kvdf_validation_auto_prefers_cpu_route_on_current_policy() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "kvdf-auto-cpu-policy".to_string(),
            backend: Backend::Auto { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kvdf_test_candidate(10, 30), kvdf_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 260,
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
            .expect("kvdf auto cpu policy validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("Auto->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("")
            .contains("Auto composite validation currently prefers CpuOnly for KVDF"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_validation_gpu_kernel_matches_cpu_metrics() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "cdmc-gpu-validate".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30), cdmc_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 54,
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
        assert_gpu_in_sample_matches_cpu(mk, &candles, &cancel);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_validation_gpu_kernel_reopt_walk_forward_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "cdmc-gpu-reopt".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30), cdmc_test_candidate(14, 42)],
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
            seed: 90,
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
        .expect("cpu cdmc reopt validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cdmc reopt validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_reopt_walk_forward_stats_match(
            "cdmc reopt_walk_forward",
            cpu_stats,
            gpu_stats,
            1e-4,
            1e-4,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_validation_gpu_kernel_trade_bootstrap_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "cdmc-gpu-bootstrap".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 93,
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
        .expect("cpu cdmc bootstrap validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cdmc bootstrap validation");

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
    fn cdmc_validation_gpu_kernel_perturb_matches_cpu() {
        assert_single_candidate_gpu_perturb_matches_cpu(
            "cdmc-gpu-perturb",
            cdmc_test_candidate(10, 30),
            94,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_validation_gpu_kernel_walk_forward_matches_cpu_metrics() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "cdmc-gpu-walk-forward".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30)],
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
            seed: 87,
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
        .expect("cpu cdmc walk_forward validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cdmc walk_forward validation");

        let cpu_wf = cpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("cpu walk_forward");
        let gpu_wf = gpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("gpu walk_forward");
        assert_walk_forward_stats_match("cdmc walk_forward", cpu_wf, gpu_wf, 1e-4, 1e-4);
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_validation_gpu_kernel_cscv_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "cdmc-gpu-cscv".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30), cdmc_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: None,
            seed: 88,
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
        .expect("cpu cdmc cscv validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cdmc cscv validation");

        let cpu_cscv = cpu_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_res.cscv_pbo.as_ref().expect("gpu cscv");
        assert_eq!(cpu_cscv.slices, gpu_cscv.slices);
        assert_eq!(cpu_cscv.slice_bars, gpu_cscv.slice_bars);
        assert!((cpu_cscv.pbo - gpu_cscv.pbo).abs() < 1e-9);
        assert!((cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs() < 1e-9);
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            assert!((cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs() < 1e-9);
            assert!(
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs()
                    < 1e-9
            );
            assert!(
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs()
                    < 1e-9
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_validation_gpu_kernel_spa_matches_cpu() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "cdmc-gpu-spa".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30), cdmc_test_candidate(14, 42)],
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
            seed: 89,
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
        .expect("cpu cdmc spa validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cdmc spa validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_spa = cpu_res.spa.as_ref().expect("cpu spa");
        let gpu_spa = gpu_res.spa.as_ref().expect("gpu spa");
        assert_eq!(cpu_spa.slice_count, gpu_spa.slice_count);
        assert_eq!(cpu_spa.bootstrap_samples, gpu_spa.bootstrap_samples);
        assert_eq!(cpu_spa.best_candidate_key, gpu_spa.best_candidate_key);
        assert!((cpu_spa.best_candidate_mean_diff - gpu_spa.best_candidate_mean_diff).abs() < 1e-9);
        assert!((cpu_spa.best_candidate_t_stat - gpu_spa.best_candidate_t_stat).abs() < 1e-9);
        assert!((cpu_spa.p_value - gpu_spa.p_value).abs() < 1e-9);
        assert_eq!(cpu_spa.passed, gpu_spa.passed);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic drift characterization"]
    fn cdmc_validation_gpu_kernel_advanced_route_drift_diagnostic() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let wf_req = |backend: Backend| CompositeValidationRequest {
            data_id: "cdmc-gpu-walk-forward-drift".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30)],
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
            seed: 187,
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
        let cpu_wf_res = run_composite_validation_blocking_with_candles(
            wf_req(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu cdmc walk_forward diagnostic");
        let gpu_wf_res = run_composite_validation_blocking_with_candles(
            wf_req(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cdmc walk_forward diagnostic");
        let cpu_wf = cpu_wf_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("cpu wf");
        let gpu_wf = gpu_wf_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("gpu wf");
        println!(
            "CDMC WF drift: windows_cpu={} windows_gpu={} trades_median_cpu={} trades_median_gpu={} trades_worst_cpu={} trades_worst_gpu={} sharpe_median={} sharpe_worst={} max_dd_median={} max_dd_worst={} pnl_median={} pnl_worst={}",
            cpu_wf.windows,
            gpu_wf.windows,
            cpu_wf.trades_median,
            gpu_wf.trades_median,
            cpu_wf.trades_worst,
            gpu_wf.trades_worst,
            (cpu_wf.sharpe_median - gpu_wf.sharpe_median).abs(),
            (cpu_wf.sharpe_worst - gpu_wf.sharpe_worst).abs(),
            (cpu_wf.max_dd_median - gpu_wf.max_dd_median).abs(),
            (cpu_wf.max_dd_worst - gpu_wf.max_dd_worst).abs(),
            (cpu_wf.pnl_median - gpu_wf.pnl_median).abs(),
            (cpu_wf.pnl_worst - gpu_wf.pnl_worst).abs(),
        );

        let cscv_req = |backend: Backend| CompositeValidationRequest {
            data_id: "cdmc-gpu-cscv-drift".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30), cdmc_test_candidate(14, 42)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: None,
            seed: 188,
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
        let cpu_cscv_res = run_composite_validation_blocking_with_candles(
            cscv_req(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu cdmc cscv diagnostic");
        let gpu_cscv_res = run_composite_validation_blocking_with_candles(
            cscv_req(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cdmc cscv diagnostic");
        let cpu_cscv = cpu_cscv_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_cscv_res.cscv_pbo.as_ref().expect("gpu cscv");
        println!(
            "CDMC CSCV run drift: pbo={} lambda_p50={}",
            (cpu_cscv.pbo - gpu_cscv.pbo).abs(),
            (cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs(),
        );
        for (idx, (cpu, gpu)) in cpu_cscv_res
            .candidates
            .iter()
            .zip(gpu_cscv_res.candidates.iter())
            .enumerate()
        {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            println!(
                "CDMC CSCV cand#{idx} drift: selected_frac={} oos_rank={} oos_metric={}",
                (cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs(),
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs(),
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs(),
            );
        }

        let spa_req = |backend: Backend| CompositeValidationRequest {
            data_id: "cdmc-gpu-spa-drift".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30), cdmc_test_candidate(14, 42)],
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
            seed: 189,
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
        let cpu_spa_res = run_composite_validation_blocking_with_candles(
            spa_req(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu cdmc spa diagnostic");
        let gpu_spa_res = run_composite_validation_blocking_with_candles(
            spa_req(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cdmc spa diagnostic");
        let cpu_spa = cpu_spa_res.spa.as_ref().expect("cpu spa");
        let gpu_spa = gpu_spa_res.spa.as_ref().expect("gpu spa");
        println!(
            "CDMC SPA drift: mean_diff={} t_stat={} p_value={} passed={} cpu_key={} gpu_key={}",
            (cpu_spa.best_candidate_mean_diff - gpu_spa.best_candidate_mean_diff).abs(),
            (cpu_spa.best_candidate_t_stat - gpu_spa.best_candidate_t_stat).abs(),
            (cpu_spa.p_value - gpu_spa.p_value).abs(),
            cpu_spa.passed == gpu_spa.passed,
            cpu_spa.best_candidate_key,
            gpu_spa.best_candidate_key,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kqsr_validation_gpu_kernel_advanced_routes_smoke() {
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "kqsr-gpu-advanced".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kqsr_test_candidate(10, 30), kqsr_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 128,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
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
        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("kqsr advanced gpu validation");
        assert_gpu_native_advanced_validation_surfaces(&res);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_validation_gpu_kernel_advanced_routes_smoke() {
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "carm-gpu-advanced".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30), carm_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 128,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 62,
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
            .expect("carm advanced gpu validation");
        assert_gpu_native_advanced_validation_surfaces(&res);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_validation_gpu_kernel_advanced_route_outcome_parity_on_current_fixture() {
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "carm-gpu-advanced-outcome".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30), carm_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 128,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 162,
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
        .expect("cpu carm advanced validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu carm advanced validation");

        assert_gpu_native_advanced_validation_surfaces(&gpu_res);
        assert_composite_validation_outcome_parity(&cpu_res, &gpu_res);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_validation_auto_prefers_cpu_route_on_current_policy() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "carm-auto-cpu-policy".to_string(),
            backend: Backend::Auto { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![carm_test_candidate(10, 30), carm_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 262,
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
            .expect("carm auto cpu policy validation");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("Auto->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("")
            .contains("Auto composite validation currently prefers CpuOnly for CARM"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kvdf_validation_gpu_kernel_advanced_routes_smoke() {
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "kvdf-gpu-advanced".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![kvdf_test_candidate(10, 30), kvdf_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 128,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 63,
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
            .expect("kvdf advanced gpu validation");
        assert_gpu_native_advanced_validation_surfaces(&res);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_validation_gpu_kernel_advanced_routes_smoke() {
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "cdmc-gpu-advanced".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30), cdmc_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 128,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 64,
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
            .expect("cdmc advanced gpu validation");
        assert_gpu_native_advanced_validation_surfaces(&res);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_validation_gpu_kernel_advanced_route_outcome_parity_on_current_fixture() {
        let candles = make_noisy_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let mk = |backend: Backend| CompositeValidationRequest {
            data_id: "cdmc-gpu-advanced-outcome".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30), cdmc_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: -0.02,
                pct_max: 0.02,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 128,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 187,
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
        .expect("cpu cdmc advanced validation");
        let gpu_res = run_composite_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cdmc advanced validation");

        assert_gpu_native_advanced_validation_surfaces(&gpu_res);
        assert_composite_validation_outcome_parity(&cpu_res, &gpu_res);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_validation_auto_keeps_gpu_route_on_current_policy() {
        let candles = make_noisy_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = CompositeValidationRequest {
            data_id: "cdmc-auto-gpu-policy".to_string(),
            backend: Backend::Auto { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![cdmc_test_candidate(10, 30), cdmc_test_candidate(14, 42)],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 512,
                step_bars: 256,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 768,
                step_bars: 384,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 32,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 287,
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
            .expect("cdmc auto gpu policy validation");
        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert_eq!(res.backend_route.as_deref(), Some("Auto->GpuKernel"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("")
            .is_empty());
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
            assert!(
                (cpu.in_sample.downside_deviation - gpu.in_sample.downside_deviation).abs() < 1e-4,
                "downside_deviation cpu={} gpu={}",
                cpu.in_sample.downside_deviation,
                gpu.in_sample.downside_deviation
            );
            assert!(
                (cpu.in_sample.ulcer_index - gpu.in_sample.ulcer_index).abs() < 1e-4,
                "ulcer_index cpu={} gpu={}",
                cpu.in_sample.ulcer_index,
                gpu.in_sample.ulcer_index
            );
            assert_close_metric_scaled(
                "mkcs in_sample",
                "martin_ratio",
                cpu.in_sample.martin_ratio,
                gpu.in_sample.martin_ratio,
                1e-4,
                1e-4,
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
        assert_walk_forward_stats_match("mkcs walk_forward", cpu_wf, gpu_wf, 1e-4, 1e-4);
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
        assert_reopt_walk_forward_stats_match(
            "mkcs reopt_walk_forward",
            cpu_stats,
            gpu_stats,
            5e-2,
            5e-2,
        );
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
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
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
    fn rocb_validation_gpu_kernel_perturb_matches_cpu() {
        assert_single_candidate_gpu_perturb_matches_cpu(
            "rocb-gpu-perturb",
            rocb_validation_test_candidate(8, 24),
            57,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_validation_gpu_kernel_perturb_matches_cpu() {
        assert_single_candidate_gpu_perturb_matches_cpu(
            "vcc-gpu-perturb",
            vcc_test_candidate(10, 30),
            58,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_validation_gpu_kernel_perturb_matches_cpu() {
        assert_single_candidate_gpu_perturb_matches_cpu(
            "mkcs-gpu-perturb",
            mkcs_test_candidate(10, 30),
            59,
        );
    }
}
