use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use ta_optimizer::stream::StreamAggregator;
use ta_optimizer::{ObjectiveKind, OptimizationMode, OptimizationResult};
use ta_strategies::composite_cpu::CompositeSignalMode;
use ta_strategies::double_ma::{DoubleMaCurves, DoubleMaParams, Metrics, StrategyConfig};
#[cfg(feature = "cuda")]
use vector_ta::utilities::enums::Kernel;

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
use crate::carm::{
    build_carm_desired_positions, compute_carm_signals, compute_carm_start_t, resolve_carm_periods,
    validate_carm_params, CarmStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::carm::{
    compute_carm_mixed_signals_f32_with_attempt_state,
    compute_carm_otto_signals_f32_with_attempt_state,
    compute_carm_rocr_signal_f32_with_attempt_state, CarmGpuAttemptState,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::carm_gpu_kernel::{
    eval_carm_metrics_gpu_kernel_device_rows, CarmGpuComboBatch, CarmGpuDeviceSignalMatrices,
};
use crate::cdmc::{
    build_cdmc_desired_positions, compute_cdmc_signals, compute_cdmc_start_t, resolve_cdmc_periods,
    validate_cdmc_params, CdmcStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::cdmc::{
    compute_cdmc_correlation_cycle_output_sweep_cuda_device,
    compute_cdmc_decycler_sweep_cuda_device, compute_cdmc_dpo_sweep_cuda_device,
    compute_cdmc_mama_pair_cuda_device,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::cdmc_gpu_kernel::{
    eval_cdmc_metrics_gpu_kernel_device_rows, CdmcGpuComboBatch, CdmcGpuDeviceSignalMatrices,
};
use crate::composite_mo::CompositeGpuPrescanConfig;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::composite_profile_route::composite_profile_from_strategy;
use crate::composite_profile_route::composite_profile_gpu_target_label;
#[cfg(feature = "cuda")]
use crate::composite_profile_route::composite_profile_runtime_label;
use crate::double_ma::{
    build_indicator_bars_for_chart, compute_halftrend_trend, compute_log_returns_f64,
    cpu_tile_sizes, estimate_pairs_fast_lt_slow, expand_u16_range, first_finite_idx,
    ma_matrix_cpu_f64, ma_matrix_with_backend_f64, resolve_alma_params_for_side,
    run_double_ma_optimization_blocking_with_candles, strategy_cfg_for_optimization,
    validate_ma_kind, validate_strategy_cfg_for_optimization, Backend, BackendOptimizationResult,
    BackendUsed, DoubleMaParamsResolved, DoubleMaRequest, DoubleMaShortlistConfidenceMode,
    DoubleMaShortlistUnionMode, HalfTrendRegimeFilter, LocalStabilityPoint, LocalStabilityResult,
    LocalStabilitySummary, OptimizationModeResolved,
};
#[cfg(feature = "cuda")]
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cuda_with_inputs, DynamicIndicatorCudaComputeOutput,
    DynamicIndicatorCudaSeries,
};
#[cfg(feature = "cuda")]
use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, IndicatorParamKV, IndicatorParamValue,
};
use crate::kqsr::{
    build_kqsr_desired_positions, compute_kqsr_signals, compute_kqsr_start_t, resolve_kqsr_periods,
    validate_kqsr_params, KqsrStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::kqsr::{compute_kqsr_fast_signals_f32, compute_kqsr_slow_signals_f32};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::kqsr_gpu_kernel::{
    eval_kqsr_metrics_gpu_kernel_device_rows, KqsrGpuComboBatch, KqsrGpuDeviceSignalMatrices,
};
use crate::kvdf::{
    build_kvdf_desired_positions, compute_kvdf_dti_signal, compute_kvdf_fast_signals,
    compute_kvdf_shared_signals, compute_kvdf_signals, compute_kvdf_slow_signals,
    compute_kvdf_start_t, resolve_kvdf_periods, validate_kvdf_params, KvdfStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::kvdf_gpu_kernel::{
    eval_kvdf_metrics_gpu_kernel_device_rows, KvdfGpuComboBatch, KvdfGpuDeviceSignalMatrices,
};
use crate::matrix_index::DoubleMaFastLtSlowIndex;
use crate::mkcs::{
    build_mkcs_desired_positions, compute_mkcs_breakout_signals, compute_mkcs_compression_signals,
    compute_mkcs_start_t, resolve_mkcs_periods, validate_mkcs_params, MkcsStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::mkcs::{
    compute_mkcs_donchian_sweeps_cuda_device, compute_mkcs_stddev_sweep_cuda_device,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::mkcs_gpu_kernel::{
    eval_mkcs_metrics_gpu_kernel_device_rows, MkcsGpuComboBatch, MkcsGpuDeviceSignalMatrices,
};
use crate::prescan_predictive::{predictive_prescan_seed_pairs, PredictivePrescanParams};
use crate::progress::ProgressSink;
use crate::request_normalization::{normalize_heatmap_bins, normalize_topk_include_all};
use crate::rocb::{
    build_rocb_desired_positions_into, compute_rocb_breakout_signals, compute_rocb_risk_signals,
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
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::strategy_ir::StrategyProfileKind;
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
use crate::vcet::{
    build_vcet_desired_positions, compute_vcet_chandelier_signals, compute_vcet_start_t,
    compute_vcet_vidya_signals, resolve_vcet_periods, validate_vcet_params, VcetStrategyParams,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcet::{
    compute_vcet_chandelier_signal_cuda_device, compute_vcet_vidya_signal_cuda_device,
    VcetOwnedDeviceMatrix,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::vcet_gpu_kernel::{
    eval_vcet_metrics_gpu_kernel_device_rows, VcetGpuComboBatch, VcetGpuOwnedSignalTables,
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
#[cfg(feature = "cuda")]
use vector_ta::indicators::dispatch::CudaOutputTarget;

#[cfg(feature = "cuda")]
fn prepared_cuda_inputs_for_source_hlc(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
) -> Option<PreparedCudaIndicatorInputs> {
    PreparedCudaIndicatorInputs::from_candles_for_input_kinds(
        candles,
        ma_source,
        &[crate::indicator_catalog::IndicatorInputKind::Ohlc],
    )
    .ok()
}

#[cfg(feature = "cuda")]
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

#[cfg(feature = "cuda")]
fn prepared_cuda_inputs_for_source_ohlcv(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
) -> Option<PreparedCudaIndicatorInputs> {
    PreparedCudaIndicatorInputs::from_candles_for_input_kinds(
        candles,
        ma_source,
        &[crate::indicator_catalog::IndicatorInputKind::Ohlcv],
    )
    .ok()
}

#[cfg(feature = "cuda")]
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

#[cfg(feature = "cuda")]
fn prepared_cuda_inputs_for_source_slice(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
) -> Option<PreparedCudaIndicatorInputs> {
    PreparedCudaIndicatorInputs::from_candles_for_input_kinds(
        candles,
        ma_source,
        &[crate::indicator_catalog::IndicatorInputKind::Slice],
    )
    .ok()
}

#[cfg(feature = "cuda")]
fn prepared_cuda_inputs_for_source_slice_on_device(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    device_id: usize,
) -> Result<PreparedCudaIndicatorInputs, String> {
    PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(
        candles,
        ma_source,
        device_id,
        &[crate::indicator_catalog::IndicatorInputKind::Slice],
    )
    .map_err(|e| e.to_string())
}

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
    pub objective: ObjectiveKind,
    pub mode: OptimizationMode,
    /// Optional stage-0 prescan (double-MA optimization) to narrow MA-length ranges before running
    /// the composite grid search.
    #[serde(default)]
    pub gpu_prescan: Option<CompositeGpuPrescanConfig>,
    #[serde(default)]
    pub extra_shortlist_objectives: Option<Vec<ObjectiveKind>>,
    #[serde(default)]
    pub shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    #[serde(default)]
    pub shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
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

fn resolved_all_scores(
    all: &Option<Vec<(DoubleMaParamsResolved, Metrics)>>,
    objective: ObjectiveKind,
) -> Option<Vec<f64>> {
    all.as_ref().map(|rows| {
        rows.iter()
            .map(|(_, metrics)| objective_score(metrics, objective))
            .collect()
    })
}

fn standalone_unified_gpu_strategy_requested(req: &CompositeCpuRequest) -> bool {
    req.asr.is_some()
        || req.vcc.is_some()
        || req.vemr.is_some()
        || req.rocb.is_some()
        || req.vcet.is_some()
        || req.mkcs.is_some()
        || req.kqsr.is_some()
        || req.carm.is_some()
        || req.kvdf.is_some()
        || req.cdmc.is_some()
}

fn composite_easy_shortlist_features_requested(req: &CompositeCpuRequest) -> bool {
    req.extra_shortlist_objectives
        .as_ref()
        .is_some_and(|values| !values.is_empty())
        || req.shortlist_union_mode.is_some()
        || req.shortlist_confidence_mode.is_some()
}

fn validate_standalone_easy_shortlist_request(req: &CompositeCpuRequest) -> Result<(), String> {
    if !composite_easy_shortlist_features_requested(req) {
        return Ok(());
    }

    if !standalone_unified_gpu_strategy_requested(req) {
        return Err(
            "Standalone shortlist extras are currently supported only for the standalone unified-GPU strategy routes."
                .to_string(),
        );
    }

    if !matches!(req.backend, Backend::GpuKernel { .. }) {
        return Err(
            "Standalone shortlist extras currently require the requested backend to be GpuKernel."
                .to_string(),
        );
    }

    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);
    let extra_requested = req
        .extra_shortlist_objectives
        .as_ref()
        .is_some_and(|values| !values.is_empty());

    if extra_requested {
        if top_k == 0 {
            return Err("Standalone extra_shortlist_objectives require top_k > 0.".to_string());
        }
        if !include_all {
            return Err(
                "Standalone extra_shortlist_objectives currently require include_all=true."
                    .to_string(),
            );
        }
    }

    if req.shortlist_union_mode.is_some() && !extra_requested {
        return Err(
            "Standalone shortlist_union_mode requires extra_shortlist_objectives.".to_string(),
        );
    }

    if req.shortlist_confidence_mode.is_some() && top_k == 0 {
        return Err("Standalone shortlist_confidence_mode requires top_k > 0.".to_string());
    }

    Ok(())
}

fn standalone_shortlist_confidence_value_for_host(
    metrics: &Metrics,
    mode: DoubleMaShortlistConfidenceMode,
    obs: usize,
    peer_mean: f64,
    peer_std: f64,
    trials: usize,
) -> f64 {
    let (psr, dsr) = crate::validation_stats::compute_psr_dsr(
        metrics.sharpe,
        obs.max(2),
        trials.max(1),
        peer_mean,
        peer_std,
    );
    match mode {
        DoubleMaShortlistConfidenceMode::Psr => psr.unwrap_or(0.0),
        DoubleMaShortlistConfidenceMode::Dsr => dsr.unwrap_or(0.0),
    }
}

fn standalone_shortlist_union_objectives(
    primary_objective: ObjectiveKind,
    extra_objectives: &[ObjectiveKind],
) -> Vec<ObjectiveKind> {
    let mut objectives = Vec::with_capacity(extra_objectives.len().saturating_add(1));
    objectives.push(primary_objective);
    for &objective in extra_objectives {
        if objective != primary_objective && !objectives.contains(&objective) {
            objectives.push(objective);
        }
    }
    objectives
}

fn standalone_shortlist_union_score_rows(
    rows: &[(DoubleMaParams, Metrics)],
    objectives: &[ObjectiveKind],
) -> Vec<Vec<f64>> {
    rows.iter()
        .map(|(_, metrics)| {
            objectives
                .iter()
                .map(|&objective| objective_score(metrics, objective))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn standalone_shortlist_union_epsilon(score_rows: &[Vec<f64>]) -> Vec<f64> {
    if score_rows.is_empty() {
        return Vec::new();
    }
    let width = score_rows[0].len();
    let mut mins = vec![f64::INFINITY; width];
    let mut maxs = vec![f64::NEG_INFINITY; width];
    for scores in score_rows {
        for (idx, &score) in scores.iter().enumerate() {
            mins[idx] = mins[idx].min(score);
            maxs[idx] = maxs[idx].max(score);
        }
    }
    mins.iter()
        .zip(maxs.iter())
        .map(|(&min_score, &max_score)| {
            let span = (max_score - min_score).abs();
            let scale = min_score.abs().max(max_score.abs()).max(1.0);
            span * 0.01 + scale * 1.0e-6
        })
        .collect()
}

fn standalone_shortlist_union_dominates(a: &[f64], b: &[f64], eps: &[f64]) -> bool {
    let mut strictly_better = false;
    for idx in 0..a.len().min(b.len()).min(eps.len()) {
        if a[idx] + eps[idx] < b[idx] {
            return false;
        }
        if a[idx] > b[idx] + eps[idx] {
            strictly_better = true;
        }
    }
    strictly_better
}

fn standalone_shortlist_union_front_ranks(score_rows: &[Vec<f64>], eps: &[f64]) -> Vec<usize> {
    let n = score_rows.len();
    let mut dominates: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut dominated_by_count: Vec<usize> = vec![0; n];
    let mut front_ranks: Vec<usize> = vec![usize::MAX; n];
    let mut current_front: Vec<usize> = Vec::new();

    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            if standalone_shortlist_union_dominates(&score_rows[i], &score_rows[j], eps) {
                dominates[i].push(j);
            } else if standalone_shortlist_union_dominates(&score_rows[j], &score_rows[i], eps) {
                dominated_by_count[i] = dominated_by_count[i].saturating_add(1);
            }
        }
        if dominated_by_count[i] == 0 {
            front_ranks[i] = 0;
            current_front.push(i);
        }
    }

    let mut depth = 0usize;
    while !current_front.is_empty() {
        let mut next_front: Vec<usize> = Vec::new();
        for &idx in &current_front {
            for &dominated_idx in &dominates[idx] {
                dominated_by_count[dominated_idx] =
                    dominated_by_count[dominated_idx].saturating_sub(1);
                if dominated_by_count[dominated_idx] == 0
                    && front_ranks[dominated_idx] == usize::MAX
                {
                    front_ranks[dominated_idx] = depth.saturating_add(1);
                    next_front.push(dominated_idx);
                }
            }
        }
        depth = depth.saturating_add(1);
        current_front = next_front;
    }

    for rank in &mut front_ranks {
        if *rank == usize::MAX {
            *rank = depth;
        }
    }
    front_ranks
}

fn standalone_shortlist_union_rank_rows(score_rows: &[Vec<f64>]) -> Vec<Vec<usize>> {
    if score_rows.is_empty() {
        return Vec::new();
    }
    let height = score_rows.len();
    let width = score_rows[0].len();
    let mut ranks = vec![vec![0usize; width]; height];
    for col in 0..width {
        let mut ordered: Vec<(usize, f64)> = score_rows
            .iter()
            .enumerate()
            .map(|(row_idx, scores)| (row_idx, scores.get(col).copied().unwrap_or(0.0)))
            .collect();
        ordered.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let mut current_rank = 0usize;
        let mut previous_score: Option<f64> = None;
        for (position, (row_idx, score)) in ordered.into_iter().enumerate() {
            if let Some(previous) = previous_score {
                if score.total_cmp(&previous) != std::cmp::Ordering::Equal {
                    current_rank = position;
                }
            }
            ranks[row_idx][col] = current_rank;
            previous_score = Some(score);
        }
    }
    ranks
}

fn standalone_shortlist_union_weighted_rank_penalties(score_rows: &[Vec<f64>]) -> Vec<f64> {
    if score_rows.is_empty() {
        return Vec::new();
    }
    let rank_rows = standalone_shortlist_union_rank_rows(score_rows);
    rank_rows
        .into_iter()
        .map(|row_ranks| {
            row_ranks
                .iter()
                .enumerate()
                .map(|(idx, &rank)| rank as f64 / (idx as f64 + 1.0))
                .sum::<f64>()
        })
        .collect()
}

fn standalone_rerank_shortlist_by_union_mode(
    rows: &[(DoubleMaParams, Metrics)],
    primary_objective: ObjectiveKind,
    extra_objectives: &[ObjectiveKind],
    mode: DoubleMaShortlistUnionMode,
) -> Vec<(DoubleMaParams, Metrics)> {
    if rows.is_empty() {
        return Vec::new();
    }

    let objectives = standalone_shortlist_union_objectives(primary_objective, extra_objectives);
    if objectives.len() <= 1 {
        return rows.to_vec();
    }

    let score_rows = standalone_shortlist_union_score_rows(rows, objectives.as_slice());
    let eps = if matches!(mode, DoubleMaShortlistUnionMode::EpsilonFront) {
        standalone_shortlist_union_epsilon(&score_rows)
    } else {
        vec![0.0; objectives.len()]
    };
    let front_ranks = if matches!(
        mode,
        DoubleMaShortlistUnionMode::Pareto | DoubleMaShortlistUnionMode::EpsilonFront
    ) {
        standalone_shortlist_union_front_ranks(&score_rows, &eps)
    } else {
        vec![0usize; rows.len()]
    };
    let weighted_rank_penalties = if matches!(mode, DoubleMaShortlistUnionMode::WeightedRank) {
        standalone_shortlist_union_weighted_rank_penalties(&score_rows)
    } else {
        vec![0.0; rows.len()]
    };

    let mut ranked: Vec<(DoubleMaParams, Metrics, Vec<f64>, usize, f64)> = rows
        .iter()
        .cloned()
        .zip(score_rows)
        .zip(front_ranks)
        .zip(weighted_rank_penalties)
        .map(
            |((((params, metrics), scores), front_rank), weighted_rank_penalty)| {
                (params, metrics, scores, front_rank, weighted_rank_penalty)
            },
        )
        .collect();

    ranked.sort_by(|a, b| {
        if matches!(mode, DoubleMaShortlistUnionMode::WeightedRank) {
            let weighted_rank_cmp = a.4.total_cmp(&b.4);
            if weighted_rank_cmp != std::cmp::Ordering::Equal {
                return weighted_rank_cmp;
            }
        }
        if !matches!(mode, DoubleMaShortlistUnionMode::Lexicographic) {
            let front_cmp = a.3.cmp(&b.3);
            if front_cmp != std::cmp::Ordering::Equal {
                return front_cmp;
            }
        }
        for idx in 0..a.2.len().min(b.2.len()) {
            let cmp = b.2[idx].total_cmp(&a.2[idx]);
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        }
        let primary_cmp = objective_score(&b.1, primary_objective)
            .total_cmp(&objective_score(&a.1, primary_objective));
        if primary_cmp != std::cmp::Ordering::Equal {
            return primary_cmp;
        }
        a.1.max_dd.total_cmp(&b.1.max_dd)
    });

    ranked
        .into_iter()
        .map(|(params, metrics, _, _, _)| (params, metrics))
        .collect()
}

fn apply_requested_standalone_union_shortlist(
    result: &mut OptimizationResult,
    req: &CompositeCpuRequest,
    objective: ObjectiveKind,
) -> Result<(), String> {
    let extra_objectives = match req
        .extra_shortlist_objectives
        .as_deref()
        .filter(|values| !values.is_empty())
    {
        Some(values) => values,
        None => return Ok(()),
    };

    let top_limit = result.top.len().max(1);
    let all_rows = result.all.as_ref().ok_or_else(|| {
        "Standalone extra_shortlist_objectives currently require include_all=true.".to_string()
    })?;
    let shortlist_rows = crate::double_ma::build_exact_extra_shortlist_rows(
        all_rows.as_slice(),
        objective,
        extra_objectives,
        top_limit,
    );
    if shortlist_rows.is_empty() {
        return Err("standalone extra_shortlist_objectives produced no shortlist rows".to_string());
    }

    let reranked = if let Some(mode) = req.shortlist_union_mode {
        standalone_rerank_shortlist_by_union_mode(
            shortlist_rows.as_slice(),
            objective,
            extra_objectives,
            mode,
        )
    } else {
        shortlist_rows
    };

    let (best_params, best_metrics) = reranked
        .first()
        .cloned()
        .ok_or_else(|| "standalone shortlist rerank produced no result".to_string())?;
    result.best_params = best_params;
    result.best_metrics = best_metrics;
    result.best_score = Some(objective_score(&result.best_metrics, objective));
    result.top = reranked.into_iter().take(top_limit).collect();
    result.top_scores = Some(
        result
            .top
            .iter()
            .map(|(_, metrics)| objective_score(metrics, objective))
            .collect(),
    );
    Ok(())
}

fn finalize_standalone_optimization_result(
    agg: StreamAggregator,
    empty_err: &str,
    req: &CompositeCpuRequest,
    objective: ObjectiveKind,
) -> Result<OptimizationResult, String> {
    let mut result = agg.finalize().ok_or_else(|| empty_err.to_string())?;
    apply_requested_standalone_union_shortlist(&mut result, req, objective)?;
    Ok(result)
}

fn apply_requested_standalone_confidence_rerank<F>(
    result: &mut OptimizationResult,
    req: &CompositeCpuRequest,
    objective: ObjectiveKind,
    mut obs_bars_for: F,
) -> Result<(), String>
where
    F: FnMut(&DoubleMaParams) -> Result<usize, String>,
{
    let Some(mode) = req.shortlist_confidence_mode else {
        return Ok(());
    };
    if result.top.is_empty() {
        return Ok(());
    }

    let sharpe_peers: Vec<f64> = result
        .top
        .iter()
        .map(|(_, metrics)| metrics.sharpe)
        .filter(|value| value.is_finite())
        .collect();
    let (peer_mean, peer_std) = crate::sharpe_stats::mean_std(&sharpe_peers);
    let trials = sharpe_peers.len().max(1);

    let mut ranked: Vec<(DoubleMaParams, Metrics, f64)> = Vec::with_capacity(result.top.len());
    for (params, metrics) in result.top.iter().cloned() {
        let obs = obs_bars_for(&params)?;
        let confidence = standalone_shortlist_confidence_value_for_host(
            &metrics, mode, obs, peer_mean, peer_std, trials,
        );
        ranked.push((params, metrics, confidence));
    }

    ranked.sort_by(|a, b| {
        b.2.total_cmp(&a.2)
            .then_with(|| {
                let sa = objective_score(&a.1, objective);
                let sb = objective_score(&b.1, objective);
                sb.total_cmp(&sa)
            })
            .then_with(|| a.1.max_dd.total_cmp(&b.1.max_dd))
    });

    let (best_params, best_metrics) = ranked
        .first()
        .map(|(params, metrics, _)| (params.clone(), metrics.clone()))
        .ok_or_else(|| "standalone shortlist-confidence rerank produced no result".to_string())?;
    result.best_params = best_params;
    result.best_metrics = best_metrics;
    result.best_score = Some(objective_score(&result.best_metrics, objective));
    result.top = ranked
        .into_iter()
        .map(|(params, metrics, _)| (params, metrics))
        .collect();
    result.top_scores = Some(
        result
            .top
            .iter()
            .map(|(_, metrics)| objective_score(metrics, objective))
            .collect(),
    );
    Ok(())
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

fn composite_backend_label(backend: &Backend) -> &'static str {
    match backend {
        Backend::Auto { .. } => "Auto",
        Backend::CpuOnly => "CpuOnly",
        Backend::GpuOnly { .. } => "GpuOnly",
        Backend::GpuHybridKernel { .. } => "GpuHybridKernel",
        Backend::GpuKernel { .. } => "GpuKernel",
    }
}

#[cfg(feature = "cuda")]
fn backend_device_id(backend: &Backend) -> Option<usize> {
    match backend {
        Backend::Auto { device_id }
        | Backend::GpuOnly { device_id }
        | Backend::GpuHybridKernel { device_id }
        | Backend::GpuKernel { device_id } => Some(*device_id as usize),
        Backend::CpuOnly => None,
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn composite_gpu_kernel_route_label(requested_backend: &Backend) -> String {
    if matches!(requested_backend, Backend::Auto { .. }) {
        "Auto->GpuKernel".to_string()
    } else if matches!(requested_backend, Backend::GpuKernel { .. }) {
        "GpuKernel".to_string()
    } else {
        format!("{}->GpuKernel", composite_backend_label(requested_backend))
    }
}

#[cfg_attr(
    not(all(feature = "cuda", feature = "cuda-backtest-kernel")),
    allow(dead_code)
)]
pub(crate) fn build_base_composite_hard_gate_regime_f32(
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_prices: &[f64],
    ott_line: &[f64],
) -> Result<Vec<f32>, String> {
    let n = halftrend_trend.len();
    if macz_hist.len() != n || ott_prices.len() != n || ott_line.len() != n {
        return Err("base composite hard-gate regime input length mismatch".to_string());
    }

    let mut regime = vec![f32::NAN; n];
    for idx in 0..n {
        let ht = halftrend_trend[idx];
        let macz = macz_hist[idx];
        let ott_price = ott_prices[idx];
        let ott_gate = ott_line[idx];
        if !ht.is_finite() || !macz.is_finite() || !ott_price.is_finite() || !ott_gate.is_finite() {
            continue;
        }

        let long_ok = ht < 0.5 && macz > 0.0 && ott_price > ott_gate;
        let short_ok = ht >= 0.5 && macz < 0.0 && ott_price < ott_gate;
        regime[idx] = if long_ok {
            0.0
        } else if short_ok {
            1.0
        } else {
            f32::NAN
        };
    }

    Ok(regime)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(clippy::too_many_arguments)]
fn try_optimize_base_composite_gpu_kernel(
    req: &CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma: &str,
    slow_ma: &str,
    ma_source: &str,
    first_valid: usize,
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_prices: &[f64],
    ott_line: &[f64],
    strategy: &StrategyConfig,
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
    indicator_backend: &Backend,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    if req.signal_mode != CompositeSignalMode::HardGate {
        return Err(
            "base composite GpuKernel path currently supports signal_mode=HardGate only"
                .to_string(),
        );
    }
    if allowed_pairs.is_some() {
        return Err(
            "base composite GpuKernel path does not yet support prescan-subset allowed_pairs"
                .to_string(),
        );
    }

    let blockers = crate::double_ma_backend::gpu_kernel_blockers_for_request(
        fast_ma,
        slow_ma,
        strategy,
        true,
        objective,
        top_k,
        include_all,
    );
    if !blockers.is_empty() {
        return Err(format!(
            "base composite GpuKernel path unavailable: {}",
            blockers.join("; ")
        ));
    }

    let device_id = backend_device_id(indicator_backend)
        .ok_or_else(|| "base composite GpuKernel path requires a GPU-capable backend".to_string())?
        as u32;
    let (fast_precomputed_f64, fast_cols) = ma_matrix_cpu_f64(
        candles,
        fast_ma,
        ma_source,
        fast_periods,
        req.fast_ma_params.as_ref(),
        fast_alma_offset,
        fast_alma_sigma,
    )?;
    let (slow_precomputed_f64, slow_cols) = ma_matrix_cpu_f64(
        candles,
        slow_ma,
        ma_source,
        slow_periods,
        req.slow_ma_params.as_ref(),
        slow_alma_offset,
        slow_alma_sigma,
    )?;
    if fast_cols != slow_cols {
        return Err("base composite GpuKernel precomputed MA column mismatch".to_string());
    }
    let fast_precomputed_f32: Vec<f32> = fast_precomputed_f64
        .into_iter()
        .map(|value| value as f32)
        .collect();
    let slow_precomputed_f32: Vec<f32> = slow_precomputed_f64
        .into_iter()
        .map(|value| value as f32)
        .collect();
    let fast_precomputed = crate::gpu_backtest_kernel::PrecomputedHostMaMatrix {
        periods: fast_periods,
        cols: fast_cols,
        values_row_major: &fast_precomputed_f32,
    };
    let slow_precomputed = crate::gpu_backtest_kernel::PrecomputedHostMaMatrix {
        periods: slow_periods,
        cols: slow_cols,
        values_row_major: &slow_precomputed_f32,
    };
    let regime = build_base_composite_hard_gate_regime_f32(
        halftrend_trend,
        macz_hist,
        ott_prices,
        ott_line,
    )?;
    let device_regime =
        crate::gpu_backtest_kernel::upload_host_regime_filter(device_id, &regime, first_valid)?;

    crate::gpu_backtest_kernel::optimize_double_ma_gpu_kernel_with_precomputed_and_regime(
        candles,
        fast_periods,
        slow_periods,
        fast_ma,
        slow_ma,
        ma_source,
        req.fast_ma_params.as_ref(),
        req.slow_ma_params.as_ref(),
        Some(crate::gpu_backtest_kernel::RegimeFilterInput::DeviceF32(
            &device_regime,
        )),
        strategy,
        objective,
        None,
        None,
        None,
        None,
        None,
        None,
        top_k,
        include_all,
        crate::gpu_backtest_kernel::KernelConfig {
            device_id,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            hurdle_tau: strategy.hurdle_tau.unwrap_or(0.0),
            include_trade_metrics: false,
            metric_contract: None,
        },
        Some(&fast_precomputed),
        Some(&slow_precomputed),
        None,
        None,
        total_pairs,
        heatmap_bins,
        Some(first_valid),
        None,
        cancel,
        progress,
    )
}

#[cfg(feature = "cuda")]
fn cuda_series_to_f64(out: DynamicIndicatorCudaComputeOutput) -> Result<Vec<f64>, String> {
    match out.series {
        DynamicIndicatorCudaSeries::HostF32(v) => Ok(v.into_iter().map(|x| x as f64).collect()),
        DynamicIndicatorCudaSeries::DeviceF32(_) => {
            Err("composite expected CUDA HostF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
fn resolve_output_id(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("Composite indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.outputs.is_empty() {
        return Err(format!(
            "Composite indicator '{indicator_id}' has no declared outputs"
        ));
    }

    for preferred in preferred_ids {
        if let Some(found) = descriptor
            .outputs
            .iter()
            .find(|out| out.id.eq_ignore_ascii_case(preferred))
        {
            return Ok(found.id.clone());
        }
    }

    let preferred_lower: Vec<String> = preferred_ids
        .iter()
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .collect();
    for out in &descriptor.outputs {
        let out_id = out.id.to_ascii_lowercase();
        if preferred_lower
            .iter()
            .any(|p| out_id.contains(p) || p.contains(out_id.as_str()))
        {
            return Ok(out.id.clone());
        }
    }

    Ok(descriptor.outputs[0].id.clone())
}

#[cfg(feature = "cuda")]
fn try_compute_indicator_series_cuda_f64(
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    backend: &Backend,
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f64>, String> {
    if matches!(backend, Backend::CpuOnly) {
        return Err("CPU backend requested".to_string());
    }
    let prepared =
        prepared_cuda_inputs.ok_or_else(|| "missing prepared CUDA indicator inputs".to_string())?;
    let out = compute_dynamic_indicator_cuda_with_inputs(
        prepared,
        indicator_id,
        Some(output_id),
        params,
        Kernel::Auto,
        CudaOutputTarget::HostF32,
    )
    .map_err(|e| e.to_string())?;
    cuda_series_to_f64(out)
}

#[cfg(feature = "cuda")]
fn build_halftrend_params(filter: &HalfTrendRegimeFilter) -> Vec<IndicatorParamKV> {
    vec![
        IndicatorParamKV {
            key: "amplitude".to_string(),
            value: IndicatorParamValue::Int(filter.amplitude as i64),
        },
        IndicatorParamKV {
            key: "channel_deviation".to_string(),
            value: IndicatorParamValue::Float(filter.channel_deviation),
        },
        IndicatorParamKV {
            key: "atr_period".to_string(),
            value: IndicatorParamValue::Int(filter.atr_period as i64),
        },
    ]
}

#[cfg(feature = "cuda")]
fn build_macz_params(cfg: &MaczGateParams) -> Vec<IndicatorParamKV> {
    vec![
        IndicatorParamKV {
            key: "fast_length".to_string(),
            value: IndicatorParamValue::Int(cfg.fast_length as i64),
        },
        IndicatorParamKV {
            key: "slow_length".to_string(),
            value: IndicatorParamValue::Int(cfg.slow_length as i64),
        },
        IndicatorParamKV {
            key: "signal_length".to_string(),
            value: IndicatorParamValue::Int(cfg.signal_length as i64),
        },
        IndicatorParamKV {
            key: "lengthz".to_string(),
            value: IndicatorParamValue::Int(cfg.lengthz as i64),
        },
        IndicatorParamKV {
            key: "length_stdev".to_string(),
            value: IndicatorParamValue::Int(cfg.length_stdev as i64),
        },
        IndicatorParamKV {
            key: "a".to_string(),
            value: IndicatorParamValue::Float(cfg.a),
        },
        IndicatorParamKV {
            key: "b".to_string(),
            value: IndicatorParamValue::Float(cfg.b),
        },
        IndicatorParamKV {
            key: "use_lag".to_string(),
            value: IndicatorParamValue::Bool(cfg.use_lag),
        },
        IndicatorParamKV {
            key: "gamma".to_string(),
            value: IndicatorParamValue::Float(cfg.gamma),
        },
    ]
}

#[cfg(feature = "cuda")]
fn build_ott_params(cfg: &OttGateParams) -> Vec<IndicatorParamKV> {
    vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(cfg.period as i64),
        },
        IndicatorParamKV {
            key: "percent".to_string(),
            value: IndicatorParamValue::Float(cfg.percent),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString(cfg.ma_type.clone()),
        },
    ]
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
            let backend_label = composite_backend_label(&req.backend).to_string();
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
                        } else if matches!(req.backend, Backend::GpuKernel { .. }) {
                            gpu_res.backend_route = Some("GpuKernel".to_string());
                        } else {
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
            let backend_label = composite_backend_label(&req.backend).to_string();
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &ma_source, device_id as usize)?;
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
        close_device_ptr: prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
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
    let combo_end_t = vec![
        i32::try_from(n).map_err(|_| "ASR end_t exceeds i32 range".to_string())?;
        combo_fast_row_idx.len()
    ];
    let combos = AsrGpuComboBatch {
        fast_row_index: &combo_fast_row_idx,
        slow_row_index: &combo_slow_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "ASR grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_asr_periods(&asr_cfg, params.fast_len, params.slow_len);
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
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Gpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: Some("GpuKernel".to_string()),
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
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
        prepared_cuda_inputs_for_source_hlc(candles, &ma_source)
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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "ASR grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_asr_periods(&asr_cfg, params.fast_len, params.slow_len);
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
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Cpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
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
            let backend_label = composite_backend_label(&req.backend).to_string();
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
                        } else if matches!(req.backend, Backend::GpuKernel { .. }) {
                            gpu_res.backend_route = Some("GpuKernel".to_string());
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
            let backend_label = composite_backend_label(&req.backend).to_string();
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &ma_source, device_id as usize)?;

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
    let mut fast_squeeze_host_row_major: Vec<f32> = Vec::with_capacity(fast_rows * n);
    let mut fast_momentum_host_row_major: Vec<f32> = Vec::with_capacity(fast_rows * n);
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
        let computed = compute_vcc_fast_signals(
            candles,
            &ma_source,
            &Backend::CpuOnly,
            &vcc_cfg,
            key.0,
            key.1,
            Some(&prepared_cuda_inputs),
        )?;
        fast_squeeze_host_row_major.extend(computed.squeeze_state.iter().map(|v| *v as f32));
        fast_momentum_host_row_major.extend(computed.momentum.iter().map(|v| *v as f32));
        fast_first_valid_lookup.insert(key, computed.first_valid);
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
    let mut trend_host_row_major: Vec<f32> = Vec::with_capacity(trend_rows * n);
    for (row, key) in trend_keys.iter().copied().enumerate() {
        let tr_ptr = trend_sweep
            .row_ptr_for_period(key)
            .ok_or_else(|| format!("VCC trend row missing for period={key}"))?;
        trend_rows_ptr.push(tr_ptr);
        trend_row_lookup.insert(key, row as i32);
        let computed = compute_vcc_trend_signals(
            candles,
            &ma_source,
            &Backend::CpuOnly,
            key,
            Some(&prepared_cuda_inputs),
        )?;
        trend_host_row_major.extend(computed.trend_state.iter().map(|v| *v as f32));
        trend_first_valid_lookup.insert(key, computed.first_valid);
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
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
        cols: n,
        fast_rows,
        trend_rows,
        fast_squeeze_rows: &fast_squeeze_rows,
        fast_squeeze_host_row_major: Some(&fast_squeeze_host_row_major),
        fast_momentum_rows: &fast_momentum_rows,
        fast_momentum_host_row_major: Some(&fast_momentum_host_row_major),
        trend_rows_ptr: &trend_rows_ptr,
        trend_host_row_major: Some(&trend_host_row_major),
    };
    let combo_end_t = vec![
        i32::try_from(n).map_err(|_| "VCC end_t exceeds i32 range".to_string())?;
        combo_fast_row_idx.len()
    ];
    let combos = VccGpuComboBatch {
        fast_row_index: &combo_fast_row_idx,
        trend_row_index: &combo_trend_row_idx,
        start_t: &combo_start_t,
        metric_start_t: &combo_start_t,
        end_t: &combo_end_t,
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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "VCC grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_vcc_periods(&vcc_cfg, params.fast_len, params.slow_len);
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
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Gpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "VCC grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_vcc_periods(&vcc_cfg, params.fast_len, params.slow_len);
        let fast_signals = fast_cache
            .get(&(periods.squeeze, periods.momentum))
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
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Cpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
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
            let backend_label = composite_backend_label(&req.backend).to_string();
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
                        } else if matches!(req.backend, Backend::GpuKernel { .. }) {
                            gpu_res.backend_route = Some("GpuKernel".to_string());
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
            let backend_label = composite_backend_label(&req.backend).to_string();
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

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &ma_source, device_id as usize)?;

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
            .map_err(|_| "VEMR end_t exceeds i32 range".to_string())?;
        combo_z_row_idx.len()
    ];
    let combos = VemrGpuComboBatch {
        z_row_index: &combo_z_row_idx,
        bb_row_index: &combo_bb_row_idx,
        atr_row_index: &combo_atr_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
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
    let (best_params, best_metrics, best_score, top, top_scores, all, heatmap, num_combos) =
        match gpu_eval {
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

                let mut result = finalize_standalone_optimization_result(
                    agg,
                    "VEMR grid search produced no result",
                    &req,
                    objective,
                )?;
                apply_requested_standalone_confidence_rerank(
                    &mut result,
                    &req,
                    objective,
                    |params| {
                        let periods =
                            resolve_vemr_periods(&vemr_cfg, params.fast_len, params.slow_len);
                        let fast_first_valid = *fast_first_valid_lookup
                            .get(&(periods.z, periods.bb))
                            .ok_or_else(|| "VEMR fast first_valid lookup miss".to_string())?;
                        let risk_first_valid = *risk_first_valid_lookup
                            .get(&periods.atr)
                            .ok_or_else(|| "VEMR risk first_valid lookup miss".to_string())?;
                        let max_period =
                            usize::from(periods.z.max(periods.bb).max(periods.atr)).max(1);
                        let start_t = compute_vemr_start_t(
                            fv_close,
                            fv_ma,
                            fast_first_valid,
                            risk_first_valid,
                            max_period,
                            strategy.trade_on_next_bar,
                        );
                        Ok(n.saturating_sub(start_t).max(2))
                    },
                )?;
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
                    result.best_score,
                    top,
                    result.top_scores.clone(),
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
                let best_metrics = top_rows[0].metrics.clone();

                let mut temp_result = OptimizationResult {
                    best_params: best_combo.clone(),
                    best_metrics,
                    best_score: Some(objective_score(&top_rows[0].metrics, objective)),
                    top: Vec::with_capacity(top_k),
                    top_scores: None,
                    all: None,
                    all_scores: None,
                    heatmap: None,
                    num_combos: num_combos_total,
                    num_candles: n,
                };
                if top_k > 0 {
                    for row in top_rows.into_iter().take(top_k) {
                        let params = combo_params
                            .get(row.combo_index)
                            .ok_or_else(|| "VEMR GPU top combo index out of range".to_string())?;
                        temp_result.top.push((params.clone(), row.metrics));
                    }
                }
                temp_result.top_scores = if temp_result.top.is_empty() {
                    None
                } else {
                    Some(
                        temp_result
                            .top
                            .iter()
                            .map(|(_, metrics)| objective_score(metrics, objective))
                            .collect(),
                    )
                };
                apply_requested_standalone_confidence_rerank(
                    &mut temp_result,
                    &req,
                    objective,
                    |params| {
                        let periods =
                            resolve_vemr_periods(&vemr_cfg, params.fast_len, params.slow_len);
                        let fast_first_valid = *fast_first_valid_lookup
                            .get(&(periods.z, periods.bb))
                            .ok_or_else(|| "VEMR fast first_valid lookup miss".to_string())?;
                        let risk_first_valid = *risk_first_valid_lookup
                            .get(&periods.atr)
                            .ok_or_else(|| "VEMR risk first_valid lookup miss".to_string())?;
                        let max_period =
                            usize::from(periods.z.max(periods.bb).max(periods.atr)).max(1);
                        let start_t = compute_vemr_start_t(
                            fv_close,
                            fv_ma,
                            fast_first_valid,
                            risk_first_valid,
                            max_period,
                            strategy.trade_on_next_bar,
                        );
                        Ok(n.saturating_sub(start_t).max(2))
                    },
                )?;
                let best_params = resolve_params(&temp_result.best_params);
                let best_metrics = temp_result.best_metrics.clone();
                let top: Vec<(DoubleMaParamsResolved, Metrics)> = temp_result
                    .top
                    .iter()
                    .map(|(params, metrics)| (resolve_params(params), metrics.clone()))
                    .collect();
                emit_progress(progress, total_pairs, total_pairs, "vemr_optimize");

                (
                    best_params,
                    best_metrics,
                    temp_result.best_score,
                    top,
                    temp_result.top_scores.clone(),
                    None,
                    None,
                    num_combos_total,
                )
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
        best_score,
        top,
        top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
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
        prepared_cuda_inputs_for_source_hlc(candles, &ma_source)
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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "VEMR grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_vemr_periods(&vemr_cfg, params.fast_len, params.slow_len);
        let fast_signals = fast_cache
            .get(&(periods.z, periods.bb))
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
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_score: result.best_score,
        mode_used,
        backend_used,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
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
        prepared_cuda_inputs_for_source_hlc(candles, &req.ma_source)
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
        prepared_cuda_inputs_for_source_hlc(candles, &req.ma_source)
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
        prepared_cuda_inputs_for_source_hlc(candles, &req.ma_source)
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
        prepared_cuda_inputs_for_source_hlc(candles, &req.ma_source)
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

fn run_vcc_local_stability(
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
    let vcc_cfg = req.vcc.clone().unwrap_or_default();
    validate_vcc_params(&vcc_cfg)?;

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
        prepared_cuda_inputs_for_source_hlc(candles, &req.ma_source)
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

    let mut fast_cache: HashMap<(u16, u16), crate::vcc::VccFastSignals> = HashMap::new();
    let mut trend_cache: HashMap<u16, crate::vcc::VccTrendSignals> = HashMap::new();
    let mut max_period = 1usize;

    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            let periods = resolve_vcc_periods(&vcc_cfg, fast_len, slow_len);
            let fast_key = (periods.squeeze, periods.momentum);
            if !fast_cache.contains_key(&fast_key) {
                let computed = compute_vcc_fast_signals(
                    candles,
                    &req.ma_source,
                    &req.backend,
                    &vcc_cfg,
                    periods.squeeze,
                    periods.momentum,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                fast_cache.insert(fast_key, computed);
            }
            if !trend_cache.contains_key(&periods.trend) {
                let computed = compute_vcc_trend_signals(
                    candles,
                    &req.ma_source,
                    &req.backend,
                    periods.trend,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                trend_cache.insert(periods.trend, computed);
            }
            let cur_max =
                usize::from(periods.squeeze.max(periods.momentum).max(periods.trend)).max(1);
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
    let first_valid_trend = trend_cache
        .values()
        .map(|row| row.first_valid)
        .max()
        .unwrap_or(fv_ma);
    let start_t = compute_vcc_start_t(
        fv_close,
        fv_ma,
        first_valid_fast,
        first_valid_trend,
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
            let periods = resolve_vcc_periods(&vcc_cfg, fast_len, slow_len);
            let fast_signals = fast_cache
                .get(&(periods.squeeze, periods.momentum))
                .ok_or_else(|| "VCC fast signal cache miss".to_string())?;
            let trend_signals = trend_cache
                .get(&periods.trend)
                .ok_or_else(|| "VCC trend signal cache miss".to_string())?;

            let desired = build_vcc_desired_positions(
                close,
                fast_signals,
                trend_signals,
                start_t,
                &strategy,
                &vcc_cfg,
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

fn run_rocb_local_stability(
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
    let rocb_cfg = req.rocb.clone().unwrap_or_default();
    validate_rocb_params(&rocb_cfg)?;

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let high = vector_ta::utilities::data_loader::source_type(candles, "high");
    let low = vector_ta::utilities::data_loader::source_type(candles, "low");
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_high = first_finite_idx(high).ok_or_else(|| "high series all non-finite".to_string())?;
    let fv_low = first_finite_idx(low).ok_or_else(|| "low series all non-finite".to_string())?;

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(req.backend, Backend::CpuOnly) {
        None
    } else {
        PreparedCudaIndicatorInputs::from_candles(candles, "close").ok()
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

    let mut breakout_cache: HashMap<u16, crate::rocb::RocbBreakoutSignals> = HashMap::new();
    let mut risk_cache: HashMap<u16, crate::rocb::RocbRiskSignals> = HashMap::new();
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            let periods = resolve_rocb_periods(&rocb_cfg, fast_len, slow_len);
            if !breakout_cache.contains_key(&periods.channel) {
                let built = compute_rocb_breakout_signals(
                    candles,
                    "close",
                    &req.backend,
                    periods.channel,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                breakout_cache.insert(periods.channel, built);
            }
            if !risk_cache.contains_key(&periods.atr) {
                let built = compute_rocb_risk_signals(
                    candles,
                    "close",
                    &req.backend,
                    periods.atr,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                risk_cache.insert(periods.atr, built);
            }
        }
    }

    let first_valid_breakout = breakout_cache
        .values()
        .map(|row| row.first_valid)
        .max()
        .unwrap_or(fv_close.max(fv_high).max(fv_low));
    let first_valid_risk = risk_cache
        .values()
        .map(|row| row.first_valid)
        .max()
        .unwrap_or(fv_close.max(fv_high).max(fv_low));
    let start_t = compute_rocb_start_t(
        fv_close,
        fv_high,
        fv_low,
        first_valid_breakout,
        first_valid_risk,
        strategy.trade_on_next_bar,
    );
    if start_t >= n {
        return Err("not enough candles after warmup for analysis".to_string());
    }

    let lr = compute_log_returns_f64(close);
    let objective = req.objective;

    let mut points: Vec<LocalStabilityPoint> = Vec::new();
    let mut desired: Vec<i32> = Vec::with_capacity(n);
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            let periods = resolve_rocb_periods(&rocb_cfg, fast_len, slow_len);
            let breakout = breakout_cache
                .get(&periods.channel)
                .ok_or_else(|| "ROCB breakout signal cache miss".to_string())?;
            let risk = risk_cache
                .get(&periods.atr)
                .ok_or_else(|| "ROCB risk signal cache miss".to_string())?;

            build_rocb_desired_positions_into(
                &mut desired,
                close,
                breakout,
                risk,
                start_t,
                &strategy,
                &rocb_cfg,
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
            let backend_label = composite_backend_label(&req.backend).to_string();
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
            let backend_label = composite_backend_label(&req.backend).to_string();
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
    let close_f32: Vec<f32> = close.iter().map(|v| *v as f32).collect();

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;

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
        &["upper_band", "upper", "up"],
        channel_min,
        channel_max,
        1,
    )?;
    let lower_sweep = compute_rocb_donchian_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["lower_band", "lower", "down"],
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

    let mut combo_channel_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_atr_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_params: Vec<DoubleMaParams> = Vec::with_capacity(total_pairs);
    let mut combo_start_lookup: HashMap<(u16, u16), usize> = HashMap::with_capacity(total_pairs);

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
            let channel_row = upper_sweep
                .row_index_for_period(periods.channel)
                .ok_or_else(|| {
                    format!(
                        "ROCB GPU optimization missing channel row for period={}",
                        periods.channel
                    )
                })?;
            if lower_sweep.row_index_for_period(periods.channel).is_none() {
                return Err(format!(
                    "ROCB GPU optimization missing lower channel row for period={}",
                    periods.channel
                ));
            }
            let atr_row = atr_sweep.row_index_for_period(periods.atr).ok_or_else(|| {
                format!(
                    "ROCB GPU optimization missing ATR row for period={}",
                    periods.atr
                )
            })?;
            let channel_first_valid = upper_sweep
                .first_valid_for_period(periods.channel)
                .max(lower_sweep.first_valid_for_period(periods.channel));
            let atr_first_valid = atr_sweep.first_valid_for_period(periods.atr);
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

            combo_channel_row_idx.push(
                i32::try_from(channel_row)
                    .map_err(|_| "ROCB channel row exceeds i32 range".to_string())?,
            );
            combo_atr_row_idx.push(
                i32::try_from(atr_row).map_err(|_| "ROCB ATR row exceeds i32 range".to_string())?,
            );
            combo_start_t.push(
                i32::try_from(start_t).map_err(|_| "ROCB start_t exceeds i32 range".to_string())?,
            );
            combo_start_lookup.insert((fast_len, slow_len), start_t);
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
            .map_err(|_| "ROCB end_t exceeds i32 range".to_string())?;
        combo_channel_row_idx.len()
    ];
    let combos = RocbGpuComboBatch {
        channel_row_index: &combo_channel_row_idx,
        atr_row_index: &combo_atr_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };

    let start = std::time::Instant::now();
    let metrics = eval_rocb_metrics_gpu_kernel_device_rows(
        device_id, &signals, &combos, &strategy, &rocb_cfg,
    )?;
    if metrics.len() != combo_params.len() {
        return Err("ROCB GPU kernel output length mismatch".to_string());
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
            emit_progress(progress, i + 1, total_pairs, "rocb_optimize");
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "rocb_optimize");

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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "ROCB grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let start_t = *combo_start_lookup
            .get(&(params.fast_len, params.slow_len))
            .ok_or_else(|| "ROCB combo start_t lookup miss".to_string())?;
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_score: result.best_score,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap: result.heatmap.clone(),
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: Some("GpuKernel".to_string()),
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
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
        Some(prepared_cuda_inputs_for_source_hlc_on_device(
            candles,
            "close",
            match indicator_backend {
                Backend::Auto { device_id }
                | Backend::GpuOnly { device_id }
                | Backend::GpuHybridKernel { device_id }
                | Backend::GpuKernel { device_id } => *device_id as usize,
                Backend::CpuOnly => 0,
            },
        )?)
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

    let mut desired = Vec::with_capacity(n);
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

            build_rocb_desired_positions_into(
                &mut desired,
                close,
                breakout,
                risk,
                start_t,
                &strategy,
                &rocb_cfg,
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
            processed += 1;
            if (processed & 0x03ff) == 0 {
                emit_progress(progress, processed, total_pairs, "rocb_optimize");
            }
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "rocb_optimize");

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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "ROCB grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, req, objective, |params| {
        let periods = resolve_rocb_periods(&rocb_cfg, params.fast_len, params.slow_len);
        let breakout = breakout_cache
            .get(&periods.channel)
            .ok_or_else(|| "ROCB breakout cache miss".to_string())?;
        let risk = risk_cache
            .get(&periods.atr)
            .ok_or_else(|| "ROCB risk cache miss".to_string())?;
        let start_t = compute_rocb_start_t(
            fv_close,
            fv_high,
            fv_low,
            breakout.first_valid,
            risk.first_valid,
            strategy.trade_on_next_bar,
        );
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_score: result.best_score,
        mode_used,
        backend_used,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms: 0,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap: result.heatmap.clone(),
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: Some("GpuKernel".to_string()),
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_vcet_optimization(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        if !matches!(req.backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            let device_id = match req.backend {
                Backend::Auto { device_id }
                | Backend::GpuOnly { device_id }
                | Backend::GpuHybridKernel { device_id }
                | Backend::GpuKernel { device_id } => device_id,
                Backend::CpuOnly => 0,
            };
            if matches!(
                req.backend,
                Backend::GpuOnly { .. } | Backend::GpuHybridKernel { .. }
            ) {
                let mut cpu_res = run_vcet_optimization_cpu(req, candles, cancel, progress)?;
                cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
                cpu_res.backend_fallback_reason = Some(
                    "VCET optimization currently supports GpuKernel-compatible execution only."
                        .to_string(),
                );
                return Ok(cpu_res);
            }
            match run_vcet_optimization_gpu(&req, candles, cancel, progress, device_id) {
                Ok(mut gpu_res) => {
                    if matches!(req.backend, Backend::Auto { .. }) {
                        gpu_res.backend_route = Some("Auto->GpuKernel".to_string());
                    } else if !matches!(req.backend, Backend::GpuKernel { .. }) {
                        gpu_res.backend_route = Some(format!("{backend_label}->GpuKernel"));
                    }
                    return Ok(gpu_res);
                }
                Err(err) => {
                    let mut cpu_res = run_vcet_optimization_cpu(req, candles, cancel, progress)?;
                    cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
                    cpu_res.backend_fallback_reason = Some(err);
                    return Ok(cpu_res);
                }
            }
        }
    }

    #[cfg(all(feature = "cuda", not(feature = "cuda-backtest-kernel")))]
    {
        if !matches!(req.backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            let mut cpu_res = run_vcet_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason = Some(
                "VCET GPU kernel backend is disabled (build without `cuda-backtest-kernel`)."
                    .to_string(),
            );
            return Ok(cpu_res);
        }
    }

    run_vcet_optimization_cpu(req, candles, cancel, progress)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn run_vcet_optimization_gpu(
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
    let vcet_cfg = req.vcet.clone().unwrap_or_default();
    validate_vcet_params(&vcet_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "VCET")?;
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
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;

    let mut vidya_keys: Vec<(u16, u16)> = Vec::new();
    let mut ce_keys: Vec<u16> = Vec::new();
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            let periods = resolve_vcet_periods(&vcet_cfg, fast_len, slow_len);
            vidya_keys.push((periods.vidya_short, periods.vidya_long));
            ce_keys.push(periods.ce);
        }
    }
    vidya_keys.sort_unstable();
    vidya_keys.dedup();
    ce_keys.sort_unstable();
    ce_keys.dedup();
    if vidya_keys.is_empty() || ce_keys.is_empty() {
        return Err("VCET GPU period set is empty".to_string());
    }

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
            vcet_cfg.vidya_alpha,
        )?;
        if matrix.rows != 1 || matrix.cols != n {
            return Err("VCET GPU VIDYA matrix shape mismatch".to_string());
        }
        let idx = i32::try_from(vidya_keepalive.len())
            .map_err(|_| "VCET GPU VIDYA row index exceeds i32 range".to_string())?;
        let first_valid = usize::from(vidya_long.saturating_sub(1));
        vidya_lookup.insert((vidya_short, vidya_long), (idx, first_valid));
        vidya_keepalive.push(matrix);
    }

    for &ce_period in &ce_keys {
        let (long_matrix, short_matrix) = compute_vcet_chandelier_signal_cuda_device(
            &prepared_cuda_inputs,
            ce_period,
            vcet_cfg.ce_mult,
            vcet_cfg.ce_use_close,
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
        let first_valid = usize::from(ce_period.saturating_sub(1));
        ce_lookup.insert(ce_period, (idx, first_valid));
        ce_long_keepalive.push(long_matrix);
        ce_short_keepalive.push(short_matrix);
    }

    let mut combo_vidya_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_ce_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_params: Vec<DoubleMaParams> = Vec::with_capacity(total_pairs);

    emit_progress(progress, 0, total_pairs, "vcet_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_vcet_periods(&vcet_cfg, fast_len, slow_len);
            let (vidya_row, vidya_first_valid) = *vidya_lookup
                .get(&(periods.vidya_short, periods.vidya_long))
                .ok_or_else(|| "VCET GPU VIDYA row lookup miss".to_string())?;
            let (ce_row, ce_first_valid) = *ce_lookup
                .get(&periods.ce)
                .ok_or_else(|| "VCET GPU chandelier row lookup miss".to_string())?;
            let start_t = compute_vcet_start_t(
                fv_close,
                fv_high,
                fv_low,
                vidya_first_valid,
                ce_first_valid,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            combo_vidya_row_idx.push(vidya_row);
            combo_ce_row_idx.push(ce_row);
            combo_start_t.push(
                i32::try_from(start_t)
                    .map_err(|_| "VCET GPU start_t exceeds i32 range".to_string())?,
            );
            combo_params.push(DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            });

            if (combo_params.len() & 0x03ff) == 0 {
                emit_progress(progress, combo_params.len(), total_pairs, "vcet_optimize");
            }
        }
    }
    emit_progress(progress, combo_params.len(), total_pairs, "vcet_optimize");

    if combo_params.is_empty() {
        return Err("VCET grid search produced no valid parameter combinations".to_string());
    }

    let signal_tables = VcetGpuOwnedSignalTables::from_owned_rows(
        n,
        vidya_keepalive,
        ce_long_keepalive,
        ce_short_keepalive,
    )?;
    let signals = signal_tables.as_signal_matrices(
        prepared_cuda_inputs.close(),
        prepared_cuda_inputs
            .close_device()
            .buffer()
            .as_device_ptr()
            .as_raw(),
    );
    let combo_end_t = vec![
        i32::try_from(n)
            .map_err(|_| "VCET end_t exceeds i32 range".to_string())?;
        combo_vidya_row_idx.len()
    ];
    let combos = VcetGpuComboBatch {
        vidya_row_index: &combo_vidya_row_idx,
        ce_row_index: &combo_ce_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };

    let start = std::time::Instant::now();
    let metrics = eval_vcet_metrics_gpu_kernel_device_rows(
        device_id, &signals, &combos, &strategy, &vcet_cfg,
    )?;
    if metrics.len() != combo_params.len() {
        return Err("VCET GPU kernel output length mismatch".to_string());
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
            emit_progress(progress, i + 1, total_pairs, "vcet_optimize");
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "vcet_optimize");

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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "VCET grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_vcet_periods(&vcet_cfg, params.fast_len, params.slow_len);
        let (_, vidya_first_valid) = *vidya_lookup
            .get(&(periods.vidya_short, periods.vidya_long))
            .ok_or_else(|| "VCET GPU VIDYA first_valid lookup miss".to_string())?;
        let (_, ce_first_valid) = *ce_lookup
            .get(&periods.ce)
            .ok_or_else(|| "VCET GPU chandelier first_valid lookup miss".to_string())?;
        let start_t = compute_vcet_start_t(
            fv_close,
            fv_high,
            fv_low,
            vidya_first_valid,
            ce_first_valid,
            strategy.trade_on_next_bar,
        );
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_score: result.best_score,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap: result.heatmap.clone(),
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: Some("GpuKernel".to_string()),
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_vcet_optimization_cpu(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    run_vcet_optimization_impl(
        &req,
        candles,
        cancel,
        progress,
        &Backend::CpuOnly,
        BackendUsed::Cpu,
    )
}

fn run_vcet_optimization_impl(
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
    let vcet_cfg = req.vcet.clone().unwrap_or_default();
    validate_vcet_params(&vcet_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "VCET")?;
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
        Some(prepared_cuda_inputs_for_source_hlc_on_device(
            candles,
            &req.ma_source,
            match indicator_backend {
                Backend::Auto { device_id }
                | Backend::GpuOnly { device_id }
                | Backend::GpuHybridKernel { device_id }
                | Backend::GpuKernel { device_id } => *device_id as usize,
                Backend::CpuOnly => 0,
            },
        )?)
    } else {
        None
    };

    let mut vidya_cache: HashMap<(u16, u16), crate::vcet::VcetVidyaSignals> = HashMap::new();
    let mut ce_cache: HashMap<u16, crate::vcet::VcetChandelierSignals> = HashMap::new();
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
    emit_progress(progress, 0, total_pairs, "vcet_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_vcet_periods(&vcet_cfg, fast_len, slow_len);
            let vidya_key = (periods.vidya_short, periods.vidya_long);
            let vidya = if let Some(v) = vidya_cache.get(&vidya_key) {
                v
            } else {
                let built = compute_vcet_vidya_signals(
                    candles,
                    &req.ma_source,
                    indicator_backend,
                    periods,
                    vcet_cfg.vidya_alpha,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                vidya_cache.insert(vidya_key, built);
                vidya_cache
                    .get(&vidya_key)
                    .ok_or_else(|| "VCET vidya cache insert failed".to_string())?
            };
            let ce = if let Some(v) = ce_cache.get(&periods.ce) {
                v
            } else {
                let built = compute_vcet_chandelier_signals(
                    candles,
                    indicator_backend,
                    periods.ce,
                    vcet_cfg.ce_mult,
                    vcet_cfg.ce_use_close,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                ce_cache.insert(periods.ce, built);
                ce_cache
                    .get(&periods.ce)
                    .ok_or_else(|| "VCET chandelier cache insert failed".to_string())?
            };

            let start_t = compute_vcet_start_t(
                fv_close,
                fv_high,
                fv_low,
                vidya.first_valid,
                ce.first_valid,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            let desired =
                build_vcet_desired_positions(close, vidya, ce, start_t, &strategy, &vcet_cfg);
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
                emit_progress(progress, processed, total_pairs, "vcet_optimize");
            }
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "vcet_optimize");

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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "VCET grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, req, objective, |params| {
        let periods = resolve_vcet_periods(&vcet_cfg, params.fast_len, params.slow_len);
        let vidya = vidya_cache
            .get(&(periods.vidya_short, periods.vidya_long))
            .ok_or_else(|| "VCET vidya cache miss".to_string())?;
        let ce = ce_cache
            .get(&periods.ce)
            .ok_or_else(|| "VCET chandelier cache miss".to_string())?;
        let start_t = compute_vcet_start_t(
            fv_close,
            fv_high,
            fv_low,
            vidya.first_valid,
            ce.first_valid,
            strategy.trade_on_next_bar,
        );
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_score: result.best_score,
        mode_used,
        backend_used,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms: 0,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap: result.heatmap.clone(),
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_mkcs_optimization(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let normalized_backend = req.backend.clone().with_preferred_cuda_device()?;
        if !matches!(normalized_backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            if vector_ta::cuda::cuda_available() {
                let device_id = match &normalized_backend {
                    Backend::Auto { device_id }
                    | Backend::GpuOnly { device_id }
                    | Backend::GpuHybridKernel { device_id }
                    | Backend::GpuKernel { device_id } => *device_id,
                    Backend::CpuOnly => 0,
                };
                match run_mkcs_optimization_gpu(&req, candles, cancel, progress, device_id) {
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
                            run_mkcs_optimization_cpu(req, candles, cancel, progress)?;
                        cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
                        cpu_res.backend_fallback_reason = Some(err);
                        return Ok(cpu_res);
                    }
                }
            }

            let mut cpu_res = run_mkcs_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason =
                Some("CUDA unavailable; falling back to CPU MKCS optimization.".to_string());
            return Ok(cpu_res);
        }
    }

    #[cfg(all(feature = "cuda", not(feature = "cuda-backtest-kernel")))]
    {
        if !matches!(req.backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            let mut cpu_res = run_mkcs_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason = Some(
                "MKCS GPU kernel backend is disabled (build without `cuda-backtest-kernel`)."
                    .to_string(),
            );
            return Ok(cpu_res);
        }
    }

    run_mkcs_optimization_cpu(req, candles, cancel, progress)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn run_mkcs_optimization_gpu(
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
    let mkcs_cfg = req.mkcs.clone().unwrap_or_default();
    validate_mkcs_params(&mkcs_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "MKCS")?;
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
        prepared_cuda_inputs_for_source_hlc_on_device(candles, "close", device_id as usize)?;

    let mut stddev_keys: Vec<u16> = fast_periods
        .iter()
        .map(|&fast_len| resolve_mkcs_periods(&mkcs_cfg, fast_len, 1).stddev)
        .collect();
    stddev_keys.sort_unstable();
    stddev_keys.dedup();
    if stddev_keys.is_empty() {
        return Err("MKCS stddev period set is empty".to_string());
    }
    let stddev_min = *stddev_keys.first().unwrap_or(&1);
    let stddev_max = *stddev_keys.last().unwrap_or(&stddev_min);
    let stddev_sweep =
        compute_mkcs_stddev_sweep_cuda_device(&prepared_cuda_inputs, stddev_min, stddev_max, 1)?;
    if stddev_sweep.matrix.cols != n {
        return Err("MKCS stddev sweep column mismatch".to_string());
    }

    let mut stddev_row_lookup: HashMap<u16, i32> = HashMap::with_capacity(stddev_keys.len());
    let mut stddev_first_valid_lookup: HashMap<u16, usize> =
        HashMap::with_capacity(stddev_keys.len());
    for period in stddev_keys.iter().copied() {
        let row = stddev_sweep
            .row_index_for_period(period)
            .ok_or_else(|| format!("MKCS stddev row missing for period={period}"))?;
        let row_i =
            i32::try_from(row).map_err(|_| "MKCS stddev row exceeds i32 range".to_string())?;
        stddev_row_lookup.insert(period, row_i);
        stddev_first_valid_lookup.insert(period, stddev_sweep.first_valid_for_period(period));
    }

    let mut breakout_keys: Vec<u16> = slow_periods
        .iter()
        .map(|&slow_len| resolve_mkcs_periods(&mkcs_cfg, 1, slow_len).breakout)
        .collect();
    breakout_keys.sort_unstable();
    breakout_keys.dedup();
    if breakout_keys.is_empty() {
        return Err("MKCS breakout lookback set is empty".to_string());
    }
    let breakout_min = *breakout_keys.first().unwrap_or(&1);
    let breakout_max = *breakout_keys.last().unwrap_or(&breakout_min);
    let donchian_sweeps = compute_mkcs_donchian_sweeps_cuda_device(
        &prepared_cuda_inputs,
        breakout_min,
        breakout_max,
        1,
    )?;
    let upper_sweep = donchian_sweeps.upper;
    let lower_sweep = donchian_sweeps.lower;
    if upper_sweep.matrix.cols != n || lower_sweep.matrix.cols != n {
        return Err("MKCS donchian sweep column mismatch".to_string());
    }
    if upper_sweep.matrix.rows != lower_sweep.matrix.rows {
        return Err("MKCS donchian sweep row mismatch".to_string());
    }

    let mut breakout_row_lookup: HashMap<u16, i32> = HashMap::with_capacity(breakout_keys.len());
    let mut breakout_first_valid_lookup: HashMap<u16, usize> =
        HashMap::with_capacity(breakout_keys.len());
    for period in breakout_keys.iter().copied() {
        let upper_row = upper_sweep
            .row_index_for_period(period)
            .ok_or_else(|| format!("MKCS upper row missing for period={period}"))?;
        if lower_sweep.row_index_for_period(period).is_none() {
            return Err(format!("MKCS lower row missing for period={period}"));
        }
        let row_i = i32::try_from(upper_row)
            .map_err(|_| "MKCS breakout row exceeds i32 range".to_string())?;
        breakout_row_lookup.insert(period, row_i);
        breakout_first_valid_lookup.insert(period, upper_sweep.first_valid_for_period(period));
    }

    let mut combo_stddev_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_breakout_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_params: Vec<DoubleMaParams> = Vec::with_capacity(total_pairs);

    emit_progress(progress, 0, total_pairs, "mkcs_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_mkcs_periods(&mkcs_cfg, fast_len, slow_len);
            let stddev_row = *stddev_row_lookup
                .get(&periods.stddev)
                .ok_or_else(|| "MKCS stddev row lookup miss".to_string())?;
            let breakout_row = *breakout_row_lookup
                .get(&periods.breakout)
                .ok_or_else(|| "MKCS breakout row lookup miss".to_string())?;
            let stddev_first_valid = *stddev_first_valid_lookup
                .get(&periods.stddev)
                .ok_or_else(|| "MKCS stddev first_valid lookup miss".to_string())?;
            let breakout_first_valid = *breakout_first_valid_lookup
                .get(&periods.breakout)
                .ok_or_else(|| "MKCS breakout first_valid lookup miss".to_string())?;

            let start_t = compute_mkcs_start_t(
                fv_close,
                fv_high,
                fv_low,
                stddev_first_valid,
                breakout_first_valid,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            combo_stddev_row_idx.push(stddev_row);
            combo_breakout_row_idx.push(breakout_row);
            combo_start_t.push(
                i32::try_from(start_t).map_err(|_| "MKCS start_t exceeds i32 range".to_string())?,
            );
            combo_params.push(DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            });

            if (combo_params.len() & 0x03ff) == 0 {
                emit_progress(progress, combo_params.len(), total_pairs, "mkcs_optimize");
            }
        }
    }
    emit_progress(progress, combo_params.len(), total_pairs, "mkcs_optimize");

    if combo_params.is_empty() {
        return Err("MKCS grid search produced no valid parameter combinations".to_string());
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
            .map_err(|_| "MKCS end_t exceeds i32 range".to_string())?;
        combo_stddev_row_idx.len()
    ];
    let combos = MkcsGpuComboBatch {
        stddev_row_index: &combo_stddev_row_idx,
        breakout_row_index: &combo_breakout_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };

    let start = std::time::Instant::now();
    let metrics = eval_mkcs_metrics_gpu_kernel_device_rows(
        device_id, &signals, &combos, &strategy, &mkcs_cfg,
    )?;
    if metrics.len() != combo_params.len() {
        return Err("MKCS GPU kernel output length mismatch".to_string());
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
            emit_progress(progress, i + 1, total_pairs, "mkcs_optimize");
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "mkcs_optimize");

    let mut result = finalize_standalone_optimization_result(
        agg,
        "MKCS grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_mkcs_periods(&mkcs_cfg, params.fast_len, params.slow_len);
        let stddev_first_valid = *stddev_first_valid_lookup
            .get(&periods.stddev)
            .ok_or_else(|| "MKCS stddev first_valid lookup miss".to_string())?;
        let breakout_first_valid = *breakout_first_valid_lookup
            .get(&periods.breakout)
            .ok_or_else(|| "MKCS breakout first_valid lookup miss".to_string())?;
        let start_t = compute_mkcs_start_t(
            fv_close,
            fv_high,
            fv_low,
            stddev_first_valid,
            breakout_first_valid,
            strategy.trade_on_next_bar,
        );
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_score: result.best_score,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap: result.heatmap.clone(),
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_kqsr_optimization(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let normalized_backend = req.backend.clone().with_preferred_cuda_device()?;
        if !matches!(normalized_backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            if vector_ta::cuda::cuda_available() {
                let device_id = match &normalized_backend {
                    Backend::Auto { device_id }
                    | Backend::GpuOnly { device_id }
                    | Backend::GpuHybridKernel { device_id }
                    | Backend::GpuKernel { device_id } => *device_id,
                    Backend::CpuOnly => 0,
                };
                match run_kqsr_optimization_gpu(&req, candles, cancel, progress, device_id) {
                    Ok(mut gpu_res) => {
                        if matches!(req.backend, Backend::Auto { .. }) {
                            gpu_res.backend_route = Some("Auto->GpuKernel".to_string());
                        } else if matches!(req.backend, Backend::GpuKernel { .. }) {
                            gpu_res.backend_route = Some("GpuKernel".to_string());
                        } else {
                            gpu_res.backend_route = Some(format!("{backend_label}->GpuKernel"));
                        }
                        return Ok(gpu_res);
                    }
                    Err(err) => {
                        let mut cpu_res =
                            run_kqsr_optimization_cpu(req, candles, cancel, progress)?;
                        cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
                        cpu_res.backend_fallback_reason = Some(err);
                        return Ok(cpu_res);
                    }
                }
            }

            let mut cpu_res = run_kqsr_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason =
                Some("CUDA unavailable; falling back to CPU KQSR optimization.".to_string());
            return Ok(cpu_res);
        }
    }

    #[cfg(all(feature = "cuda", not(feature = "cuda-backtest-kernel")))]
    {
        if !matches!(req.backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            let mut cpu_res = run_kqsr_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason = Some(
                "KQSR GPU kernel backend is disabled (build without `cuda-backtest-kernel`)."
                    .to_string(),
            );
            return Ok(cpu_res);
        }
    }

    run_kqsr_optimization_cpu(req, candles, cancel, progress)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn run_kqsr_optimization_gpu(
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
    let kqsr_cfg = req.kqsr.clone().unwrap_or_default();
    validate_kqsr_params(&kqsr_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "KQSR")?;
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

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;

    let slow_seed = *slow_periods
        .first()
        .ok_or_else(|| "KQSR slow period range is empty".to_string())?;
    let fast_seed = *fast_periods
        .first()
        .ok_or_else(|| "KQSR fast period range is empty".to_string())?;

    let fast_rows = fast_periods.len();
    let slow_rows = slow_periods.len();
    let row_major_len_fast = fast_rows
        .checked_mul(n)
        .ok_or_else(|| "KQSR fast signal matrix overflow".to_string())?;
    let row_major_len_slow = slow_rows
        .checked_mul(n)
        .ok_or_else(|| "KQSR slow signal matrix overflow".to_string())?;

    let mut j_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut srsi_k_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut srsi_d_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut ift_rsi_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut fast_first_valid: Vec<usize> = Vec::with_capacity(fast_rows);

    emit_progress(progress, 0, total_pairs, "kqsr_optimize");
    for &fast_len in &fast_periods {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let periods = resolve_kqsr_periods(&kqsr_cfg, fast_len, slow_seed);
        let signals = compute_kqsr_fast_signals_f32(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            &kqsr_cfg,
            Some(&prepared_cuda_inputs),
        )?;
        fast_first_valid.push(signals.first_valid);
        j_row_major.extend_from_slice(&signals.j);
        srsi_k_row_major.extend_from_slice(&signals.srsi_k);
        srsi_d_row_major.extend_from_slice(&signals.srsi_d);
        ift_rsi_row_major.extend_from_slice(&signals.ift_rsi);
    }

    let mut qqe_fast_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut qqe_slow_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut prb_upper_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut prb_lower_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut slow_first_valid: Vec<usize> = Vec::with_capacity(slow_rows);
    for &slow_len in &slow_periods {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let periods = resolve_kqsr_periods(&kqsr_cfg, fast_seed, slow_len);
        let signals = compute_kqsr_slow_signals_f32(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            Some(&prepared_cuda_inputs),
        )?;
        slow_first_valid.push(signals.first_valid);
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

    let mut combo_fast_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_slow_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_params: Vec<DoubleMaParams> = Vec::with_capacity(total_pairs);

    for (fast_idx, &fast_len) in fast_periods.iter().enumerate() {
        for (slow_idx, &slow_len) in slow_periods.iter().enumerate() {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let signals_first_valid = fast_first_valid
                .get(fast_idx)
                .copied()
                .unwrap_or(n)
                .max(slow_first_valid.get(slow_idx).copied().unwrap_or(n));
            let start_t =
                compute_kqsr_start_t(fv_close, signals_first_valid, strategy.trade_on_next_bar);
            if start_t >= n {
                continue;
            }

            combo_fast_row_idx.push(
                i32::try_from(fast_idx)
                    .map_err(|_| "KQSR fast row index exceeds i32 range".to_string())?,
            );
            combo_slow_row_idx.push(
                i32::try_from(slow_idx)
                    .map_err(|_| "KQSR slow row index exceeds i32 range".to_string())?,
            );
            combo_start_t.push(
                i32::try_from(start_t).map_err(|_| "KQSR start_t exceeds i32 range".to_string())?,
            );
            combo_params.push(DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            });

            if (combo_params.len() & 0x03ff) == 0 {
                emit_progress(progress, combo_params.len(), total_pairs, "kqsr_optimize");
            }
        }
    }
    emit_progress(progress, combo_params.len(), total_pairs, "kqsr_optimize");

    if combo_params.is_empty() {
        return Err("KQSR grid search produced no valid parameter combinations".to_string());
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

    let start = std::time::Instant::now();
    let metrics = eval_kqsr_metrics_gpu_kernel_device_rows(
        device_id, &signals, &combos, &strategy, &kqsr_cfg,
    )?;
    if metrics.len() != combo_params.len() {
        return Err("KQSR GPU kernel output length mismatch".to_string());
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
            emit_progress(progress, i + 1, total_pairs, "kqsr_optimize");
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "kqsr_optimize");

    let mut result = finalize_standalone_optimization_result(
        agg,
        "KQSR grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_kqsr_periods(&kqsr_cfg, params.fast_len, params.slow_len);
        let fast_idx = fast_periods
            .iter()
            .position(|&len| len == params.fast_len)
            .ok_or_else(|| "KQSR fast period index miss".to_string())?;
        let slow_idx = slow_periods
            .iter()
            .position(|&len| len == params.slow_len)
            .ok_or_else(|| "KQSR slow period index miss".to_string())?;
        let signals_first_valid = fast_first_valid
            .get(fast_idx)
            .copied()
            .unwrap_or(n)
            .max(slow_first_valid.get(slow_idx).copied().unwrap_or(n));
        let _ = periods;
        let start_t =
            compute_kqsr_start_t(fv_close, signals_first_valid, strategy.trade_on_next_bar);
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_metrics: result.best_metrics,
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Gpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap: result.heatmap.clone(),
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_kqsr_optimization_cpu(
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
    let kqsr_cfg = req.kqsr.clone().unwrap_or_default();
    validate_kqsr_params(&kqsr_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "KQSR")?;
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

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let lr = compute_log_returns_f64(close);
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(req.backend, Backend::CpuOnly) {
        None
    } else {
        prepared_cuda_inputs_for_source_hlc(candles, &req.ma_source)
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
    emit_progress(progress, 0, total_pairs, "kqsr_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_kqsr_periods(&kqsr_cfg, fast_len, slow_len);
            let signals = compute_kqsr_signals(
                candles,
                &req.ma_source,
                &req.backend,
                periods,
                &kqsr_cfg,
                #[cfg(feature = "cuda")]
                prepared_cuda_inputs.as_ref(),
            )?;
            let start_t =
                compute_kqsr_start_t(fv_close, signals.first_valid, strategy.trade_on_next_bar);
            if start_t >= n {
                continue;
            }

            let desired =
                build_kqsr_desired_positions(close, &signals, start_t, &strategy, &kqsr_cfg);
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
                emit_progress(progress, processed, total_pairs, "kqsr_optimize");
            }
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "kqsr_optimize");

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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "KQSR grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_kqsr_periods(&kqsr_cfg, params.fast_len, params.slow_len);
        let signals = compute_kqsr_signals(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            &kqsr_cfg,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs.as_ref(),
        )?;
        let start_t =
            compute_kqsr_start_t(fv_close, signals.first_valid, strategy.trade_on_next_bar);
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
    let matrix_index =
        DoubleMaFastLtSlowIndex::new(fast_periods.clone(), slow_periods.clone()).ok();
    let best_col = matrix_index
        .as_ref()
        .and_then(|idx| idx.col_id(best_params.fast_len, best_params.slow_len));
    let top_cols = matrix_index.as_ref().map(|idx| {
        top.iter()
            .filter_map(|(p, _)| idx.col_id(p.fast_len, p.slow_len))
            .collect::<Vec<_>>()
    });
    let top_cols = match top_cols {
        Some(v) if v.len() == top.len() => Some(v),
        _ => None,
    };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Cpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms: 0,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_carm_optimization(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let normalized_backend = req.backend.clone().with_preferred_cuda_device()?;
        if !matches!(normalized_backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            if vector_ta::cuda::cuda_available() {
                let device_id = match &normalized_backend {
                    Backend::Auto { device_id }
                    | Backend::GpuOnly { device_id }
                    | Backend::GpuHybridKernel { device_id }
                    | Backend::GpuKernel { device_id } => *device_id,
                    Backend::CpuOnly => 0,
                };
                match run_carm_optimization_gpu(&req, candles, cancel, progress, device_id) {
                    Ok(mut gpu_res) => {
                        if matches!(req.backend, Backend::Auto { .. }) {
                            gpu_res.backend_route = Some("Auto->GpuKernel".to_string());
                        } else if matches!(req.backend, Backend::GpuKernel { .. }) {
                            gpu_res.backend_route = Some("GpuKernel".to_string());
                        } else {
                            gpu_res.backend_route = Some(format!("{backend_label}->GpuKernel"));
                        }
                        return Ok(gpu_res);
                    }
                    Err(err) => {
                        let mut cpu_res =
                            run_carm_optimization_cpu(req, candles, cancel, progress)?;
                        cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
                        cpu_res.backend_fallback_reason = Some(err);
                        return Ok(cpu_res);
                    }
                }
            }

            let mut cpu_res = run_carm_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason =
                Some("CUDA unavailable; falling back to CPU CARM optimization.".to_string());
            return Ok(cpu_res);
        }
    }

    #[cfg(all(feature = "cuda", not(feature = "cuda-backtest-kernel")))]
    {
        if !matches!(req.backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            let mut cpu_res = run_carm_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason = Some(
                "CARM GPU kernel backend is disabled (build without `cuda-backtest-kernel`)."
                    .to_string(),
            );
            return Ok(cpu_res);
        }
    }

    run_carm_optimization_cpu(req, candles, cancel, progress)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn run_carm_optimization_gpu(
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
    let carm_cfg = req.carm.clone().unwrap_or_default();
    validate_carm_params(&carm_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "CARM")?;
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

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_hlc_on_device(candles, &req.ma_source, device_id as usize)?;
    let mut gpu_attempt_state = CarmGpuAttemptState::default();

    let fast_rows = fast_periods.len();
    let row_major_len_fast = fast_rows
        .checked_mul(n)
        .ok_or_else(|| "CARM fast signal matrix overflow".to_string())?;
    let mut rocr_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut rocr_first_valid: Vec<usize> = Vec::with_capacity(fast_rows);
    let slow_seed = *slow_periods
        .first()
        .ok_or_else(|| "CARM slow period range is empty".to_string())?;
    for &fast_len in &fast_periods {
        let periods = resolve_carm_periods(&carm_cfg, fast_len, slow_seed);
        let rocr = compute_carm_rocr_signal_f32_with_attempt_state(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            Some(&prepared_cuda_inputs),
            &mut gpu_attempt_state,
        )?;
        rocr_first_valid.push(rocr.iter().position(|v| v.is_finite()).unwrap_or(n));
        rocr_row_major.extend_from_slice(&rocr);
    }

    let mut coppock_row_major: Vec<f32> = Vec::with_capacity(total_pairs * n);
    let mut ao_row_major: Vec<f32> = Vec::with_capacity(total_pairs * n);
    let mut apo_row_major: Vec<f32> = Vec::with_capacity(total_pairs * n);
    let mut otto_hott_row_major: Vec<f32> = Vec::with_capacity(total_pairs * n);
    let mut otto_lott_row_major: Vec<f32> = Vec::with_capacity(total_pairs * n);
    let mut otto_first_valid_by_row: Vec<usize> = Vec::new();
    let mut otto_row_lookup: HashMap<(u16, u16, u16), i32> = HashMap::new();
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_fast_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_otto_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_params: Vec<DoubleMaParams> = Vec::with_capacity(total_pairs);

    emit_progress(progress, 0, total_pairs, "carm_optimize");
    for (fast_idx, &fast_len) in fast_periods.iter().enumerate() {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_carm_periods(&carm_cfg, fast_len, slow_len);
            let otto_key = (
                periods.otto,
                periods.coppock_short.max(2),
                periods.ao_slow.max(periods.otto),
            );
            let mixed = compute_carm_mixed_signals_f32_with_attempt_state(
                candles,
                &req.ma_source,
                &req.backend,
                periods,
                Some(&prepared_cuda_inputs),
                &mut gpu_attempt_state,
            )?;
            let otto_row = if let Some(&row) = otto_row_lookup.get(&otto_key) {
                row
            } else {
                let otto = compute_carm_otto_signals_f32_with_attempt_state(
                    candles,
                    &req.ma_source,
                    &req.backend,
                    periods,
                    &carm_cfg,
                    Some(&prepared_cuda_inputs),
                    &mut gpu_attempt_state,
                )?;
                let row = i32::try_from(otto_first_valid_by_row.len())
                    .map_err(|_| "CARM OTTO row index exceeds i32 range".to_string())?;
                otto_first_valid_by_row.push(otto.first_valid);
                otto_hott_row_major.extend_from_slice(&otto.hott);
                otto_lott_row_major.extend_from_slice(&otto.lott);
                otto_row_lookup.insert(otto_key, row);
                row
            };
            let otto_first_valid = otto_first_valid_by_row
                .get(usize::try_from(otto_row).unwrap_or(usize::MAX))
                .copied()
                .unwrap_or(n);
            let signals_first_valid = mixed
                .first_valid
                .max(*rocr_first_valid.get(fast_idx).unwrap_or(&n))
                .max(otto_first_valid);
            let start_t =
                compute_carm_start_t(fv_close, signals_first_valid, strategy.trade_on_next_bar);
            if start_t >= n {
                continue;
            }

            coppock_row_major.extend_from_slice(&mixed.coppock);
            ao_row_major.extend_from_slice(&mixed.ao);
            apo_row_major.extend_from_slice(&mixed.apo);
            combo_start_t.push(
                i32::try_from(start_t).map_err(|_| "CARM start_t exceeds i32 range".to_string())?,
            );
            combo_fast_row_idx.push(
                i32::try_from(fast_idx)
                    .map_err(|_| "CARM fast row index exceeds i32 range".to_string())?,
            );
            combo_otto_row_idx.push(otto_row);
            combo_params.push(DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            });

            if (combo_params.len() & 0x03ff) == 0 {
                emit_progress(progress, combo_params.len(), total_pairs, "carm_optimize");
            }
        }
    }
    emit_progress(progress, combo_params.len(), total_pairs, "carm_optimize");

    if combo_params.is_empty() {
        return Err("CARM grid search produced no valid parameter combinations".to_string());
    }

    let signal_rows = combo_params.len();
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
        mixed_rows: signal_rows,
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
    let combo_end_t = vec![
        i32::try_from(n)
            .map_err(|_| "CARM end_t exceeds i32 range".to_string())?;
        signal_rows
    ];
    let combos = CarmGpuComboBatch {
        fast_row_index: &combo_fast_row_idx,
        otto_row_index: &combo_otto_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };

    let start = std::time::Instant::now();
    let metrics = eval_carm_metrics_gpu_kernel_device_rows(
        device_id, &signals, &combos, &strategy, &carm_cfg,
    )?;
    if metrics.len() != combo_params.len() {
        return Err("CARM GPU kernel output length mismatch".to_string());
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
            emit_progress(progress, i + 1, total_pairs, "carm_optimize");
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "carm_optimize");

    let mut result = finalize_standalone_optimization_result(
        agg,
        "CARM grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_carm_periods(&carm_cfg, params.fast_len, params.slow_len);
        let fast_idx = fast_periods
            .iter()
            .position(|&len| len == params.fast_len)
            .ok_or_else(|| "CARM fast period index miss".to_string())?;
        let otto_key = (
            periods.otto,
            periods.coppock_short.max(2),
            periods.ao_slow.max(periods.otto),
        );
        let otto_row = *otto_row_lookup
            .get(&otto_key)
            .ok_or_else(|| "CARM OTTO row lookup miss".to_string())?;
        let otto_first_valid = otto_first_valid_by_row
            .get(usize::try_from(otto_row).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or(n);
        let mixed = compute_carm_mixed_signals_f32_with_attempt_state(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            Some(&prepared_cuda_inputs),
            &mut gpu_attempt_state,
        )?;
        let signals_first_valid = mixed
            .first_valid
            .max(*rocr_first_valid.get(fast_idx).unwrap_or(&n))
            .max(otto_first_valid);
        let start_t =
            compute_carm_start_t(fv_close, signals_first_valid, strategy.trade_on_next_bar);
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_metrics: result.best_metrics,
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Gpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap: result.heatmap.clone(),
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_carm_optimization_cpu(
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
    let carm_cfg = req.carm.clone().unwrap_or_default();
    validate_carm_params(&carm_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "CARM")?;
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

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let lr = compute_log_returns_f64(close);
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(req.backend, Backend::CpuOnly) {
        None
    } else {
        prepared_cuda_inputs_for_source_hlc(candles, &req.ma_source)
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
    emit_progress(progress, 0, total_pairs, "carm_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_carm_periods(&carm_cfg, fast_len, slow_len);
            let signals = compute_carm_signals(
                candles,
                &req.ma_source,
                &req.backend,
                periods,
                &carm_cfg,
                #[cfg(feature = "cuda")]
                prepared_cuda_inputs.as_ref(),
            )?;
            let start_t =
                compute_carm_start_t(fv_close, signals.first_valid, strategy.trade_on_next_bar);
            if start_t >= n {
                continue;
            }

            let desired =
                build_carm_desired_positions(close, &signals, start_t, &strategy, &carm_cfg);
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
                emit_progress(progress, processed, total_pairs, "carm_optimize");
            }
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "carm_optimize");

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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "CARM grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_carm_periods(&carm_cfg, params.fast_len, params.slow_len);
        let signals = compute_carm_signals(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            &carm_cfg,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs.as_ref(),
        )?;
        let start_t =
            compute_carm_start_t(fv_close, signals.first_valid, strategy.trade_on_next_bar);
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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

    let matrix_index =
        DoubleMaFastLtSlowIndex::new(fast_periods.clone(), slow_periods.clone()).ok();
    let best_col = matrix_index
        .as_ref()
        .and_then(|idx| idx.col_id(best_params.fast_len, best_params.slow_len));
    let top_cols = matrix_index.as_ref().map(|idx| {
        top.iter()
            .filter_map(|(p, _)| idx.col_id(p.fast_len, p.slow_len))
            .collect::<Vec<_>>()
    });
    let top_cols = match top_cols {
        Some(v) if v.len() == top.len() => Some(v),
        _ => None,
    };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Cpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms: 0,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_kvdf_optimization(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let normalized_backend = req.backend.clone().with_preferred_cuda_device()?;
        if !matches!(normalized_backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            if vector_ta::cuda::cuda_available() {
                let device_id = match &normalized_backend {
                    Backend::Auto { device_id }
                    | Backend::GpuOnly { device_id }
                    | Backend::GpuHybridKernel { device_id }
                    | Backend::GpuKernel { device_id } => *device_id,
                    Backend::CpuOnly => 0,
                };
                match run_kvdf_optimization_gpu(&req, candles, cancel, progress, device_id) {
                    Ok(mut gpu_res) => {
                        if matches!(req.backend, Backend::Auto { .. }) {
                            gpu_res.backend_route = Some("Auto->GpuKernel".to_string());
                        } else if matches!(req.backend, Backend::GpuKernel { .. }) {
                            gpu_res.backend_route = Some("GpuKernel".to_string());
                        } else {
                            gpu_res.backend_route = Some(format!("{backend_label}->GpuKernel"));
                        }
                        return Ok(gpu_res);
                    }
                    Err(err) => {
                        let mut cpu_res =
                            run_kvdf_optimization_cpu(req, candles, cancel, progress)?;
                        cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
                        cpu_res.backend_fallback_reason = Some(err);
                        return Ok(cpu_res);
                    }
                }
            }

            let mut cpu_res = run_kvdf_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason =
                Some("CUDA unavailable; falling back to CPU KVDF optimization.".to_string());
            return Ok(cpu_res);
        }
    }

    #[cfg(all(feature = "cuda", not(feature = "cuda-backtest-kernel")))]
    {
        if !matches!(req.backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            let mut cpu_res = run_kvdf_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason = Some(
                "KVDF GPU kernel backend is disabled (build without `cuda-backtest-kernel`)."
                    .to_string(),
            );
            return Ok(cpu_res);
        }
    }

    run_kvdf_optimization_cpu(req, candles, cancel, progress)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn run_kvdf_optimization_gpu(
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
    let kvdf_cfg = req.kvdf.clone().unwrap_or_default();
    validate_kvdf_params(&kvdf_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "KVDF")?;
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

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let prepared_cuda_inputs = prepared_cuda_inputs_for_source_ohlcv_on_device(
        candles,
        &req.ma_source,
        device_id as usize,
    )?;

    let fast_rows = fast_periods.len();
    let slow_rows = slow_periods.len();
    let row_major_len_fast = fast_rows
        .checked_mul(n)
        .ok_or_else(|| "KVDF fast signal matrix overflow".to_string())?;
    let row_major_len_slow = slow_rows
        .checked_mul(n)
        .ok_or_else(|| "KVDF slow signal matrix overflow".to_string())?;

    let shared = compute_kvdf_shared_signals(
        candles,
        &req.ma_source,
        &req.backend,
        &kvdf_cfg,
        Some(&prepared_cuda_inputs),
    )?;
    let shared_vwap_f32: Vec<f32> = shared.vwap.iter().map(|v| *v as f32).collect();

    let slow_seed = *slow_periods
        .first()
        .ok_or_else(|| "KVDF slow period range is empty".to_string())?;
    let fast_seed = *fast_periods
        .first()
        .ok_or_else(|| "KVDF fast period range is empty".to_string())?;

    let mut fvg_upper_ts_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut fvg_lower_ts_row_major: Vec<f32> = Vec::with_capacity(row_major_len_fast);
    let mut fast_first_valid: Vec<usize> = Vec::with_capacity(fast_rows);
    for &fast_len in &fast_periods {
        let periods = resolve_kvdf_periods(&kvdf_cfg, fast_len, slow_seed);
        let fast = compute_kvdf_fast_signals(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            Some(&prepared_cuda_inputs),
        )?;
        fast_first_valid.push(fast.first_valid);
        fvg_upper_ts_row_major.extend(fast.fvg_upper_ts.iter().map(|v| *v as f32));
        fvg_lower_ts_row_major.extend(fast.fvg_lower_ts.iter().map(|v| *v as f32));
    }

    let mut keltner_upper_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut keltner_middle_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut keltner_lower_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut devstop_long_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut devstop_short_row_major: Vec<f32> = Vec::with_capacity(row_major_len_slow);
    let mut slow_first_valid: Vec<usize> = Vec::with_capacity(slow_rows);
    for &slow_len in &slow_periods {
        let periods = resolve_kvdf_periods(&kvdf_cfg, fast_seed, slow_len);
        let slow = compute_kvdf_slow_signals(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            &kvdf_cfg,
            Some(&prepared_cuda_inputs),
        )?;
        slow_first_valid.push(slow.first_valid);
        keltner_upper_row_major.extend(slow.keltner_upper.iter().map(|v| *v as f32));
        keltner_middle_row_major.extend(slow.keltner_middle.iter().map(|v| *v as f32));
        keltner_lower_row_major.extend(slow.keltner_lower.iter().map(|v| *v as f32));
        devstop_long_row_major.extend(slow.devstop_long.iter().map(|v| *v as f32));
        devstop_short_row_major.extend(slow.devstop_short.iter().map(|v| *v as f32));
    }

    let mut dti_row_major: Vec<f32> = Vec::with_capacity(total_pairs * n);
    let mut dti_first_valid_by_row: Vec<usize> = Vec::new();
    let mut dti_row_lookup: HashMap<(u16, u16, u16), i32> = HashMap::new();
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_fast_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_slow_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_dti_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_params: Vec<DoubleMaParams> = Vec::with_capacity(total_pairs);

    emit_progress(progress, 0, total_pairs, "kvdf_optimize");
    for (fast_idx, &fast_len) in fast_periods.iter().enumerate() {
        for (slow_idx, &slow_len) in slow_periods.iter().enumerate() {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_kvdf_periods(&kvdf_cfg, fast_len, slow_len);
            let dti_key = (periods.dti_r, periods.dti_s, periods.dti_u);
            let dti_row = if let Some(&row) = dti_row_lookup.get(&dti_key) {
                row
            } else {
                let dti = compute_kvdf_dti_signal(
                    candles,
                    &req.ma_source,
                    &req.backend,
                    periods,
                    Some(&prepared_cuda_inputs),
                )?;
                let row = i32::try_from(dti_first_valid_by_row.len())
                    .map_err(|_| "KVDF DTI row index exceeds i32 range".to_string())?;
                dti_first_valid_by_row.push(first_finite_idx(&dti).unwrap_or(n));
                dti_row_major.extend(dti.iter().map(|v| *v as f32));
                dti_row_lookup.insert(dti_key, row);
                row
            };
            let dti_first_valid = dti_first_valid_by_row
                .get(usize::try_from(dti_row).unwrap_or(usize::MAX))
                .copied()
                .unwrap_or(n);
            let signals_first_valid = shared
                .first_valid
                .max(*fast_first_valid.get(fast_idx).unwrap_or(&n))
                .max(*slow_first_valid.get(slow_idx).unwrap_or(&n))
                .max(dti_first_valid);
            let start_t =
                compute_kvdf_start_t(fv_close, signals_first_valid, strategy.trade_on_next_bar);
            if start_t >= n {
                continue;
            }

            combo_start_t.push(
                i32::try_from(start_t).map_err(|_| "KVDF start_t exceeds i32 range".to_string())?,
            );
            combo_fast_row_idx.push(
                i32::try_from(fast_idx)
                    .map_err(|_| "KVDF fast row index exceeds i32 range".to_string())?,
            );
            combo_slow_row_idx.push(
                i32::try_from(slow_idx)
                    .map_err(|_| "KVDF slow row index exceeds i32 range".to_string())?,
            );
            combo_dti_row_idx.push(dti_row);
            combo_params.push(DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            });

            if (combo_params.len() & 0x03ff) == 0 {
                emit_progress(progress, combo_params.len(), total_pairs, "kvdf_optimize");
            }
        }
    }
    emit_progress(progress, combo_params.len(), total_pairs, "kvdf_optimize");

    if combo_params.is_empty() {
        return Err("KVDF grid search produced no valid parameter combinations".to_string());
    }

    let signal_rows = combo_params.len();
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
    let combo_end_t = vec![
        i32::try_from(n)
            .map_err(|_| "KVDF end_t exceeds i32 range".to_string())?;
        signal_rows
    ];
    let combos = KvdfGpuComboBatch {
        fast_row_index: &combo_fast_row_idx,
        slow_row_index: &combo_slow_row_idx,
        dti_row_index: &combo_dti_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };

    let start = std::time::Instant::now();
    let metrics = eval_kvdf_metrics_gpu_kernel_device_rows(
        device_id, &signals, &combos, &strategy, &kvdf_cfg,
    )?;
    if metrics.len() != combo_params.len() {
        return Err("KVDF GPU kernel output length mismatch".to_string());
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
            emit_progress(progress, i + 1, total_pairs, "kvdf_optimize");
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "kvdf_optimize");

    let mut result = finalize_standalone_optimization_result(
        agg,
        "KVDF grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_kvdf_periods(&kvdf_cfg, params.fast_len, params.slow_len);
        let fast_idx = fast_periods
            .iter()
            .position(|&len| len == params.fast_len)
            .ok_or_else(|| "KVDF fast period index miss".to_string())?;
        let slow_idx = slow_periods
            .iter()
            .position(|&len| len == params.slow_len)
            .ok_or_else(|| "KVDF slow period index miss".to_string())?;
        let dti_key = (periods.dti_r, periods.dti_s, periods.dti_u);
        let dti_row = *dti_row_lookup
            .get(&dti_key)
            .ok_or_else(|| "KVDF DTI row lookup miss".to_string())?;
        let dti_first_valid = dti_first_valid_by_row
            .get(usize::try_from(dti_row).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or(n);
        let signals_first_valid = shared
            .first_valid
            .max(*fast_first_valid.get(fast_idx).unwrap_or(&n))
            .max(*slow_first_valid.get(slow_idx).unwrap_or(&n))
            .max(dti_first_valid);
        let start_t =
            compute_kvdf_start_t(fv_close, signals_first_valid, strategy.trade_on_next_bar);
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_metrics: result.best_metrics,
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Gpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap: result.heatmap.clone(),
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_kvdf_optimization_cpu(
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
    let kvdf_cfg = req.kvdf.clone().unwrap_or_default();
    validate_kvdf_params(&kvdf_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "KVDF")?;
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

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let lr = compute_log_returns_f64(close);
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(req.backend, Backend::CpuOnly) {
        None
    } else {
        prepared_cuda_inputs_for_source_ohlcv(candles, &req.ma_source)
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
    emit_progress(progress, 0, total_pairs, "kvdf_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_kvdf_periods(&kvdf_cfg, fast_len, slow_len);
            let signals = compute_kvdf_signals(
                candles,
                &req.ma_source,
                &req.backend,
                periods,
                &kvdf_cfg,
                #[cfg(feature = "cuda")]
                prepared_cuda_inputs.as_ref(),
            )?;
            let start_t =
                compute_kvdf_start_t(fv_close, signals.first_valid, strategy.trade_on_next_bar);
            if start_t >= n {
                continue;
            }

            let desired =
                build_kvdf_desired_positions(close, &signals, start_t, &strategy, &kvdf_cfg);
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
                emit_progress(progress, processed, total_pairs, "kvdf_optimize");
            }
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "kvdf_optimize");

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

    let mut result = finalize_standalone_optimization_result(
        agg,
        "KVDF grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_kvdf_periods(&kvdf_cfg, params.fast_len, params.slow_len);
        let signals = compute_kvdf_signals(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            &kvdf_cfg,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs.as_ref(),
        )?;
        let start_t =
            compute_kvdf_start_t(fv_close, signals.first_valid, strategy.trade_on_next_bar);
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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

    let matrix_index =
        DoubleMaFastLtSlowIndex::new(fast_periods.clone(), slow_periods.clone()).ok();
    let best_col = matrix_index
        .as_ref()
        .and_then(|idx| idx.col_id(best_params.fast_len, best_params.slow_len));
    let top_cols = matrix_index.as_ref().map(|idx| {
        top.iter()
            .filter_map(|(p, _)| idx.col_id(p.fast_len, p.slow_len))
            .collect::<Vec<_>>()
    });
    let top_cols = match top_cols {
        Some(v) if v.len() == top.len() => Some(v),
        _ => None,
    };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Cpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms: 0,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_cdmc_optimization(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let normalized_backend = req.backend.clone().with_preferred_cuda_device()?;
        if !matches!(normalized_backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            if vector_ta::cuda::cuda_available() {
                let device_id = match &normalized_backend {
                    Backend::Auto { device_id }
                    | Backend::GpuOnly { device_id }
                    | Backend::GpuHybridKernel { device_id }
                    | Backend::GpuKernel { device_id } => *device_id,
                    Backend::CpuOnly => 0,
                };
                match run_cdmc_optimization_gpu(&req, candles, cancel, progress, device_id) {
                    Ok(mut gpu_res) => {
                        if matches!(req.backend, Backend::Auto { .. }) {
                            gpu_res.backend_route = Some("Auto->GpuKernel".to_string());
                        } else if matches!(req.backend, Backend::GpuKernel { .. }) {
                            gpu_res.backend_route = Some("GpuKernel".to_string());
                        } else {
                            gpu_res.backend_route = Some(format!("{backend_label}->GpuKernel"));
                        }
                        return Ok(gpu_res);
                    }
                    Err(err) => {
                        let mut cpu_res =
                            run_cdmc_optimization_cpu(req, candles, cancel, progress)?;
                        cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
                        cpu_res.backend_fallback_reason = Some(err);
                        return Ok(cpu_res);
                    }
                }
            }

            let mut cpu_res = run_cdmc_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason =
                Some("CUDA unavailable; falling back to CPU CDMC optimization.".to_string());
            return Ok(cpu_res);
        }
    }

    #[cfg(all(feature = "cuda", not(feature = "cuda-backtest-kernel")))]
    {
        if !matches!(req.backend, Backend::CpuOnly) {
            let backend_label = composite_backend_label(&req.backend).to_string();
            let mut cpu_res = run_cdmc_optimization_cpu(req, candles, cancel, progress)?;
            cpu_res.backend_route = Some(format!("{backend_label}->CpuOnly"));
            cpu_res.backend_fallback_reason = Some(
                "CDMC GPU kernel backend is disabled (build without `cuda-backtest-kernel`)."
                    .to_string(),
            );
            return Ok(cpu_res);
        }
    }

    run_cdmc_optimization_cpu(req, candles, cancel, progress)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn run_cdmc_optimization_gpu(
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
    let cdmc_cfg = req.cdmc.clone().unwrap_or_default();
    validate_cdmc_params(&cdmc_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "CDMC")?;
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
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;

    let prepared_cuda_inputs =
        prepared_cuda_inputs_for_source_slice_on_device(candles, "close", device_id as usize)?;

    let mut dpo_keys: Vec<u16> = fast_periods
        .iter()
        .map(|&fast_len| resolve_cdmc_periods(&cdmc_cfg, fast_len, 1).dpo)
        .collect();
    dpo_keys.sort_unstable();
    dpo_keys.dedup();
    if dpo_keys.is_empty() {
        return Err("CDMC DPO period set is empty".to_string());
    }
    let dpo_min = *dpo_keys.first().unwrap_or(&1);
    let dpo_max = *dpo_keys.last().unwrap_or(&dpo_min);
    let dpo_sweep = compute_cdmc_dpo_sweep_cuda_device(&prepared_cuda_inputs, dpo_min, dpo_max, 1)?;
    if dpo_sweep.matrix.cols != n {
        return Err("CDMC DPO sweep column mismatch".to_string());
    }
    let mut dpo_row_lookup: HashMap<u16, i32> = HashMap::with_capacity(dpo_keys.len());
    let mut dpo_first_valid_lookup: HashMap<u16, usize> = HashMap::with_capacity(dpo_keys.len());
    for period in dpo_keys.iter().copied() {
        let row = dpo_sweep
            .row_index_for_period(period)
            .ok_or_else(|| format!("CDMC DPO row missing for period={period}"))?;
        let row_i = i32::try_from(row).map_err(|_| "CDMC DPO row exceeds i32 range".to_string())?;
        dpo_row_lookup.insert(period, row_i);
        dpo_first_valid_lookup.insert(period, dpo_sweep.first_valid_for_period(period));
    }

    let mut cycle_keys: Vec<u16> = slow_periods
        .iter()
        .map(|&slow_len| resolve_cdmc_periods(&cdmc_cfg, 1, slow_len).correlation_cycle)
        .collect();
    cycle_keys.sort_unstable();
    cycle_keys.dedup();
    if cycle_keys.is_empty() {
        return Err("CDMC correlation-cycle period set is empty".to_string());
    }
    let cycle_min = *cycle_keys.first().unwrap_or(&1);
    let cycle_max = *cycle_keys.last().unwrap_or(&cycle_min);
    let angle_sweep = compute_cdmc_correlation_cycle_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["angle"],
        cycle_min,
        cycle_max,
        1,
    )?;
    let state_sweep = compute_cdmc_correlation_cycle_output_sweep_cuda_device(
        &prepared_cuda_inputs,
        &["state"],
        cycle_min,
        cycle_max,
        1,
    )?;
    if angle_sweep.matrix.cols != n || state_sweep.matrix.cols != n {
        return Err("CDMC correlation-cycle sweep column mismatch".to_string());
    }
    if angle_sweep.matrix.rows != state_sweep.matrix.rows {
        return Err("CDMC correlation-cycle sweep row mismatch".to_string());
    }
    let mut cycle_row_lookup: HashMap<u16, i32> = HashMap::with_capacity(cycle_keys.len());
    let mut cycle_first_valid_lookup: HashMap<u16, usize> =
        HashMap::with_capacity(cycle_keys.len());
    for period in cycle_keys.iter().copied() {
        let row = angle_sweep
            .row_index_for_period(period)
            .ok_or_else(|| format!("CDMC correlation-cycle row missing for period={period}"))?;
        if state_sweep.row_index_for_period(period).is_none() {
            return Err(format!(
                "CDMC correlation-cycle state row missing for period={period}"
            ));
        }
        let row_i = i32::try_from(row)
            .map_err(|_| "CDMC correlation-cycle row exceeds i32 range".to_string())?;
        cycle_row_lookup.insert(period, row_i);
        cycle_first_valid_lookup.insert(period, angle_sweep.first_valid_for_period(period));
    }

    let mut decycler_keys: Vec<u16> = slow_periods
        .iter()
        .map(|&slow_len| resolve_cdmc_periods(&cdmc_cfg, 1, slow_len).decycler)
        .collect();
    decycler_keys.sort_unstable();
    decycler_keys.dedup();
    if decycler_keys.is_empty() {
        return Err("CDMC decycler period set is empty".to_string());
    }
    let decycler_min = *decycler_keys.first().unwrap_or(&1);
    let decycler_max = *decycler_keys.last().unwrap_or(&decycler_min);
    let decycler_sweep = compute_cdmc_decycler_sweep_cuda_device(
        &prepared_cuda_inputs,
        decycler_min,
        decycler_max,
        1,
    )?;
    if decycler_sweep.matrix.cols != n {
        return Err("CDMC decycler sweep column mismatch".to_string());
    }
    let mut decycler_row_lookup: HashMap<u16, i32> = HashMap::with_capacity(decycler_keys.len());
    let mut decycler_first_valid_lookup: HashMap<u16, usize> =
        HashMap::with_capacity(decycler_keys.len());
    for period in decycler_keys.iter().copied() {
        let row = decycler_sweep
            .row_index_for_period(period)
            .ok_or_else(|| format!("CDMC decycler row missing for period={period}"))?;
        let row_i =
            i32::try_from(row).map_err(|_| "CDMC decycler row exceeds i32 range".to_string())?;
        decycler_row_lookup.insert(period, row_i);
        decycler_first_valid_lookup.insert(period, decycler_sweep.first_valid_for_period(period));
    }

    let ((mama_matrix, fama_matrix), mama_first_valid) =
        compute_cdmc_mama_pair_cuda_device(&prepared_cuda_inputs, &cdmc_cfg)?;
    let fama_first_valid = mama_first_valid;
    if mama_matrix.cols != n || fama_matrix.cols != n {
        return Err("CDMC MAMA/FAMA column mismatch".to_string());
    }
    if mama_matrix.rows != 1 || fama_matrix.rows != 1 {
        return Err("CDMC MAMA/FAMA device output must be single-row".to_string());
    }
    if dpo_sweep.matrix.device_id != device_id
        || angle_sweep.matrix.device_id != device_id
        || state_sweep.matrix.device_id != device_id
        || decycler_sweep.matrix.device_id != device_id
        || mama_matrix.device_id != device_id
        || fama_matrix.device_id != device_id
    {
        return Err("CDMC GPU signal staging device mismatch".to_string());
    }

    let mut combo_dpo_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_cycle_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_decycler_row_idx: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_start_t: Vec<i32> = Vec::with_capacity(total_pairs);
    let mut combo_params: Vec<DoubleMaParams> = Vec::with_capacity(total_pairs);

    emit_progress(progress, 0, total_pairs, "cdmc_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_cdmc_periods(&cdmc_cfg, fast_len, slow_len);
            let dpo_row = *dpo_row_lookup
                .get(&periods.dpo)
                .ok_or_else(|| "CDMC DPO row lookup miss".to_string())?;
            let cycle_row = *cycle_row_lookup
                .get(&periods.correlation_cycle)
                .ok_or_else(|| "CDMC correlation-cycle row lookup miss".to_string())?;
            let decycler_row = *decycler_row_lookup
                .get(&periods.decycler)
                .ok_or_else(|| "CDMC decycler row lookup miss".to_string())?;
            let dpo_first_valid = *dpo_first_valid_lookup
                .get(&periods.dpo)
                .ok_or_else(|| "CDMC DPO first_valid lookup miss".to_string())?;
            let cycle_first_valid = *cycle_first_valid_lookup
                .get(&periods.correlation_cycle)
                .ok_or_else(|| "CDMC correlation-cycle first_valid lookup miss".to_string())?;
            let decycler_first_valid = *decycler_first_valid_lookup
                .get(&periods.decycler)
                .ok_or_else(|| "CDMC decycler first_valid lookup miss".to_string())?;
            let signals_first_valid = dpo_first_valid
                .max(cycle_first_valid)
                .max(mama_first_valid)
                .max(fama_first_valid)
                .max(decycler_first_valid);
            let start_t =
                compute_cdmc_start_t(fv_close, signals_first_valid, strategy.trade_on_next_bar);
            if start_t >= n {
                continue;
            }

            combo_dpo_row_idx.push(dpo_row);
            combo_cycle_row_idx.push(cycle_row);
            combo_decycler_row_idx.push(decycler_row);
            combo_start_t.push(
                i32::try_from(start_t).map_err(|_| "CDMC start_t exceeds i32 range".to_string())?,
            );
            combo_params.push(DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            });

            if (combo_params.len() & 0x03ff) == 0 {
                emit_progress(progress, combo_params.len(), total_pairs, "cdmc_optimize");
            }
        }
    }
    emit_progress(progress, combo_params.len(), total_pairs, "cdmc_optimize");

    if combo_params.is_empty() {
        return Err("CDMC grid search produced no valid parameter combinations".to_string());
    }

    let signals = CdmcGpuDeviceSignalMatrices {
        close_f32: prepared_cuda_inputs.close(),
        close_device_ptr: prepared_cuda_inputs.close_device().device_ptr(),
        cols: n,
        dpo_row_major_ptr: dpo_sweep.matrix.device_ptr,
        dpo_rows: dpo_sweep.matrix.rows,
        dpo_stride: dpo_sweep.matrix.cols,
        cycle_angle_row_major_ptr: angle_sweep.matrix.device_ptr,
        cycle_state_row_major_ptr: state_sweep.matrix.device_ptr,
        cycle_rows: angle_sweep.matrix.rows,
        cycle_stride: angle_sweep.matrix.cols,
        mama_ptr: mama_matrix.device_ptr,
        fama_ptr: fama_matrix.device_ptr,
        decycler_row_major_ptr: decycler_sweep.matrix.device_ptr,
        decycler_rows: decycler_sweep.matrix.rows,
        decycler_stride: decycler_sweep.matrix.cols,
    };
    let combo_end_t = vec![
        i32::try_from(n)
            .map_err(|_| "CDMC end_t exceeds i32 range".to_string())?;
        combo_dpo_row_idx.len()
    ];
    let combos = CdmcGpuComboBatch {
        dpo_row_index: &combo_dpo_row_idx,
        cycle_row_index: &combo_cycle_row_idx,
        decycler_row_index: &combo_decycler_row_idx,
        start_t: &combo_start_t,
        end_t: &combo_end_t,
    };

    let start = std::time::Instant::now();
    let metrics = eval_cdmc_metrics_gpu_kernel_device_rows(
        device_id, &signals, &combos, &strategy, &cdmc_cfg,
    )?;
    if metrics.len() != combo_params.len() {
        return Err("CDMC GPU kernel output length mismatch".to_string());
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
            emit_progress(progress, i + 1, total_pairs, "cdmc_optimize");
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "cdmc_optimize");

    let mut result = finalize_standalone_optimization_result(
        agg,
        "CDMC grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_cdmc_periods(&cdmc_cfg, params.fast_len, params.slow_len);
        let dpo_first_valid = *dpo_first_valid_lookup
            .get(&periods.dpo)
            .ok_or_else(|| "CDMC DPO first_valid lookup miss".to_string())?;
        let cycle_first_valid = *cycle_first_valid_lookup
            .get(&periods.correlation_cycle)
            .ok_or_else(|| "CDMC correlation-cycle first_valid lookup miss".to_string())?;
        let decycler_first_valid = *decycler_first_valid_lookup
            .get(&periods.decycler)
            .ok_or_else(|| "CDMC decycler first_valid lookup miss".to_string())?;
        let signals_first_valid = dpo_first_valid
            .max(cycle_first_valid)
            .max(mama_first_valid)
            .max(fama_first_valid)
            .max(decycler_first_valid);
        let start_t =
            compute_cdmc_start_t(fv_close, signals_first_valid, strategy.trade_on_next_bar);
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
        best_metrics: result.best_metrics,
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Gpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap: result.heatmap.clone(),
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_cdmc_optimization_cpu(
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
    let cdmc_cfg = req.cdmc.clone().unwrap_or_default();
    validate_cdmc_params(&cdmc_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "CDMC")?;
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

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let lr = compute_log_returns_f64(close);
    let n = close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(req.backend, Backend::CpuOnly) {
        None
    } else {
        prepared_cuda_inputs_for_source_slice(candles, &req.ma_source)
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
    emit_progress(progress, 0, total_pairs, "cdmc_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_cdmc_periods(&cdmc_cfg, fast_len, slow_len);
            let signals = compute_cdmc_signals(
                candles,
                &req.ma_source,
                &req.backend,
                periods,
                &cdmc_cfg,
                #[cfg(feature = "cuda")]
                prepared_cuda_inputs.as_ref(),
            )?;
            let start_t =
                compute_cdmc_start_t(fv_close, signals.first_valid, strategy.trade_on_next_bar);
            if start_t >= n {
                continue;
            }

            let desired =
                build_cdmc_desired_positions(close, &signals, start_t, &strategy, &cdmc_cfg);
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
                emit_progress(progress, processed, total_pairs, "cdmc_optimize");
            }
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "cdmc_optimize");

    let mut result = finalize_standalone_optimization_result(
        agg,
        "CDMC grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, &req, objective, |params| {
        let periods = resolve_cdmc_periods(&cdmc_cfg, params.fast_len, params.slow_len);
        let signals = compute_cdmc_signals(
            candles,
            &req.ma_source,
            &req.backend,
            periods,
            &cdmc_cfg,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs.as_ref(),
        )?;
        let start_t =
            compute_cdmc_start_t(fv_close, signals.first_valid, strategy.trade_on_next_bar);
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
    let heatmap = result.heatmap.clone();

    let matrix_index =
        DoubleMaFastLtSlowIndex::new(fast_periods.clone(), slow_periods.clone()).ok();
    let best_col = matrix_index
        .as_ref()
        .and_then(|idx| idx.col_id(best_params.fast_len, best_params.slow_len));
    let top_cols = matrix_index.as_ref().map(|idx| {
        top.iter()
            .filter_map(|(p, _)| idx.col_id(p.fast_len, p.slow_len))
            .collect::<Vec<_>>()
    });
    let top_cols = match top_cols {
        Some(v) if v.len() == top.len() => Some(v),
        _ => None,
    };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        best_score: result.best_score,
        mode_used,
        backend_used: BackendUsed::Cpu,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms: 0,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

fn run_mkcs_optimization_cpu(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    run_mkcs_optimization_impl(
        &req,
        candles,
        cancel,
        progress,
        &Backend::CpuOnly,
        BackendUsed::Cpu,
    )
}

fn run_mkcs_optimization_impl(
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
    let mkcs_cfg = req.mkcs.clone().unwrap_or_default();
    validate_mkcs_params(&mkcs_cfg)?;

    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    let mode_used = resolve_composite_grid_mode(&req.mode, "MKCS")?;
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

    let mut compression_cache: HashMap<u16, crate::mkcs::MkcsCompressionSignals> = HashMap::new();
    let mut breakout_cache: HashMap<u16, crate::mkcs::MkcsBreakoutSignals> = HashMap::new();
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
    emit_progress(progress, 0, total_pairs, "mkcs_optimize");
    for &fast_len in &fast_periods {
        for &slow_len in &slow_periods {
            if fast_len >= slow_len {
                continue;
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let periods = resolve_mkcs_periods(&mkcs_cfg, fast_len, slow_len);
            let compression = if let Some(v) = compression_cache.get(&periods.stddev) {
                v
            } else {
                let built = compute_mkcs_compression_signals(
                    candles,
                    "close",
                    indicator_backend,
                    periods.stddev,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                compression_cache.insert(periods.stddev, built);
                compression_cache
                    .get(&periods.stddev)
                    .ok_or_else(|| "MKCS compression cache insert failed".to_string())?
            };
            let breakout = if let Some(v) = breakout_cache.get(&periods.breakout) {
                v
            } else {
                let built = compute_mkcs_breakout_signals(
                    candles,
                    "close",
                    indicator_backend,
                    periods.breakout,
                    #[cfg(feature = "cuda")]
                    prepared_cuda_inputs.as_ref(),
                )?;
                breakout_cache.insert(periods.breakout, built);
                breakout_cache
                    .get(&periods.breakout)
                    .ok_or_else(|| "MKCS breakout cache insert failed".to_string())?
            };

            let start_t = compute_mkcs_start_t(
                fv_close,
                fv_high,
                fv_low,
                compression.first_valid,
                breakout.first_valid,
                strategy.trade_on_next_bar,
            );
            if start_t >= n {
                continue;
            }

            let desired = build_mkcs_desired_positions(
                close,
                compression,
                breakout,
                start_t,
                &strategy,
                &mkcs_cfg,
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
            processed += 1;
            if (processed & 0x03ff) == 0 {
                emit_progress(progress, processed, total_pairs, "mkcs_optimize");
            }
        }
    }
    emit_progress(progress, total_pairs, total_pairs, "mkcs_optimize");

    let mut result = finalize_standalone_optimization_result(
        agg,
        "MKCS grid search produced no result",
        &req,
        objective,
    )?;
    apply_requested_standalone_confidence_rerank(&mut result, req, objective, |params| {
        let periods = resolve_mkcs_periods(&mkcs_cfg, params.fast_len, params.slow_len);
        let compression = compression_cache
            .get(&periods.stddev)
            .ok_or_else(|| "MKCS compression cache miss".to_string())?;
        let breakout = breakout_cache
            .get(&periods.breakout)
            .ok_or_else(|| "MKCS breakout cache miss".to_string())?;
        let start_t = compute_mkcs_start_t(
            fv_close,
            fv_high,
            fv_low,
            compression.first_valid,
            breakout.first_valid,
            strategy.trade_on_next_bar,
        );
        Ok(n.saturating_sub(start_t).max(2))
    })?;
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
    let heatmap = result.heatmap.clone();
    let matrix_index =
        DoubleMaFastLtSlowIndex::new(fast_periods.clone(), slow_periods.clone()).ok();
    let best_col = matrix_index
        .as_ref()
        .and_then(|idx| idx.col_id(best_params.fast_len, best_params.slow_len));
    let top_cols = matrix_index.as_ref().map(|idx| {
        top.iter()
            .filter_map(|(p, _)| idx.col_id(p.fast_len, p.slow_len))
            .collect::<Vec<_>>()
    });
    let top_cols = match top_cols {
        Some(v) if v.len() == top.len() => Some(v),
        _ => None,
    };

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        best_score: result.best_score,
        mode_used,
        backend_used,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms: 0,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route: None,
        backend_fallback_reason: None,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
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
        + usize::from(req.rocb.is_some())
        + usize::from(req.vcet.is_some())
        + usize::from(req.mkcs.is_some())
        + usize::from(req.kqsr.is_some())
        + usize::from(req.carm.is_some())
        + usize::from(req.kvdf.is_some())
        + usize::from(req.cdmc.is_some());
    if custom_strategy_count > 1 {
        return Err(
            "Select only one of ASR, VCC, VEMR, ROCB, VCET, MKCS, KQSR, CARM, KVDF, or CDMC strategy config.".to_string(),
        );
    }
    validate_standalone_easy_shortlist_request(&req)?;
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
    if req.vcet.is_some() {
        return run_vcet_optimization(req, candles, cancel, progress);
    }
    if req.mkcs.is_some() {
        return run_mkcs_optimization(req, candles, cancel, progress);
    }
    if req.kqsr.is_some() {
        return run_kqsr_optimization(req, candles, cancel, progress);
    }
    if req.carm.is_some() {
        return run_carm_optimization(req, candles, cancel, progress);
    }
    if req.kvdf.is_some() {
        return run_kvdf_optimization(req, candles, cancel, progress);
    }
    if req.cdmc.is_some() {
        return run_cdmc_optimization(req, candles, cancel, progress);
    }

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    #[cfg(feature = "cuda")]
    let normalized_backend = req.backend.clone().with_preferred_cuda_device()?;
    #[cfg(not(feature = "cuda"))]
    let _normalized_backend = req.backend.clone();

    let mut backend_route: Option<String> = None;
    let mut backend_fallback_reason: Option<String> = None;

    #[cfg(feature = "cuda")]
    let backend_label = composite_backend_label(&req.backend).to_string();

    #[cfg(feature = "cuda")]
    let prepared_cuda_inputs = if matches!(normalized_backend, Backend::CpuOnly) {
        None
    } else if vector_ta::cuda::cuda_available() {
        let device_id = backend_device_id(&normalized_backend).unwrap_or(0);
        match PreparedCudaIndicatorInputs::from_candles_on_device(
            candles,
            &req.ma_source,
            device_id,
        ) {
            Ok(inputs) => {
                if matches!(req.backend, Backend::Auto { .. }) {
                    backend_route = Some(format!(
                        "Auto->{}",
                        composite_profile_runtime_label(&req.strategy)
                    ));
                } else {
                    backend_route = Some(format!(
                        "{backend_label}->{}",
                        composite_profile_runtime_label(&req.strategy)
                    ));
                }
                Some(inputs)
            }
            Err(err) => {
                backend_route = Some(format!("{backend_label}->CpuOnly"));
                backend_fallback_reason = Some(format!(
                    "failed to prepare CUDA composite inputs for target {}; falling back to CPU: {err}",
                    composite_profile_gpu_target_label(&req.strategy)
                ));
                None
            }
        }
    } else {
        backend_route = Some(format!("{backend_label}->CpuOnly"));
        backend_fallback_reason = Some(format!(
            "CUDA unavailable; falling back to CPU composite optimization (target route: {}).",
            composite_profile_gpu_target_label(&req.strategy)
        ));
        None
    };

    #[cfg(not(feature = "cuda"))]
    if !matches!(req.backend, Backend::CpuOnly) {
        backend_route = Some(format!(
            "{}->CpuOnly",
            composite_backend_label(&req.backend)
        ));
        backend_fallback_reason = Some(format!(
            "Composite GPU path requires building with `cuda` support (target route: {}).",
            composite_profile_gpu_target_label(&req.strategy)
        ));
    }

    #[cfg(feature = "cuda")]
    let indicator_backend = if prepared_cuda_inputs.is_some() {
        normalized_backend.clone()
    } else {
        Backend::CpuOnly
    };
    #[cfg(not(feature = "cuda"))]
    let indicator_backend = Backend::CpuOnly;

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
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: prescan_mode,
            top_k: Some(prescan_top_k),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: crate::double_ma::GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
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
    let mut first_valid = fv_close.max(fv_ma);

    let halftrend_trend = compute_halftrend_trend_with_backend(
        candles,
        &req.halftrend,
        #[cfg(feature = "cuda")]
        &indicator_backend,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs.as_ref(),
    )?;
    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }
    let fv_ht = first_finite_idx(&halftrend_trend)
        .ok_or_else(|| "halftrend trend series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_ht);

    let macz_hist = compute_macz_hist_with_backend(
        candles,
        &ma_source,
        &req.macz,
        #[cfg(feature = "cuda")]
        &indicator_backend,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs.as_ref(),
    )?;
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }
    let fv_macz =
        first_finite_idx(&macz_hist).ok_or_else(|| "macz series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_macz);

    let ott_line = compute_ott_line_with_backend(
        candles,
        &ma_source,
        &req.ott,
        #[cfg(feature = "cuda")]
        &indicator_backend,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs.as_ref(),
    )?;
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

    let run_cpu_path = || {
        optimize_composite_cpu_tiled(
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
            &indicator_backend,
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs.as_ref(),
            cancel,
            progress,
        )
    };

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let mut backend_used = BackendUsed::Cpu;
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let backend_used = BackendUsed::Cpu;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let mut base_gpu_kernel_fallback_reason = None::<String>;

    let result = {
        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        {
            if matches!(req.backend, Backend::GpuKernel { .. })
                && composite_profile_from_strategy(&req.strategy)
                    == StrategyProfileKind::CompositeCpu
            {
                match try_optimize_base_composite_gpu_kernel(
                    &req,
                    candles,
                    &fast_periods,
                    &slow_periods,
                    &fast_ma,
                    &slow_ma,
                    &ma_source,
                    first_valid,
                    &halftrend_trend,
                    &macz_hist,
                    ma_prices,
                    &ott_line,
                    &strategy,
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
                    &indicator_backend,
                    cancel,
                    progress,
                ) {
                    Ok(gpu_result) => {
                        backend_used = BackendUsed::Gpu;
                        backend_route = Some(composite_gpu_kernel_route_label(&req.backend));
                        backend_fallback_reason = None;
                        gpu_result
                    }
                    Err(reason) => {
                        base_gpu_kernel_fallback_reason = Some(reason);
                        run_cpu_path()?
                    }
                }
            } else {
                run_cpu_path()?
            }
        }
        #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
        {
            run_cpu_path()?
        }
    };

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    if matches!(backend_used, BackendUsed::Cpu) {
        if let Some(reason) = base_gpu_kernel_fallback_reason.take() {
            if let Some(existing) = backend_fallback_reason.as_mut() {
                if !existing.is_empty() {
                    existing.push(' ');
                }
                existing.push_str(&reason);
            } else {
                backend_fallback_reason = Some(reason);
            }
        }
    }

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
        best_score: result.best_score,
        mode_used,
        backend_used,
        matrix_index,
        best_col,
        top_cols,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        top_scores: result.top_scores,
        all_scores: resolved_all_scores(&all, objective),
        all,
        heatmap,
        vdps: None,
        hybrid_timing_ms: None,
        gpu_kernel_timing_ms: None,
        backend_route,
        backend_fallback_reason,
        indicator_route_plan: None,
        residency: None,
        followup_cache_key: None,
        shortlist_slice_diagnostics: None,
        score_observation_context: None,
    })
}

pub fn compute_composite_cpu_drilldown_blocking_with_candles(
    req: CompositeCpuDrilldownRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<CompositeCpuDrilldownResult, String> {
    let custom_strategy_count = usize::from(req.asr.is_some())
        + usize::from(req.vcc.is_some())
        + usize::from(req.vemr.is_some())
        + usize::from(req.rocb.is_some())
        + usize::from(req.vcet.is_some())
        + usize::from(req.mkcs.is_some())
        + usize::from(req.kqsr.is_some())
        + usize::from(req.carm.is_some())
        + usize::from(req.kvdf.is_some())
        + usize::from(req.cdmc.is_some());
    if custom_strategy_count > 1 {
        return Err(
            "Select only one of ASR, VCC, VEMR, ROCB, VCET, MKCS, KQSR, CARM, KVDF, or CDMC strategy config.".to_string(),
        );
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
    if req.vcet.is_some() {
        return Err("VCET drilldown is not implemented yet.".to_string());
    }
    if req.mkcs.is_some() {
        return Err("MKCS drilldown is not implemented yet.".to_string());
    }
    if req.kqsr.is_some() {
        return Err("KQSR drilldown is not implemented yet.".to_string());
    }
    if req.carm.is_some() {
        return Err("CARM drilldown is not implemented yet.".to_string());
    }
    if req.kvdf.is_some() {
        return Err("KVDF drilldown is not implemented yet.".to_string());
    }
    if req.cdmc.is_some() {
        return Err("CDMC drilldown is not implemented yet.".to_string());
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
        + usize::from(req.rocb.is_some())
        + usize::from(req.vcet.is_some())
        + usize::from(req.mkcs.is_some())
        + usize::from(req.kqsr.is_some())
        + usize::from(req.carm.is_some())
        + usize::from(req.kvdf.is_some())
        + usize::from(req.cdmc.is_some());
    if custom_strategy_count > 1 {
        return Err(
            "Select only one of ASR, VCC, VEMR, ROCB, VCET, MKCS, KQSR, CARM, KVDF, or CDMC strategy config.".to_string(),
        );
    }
    if req.vemr.is_some() {
        return run_vemr_local_stability(req, candles);
    }
    if req.vcc.is_some() {
        return run_vcc_local_stability(req, candles);
    }
    if req.asr.is_some() {
        return run_asr_local_stability(req, candles);
    }
    if req.rocb.is_some() {
        return run_rocb_local_stability(req, candles);
    }
    if req.vcet.is_some() {
        return Err("VCET local stability is not implemented yet.".to_string());
    }
    if req.mkcs.is_some() {
        return Err("MKCS local stability is not implemented yet.".to_string());
    }
    if req.kqsr.is_some() {
        return Err("KQSR local stability is not implemented yet.".to_string());
    }
    if req.carm.is_some() {
        return Err("CARM local stability is not implemented yet.".to_string());
    }
    if req.kvdf.is_some() {
        return Err("KVDF local stability is not implemented yet.".to_string());
    }
    if req.cdmc.is_some() {
        return Err("CDMC local stability is not implemented yet.".to_string());
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

pub(crate) fn compute_halftrend_trend_with_backend(
    candles: &vector_ta::utilities::data_loader::Candles,
    filter: &HalfTrendRegimeFilter,
    #[cfg(feature = "cuda")] backend: &Backend,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f64>, String> {
    #[cfg(feature = "cuda")]
    {
        let output_id = resolve_output_id("halftrend", &["trend", "value"])?;
        let params = build_halftrend_params(filter);
        if let Ok(values) = try_compute_indicator_series_cuda_f64(
            "halftrend",
            output_id.as_str(),
            params.as_slice(),
            backend,
            prepared_cuda_inputs,
        ) {
            return Ok(values);
        }
        return compute_halftrend_trend(candles, filter);
    }

    #[cfg(not(feature = "cuda"))]
    {
        compute_halftrend_trend(candles, filter)
    }
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

pub(crate) fn compute_macz_hist_with_backend(
    candles: &vector_ta::utilities::data_loader::Candles,
    source: &str,
    p: &MaczGateParams,
    #[cfg(feature = "cuda")] backend: &Backend,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f64>, String> {
    #[cfg(feature = "cuda")]
    {
        validate_macz_params(p)?;
        let output_id = resolve_output_id("macz", &["hist", "histogram", "value"])?;
        let params = build_macz_params(p);
        if let Ok(values) = try_compute_indicator_series_cuda_f64(
            "macz",
            output_id.as_str(),
            params.as_slice(),
            backend,
            prepared_cuda_inputs,
        ) {
            return Ok(values);
        }
        return compute_macz_hist(candles, source, p);
    }

    #[cfg(not(feature = "cuda"))]
    {
        compute_macz_hist(candles, source, p)
    }
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

pub(crate) fn compute_ott_line_with_backend(
    candles: &vector_ta::utilities::data_loader::Candles,
    source: &str,
    p: &OttGateParams,
    #[cfg(feature = "cuda")] backend: &Backend,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f64>, String> {
    #[cfg(feature = "cuda")]
    {
        validate_ott_params(p)?;
        let output_id = resolve_output_id("ott", &["ott", "line", "value"])?;
        let params = build_ott_params(p);
        if let Ok(values) = try_compute_indicator_series_cuda_f64(
            "ott",
            output_id.as_str(),
            params.as_slice(),
            backend,
            prepared_cuda_inputs,
        ) {
            return Ok(values);
        }
        return compute_ott_line(candles, source, p);
    }

    #[cfg(not(feature = "cuda"))]
    {
        compute_ott_line(candles, source, p)
    }
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
    indicator_backend: &Backend,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
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
    #[cfg(feature = "cuda")]
    let ma_source_f32 = prepared_cuda_inputs.map(|inputs| inputs.source());

    for f_chunk in fast_periods.chunks(pf_tile) {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let (fast_mat, cols) = ma_matrix_with_backend_f64(
            candles,
            fast_ma_type,
            ma_source,
            f_chunk,
            fast_ma_params,
            fast_alma_offset,
            fast_alma_sigma,
            Some(indicator_backend),
            #[cfg(feature = "cuda")]
            ma_source_f32,
            #[cfg(not(feature = "cuda"))]
            None,
        )?;
        for s_chunk in slow_periods.chunks(ps_tile) {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }
            let (slow_mat, cols2) = ma_matrix_with_backend_f64(
                candles,
                slow_ma_type,
                ma_source,
                s_chunk,
                slow_ma_params,
                slow_alma_offset,
                slow_alma_sigma,
                Some(indicator_backend),
                #[cfg(feature = "cuda")]
                ma_source_f32,
                #[cfg(not(feature = "cuda"))]
                None,
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

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn assert_metric_major_parity(label: &str, metric: &str, cpu: f64, gpu: f64) {
        if cpu.is_nan() || gpu.is_nan() {
            assert!(
                cpu.is_nan() && gpu.is_nan(),
                "{label} {metric} NaN mismatch: cpu={cpu}, gpu={gpu}"
            );
            return;
        }
        let abs_tol: f64 = match metric {
            "sharpe" => 0.25,
            "pnl" | "max_dd" | "volatility" | "exposure" | "net_exposure" => 0.03,
            _ => 0.05,
        };
        let rel_tol: f64 = match metric {
            "sharpe" => 0.12,
            "pnl" | "max_dd" | "volatility" | "exposure" | "net_exposure" => 0.08,
            _ => 0.10,
        };
        let scale = cpu.abs().max(gpu.abs()).max(1.0);
        let limit = abs_tol.max(rel_tol * scale);
        let diff = (cpu - gpu).abs();
        assert!(
            diff <= limit,
            "{label} {metric} diverged: cpu={cpu}, gpu={gpu}, diff={diff}, limit={limit}"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn assert_composite_gpu_kernel_major_parity(
        mut req: CompositeCpuRequest,
        candles: &vector_ta::utilities::data_loader::Candles,
        cancel: &AtomicBool,
        label: &str,
    ) {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        req.include_all = Some(true);
        req.top_k = Some(128);
        req.heatmap_bins = Some(0);
        let objective = req.objective;

        let mut cpu_req = req.clone();
        cpu_req.backend = Backend::CpuOnly;
        let cpu_res =
            run_composite_cpu_optimization_blocking_with_candles(cpu_req, candles, cancel, None)
                .unwrap_or_else(|e| panic!("{label} cpu optimization failed: {e}"));

        let mut gpu_req = req;
        gpu_req.backend = Backend::GpuKernel { device_id: 0 };
        let gpu_res =
            run_composite_cpu_optimization_blocking_with_candles(gpu_req, candles, cancel, None)
                .unwrap_or_else(|e| panic!("{label} gpu optimization failed: {e}"));

        assert_include_all_scores_match_rows(&cpu_res, objective, &format!("{label} cpu"));
        assert_include_all_scores_match_rows(&gpu_res, objective, &format!("{label} gpu"));

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "{label} expected GPU backend, got {:?} route={:?} fallback={:?}",
            gpu_res.backend_used,
            gpu_res.backend_route,
            gpu_res.backend_fallback_reason
        );
        assert_eq!(
            cpu_res.num_combos, gpu_res.num_combos,
            "{label} combo count"
        );

        let cpu_all = cpu_res.all.expect("cpu all");
        let gpu_all = gpu_res.all.expect("gpu all");
        assert_eq!(cpu_all.len(), gpu_all.len(), "{label} all-row count");

        let cpu_map: std::collections::HashMap<(u16, u16), Metrics> = cpu_all
            .into_iter()
            .map(|(params, metrics)| ((params.fast_len, params.slow_len), metrics))
            .collect();
        let gpu_map: std::collections::HashMap<(u16, u16), Metrics> = gpu_all
            .into_iter()
            .map(|(params, metrics)| ((params.fast_len, params.slow_len), metrics))
            .collect();
        assert_eq!(cpu_map.len(), gpu_map.len(), "{label} map size");

        for (key, cpu_metrics) in &cpu_map {
            let gpu_metrics = gpu_map
                .get(key)
                .unwrap_or_else(|| panic!("{label} missing GPU metrics for combo {:?}", key));
            assert_metric_major_parity(label, "pnl", cpu_metrics.pnl, gpu_metrics.pnl);
            assert_metric_major_parity(label, "sharpe", cpu_metrics.sharpe, gpu_metrics.sharpe);
            assert_metric_major_parity(
                label,
                "volatility",
                cpu_metrics.volatility,
                gpu_metrics.volatility,
            );
            assert_metric_major_parity(label, "max_dd", cpu_metrics.max_dd, gpu_metrics.max_dd);
            assert_metric_major_parity(
                label,
                "exposure",
                cpu_metrics.exposure,
                gpu_metrics.exposure,
            );
            assert_metric_major_parity(
                label,
                "net_exposure",
                cpu_metrics.net_exposure,
                gpu_metrics.net_exposure,
            );
            assert!(
                cpu_metrics.trades.abs_diff(gpu_metrics.trades) <= 2,
                "{label} trades diverged for {:?}: cpu={}, gpu={}",
                key,
                cpu_metrics.trades,
                gpu_metrics.trades
            );
        }

        assert_metric_major_parity(
            label,
            "best_sharpe",
            cpu_res.best_metrics.sharpe,
            gpu_res.best_metrics.sharpe,
        );
        assert_metric_major_parity(
            label,
            "best_pnl",
            cpu_res.best_metrics.pnl,
            gpu_res.best_metrics.pnl,
        );
        assert_metric_major_parity(
            label,
            "best_max_dd",
            cpu_res.best_metrics.max_dd,
            gpu_res.best_metrics.max_dd,
        );
    }

    fn assert_include_all_scores_match_rows(
        res: &BackendOptimizationResult,
        objective: ObjectiveKind,
        label: &str,
    ) {
        let rows = res
            .all
            .as_ref()
            .unwrap_or_else(|| panic!("{label} include_all rows missing"));
        let scores = res
            .all_scores
            .as_ref()
            .unwrap_or_else(|| panic!("{label} include_all scores missing"));
        assert_eq!(
            rows.len(),
            scores.len(),
            "{label} all_scores length should match all-row length"
        );
        for ((_, metrics), score) in rows.iter().zip(scores.iter()) {
            let expected = objective_score(metrics, objective);
            assert!(
                expected.total_cmp(score) == std::cmp::Ordering::Equal,
                "{label} score mismatch: expected={expected}, actual={score}"
            );
        }
    }

    fn make_base_composite_req() -> CompositeCpuRequest {
        CompositeCpuRequest {
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
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

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn make_mkcs_breakout_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let len = len.max(160);
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let px = if i < 40 {
                100.0 + (i as f64 * 0.12).sin() * 0.08
            } else if i < 70 {
                100.8 + (i - 40) as f64 * 0.42
            } else if i < 100 {
                113.0 + ((i - 70) as f64 * 0.21).cos() * 0.12
            } else if i < 130 {
                112.0 + ((i - 100) as f64 * 0.15).sin() * 0.06
            } else {
                111.0 - (i - 130) as f64 * 0.34
            };
            open.push(px * 0.9995);
            high.push(px * 1.0025);
            low.push(px * 0.9975);
            close.push(px);
            vol.push(900.0 + (i % 11) as f64 * 27.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    fn make_kqsr_oscillating_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let len = len.max(240);
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let angle = (i as f64) * 0.18;
            let base = 100.0 + angle.sin() * 4.0 + (angle * 0.37).cos() * 1.2;
            let next = 100.0 + ((i + 1) as f64 * 0.18).sin() * 4.0;
            open.push(base - 0.25);
            high.push(base.max(next) + 0.9);
            low.push(base.min(next) - 0.9);
            close.push(base);
            vol.push(1_000.0 + (i % 13) as f64 * 35.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    fn make_carm_momentum_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let len = len.max(260);
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let px = if i < 70 {
                100.0 + (i as f64 * 0.12).sin() * 0.8 + i as f64 * 0.05
            } else if i < 140 {
                103.5 + (i - 70) as f64 * 0.21 + ((i - 70) as f64 * 0.16).sin() * 0.7
            } else if i < 200 {
                118.0 - (i - 140) as f64 * 0.18 + ((i - 140) as f64 * 0.22).cos() * 0.9
            } else {
                107.0 + (i - 200) as f64 * 0.14 + ((i - 200) as f64 * 0.14).sin() * 0.6
            };
            open.push(px * 0.998);
            high.push(px * 1.004);
            low.push(px * 0.996);
            close.push(px);
            vol.push(1_100.0 + (i % 17) as f64 * 28.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    fn make_kvdf_flow_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let len = len.max(280);
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64 * 60_000);
            let px = if i < 90 {
                100.0 + (i as f64 * 0.08).sin() * 0.9 + i as f64 * 0.03
            } else if i < 170 {
                102.8 + (i - 90) as f64 * 0.17 + ((i - 90) as f64 * 0.11).cos() * 0.8
            } else if i < 230 {
                116.0 - (i - 170) as f64 * 0.22 + ((i - 170) as f64 * 0.17).sin() * 1.1
            } else {
                102.4 + (i - 230) as f64 * 0.12 + ((i - 230) as f64 * 0.18).cos() * 0.7
            };
            open.push(px * 0.9985);
            high.push(px * 1.0045);
            low.push(px * 0.9955);
            close.push(px);
            vol.push(1_500.0 + (i % 19) as f64 * 55.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    fn make_cdmc_cycle_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let len = len.max(260);
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let angle = i as f64 * 0.11;
            let trend = if i < len / 2 {
                i as f64 * 0.035
            } else {
                (len - i) as f64 * 0.028
            };
            let px = 100.0 + angle.sin() * 3.6 + (angle * 0.43).cos() * 1.1 + trend;
            open.push(px * 0.9988);
            high.push(px * 1.0038);
            low.push(px * 0.9962);
            close.push(px);
            vol.push(1_050.0 + (i % 11) as f64 * 24.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn make_mkcs_req(backend: Backend) -> CompositeCpuRequest {
        let mut strategy = StrategyConfig::default();
        strategy.long_only = true;
        strategy.allow_flip = false;

        let mut mkcs = MkcsStrategyParams::default();
        mkcs.compression_threshold = 0.012;
        mkcs.reexpansion_threshold = 0.022;

        CompositeCpuRequest {
            backend,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            mkcs: Some(mkcs),
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
            data_id: "test".to_string(),
            fast_range: (5, 8, 1),
            slow_range: (10, 14, 2),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy,
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(8),
            include_all: Some(false),
            heatmap_bins: Some(0),
        }
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
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
    fn base_composite_hard_gate_regime_encoding_matches_double_ma_cpu_eval() {
        let candles = make_trending_candles(3000);
        let req = make_base_composite_req();
        let strategy = strategy_cfg_for_optimization(req.strategy.clone());
        let close = vector_ta::utilities::data_loader::source_type(&candles, "close");
        let ma_prices = vector_ta::utilities::data_loader::source_type(&candles, &req.ma_source);
        let lr = compute_log_returns_f64(close);

        let fv_close = first_finite_idx(close).expect("finite close");
        let fv_ma = first_finite_idx(ma_prices).expect("finite ma source");
        let mut first_valid = fv_close.max(fv_ma);

        let halftrend_trend =
            compute_halftrend_trend(&candles, &req.halftrend).expect("halftrend trend");
        first_valid = first_valid.max(first_finite_idx(&halftrend_trend).expect("finite ht"));

        let macz_hist = compute_macz_hist_with_backend(
            &candles,
            &req.ma_source,
            &req.macz,
            #[cfg(feature = "cuda")]
            &Backend::CpuOnly,
            #[cfg(feature = "cuda")]
            None,
        )
        .expect("macz hist");
        first_valid = first_valid.max(first_finite_idx(&macz_hist).expect("finite macz"));

        let ott_line = compute_ott_line_with_backend(
            &candles,
            &req.ma_source,
            &req.ott,
            #[cfg(feature = "cuda")]
            &Backend::CpuOnly,
            #[cfg(feature = "cuda")]
            None,
        )
        .expect("ott line");
        first_valid = first_valid.max(first_finite_idx(&ott_line).expect("finite ott"));

        let fast_periods = [5u16];
        let slow_periods = [10u16];
        let (fast_mat, fast_cols) = ma_matrix_cpu_f64(
            &candles,
            "sma",
            &req.ma_source,
            &fast_periods,
            None,
            0.85,
            6.0,
        )
        .expect("fast ma matrix");
        let (slow_mat, slow_cols) = ma_matrix_cpu_f64(
            &candles,
            "sma",
            &req.ma_source,
            &slow_periods,
            None,
            0.85,
            6.0,
        )
        .expect("slow ma matrix");
        assert_eq!(fast_cols, close.len());
        assert_eq!(slow_cols, close.len());
        let fast = &fast_mat[..fast_cols];
        let slow = &slow_mat[..slow_cols];

        let trade_offset = if strategy.trade_on_next_bar {
            1usize
        } else {
            0usize
        };
        let start_t = first_valid
            .saturating_add(usize::from(slow_periods[0]).saturating_sub(1))
            .saturating_add(trade_offset);

        let composite_metrics =
            ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                &lr,
                ma_prices,
                fast,
                slow,
                &halftrend_trend,
                &macz_hist,
                &ott_line,
                start_t,
                &strategy,
                CompositeSignalMode::HardGate,
            );

        let regime_f32 = build_base_composite_hard_gate_regime_f32(
            &halftrend_trend,
            &macz_hist,
            ma_prices,
            &ott_line,
        )
        .expect("base composite hard-gate regime");
        let regime_f64: Vec<f64> = regime_f32.iter().map(|&value| value as f64).collect();
        let double_ma_metrics = crate::double_ma::eval_metrics_f64_with_exit_mode(
            &lr,
            close,
            fast,
            slow,
            Some(&regime_f64),
            start_t,
            &strategy,
            None,
            true,
        );

        assert!((composite_metrics.pnl - double_ma_metrics.pnl).abs() <= 1.0e-12);
        assert!((composite_metrics.sharpe - double_ma_metrics.sharpe).abs() <= 1.0e-12);
        assert!((composite_metrics.max_dd - double_ma_metrics.max_dd).abs() <= 1.0e-12);
        assert_eq!(composite_metrics.trades, double_ma_metrics.trades);
        assert!((composite_metrics.exposure - double_ma_metrics.exposure).abs() <= 1.0e-12);
        assert!((composite_metrics.net_exposure - double_ma_metrics.net_exposure).abs() <= 1.0e-12);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_hard_gate_gpu_kernel_smoke_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        req.fast_range = (5, 9, 1);
        req.slow_range = (12, 20, 2);
        req.heatmap_bins = Some(0);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("base composite gpu-kernel optimization run");

        assert!(res.num_combos > 0);
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "expected base composite GpuKernel path, got backend={:?}, route={:?}, fallback={:?}",
            res.backend_used,
            res.backend_route,
            res.backend_fallback_reason
        );
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(res.best_params.fast_len < res.best_params.slow_len);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_hard_gate_gpu_kernel_include_all_smoke_preserves_grid_shape() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mut cpu_req = make_base_composite_req();
        cpu_req.backend = Backend::CpuOnly;
        cpu_req.fast_range = (5, 7, 1);
        cpu_req.slow_range = (10, 14, 1);
        cpu_req.top_k = Some(128);
        cpu_req.include_all = Some(true);
        cpu_req.heatmap_bins = Some(0);

        let cpu_res = run_composite_cpu_optimization_blocking_with_candles(
            cpu_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("base composite cpu optimization run");

        let mut gpu_req = cpu_req;
        gpu_req.backend = Backend::GpuKernel { device_id: 0 };
        let gpu_res =
            run_composite_cpu_optimization_blocking_with_candles(gpu_req, &candles, &cancel, None)
                .expect("base composite gpu-kernel optimization run");

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "expected base composite GpuKernel backend, got {:?}",
            gpu_res.backend_used
        );
        assert_eq!(cpu_res.num_combos, gpu_res.num_combos);
        assert_eq!(cpu_res.best_score.is_some(), gpu_res.best_score.is_some());
        assert!(gpu_res.best_score.is_some());
        assert_eq!(
            cpu_res.top_scores.as_ref().map(|scores| scores.len()),
            gpu_res.top_scores.as_ref().map(|scores| scores.len())
        );
        assert!(gpu_res
            .top_scores
            .as_ref()
            .is_some_and(|scores| !scores.is_empty()));

        assert_include_all_scores_match_rows(&cpu_res, ObjectiveKind::Sharpe, "base cpu");
        assert_include_all_scores_match_rows(&gpu_res, ObjectiveKind::Sharpe, "base gpu");

        let cpu_all = cpu_res
            .all
            .expect("cpu include_all result should contain full combo rows");
        let gpu_all = gpu_res
            .all
            .expect("gpu include_all result should contain full combo rows");
        assert_eq!(cpu_all.len(), gpu_all.len());
        assert_eq!(gpu_all.len(), gpu_res.num_combos);
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.best_params.fast_len < gpu_res.best_params.slow_len);
    }

    #[test]
    fn standalone_unified_gpu_shortlist_extras_require_include_all() {
        let candles = make_trending_candles(2000);
        let cancel = AtomicBool::new(false);

        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        req.vemr = Some(VemrStrategyParams::default());
        req.fast_range = (5, 7, 1);
        req.slow_range = (12, 16, 1);
        req.top_k = Some(4);
        req.include_all = Some(false);
        req.heatmap_bins = Some(0);
        req.extra_shortlist_objectives = Some(vec![ObjectiveKind::Pnl]);

        let err =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect_err("standalone shortlist extras should require include_all=true");
        assert!(
            err.to_ascii_lowercase().contains("include_all=true"),
            "unexpected error: {err}"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn base_composite_gpu_kernel_request_falls_back_for_entry_overlay_mode() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        req.signal_mode = CompositeSignalMode::EntryCoreExitOverlay;
        req.heatmap_bins = Some(0);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("base composite entry-overlay fallback run");

        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(
            res.backend_route.as_deref(),
            Some("GpuKernel->CompositeCpu")
        );
        let reason = res.backend_fallback_reason.unwrap_or_default();
        assert!(
            reason.contains("signal_mode=HardGate"),
            "unexpected fallback reason: {reason}"
        );
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

    #[cfg(feature = "cuda")]
    #[test]
    fn composite_trend_gpu_request_preserves_results_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut cpu_req = make_base_composite_req();
        cpu_req.strategy.trend_quality_enable = true;

        let cpu_res = run_composite_cpu_optimization_blocking_with_candles(
            cpu_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu composite trend run");

        let mut gpu_req = cpu_req;
        gpu_req.backend = Backend::GpuKernel { device_id: 0 };
        let gpu_res =
            run_composite_cpu_optimization_blocking_with_candles(gpu_req, &candles, &cancel, None)
                .expect("gpu-routed composite trend run");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Cpu));
        assert_eq!(
            gpu_res.backend_route.as_deref(),
            Some("GpuKernel->CompositeTrendCpu")
        );
        assert_eq!(cpu_res.num_combos, gpu_res.num_combos);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn breakout_momentum_gpu_hybrid_request_preserves_results_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut cpu_req = make_base_composite_req();
        cpu_req.strategy.breakout_momentum_enable = true;

        let cpu_res = run_composite_cpu_optimization_blocking_with_candles(
            cpu_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu breakout momentum run");

        let mut hybrid_req = cpu_req;
        hybrid_req.backend = Backend::GpuHybridKernel { device_id: 0 };
        let hybrid_res = run_composite_cpu_optimization_blocking_with_candles(
            hybrid_req, &candles, &cancel, None,
        )
        .expect("hybrid breakout momentum run");

        assert!(matches!(hybrid_res.backend_used, BackendUsed::Cpu));
        assert_eq!(
            hybrid_res.backend_route.as_deref(),
            Some("GpuHybridKernel->BreakoutMomentumCpu")
        );
        assert_eq!(cpu_res.num_combos, hybrid_res.num_combos);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn regime_hybrid_gpu_hybrid_request_preserves_results_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut cpu_req = make_base_composite_req();
        cpu_req.strategy.trend_quality_enable = true;
        cpu_req.strategy.regime_hybrid_enable = true;

        let cpu_res = run_composite_cpu_optimization_blocking_with_candles(
            cpu_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu regime hybrid run");

        let mut hybrid_req = cpu_req;
        hybrid_req.backend = Backend::GpuHybridKernel { device_id: 0 };
        let hybrid_res = run_composite_cpu_optimization_blocking_with_candles(
            hybrid_req, &candles, &cancel, None,
        )
        .expect("hybrid regime hybrid run");

        assert!(matches!(hybrid_res.backend_used, BackendUsed::Cpu));
        assert_eq!(
            hybrid_res.backend_route.as_deref(),
            Some("GpuHybridKernel->RegimeHybridCpu")
        );
        assert_eq!(cpu_res.num_combos, hybrid_res.num_combos);
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
        assert!(res.best_params.fast_len < res.best_params.slow_len);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_optimization_gpu_kernel_falls_back_to_cpu_with_stop_loss_overlay() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        req.vemr = Some(VemrStrategyParams::default());
        req.fast_range = (5, 9, 1);
        req.slow_range = (20, 30, 2);
        req.strategy = StrategyConfig {
            stop_loss_pct: Some(0.02),
            ..StrategyConfig::default()
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("vemr gpu fallback optimization");
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
    fn rocb_gpu_kernel_request_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        req.rocb = Some(RocbStrategyParams::default());
        req.fast_range = (5, 9, 1);
        req.slow_range = (20, 30, 2);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("rocb gpu-kernel optimization run");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
        if !matches!(res.backend_used, BackendUsed::Gpu) {
            panic!(
                "backend_used={:?} route={:?} fallback={:?}",
                res.backend_used, res.backend_route, res.backend_fallback_reason
            );
        }
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
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
        assert_eq!(cpu_res.best_score.is_some(), gpu_res.best_score.is_some());
        assert!(gpu_res.best_score.is_some());
        assert_eq!(
            cpu_res.top_scores.as_ref().map(|scores| scores.len()),
            gpu_res.top_scores.as_ref().map(|scores| scores.len())
        );
        assert!(gpu_res
            .top_scores
            .as_ref()
            .is_some_and(|scores| !scores.is_empty()));

        assert_include_all_scores_match_rows(&cpu_res, ObjectiveKind::Sharpe, "vcc cpu");
        assert_include_all_scores_match_rows(&gpu_res, ObjectiveKind::Sharpe, "vcc gpu");

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
            assert!(
                (i64::from(gpu_metrics.trades) - i64::from(cpu_metrics.trades)).abs() <= 1,
                "trades mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.trades,
                cpu_metrics.trades
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
            assert!(
                (i64::from(gpu_metrics.trades) - i64::from(cpu_metrics.trades)).abs() <= 1,
                "trades mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.trades,
                cpu_metrics.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_gpu_kernel_supports_standalone_shortlist_extras() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        req.vemr = Some(VemrStrategyParams::default());
        req.fast_range = (5, 7, 1);
        req.slow_range = (12, 16, 1);
        req.top_k = Some(4);
        req.include_all = Some(true);
        req.heatmap_bins = Some(0);
        req.extra_shortlist_objectives = Some(vec![ObjectiveKind::Pnl, ObjectiveKind::Omega]);
        req.shortlist_union_mode = Some(DoubleMaShortlistUnionMode::WeightedRank);
        req.shortlist_confidence_mode = Some(DoubleMaShortlistConfidenceMode::Dsr);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("vemr gpu-kernel shortlist extras run");

        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert!(res.best_score.is_some());
        assert_eq!(
            res.top_scores.as_ref().map(|scores| scores.len()),
            Some(res.top.len())
        );
        assert_include_all_scores_match_rows(&res, ObjectiveKind::Sharpe, "vemr shortlist extras");
        assert!(res.all.as_ref().is_some_and(|rows| !rows.is_empty()));
        assert!(res.shortlist_slice_diagnostics.is_none());
        assert!(res.score_observation_context.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vemr_gpu_kernel_supports_standalone_confidence_rerank_without_include_all() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        req.vemr = Some(VemrStrategyParams::default());
        req.fast_range = (5, 7, 1);
        req.slow_range = (12, 16, 1);
        req.top_k = Some(4);
        req.include_all = Some(false);
        req.heatmap_bins = Some(0);
        req.shortlist_confidence_mode = Some(DoubleMaShortlistConfidenceMode::Dsr);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("vemr gpu-kernel confidence rerank run");

        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert!(res.all.is_none());
        assert!(res.best_score.is_some());
        assert_eq!(res.top.len(), 4);
        assert_eq!(
            res.top_scores.as_ref().map(|scores| scores.len()),
            Some(res.top.len())
        );
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
            assert!(
                (i64::from(gpu_metrics.trades) - i64::from(cpu_metrics.trades)).abs() <= 1,
                "trades mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.trades,
                cpu_metrics.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic"]
    fn vcc_optimization_gpu_kernel_diagnostic_small_grid_maps() {
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

        let cpu_all = cpu_res.all.expect("cpu all");
        let gpu_all = gpu_res.all.expect("gpu all");
        let mut cpu_map: std::collections::HashMap<(u16, u16), Metrics> =
            std::collections::HashMap::with_capacity(cpu_all.len());
        for (params, metrics) in cpu_all {
            cpu_map.insert((params.fast_len, params.slow_len), metrics);
        }
        for (params, gpu_metrics) in gpu_all {
            let key = (params.fast_len, params.slow_len);
            let cpu_metrics = cpu_map.get(&key).unwrap();
            eprintln!(
                "vcc opt diag {:?}: cpu pnl={} sharpe={} trades={} | gpu pnl={} sharpe={} trades={}",
                key,
                cpu_metrics.pnl,
                cpu_metrics.sharpe,
                cpu_metrics.trades,
                gpu_metrics.pnl,
                gpu_metrics.sharpe,
                gpu_metrics.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_optimization_gpu_kernel_smoke_runs_when_cuda_available() {
        let _guard = lock_env();
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

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn asr_optimization_gpu_kernel_matches_cpu_small_grid_with_ema_zscore() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let mut cpu_req = make_base_composite_req();
        cpu_req.backend = Backend::CpuOnly;
        let mut asr = AsrStrategyParams::default();
        asr.z_ma_type = "ema".to_string();
        cpu_req.asr = Some(asr);
        cpu_req.fast_range = (5, 7, 1);
        cpu_req.slow_range = (20, 24, 2);
        cpu_req.top_k = Some(64);
        cpu_req.include_all = Some(true);

        let cpu_res = run_composite_cpu_optimization_blocking_with_candles(
            cpu_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("asr cpu ema optimization run");

        let mut gpu_req = cpu_req;
        gpu_req.backend = Backend::GpuKernel { device_id: 0 };
        let gpu_res =
            run_composite_cpu_optimization_blocking_with_candles(gpu_req, &candles, &cancel, None)
                .expect("asr gpu ema optimization run");

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "expected ASR GPU backend, got backend={:?}, route={:?}, fallback={:?}",
            gpu_res.backend_used,
            gpu_res.backend_route,
            gpu_res.backend_fallback_reason
        );
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());
        assert_eq!(cpu_res.num_combos, gpu_res.num_combos);
        assert_eq!(
            cpu_res.top_scores.as_ref().map(|scores| scores.len()),
            gpu_res.top_scores.as_ref().map(|scores| scores.len())
        );
        assert_include_all_scores_match_rows(&cpu_res, ObjectiveKind::Sharpe, "asr ema cpu");
        assert_include_all_scores_match_rows(&gpu_res, ObjectiveKind::Sharpe, "asr ema gpu");

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
                (gpu_metrics.sharpe - cpu_metrics.sharpe).abs() <= tol,
                "sharpe mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.sharpe,
                cpu_metrics.sharpe
            );
            assert!(
                (gpu_metrics.max_dd - cpu_metrics.max_dd).abs() <= tol,
                "max_dd mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.max_dd,
                cpu_metrics.max_dd
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
    fn asr_optimization_gpu_kernel_falls_back_to_cpu_for_nondefault_zscore_devtype() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        let mut asr = AsrStrategyParams::default();
        asr.z_devtype = 1;
        req.asr = Some(asr);
        req.fast_range = (5, 9, 1);
        req.slow_range = (20, 30, 2);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("asr gpu fallback optimization run");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or_default()
            .contains("unsupported devtype 1"));
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
    fn vcc_local_stability_smoke_runs() {
        let candles = make_trending_candles(5000);
        let req = CompositeCpuLocalStabilityRequest {
            backend: Backend::CpuOnly,
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
            .expect("vcc local stability run");
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
    fn rocb_local_stability_smoke_runs() {
        let candles = make_trending_candles(5000);
        let req = CompositeCpuLocalStabilityRequest {
            backend: Backend::CpuOnly,
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
            .expect("rocb local stability run");
        assert!(!res.points.is_empty());
        assert!(res.summary.base.is_finite());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn rocb_optimization_gpu_kernel_smoke_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut req = make_base_composite_req();
        req.backend = Backend::GpuKernel { device_id: 0 };
        req.rocb = Some(RocbStrategyParams::default());
        req.fast_range = (5, 9, 1);
        req.slow_range = (20, 30, 2);

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("rocb gpu-kernel optimization run");
        assert!(res.num_combos > 0);
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "expected ROCB GPU path, got backend={:?}, route={:?}, fallback={:?}",
            res.backend_used,
            res.backend_route,
            res.backend_fallback_reason
        );
        assert!(res.backend_fallback_reason.is_none());
        assert!(res.best_params.fast_len < res.best_params.slow_len);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn rocb_optimization_gpu_kernel_sanity_matches_cpu_small_grid() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let mut cpu_req = make_base_composite_req();
        cpu_req.backend = Backend::CpuOnly;
        cpu_req.rocb = Some(RocbStrategyParams::default());
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
        .expect("rocb cpu optimization run");

        let mut gpu_req = cpu_req;
        gpu_req.backend = Backend::GpuKernel { device_id: 0 };
        let gpu_res =
            run_composite_cpu_optimization_blocking_with_candles(gpu_req, &candles, &cancel, None)
                .expect("rocb gpu-kernel optimization run");

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "expected ROCB GPU backend, got {:?}",
            gpu_res.backend_used
        );
        assert!(gpu_res.backend_fallback_reason.is_none());
        assert_eq!(cpu_res.num_combos, gpu_res.num_combos);
        assert!(gpu_res.best_score.is_some());
        assert_eq!(
            cpu_res.top_scores.as_ref().map(|scores| scores.len()),
            gpu_res.top_scores.as_ref().map(|scores| scores.len())
        );
        assert_include_all_scores_match_rows(&cpu_res, ObjectiveKind::Sharpe, "rocb cpu");
        assert_include_all_scores_match_rows(&gpu_res, ObjectiveKind::Sharpe, "rocb gpu");

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
            assert!(
                (i64::from(gpu_metrics.trades) - i64::from(cpu_metrics.trades)).abs() <= 1,
                "trades mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.trades,
                cpu_metrics.trades
            );
        }
    }

    #[test]
    fn mkcs_optimization_smoke_runs() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "test".to_string(),
            fast_range: (5, 15, 5),
            slow_range: (20, 40, 10),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("mkcs optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn kqsr_optimization_smoke_runs() {
        let candles = make_kqsr_oscillating_candles(600);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (12, 20, 4),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("kqsr optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    #[test]
    fn kqsr_gpu_request_falls_back_to_cpu_without_cuda_kernel_support() {
        let candles = make_kqsr_oscillating_candles(600);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (12, 20, 4),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("kqsr fallback optimization");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        #[cfg(feature = "cuda")]
        {
            assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
            assert!(res
                .backend_fallback_reason
                .as_deref()
                .unwrap_or("")
                .contains("disabled"));
        }
        #[cfg(not(feature = "cuda"))]
        {
            assert!(res.backend_route.is_none());
            assert!(res.backend_fallback_reason.is_none());
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kqsr_gpu_kernel_request_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_kqsr_oscillating_candles(600);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (12, 20, 4),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("kqsr gpu optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
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
    fn kqsr_gpu_kernel_major_parity_vs_cpu() {
        let candles = make_kqsr_oscillating_candles(600);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "kqsr-parity".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (12, 20, 4),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        assert_composite_gpu_kernel_major_parity(req, &candles, &cancel, "kqsr");
    }

    #[test]
    fn carm_optimization_smoke_runs() {
        let candles = make_carm_momentum_candles(640);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("carm optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    #[test]
    fn carm_gpu_request_falls_back_to_cpu_without_cuda_kernel_support() {
        let candles = make_carm_momentum_candles(640);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("carm fallback optimization");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        #[cfg(feature = "cuda")]
        {
            assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
            assert!(res
                .backend_fallback_reason
                .as_deref()
                .unwrap_or("")
                .contains("disabled"));
        }
        #[cfg(not(feature = "cuda"))]
        {
            assert!(res.backend_route.is_none());
            assert!(res.backend_fallback_reason.is_none());
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn carm_gpu_kernel_request_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_carm_momentum_candles(640);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("carm gpu optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
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
    fn carm_gpu_kernel_major_parity_vs_cpu() {
        let candles = make_carm_momentum_candles(640);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "carm-parity".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        assert_composite_gpu_kernel_major_parity(req, &candles, &cancel, "carm");
    }

    #[test]
    fn kvdf_optimization_smoke_runs() {
        let candles = make_kvdf_flow_candles(720);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("kvdf optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    #[test]
    fn kvdf_gpu_request_falls_back_to_cpu_without_cuda_kernel_support() {
        let candles = make_kvdf_flow_candles(720);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("kvdf fallback optimization");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        #[cfg(feature = "cuda")]
        {
            assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
            assert!(res
                .backend_fallback_reason
                .as_deref()
                .unwrap_or("")
                .contains("disabled"));
        }
        #[cfg(not(feature = "cuda"))]
        {
            assert!(res.backend_route.is_none());
            assert!(res.backend_fallback_reason.is_none());
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kvdf_gpu_kernel_request_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_kvdf_flow_candles(720);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("kvdf gpu optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
        if !matches!(res.backend_used, BackendUsed::Gpu) {
            panic!(
                "backend_used={:?} route={:?} fallback={:?}",
                res.backend_used, res.backend_route, res.backend_fallback_reason
            );
        }
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kvdf_gpu_kernel_major_parity_vs_cpu() {
        let candles = make_kvdf_flow_candles(720);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "kvdf-parity".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        assert_composite_gpu_kernel_major_parity(req, &candles, &cancel, "kvdf");
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kvdf_gpu_kernel_major_parity_covers_strategy_flag_variants() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_kvdf_flow_candles(720);
        let cancel = AtomicBool::new(false);
        let base_req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "kvdf-flags".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };
        let cases = vec![
            (
                "kvdf/long_only",
                StrategyConfig {
                    long_only: true,
                    ..StrategyConfig::default()
                },
            ),
            (
                "kvdf/no_flip",
                StrategyConfig {
                    allow_flip: false,
                    ..StrategyConfig::default()
                },
            ),
            (
                "kvdf/next_bar",
                StrategyConfig {
                    trade_on_next_bar: true,
                    ..StrategyConfig::default()
                },
            ),
            (
                "kvdf/long_only_no_flip_next_bar",
                StrategyConfig {
                    long_only: true,
                    allow_flip: false,
                    trade_on_next_bar: true,
                    ..StrategyConfig::default()
                },
            ),
        ];

        for (label, strategy) in cases {
            let mut req = base_req.clone();
            req.data_id = label.replace('/', "-");
            req.strategy = strategy;
            assert_composite_gpu_kernel_major_parity(req, &candles, &cancel, label);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn kvdf_gpu_kernel_request_falls_back_to_cpu_with_stop_loss_overlay() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_kvdf_flow_candles(720);
        let cancel = AtomicBool::new(false);
        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "kvdf-gpu-fallback".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig {
                stop_loss_pct: Some(0.02),
                ..StrategyConfig::default()
            },
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("kvdf gpu fallback optimization");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or_default()
            .contains("without overlays/stop-loss/take-profit"));
    }

    #[test]
    fn cdmc_optimization_smoke_runs() {
        let candles = make_cdmc_cycle_candles(640);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("cdmc optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    #[test]
    fn cdmc_gpu_request_falls_back_to_cpu_without_cuda_kernel_support() {
        let candles = make_cdmc_cycle_candles(640);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("cdmc fallback optimization");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        #[cfg(feature = "cuda")]
        {
            assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
            assert!(res
                .backend_fallback_reason
                .as_deref()
                .unwrap_or("")
                .contains("disabled"));
        }
        #[cfg(not(feature = "cuda"))]
        {
            assert!(res.backend_route.is_none());
            assert!(res.backend_fallback_reason.is_none());
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_gpu_kernel_request_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_cdmc_cycle_candles(640);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("cdmc gpu optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cdmc_gpu_kernel_major_parity_vs_cpu() {
        let candles = make_cdmc_cycle_candles(640);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "cdmc-parity".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        assert_composite_gpu_kernel_major_parity(req, &candles, &cancel, "cdmc");
    }

    #[test]
    fn vcet_optimization_smoke_runs() {
        let candles = make_trending_candles(640);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "vcet-test".to_string(),
            fast_range: (5, 9, 2),
            slow_range: (14, 24, 5),
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(8),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("vcet optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcet_gpu_kernel_request_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "vcet-gpu".to_string(),
            fast_range: (5, 11, 2),
            slow_range: (20, 36, 4),
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(8),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("vcet gpu optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
        if !matches!(res.backend_used, BackendUsed::Gpu) {
            panic!(
                "backend_used={:?} route={:?} fallback={:?}",
                res.backend_used, res.backend_route, res.backend_fallback_reason
            );
        }
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcet_gpu_kernel_request_falls_back_to_cpu_with_stop_loss_overlay() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(3000);
        let cancel = AtomicBool::new(false);
        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "vcet-gpu-fallback".to_string(),
            fast_range: (5, 11, 2),
            slow_range: (20, 36, 4),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig {
                stop_loss_pct: Some(0.02),
                ..StrategyConfig::default()
            },
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(8),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("vcet gpu fallback optimization");
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
    fn vcet_gpu_kernel_major_parity_vs_cpu() {
        let candles = make_trending_candles(640);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "vcet-parity".to_string(),
            fast_range: (5, 11, 2),
            slow_range: (20, 36, 4),
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(8),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        assert_composite_gpu_kernel_major_parity(req, &candles, &cancel, "vcet");
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcet_gpu_kernel_major_parity_covers_strategy_flag_variants() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(640);
        let cancel = AtomicBool::new(false);
        let base_req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
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
            data_id: "vcet-flags".to_string(),
            fast_range: (5, 11, 2),
            slow_range: (20, 36, 4),
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(8),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };
        let cases = vec![
            (
                "vcet/long_only",
                StrategyConfig {
                    long_only: true,
                    ..StrategyConfig::default()
                },
            ),
            (
                "vcet/no_flip",
                StrategyConfig {
                    allow_flip: false,
                    ..StrategyConfig::default()
                },
            ),
            (
                "vcet/next_bar",
                StrategyConfig {
                    trade_on_next_bar: true,
                    ..StrategyConfig::default()
                },
            ),
            (
                "vcet/long_only_no_flip_next_bar",
                StrategyConfig {
                    long_only: true,
                    allow_flip: false,
                    trade_on_next_bar: true,
                    ..StrategyConfig::default()
                },
            ),
        ];

        for (label, strategy) in cases {
            let mut req = base_req.clone();
            req.data_id = label.replace('/', "-");
            req.strategy = strategy;
            assert_composite_gpu_kernel_major_parity(req, &candles, &cancel, label);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_gpu_kernel_request_runs_when_cuda_available() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "test".to_string(),
            fast_range: (5, 15, 5),
            slow_range: (20, 40, 10),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("mkcs gpu optimization");
        assert!(res.num_combos > 0);
        assert!(!res.top.is_empty());
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
    fn mkcs_gpu_kernel_request_falls_back_to_cpu_with_stop_loss_overlay() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            backend: Backend::GpuKernel { device_id: 0 },
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
            data_id: "mkcs-gpu-fallback".to_string(),
            fast_range: (5, 15, 5),
            slow_range: (20, 40, 10),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig {
                stop_loss_pct: Some(0.02),
                ..StrategyConfig::default()
            },
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("mkcs gpu fallback optimization");
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
    fn mkcs_optimization_gpu_kernel_sanity_matches_cpu_small_grid() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_mkcs_breakout_candles(220);
        let cancel = AtomicBool::new(false);

        let mut cpu_req = make_mkcs_req(Backend::CpuOnly);
        cpu_req.fast_range = (5, 7, 1);
        cpu_req.slow_range = (10, 14, 2);
        cpu_req.top_k = Some(64);
        cpu_req.include_all = Some(true);

        let cpu_res = run_composite_cpu_optimization_blocking_with_candles(
            cpu_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("mkcs cpu optimization run");

        let mut gpu_req = cpu_req;
        gpu_req.backend = Backend::GpuKernel { device_id: 0 };
        let gpu_res =
            run_composite_cpu_optimization_blocking_with_candles(gpu_req, &candles, &cancel, None)
                .expect("mkcs gpu-kernel optimization run");

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "expected MKCS GPU backend, got {:?}",
            gpu_res.backend_used
        );
        assert_eq!(cpu_res.num_combos, gpu_res.num_combos);
        assert_eq!(cpu_res.best_score.is_some(), gpu_res.best_score.is_some());
        assert!(gpu_res.best_score.is_some());
        assert_eq!(
            cpu_res.top_scores.as_ref().map(|scores| scores.len()),
            gpu_res.top_scores.as_ref().map(|scores| scores.len())
        );
        assert!(gpu_res
            .top_scores
            .as_ref()
            .is_some_and(|scores| !scores.is_empty()));

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
            assert!(
                (i64::from(gpu_metrics.trades) - i64::from(cpu_metrics.trades)).abs() <= 1,
                "trades mismatch for {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.trades,
                cpu_metrics.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_optimization_gpu_kernel_topk_matches_cpu_small_grid() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_mkcs_breakout_candles(220);
        let cancel = AtomicBool::new(false);

        let mut cpu_req = make_mkcs_req(Backend::CpuOnly);
        cpu_req.fast_range = (5, 8, 1);
        cpu_req.slow_range = (10, 16, 2);
        cpu_req.objective = ObjectiveKind::Sharpe;
        cpu_req.top_k = Some(4);
        cpu_req.include_all = Some(false);
        cpu_req.heatmap_bins = Some(0);

        let cpu_res = run_composite_cpu_optimization_blocking_with_candles(
            cpu_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("mkcs cpu optimization run");

        let mut gpu_req = cpu_req;
        gpu_req.backend = Backend::GpuKernel { device_id: 0 };
        let gpu_res =
            run_composite_cpu_optimization_blocking_with_candles(gpu_req, &candles, &cancel, None)
                .expect("mkcs gpu-kernel optimization run");

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "expected MKCS GPU backend, got {:?}",
            gpu_res.backend_used
        );
        assert_eq!(cpu_res.num_combos, gpu_res.num_combos);
        assert_eq!(cpu_res.top.len(), gpu_res.top.len());

        let cpu_best_score = objective_score(&cpu_res.best_metrics, ObjectiveKind::Sharpe);
        let gpu_best_score = objective_score(&gpu_res.best_metrics, ObjectiveKind::Sharpe);
        assert!(
            (cpu_best_score - gpu_best_score).abs() <= 5.0e-2,
            "best score mismatch: cpu={} gpu={}",
            cpu_best_score,
            gpu_best_score
        );
        assert_eq!(
            (cpu_res.best_params.fast_len, cpu_res.best_params.slow_len),
            (gpu_res.best_params.fast_len, gpu_res.best_params.slow_len),
            "best params mismatch"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mkcs_optimization_gpu_kernel_matches_cpu_small_grid() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_mkcs_breakout_candles(220);
        let cancel = AtomicBool::new(false);

        let mut cpu_req = make_mkcs_req(Backend::CpuOnly);
        cpu_req.fast_range = (5, 7, 1);
        cpu_req.slow_range = (12, 16, 1);
        cpu_req.top_k = Some(128);
        cpu_req.include_all = Some(true);
        cpu_req.heatmap_bins = Some(0);

        let cpu_res = run_composite_cpu_optimization_blocking_with_candles(
            cpu_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("mkcs cpu optimization run");

        let mut gpu_req = cpu_req;
        gpu_req.backend = Backend::GpuKernel { device_id: 0 };
        let gpu_res =
            run_composite_cpu_optimization_blocking_with_candles(gpu_req, &candles, &cancel, None)
                .expect("mkcs gpu-kernel optimization run");

        assert!(
            matches!(gpu_res.backend_used, BackendUsed::Gpu),
            "expected MKCS GPU backend, got {:?}",
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

        let tol = 2.5e-1;
        for (params, gpu_metrics) in gpu_all {
            let key = (params.fast_len, params.slow_len);
            let cpu_metrics = cpu_map
                .get(&key)
                .unwrap_or_else(|| panic!("cpu metrics missing for combo {:?}", key));
            assert!(
                (gpu_metrics.pnl - cpu_metrics.pnl).abs() <= tol,
                "MKCS pnl mismatch for combo {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.pnl,
                cpu_metrics.pnl
            );
            assert!(
                (gpu_metrics.sharpe - cpu_metrics.sharpe).abs() <= tol,
                "MKCS sharpe mismatch for combo {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.sharpe,
                cpu_metrics.sharpe
            );
            assert!(
                (gpu_metrics.max_dd - cpu_metrics.max_dd).abs() <= tol,
                "MKCS max_dd mismatch for combo {:?}: gpu={} cpu={}",
                key,
                gpu_metrics.max_dd,
                cpu_metrics.max_dd
            );
        }
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
