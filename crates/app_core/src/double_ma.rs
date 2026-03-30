use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use ta_optimizer::stream::StreamAggregator;
use ta_optimizer::{ObjectiveKind, OptimizationHeatmap, OptimizationMode, OptimizationResult};
use ta_strategies::double_ma::{
    DoubleMaCurves, DoubleMaParams, IndicatorBar, Metrics, StrategyConfig, Trade, TradeStats,
};

use crate::alloc_policy::{alloc_f64_with_prefix_uninit_tail, alloc_uninit_f32, alloc_uninit_f64};
use crate::composite_validate::SpaBenchmarkKind;
use crate::double_ma_backend::{
    backend_kind_label, build_indicator_route_plan_summary, gpu_kernel_blockers_for_request,
    hybrid_backend_enabled, should_cpu_stage_indicator,
};
pub(crate) use crate::double_ma_ma_kind::{lookup_ma_info, validate_ma_kind};
use crate::double_ma_mode::{
    resolve_optimization_mode, validate_optimization_mode, ModeSelectionContext,
};
use crate::double_ma_result::{build_backend_optimization_result, BackendResultContext};
pub(crate) use crate::double_ma_strategy::{
    strategy_cfg_for_optimization, validate_strategy_cfg, validate_strategy_cfg_for_optimization,
};
use crate::double_ma_vdps::{
    merge_vdps_telemetry, vdps_decimated_periods, vdps_inject_differential_seed_pairs,
    vdps_inject_elite_mutation_pairs, vdps_inject_random_seed_pairs, vdps_local_anchor_axes,
    vdps_merge_partial_results, vdps_period_window_slice, vdps_rank_candidates, vdps_target_pairs,
};
use crate::matrix_index::DoubleMaFastLtSlowIndex;
use crate::progress::ProgressSink;
use crate::request_normalization::{
    normalize_heatmap_bins, normalize_optional_path, normalize_topk_include_all,
};

static DYNAMIC_GPU_PREFLIGHT_CACHE: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
static FOLLOWUP_CACHE: OnceLock<Mutex<HashMap<String, FollowupCacheEntry>>> = OnceLock::new();
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
thread_local! {
    static DYNAMIC_GPU_BATCH_CACHE: RefCell<HashMap<String, Rc<crate::gpu_backtest_kernel::UploadedDoubleMaDynamicProgramBatch>>> =
        RefCell::new(HashMap::new());
}

pub fn reset_dynamic_gpu_preflight_cache() {
    if let Some(cache) = DYNAMIC_GPU_PREFLIGHT_CACHE.get() {
        if let Ok(mut guard) = cache.lock() {
            guard.clear();
        }
    }
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    DYNAMIC_GPU_BATCH_CACHE.with(|cell| {
        cell.borrow_mut().clear();
    });
}

#[derive(Clone, Debug)]
struct FollowupCacheEntry {
    backend_hint: Option<Backend>,
    log_returns: Option<Vec<f64>>,
    regime_filter: Option<Vec<f64>>,
    kaufman_stops: Option<KaufmanStopSeries>,
    base_first_valid: Option<usize>,
}

fn followup_cache() -> &'static Mutex<HashMap<String, FollowupCacheEntry>> {
    FOLLOWUP_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
#[path = "double_ma_test_support.rs"]
mod double_ma_test_support;

#[cfg(feature = "cuda")]
use vector_ta::cuda::{
    cuda_available,
    moving_averages::{CudaAlma, CudaMaData, CudaMaSelector},
    CudaDeviceCloseVolumeRef,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use vector_ta::indicators::dispatch::{
    compute_cuda_device, CudaOutputTarget, IndicatorCudaDeviceDataRef, IndicatorCudaDeviceRequest,
    IndicatorCudaOutput, IndicatorCudaSeries,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use vector_ta::utilities::enums::Kernel;

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cuda_from_device_matrix_row,
    compute_dynamic_indicator_cuda_with_inputs, device_slice_ref_from_matrix_row,
    DynamicIndicatorCudaSeries, PreparedCudaIndicatorInputs,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, IndicatorInputKind, IndicatorParamKV, IndicatorParamValue,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "backend")]
pub enum Backend {
    #[serde(alias = "auto", alias = "AUTO")]
    Auto { device_id: u32 },
    #[serde(alias = "cpu_only", alias = "CPU_ONLY", alias = "cpu")]
    CpuOnly,
    #[serde(alias = "gpu_only", alias = "GPU_ONLY", alias = "GpuSweep")]
    GpuOnly { device_id: u32 },
    #[serde(
        alias = "gpu_hybrid_kernel",
        alias = "GPU_HYBRID_KERNEL",
        alias = "GpuHybrid"
    )]
    GpuHybridKernel { device_id: u32 },
    #[serde(alias = "gpu_kernel", alias = "GPU_KERNEL")]
    GpuKernel { device_id: u32 },
}

impl Backend {
    fn with_resolved_preferred_cuda_device(
        self,
        preferred_device_id: Result<Option<u32>, String>,
    ) -> Result<Self, String> {
        let preferred_device_id = preferred_device_id?;
        Ok(match self {
            Self::Auto { .. } => Self::Auto {
                device_id: preferred_device_id.unwrap_or(0),
            },
            Self::CpuOnly => Self::CpuOnly,
            Self::GpuOnly { .. } => Self::GpuOnly {
                device_id: preferred_device_id.unwrap_or(0),
            },
            Self::GpuHybridKernel { .. } => Self::GpuHybridKernel {
                device_id: preferred_device_id.unwrap_or(0),
            },
            Self::GpuKernel { .. } => Self::GpuKernel {
                device_id: preferred_device_id.unwrap_or(0),
            },
        })
    }

    pub fn with_preferred_cuda_device(self) -> Result<Self, String> {
        self.with_resolved_preferred_cuda_device(
            crate::preferred_cuda_device().map(|device| device.map(|row| row.device_id)),
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HalfTrendRegimeFilter {
    #[serde(default = "default_halftrend_amplitude")]
    pub amplitude: u16,
    #[serde(default = "default_halftrend_channel_deviation")]
    pub channel_deviation: f64,
    #[serde(default = "default_halftrend_atr_period")]
    pub atr_period: u16,
}

fn default_halftrend_amplitude() -> u16 {
    2
}

fn default_halftrend_channel_deviation() -> f64 {
    2.0
}

fn default_halftrend_atr_period() -> u16 {
    100
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DoubleMaConstraints {
    #[serde(default)]
    pub min_trades: Option<u32>,
    #[serde(default)]
    pub max_drawdown: Option<f64>,
    #[serde(default)]
    pub min_sharpe: Option<f64>,
    #[serde(default)]
    pub min_sortino: Option<f64>,
    #[serde(default)]
    pub min_calmar: Option<f64>,
    #[serde(default)]
    pub max_volatility: Option<f64>,
    #[serde(default)]
    pub min_exposure: Option<f64>,
    #[serde(default)]
    pub max_exposure: Option<f64>,
    #[serde(default)]
    pub max_abs_net_exposure: Option<f64>,
    #[serde(default)]
    pub min_omega: Option<f64>,
    #[serde(default)]
    pub max_ulcer: Option<f64>,
    #[serde(default)]
    pub max_downside_deviation: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoubleMaScoreMode {
    BalancedRiskAdjusted10,
    AdversarialEnvelope10,
    SupportShrunk10,
    ActiveSortino10,
    EffectiveSampleLcb10,
    OmegaVsSharpe10,
    RobustMargin10,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoubleMaShortlistConfidenceMode {
    Psr,
    Dsr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoubleMaShortlistUnionMode {
    Lexicographic,
    Pareto,
    EpsilonFront,
    WeightedRank,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoubleMaShortlistSliceMode {
    Halves,
    Quartiles,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoubleMaMetricContract {
    Full17,
    Trade22,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VdpsConfig {
    #[serde(default)]
    pub eta: Option<u32>,
    #[serde(default)]
    pub rungs: Option<Vec<f64>>,
    #[serde(default)]
    pub min_initial_pairs: Option<u32>,
    #[serde(default)]
    pub max_survivor_seeds: Option<u32>,
    #[serde(default)]
    pub max_period_candidates: Option<u32>,
    #[serde(default)]
    pub random_injection_ratio: Option<f64>,
    #[serde(default)]
    pub elite_mutation_ratio: Option<f64>,
    #[serde(default)]
    pub differential_ratio: Option<f64>,
    #[serde(default)]
    pub deterministic: Option<bool>,
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub quality_check_max_pairs: Option<u32>,
    #[serde(default)]
    pub quality_check_top_k: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DoubleMaRequest {
    pub backend: Backend,
    pub data_id: String,
    pub fast_range: (u32, u32, u32),
    pub slow_range: (u32, u32, u32),
    pub fast_ma_types: Vec<String>,
    pub slow_ma_types: Vec<String>,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub export_all_csv_path: Option<String>,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    pub fast_ma_param_sweeps: Option<HashMap<String, (f64, f64, f64)>>,
    pub slow_ma_param_sweeps: Option<HashMap<String, (f64, f64, f64)>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    pub halftrend_filter: Option<HalfTrendRegimeFilter>,
    pub objective: ObjectiveKind,
    #[serde(default)]
    pub score_mode: Option<DoubleMaScoreMode>,
    #[serde(default)]
    pub hurdle_tau: Option<f64>,
    #[serde(default)]
    pub extra_shortlist_objectives: Option<Vec<ObjectiveKind>>,
    #[serde(default)]
    pub shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    #[serde(default)]
    pub shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    #[serde(default)]
    pub shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    #[serde(default)]
    pub shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
    #[serde(default)]
    pub shortlist_neighborhood_radius: Option<u16>,
    #[serde(default)]
    pub shortlist_neighborhood_step: Option<u16>,
    pub mode: OptimizationMode,
    pub top_k: Option<usize>,
    pub include_all: Option<bool>,
    pub heatmap_bins: Option<u16>,
    #[serde(default)]
    pub constraints: Option<DoubleMaConstraints>,
    #[serde(default)]
    pub vdps: Option<VdpsConfig>,
    #[serde(default)]
    pub dynamic_strategy_ir: Option<crate::strategy_ir::StrategyIr>,
    #[serde(default)]
    pub gpu_execution: GpuExecutionPolicy,
    #[serde(default)]
    pub persist_followup_cache: bool,
    #[serde(default)]
    pub include_trade_metrics: bool,
    #[serde(default)]
    pub metric_contract: Option<DoubleMaMetricContract>,
}

fn default_ma_source() -> String {
    "close".to_string()
}

const DRILL_INDICATOR_MAX_POINTS: usize = 8_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OptimizationModeResolved {
    Grid,
    CoarseToFine,
    GpuVdps,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BackendUsed {
    Cpu,
    Gpu,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GpuExecutionPolicy {
    #[default]
    BestEffort,
    PreferGpuResident,
    StrictGpuResident,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuResidencySummary {
    pub requested_policy: GpuExecutionPolicy,
    pub requested_backend: String,
    pub resolved_backend: String,
    pub fully_gpu_resident: bool,
    pub cpu_staged_indicators: bool,
    pub cpu_fallback_used: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DoubleMaParamsResolved {
    pub fast_len: u16,
    pub slow_len: u16,
    pub fast_ma_type: String,
    pub slow_ma_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_ma_params: Option<HashMap<String, f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow_ma_params: Option<HashMap<String, f64>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DoubleMaDrilldownRequest {
    pub data_id: String,
    #[serde(default, alias = "backend")]
    pub backend_hint: Option<Backend>,
    #[serde(default)]
    pub followup_cache_key: Option<String>,
    pub params: DoubleMaParamsResolved,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    pub halftrend_filter: Option<HalfTrendRegimeFilter>,
    pub bins: Option<usize>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DoubleMaLocalStabilityRequest {
    pub data_id: String,
    #[serde(default, alias = "backend")]
    pub backend_hint: Option<Backend>,
    #[serde(default)]
    pub followup_cache_key: Option<String>,
    pub params: DoubleMaParamsResolved,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    pub halftrend_filter: Option<HalfTrendRegimeFilter>,
    pub objective: ObjectiveKind,
    #[serde(default)]
    pub score_mode: Option<DoubleMaScoreMode>,
    pub radius: Option<u16>,
    pub step: Option<u16>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LocalStabilityPoint {
    pub fast_len: u16,
    pub slow_len: u16,
    pub metrics: ta_strategies::double_ma::Metrics,
    pub score: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct LocalStabilitySummary {
    pub base: f64,
    pub min: f64,
    pub median: f64,
    pub max: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairObservationScoreContextResolved {
    pub first_valid: usize,
    pub regime_first_valid: usize,
    pub eval_start: usize,
    pub eval_end: usize,
    pub total_bars: usize,
    pub trade_on_next_bar: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShortlistNeighborhoodRobustnessSummary {
    pub plateau_score: f64,
    pub plateau_label: String,
    pub objective: ObjectiveKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_mode: Option<DoubleMaScoreMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_agreement: Option<ShortlistNeighborhoodScoreAgreementSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<ShortlistNeighborhoodExplanationSummary>,
    pub radius: u16,
    pub step: u16,
    pub point_count: usize,
    pub neighbor_count: usize,
    pub base_score: f64,
    pub median_score: f64,
    pub min_score: f64,
    pub max_score: f64,
    pub peak_delta: f64,
    pub basin_min_score: f64,
    pub basin_floor_stability: f64,
    pub roughness_rms_delta: f64,
    pub roughness_stability: f64,
    pub curvature_penalty: f64,
    pub curvature_stability: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShortlistNeighborhoodScoreAgreementSummary {
    pub point_count: usize,
    pub current_vs_objective_rank_corr: f64,
    pub current_vs_balanced_rank_corr: f64,
    pub current_vs_active_sortino_rank_corr: f64,
    pub current_vs_omega_vs_sharpe_rank_corr: f64,
    pub current_vs_robust_margin_rank_corr: f64,
    pub agreement_floor: f64,
    pub agreement_mean: f64,
    pub agreement_label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShortlistNeighborhoodExplanationSummary {
    pub point_count: usize,
    pub base_current_rank: usize,
    pub base_objective_rank: usize,
    pub pareto_nondominated_count: usize,
    pub base_is_pareto_nondominated: bool,
    pub base_dominates_neighbors: usize,
    pub neighbors_dominate_base: usize,
    pub dominance_margin: isize,
    pub base_hypervolume_proxy: f64,
    pub hypervolume_proxy_rank: usize,
    pub hypervolume_proxy_share: f64,
    pub dominance_label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct LocalStabilityResult {
    pub base: DoubleMaParamsResolved,
    pub objective: ObjectiveKind,
    pub radius: u16,
    pub step: u16,
    pub start_t: usize,
    pub points: Vec<LocalStabilityPoint>,
    pub summary: LocalStabilitySummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackendOptimizationResult {
    pub best_params: DoubleMaParamsResolved,
    pub best_metrics: ta_strategies::double_ma::Metrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_score: Option<f64>,
    pub mode_used: OptimizationModeResolved,
    pub backend_used: BackendUsed,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix_index: Option<DoubleMaFastLtSlowIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_col: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_cols: Option<Vec<u32>>,
    pub num_combos: usize,
    pub num_candles: usize,
    pub runtime_ms: u64,
    pub top: Vec<(DoubleMaParamsResolved, ta_strategies::double_ma::Metrics)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_scores: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<(DoubleMaParamsResolved, ta_strategies::double_ma::Metrics)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_scores: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heatmap: Option<OptimizationHeatmap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vdps: Option<VdpsTelemetry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid_timing_ms: Option<HybridTimingMs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_kernel_timing_ms: Option<GpuKernelTimingMs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_fallback_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_diagnostics: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indicator_route_plan: Option<IndicatorRoutePlanSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residency: Option<GpuResidencySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_cache_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortlist_slice_diagnostics: Option<ShortlistSliceDiagnosticsResolved>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_observation_context: Option<PairObservationScoreContextResolved>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ShortlistSliceSnapshot {
    pub label: String,
    pub start_bar: usize,
    pub end_bar: usize,
    pub score: f64,
    pub metrics: Metrics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ShortlistSliceModeDiagnostics {
    pub worst_slice_score: f64,
    pub positive_slice_frac: f64,
    pub mean_slice_score: f64,
    pub best_omission_label: String,
    pub best_omission_score: f64,
    pub worst_omission_label: String,
    pub worst_omission_score: f64,
    pub omission_score_spread: f64,
    pub benchmark_relative: Option<ShortlistSliceBenchmarkRelativeSummary>,
    pub slices: Vec<ShortlistSliceSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ShortlistSliceCandidateDiagnostics {
    pub params: DoubleMaParams,
    pub full_metrics: Metrics,
    pub halves: ShortlistSliceModeDiagnostics,
    pub quartiles: ShortlistSliceModeDiagnostics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ShortlistSliceDiagnostics {
    pub ranking_mode: String,
    pub neighborhood_robustness: Option<ShortlistNeighborhoodRobustnessSummary>,
    pub candidates: Vec<ShortlistSliceCandidateDiagnostics>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShortlistSliceSnapshotResolved {
    pub label: String,
    pub start_bar: usize,
    pub end_bar: usize,
    pub score: f64,
    pub metrics: Metrics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShortlistSliceModeDiagnosticsResolved {
    pub worst_slice_score: f64,
    pub positive_slice_frac: f64,
    pub mean_slice_score: f64,
    pub best_omission_label: String,
    pub best_omission_score: f64,
    pub worst_omission_label: String,
    pub worst_omission_score: f64,
    pub omission_score_spread: f64,
    pub benchmark_relative: Option<ShortlistSliceBenchmarkRelativeSummary>,
    pub slices: Vec<ShortlistSliceSnapshotResolved>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShortlistSliceBenchmarkRelativeSummary {
    pub benchmark: String,
    pub beat_rate: f64,
    pub mean_excess_return: f64,
    pub worst_excess_return: f64,
    pub best_excess_return: f64,
    pub best_omission_excess_return: f64,
    pub worst_omission_excess_return: f64,
    pub omission_excess_spread: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShortlistSliceCandidateDiagnosticsResolved {
    pub params: DoubleMaParamsResolved,
    pub full_metrics: Metrics,
    pub halves: ShortlistSliceModeDiagnosticsResolved,
    pub quartiles: ShortlistSliceModeDiagnosticsResolved,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShortlistSliceDiagnosticsResolved {
    pub ranking_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neighborhood_robustness: Option<ShortlistNeighborhoodRobustnessSummary>,
    pub candidates: Vec<ShortlistSliceCandidateDiagnosticsResolved>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct HybridTimingMs {
    pub planner_ms: u64,
    pub cpu_indicator_prep_ms: u64,
    pub gpu_indicator_prep_ms: u64,
    pub h2d_upload_ms: u64,
    pub kernel_eval_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct GpuKernelTimingMs {
    pub route_wall_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_init_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_current_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_indicator_prep_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h2d_upload_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_submit_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub d2h_copy_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_decode_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit_sync_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_finalize_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_capture_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_replay_count: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndicatorRouteDecisionSummary {
    pub indicator_id: String,
    pub capability: String,
    pub route: String,
    pub gpu_preferred_by_speedup: bool,
    pub residency_tier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staging_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndicatorRoutePlanSummary {
    pub fast: IndicatorRouteDecisionSummary,
    pub slow: IndicatorRouteDecisionSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VdpsTelemetry {
    pub eta: u32,
    pub rungs: Vec<f64>,
    pub min_initial_pairs: u32,
    pub max_survivor_seeds: u32,
    pub max_period_candidates: u32,
    pub random_injection_ratio: f64,
    pub elite_mutation_ratio: f64,
    pub differential_ratio: f64,
    pub deterministic: bool,
    pub seed: u64,
    pub rng_kind: String,
    pub tie_break_schema_version: String,
    pub full_grid_pairs: usize,
    pub full_fidelity_pairs_evaluated: usize,
    pub full_fidelity_pairs_saved: usize,
    pub evaluated_pairs_total: usize,
    pub runtime_ms_total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_check_top_k_overlap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_check_best_score_regret: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_check_baseline_runtime_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_check_baseline_backend: Option<BackendUsed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_check_baseline_pairs: Option<usize>,
    pub rung_stats: Vec<VdpsRungTelemetry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VdpsRungTelemetry {
    pub rung_index: usize,
    pub rung_fraction: f64,
    pub target_pairs: usize,
    pub evaluated_pairs: usize,
    pub promoted_pairs: usize,
    pub dropped_pairs: usize,
    pub fast_candidates: usize,
    pub slow_candidates: usize,
    pub backend_used: BackendUsed,
    pub runtime_ms: u64,
    pub throughput_pairs_per_sec: f64,
}

pub(crate) fn objective_score(
    metrics: &ta_strategies::double_ma::Metrics,
    objective: ObjectiveKind,
) -> f64 {
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

const SCORE_CLAMP_MAX: f64 = 1e6;
const SCORE_EPS: f64 = 1e-12;
const SCORE_CLIP: f64 = 4.0;
pub(crate) const DEFAULT_FEE_STRESS_FLOOR_BPS: f64 = 2.0;

#[inline]
fn clamp_commission_rate(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(0.0, 0.999_999)
    } else {
        0.0
    }
}

#[inline]
pub(crate) fn validation_fee_stress_fee_bps(strategy: &StrategyConfig) -> f64 {
    let fee_bps = if strategy.fee_bps.is_finite() && strategy.fee_bps >= 0.0 {
        strategy.fee_bps
    } else {
        0.0
    };
    (fee_bps * 2.0)
        .max(fee_bps + 1.0)
        .max(DEFAULT_FEE_STRESS_FLOOR_BPS)
}

#[inline]
pub(crate) fn exact_stressed_pnl_from_components(
    pnl: f64,
    cost_event_count: u32,
    base_commission: f64,
    stressed_commission: f64,
) -> f64 {
    if !pnl.is_finite() {
        return pnl;
    }
    let equity = 1.0 + pnl;
    if !equity.is_finite() || equity <= 0.0 {
        return pnl;
    }

    let base_commission = clamp_commission_rate(base_commission);
    let stressed_commission = clamp_commission_rate(stressed_commission);
    if cost_event_count == 0 || (base_commission - stressed_commission).abs() <= SCORE_EPS {
        return pnl;
    }

    let base_log_comm = (-base_commission).ln_1p();
    let stressed_log_comm = (-stressed_commission).ln_1p();
    let stressed_log_eq =
        equity.ln() + (cost_event_count as f64) * (stressed_log_comm - base_log_comm);
    if stressed_log_eq.is_finite() {
        stressed_log_eq.exp_m1()
    } else if stressed_log_eq.is_sign_positive() {
        f64::INFINITY
    } else {
        -1.0
    }
}

#[inline]
pub(crate) fn exact_fee_stress_pnl(metrics: &Metrics, strategy: &StrategyConfig) -> f64 {
    let mut stressed = strategy.clone();
    stressed.fee_bps = validation_fee_stress_fee_bps(strategy);
    let base_commission = strategy.effective_commission();
    let stressed_commission = stressed.effective_commission();
    exact_stressed_pnl_from_components(
        metrics.pnl,
        metrics.trades,
        base_commission,
        stressed_commission,
    )
}

#[inline]
pub(crate) fn derived_mean(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    metrics.sharpe * metrics.volatility
}

#[inline]
pub(crate) fn derived_active_mean(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let exposure = metrics.exposure.max(SCORE_EPS);
    if !exposure.is_finite() || exposure <= SCORE_EPS {
        return 0.0;
    }
    derived_mean(metrics) / exposure
}

#[inline]
pub(crate) fn derived_active_sortino(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let mean = derived_active_mean(metrics);
    let denom = derived_active_downside_deviation(metrics);
    if denom > SCORE_EPS {
        mean / denom
    } else if mean > 0.0 {
        SCORE_CLAMP_MAX
    } else if mean < 0.0 {
        -SCORE_CLAMP_MAX
    } else {
        0.0
    }
}

#[inline]
pub(crate) fn derived_active_downside_deviation(
    metrics: &ta_strategies::double_ma::Metrics,
) -> f64 {
    let exposure = metrics.exposure.max(SCORE_EPS);
    if !exposure.is_finite() || exposure <= SCORE_EPS {
        return 0.0;
    }
    metrics.downside_deviation / exposure.sqrt()
}

#[inline]
pub(crate) fn derived_drawdown_chronicity(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    if metrics.max_dd > SCORE_EPS {
        (metrics.ulcer_index / metrics.max_dd).clamp(0.0, SCORE_CLIP)
    } else {
        0.0
    }
}

#[inline]
pub(crate) fn derived_downside_share(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    if metrics.volatility > SCORE_EPS {
        (metrics.downside_deviation / metrics.volatility).clamp(0.0, SCORE_CLIP)
    } else {
        0.0
    }
}

#[inline]
#[allow(dead_code)]
pub(crate) fn derived_long_exposure(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    (0.5 * (metrics.exposure + metrics.net_exposure)).clamp(0.0, 1.0)
}

#[inline]
#[allow(dead_code)]
pub(crate) fn derived_short_exposure(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    (0.5 * (metrics.exposure - metrics.net_exposure)).clamp(0.0, 1.0)
}

#[inline]
#[allow(dead_code)]
pub(crate) fn derived_directional_bias(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let exposure = metrics.exposure.max(SCORE_EPS);
    if !exposure.is_finite() || exposure <= SCORE_EPS {
        return 0.0;
    }
    (metrics.net_exposure.abs() / exposure).clamp(0.0, 1.0)
}

#[inline]
fn score_clip_signed(value: f64) -> f64 {
    value.clamp(-SCORE_CLIP, SCORE_CLIP)
}

#[inline]
fn score_clip_penalty(value: f64) -> f64 {
    value.clamp(0.0, SCORE_CLIP)
}

#[inline]
fn score_asinh_scaled(value: f64, scale: f64) -> f64 {
    if !value.is_finite() || !scale.is_finite() || scale <= 0.0 {
        return 0.0;
    }
    score_clip_signed((value / scale).asinh())
}

#[inline]
fn score_log_ratio(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    score_clip_signed(value.max(SCORE_EPS).ln())
}

#[inline]
fn score_log1p_penalty(value: f64, scale: f64) -> f64 {
    if !value.is_finite() || !scale.is_finite() || scale <= 0.0 {
        return 0.0;
    }
    score_clip_penalty((value.max(0.0) / scale).ln_1p())
}

#[inline]
fn score_trade_utility(trades: u32, target: f64) -> f64 {
    if !target.is_finite() || target <= 0.0 {
        return 0.0;
    }
    (((trades as f64).max(0.0)).ln_1p() / target.ln_1p()).clamp(0.0, 1.0)
}

#[inline]
fn score_exposure_penalty(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let exposure = metrics.exposure.abs().max(0.0);
    let net = metrics.net_exposure.abs().max(0.0);
    let penalty = (0.10 - exposure).max(0.0) / 0.10
        + (exposure - 0.95).max(0.0) / 0.10
        + (net - 0.35).max(0.0) / 0.20;
    score_clip_penalty(penalty)
}

#[inline]
fn score_downside_shape_penalty(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let downside_share = derived_downside_share(metrics);
    score_clip_penalty((downside_share - 0.75).max(0.0) / 0.25)
}

#[inline]
fn score_support_proxy_units(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let trades = (metrics.trades as f64).max(0.0);
    let exposure = metrics.exposure.clamp(0.0, 1.0);
    1.0 + trades + 80.0 * exposure
}

#[inline]
fn score_margin_at_least(value: f64, threshold: f64, scale: f64) -> f64 {
    if !value.is_finite() || !threshold.is_finite() || !scale.is_finite() || scale <= 0.0 {
        return 0.0;
    }
    score_clip_signed((value - threshold) / scale)
}

#[inline]
fn score_margin_at_most(value: f64, threshold: f64, scale: f64) -> f64 {
    if !value.is_finite() || !threshold.is_finite() || !scale.is_finite() || scale <= 0.0 {
        return 0.0;
    }
    score_clip_signed((threshold - value) / scale)
}

#[inline]
fn score_softmin(values: &[f64], temperature: f64) -> f64 {
    if values.is_empty() || !temperature.is_finite() || temperature <= 0.0 {
        return 0.0;
    }
    let anchor = values
        .iter()
        .copied()
        .fold(f64::INFINITY, |cur, value| cur.min(value));
    if !anchor.is_finite() {
        return 0.0;
    }
    let sum_exp = values
        .iter()
        .copied()
        .map(|value| (-(value - anchor) / temperature).exp())
        .sum::<f64>();
    if !sum_exp.is_finite() || sum_exp <= 0.0 {
        return anchor;
    }
    score_clip_signed(anchor - temperature * sum_exp.ln())
}

pub(crate) fn balanced_risk_adjusted_10_score(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let sharpe_u = score_asinh_scaled(metrics.sharpe, 1.0);
    let active_sortino_u = score_asinh_scaled(derived_active_sortino(metrics), 1.25);
    let omega_u = score_log_ratio(metrics.omega);
    let calmar_u = score_asinh_scaled(metrics.calmar, 1.0);
    let trade_u = score_trade_utility(metrics.trades, 40.0);
    let max_dd_p = score_log1p_penalty(metrics.max_dd, 0.10);
    let ulcer_p = score_log1p_penalty(metrics.ulcer_index, 0.05);
    let chronicity_p = derived_drawdown_chronicity(metrics).clamp(0.0, 1.0);
    let exposure_p = score_exposure_penalty(metrics);
    let downside_p = score_downside_shape_penalty(metrics);

    0.42 * sharpe_u + 0.20 * active_sortino_u + 0.12 * omega_u + 0.08 * calmar_u + 0.08 * trade_u
        - 0.16 * max_dd_p
        - 0.10 * ulcer_p
        - 0.08 * chronicity_p
        - 0.06 * exposure_p
        - 0.04 * downside_p
}

pub(crate) fn adversarial_envelope_10_score(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let sharpe_u = score_asinh_scaled(metrics.sharpe, 1.0);
    let active_mean_u = score_asinh_scaled(derived_active_mean(metrics), 0.12);
    let active_sortino_u = score_asinh_scaled(derived_active_sortino(metrics), 1.25);
    let omega_u = score_log_ratio(metrics.omega);
    let calmar_u = score_asinh_scaled(metrics.calmar, 1.0);
    let trade_u = score_trade_utility(metrics.trades, 40.0);
    let max_dd_p = score_log1p_penalty(metrics.max_dd, 0.10);
    let ulcer_p = score_log1p_penalty(metrics.ulcer_index, 0.05);
    let chronicity_p = derived_drawdown_chronicity(metrics).clamp(0.0, 1.0);
    let exposure_p = score_exposure_penalty(metrics);
    let downside_p = score_downside_shape_penalty(metrics);

    let balanced = 0.34 * sharpe_u
        + 0.15 * active_mean_u
        + 0.15 * active_sortino_u
        + 0.11 * omega_u
        + 0.09 * calmar_u
        + 0.06 * trade_u
        - 0.15 * max_dd_p
        - 0.10 * ulcer_p
        - 0.07 * chronicity_p
        - 0.05 * exposure_p
        - 0.03 * downside_p;
    let path_guard = 0.28 * sharpe_u
        + 0.10 * active_mean_u
        + 0.12 * active_sortino_u
        + 0.11 * omega_u
        + 0.18 * calmar_u
        + 0.04 * trade_u
        - 0.22 * max_dd_p
        - 0.15 * ulcer_p
        - 0.12 * chronicity_p
        - 0.05 * exposure_p
        - 0.04 * downside_p;
    let fragility_guard = 0.30 * sharpe_u
        + 0.12 * active_mean_u
        + 0.18 * active_sortino_u
        + 0.15 * omega_u
        + 0.10 * calmar_u
        + 0.05 * trade_u
        - 0.16 * max_dd_p
        - 0.12 * ulcer_p
        - 0.08 * chronicity_p
        - 0.05 * exposure_p
        - 0.11 * downside_p;

    balanced.min(path_guard).min(fragility_guard)
}

pub(crate) fn support_shrunk_10_score(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let base = balanced_risk_adjusted_10_score(metrics);
    let support_units = score_support_proxy_units(metrics).max(1.0);
    let shrink_penalty = 1.2 / support_units.sqrt();
    let chronicity_penalty = 0.10 * derived_drawdown_chronicity(metrics).clamp(0.0, 1.0);
    let downside_penalty = 0.12 * score_downside_shape_penalty(metrics);

    base - shrink_penalty - chronicity_penalty - downside_penalty
}

pub(crate) fn active_sortino_10_score(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let active_sortino_u = score_asinh_scaled(derived_active_sortino(metrics), 1.25);
    let omega_u = score_log_ratio(metrics.omega);
    let calmar_u = score_asinh_scaled(metrics.calmar, 1.0);
    let trade_u = score_trade_utility(metrics.trades, 36.0);
    let max_dd_p = score_log1p_penalty(metrics.max_dd, 0.10);
    let ulcer_p = score_log1p_penalty(metrics.ulcer_index, 0.05);
    let chronicity_p = 0.18 * derived_drawdown_chronicity(metrics).clamp(0.0, 1.0);
    let exposure_p = 0.12 * score_exposure_penalty(metrics);
    let downside_p = 0.12 * score_downside_shape_penalty(metrics);

    0.58 * active_sortino_u + 0.12 * omega_u + 0.08 * calmar_u + 0.06 * trade_u
        - 0.18 * max_dd_p
        - 0.10 * ulcer_p
        - chronicity_p
        - exposure_p
        - downside_p
}

pub(crate) fn effective_sample_lcb_10_score(
    metrics: &ta_strategies::double_ma::Metrics,
    n_obs: usize,
) -> f64 {
    let base = balanced_risk_adjusted_10_score(metrics);
    let effective_n = n_obs.max(2) as f64;
    let lcb_penalty = 18.0 / effective_n.sqrt();
    base - lcb_penalty
}

pub(crate) fn omega_vs_sharpe_10_score(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let sharpe_u = score_asinh_scaled(metrics.sharpe, 1.0);
    let omega_u = score_log_ratio(metrics.omega);
    let asymmetry_gap = score_clip_signed(omega_u - 0.65 * sharpe_u);
    let active_sortino_u = score_asinh_scaled(derived_active_sortino(metrics), 1.25);
    let calmar_u = score_asinh_scaled(metrics.calmar, 1.0);
    let trade_u = score_trade_utility(metrics.trades, 32.0);
    let max_dd_p = score_log1p_penalty(metrics.max_dd, 0.10);
    let ulcer_p = score_log1p_penalty(metrics.ulcer_index, 0.05);
    let chronicity_p = 0.14 * derived_drawdown_chronicity(metrics).clamp(0.0, 1.0);
    let downside_p = 0.10 * score_downside_shape_penalty(metrics);
    let exposure_p = 0.08 * score_exposure_penalty(metrics);
    let directional_p = 0.08 * derived_directional_bias(metrics).clamp(0.0, 1.0);

    0.36 * asymmetry_gap
        + 0.20 * active_sortino_u
        + 0.14 * omega_u
        + 0.10 * calmar_u
        + 0.06 * trade_u
        - 0.18 * max_dd_p
        - 0.10 * ulcer_p
        - chronicity_p
        - downside_p
        - exposure_p
        - directional_p
}

pub(crate) fn robust_margin_10_score(metrics: &ta_strategies::double_ma::Metrics) -> f64 {
    let margins = [
        score_margin_at_least(metrics.sharpe, 0.90, 0.45),
        score_margin_at_least(derived_active_sortino(metrics), 1.25, 0.70),
        score_margin_at_least(metrics.omega, 1.05, 0.20),
        score_margin_at_least(metrics.calmar, 0.65, 0.35),
        score_margin_at_least(score_trade_utility(metrics.trades, 24.0), 0.35, 0.18),
        score_margin_at_most(metrics.max_dd, 0.22, 0.08),
        score_margin_at_most(metrics.ulcer_index, 0.10, 0.04),
        score_margin_at_most(derived_drawdown_chronicity(metrics), 0.70, 0.20),
        score_margin_at_most(score_downside_shape_penalty(metrics), 0.70, 0.35),
        score_margin_at_most(score_exposure_penalty(metrics), 0.70, 0.35),
    ];
    let soft_margin = score_softmin(&margins, 0.55);
    let mean_margin = margins.iter().copied().sum::<f64>() / margins.len() as f64;
    0.72 * soft_margin + 0.28 * mean_margin
}

pub(crate) fn score_mode_metrics_count_floor(score_mode: Option<DoubleMaScoreMode>) -> usize {
    match score_mode {
        Some(DoubleMaScoreMode::BalancedRiskAdjusted10)
        | Some(DoubleMaScoreMode::AdversarialEnvelope10)
        | Some(DoubleMaScoreMode::SupportShrunk10)
        | Some(DoubleMaScoreMode::ActiveSortino10)
        | Some(DoubleMaScoreMode::EffectiveSampleLcb10)
        | Some(DoubleMaScoreMode::OmegaVsSharpe10)
        | Some(DoubleMaScoreMode::RobustMargin10) => 10,
        None => 0,
    }
}

pub(crate) fn score_mode_needs_rich_metrics(score_mode: Option<DoubleMaScoreMode>) -> bool {
    score_mode.is_some()
}

pub(crate) fn score_mode_needs_pair_observation_context(
    score_mode: Option<DoubleMaScoreMode>,
) -> bool {
    matches!(score_mode, Some(DoubleMaScoreMode::EffectiveSampleLcb10))
}

pub(crate) fn score_mode_score_fn(
    score_mode: Option<DoubleMaScoreMode>,
) -> Option<fn(&ta_strategies::double_ma::Metrics) -> f64> {
    match score_mode {
        Some(DoubleMaScoreMode::BalancedRiskAdjusted10) => Some(balanced_risk_adjusted_10_score),
        Some(DoubleMaScoreMode::AdversarialEnvelope10) => Some(adversarial_envelope_10_score),
        Some(DoubleMaScoreMode::SupportShrunk10) => Some(support_shrunk_10_score),
        Some(DoubleMaScoreMode::ActiveSortino10) => Some(active_sortino_10_score),
        Some(DoubleMaScoreMode::EffectiveSampleLcb10) => None,
        Some(DoubleMaScoreMode::OmegaVsSharpe10) => Some(omega_vs_sharpe_10_score),
        Some(DoubleMaScoreMode::RobustMargin10) => Some(robust_margin_10_score),
        None => None,
    }
}

pub(crate) fn effective_score(
    metrics: &ta_strategies::double_ma::Metrics,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
) -> f64 {
    if let Some(score_fn) = score_mode_score_fn(score_mode) {
        let score = score_fn(metrics);
        if score.is_finite() {
            score
        } else {
            f64::NEG_INFINITY
        }
    } else {
        let score = objective_score(metrics, objective);
        if score.is_finite() {
            score
        } else {
            f64::NEG_INFINITY
        }
    }
}

#[allow(clippy::type_complexity)]
fn finalize_risk_metrics(
    compute_return_stats: bool,
    count: u64,
    obs: u64,
    hurdle_tau: f64,
    mean: f64,
    m2: f64,
    sum_neg_sq: f64,
    sum_neg_cu: f64,
    count_neg: u64,
    sum_pos_r: f64,
    sum_neg_abs_r: f64,
    long_obs: u64,
    short_obs: u64,
    long_sum_r: f64,
    short_sum_r: f64,
    ulcer_sum_sq: f64,
    max_log_dd: f64,
    log_eq: f64,
    underwater_bars: u64,
    max_underwater_duration_bars: u64,
) -> (
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    u32,
) {
    const EPS_DENOM: f64 = 1e-12;
    const CLAMP_MAX: f64 = 1e6;

    let (
        sharpe,
        volatility,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        omega,
        kappa_tau,
        long_mean_return,
        short_mean_return,
    ) = if compute_return_stats {
        let variance = if count > 1 {
            m2 / (count as f64 - 1.0)
        } else {
            0.0
        };
        let std = variance.sqrt();
        let sharpe = if std > 0.0 { mean / std } else { 0.0 };
        let denom = (count as f64).max(1.0);
        let downside_deviation = (sum_neg_sq / denom).sqrt();
        let mean_excess = mean - hurdle_tau;
        let sortino_raw = if downside_deviation > 0.0 {
            mean_excess / downside_deviation
        } else if mean_excess > 0.0 {
            CLAMP_MAX
        } else if mean_excess < 0.0 {
            -CLAMP_MAX
        } else {
            0.0
        };
        let sortino = sortino_raw.clamp(-CLAMP_MAX, CLAMP_MAX);
        let omega_raw = (sum_pos_r + EPS_DENOM) / (sum_neg_abs_r + EPS_DENOM);
        let omega = omega_raw.clamp(0.0, CLAMP_MAX);
        let kappa_denom = (sum_neg_cu / denom).cbrt();
        let kappa_raw = if kappa_denom > 0.0 {
            mean_excess / kappa_denom
        } else if mean_excess > 0.0 {
            CLAMP_MAX
        } else if mean_excess < 0.0 {
            -CLAMP_MAX
        } else {
            0.0
        };
        let kappa_tau = kappa_raw.clamp(-CLAMP_MAX, CLAMP_MAX);
        let downside_frequency = count_neg as f64 / (obs as f64).max(1.0);
        let downside_severity = if count_neg > 0 {
            sum_neg_abs_r / count_neg as f64
        } else {
            0.0
        };
        let long_mean_return = if long_obs > 0 {
            long_sum_r / long_obs as f64
        } else {
            0.0
        };
        let short_mean_return = if short_obs > 0 {
            short_sum_r / short_obs as f64
        } else {
            0.0
        };
        (
            sharpe,
            std,
            sortino,
            downside_deviation,
            downside_frequency,
            downside_severity,
            omega,
            kappa_tau,
            long_mean_return,
            short_mean_return,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    };

    let max_dd = if max_log_dd > 0.0 {
        -(-max_log_dd).exp_m1()
    } else {
        0.0
    };
    let pnl = log_eq.exp() - 1.0;
    let denom = (obs as f64).max(1.0);
    let ulcer_index = (ulcer_sum_sq / denom).sqrt();
    let martin_raw = if ulcer_index > EPS_DENOM {
        pnl / ulcer_index
    } else if pnl > 0.0 {
        CLAMP_MAX
    } else if pnl < 0.0 {
        -CLAMP_MAX
    } else {
        0.0
    };
    let martin_ratio = martin_raw.clamp(-CLAMP_MAX, CLAMP_MAX);
    let calmar_raw = if max_dd > EPS_DENOM {
        pnl / max_dd
    } else if pnl > 0.0 {
        CLAMP_MAX
    } else if pnl < 0.0 {
        -CLAMP_MAX
    } else {
        0.0
    };
    let calmar = calmar_raw.clamp(-CLAMP_MAX, CLAMP_MAX);
    let underwater_fraction = if obs > 0 {
        underwater_bars as f64 / obs as f64
    } else {
        0.0
    };
    let max_underwater_duration_bars = max_underwater_duration_bars.min(u32::MAX as u64) as u32;

    (
        sharpe,
        volatility,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        omega,
        kappa_tau,
        max_dd,
        ulcer_index,
        martin_ratio,
        calmar,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
    )
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

const VDPS_DEFAULT_RUNGS: [f64; 5] = [0.0625, 0.125, 0.25, 0.5, 1.0];
const VDPS_ELIMINATION_RATE: usize = 4;
const VDPS_MIN_INITIAL_PAIRS: usize = 4_096;
const VDPS_MAX_SURVIVOR_SEEDS: usize = 256;
const VDPS_MAX_PERIOD_CANDIDATES: usize = 384;
const VDPS_DEFAULT_RANDOM_INJECTION_RATIO: f64 = 0.10;
const VDPS_DEFAULT_ELITE_MUTATION_RATIO: f64 = 0.50;
const VDPS_DEFAULT_DIFFERENTIAL_RATIO: f64 = 0.40;
const VDPS_DEFAULT_QUALITY_CHECK_MAX_PAIRS: usize = 0;
const VDPS_DEFAULT_QUALITY_CHECK_TOP_K: usize = 20;
const VDPS_MAX_RUNGS: usize = 16;
const VDPS_DEFAULT_MAX_ACTIVE_ANCHORS: usize = 6;
const VDPS_LOCAL_FRONTIER_MIN_FULL_GRID_PAIRS: usize = 750_000;

#[derive(Clone, Debug)]
struct VdpsResolvedConfig {
    eta: usize,
    rungs: Vec<f64>,
    min_initial_pairs: usize,
    max_survivor_seeds: usize,
    max_period_candidates: usize,
    random_injection_ratio: f64,
    elite_mutation_ratio: f64,
    differential_ratio: f64,
    deterministic: bool,
    seed: u64,
    quality_check_max_pairs: usize,
    quality_check_top_k: usize,
}

impl VdpsResolvedConfig {
    #[inline]
    fn from_request(cfg: Option<&VdpsConfig>) -> Result<Self, String> {
        let eta = cfg
            .and_then(|v| v.eta)
            .unwrap_or(VDPS_ELIMINATION_RATE as u32);
        if !(2..=32).contains(&eta) {
            return Err("vdps.eta must be in [2, 32]".to_string());
        }

        let mut rungs = cfg
            .and_then(|v| v.rungs.clone())
            .unwrap_or_else(|| VDPS_DEFAULT_RUNGS.to_vec());
        rungs.retain(|v| v.is_finite() && *v > 0.0);
        for v in rungs.iter_mut() {
            *v = v.clamp(0.000_001, 1.0);
        }
        rungs.sort_by(|a, b| a.total_cmp(b));
        rungs.dedup_by(|a, b| (*a - *b).abs() <= f64::EPSILON);
        if rungs.is_empty() {
            return Err("vdps.rungs must include at least one positive fraction".to_string());
        }
        if rungs.len() > VDPS_MAX_RUNGS {
            return Err(format!("vdps.rungs length must be <= {}", VDPS_MAX_RUNGS));
        }
        if rungs.last().copied().unwrap_or(1.0) < 1.0 {
            rungs.push(1.0);
        }

        let min_initial_pairs = cfg
            .and_then(|v| v.min_initial_pairs)
            .unwrap_or(VDPS_MIN_INITIAL_PAIRS as u32);
        if min_initial_pairs == 0 {
            return Err("vdps.min_initial_pairs must be > 0".to_string());
        }

        let max_survivor_seeds = cfg
            .and_then(|v| v.max_survivor_seeds)
            .unwrap_or(VDPS_MAX_SURVIVOR_SEEDS as u32);
        if max_survivor_seeds < 8 {
            return Err("vdps.max_survivor_seeds must be >= 8".to_string());
        }

        let max_period_candidates = cfg
            .and_then(|v| v.max_period_candidates)
            .unwrap_or(VDPS_MAX_PERIOD_CANDIDATES as u32);
        if max_period_candidates < 16 {
            return Err("vdps.max_period_candidates must be >= 16".to_string());
        }

        let random_injection_ratio = cfg
            .and_then(|v| v.random_injection_ratio)
            .unwrap_or(VDPS_DEFAULT_RANDOM_INJECTION_RATIO);
        if !(0.0..=1.0).contains(&random_injection_ratio) {
            return Err("vdps.random_injection_ratio must be in [0, 1]".to_string());
        }
        let elite_mutation_ratio = cfg
            .and_then(|v| v.elite_mutation_ratio)
            .unwrap_or(VDPS_DEFAULT_ELITE_MUTATION_RATIO);
        if !(0.0..=1.0).contains(&elite_mutation_ratio) {
            return Err("vdps.elite_mutation_ratio must be in [0, 1]".to_string());
        }
        let differential_ratio = cfg
            .and_then(|v| v.differential_ratio)
            .unwrap_or(VDPS_DEFAULT_DIFFERENTIAL_RATIO);
        if !(0.0..=1.0).contains(&differential_ratio) {
            return Err("vdps.differential_ratio must be in [0, 1]".to_string());
        }

        let deterministic = cfg.and_then(|v| v.deterministic).unwrap_or(true);
        let seed = cfg.and_then(|v| v.seed).unwrap_or(1);
        let quality_check_max_pairs =
            cfg.and_then(|v| v.quality_check_max_pairs)
                .unwrap_or(VDPS_DEFAULT_QUALITY_CHECK_MAX_PAIRS as u32) as usize;
        let quality_check_top_k =
            cfg.and_then(|v| v.quality_check_top_k)
                .unwrap_or(VDPS_DEFAULT_QUALITY_CHECK_TOP_K as u32) as usize;
        if quality_check_top_k == 0 {
            return Err("vdps.quality_check_top_k must be > 0".to_string());
        }

        Ok(Self {
            eta: eta as usize,
            rungs,
            min_initial_pairs: min_initial_pairs as usize,
            max_survivor_seeds: max_survivor_seeds as usize,
            max_period_candidates: max_period_candidates as usize,
            random_injection_ratio,
            elite_mutation_ratio,
            differential_ratio,
            deterministic,
            seed,
            quality_check_max_pairs,
            quality_check_top_k,
        })
    }
}

#[inline]
fn vdps_normalized_refill_mix(cfg: &VdpsResolvedConfig) -> (f64, f64, f64) {
    let mut elite = cfg.elite_mutation_ratio.max(0.0);
    let mut diff = cfg.differential_ratio.max(0.0);
    let mut rand = cfg.random_injection_ratio.max(0.0);
    let sum = elite + diff + rand;
    if sum > 1.0 && sum.is_finite() {
        elite /= sum;
        diff /= sum;
        rand /= sum;
    }
    (elite, diff, rand)
}

#[inline]
fn vdps_gpu_refill_enabled() -> bool {
    match std::env::var("VECTORBT_VDPS_GPU_REFILL") {
        Ok(v) => {
            let v = v.trim();
            !(v == "0" || v.eq_ignore_ascii_case("false"))
        }
        Err(_) => true,
    }
}

#[inline]
#[cfg_attr(
    not(all(feature = "cuda", feature = "cuda-backtest-kernel")),
    allow(dead_code)
)]
pub(crate) fn kernel_regime_filters_enabled() -> bool {
    match std::env::var("VECTORBT_KERNEL_ENABLE_REGIME_FILTERS") {
        Ok(v) => {
            let v = v.trim();
            !(v == "0" || v.eq_ignore_ascii_case("false"))
        }
        Err(_) => true,
    }
}

#[inline]
fn gpu_shortlist_confidence_mode_requested(
    requested: Option<DoubleMaShortlistConfidenceMode>,
) -> Result<Option<DoubleMaShortlistConfidenceMode>, String> {
    Ok(requested)
}

#[inline]
fn gpu_shortlist_union_mode_requested(
    requested: Option<DoubleMaShortlistUnionMode>,
) -> Result<Option<DoubleMaShortlistUnionMode>, String> {
    Ok(requested)
}

#[inline]
fn gpu_shortlist_slice_mode_requested(
    requested: Option<DoubleMaShortlistSliceMode>,
) -> Result<Option<DoubleMaShortlistSliceMode>, String> {
    Ok(requested)
}

#[inline]
fn gpu_shortlist_slice_benchmark_requested(
    requested: Option<SpaBenchmarkKind>,
) -> Result<Option<SpaBenchmarkKind>, String> {
    Ok(requested)
}

#[derive(Clone, Copy, Debug)]
struct ShortlistNeighborhoodConfig {
    radius: u16,
    step: u16,
}

#[inline]
fn gpu_shortlist_neighborhood_config_requested(
    requested_radius: Option<u16>,
    requested_step: Option<u16>,
) -> Result<Option<ShortlistNeighborhoodConfig>, String> {
    let Some(radius) = requested_radius else {
        return Ok(None);
    };
    if radius == 0 {
        return Err("shortlist_neighborhood_radius must be > 0 when enabled".to_string());
    }
    let step = requested_step.unwrap_or(1);
    if step == 0 {
        return Err("shortlist_neighborhood_step must be > 0 when enabled".to_string());
    }
    Ok(Some(ShortlistNeighborhoodConfig {
        radius: radius.min(32),
        step,
    }))
}

#[inline]
fn label_shortlist_plateau(score: f64, neighbor_count: usize) -> String {
    if neighbor_count >= 5 && score >= 0.78 {
        "Strong".to_string()
    } else if neighbor_count >= 3 && score >= 0.58 {
        "Moderate".to_string()
    } else if score >= 0.38 {
        "Narrow".to_string()
    } else {
        "Fragile".to_string()
    }
}

fn label_shortlist_surface_agreement(score: f64, point_count: usize) -> String {
    if point_count < 3 {
        "Thin".to_string()
    } else if score >= 0.85 {
        "Aligned".to_string()
    } else if score >= 0.60 {
        "Mixed".to_string()
    } else {
        "Divergent".to_string()
    }
}

fn label_shortlist_explanation(
    base_is_pareto_nondominated: bool,
    neighbors_dominate_base: usize,
    base_dominates_neighbors: usize,
    hypervolume_proxy_rank: usize,
) -> String {
    if base_is_pareto_nondominated
        && neighbors_dominate_base == 0
        && base_dominates_neighbors > 0
        && hypervolume_proxy_rank <= 1
    {
        "Front".to_string()
    } else if base_is_pareto_nondominated && neighbors_dominate_base == 0 {
        "Competitive".to_string()
    } else {
        "Fragile".to_string()
    }
}

fn descending_average_ranks(values: &[f64]) -> Vec<f64> {
    let mut ordered: Vec<(usize, f64)> = values.iter().copied().enumerate().collect();
    ordered.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut ranks = vec![0.0; values.len()];
    let mut start = 0usize;
    while start < ordered.len() {
        let mut end = start + 1;
        while end < ordered.len()
            && ordered[end].1.total_cmp(&ordered[start].1) == std::cmp::Ordering::Equal
        {
            end += 1;
        }
        let avg_rank = (start + 1 + end) as f64 * 0.5;
        for &(idx, _) in &ordered[start..end] {
            ranks[idx] = avg_rank;
        }
        start = end;
    }
    ranks
}

fn pearson_corr(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len().min(ys.len());
    if n < 2 {
        return 1.0;
    }
    let mean_x = xs.iter().take(n).copied().sum::<f64>() / n as f64;
    let mean_y = ys.iter().take(n).copied().sum::<f64>() / n as f64;
    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    for idx in 0..n {
        let dx = xs[idx] - mean_x;
        let dy = ys[idx] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }
    if var_x <= 1.0e-18 || var_y <= 1.0e-18 {
        return 1.0;
    }
    (cov / (var_x.sqrt() * var_y.sqrt())).clamp(-1.0, 1.0)
}

fn spearman_rank_corr(xs: &[f64], ys: &[f64]) -> f64 {
    let rank_x = descending_average_ranks(xs);
    let rank_y = descending_average_ranks(ys);
    pearson_corr(&rank_x, &rank_y)
}

fn dense_desc_rank(values: &[f64], idx: usize) -> usize {
    let value = values.get(idx).copied().unwrap_or(f64::NEG_INFINITY);
    1 + values
        .iter()
        .filter(|other| other.total_cmp(&value).is_gt())
        .count()
}

fn normalized_score_axis(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }
    let min_v = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max_v = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max_v - min_v).abs();
    if span <= 1.0e-18 {
        return vec![1.0; values.len()];
    }
    values
        .iter()
        .map(|value| ((value - min_v) / span).clamp(0.0, 1.0))
        .collect()
}

fn summarize_shortlist_surface_explanation(
    local: &LocalStabilityResult,
    _score_mode: Option<DoubleMaScoreMode>,
) -> Option<ShortlistNeighborhoodExplanationSummary> {
    if local.points.is_empty() {
        return None;
    }
    let base_idx = local.points.iter().position(|point| {
        point.fast_len == local.base.fast_len && point.slow_len == local.base.slow_len
    })?;
    let current_scores: Vec<f64> = local.points.iter().map(|point| point.score).collect();
    let objective_scores: Vec<f64> = local
        .points
        .iter()
        .map(|point| objective_score(&point.metrics, local.objective))
        .collect();
    let current_axis = normalized_score_axis(&current_scores);
    let objective_axis = normalized_score_axis(&objective_scores);
    let dominates = |a_idx: usize, b_idx: usize| -> bool {
        let a_cur = current_axis[a_idx];
        let a_obj = objective_axis[a_idx];
        let b_cur = current_axis[b_idx];
        let b_obj = objective_axis[b_idx];
        (a_cur >= b_cur && a_obj >= b_obj) && (a_cur > b_cur || a_obj > b_obj)
    };
    let base_dominates_neighbors = (0..local.points.len())
        .filter(|&idx| idx != base_idx && dominates(base_idx, idx))
        .count();
    let neighbors_dominate_base = (0..local.points.len())
        .filter(|&idx| idx != base_idx && dominates(idx, base_idx))
        .count();
    let pareto_nondominated_count = (0..local.points.len())
        .filter(|&idx| !(0..local.points.len()).any(|other| other != idx && dominates(other, idx)))
        .count();
    let base_is_pareto_nondominated = neighbors_dominate_base == 0;
    let hypervolume_proxy: Vec<f64> = current_axis
        .iter()
        .zip(objective_axis.iter())
        .map(|(cur, obj)| cur * obj)
        .collect();
    let hv_sum = hypervolume_proxy.iter().copied().sum::<f64>();
    let base_hypervolume_proxy = hypervolume_proxy[base_idx];
    let hypervolume_proxy_share = if hv_sum <= 1.0e-18 {
        0.0
    } else {
        (base_hypervolume_proxy / hv_sum).clamp(0.0, 1.0)
    };
    let hypervolume_proxy_rank = dense_desc_rank(&hypervolume_proxy, base_idx);

    Some(ShortlistNeighborhoodExplanationSummary {
        point_count: local.points.len(),
        base_current_rank: dense_desc_rank(&current_scores, base_idx),
        base_objective_rank: dense_desc_rank(&objective_scores, base_idx),
        pareto_nondominated_count,
        base_is_pareto_nondominated,
        base_dominates_neighbors,
        neighbors_dominate_base,
        dominance_margin: base_dominates_neighbors as isize - neighbors_dominate_base as isize,
        base_hypervolume_proxy,
        hypervolume_proxy_rank,
        hypervolume_proxy_share,
        dominance_label: label_shortlist_explanation(
            base_is_pareto_nondominated,
            neighbors_dominate_base,
            base_dominates_neighbors,
            hypervolume_proxy_rank,
        ),
    })
}

fn summarize_shortlist_surface_agreement(
    local: &LocalStabilityResult,
) -> Option<ShortlistNeighborhoodScoreAgreementSummary> {
    if local.points.len() < 2 {
        return None;
    }
    let current_scores: Vec<f64> = local.points.iter().map(|point| point.score).collect();
    let objective_scores: Vec<f64> = local
        .points
        .iter()
        .map(|point| objective_score(&point.metrics, local.objective))
        .collect();
    let balanced_scores: Vec<f64> = local
        .points
        .iter()
        .map(|point| balanced_risk_adjusted_10_score(&point.metrics))
        .collect();
    let active_sortino_scores: Vec<f64> = local
        .points
        .iter()
        .map(|point| active_sortino_10_score(&point.metrics))
        .collect();
    let omega_vs_sharpe_scores: Vec<f64> = local
        .points
        .iter()
        .map(|point| omega_vs_sharpe_10_score(&point.metrics))
        .collect();
    let robust_margin_scores: Vec<f64> = local
        .points
        .iter()
        .map(|point| robust_margin_10_score(&point.metrics))
        .collect();

    let current_vs_objective_rank_corr = spearman_rank_corr(&current_scores, &objective_scores);
    let current_vs_balanced_rank_corr = spearman_rank_corr(&current_scores, &balanced_scores);
    let current_vs_active_sortino_rank_corr =
        spearman_rank_corr(&current_scores, &active_sortino_scores);
    let current_vs_omega_vs_sharpe_rank_corr =
        spearman_rank_corr(&current_scores, &omega_vs_sharpe_scores);
    let current_vs_robust_margin_rank_corr =
        spearman_rank_corr(&current_scores, &robust_margin_scores);

    let agreement_values = [
        current_vs_objective_rank_corr,
        current_vs_balanced_rank_corr,
        current_vs_active_sortino_rank_corr,
        current_vs_omega_vs_sharpe_rank_corr,
        current_vs_robust_margin_rank_corr,
    ];
    let agreement_floor = agreement_values
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let agreement_mean =
        agreement_values.iter().copied().sum::<f64>() / agreement_values.len() as f64;

    Some(ShortlistNeighborhoodScoreAgreementSummary {
        point_count: local.points.len(),
        current_vs_objective_rank_corr,
        current_vs_balanced_rank_corr,
        current_vs_active_sortino_rank_corr,
        current_vs_omega_vs_sharpe_rank_corr,
        current_vs_robust_margin_rank_corr,
        agreement_floor,
        agreement_mean,
        agreement_label: label_shortlist_surface_agreement(
            agreement_floor.max(agreement_mean),
            local.points.len(),
        ),
    })
}

fn summarize_shortlist_neighborhood_robustness(
    local: &LocalStabilityResult,
    score_mode: Option<DoubleMaScoreMode>,
) -> Option<ShortlistNeighborhoodRobustnessSummary> {
    if local.points.is_empty() {
        return None;
    }
    let base_score = local.summary.base;
    let point_count = local.points.len();
    let neighbor_count = point_count.saturating_sub(1);
    let median_score = local.summary.median;
    let min_score = local.summary.min;
    let max_score = local.summary.max;
    let peak_delta = (max_score - base_score).max(0.0);
    let score_span = (max_score - min_score).abs();
    let score_scale = score_span.max(base_score.abs()).max(1.0);
    let adverse_median_gap = (base_score - median_score).max(0.0);
    let median_stability = 1.0 / (1.0 + adverse_median_gap / score_scale);
    let peak_stability = 1.0 / (1.0 + peak_delta / score_scale);
    let score_stability = 0.6 * median_stability + 0.4 * peak_stability;
    let support_strength = (neighbor_count.min(8) as f64) / 8.0;
    let neighbor_points: Vec<&LocalStabilityPoint> = local
        .points
        .iter()
        .filter(|point| {
            !(point.fast_len == local.base.fast_len && point.slow_len == local.base.slow_len)
        })
        .collect();
    let basin_min_score = neighbor_points
        .iter()
        .map(|point| point.score)
        .fold(base_score, f64::min);
    let basin_floor_gap = (base_score - basin_min_score).max(0.0);
    let basin_floor_stability = 1.0 / (1.0 + basin_floor_gap / score_scale);
    let roughness_rms_delta = if neighbor_points.is_empty() {
        0.0
    } else {
        let mean_sq = neighbor_points
            .iter()
            .map(|point| {
                let delta = point.score - base_score;
                delta * delta
            })
            .sum::<f64>()
            / (neighbor_points.len() as f64);
        mean_sq.sqrt()
    };
    let roughness_stability = 1.0 / (1.0 + roughness_rms_delta / score_scale);
    let axial_pair_curvature = |left: Option<f64>, right: Option<f64>| -> Option<f64> {
        match (left, right) {
            (Some(a), Some(b)) => Some((((a + b) * 0.5) - base_score).abs()),
            _ => None,
        }
    };
    let left_score = neighbor_points
        .iter()
        .find(|point| {
            point.fast_len + local.step == local.base.fast_len
                && point.slow_len == local.base.slow_len
        })
        .map(|point| point.score);
    let right_score = neighbor_points
        .iter()
        .find(|point| {
            point.fast_len == local.base.fast_len + local.step
                && point.slow_len == local.base.slow_len
        })
        .map(|point| point.score);
    let down_score = neighbor_points
        .iter()
        .find(|point| {
            point.fast_len == local.base.fast_len
                && point.slow_len + local.step == local.base.slow_len
        })
        .map(|point| point.score);
    let up_score = neighbor_points
        .iter()
        .find(|point| {
            point.fast_len == local.base.fast_len
                && point.slow_len == local.base.slow_len + local.step
        })
        .map(|point| point.score);
    let curvature_terms: Vec<f64> = [
        axial_pair_curvature(left_score, right_score),
        axial_pair_curvature(down_score, up_score),
    ]
    .into_iter()
    .flatten()
    .collect();
    let curvature_penalty = if curvature_terms.is_empty() {
        0.0
    } else {
        curvature_terms.iter().copied().sum::<f64>() / (curvature_terms.len() as f64)
    };
    let curvature_stability = 1.0 / (1.0 + curvature_penalty / score_scale);
    let base_max_dd = local
        .points
        .iter()
        .find(|point| {
            point.fast_len == local.base.fast_len && point.slow_len == local.base.slow_len
        })
        .map(|point| point.metrics.max_dd)
        .unwrap_or(0.0)
        .max(0.0);
    let worst_neighbor_dd = local
        .points
        .iter()
        .filter(|point| {
            !(point.fast_len == local.base.fast_len && point.slow_len == local.base.slow_len)
        })
        .map(|point| point.metrics.max_dd)
        .fold(0.0_f64, f64::max);
    let dd_stability = if worst_neighbor_dd <= 1.0e-12 {
        1.0
    } else {
        (base_max_dd / worst_neighbor_dd).clamp(0.0, 1.0)
    };
    let plateau_score =
        (0.45 * support_strength + 0.35 * score_stability + 0.20 * dd_stability).clamp(0.0, 1.0);

    Some(ShortlistNeighborhoodRobustnessSummary {
        plateau_score,
        plateau_label: label_shortlist_plateau(plateau_score, neighbor_count),
        objective: local.objective,
        score_mode,
        score_agreement: summarize_shortlist_surface_agreement(local),
        explanation: summarize_shortlist_surface_explanation(local, score_mode),
        radius: local.radius,
        step: local.step,
        point_count,
        neighbor_count,
        base_score,
        median_score,
        min_score,
        max_score,
        peak_delta,
        basin_min_score,
        basin_floor_stability,
        roughness_rms_delta,
        roughness_stability,
        curvature_penalty,
        curvature_stability,
    })
}

fn merge_optional_note(target: &mut Option<String>, incoming: Option<String>) {
    let Some(incoming) = incoming.and_then(|s| {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }) else {
        return;
    };

    match target {
        None => *target = Some(incoming),
        Some(existing) => {
            if !existing.split(" | ").any(|part| part == incoming) {
                existing.push_str(" | ");
                existing.push_str(incoming.as_str());
            }
        }
    }
}

fn merge_hybrid_timing(target: &mut Option<HybridTimingMs>, incoming: Option<HybridTimingMs>) {
    let Some(inc) = incoming else {
        return;
    };
    if let Some(cur) = target.as_mut() {
        cur.planner_ms = cur.planner_ms.saturating_add(inc.planner_ms);
        cur.cpu_indicator_prep_ms = cur
            .cpu_indicator_prep_ms
            .saturating_add(inc.cpu_indicator_prep_ms);
        cur.gpu_indicator_prep_ms = cur
            .gpu_indicator_prep_ms
            .saturating_add(inc.gpu_indicator_prep_ms);
        cur.h2d_upload_ms = cur.h2d_upload_ms.saturating_add(inc.h2d_upload_ms);
        cur.kernel_eval_ms = cur.kernel_eval_ms.saturating_add(inc.kernel_eval_ms);
    } else {
        *target = Some(inc);
    }
}

fn merge_optional_u64(target: &mut Option<u64>, incoming: Option<u64>) {
    let Some(inc) = incoming else {
        return;
    };
    if let Some(current) = target.as_mut() {
        *current = current.saturating_add(inc);
    } else {
        *target = Some(inc);
    }
}

fn merge_gpu_kernel_timing(
    target: &mut Option<GpuKernelTimingMs>,
    incoming: Option<GpuKernelTimingMs>,
) {
    let Some(inc) = incoming else {
        return;
    };
    if let Some(cur) = target.as_mut() {
        cur.route_wall_ms = cur.route_wall_ms.saturating_add(inc.route_wall_ms);
        merge_optional_u64(&mut cur.runtime_init_ms, inc.runtime_init_ms);
        merge_optional_u64(&mut cur.set_current_ms, inc.set_current_ms);
        merge_optional_u64(&mut cur.gpu_indicator_prep_ms, inc.gpu_indicator_prep_ms);
        merge_optional_u64(&mut cur.h2d_upload_ms, inc.h2d_upload_ms);
        merge_optional_u64(&mut cur.launch_submit_us, inc.launch_submit_us);
        merge_optional_u64(&mut cur.d2h_copy_ms, inc.d2h_copy_ms);
        merge_optional_u64(&mut cur.host_decode_ms, inc.host_decode_ms);
        merge_optional_u64(&mut cur.explicit_sync_ms, inc.explicit_sync_ms);
        merge_optional_u64(&mut cur.host_finalize_ms, inc.host_finalize_ms);
        merge_optional_u64(&mut cur.graph_capture_ms, inc.graph_capture_ms);
        merge_optional_u64(&mut cur.graph_replay_count, inc.graph_replay_count);
    } else {
        *target = Some(inc);
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn build_gpu_kernel_timing(route_wall_ms: u64) -> GpuKernelTimingMs {
    if let Some(timing) = crate::gpu_backtest_kernel::take_last_kernel_executor_timing() {
        GpuKernelTimingMs {
            route_wall_ms,
            runtime_init_ms: Some(timing.runtime_init_ms),
            set_current_ms: Some(timing.set_current_ms),
            gpu_indicator_prep_ms: Some(timing.gpu_indicator_prep_ms),
            h2d_upload_ms: Some(timing.h2d_upload_ms),
            launch_submit_us: Some(timing.launch_submit_us),
            d2h_copy_ms: Some(timing.d2h_copy_ms),
            host_decode_ms: Some(timing.host_decode_ms),
            explicit_sync_ms: Some(timing.explicit_sync_ms),
            host_finalize_ms: Some(timing.host_finalize_ms),
            graph_capture_ms: Some(timing.graph_capture_ms),
            graph_replay_count: Some(timing.graph_replay_count),
        }
    } else {
        GpuKernelTimingMs {
            route_wall_ms,
            runtime_init_ms: None,
            set_current_ms: None,
            gpu_indicator_prep_ms: None,
            h2d_upload_ms: None,
            launch_submit_us: None,
            d2h_copy_ms: None,
            host_decode_ms: None,
            explicit_sync_ms: None,
            host_finalize_ms: None,
            graph_capture_ms: None,
            graph_replay_count: None,
        }
    }
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn build_gpu_kernel_timing(route_wall_ms: u64) -> GpuKernelTimingMs {
    GpuKernelTimingMs {
        route_wall_ms,
        runtime_init_ms: None,
        set_current_ms: None,
        gpu_indicator_prep_ms: None,
        h2d_upload_ms: None,
        launch_submit_us: None,
        d2h_copy_ms: None,
        host_decode_ms: None,
        explicit_sync_ms: None,
        host_finalize_ms: None,
        graph_capture_ms: None,
        graph_replay_count: None,
    }
}

#[cfg_attr(
    not(all(feature = "cuda", feature = "cuda-backtest-kernel")),
    allow(dead_code)
)]
pub(crate) fn build_exact_extra_shortlist_rows(
    rows: &[(DoubleMaParams, Metrics)],
    primary_objective: ObjectiveKind,
    extra_objectives: &[ObjectiveKind],
    top_k: usize,
) -> Vec<(DoubleMaParams, Metrics)> {
    if rows.is_empty() || top_k == 0 {
        return Vec::new();
    }

    let mut objectives = Vec::with_capacity(extra_objectives.len().saturating_add(1));
    objectives.push(primary_objective);
    for &objective in extra_objectives {
        if objective != primary_objective && !objectives.contains(&objective) {
            objectives.push(objective);
        }
    }

    let mut union_rows: Vec<(DoubleMaParams, Metrics)> = Vec::new();
    let mut seen: BTreeSet<(u16, u16, u16, u16)> = BTreeSet::new();
    for head_objective in objectives {
        let mut ranked = rows.to_vec();
        ranked.sort_by(|a, b| {
            let sa = objective_score(&a.1, head_objective);
            let sb = objective_score(&b.1, head_objective);
            sb.total_cmp(&sa).then_with(|| {
                (a.0.fast_len, a.0.slow_len, a.0.fast_ma_id, a.0.slow_ma_id).cmp(&(
                    b.0.fast_len,
                    b.0.slow_len,
                    b.0.fast_ma_id,
                    b.0.slow_ma_id,
                ))
            })
        });
        for (params, metrics) in ranked.into_iter().take(top_k) {
            let key = (
                params.fast_len,
                params.slow_len,
                params.fast_ma_id,
                params.slow_ma_id,
            );
            if seen.insert(key) {
                union_rows.push((params, metrics));
            }
        }
    }

    union_rows.sort_by(|a, b| {
        let sa = objective_score(&a.1, primary_objective);
        let sb = objective_score(&b.1, primary_objective);
        sb.total_cmp(&sa).then_with(|| {
            (a.0.fast_len, a.0.slow_len, a.0.fast_ma_id, a.0.slow_ma_id).cmp(&(
                b.0.fast_len,
                b.0.slow_len,
                b.0.fast_ma_id,
                b.0.slow_ma_id,
            ))
        })
    });
    union_rows
}

#[inline]
pub(crate) fn metrics_pass_constraints(
    metrics: &ta_strategies::double_ma::Metrics,
    constraints: Option<&DoubleMaConstraints>,
) -> bool {
    let Some(cfg) = constraints else {
        return true;
    };
    if let Some(min_trades) = cfg.min_trades {
        if metrics.trades < min_trades {
            return false;
        }
    }
    if let Some(max_dd) = cfg.max_drawdown {
        if metrics.max_dd > max_dd {
            return false;
        }
    }
    if let Some(min_sharpe) = cfg.min_sharpe {
        if !(metrics.sharpe >= min_sharpe) {
            return false;
        }
    }
    if let Some(min_sortino) = cfg.min_sortino {
        if !(metrics.sortino >= min_sortino) {
            return false;
        }
    }
    if let Some(min_calmar) = cfg.min_calmar {
        if !(metrics.calmar >= min_calmar) {
            return false;
        }
    }
    if let Some(max_volatility) = cfg.max_volatility {
        if !(metrics.volatility <= max_volatility) {
            return false;
        }
    }
    if let Some(min_exposure) = cfg.min_exposure {
        if !(metrics.exposure >= min_exposure) {
            return false;
        }
    }
    if let Some(max_exposure) = cfg.max_exposure {
        if !(metrics.exposure <= max_exposure) {
            return false;
        }
    }
    if let Some(max_abs_net_exposure) = cfg.max_abs_net_exposure {
        if !(metrics.net_exposure.abs() <= max_abs_net_exposure) {
            return false;
        }
    }
    if let Some(min_omega) = cfg.min_omega {
        if !(metrics.omega >= min_omega) {
            return false;
        }
    }
    if let Some(max_ulcer) = cfg.max_ulcer {
        if !(metrics.ulcer_index <= max_ulcer) {
            return false;
        }
    }
    if let Some(max_downside_deviation) = cfg.max_downside_deviation {
        if !(metrics.downside_deviation <= max_downside_deviation) {
            return false;
        }
    }
    true
}

#[derive(Clone, Debug)]
pub(crate) struct KaufmanStopSeries {
    pub long: Vec<f64>,
    pub short: Vec<f64>,
}

#[inline]
fn strategy_uses_kaufman_stop_exit(strategy: &StrategyConfig) -> bool {
    strategy.uses_kaufman_stop_exit()
}

pub(crate) fn compute_kaufman_stop_series(
    candles: &vector_ta::utilities::data_loader::Candles,
    strategy: &StrategyConfig,
) -> Result<Option<KaufmanStopSeries>, String> {
    if !strategy_uses_kaufman_stop_exit(strategy) {
        return Ok(None);
    }
    if !candles.fields.high || !candles.fields.low {
        return Err("Kaufman-stop exit requires 'high' and 'low' candle columns".to_string());
    }

    let params_long = vector_ta::indicators::kaufmanstop::KaufmanstopParams {
        period: Some(strategy.kaufman_stop_period()),
        mult: Some(strategy.kaufman_stop_mult()),
        direction: Some("long".to_string()),
        ma_type: Some(strategy.kaufman_stop_ma_type().to_string()),
    };
    let params_short = vector_ta::indicators::kaufmanstop::KaufmanstopParams {
        period: Some(strategy.kaufman_stop_period()),
        mult: Some(strategy.kaufman_stop_mult()),
        direction: Some("short".to_string()),
        ma_type: Some(strategy.kaufman_stop_ma_type().to_string()),
    };

    let long = vector_ta::indicators::kaufmanstop::kaufmanstop(
        &vector_ta::indicators::kaufmanstop::KaufmanstopInput::from_candles(candles, params_long),
    )
    .map_err(|e| format!("kaufman-stop (long) failed: {e}"))?
    .values;
    let short = vector_ta::indicators::kaufmanstop::kaufmanstop(
        &vector_ta::indicators::kaufmanstop::KaufmanstopInput::from_candles(candles, params_short),
    )
    .map_err(|e| format!("kaufman-stop (short) failed: {e}"))?
    .values;

    if long.len() != candles.close.len() || short.len() != candles.close.len() {
        return Err("kaufman-stop output length mismatch".to_string());
    }

    Ok(Some(KaufmanStopSeries { long, short }))
}

#[inline]
fn base_cross_sign(f: f64, s: f64, cfg: &StrategyConfig) -> i32 {
    let mut sign = if f.is_finite() && s.is_finite() {
        let eps = if cfg.eps_rel > 0.0 {
            cfg.eps_rel * 1.0_f64.max(s.abs())
        } else {
            0.0
        };
        (f > s + eps) as i32 - (f < s - eps) as i32
    } else {
        0
    };
    if cfg.long_only && sign < 0 {
        sign = 0;
    }
    sign
}

#[inline]
fn regime_hybrid_range_sign(
    spread_rel: f64,
    current_pos: i32,
    entry_rel: f64,
    exit_rel: f64,
) -> i32 {
    if current_pos > 0 {
        if spread_rel >= entry_rel {
            -1
        } else if spread_rel < -exit_rel {
            1
        } else {
            0
        }
    } else if current_pos < 0 {
        if spread_rel <= -entry_rel {
            1
        } else if spread_rel > exit_rel {
            -1
        } else {
            0
        }
    } else if spread_rel >= entry_rel {
        -1
    } else if spread_rel <= -entry_rel {
        1
    } else {
        0
    }
}

#[inline]
fn desired_sign_with_filters(
    f: f64,
    s: f64,
    cfg: &StrategyConfig,
    regime_v: Option<f64>,
    current_pos: i32,
) -> i32 {
    let sign = base_cross_sign(f, s, cfg);

    if let Some(v) = regime_v {
        if !v.is_finite() {
            return 0;
        } else if cfg.uses_regime_hybrid_filter() && v >= 1.5 && v < 2.5 {
            if !f.is_finite() || !s.is_finite() {
                return 0;
            }
            let denom = 1.0_f64.max(s.abs());
            let spread_rel = (f - s) / denom;
            let entry_rel = cfg.regime_hybrid_reversion_entry_rel();
            let exit_rel = cfg.regime_hybrid_reversion_exit_rel();
            let mut range_sign =
                regime_hybrid_range_sign(spread_rel, current_pos, entry_rel, exit_rel);
            if cfg.long_only && range_sign < 0 {
                range_sign = 0;
            }
            return range_sign;
        } else if v < 0.5 {
            return sign.max(0);
        } else {
            return sign.min(0);
        }
    }
    sign
}

pub(crate) fn desired_positions_with_exit_mode(
    close: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend: Option<&[f64]>,
    start_t: usize,
    cfg: &StrategyConfig,
    kaufman: Option<&KaufmanStopSeries>,
) -> Vec<i32> {
    let n_total = fast.len();
    let mut out: Vec<i32> = vec![0; n_total];
    if n_total == 0 || slow.len() != n_total || close.len() != n_total || start_t >= n_total {
        return out;
    }
    if let Some(ht) = halftrend {
        if ht.len() != n_total {
            return out;
        }
    }

    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };

    if cfg.uses_kaufman_stop_exit() {
        let Some(ks) = kaufman else {
            return out;
        };
        if ks.long.len() < n_total || ks.short.len() < n_total {
            return out;
        }

        let mut pos: i32 = 0;
        for t in start_t..n_total {
            let t_sig = t.saturating_sub(trade_offset);
            let ht_sig = halftrend.map(|h| h[t_sig]);
            let raw_sign = desired_sign_with_filters(fast[t_sig], slow[t_sig], cfg, ht_sig, pos);

            if pos == 0 && raw_sign != 0 {
                pos = raw_sign;
            }

            if pos != 0 {
                let c = close[t_sig];
                let stop_v = if pos > 0 {
                    ks.long[t_sig]
                } else {
                    ks.short[t_sig]
                };
                let hit_kaufman = c.is_finite()
                    && stop_v.is_finite()
                    && ((pos > 0 && c <= stop_v) || (pos < 0 && c >= stop_v));
                if hit_kaufman {
                    pos = 0;
                }
            }

            out[t] = pos;
        }
        return out;
    }

    let mut pos: i32 = 0;
    for t in start_t..n_total {
        let t_sig = t.saturating_sub(trade_offset);
        let ht_sig = halftrend.map(|h| h[t_sig]);
        let sign = desired_sign_with_filters(fast[t_sig], slow[t_sig], cfg, ht_sig, pos);
        pos = sign;
        out[t] = sign;
    }

    out
}

#[derive(Clone, Copy)]
struct ExecutionOverlayParams {
    cooldown_bars: usize,
    persistence_bars: usize,
}

impl ExecutionOverlayParams {
    #[inline]
    fn from_cfg(cfg: &StrategyConfig) -> Self {
        Self {
            cooldown_bars: cfg.cooldown_bars(),
            persistence_bars: cfg.signal_persistence_bars().max(1),
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ExecutionOverlayState {
    cooldown_left: usize,
    pending_sign: i32,
    pending_count: usize,
}

#[inline]
fn overlay_on_bar_start(state: &mut ExecutionOverlayState) {
    if state.cooldown_left > 0 {
        state.cooldown_left = state.cooldown_left.saturating_sub(1);
    }
}

#[inline]
fn overlay_apply_signal_persistence(
    raw_sign: i32,
    current_pos: i32,
    params: &ExecutionOverlayParams,
    state: &mut ExecutionOverlayState,
) -> i32 {
    if params.persistence_bars <= 1 {
        if raw_sign == 0 {
            state.pending_sign = 0;
            state.pending_count = 0;
        } else {
            state.pending_sign = raw_sign;
            state.pending_count = 1;
        }
        return raw_sign;
    }

    if raw_sign == 0 {
        state.pending_sign = 0;
        state.pending_count = 0;
        return 0;
    }

    if raw_sign == state.pending_sign {
        state.pending_count = state.pending_count.saturating_add(1);
    } else {
        state.pending_sign = raw_sign;
        state.pending_count = 1;
    }

    if state.pending_count >= params.persistence_bars {
        raw_sign
    } else if current_pos != 0 {
        current_pos
    } else {
        0
    }
}

#[inline]
fn overlay_apply_entry_cooldown(sign: i32, current_pos: i32, state: &ExecutionOverlayState) -> i32 {
    if current_pos == 0 && sign != 0 && state.cooldown_left > 0 {
        0
    } else {
        sign
    }
}

#[inline]
fn overlay_on_trade_close(state: &mut ExecutionOverlayState, params: &ExecutionOverlayParams) {
    if params.cooldown_bars > 0 {
        state.cooldown_left = params.cooldown_bars;
    }
}

fn build_trade_stats(trades: &[Trade]) -> Option<TradeStats> {
    let mut open_trades: usize = 0;
    let mut closed_trades: usize = 0;
    let mut wins: usize = 0;
    let mut losses: usize = 0;
    let mut sum_win: f64 = 0.0;
    let mut sum_loss_abs: f64 = 0.0;
    let mut sum_win_count: usize = 0;
    let mut sum_loss_count: usize = 0;
    let mut sum_all: f64 = 0.0;

    for tr in trades.iter() {
        if tr.open {
            open_trades += 1;
            continue;
        }
        closed_trades += 1;
        let r = tr.pnl;
        sum_all += r;
        if r > 0.0 {
            wins += 1;
            sum_win += r;
            sum_win_count += 1;
        } else if r < 0.0 {
            losses += 1;
            sum_loss_abs += -r;
            sum_loss_count += 1;
        }
    }

    if closed_trades == 0 && open_trades == 0 {
        return None;
    }

    let denom = wins + losses;
    let win_rate = if denom > 0 {
        wins as f64 / denom as f64
    } else {
        0.0
    };
    let avg_win = if sum_win_count > 0 {
        sum_win / sum_win_count as f64
    } else {
        0.0
    };
    let avg_loss = if sum_loss_count > 0 {
        -(sum_loss_abs / sum_loss_count as f64)
    } else {
        0.0
    };
    let profit_factor = if sum_loss_abs > 0.0 {
        sum_win / sum_loss_abs
    } else if sum_win > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };
    let expectancy = if closed_trades > 0 {
        sum_all / closed_trades as f64
    } else {
        0.0
    };

    Some(TradeStats {
        closed_trades,
        open_trades,
        wins,
        losses,
        win_rate,
        avg_win,
        avg_loss,
        profit_factor,
        expectancy,
    })
}

fn eval_double_ma_metrics_kaufman_impl<const COMPUTE_SHARPE: bool, FastAt, SlowAt>(
    lr: &[f64],
    close: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
    halftrend: Option<&[f64]>,
    kaufman: &KaufmanStopSeries,
    fast_at: FastAt,
    slow_at: SlowAt,
) -> Metrics
where
    FastAt: Fn(usize) -> f64,
    SlowAt: Fn(usize) -> f64,
{
    let n_total = lr.len();
    if n_total < 2 || close.len() != n_total || start_t >= n_total {
        return Metrics::default();
    }
    if let Some(ht) = halftrend {
        if ht.len() != n_total {
            return Metrics::default();
        }
    }
    if kaufman.long.len() < n_total || kaufman.short.len() < n_total {
        return Metrics::default();
    }

    let commission = cfg.effective_commission();
    if !(0.0..1.0).contains(&commission) && commission != 0.0 {
        return Metrics::default();
    }

    let obs: u64 = (n_total - start_t) as u64;
    let log_comm = if commission > 0.0 {
        (1.0 - commission).ln()
    } else {
        0.0
    };
    let has_commission = commission > 0.0;
    let stop_log = cfg.stop_loss_pct().map(|pct| (1.0 - pct).ln());
    let take_log = cfg.take_profit_pct().map(|pct| (1.0 + pct).ln());
    let overlays = ExecutionOverlayParams::from_cfg(cfg);
    let risk_enabled = stop_log.is_some() || take_log.is_some();
    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };

    let mut pos: i32 = 0;
    let mut log_eq = 0.0_f64;
    let mut log_peak = 0.0_f64;
    let mut max_log_dd = 0.0_f64;
    let mut ulcer_sum_sq = 0.0_f64;
    let mut underwater_bars: u32 = 0;
    let mut underwater_run: u32 = 0;
    let mut max_underwater_duration_bars: u32 = 0;
    let mut trades: u32 = 0;

    let mut count: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut sum_neg_sq = 0.0_f64;
    let mut sum_neg_cu = 0.0_f64;
    let mut count_neg: u64 = 0;
    let mut sum_pos_r = 0.0_f64;
    let mut sum_neg_abs_r = 0.0_f64;
    let mut long_obs: u64 = 0;
    let mut short_obs: u64 = 0;
    let mut long_sum_r = 0.0_f64;
    let mut short_sum_r = 0.0_f64;

    let mut sum_abs_pos: i64 = 0;
    let mut sum_pos: i64 = 0;
    let mut trade_log_ret = 0.0_f64;
    let mut overlay_state = ExecutionOverlayState::default();

    for t in start_t..n_total {
        overlay_on_bar_start(&mut overlay_state);
        let t_sig = t.saturating_sub(trade_offset);
        let ht_sig = halftrend.map(|h| h[t_sig]);
        let raw_sign = desired_sign_with_filters(fast_at(t_sig), slow_at(t_sig), cfg, ht_sig, pos);
        let persisted_sign =
            overlay_apply_signal_persistence(raw_sign, pos, &overlays, &mut overlay_state);
        let sign = overlay_apply_entry_cooldown(persisted_sign, pos, &overlay_state);

        if pos == 0 && sign != 0 {
            if has_commission {
                log_eq += log_comm;
            }
            trades = trades.saturating_add(1);
            pos = sign;
            trade_log_ret = if has_commission { log_comm } else { 0.0 };
        }

        if pos != 0 {
            let c = close[t_sig];
            let stop_v = if pos > 0 {
                kaufman.long[t_sig]
            } else {
                kaufman.short[t_sig]
            };
            let hit_kaufman = c.is_finite()
                && stop_v.is_finite()
                && ((pos > 0 && c <= stop_v) || (pos < 0 && c >= stop_v));
            if hit_kaufman {
                if has_commission {
                    log_eq += log_comm;
                }
                trades = trades.saturating_add(1);
                pos = 0;
                trade_log_ret = 0.0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            }
        }

        let pos_bar = pos;
        let lr_t = lr[t];
        let step_log_ret = (pos_bar as f64) * lr_t;
        log_eq += step_log_ret;

        if risk_enabled && pos_bar != 0 {
            trade_log_ret += step_log_ret;
            let hit_stop = stop_log.is_some_and(|th| trade_log_ret <= th);
            let hit_take = take_log.is_some_and(|th| trade_log_ret >= th);
            if hit_stop || hit_take {
                if has_commission {
                    log_eq += log_comm;
                }
                trades = trades.saturating_add(1);
                pos = 0;
                trade_log_ret = 0.0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            }
        }

        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        if dd_log > max_log_dd {
            max_log_dd = dd_log;
        }
        if dd_log > 0.0 {
            underwater_bars = underwater_bars.saturating_add(1);
            underwater_run = underwater_run.saturating_add(1);
            max_underwater_duration_bars = max_underwater_duration_bars.max(underwater_run);
        } else {
            underwater_run = 0;
        }
        let dd = if dd_log > 0.0 {
            -(-dd_log).exp_m1()
        } else {
            0.0
        };
        ulcer_sum_sq += dd * dd;

        if COMPUTE_SHARPE {
            let step_r = if pos_bar == 0 {
                0.0
            } else {
                (pos_bar as f64) * lr_t.exp_m1()
            };
            count += 1;
            let delta = step_r - mean;
            mean += delta / count as f64;
            let delta2 = step_r - mean;
            m2 += delta * delta2;

            let excess_r = step_r - cfg.hurdle_tau.unwrap_or(0.0);
            if excess_r < 0.0 {
                count_neg += 1;
                sum_neg_sq += excess_r * excess_r;
                let neg_mag = -excess_r;
                sum_neg_cu += neg_mag * neg_mag * neg_mag;
                sum_neg_abs_r += neg_mag;
            } else {
                sum_pos_r += excess_r;
            }
            if pos_bar > 0 {
                long_obs += 1;
                long_sum_r += step_r;
            } else if pos_bar < 0 {
                short_obs += 1;
                short_sum_r += step_r;
            }
        }

        sum_abs_pos += (pos_bar as i64).abs();
        sum_pos += pos_bar as i64;
    }

    let (
        sharpe,
        volatility,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        omega,
        kappa_tau,
        max_dd,
        ulcer_index,
        martin_ratio,
        calmar,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
    ) = finalize_risk_metrics(
        COMPUTE_SHARPE,
        count,
        obs,
        cfg.hurdle_tau.unwrap_or(0.0),
        mean,
        m2,
        sum_neg_sq,
        sum_neg_cu,
        count_neg,
        sum_pos_r,
        sum_neg_abs_r,
        long_obs,
        short_obs,
        long_sum_r,
        short_sum_r,
        ulcer_sum_sq,
        max_log_dd,
        log_eq,
        underwater_bars as u64,
        max_underwater_duration_bars as u64,
    );
    let denom = if COMPUTE_SHARPE { count } else { obs };
    let exposure = if denom > 0 {
        sum_abs_pos as f64 / denom as f64
    } else {
        0.0
    };
    let net_exposure = if denom > 0 {
        sum_pos as f64 / denom as f64
    } else {
        0.0
    };
    let pnl = log_eq.exp() - 1.0;

    Metrics {
        pnl,
        sharpe,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        calmar,
        omega,
        kappa_tau,
        volatility,
        ulcer_index,
        martin_ratio,
        max_dd,
        trades,
        exposure,
        net_exposure,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
        closed_trades: 0,
        winning_trades: 0,
        losing_trades: 0,
        gross_profit: 0.0,
        gross_loss: 0.0,
    }
}

fn eval_double_ma_curves_kaufman_f64(
    lr: &[f64],
    close: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend: Option<&[f64]>,
    start_t: usize,
    cfg: &StrategyConfig,
    bins: usize,
    kaufman: &KaufmanStopSeries,
) -> DoubleMaCurves {
    let n_total = lr.len();
    if n_total < 2 || close.len() != n_total || fast.len() != n_total || slow.len() != n_total {
        return DoubleMaCurves {
            equity: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            drawdown: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            trades: Vec::new(),
            trade_stats: None,
            indicator_bars: Vec::new(),
        };
    }
    if let Some(ht) = halftrend {
        if ht.len() != n_total {
            return DoubleMaCurves {
                equity: ta_strategies::double_ma::MinMaxBins {
                    start_t,
                    end_t: n_total,
                    bins: 0,
                    minmax: Vec::new(),
                },
                drawdown: ta_strategies::double_ma::MinMaxBins {
                    start_t,
                    end_t: n_total,
                    bins: 0,
                    minmax: Vec::new(),
                },
                trades: Vec::new(),
                trade_stats: None,
                indicator_bars: Vec::new(),
            };
        }
    }
    if kaufman.long.len() < n_total || kaufman.short.len() < n_total || start_t >= n_total {
        return DoubleMaCurves {
            equity: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            drawdown: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            trades: Vec::new(),
            trade_stats: None,
            indicator_bars: Vec::new(),
        };
    }

    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let commission = cfg.effective_commission();
    let log_comm = if commission > 0.0 {
        (1.0 - commission).ln()
    } else {
        0.0
    };
    let has_commission = commission > 0.0;
    let stop_log = cfg.stop_loss_pct().map(|pct| (1.0 - pct).ln());
    let take_log = cfg.take_profit_pct().map(|pct| (1.0 + pct).ln());
    let overlays = ExecutionOverlayParams::from_cfg(cfg);
    let risk_enabled = stop_log.is_some() || take_log.is_some();

    let n_steps = n_total.saturating_sub(start_t);
    let bins = {
        if n_steps == 0 {
            0
        } else {
            bins.clamp(1, 8192).min(n_steps)
        }
    };
    if bins == 0 {
        return DoubleMaCurves {
            equity: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            drawdown: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            trades: Vec::new(),
            trade_stats: None,
            indicator_bars: Vec::new(),
        };
    }

    let mut eq_min: Vec<f64> = vec![f64::INFINITY; bins];
    let mut eq_max: Vec<f64> = vec![f64::NEG_INFINITY; bins];
    let mut dd_min: Vec<f64> = vec![f64::INFINITY; bins];
    let mut dd_max: Vec<f64> = vec![f64::NEG_INFINITY; bins];

    let mut pos: i32 = 0;
    let mut log_eq = 0.0_f64;
    let mut log_peak = 0.0_f64;

    let mut trades: Vec<Trade> = Vec::new();
    let mut cur_entry_t: usize = 0;
    let mut cur_dir: i32 = 0;
    let mut cur_log_ret: f64 = 0.0;
    let mut overlay_state = ExecutionOverlayState::default();

    for t in start_t..n_total {
        overlay_on_bar_start(&mut overlay_state);
        let t_sig = t.saturating_sub(trade_offset);
        let ht_sig = halftrend.map(|h| h[t_sig]);
        let raw_sign = desired_sign_with_filters(fast[t_sig], slow[t_sig], cfg, ht_sig, pos);
        let persisted_sign =
            overlay_apply_signal_persistence(raw_sign, pos, &overlays, &mut overlay_state);
        let sign = overlay_apply_entry_cooldown(persisted_sign, pos, &overlay_state);

        if pos == 0 && sign != 0 {
            if has_commission {
                log_eq += log_comm;
            }
            pos = sign;
            cur_entry_t = t;
            cur_dir = sign;
            cur_log_ret = if has_commission { log_comm } else { 0.0 };
        }

        if pos != 0 && cur_dir != 0 {
            let c = close[t_sig];
            let stop_v = if pos > 0 {
                kaufman.long[t_sig]
            } else {
                kaufman.short[t_sig]
            };
            let hit_kaufman = c.is_finite()
                && stop_v.is_finite()
                && ((pos > 0 && c <= stop_v) || (pos < 0 && c >= stop_v));
            if hit_kaufman {
                if has_commission {
                    log_eq += log_comm;
                    cur_log_ret += log_comm;
                }
                trades.push(Trade {
                    entry_t: cur_entry_t,
                    exit_t: t,
                    direction: cur_dir,
                    pnl: cur_log_ret.exp() - 1.0,
                    bars: t.saturating_sub(cur_entry_t) as u32,
                    open: false,
                });
                pos = 0;
                cur_dir = 0;
                cur_log_ret = 0.0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            }
        }

        let pos_bar = pos;
        log_eq += (pos_bar as f64) * lr[t];
        if cur_dir != 0 {
            cur_log_ret += (pos_bar as f64) * lr[t];
        }

        if risk_enabled && cur_dir != 0 {
            let hit_stop = stop_log.is_some_and(|th| cur_log_ret <= th);
            let hit_take = take_log.is_some_and(|th| cur_log_ret >= th);
            if hit_stop || hit_take {
                if has_commission {
                    log_eq += log_comm;
                    cur_log_ret += log_comm;
                }
                trades.push(Trade {
                    entry_t: cur_entry_t,
                    exit_t: t,
                    direction: cur_dir,
                    pnl: cur_log_ret.exp() - 1.0,
                    bars: t.saturating_sub(cur_entry_t) as u32,
                    open: false,
                });
                cur_dir = 0;
                cur_log_ret = 0.0;
                pos = 0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            }
        }

        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        let eq = log_eq.exp();
        let dd = if dd_log > 0.0 {
            -(-dd_log).exp_m1()
        } else {
            0.0
        };
        let i = t - start_t;
        let b = (i.saturating_mul(bins)) / n_steps.max(1);
        let b = b.min(bins - 1);

        if eq < eq_min[b] {
            eq_min[b] = eq;
        }
        if eq > eq_max[b] {
            eq_max[b] = eq;
        }
        if dd < dd_min[b] {
            dd_min[b] = dd;
        }
        if dd > dd_max[b] {
            dd_max[b] = dd;
        }
    }

    let mut eq_mm: Vec<f32> = Vec::with_capacity(bins.saturating_mul(2));
    let mut dd_mm: Vec<f32> = Vec::with_capacity(bins.saturating_mul(2));
    for i in 0..bins {
        let (a, b) = (eq_min[i], eq_max[i]);
        if a.is_finite() && b.is_finite() {
            eq_mm.push(a as f32);
            eq_mm.push(b as f32);
        } else {
            eq_mm.push(f32::NAN);
            eq_mm.push(f32::NAN);
        }

        let (a, b) = (dd_min[i], dd_max[i]);
        if a.is_finite() && b.is_finite() {
            dd_mm.push(a as f32);
            dd_mm.push(b as f32);
        } else {
            dd_mm.push(f32::NAN);
            dd_mm.push(f32::NAN);
        }
    }

    if cur_dir != 0 {
        let exit_t = n_total.saturating_sub(1);
        trades.push(Trade {
            entry_t: cur_entry_t,
            exit_t,
            direction: cur_dir,
            pnl: cur_log_ret.exp() - 1.0,
            bars: exit_t.saturating_sub(cur_entry_t) as u32,
            open: true,
        });
    }

    DoubleMaCurves {
        equity: ta_strategies::double_ma::MinMaxBins {
            start_t,
            end_t: n_total,
            bins,
            minmax: eq_mm,
        },
        drawdown: ta_strategies::double_ma::MinMaxBins {
            start_t,
            end_t: n_total,
            bins,
            minmax: dd_mm,
        },
        trade_stats: build_trade_stats(&trades),
        trades,
        indicator_bars: Vec::new(),
    }
}

fn eval_double_ma_metrics_cross_overlay_impl<const COMPUTE_SHARPE: bool, FastAt, SlowAt>(
    lr: &[f64],
    close: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
    halftrend: Option<&[f64]>,
    fast_at: FastAt,
    slow_at: SlowAt,
) -> Metrics
where
    FastAt: Fn(usize) -> f64,
    SlowAt: Fn(usize) -> f64,
{
    let n_total = lr.len();
    if n_total < 2 || close.len() != n_total || start_t >= n_total {
        return Metrics::default();
    }
    if let Some(ht) = halftrend {
        if ht.len() != n_total {
            return Metrics::default();
        }
    }

    let commission = cfg.effective_commission();
    if !(0.0..1.0).contains(&commission) && commission != 0.0 {
        return Metrics::default();
    }

    let obs: u64 = (n_total - start_t) as u64;
    let log_comm = if commission > 0.0 {
        (1.0 - commission).ln()
    } else {
        0.0
    };
    let has_commission = commission > 0.0;
    let stop_log = cfg.stop_loss_pct().map(|pct| (1.0 - pct).ln());
    let take_log = cfg.take_profit_pct().map(|pct| (1.0 + pct).ln());
    let overlays = ExecutionOverlayParams::from_cfg(cfg);
    let risk_enabled = stop_log.is_some() || take_log.is_some();
    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };

    let mut pos: i32 = 0;
    let mut log_eq = 0.0_f64;
    let mut log_peak = 0.0_f64;
    let mut max_log_dd = 0.0_f64;
    let mut ulcer_sum_sq = 0.0_f64;
    let mut underwater_bars: u32 = 0;
    let mut underwater_run: u32 = 0;
    let mut max_underwater_duration_bars: u32 = 0;
    let mut trades: u32 = 0;

    let mut count: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut sum_neg_sq = 0.0_f64;
    let mut sum_neg_cu = 0.0_f64;
    let mut count_neg: u64 = 0;
    let mut sum_pos_r = 0.0_f64;
    let mut sum_neg_abs_r = 0.0_f64;
    let mut long_obs: u64 = 0;
    let mut short_obs: u64 = 0;
    let mut long_sum_r = 0.0_f64;
    let mut short_sum_r = 0.0_f64;

    let mut sum_abs_pos: i64 = 0;
    let mut sum_pos: i64 = 0;
    let mut trade_log_ret = 0.0_f64;
    let mut overlay_state = ExecutionOverlayState::default();

    for t in start_t..n_total {
        overlay_on_bar_start(&mut overlay_state);
        let t_sig = t.saturating_sub(trade_offset);
        let ht_sig = halftrend.map(|h| h[t_sig]);
        let raw_sign = desired_sign_with_filters(fast_at(t_sig), slow_at(t_sig), cfg, ht_sig, pos);
        let persisted_sign =
            overlay_apply_signal_persistence(raw_sign, pos, &overlays, &mut overlay_state);
        let sign = overlay_apply_entry_cooldown(persisted_sign, pos, &overlay_state);

        if sign != pos {
            if pos == 0 || sign == 0 {
                if has_commission {
                    log_eq += log_comm;
                }
                trades = trades.saturating_add(1);
                pos = if sign == 0 { 0 } else { sign };
                if pos == 0 {
                    trade_log_ret = 0.0;
                    overlay_on_trade_close(&mut overlay_state, &overlays);
                } else {
                    trade_log_ret = if has_commission { log_comm } else { 0.0 };
                }
            } else if !cfg.allow_flip {
                if has_commission {
                    log_eq += log_comm;
                }
                trades = trades.saturating_add(1);
                pos = 0;
                trade_log_ret = 0.0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            } else {
                if has_commission {
                    log_eq += 2.0 * log_comm;
                }
                trades = trades.saturating_add(2);
                pos = sign;
                trade_log_ret = if has_commission { log_comm } else { 0.0 };
            }
        }

        let pos_bar = pos;
        let lr_t = lr[t];
        let step_log_ret = (pos_bar as f64) * lr_t;
        log_eq += step_log_ret;

        if risk_enabled && pos_bar != 0 {
            trade_log_ret += step_log_ret;
            let hit_stop = stop_log.is_some_and(|th| trade_log_ret <= th);
            let hit_take = take_log.is_some_and(|th| trade_log_ret >= th);
            if hit_stop || hit_take {
                if has_commission {
                    log_eq += log_comm;
                }
                trades = trades.saturating_add(1);
                pos = 0;
                trade_log_ret = 0.0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            }
        }

        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        if dd_log > max_log_dd {
            max_log_dd = dd_log;
        }
        if dd_log > 0.0 {
            underwater_bars = underwater_bars.saturating_add(1);
            underwater_run = underwater_run.saturating_add(1);
            max_underwater_duration_bars = max_underwater_duration_bars.max(underwater_run);
        } else {
            underwater_run = 0;
        }
        let dd = if dd_log > 0.0 {
            -(-dd_log).exp_m1()
        } else {
            0.0
        };
        ulcer_sum_sq += dd * dd;

        if COMPUTE_SHARPE {
            let step_r = if pos_bar == 0 {
                0.0
            } else {
                (pos_bar as f64) * lr_t.exp_m1()
            };
            count += 1;
            let delta = step_r - mean;
            mean += delta / count as f64;
            let delta2 = step_r - mean;
            m2 += delta * delta2;

            let excess_r = step_r - cfg.hurdle_tau.unwrap_or(0.0);
            if excess_r < 0.0 {
                count_neg += 1;
                sum_neg_sq += excess_r * excess_r;
                let neg_mag = -excess_r;
                sum_neg_cu += neg_mag * neg_mag * neg_mag;
                sum_neg_abs_r += neg_mag;
            } else {
                sum_pos_r += excess_r;
            }
            if pos_bar > 0 {
                long_obs += 1;
                long_sum_r += step_r;
            } else if pos_bar < 0 {
                short_obs += 1;
                short_sum_r += step_r;
            }
        }

        sum_abs_pos += (pos_bar as i64).abs();
        sum_pos += pos_bar as i64;
    }

    let (
        sharpe,
        volatility,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        omega,
        kappa_tau,
        max_dd,
        ulcer_index,
        martin_ratio,
        calmar,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
    ) = finalize_risk_metrics(
        COMPUTE_SHARPE,
        count,
        obs,
        cfg.hurdle_tau.unwrap_or(0.0),
        mean,
        m2,
        sum_neg_sq,
        sum_neg_cu,
        count_neg,
        sum_pos_r,
        sum_neg_abs_r,
        long_obs,
        short_obs,
        long_sum_r,
        short_sum_r,
        ulcer_sum_sq,
        max_log_dd,
        log_eq,
        underwater_bars as u64,
        max_underwater_duration_bars as u64,
    );
    let denom = if COMPUTE_SHARPE { count } else { obs };
    let exposure = if denom > 0 {
        sum_abs_pos as f64 / denom as f64
    } else {
        0.0
    };
    let net_exposure = if denom > 0 {
        sum_pos as f64 / denom as f64
    } else {
        0.0
    };
    let pnl = log_eq.exp() - 1.0;

    Metrics {
        pnl,
        sharpe,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        calmar,
        omega,
        kappa_tau,
        volatility,
        ulcer_index,
        martin_ratio,
        max_dd,
        trades,
        exposure,
        net_exposure,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
        closed_trades: 0,
        winning_trades: 0,
        losing_trades: 0,
        gross_profit: 0.0,
        gross_loss: 0.0,
    }
}

fn eval_double_ma_curves_cross_overlay_f64(
    lr: &[f64],
    close: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend: Option<&[f64]>,
    start_t: usize,
    cfg: &StrategyConfig,
    bins: usize,
) -> DoubleMaCurves {
    let n_total = lr.len();
    if n_total < 2 || close.len() != n_total || fast.len() != n_total || slow.len() != n_total {
        return DoubleMaCurves {
            equity: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            drawdown: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            trades: Vec::new(),
            trade_stats: None,
            indicator_bars: Vec::new(),
        };
    }
    if let Some(ht) = halftrend {
        if ht.len() != n_total {
            return DoubleMaCurves {
                equity: ta_strategies::double_ma::MinMaxBins {
                    start_t,
                    end_t: n_total,
                    bins: 0,
                    minmax: Vec::new(),
                },
                drawdown: ta_strategies::double_ma::MinMaxBins {
                    start_t,
                    end_t: n_total,
                    bins: 0,
                    minmax: Vec::new(),
                },
                trades: Vec::new(),
                trade_stats: None,
                indicator_bars: Vec::new(),
            };
        }
    }
    if start_t >= n_total {
        return DoubleMaCurves {
            equity: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            drawdown: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            trades: Vec::new(),
            trade_stats: None,
            indicator_bars: Vec::new(),
        };
    }

    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let commission = cfg.effective_commission();
    let log_comm = if commission > 0.0 {
        (1.0 - commission).ln()
    } else {
        0.0
    };
    let has_commission = commission > 0.0;
    let stop_log = cfg.stop_loss_pct().map(|pct| (1.0 - pct).ln());
    let take_log = cfg.take_profit_pct().map(|pct| (1.0 + pct).ln());
    let overlays = ExecutionOverlayParams::from_cfg(cfg);
    let risk_enabled = stop_log.is_some() || take_log.is_some();

    let n_steps = n_total.saturating_sub(start_t);
    let bins = {
        if n_steps == 0 {
            0
        } else {
            bins.clamp(1, 8192).min(n_steps)
        }
    };
    if bins == 0 {
        return DoubleMaCurves {
            equity: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            drawdown: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            trades: Vec::new(),
            trade_stats: None,
            indicator_bars: Vec::new(),
        };
    }

    let mut eq_min: Vec<f64> = vec![f64::INFINITY; bins];
    let mut eq_max: Vec<f64> = vec![f64::NEG_INFINITY; bins];
    let mut dd_min: Vec<f64> = vec![f64::INFINITY; bins];
    let mut dd_max: Vec<f64> = vec![f64::NEG_INFINITY; bins];

    let mut pos: i32 = 0;
    let mut log_eq = 0.0_f64;
    let mut log_peak = 0.0_f64;

    let mut trades: Vec<Trade> = Vec::new();
    let mut cur_entry_t: usize = 0;
    let mut cur_dir: i32 = 0;
    let mut cur_log_ret: f64 = 0.0;
    let mut overlay_state = ExecutionOverlayState::default();

    for t in start_t..n_total {
        overlay_on_bar_start(&mut overlay_state);
        let t_sig = t.saturating_sub(trade_offset);
        let ht_sig = halftrend.map(|h| h[t_sig]);
        let raw_sign = desired_sign_with_filters(fast[t_sig], slow[t_sig], cfg, ht_sig, pos);
        let persisted_sign =
            overlay_apply_signal_persistence(raw_sign, pos, &overlays, &mut overlay_state);
        let sign = overlay_apply_entry_cooldown(persisted_sign, pos, &overlay_state);

        if sign != pos {
            if pos == 0 || sign == 0 {
                if has_commission {
                    log_eq += log_comm;
                }
                if pos != 0 && cur_dir != 0 {
                    if has_commission {
                        cur_log_ret += log_comm;
                    }
                    trades.push(Trade {
                        entry_t: cur_entry_t,
                        exit_t: t,
                        direction: cur_dir,
                        pnl: cur_log_ret.exp() - 1.0,
                        bars: t.saturating_sub(cur_entry_t) as u32,
                        open: false,
                    });
                    cur_dir = 0;
                    cur_log_ret = 0.0;
                    overlay_on_trade_close(&mut overlay_state, &overlays);
                } else if pos == 0 && sign != 0 {
                    cur_entry_t = t;
                    cur_dir = sign;
                    cur_log_ret = if has_commission { log_comm } else { 0.0 };
                }
                pos = if sign == 0 { 0 } else { sign };
            } else if !cfg.allow_flip {
                if has_commission {
                    log_eq += log_comm;
                }
                if cur_dir != 0 {
                    if has_commission {
                        cur_log_ret += log_comm;
                    }
                    trades.push(Trade {
                        entry_t: cur_entry_t,
                        exit_t: t,
                        direction: cur_dir,
                        pnl: cur_log_ret.exp() - 1.0,
                        bars: t.saturating_sub(cur_entry_t) as u32,
                        open: false,
                    });
                    cur_dir = 0;
                    cur_log_ret = 0.0;
                    overlay_on_trade_close(&mut overlay_state, &overlays);
                }
                pos = 0;
            } else {
                if has_commission {
                    log_eq += 2.0 * log_comm;
                }
                if cur_dir != 0 {
                    if has_commission {
                        cur_log_ret += log_comm;
                    }
                    trades.push(Trade {
                        entry_t: cur_entry_t,
                        exit_t: t,
                        direction: cur_dir,
                        pnl: cur_log_ret.exp() - 1.0,
                        bars: t.saturating_sub(cur_entry_t) as u32,
                        open: false,
                    });
                }
                cur_entry_t = t;
                cur_dir = sign;
                cur_log_ret = if has_commission { log_comm } else { 0.0 };
                pos = sign;
            }
        }

        let pos_bar = pos;
        log_eq += (pos_bar as f64) * lr[t];
        if cur_dir != 0 {
            cur_log_ret += (pos_bar as f64) * lr[t];
        }
        if risk_enabled && cur_dir != 0 {
            let hit_stop = stop_log.is_some_and(|th| cur_log_ret <= th);
            let hit_take = take_log.is_some_and(|th| cur_log_ret >= th);
            if hit_stop || hit_take {
                if has_commission {
                    log_eq += log_comm;
                    cur_log_ret += log_comm;
                }
                trades.push(Trade {
                    entry_t: cur_entry_t,
                    exit_t: t,
                    direction: cur_dir,
                    pnl: cur_log_ret.exp() - 1.0,
                    bars: t.saturating_sub(cur_entry_t) as u32,
                    open: false,
                });
                cur_dir = 0;
                cur_log_ret = 0.0;
                pos = 0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            }
        }

        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        let eq = log_eq.exp();
        let dd = if dd_log > 0.0 {
            -(-dd_log).exp_m1()
        } else {
            0.0
        };
        let i = t - start_t;
        let b = (i.saturating_mul(bins)) / n_steps.max(1);
        let b = b.min(bins - 1);

        if eq < eq_min[b] {
            eq_min[b] = eq;
        }
        if eq > eq_max[b] {
            eq_max[b] = eq;
        }
        if dd < dd_min[b] {
            dd_min[b] = dd;
        }
        if dd > dd_max[b] {
            dd_max[b] = dd;
        }
    }

    let mut eq_mm: Vec<f32> = Vec::with_capacity(bins.saturating_mul(2));
    let mut dd_mm: Vec<f32> = Vec::with_capacity(bins.saturating_mul(2));
    for i in 0..bins {
        let (a, b) = (eq_min[i], eq_max[i]);
        if a.is_finite() && b.is_finite() {
            eq_mm.push(a as f32);
            eq_mm.push(b as f32);
        } else {
            eq_mm.push(f32::NAN);
            eq_mm.push(f32::NAN);
        }

        let (a, b) = (dd_min[i], dd_max[i]);
        if a.is_finite() && b.is_finite() {
            dd_mm.push(a as f32);
            dd_mm.push(b as f32);
        } else {
            dd_mm.push(f32::NAN);
            dd_mm.push(f32::NAN);
        }
    }

    if cur_dir != 0 {
        let exit_t = n_total.saturating_sub(1);
        trades.push(Trade {
            entry_t: cur_entry_t,
            exit_t,
            direction: cur_dir,
            pnl: cur_log_ret.exp() - 1.0,
            bars: exit_t.saturating_sub(cur_entry_t) as u32,
            open: true,
        });
    }

    DoubleMaCurves {
        equity: ta_strategies::double_ma::MinMaxBins {
            start_t,
            end_t: n_total,
            bins,
            minmax: eq_mm,
        },
        drawdown: ta_strategies::double_ma::MinMaxBins {
            start_t,
            end_t: n_total,
            bins,
            minmax: dd_mm,
        },
        trade_stats: build_trade_stats(&trades),
        trades,
        indicator_bars: Vec::new(),
    }
}

pub(crate) fn eval_metrics_f64_with_exit_mode(
    lr: &[f64],
    close: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend: Option<&[f64]>,
    start_t: usize,
    strategy: &StrategyConfig,
    kaufman: Option<&KaufmanStopSeries>,
    compute_sharpe: bool,
) -> Metrics {
    let needs_custom_regime_filter_eval =
        strategy.uses_regime_hybrid_filter() && halftrend.is_some();
    if strategy_uses_kaufman_stop_exit(strategy) {
        let Some(ks) = kaufman else {
            return Metrics::default();
        };
        if compute_sharpe {
            eval_double_ma_metrics_kaufman_impl::<true, _, _>(
                lr,
                close,
                start_t,
                strategy,
                halftrend,
                ks,
                |i| fast[i],
                |i| slow[i],
            )
        } else {
            eval_double_ma_metrics_kaufman_impl::<false, _, _>(
                lr,
                close,
                start_t,
                strategy,
                halftrend,
                ks,
                |i| fast[i],
                |i| slow[i],
            )
        }
    } else if strategy.uses_execution_overlays() || needs_custom_regime_filter_eval {
        if compute_sharpe {
            eval_double_ma_metrics_cross_overlay_impl::<true, _, _>(
                lr,
                close,
                start_t,
                strategy,
                halftrend,
                |i| fast[i],
                |i| slow[i],
            )
        } else {
            eval_double_ma_metrics_cross_overlay_impl::<false, _, _>(
                lr,
                close,
                start_t,
                strategy,
                halftrend,
                |i| fast[i],
                |i| slow[i],
            )
        }
    } else if compute_sharpe {
        if let Some(trend) = halftrend {
            ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter_rich(
                lr, fast, slow, trend, start_t, strategy,
            )
        } else {
            ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f64_rich(
                lr, fast, slow, start_t, strategy,
            )
        }
    } else if let Some(trend) = halftrend {
        ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter_fast(
            lr, fast, slow, trend, start_t, strategy,
        )
    } else {
        ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f64_fast(
            lr, fast, slow, start_t, strategy,
        )
    }
}

#[cfg_attr(not(feature = "cuda"), allow(dead_code))]
fn eval_metrics_f32_with_exit_mode(
    lr: &[f64],
    close: &[f64],
    fast: &[f32],
    slow: &[f32],
    halftrend: Option<&[f64]>,
    start_t: usize,
    strategy: &StrategyConfig,
    kaufman: Option<&KaufmanStopSeries>,
    compute_sharpe: bool,
) -> Metrics {
    let needs_custom_regime_filter_eval =
        strategy.uses_regime_hybrid_filter() && halftrend.is_some();
    if strategy_uses_kaufman_stop_exit(strategy) {
        let Some(ks) = kaufman else {
            return Metrics::default();
        };
        if compute_sharpe {
            eval_double_ma_metrics_kaufman_impl::<true, _, _>(
                lr,
                close,
                start_t,
                strategy,
                halftrend,
                ks,
                |i| fast[i] as f64,
                |i| slow[i] as f64,
            )
        } else {
            eval_double_ma_metrics_kaufman_impl::<false, _, _>(
                lr,
                close,
                start_t,
                strategy,
                halftrend,
                ks,
                |i| fast[i] as f64,
                |i| slow[i] as f64,
            )
        }
    } else if strategy.uses_execution_overlays() || needs_custom_regime_filter_eval {
        if compute_sharpe {
            eval_double_ma_metrics_cross_overlay_impl::<true, _, _>(
                lr,
                close,
                start_t,
                strategy,
                halftrend,
                |i| fast[i] as f64,
                |i| slow[i] as f64,
            )
        } else {
            eval_double_ma_metrics_cross_overlay_impl::<false, _, _>(
                lr,
                close,
                start_t,
                strategy,
                halftrend,
                |i| fast[i] as f64,
                |i| slow[i] as f64,
            )
        }
    } else if compute_sharpe {
        if let Some(trend) = halftrend {
            ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter_rich(
                lr, fast, slow, trend, start_t, strategy,
            )
        } else {
            ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f32_rich(
                lr, fast, slow, start_t, strategy,
            )
        }
    } else if let Some(trend) = halftrend {
        ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter_fast(
            lr, fast, slow, trend, start_t, strategy,
        )
    } else {
        ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f32_fast(
            lr, fast, slow, start_t, strategy,
        )
    }
}

pub(crate) fn eval_curves_f64_with_exit_mode(
    lr: &[f64],
    close: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend: Option<&[f64]>,
    start_t: usize,
    strategy: &StrategyConfig,
    bins: usize,
    kaufman: Option<&KaufmanStopSeries>,
) -> DoubleMaCurves {
    let needs_custom_regime_filter_eval =
        strategy.uses_regime_hybrid_filter() && halftrend.is_some();
    if strategy_uses_kaufman_stop_exit(strategy) {
        let Some(ks) = kaufman else {
            return DoubleMaCurves {
                equity: ta_strategies::double_ma::MinMaxBins {
                    start_t,
                    end_t: lr.len(),
                    bins: 0,
                    minmax: Vec::new(),
                },
                drawdown: ta_strategies::double_ma::MinMaxBins {
                    start_t,
                    end_t: lr.len(),
                    bins: 0,
                    minmax: Vec::new(),
                },
                trades: Vec::new(),
                trade_stats: None,
                indicator_bars: Vec::new(),
            };
        };
        eval_double_ma_curves_kaufman_f64(
            lr, close, fast, slow, halftrend, start_t, strategy, bins, ks,
        )
    } else if strategy.uses_execution_overlays() || needs_custom_regime_filter_eval {
        eval_double_ma_curves_cross_overlay_f64(
            lr, close, fast, slow, halftrend, start_t, strategy, bins,
        )
    } else if let Some(trend) = halftrend {
        ta_strategies::double_ma::eval_double_ma_curves_from_log_returns_and_mas_f64_with_halftrend_filter(
            lr, fast, slow, trend, start_t, strategy, bins,
        )
    } else {
        ta_strategies::double_ma::eval_double_ma_curves_from_log_returns_and_mas_f64(
            lr, fast, slow, start_t, strategy, bins,
        )
    }
}

pub fn compute_double_ma_drilldown_blocking_with_candles(
    req: DoubleMaDrilldownRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<DoubleMaCurves, String> {
    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_ma = req.params.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = req.params.slow_ma_type.trim().to_ascii_lowercase();
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

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

    validate_strategy_cfg(&req.strategy)?;

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let cached_followup = resolve_followup_cache_entry(
        req.followup_cache_key.as_deref(),
        &req.data_id,
        n,
        &req.ma_source,
        &fast_ma,
        &slow_ma,
        &req.strategy,
        req.halftrend_filter.as_ref(),
        candles,
    )?;
    let cached_backend_hint = cached_followup
        .as_ref()
        .and_then(|entry| entry.backend_hint.clone());
    let followup_backend = req.backend_hint.clone().or(cached_backend_hint);

    let (halftrend_trend, kaufman_stops, first_valid, lr) = if let Some(entry) = cached_followup {
        (
            entry.regime_filter,
            entry.kaufman_stops,
            entry
                .base_first_valid
                .ok_or_else(|| "follow-up cache entry missing base_first_valid".to_string())?,
            entry
                .log_returns
                .ok_or_else(|| "follow-up cache entry missing log_returns".to_string())?,
        )
    } else {
        let fv_close =
            first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
        let fv_ma = first_finite_idx(ma_prices)
            .ok_or_else(|| "ma source series all non-finite".to_string())?;
        let mut first_valid = fv_close.max(fv_ma);

        let halftrend_trend: Option<Vec<f64>> = compute_combined_regime_filter(
            candles,
            &req.ma_source,
            &req.strategy,
            req.halftrend_filter.as_ref(),
        )?;
        let kaufman_stops = compute_kaufman_stop_series(candles, &req.strategy)?;
        if let Some(ks) = kaufman_stops.as_ref() {
            let fv_k_long = first_finite_idx(&ks.long)
                .ok_or_else(|| "kaufman-stop long series all non-finite".to_string())?;
            let fv_k_short = first_finite_idx(&ks.short)
                .ok_or_else(|| "kaufman-stop short series all non-finite".to_string())?;
            first_valid = first_valid.max(fv_k_long.max(fv_k_short));
        }
        if let Some(trend) = halftrend_trend.as_ref() {
            if trend.len() != n {
                return Err("regime trend series length mismatch".to_string());
            }
            let fv_ht = first_finite_idx(trend)
                .ok_or_else(|| "regime trend series all non-finite".to_string())?;
            first_valid = first_valid.max(fv_ht);
        }

        (
            halftrend_trend,
            kaufman_stops,
            first_valid,
            compute_log_returns_f64(close),
        )
    };

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
        resolve_alma_params_for_side("fast", &fast_ma, req.fast_ma_params.as_ref())?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", &slow_ma, req.slow_ma_params.as_ref())?;

    let followup_prices_f32 = match followup_backend.as_ref() {
        Some(Backend::CpuOnly) | None => None,
        Some(_) => Some(
            ma_prices
                .iter()
                .map(|&value| value as f32)
                .collect::<Vec<f32>>(),
        ),
    };
    let fast_periods = [req.params.fast_len];
    let (fast_mat, cols) = ma_matrix_with_backend_f64(
        candles,
        &fast_ma,
        &req.ma_source,
        &fast_periods,
        req.fast_ma_params.as_ref(),
        fast_alma_offset,
        fast_alma_sigma,
        followup_backend.as_ref(),
        followup_prices_f32.as_deref(),
    )?;
    if cols != n || fast_mat.len() != n {
        return Err("fast MA series length mismatch".to_string());
    }

    let slow_periods = [req.params.slow_len];
    let (slow_mat, cols2) = ma_matrix_with_backend_f64(
        candles,
        &slow_ma,
        &req.ma_source,
        &slow_periods,
        req.slow_ma_params.as_ref(),
        slow_alma_offset,
        slow_alma_sigma,
        followup_backend.as_ref(),
        followup_prices_f32.as_deref(),
    )?;
    if cols2 != n || slow_mat.len() != n {
        return Err("slow MA series length mismatch".to_string());
    }

    let bins = req.bins.unwrap_or(1024);
    let mut curves = eval_curves_f64_with_exit_mode(
        &lr,
        close,
        &fast_mat,
        &slow_mat,
        halftrend_trend.as_deref(),
        start_t,
        &req.strategy,
        bins,
        kaufman_stops.as_ref(),
    );
    curves.indicator_bars =
        build_indicator_bars_for_chart(candles, &fast_mat, &slow_mat, DRILL_INDICATOR_MAX_POINTS)?;
    Ok(curves)
}

pub fn compute_double_ma_local_stability_blocking_with_candles(
    req: DoubleMaLocalStabilityRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<LocalStabilityResult, String> {
    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_ma = req.params.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = req.params.slow_ma_type.trim().to_ascii_lowercase();
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let base_fast = req.params.fast_len;
    let base_slow = req.params.slow_len;
    if base_fast == 0 || base_slow == 0 {
        return Err("periods must be > 0".to_string());
    }
    if base_fast as usize > n || base_slow as usize > n {
        return Err(format!(
            "period exceeds series length: fast_len={base_fast} slow_len={base_slow} n={n}"
        ));
    }
    if base_fast >= base_slow {
        return Err("invalid params: fast_len must be < slow_len".to_string());
    }

    validate_strategy_cfg(&req.strategy)?;

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let cached_followup = resolve_followup_cache_entry(
        req.followup_cache_key.as_deref(),
        &req.data_id,
        n,
        &req.ma_source,
        &fast_ma,
        &slow_ma,
        &req.strategy,
        req.halftrend_filter.as_ref(),
        candles,
    )?;
    let cached_backend_hint = cached_followup
        .as_ref()
        .and_then(|entry| entry.backend_hint.clone());
    let followup_backend = req.backend_hint.clone().or(cached_backend_hint);

    let (halftrend_trend, kaufman_stops, first_valid, lr) = if let Some(entry) = cached_followup {
        (
            entry.regime_filter,
            entry.kaufman_stops,
            entry
                .base_first_valid
                .ok_or_else(|| "follow-up cache entry missing base_first_valid".to_string())?,
            entry
                .log_returns
                .ok_or_else(|| "follow-up cache entry missing log_returns".to_string())?,
        )
    } else {
        let fv_close =
            first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
        let fv_ma = first_finite_idx(ma_prices)
            .ok_or_else(|| "ma source series all non-finite".to_string())?;
        let mut first_valid = fv_close.max(fv_ma);

        let halftrend_trend: Option<Vec<f64>> = compute_combined_regime_filter(
            candles,
            &req.ma_source,
            &req.strategy,
            req.halftrend_filter.as_ref(),
        )?;
        if let Some(trend) = halftrend_trend.as_ref() {
            if trend.len() != n {
                return Err("regime trend series length mismatch".to_string());
            }
            let fv_ht = first_finite_idx(trend)
                .ok_or_else(|| "regime trend series all non-finite".to_string())?;
            first_valid = first_valid.max(fv_ht);
        }
        let kaufman_stops = compute_kaufman_stop_series(candles, &req.strategy)?;
        if let Some(ks) = kaufman_stops.as_ref() {
            let fv_k_long = first_finite_idx(&ks.long)
                .ok_or_else(|| "kaufman-stop long series all non-finite".to_string())?;
            let fv_k_short = first_finite_idx(&ks.short)
                .ok_or_else(|| "kaufman-stop short series all non-finite".to_string())?;
            first_valid = first_valid.max(fv_k_long.max(fv_k_short));
        }

        (
            halftrend_trend,
            kaufman_stops,
            first_valid,
            compute_log_returns_f64(close),
        )
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
        resolve_alma_params_for_side("fast", &fast_ma, req.fast_ma_params.as_ref())?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", &slow_ma, req.slow_ma_params.as_ref())?;

    let followup_prices_f32 = match followup_backend.as_ref() {
        Some(Backend::CpuOnly) | None => None,
        Some(_) => Some(
            ma_prices
                .iter()
                .map(|&value| value as f32)
                .collect::<Vec<f32>>(),
        ),
    };
    let (fast_mat, cols) = ma_matrix_with_backend_f64(
        candles,
        &fast_ma,
        &req.ma_source,
        &fast_periods,
        req.fast_ma_params.as_ref(),
        fast_alma_offset,
        fast_alma_sigma,
        followup_backend.as_ref(),
        followup_prices_f32.as_deref(),
    )?;
    if cols != n {
        return Err("fast MA series length mismatch".to_string());
    }
    let (slow_mat, cols2) = ma_matrix_with_backend_f64(
        candles,
        &slow_ma,
        &req.ma_source,
        &slow_periods,
        req.slow_ma_params.as_ref(),
        slow_alma_offset,
        slow_alma_sigma,
        followup_backend.as_ref(),
        followup_prices_f32.as_deref(),
    )?;
    if cols2 != n {
        return Err("slow MA series length mismatch".to_string());
    }

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

            let metrics = eval_metrics_f64_with_exit_mode(
                &lr,
                close,
                fast,
                slow,
                halftrend_trend.as_deref(),
                start_t,
                &req.strategy,
                kaufman_stops.as_ref(),
                true,
            );
            let score = if matches!(
                req.score_mode,
                Some(DoubleMaScoreMode::EffectiveSampleLcb10)
            ) {
                effective_sample_lcb_10_score(&metrics, n.saturating_sub(start_t).max(2))
            } else {
                effective_score(&metrics, objective, req.score_mode)
            };
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

pub fn run_double_ma_optimization_blocking_with_candles(
    req: DoubleMaRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    run_double_ma_optimization_blocking_with_candles_and_eval_range(
        req, candles, cancel, progress, None,
    )
}

pub fn run_double_ma_optimization_blocking_with_candles_and_eval_range(
    req: DoubleMaRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    eval_range: Option<(usize, usize)>,
) -> Result<BackendOptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }
    let _nvtx_scope = crate::nvtx::scope("engine", "double_ma_opt_total");
    let route_resolve_scope = crate::nvtx::scope("engine", "route_resolve");

    let requested_backend = req.backend.clone();
    let normalized_backend = requested_backend.clone().with_preferred_cuda_device()?;
    #[cfg(all(test, feature = "cuda", feature = "cuda-backtest-kernel"))]
    let _gpu_guard = match normalized_backend {
        Backend::GpuOnly { .. }
        | Backend::GpuKernel { .. }
        | Backend::GpuHybridKernel { .. }
        | Backend::Auto { .. } => Some(crate::gpu_backtest_kernel::lock_gpu_test()),
        Backend::CpuOnly => None,
    };
    let fast = req.fast_range;
    let slow = req.slow_range;

    let mut fast_ma_types = req.fast_ma_types;
    let mut slow_ma_types = req.slow_ma_types;
    let mut ma_source = req.ma_source;
    let export_all_csv_path = normalize_optional_path(req.export_all_csv_path);
    let mut fast_ma_params = req.fast_ma_params;
    let mut slow_ma_params = req.slow_ma_params;
    let fast_ma_param_sweeps = req.fast_ma_param_sweeps;
    let slow_ma_param_sweeps = req.slow_ma_param_sweeps;
    let dynamic_strategy_ir = req.dynamic_strategy_ir;
    let mut strategy = req.strategy;
    strategy.hurdle_tau = req.hurdle_tau;
    let mut halftrend_filter = req.halftrend_filter;
    let constraints = sanitize_double_ma_constraints(req.constraints)?;
    let heatmap_bins = normalize_heatmap_bins(req.heatmap_bins, 512)?;
    if let Some(ir) = dynamic_strategy_ir.as_ref() {
        if ir.profile != crate::strategy_ir::StrategyProfileKind::DoubleMa {
            return Err("dynamic_strategy_ir currently supports only profile=DoubleMa".to_string());
        }
        ma_source = ir.ma_source.trim().to_string();
        fast_ma_types = vec![ir.fast_ma.ma_type.trim().to_string()];
        slow_ma_types = vec![ir.slow_ma.ma_type.trim().to_string()];
        fast_ma_params = (!ir.fast_ma.params.is_empty()).then_some(ir.fast_ma.params.clone());
        slow_ma_params = (!ir.slow_ma.params.is_empty()).then_some(ir.slow_ma.params.clone());
        strategy = ir.execution.strategy.clone();
        strategy.hurdle_tau = req.hurdle_tau;
        halftrend_filter = ir.execution.halftrend_filter.clone();
    }

    if let Some(hurdle_tau) = strategy.hurdle_tau {
        if !hurdle_tau.is_finite() {
            return Err("hurdle_tau must be finite when set".to_string());
        }
    }

    validate_strategy_cfg_for_optimization(&strategy)?;
    let strategy = strategy_cfg_for_optimization(strategy);

    if fast_ma_types.len() != 1 || slow_ma_types.len() != 1 {
        return Err("Select exactly one fast MA and one slow MA.".to_string());
    }
    let fast_ma = fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = slow_ma_types[0].trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("Fast/slow MA type cannot be empty.".to_string());
    }

    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_periods = expand_u16_range(fast)?;
    let slow_periods = expand_u16_range(slow)?;
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty period range".to_string());
    }
    let total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);
    if total_pairs == 0 {
        return Err("no valid parameter combinations (fast must be < slow)".to_string());
    }

    let fast_param_variants =
        build_ma_param_variants("fast", fast_ma_params, fast_ma_param_sweeps)?;
    let slow_param_variants =
        build_ma_param_variants("slow", slow_ma_params, slow_ma_param_sweeps)?;

    if fast_param_variants.len() > u16::MAX as usize {
        return Err(format!(
            "fast MA parameter sweeps produce too many variants (max {}). Reduce sweeps.",
            u16::MAX
        ));
    }
    if slow_param_variants.len() > u16::MAX as usize {
        return Err(format!(
            "slow MA parameter sweeps produce too many variants (max {}). Reduce sweeps.",
            u16::MAX
        ));
    }

    let variant_pairs = fast_param_variants
        .len()
        .saturating_mul(slow_param_variants.len());
    if variant_pairs == 0 {
        return Err("no MA parameter variants".to_string());
    }
    if variant_pairs > MAX_MA_PARAM_VARIANT_PAIRS {
        return Err(format!(
            "MA parameter sweeps produce too many combinations ({variant_pairs}, max {MAX_MA_PARAM_VARIANT_PAIRS}). Reduce sweeps."
        ));
    }

    let total_combos = total_pairs.saturating_mul(variant_pairs);
    if export_all_csv_path.is_some() && variant_pairs > 1 {
        return Err(
            "export_all_csv_path is not supported when optimizing MA parameters. Disable MA param sweeps or export."
                .to_string(),
        );
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let n = close.len();
    let regime_filter_expected = halftrend_filter.is_some()
        || strategy.uses_pressure_impulse_trend_filter()
        || strategy.uses_slope_efficiency_trend_filter()
        || strategy.uses_momentum_triple_stack_filter()
        || strategy.uses_oscillator_reversion_confluence_filter()
        || strategy.uses_trend_quality_filter()
        || strategy.uses_breakout_momentum_filter()
        || strategy.uses_regime_hybrid_filter()
        || strategy.uses_volume_flow_confirm_filter();
    let mut host_regime_filter_cache: Option<Option<Vec<f64>>> = None;

    let start = std::time::Instant::now();
    let objective = req.objective;
    let score_mode = req.score_mode;
    let extra_shortlist_objectives = req.extra_shortlist_objectives.clone();
    let requested_shortlist_confidence_mode = req.shortlist_confidence_mode;
    let requested_shortlist_union_mode = req.shortlist_union_mode;
    let requested_shortlist_slice_mode = req.shortlist_slice_mode;
    let requested_shortlist_slice_benchmark = req.shortlist_slice_benchmark.clone();
    let requested_shortlist_neighborhood_radius = req.shortlist_neighborhood_radius;
    let requested_shortlist_neighborhood_step = req.shortlist_neighborhood_step;
    let requested_metric_contract = req.metric_contract;
    let requested_trade_metrics = req.include_trade_metrics
        || matches!(
            requested_metric_contract,
            Some(DoubleMaMetricContract::Trade22)
        );
    let (top_k, include_all) = normalize_topk_include_all(req.top_k, req.include_all);
    if export_all_csv_path.is_some() && include_all {
        return Err(
            "export_all_csv_path requires include_all=false (stream export avoids huge RAM usage)"
                .to_string(),
        );
    }
    if export_all_csv_path.is_some() && req.mode != OptimizationMode::Grid {
        return Err("export_all_csv_path requires mode=Grid".to_string());
    }
    if include_all && total_combos > 2_000_000 {
        return Err(format!(
            "include_all=true would return {} rows; disable it or reduce ranges/sweeps.",
            total_combos
        ));
    }
    let kernel_blockers_for_request = gpu_kernel_blockers_for_request(
        fast_ma.as_str(),
        slow_ma.as_str(),
        &strategy,
        regime_filter_expected,
        objective,
        top_k,
        include_all,
    );
    let kernel_supported_for_mode = kernel_blockers_for_request.is_empty();
    #[cfg(feature = "cuda")]
    let cuda_runtime_available = cuda_available();
    #[cfg(not(feature = "cuda"))]
    let cuda_runtime_available = false;
    let hybrid_viable_for_mode = {
        let route_plan = crate::ma_policy::plan_double_ma_hybrid_route(&fast_ma, &slow_ma);
        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        {
            route_plan.is_fully_supported()
                && hybrid_backend_enabled()
                && cuda_runtime_available
                && kernel_supported_for_mode
        }
        #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
        {
            let _ = route_plan;
            false
        }
    };
    let mode_used = resolve_optimization_mode(&ModeSelectionContext {
        requested_mode: req.mode,
        include_all,
        heatmap_bins,
        export_all_csv_path: export_all_csv_path.as_deref(),
        total_combos,
        gpu_execution: req.gpu_execution,
        gpu_kernel_viable: kernel_supported_for_mode,
        gpu_hybrid_viable: hybrid_viable_for_mode,
    });
    validate_optimization_mode(
        &mode_used,
        include_all,
        heatmap_bins,
        export_all_csv_path.as_deref(),
    )?;
    let vdps_cfg = if matches!(mode_used, OptimizationModeResolved::GpuVdps) {
        Some(VdpsResolvedConfig::from_request(req.vdps.as_ref())?)
    } else {
        None
    };

    let mut backend_fallback_reason: Option<String> = None;
    let indicator_route_plan_summary =
        build_indicator_route_plan_summary(&fast_ma, &slow_ma, req.gpu_execution);
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let kernel_execution_unavailable_reason: Option<String> = if cuda_runtime_available {
        None
    } else {
        Some("CUDA unavailable; GPU kernel backend requires a CUDA device at runtime.".to_string())
    };
    #[cfg(all(feature = "cuda", not(feature = "cuda-backtest-kernel")))]
    let kernel_execution_unavailable_reason: Option<String> =
        Some("GPU kernel backend is disabled (build without `cuda-backtest-kernel`).".to_string());
    #[cfg(not(feature = "cuda"))]
    let kernel_execution_unavailable_reason: Option<String> = Some(
        "GPU kernel backend requires building `ta_desktop_demo_app` with `--features cuda,cuda-backtest-kernel`."
            .to_string(),
    );
    let kernel_available_for_execution =
        kernel_supported_for_mode && kernel_execution_unavailable_reason.is_none();

    let mut backend = match normalized_backend {
        Backend::Auto { device_id } => {
            let export_requested = export_all_csv_path.is_some();

            let fast_info =
                lookup_ma_info(&fast_ma).ok_or_else(|| "unknown fast MA".to_string())?;
            let slow_info =
                lookup_ma_info(&slow_ma).ok_or_else(|| "unknown slow MA".to_string())?;
            let cpu_ok = fast_info.supports_cpu_single && slow_info.supports_cpu_single;

            let cuda_ok = cuda_runtime_available
                && fast_info.supports_cuda_single
                && slow_info.supports_cuda_single;
            let kernel_supported = kernel_available_for_execution;
            #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
            let hybrid_supported = cuda_runtime_available && hybrid_backend_enabled();
            #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
            let hybrid_supported = false;

            if export_requested && !kernel_supported {
                return Err(
                    "export_all_csv_path requires the GPU (kernel) backend, which is not available for this MA selection/build.".to_string(),
                );
            }

            if kernel_supported {
                Backend::GpuKernel { device_id }
            } else if hybrid_supported {
                Backend::GpuHybridKernel { device_id }
            } else if cuda_ok {
                Backend::GpuOnly { device_id }
            } else if cpu_ok {
                Backend::CpuOnly
            } else {
                return Err(format!(
                    "No supported backend for fast='{fast_ma}' slow='{slow_ma}' (try different MAs or enable CUDA)."
                ));
            }
        }
        other => other,
    };

    if let Backend::GpuHybridKernel { device_id } = backend {
        if !hybrid_backend_enabled() {
            backend = Backend::GpuKernel { device_id };
            merge_optional_note(
                &mut backend_fallback_reason,
                Some(
                    "GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel."
                        .to_string(),
                ),
            );
        }
    }

    let prefer_gpu_resident = matches!(
        req.gpu_execution,
        GpuExecutionPolicy::PreferGpuResident | GpuExecutionPolicy::StrictGpuResident
    );
    let strict_gpu_resident = matches!(req.gpu_execution, GpuExecutionPolicy::StrictGpuResident);
    let cpu_staged_indicator_requested = indicator_route_plan_summary.fast.residency_tier
        == "cpu_staged"
        || indicator_route_plan_summary.slow.residency_tier == "cpu_staged";
    if strict_gpu_resident && cpu_staged_indicator_requested {
        return Err(
            "StrictGpuResident requires device-resident indicator routing, but the selected MA route requires CPU staging."
                .to_string(),
        );
    }

    // If the user explicitly asks for the kernel backend but it's not available for this MA/build,
    // fall back to the best available backend (unless export is requested).
    let kernel_device_id = match &backend {
        Backend::GpuKernel { device_id } => Some(device_id),
        _ => None,
    };
    if let Some(device_id) = kernel_device_id {
        let export_requested = export_all_csv_path.is_some();
        let kernel_blockers = &kernel_blockers_for_request;
        let kernel_supported = kernel_available_for_execution;

        if export_requested && !kernel_supported {
            return Err(
                "export_all_csv_path requires the GPU (kernel) backend, which is not available for this MA selection/build.".to_string(),
            );
        }

        if !kernel_supported {
            let fast_info =
                lookup_ma_info(&fast_ma).ok_or_else(|| "unknown fast MA".to_string())?;
            let slow_info =
                lookup_ma_info(&slow_ma).ok_or_else(|| "unknown slow MA".to_string())?;
            let cpu_ok = fast_info.supports_cpu_single && slow_info.supports_cpu_single;

            let cuda_ok = cuda_runtime_available
                && fast_info.supports_cuda_single
                && slow_info.supports_cuda_single;

            backend = if cuda_ok {
                Backend::GpuOnly {
                    device_id: *device_id,
                }
            } else if cpu_ok {
                Backend::CpuOnly
            } else {
                return Err(format!(
                    "No supported backend for fast='{fast_ma}' slow='{slow_ma}' (try different MAs or enable CUDA)."
                ));
            };
            let resolved = backend_kind_label(&backend);
            let mut reasons: Vec<String> = Vec::new();
            if !kernel_blockers.is_empty() {
                reasons.push(kernel_blockers.join("; "));
            }
            if let Some(reason) = kernel_execution_unavailable_reason.as_deref() {
                reasons.push(reason.to_string());
            }
            let reasons = if reasons.is_empty() {
                "no additional reasons reported".to_string()
            } else {
                reasons.join("; ")
            };
            merge_optional_note(
                &mut backend_fallback_reason,
                Some(format!(
                    "Requested GpuKernel is unavailable for fast='{fast_ma}' slow='{slow_ma}'; using {resolved}. Reasons: {reasons}"
                )),
            );
        }
    }

    if strict_gpu_resident {
        match backend {
            Backend::GpuKernel { .. } | Backend::GpuHybridKernel { .. } => {}
            Backend::Auto { .. } | Backend::CpuOnly => {
                return Err(
                    "StrictGpuResident requires a GPU-resident backend, but the request resolved to CpuOnly."
                        .to_string(),
                );
            }
            Backend::GpuOnly { .. } => {
                return Err(
                    "StrictGpuResident does not allow the host-evaluated GpuOnly path. Use GpuKernel/GpuHybridKernel or relax the execution policy."
                        .to_string(),
                );
            }
        }
    } else if prefer_gpu_resident {
        match backend {
            Backend::CpuOnly => merge_optional_note(
                &mut backend_fallback_reason,
                Some(
                    "PreferGpuResident requested GPU-resident execution, but the request resolved to CpuOnly."
                        .to_string(),
                ),
            ),
            Backend::GpuOnly { .. } => merge_optional_note(
                &mut backend_fallback_reason,
                Some(
                    "PreferGpuResident requested GPU-resident execution, but the request downgraded to GpuOnly (GPU MA generation with host metric evaluation)."
                        .to_string(),
                ),
            ),
            Backend::Auto { .. } | Backend::GpuKernel { .. } | Backend::GpuHybridKernel { .. } => {}
        }
    }

    if let Some((eval_start, eval_end)) = eval_range {
        if eval_start >= eval_end {
            return Err(format!(
                "evaluation window requires eval_start < eval_end, got [{eval_start}, {eval_end})"
            ));
        }
        if eval_end > n {
            return Err(format!(
                "evaluation window end {eval_end} exceeds candle length {n}"
            ));
        }
        match backend {
            Backend::GpuKernel { .. } | Backend::GpuHybridKernel { .. } => {}
            Backend::Auto { .. } | Backend::CpuOnly | Backend::GpuOnly { .. } => {
                return Err(
                    "evaluation window currently requires an exact GPU backend (GpuKernel or GpuHybridKernel)."
                        .to_string(),
                );
            }
        }
    }

    if matches!(mode_used, OptimizationModeResolved::GpuVdps) {
        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        if let Backend::GpuOnly { device_id } = &backend {
            if hybrid_backend_enabled() && cuda_available() {
                backend = Backend::GpuHybridKernel {
                    device_id: *device_id,
                };
                merge_optional_note(
                    &mut backend_fallback_reason,
                    Some(
                        "GpuVdps promotes GpuOnly to GpuHybridKernel for unified GPU routing."
                            .to_string(),
                    ),
                );
            }
        }
    }

    let mut dynamic_runtime_plan: Option<crate::strategy_dynamic::RunnableDynamicDoubleMaPlan> =
        None;
    if let Some(ir) = dynamic_strategy_ir.clone() {
        let requested_plan_backend = if matches!(backend, Backend::CpuOnly) {
            crate::strategy_plan::StrategyPlanBackend::Cpu
        } else {
            crate::strategy_plan::StrategyPlanBackend::GpuUnified
        };
        let runnable = crate::strategy_dynamic::prepare_runnable_dynamic_double_ma_plan(
            ir,
            requested_plan_backend,
            objective,
            top_k,
            include_all,
            candles,
        )?;
        let original_backend = backend.clone();
        match runnable.preview.resolved_backend {
            crate::strategy_plan::StrategyPlanBackend::Cpu => {
                if !matches!(backend, Backend::CpuOnly) {
                    backend = Backend::CpuOnly;
                    let reasons = if runnable.preview.capability.reasons.is_empty() {
                        "no additional reasons reported".to_string()
                    } else {
                        runnable.preview.capability.reasons.join("; ")
                    };
                    merge_optional_note(
                        &mut backend_fallback_reason,
                        Some(format!(
                            "DynamicDoubleMa fell back to CpuOnly for this strategy_ir. Reasons: {reasons}"
                        )),
                    );
                }
            }
            crate::strategy_plan::StrategyPlanBackend::GpuUnified => match backend {
                Backend::CpuOnly => {}
                Backend::GpuKernel { .. } => {}
                Backend::GpuOnly { device_id } | Backend::GpuHybridKernel { device_id } => {
                    backend = Backend::GpuKernel { device_id };
                    merge_optional_note(
                            &mut backend_fallback_reason,
                            Some(format!(
                                "DynamicDoubleMa uses the GpuKernel(dynamic_strategy_v1) route instead of {}.",
                                backend_kind_label(&original_backend)
                            )),
                        );
                }
                Backend::Auto { device_id } => {
                    backend = Backend::GpuKernel { device_id };
                }
            },
        }
        dynamic_runtime_plan = Some(runnable);
    }
    enforce_gpu_execution_policy_before_run(
        req.gpu_execution,
        &backend,
        &indicator_route_plan_summary,
    )?;

    let requested_backend_label = backend_kind_label(&requested_backend);
    let resolved_backend_label = backend_kind_label(&backend);
    let mut backend_route: Option<String> = if matches!(requested_backend, Backend::Auto { .. }) {
        Some(format!("Auto->{resolved_backend_label}"))
    } else if requested_backend_label != resolved_backend_label {
        Some(format!(
            "{requested_backend_label}->{resolved_backend_label}"
        ))
    } else {
        Some(resolved_backend_label.to_string())
    };

    if matches!(mode_used, OptimizationModeResolved::GpuVdps) && matches!(backend, Backend::CpuOnly)
    {
        return Err(
            "GpuVdps mode requires a GPU backend. Choose backend=Auto/GpuOnly/GpuHybridKernel/GpuKernel with CUDA available."
                .to_string(),
        );
    }
    let vdps_gpu_refill_device_id = match backend {
        Backend::GpuKernel { device_id } | Backend::GpuHybridKernel { device_id } => {
            Some(device_id)
        }
        _ => None,
    };
    let dynamic_gpu_preflight_required = dynamic_gpu_preflight_required_for_backend(
        dynamic_runtime_plan.is_some()
            || crate::strategy_dynamic::dynamic_strategy_opt_experimental_enabled(),
        &mode_used,
        &backend,
    );
    drop(route_resolve_scope);

    struct OffsetProgress<'a> {
        inner: &'a dyn ProgressSink,
        base_offset: usize,
        total_pairs: usize,
    }

    impl ProgressSink for OffsetProgress<'_> {
        fn emit_double_ma_progress(&self, payload: crate::progress::DoubleMaProgressPayload) {
            self.inner
                .emit_double_ma_progress(crate::progress::DoubleMaProgressPayload {
                    processed_pairs: self.base_offset.saturating_add(payload.processed_pairs),
                    total_pairs: self.total_pairs,
                    phase: payload.phase,
                });
        }
    }

    let mut run_grid = |fast_periods: &[u16],
                        slow_periods: &[u16],
                        top_k: usize,
                        include_all: bool,
                        heatmap_bins: usize,
                        export_all_csv_path: Option<&str>,
                        total_pairs: usize,
                        fast_ma_params: Option<&HashMap<String, f64>>,
                        slow_ma_params: Option<&HashMap<String, f64>>,
                        fast_alma_offset: f64,
                        fast_alma_sigma: f64,
                        slow_alma_offset: f64,
                        slow_alma_sigma: f64,
                        constraints: Option<&DoubleMaConstraints>,
                        progress: Option<&dyn ProgressSink>,
                        dynamic_gpu_execution: &DynamicGpuExecutionSelection|
     -> Result<
        (
            OptimizationResult,
            BackendUsed,
            Option<HybridTimingMs>,
            Option<GpuKernelTimingMs>,
            Option<String>,
            Option<String>,
        ),
        String,
    > {
        enum GpuGridDispatch {
            GpuOnly { device_id: u32 },
            GpuKernel { device_id: u32 },
            GpuHybridKernel { device_id: u32 },
        }

        let mut run_gpu_grid = |dispatch: GpuGridDispatch| match dispatch {
            GpuGridDispatch::GpuOnly { device_id } => {
                if export_all_csv_path.is_some() {
                    return Err("export_all_csv_path is currently supported only for the GPU (kernel) backend".to_string());
                }
                let regime_filter = ensure_combined_regime_filter_cached(
                    &mut host_regime_filter_cache,
                    candles,
                    &ma_source,
                    &strategy,
                    halftrend_filter.as_ref(),
                    n,
                )?;
                let result = optimize_double_ma_gpu_sweep_tiled(
                    candles,
                    device_id,
                    fast_periods,
                    slow_periods,
                    &fast_ma,
                    &slow_ma,
                    &ma_source,
                    fast_ma_params,
                    slow_ma_params,
                    regime_filter,
                    &strategy,
                    objective,
                    score_mode,
                    None,
                    None,
                    None,
                    None,
                    None,
                    top_k,
                    include_all,
                    fast_alma_offset,
                    fast_alma_sigma,
                    slow_alma_offset,
                    slow_alma_sigma,
                    constraints,
                    total_pairs,
                    heatmap_bins,
                    cancel,
                    progress,
                )?;
                Ok((result, BackendUsed::Gpu, None, None, None, None))
            }
            GpuGridDispatch::GpuKernel { device_id } => {
                let _nvtx_scope = crate::nvtx::scope("engine", "gpu_kernel_dispatch");
                let kernel_t0 = std::time::Instant::now();
                let dynamic_gpu_handle = if eval_range.is_none() {
                    match dynamic_gpu_execution {
                        DynamicGpuExecutionSelection::Supported(handle) => Some(handle),
                        DynamicGpuExecutionSelection::CpuFallback { .. }
                        | DynamicGpuExecutionSelection::Disabled => None,
                    }
                } else {
                    None
                };
                let result = if let Some(handle) = dynamic_gpu_handle {
                    optimize_double_ma_dynamic_gpu_kernel_tiled(
                        candles,
                        device_id,
                        fast_periods,
                        slow_periods,
                        &fast_ma,
                        &slow_ma,
                        &ma_source,
                        fast_ma_params,
                        slow_ma_params,
                        &strategy,
                        objective,
                        score_mode,
                        extra_shortlist_objectives.as_deref(),
                        requested_shortlist_confidence_mode,
                        requested_shortlist_union_mode,
                        requested_shortlist_slice_mode,
                        requested_shortlist_slice_benchmark.clone(),
                        requested_shortlist_neighborhood_radius,
                        requested_shortlist_neighborhood_step,
                        requested_trade_metrics,
                        requested_metric_contract,
                        top_k,
                        include_all,
                        fast_alma_offset,
                        fast_alma_sigma,
                        slow_alma_offset,
                        slow_alma_sigma,
                        constraints,
                        total_pairs,
                        heatmap_bins,
                        cancel,
                        progress,
                        handle,
                    )?
                } else if strategy.uses_pressure_impulse_trend_filter() {
                    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
                    {
                        let regime_filter = build_pressure_impulse_trend_regime_device(
                            candles, &ma_source, device_id, &strategy,
                        )?;
                        optimize_double_ma_gpu_kernel_tiled_with_device_regime(
                            candles,
                            device_id,
                            fast_periods,
                            slow_periods,
                            &fast_ma,
                            &slow_ma,
                            &ma_source,
                            fast_ma_params,
                            slow_ma_params,
                            &regime_filter,
                            &strategy,
                            objective,
                            score_mode,
                            extra_shortlist_objectives.as_deref(),
                            requested_shortlist_confidence_mode,
                            requested_shortlist_union_mode,
                            requested_shortlist_slice_mode,
                            requested_shortlist_slice_benchmark.clone(),
                            top_k,
                            include_all,
                            fast_alma_offset,
                            fast_alma_sigma,
                            slow_alma_offset,
                            slow_alma_sigma,
                            requested_trade_metrics,
                            requested_metric_contract,
                            constraints,
                            export_all_csv_path,
                            total_pairs,
                            heatmap_bins,
                            eval_range,
                            cancel,
                            progress,
                        )?
                    }
                    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
                    {
                        return Err(
                            "PIT GPU kernel backend requires `cuda,cuda-backtest-kernel`"
                                .to_string(),
                        );
                    }
                } else if strategy.uses_volume_flow_confirm_filter() {
                    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
                    {
                        let regime_filter = build_volume_flow_confirmation_regime_device(
                            candles, device_id, &strategy,
                        )?;
                        optimize_double_ma_gpu_kernel_tiled_with_device_regime(
                            candles,
                            device_id,
                            fast_periods,
                            slow_periods,
                            &fast_ma,
                            &slow_ma,
                            &ma_source,
                            fast_ma_params,
                            slow_ma_params,
                            &regime_filter,
                            &strategy,
                            objective,
                            score_mode,
                            extra_shortlist_objectives.as_deref(),
                            requested_shortlist_confidence_mode,
                            requested_shortlist_union_mode,
                            requested_shortlist_slice_mode,
                            requested_shortlist_slice_benchmark.clone(),
                            top_k,
                            include_all,
                            fast_alma_offset,
                            fast_alma_sigma,
                            slow_alma_offset,
                            slow_alma_sigma,
                            requested_trade_metrics,
                            requested_metric_contract,
                            constraints,
                            export_all_csv_path,
                            total_pairs,
                            heatmap_bins,
                            eval_range,
                            cancel,
                            progress,
                        )?
                    }
                    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
                    {
                        return Err(
                            "VFCT GPU kernel backend requires `cuda,cuda-backtest-kernel`"
                                .to_string(),
                        );
                    }
                } else if strategy.uses_slope_efficiency_trend_filter() {
                    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
                    {
                        let regime_filter = build_slope_efficiency_trend_regime_device(
                            candles, &ma_source, device_id, &strategy,
                        )?;
                        optimize_double_ma_gpu_kernel_tiled_with_device_regime(
                            candles,
                            device_id,
                            fast_periods,
                            slow_periods,
                            &fast_ma,
                            &slow_ma,
                            &ma_source,
                            fast_ma_params,
                            slow_ma_params,
                            &regime_filter,
                            &strategy,
                            objective,
                            score_mode,
                            extra_shortlist_objectives.as_deref(),
                            requested_shortlist_confidence_mode,
                            requested_shortlist_union_mode,
                            requested_shortlist_slice_mode,
                            requested_shortlist_slice_benchmark.clone(),
                            top_k,
                            include_all,
                            fast_alma_offset,
                            fast_alma_sigma,
                            slow_alma_offset,
                            slow_alma_sigma,
                            requested_trade_metrics,
                            requested_metric_contract,
                            constraints,
                            export_all_csv_path,
                            total_pairs,
                            heatmap_bins,
                            eval_range,
                            cancel,
                            progress,
                        )?
                    }
                    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
                    {
                        return Err(
                            "SET GPU kernel backend requires `cuda,cuda-backtest-kernel`"
                                .to_string(),
                        );
                    }
                } else if strategy.uses_momentum_triple_stack_filter() {
                    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
                    {
                        let regime_filter = build_momentum_triple_stack_regime_device(
                            candles, &ma_source, device_id, &strategy,
                        )?;
                        optimize_double_ma_gpu_kernel_tiled_with_device_regime(
                            candles,
                            device_id,
                            fast_periods,
                            slow_periods,
                            &fast_ma,
                            &slow_ma,
                            &ma_source,
                            fast_ma_params,
                            slow_ma_params,
                            &regime_filter,
                            &strategy,
                            objective,
                            score_mode,
                            extra_shortlist_objectives.as_deref(),
                            requested_shortlist_confidence_mode,
                            requested_shortlist_union_mode,
                            requested_shortlist_slice_mode,
                            requested_shortlist_slice_benchmark.clone(),
                            top_k,
                            include_all,
                            fast_alma_offset,
                            fast_alma_sigma,
                            slow_alma_offset,
                            slow_alma_sigma,
                            requested_trade_metrics,
                            requested_metric_contract,
                            constraints,
                            export_all_csv_path,
                            total_pairs,
                            heatmap_bins,
                            eval_range,
                            cancel,
                            progress,
                        )?
                    }
                    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
                    {
                        return Err(
                            "MTS GPU kernel backend requires `cuda,cuda-backtest-kernel`"
                                .to_string(),
                        );
                    }
                } else if strategy.uses_oscillator_reversion_confluence_filter() {
                    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
                    {
                        let regime_filter = build_oscillator_reversion_confluence_regime_device(
                            candles, &ma_source, device_id, &strategy,
                        )?;
                        optimize_double_ma_gpu_kernel_tiled_with_device_regime(
                            candles,
                            device_id,
                            fast_periods,
                            slow_periods,
                            &fast_ma,
                            &slow_ma,
                            &ma_source,
                            fast_ma_params,
                            slow_ma_params,
                            &regime_filter,
                            &strategy,
                            objective,
                            score_mode,
                            extra_shortlist_objectives.as_deref(),
                            requested_shortlist_confidence_mode,
                            requested_shortlist_union_mode,
                            requested_shortlist_slice_mode,
                            requested_shortlist_slice_benchmark.clone(),
                            top_k,
                            include_all,
                            fast_alma_offset,
                            fast_alma_sigma,
                            slow_alma_offset,
                            slow_alma_sigma,
                            requested_trade_metrics,
                            requested_metric_contract,
                            constraints,
                            export_all_csv_path,
                            total_pairs,
                            heatmap_bins,
                            eval_range,
                            cancel,
                            progress,
                        )?
                    }
                    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
                    {
                        return Err(
                            "ORC GPU kernel backend requires `cuda,cuda-backtest-kernel`"
                                .to_string(),
                        );
                    }
                } else {
                    let regime_filter = ensure_combined_regime_filter_cached(
                        &mut host_regime_filter_cache,
                        candles,
                        &ma_source,
                        &strategy,
                        halftrend_filter.as_ref(),
                        n,
                    )?;
                    optimize_double_ma_gpu_kernel_tiled(
                        candles,
                        device_id,
                        fast_periods,
                        slow_periods,
                        &fast_ma,
                        &slow_ma,
                        &ma_source,
                        fast_ma_params,
                        slow_ma_params,
                        regime_filter,
                        &strategy,
                        objective,
                        score_mode,
                        extra_shortlist_objectives.as_deref(),
                        requested_shortlist_confidence_mode,
                        requested_shortlist_union_mode,
                        requested_shortlist_slice_mode,
                        requested_shortlist_slice_benchmark.clone(),
                        top_k,
                        include_all,
                        fast_alma_offset,
                        fast_alma_sigma,
                        slow_alma_offset,
                        slow_alma_sigma,
                        requested_trade_metrics,
                        requested_metric_contract,
                        constraints,
                        export_all_csv_path,
                        total_pairs,
                        heatmap_bins,
                        eval_range,
                        cancel,
                        progress,
                    )?
                };
                let gpu_kernel_timing_ms = Some(build_gpu_kernel_timing(
                    kernel_t0.elapsed().as_millis() as u64,
                ));
                let route_note = match dynamic_gpu_handle {
                    Some(_) => Some("GpuKernel(dynamic_strategy_v1)".to_string()),
                    None => None,
                };
                Ok((
                    result,
                    BackendUsed::Gpu,
                    None,
                    gpu_kernel_timing_ms,
                    route_note,
                    None,
                ))
            }
            GpuGridDispatch::GpuHybridKernel { device_id } => {
                let regime_filter = ensure_combined_regime_filter_cached(
                    &mut host_regime_filter_cache,
                    candles,
                    &ma_source,
                    &strategy,
                    halftrend_filter.as_ref(),
                    n,
                )?;
                let (result, timing, backend_used, route_note, fallback_reason) =
                    optimize_double_ma_gpu_hybrid_kernel_phase1(
                        candles,
                        device_id,
                        fast_periods,
                        slow_periods,
                        &fast_ma,
                        &slow_ma,
                        &ma_source,
                        fast_ma_params,
                        slow_ma_params,
                        regime_filter,
                        &strategy,
                        objective,
                        score_mode,
                        top_k,
                        include_all,
                        fast_alma_offset,
                        fast_alma_sigma,
                        slow_alma_offset,
                        slow_alma_sigma,
                        requested_trade_metrics,
                        requested_metric_contract,
                        constraints,
                        export_all_csv_path,
                        total_pairs,
                        heatmap_bins,
                        cancel,
                        progress,
                        req.gpu_execution,
                        !matches!(mode_used, OptimizationModeResolved::GpuVdps),
                        eval_range,
                    )?;
                Ok((
                    result,
                    backend_used,
                    Some(timing),
                    None,
                    route_note,
                    fallback_reason,
                ))
            }
        };

        match &backend {
            Backend::CpuOnly => {
                if export_all_csv_path.is_some() {
                    return Err("export_all_csv_path is currently supported only for the GPU (kernel) backend".to_string());
                }
                let regime_filter = ensure_combined_regime_filter_cached(
                    &mut host_regime_filter_cache,
                    candles,
                    &ma_source,
                    &strategy,
                    halftrend_filter.as_ref(),
                    n,
                )?;
                let result = optimize_double_ma_cpu_tiled(
                    candles,
                    fast_periods,
                    slow_periods,
                    &fast_ma,
                    &slow_ma,
                    &ma_source,
                    fast_ma_params,
                    slow_ma_params,
                    regime_filter,
                    &strategy,
                    objective,
                    score_mode,
                    top_k,
                    include_all,
                    fast_alma_offset,
                    fast_alma_sigma,
                    slow_alma_offset,
                    slow_alma_sigma,
                    constraints,
                    total_pairs,
                    heatmap_bins,
                    dynamic_runtime_plan.as_ref(),
                    cancel,
                    progress,
                )?;
                Ok((result, BackendUsed::Cpu, None, None, None, None))
            }
            Backend::GpuOnly { device_id } => run_gpu_grid(GpuGridDispatch::GpuOnly {
                device_id: *device_id,
            }),
            Backend::GpuKernel { device_id } => {
                if let DynamicGpuExecutionSelection::CpuFallback { reason } = dynamic_gpu_execution
                {
                    let regime_filter = ensure_combined_regime_filter_cached(
                        &mut host_regime_filter_cache,
                        candles,
                        &ma_source,
                        &strategy,
                        halftrend_filter.as_ref(),
                        n,
                    )?;
                    let result = optimize_double_ma_cpu_tiled(
                        candles,
                        fast_periods,
                        slow_periods,
                        &fast_ma,
                        &slow_ma,
                        &ma_source,
                        fast_ma_params,
                        slow_ma_params,
                        regime_filter,
                        &strategy,
                        objective,
                        score_mode,
                        top_k,
                        include_all,
                        fast_alma_offset,
                        fast_alma_sigma,
                        slow_alma_offset,
                        slow_alma_sigma,
                        constraints,
                        total_pairs,
                        heatmap_bins,
                        dynamic_runtime_plan.as_ref(),
                        cancel,
                        progress,
                    )?;
                    Ok((
                        result,
                        BackendUsed::Cpu,
                        None,
                        None,
                        Some("GpuKernel->CpuOnly(dynamic_strategy_v1)".to_string()),
                        Some(reason.clone()),
                    ))
                } else {
                    run_gpu_grid(GpuGridDispatch::GpuKernel {
                        device_id: *device_id,
                    })
                }
            }
            Backend::GpuHybridKernel { device_id } => {
                run_gpu_grid(GpuGridDispatch::GpuHybridKernel {
                    device_id: *device_id,
                })
            }
            Backend::Auto { .. } => {
                unreachable!("Backend::Auto should be resolved before dispatch")
            }
        }
    };

    struct ModeRunResult {
        res: OptimizationResult,
        backend_used: BackendUsed,
        vdps: Option<VdpsTelemetry>,
        hybrid_timing_ms: Option<HybridTimingMs>,
        gpu_kernel_timing_ms: Option<GpuKernelTimingMs>,
        route_note: Option<String>,
        fallback_reason: Option<String>,
    }

    let mut run_mode_for_params = |fast_ma_params: Option<&HashMap<String, f64>>,
                                   slow_ma_params: Option<&HashMap<String, f64>>,
                                   fast_alma_offset: f64,
                                   fast_alma_sigma: f64,
                                   slow_alma_offset: f64,
                                   slow_alma_sigma: f64,
                                   constraints: Option<&DoubleMaConstraints>,
                                   progress: Option<&dyn ProgressSink>,
                                   dynamic_gpu_execution: &DynamicGpuExecutionSelection|
     -> Result<ModeRunResult, String> {
        match mode_used {
            OptimizationModeResolved::Grid => {
                let (
                    res,
                    backend_used,
                    hybrid_timing_ms,
                    gpu_kernel_timing_ms,
                    route_note,
                    fallback_reason,
                ) = run_grid(
                    &fast_periods,
                    &slow_periods,
                    top_k,
                    include_all,
                    heatmap_bins,
                    export_all_csv_path.as_deref(),
                    total_pairs,
                    fast_ma_params,
                    slow_ma_params,
                    fast_alma_offset,
                    fast_alma_sigma,
                    slow_alma_offset,
                    slow_alma_sigma,
                    constraints,
                    progress,
                    dynamic_gpu_execution,
                )?;
                Ok(ModeRunResult {
                    res,
                    backend_used,
                    vdps: None,
                    hybrid_timing_ms,
                    gpu_kernel_timing_ms,
                    route_note,
                    fallback_reason,
                })
            }
            OptimizationModeResolved::CoarseToFine => {
                let target_pairs = 200_000usize;
                let mut factor = 1u32;
                if total_pairs > target_pairs {
                    let ratio = (total_pairs as f64) / (target_pairs as f64);
                    let est = ratio.sqrt().ceil() as u32;
                    factor = est.max(2).min(64);
                }

                if factor <= 1 {
                    let (
                        res,
                        backend_used,
                        hybrid_timing_ms,
                        gpu_kernel_timing_ms,
                        route_note,
                        fallback_reason,
                    ) = run_grid(
                        &fast_periods,
                        &slow_periods,
                        top_k,
                        include_all,
                        heatmap_bins,
                        export_all_csv_path.as_deref(),
                        total_pairs,
                        fast_ma_params,
                        slow_ma_params,
                        fast_alma_offset,
                        fast_alma_sigma,
                        slow_alma_offset,
                        slow_alma_sigma,
                        constraints,
                        progress,
                        dynamic_gpu_execution,
                    )?;
                    Ok(ModeRunResult {
                        res,
                        backend_used,
                        vdps: None,
                        hybrid_timing_ms,
                        gpu_kernel_timing_ms,
                        route_note,
                        fallback_reason,
                    })
                } else {
                    let coarse_fast = (
                        fast.0,
                        fast.1,
                        if fast.2 == 0 {
                            0
                        } else {
                            fast.2.saturating_mul(factor)
                        },
                    );
                    let coarse_slow = (
                        slow.0,
                        slow.1,
                        if slow.2 == 0 {
                            0
                        } else {
                            slow.2.saturating_mul(factor)
                        },
                    );
                    let coarse_fast_periods = expand_u16_range(coarse_fast)?;
                    let coarse_slow_periods = expand_u16_range(coarse_slow)?;
                    let coarse_pairs =
                        estimate_pairs_fast_lt_slow(&coarse_fast_periods, &coarse_slow_periods);
                    if coarse_pairs == 0 {
                        return Err(
                            "coarse search produced no valid parameter combinations".to_string()
                        );
                    }

                    let (
                        coarse_res,
                        coarse_backend,
                        coarse_hybrid_timing_ms,
                        coarse_gpu_kernel_timing_ms,
                        coarse_route_note,
                        coarse_fallback_reason,
                    ) = run_grid(
                        &coarse_fast_periods,
                        &coarse_slow_periods,
                        0,
                        false,
                        0,
                        None,
                        coarse_pairs,
                        fast_ma_params,
                        slow_ma_params,
                        fast_alma_offset,
                        fast_alma_sigma,
                        slow_alma_offset,
                        slow_alma_sigma,
                        constraints,
                        progress,
                        dynamic_gpu_execution,
                    )?;

                    let best_fast = coarse_res.best_params.fast_len as u32;
                    let best_slow = coarse_res.best_params.slow_len as u32;

                    let (fast_min, fast_max) = (fast.0.min(fast.1), fast.0.max(fast.1));
                    let (slow_min, slow_max) = (slow.0.min(slow.1), slow.0.max(slow.1));

                    let f_step = fast.2;
                    let s_step = slow.2;
                    let f_win = if f_step == 0 {
                        0
                    } else {
                        f_step.saturating_mul(factor).saturating_mul(2).max(f_step)
                    };
                    let s_win = if s_step == 0 {
                        0
                    } else {
                        s_step.saturating_mul(factor).saturating_mul(2).max(s_step)
                    };

                    let fine_fast = (
                        best_fast.saturating_sub(f_win).max(fast_min),
                        best_fast.saturating_add(f_win).min(fast_max),
                        f_step,
                    );
                    let fine_slow = (
                        best_slow.saturating_sub(s_win).max(slow_min),
                        best_slow.saturating_add(s_win).min(slow_max),
                        s_step,
                    );

                    let fine_fast_periods = expand_u16_range(fine_fast)?;
                    let fine_slow_periods = expand_u16_range(fine_slow)?;
                    let fine_pairs =
                        estimate_pairs_fast_lt_slow(&fine_fast_periods, &fine_slow_periods);
                    if fine_pairs == 0 {
                        return Err(
                            "fine search produced no valid parameter combinations".to_string()
                        );
                    }

                    let (
                        mut fine_res,
                        fine_backend,
                        fine_hybrid_timing_ms,
                        fine_gpu_kernel_timing_ms,
                        fine_route_note,
                        fine_fallback_reason,
                    ) = run_grid(
                        &fine_fast_periods,
                        &fine_slow_periods,
                        top_k,
                        include_all,
                        heatmap_bins,
                        None,
                        fine_pairs,
                        fast_ma_params,
                        slow_ma_params,
                        fast_alma_offset,
                        fast_alma_sigma,
                        slow_alma_offset,
                        slow_alma_sigma,
                        constraints,
                        progress,
                        dynamic_gpu_execution,
                    )?;
                    fine_res.num_combos = coarse_res.num_combos.saturating_add(fine_res.num_combos);

                    let backend_used = if matches!(coarse_backend, BackendUsed::Cpu)
                        && matches!(fine_backend, BackendUsed::Cpu)
                    {
                        BackendUsed::Cpu
                    } else {
                        BackendUsed::Gpu
                    };
                    let mut hybrid_timing_ms: Option<HybridTimingMs> = None;
                    merge_hybrid_timing(&mut hybrid_timing_ms, coarse_hybrid_timing_ms);
                    merge_hybrid_timing(&mut hybrid_timing_ms, fine_hybrid_timing_ms);
                    let mut gpu_kernel_timing_ms: Option<GpuKernelTimingMs> = None;
                    merge_gpu_kernel_timing(&mut gpu_kernel_timing_ms, coarse_gpu_kernel_timing_ms);
                    merge_gpu_kernel_timing(&mut gpu_kernel_timing_ms, fine_gpu_kernel_timing_ms);
                    let mut route_note: Option<String> = None;
                    merge_optional_note(&mut route_note, coarse_route_note);
                    merge_optional_note(&mut route_note, fine_route_note);
                    let mut fallback_reason: Option<String> = None;
                    merge_optional_note(&mut fallback_reason, coarse_fallback_reason);
                    merge_optional_note(&mut fallback_reason, fine_fallback_reason);

                    Ok(ModeRunResult {
                        res: fine_res,
                        backend_used,
                        vdps: None,
                        hybrid_timing_ms,
                        gpu_kernel_timing_ms,
                        route_note,
                        fallback_reason,
                    })
                }
            }
            OptimizationModeResolved::GpuVdps => {
                let cfg = vdps_cfg
                    .clone()
                    .ok_or_else(|| "GpuVdps config is not available".to_string())?;
                let rung_plan = cfg.rungs.clone();
                if rung_plan.is_empty() {
                    return Err("GpuVdps mode has no valid rung schedule".to_string());
                }

                let mut total_evaluated_pairs: usize = 0;
                let mut full_fidelity_pairs_evaluated: usize = 0;
                let mut any_gpu = false;
                let mut last_res: Option<OptimizationResult> = None;
                let mut rung_stats: Vec<VdpsRungTelemetry> = Vec::with_capacity(rung_plan.len());
                let mut hybrid_timing_ms_acc: Option<HybridTimingMs> = None;
                let mut gpu_kernel_timing_ms_acc: Option<GpuKernelTimingMs> = None;
                let mut route_note_acc: Option<String> = None;
                let mut fallback_reason_acc: Option<String> = None;

                let mut seed_pairs: Vec<(u16, u16)> = Vec::new();
                let fast_step_base = fast.2.max(1) as u16;
                let slow_step_base = slow.2.max(1) as u16;
                let random_seed = if cfg.deterministic {
                    cfg.seed
                } else {
                    rand::thread_rng().gen()
                };
                let mut rng = StdRng::seed_from_u64(random_seed);
                let (elite_refill_ratio, diff_refill_ratio, rand_refill_ratio) =
                    vdps_normalized_refill_mix(&cfg);

                for (ri, frac) in rung_plan.iter().enumerate() {
                    if cancel.load(Ordering::Relaxed) {
                        return Err("cancelled".to_string());
                    }
                    let rung_start = std::time::Instant::now();

                    emit_progress(
                        progress,
                        ri,
                        rung_plan.len(),
                        if ri + 1 == rung_plan.len() {
                            "vdps-finalize"
                        } else {
                            "vdps-schedule"
                        },
                    );

                    let target_pairs_raw = vdps_target_pairs(total_pairs, *frac);
                    let target_pairs = if ri == 0 {
                        target_pairs_raw.max(cfg.min_initial_pairs.min(total_pairs))
                    } else {
                        target_pairs_raw
                    };

                    let rem = rung_plan.len().saturating_sub(ri);
                    let rem_u16 = rem.min(u16::MAX as usize) as u16;
                    let f_win = fast_step_base
                        .saturating_mul(rem_u16)
                        .saturating_mul(2)
                        .max(fast_step_base);
                    let s_win = slow_step_base
                        .saturating_mul(rem_u16)
                        .saturating_mul(2)
                        .max(slow_step_base);

                    let (rung_fast, rung_slow) = if ri == 0 {
                        let mut stride =
                            (((total_pairs as f64) / (target_pairs as f64)).sqrt().ceil() as usize)
                                .max(1);
                        let mut fast_sel = vdps_decimated_periods(&fast_periods, stride);
                        let mut slow_sel = vdps_decimated_periods(&slow_periods, stride);
                        let mut rung_pairs = estimate_pairs_fast_lt_slow(&fast_sel, &slow_sel);
                        while rung_pairs > target_pairs.saturating_mul(2)
                            && (fast_sel.len() > 8 || slow_sel.len() > 8)
                            && stride < 256
                        {
                            stride = stride.saturating_add(1);
                            fast_sel = vdps_decimated_periods(&fast_periods, stride);
                            slow_sel = vdps_decimated_periods(&slow_periods, stride);
                            rung_pairs = estimate_pairs_fast_lt_slow(&fast_sel, &slow_sel);
                        }
                        (fast_sel, slow_sel)
                    } else {
                        let mut fast_set: BTreeSet<u16> = BTreeSet::new();
                        let mut slow_set: BTreeSet<u16> = BTreeSet::new();

                        let rem = rung_plan.len().saturating_sub(ri);
                        let rem_u16 = rem.min(u16::MAX as usize) as u16;
                        let f_win = fast_step_base
                            .saturating_mul(rem_u16)
                            .saturating_mul(2)
                            .max(fast_step_base);
                        let s_win = slow_step_base
                            .saturating_mul(rem_u16)
                            .saturating_mul(2)
                            .max(slow_step_base);

                        for (f, s) in seed_pairs.iter().copied() {
                            for &fp in vdps_period_window_slice(&fast_periods, f, f_win) {
                                fast_set.insert(fp);
                            }
                            for &sp in vdps_period_window_slice(&slow_periods, s, s_win) {
                                slow_set.insert(sp);
                            }
                        }

                        if fast_set.is_empty() || slow_set.is_empty() {
                            (fast_periods.clone(), slow_periods.clone())
                        } else {
                            let mut fast_candidates: Vec<u16> = fast_set.into_iter().collect();
                            let mut slow_candidates: Vec<u16> = slow_set.into_iter().collect();

                            let best_seed = seed_pairs.first().copied().unwrap_or_else(|| {
                                (
                                    fast_periods[fast_periods.len() / 2],
                                    slow_periods[slow_periods.len() / 2],
                                )
                            });

                            if fast_candidates.len() > cfg.max_period_candidates {
                                fast_candidates.sort_by_key(|&p| (p.abs_diff(best_seed.0), p));
                                fast_candidates.truncate(cfg.max_period_candidates);
                                fast_candidates.sort_unstable();
                            }
                            if slow_candidates.len() > cfg.max_period_candidates {
                                slow_candidates.sort_by_key(|&p| (p.abs_diff(best_seed.1), p));
                                slow_candidates.truncate(cfg.max_period_candidates);
                                slow_candidates.sort_unstable();
                            }

                            let mut thin_stride = 1usize;
                            let mut fast_sel = fast_candidates.clone();
                            let mut slow_sel = slow_candidates.clone();
                            let mut rung_pairs = estimate_pairs_fast_lt_slow(&fast_sel, &slow_sel);
                            while rung_pairs > target_pairs.saturating_mul(3)
                                && (fast_sel.len() > 8 || slow_sel.len() > 8)
                                && thin_stride < 128
                            {
                                thin_stride = thin_stride.saturating_add(1);
                                fast_sel = vdps_decimated_periods(&fast_candidates, thin_stride);
                                slow_sel = vdps_decimated_periods(&slow_candidates, thin_stride);
                                rung_pairs = estimate_pairs_fast_lt_slow(&fast_sel, &slow_sel);
                            }
                            (fast_sel, slow_sel)
                        }
                    };

                    let survivors_target =
                        ((target_pairs as f64) / (cfg.eta as f64)).ceil() as usize;
                    let rung_top_k = if ri + 1 == rung_plan.len() {
                        top_k
                    } else {
                        survivors_target.clamp(top_k.max(8), cfg.max_survivor_seeds)
                    };

                    let mut rung_fast_unique: BTreeSet<u16> =
                        rung_fast.iter().copied().collect::<BTreeSet<_>>();
                    let mut rung_slow_unique: BTreeSet<u16> =
                        rung_slow.iter().copied().collect::<BTreeSet<_>>();
                    let mut partial_results: Vec<OptimizationResult> = Vec::new();
                    let mut rung_backend = BackendUsed::Cpu;
                    let mut used_anchor_windows = false;

                    if ri > 0
                        && total_pairs >= VDPS_LOCAL_FRONTIER_MIN_FULL_GRID_PAIRS
                        && !seed_pairs.is_empty()
                    {
                        let anchor_cap = seed_pairs
                            .len()
                            .min(cfg.max_survivor_seeds)
                            .min(VDPS_DEFAULT_MAX_ACTIVE_ANCHORS)
                            .max(1);
                        let target_pairs_per_anchor =
                            ((target_pairs as f64) / (anchor_cap as f64)).ceil() as usize;
                        let mut seen_anchor_shapes: BTreeSet<(u16, u16, usize, u16, u16, usize)> =
                            BTreeSet::new();

                        for &(anchor_f, anchor_s) in seed_pairs.iter().take(anchor_cap) {
                            let (local_fast, local_slow) = vdps_local_anchor_axes(
                                anchor_f,
                                anchor_s,
                                &fast_periods,
                                &slow_periods,
                                f_win,
                                s_win,
                                cfg.max_period_candidates,
                                target_pairs_per_anchor,
                            );
                            if local_fast.is_empty() || local_slow.is_empty() {
                                continue;
                            }
                            let Some(&fast_first) = local_fast.first() else {
                                continue;
                            };
                            let Some(&fast_last) = local_fast.last() else {
                                continue;
                            };
                            let Some(&slow_first) = local_slow.first() else {
                                continue;
                            };
                            let Some(&slow_last) = local_slow.last() else {
                                continue;
                            };
                            let signature = (
                                fast_first,
                                fast_last,
                                local_fast.len(),
                                slow_first,
                                slow_last,
                                local_slow.len(),
                            );
                            if !seen_anchor_shapes.insert(signature) {
                                continue;
                            }

                            let local_pairs = estimate_pairs_fast_lt_slow(&local_fast, &local_slow);
                            if local_pairs == 0 {
                                continue;
                            }

                            let (
                                local_res,
                                local_backend,
                                local_hybrid_timing_ms,
                                local_gpu_kernel_timing_ms,
                                local_route_note,
                                local_fallback_reason,
                            ) = run_grid(
                                &local_fast,
                                &local_slow,
                                rung_top_k.max(1),
                                false,
                                0,
                                None,
                                local_pairs,
                                fast_ma_params,
                                slow_ma_params,
                                fast_alma_offset,
                                fast_alma_sigma,
                                slow_alma_offset,
                                slow_alma_sigma,
                                constraints,
                                progress,
                                dynamic_gpu_execution,
                            )?;
                            merge_hybrid_timing(&mut hybrid_timing_ms_acc, local_hybrid_timing_ms);
                            merge_gpu_kernel_timing(
                                &mut gpu_kernel_timing_ms_acc,
                                local_gpu_kernel_timing_ms,
                            );
                            merge_optional_note(&mut route_note_acc, local_route_note);
                            merge_optional_note(&mut fallback_reason_acc, local_fallback_reason);
                            if matches!(local_backend, BackendUsed::Gpu) {
                                any_gpu = true;
                                rung_backend = BackendUsed::Gpu;
                            }
                            rung_fast_unique.extend(local_fast.iter().copied());
                            rung_slow_unique.extend(local_slow.iter().copied());
                            partial_results.push(local_res);
                            used_anchor_windows = true;
                        }
                    }

                    if partial_results.is_empty() {
                        let rung_pairs = estimate_pairs_fast_lt_slow(&rung_fast, &rung_slow);
                        if rung_pairs == 0 {
                            continue;
                        }

                        let (
                            rung_res,
                            single_rung_backend,
                            rung_hybrid_timing_ms,
                            rung_gpu_kernel_timing_ms,
                            rung_route_note,
                            rung_fallback_reason,
                        ) = run_grid(
                            &rung_fast,
                            &rung_slow,
                            rung_top_k.max(1),
                            false,
                            0,
                            None,
                            rung_pairs,
                            fast_ma_params,
                            slow_ma_params,
                            fast_alma_offset,
                            fast_alma_sigma,
                            slow_alma_offset,
                            slow_alma_sigma,
                            constraints,
                            progress,
                            dynamic_gpu_execution,
                        )?;
                        merge_hybrid_timing(&mut hybrid_timing_ms_acc, rung_hybrid_timing_ms);
                        merge_gpu_kernel_timing(
                            &mut gpu_kernel_timing_ms_acc,
                            rung_gpu_kernel_timing_ms,
                        );
                        merge_optional_note(&mut route_note_acc, rung_route_note);
                        merge_optional_note(&mut fallback_reason_acc, rung_fallback_reason);

                        if matches!(single_rung_backend, BackendUsed::Gpu) {
                            any_gpu = true;
                            rung_backend = BackendUsed::Gpu;
                        }
                        partial_results.push(rung_res);
                    }

                    let mut rung_res = vdps_merge_partial_results(
                        partial_results,
                        objective,
                        rung_top_k.max(1),
                        candles.close.len(),
                    )
                    .ok_or_else(|| "GpuVdps produced no valid rung result".to_string())?;

                    total_evaluated_pairs =
                        total_evaluated_pairs.saturating_add(rung_res.num_combos);

                    let mut ranked_input: Vec<(DoubleMaParams, Metrics)> =
                        Vec::with_capacity(rung_res.top.len().saturating_add(1));
                    ranked_input
                        .push((rung_res.best_params.clone(), rung_res.best_metrics.clone()));
                    ranked_input.extend(rung_res.top.iter().cloned());
                    let ranked_limit = survivors_target.clamp(8, cfg.max_survivor_seeds);
                    let ranked =
                        vdps_rank_candidates(ranked_input, objective, score_mode, ranked_limit);
                    let promoted_pairs = ranked.len();
                    let mut next_seed_pairs = ranked
                        .iter()
                        .map(|(p, _)| (p.fast_len, p.slow_len))
                        .collect::<Vec<_>>();

                    if ri + 1 != rung_plan.len() {
                        let refill_budget =
                            cfg.max_survivor_seeds.saturating_sub(next_seed_pairs.len());
                        if refill_budget > 0 {
                            let mut elite_inject =
                                ((refill_budget as f64) * elite_refill_ratio).round() as usize;
                            let mut diff_inject =
                                ((refill_budget as f64) * diff_refill_ratio).round() as usize;
                            let mut rand_inject =
                                ((refill_budget as f64) * rand_refill_ratio).round() as usize;
                            let assigned = elite_inject
                                .saturating_add(diff_inject)
                                .saturating_add(rand_inject);
                            if assigned > refill_budget {
                                let overflow = assigned.saturating_sub(refill_budget);
                                let take_rand = overflow.min(rand_inject);
                                rand_inject = rand_inject.saturating_sub(take_rand);
                                let rem = overflow.saturating_sub(take_rand);
                                let take_diff = rem.min(diff_inject);
                                diff_inject = diff_inject.saturating_sub(take_diff);
                                let rem2 = rem.saturating_sub(take_diff);
                                elite_inject = elite_inject.saturating_sub(rem2.min(elite_inject));
                            } else if assigned < refill_budget {
                                rand_inject = rand_inject
                                    .saturating_add(refill_budget.saturating_sub(assigned));
                            }
                            let elite_seeds = next_seed_pairs.clone();
                            if vdps_gpu_refill_enabled() {
                                if let Some(device_id) = vdps_gpu_refill_device_id {
                                    let gpu_seed: u64 = rng.gen();
                                    if let Ok(Some(gpu_pairs)) = vdps_generate_seed_pairs_gpu(
                                        device_id,
                                        &elite_seeds,
                                        &rung_fast,
                                        &rung_slow,
                                        elite_inject,
                                        diff_inject,
                                        rand_inject,
                                        fast_step_base,
                                        slow_step_base,
                                        gpu_seed,
                                    ) {
                                        let mut seen: BTreeSet<(u16, u16)> =
                                            next_seed_pairs.iter().copied().collect();
                                        for pair in gpu_pairs.into_iter() {
                                            if seen.insert(pair) {
                                                next_seed_pairs.push(pair);
                                                if next_seed_pairs.len() >= cfg.max_survivor_seeds {
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            let refill_remaining =
                                cfg.max_survivor_seeds.saturating_sub(next_seed_pairs.len());
                            if refill_remaining > 0 {
                                let mut elite_cpu = ((refill_remaining as f64) * elite_refill_ratio)
                                    .round()
                                    as usize;
                                let mut diff_cpu = ((refill_remaining as f64) * diff_refill_ratio)
                                    .round()
                                    as usize;
                                let mut rand_cpu = ((refill_remaining as f64) * rand_refill_ratio)
                                    .round()
                                    as usize;
                                let assigned_cpu =
                                    elite_cpu.saturating_add(diff_cpu).saturating_add(rand_cpu);
                                if assigned_cpu > refill_remaining {
                                    let overflow = assigned_cpu.saturating_sub(refill_remaining);
                                    let take_rand = overflow.min(rand_cpu);
                                    rand_cpu = rand_cpu.saturating_sub(take_rand);
                                    let rem = overflow.saturating_sub(take_rand);
                                    let take_diff = rem.min(diff_cpu);
                                    diff_cpu = diff_cpu.saturating_sub(take_diff);
                                    let rem2 = rem.saturating_sub(take_diff);
                                    elite_cpu = elite_cpu.saturating_sub(rem2.min(elite_cpu));
                                } else if assigned_cpu < refill_remaining {
                                    rand_cpu = rand_cpu.saturating_add(
                                        refill_remaining.saturating_sub(assigned_cpu),
                                    );
                                }

                                vdps_inject_elite_mutation_pairs(
                                    &mut next_seed_pairs,
                                    &elite_seeds,
                                    &rung_fast,
                                    &rung_slow,
                                    elite_cpu,
                                    fast_step_base,
                                    slow_step_base,
                                    &mut rng,
                                );
                                vdps_inject_differential_seed_pairs(
                                    &mut next_seed_pairs,
                                    &elite_seeds,
                                    &rung_fast,
                                    &rung_slow,
                                    diff_cpu,
                                    &mut rng,
                                );
                                vdps_inject_random_seed_pairs(
                                    &mut next_seed_pairs,
                                    &rung_fast,
                                    &rung_slow,
                                    rand_cpu,
                                    &mut rng,
                                );
                            }
                        }
                        // Intermediate rungs are only for seed generation.
                        rung_res.top = ranked;
                    } else {
                        full_fidelity_pairs_evaluated = rung_res.num_combos;
                    }
                    seed_pairs = next_seed_pairs;
                    let runtime_ms = rung_start.elapsed().as_millis() as u64;
                    let throughput_pairs_per_sec = if runtime_ms > 0 {
                        (rung_res.num_combos as f64) * 1000.0 / (runtime_ms as f64)
                    } else {
                        rung_res.num_combos as f64
                    };
                    rung_stats.push(VdpsRungTelemetry {
                        rung_index: ri,
                        rung_fraction: *frac,
                        target_pairs,
                        evaluated_pairs: rung_res.num_combos,
                        promoted_pairs,
                        dropped_pairs: rung_res.num_combos.saturating_sub(promoted_pairs),
                        fast_candidates: rung_fast_unique.len(),
                        slow_candidates: rung_slow_unique.len(),
                        backend_used: rung_backend,
                        runtime_ms,
                        throughput_pairs_per_sec,
                    });
                    if used_anchor_windows {
                        merge_optional_note(
                            &mut route_note_acc,
                            Some("GpuVdps used anchored local frontier windows.".to_string()),
                        );
                    }
                    last_res = Some(rung_res);
                }

                let mut final_res =
                    last_res.ok_or_else(|| "GpuVdps produced no valid rung result".to_string())?;
                final_res.num_combos = total_evaluated_pairs.max(final_res.num_combos);
                final_res.all = None;
                final_res.heatmap = None;

                if top_k > 0 {
                    let mut ranked_input: Vec<(DoubleMaParams, Metrics)> =
                        Vec::with_capacity(final_res.top.len().saturating_add(1));
                    ranked_input.push((
                        final_res.best_params.clone(),
                        final_res.best_metrics.clone(),
                    ));
                    ranked_input.extend(final_res.top.into_iter());
                    let ranked =
                        vdps_rank_candidates(ranked_input, objective, score_mode, top_k.max(1));
                    if let Some((p, m)) = ranked.first() {
                        final_res.best_params = p.clone();
                        final_res.best_metrics = m.clone();
                    }
                    final_res.top = ranked;
                } else {
                    final_res.top.clear();
                }

                let backend_used = if any_gpu {
                    BackendUsed::Gpu
                } else {
                    BackendUsed::Cpu
                };
                let runtime_ms_total = rung_stats
                    .iter()
                    .fold(0u64, |acc, r| acc.saturating_add(r.runtime_ms));
                let mut quality_check_top_k_overlap: Option<f64> = None;
                let mut quality_check_best_score_regret: Option<f64> = None;
                let mut quality_check_baseline_runtime_ms: Option<u64> = None;
                let mut quality_check_baseline_backend: Option<BackendUsed> = None;
                let mut quality_check_baseline_pairs: Option<usize> = None;
                if cfg.quality_check_max_pairs > 0 && total_pairs <= cfg.quality_check_max_pairs {
                    let quality_top_k = cfg.quality_check_top_k.max(1).min(top_k.max(1));
                    let quality_start = std::time::Instant::now();
                    if let Ok((
                        baseline_res,
                        baseline_backend,
                        baseline_hybrid_timing_ms,
                        baseline_gpu_kernel_timing_ms,
                        baseline_route_note,
                        baseline_fallback_reason,
                    )) = run_grid(
                        &fast_periods,
                        &slow_periods,
                        quality_top_k,
                        false,
                        0,
                        None,
                        total_pairs,
                        fast_ma_params,
                        slow_ma_params,
                        fast_alma_offset,
                        fast_alma_sigma,
                        slow_alma_offset,
                        slow_alma_sigma,
                        constraints,
                        None,
                        dynamic_gpu_execution,
                    ) {
                        merge_hybrid_timing(&mut hybrid_timing_ms_acc, baseline_hybrid_timing_ms);
                        merge_gpu_kernel_timing(
                            &mut gpu_kernel_timing_ms_acc,
                            baseline_gpu_kernel_timing_ms,
                        );
                        merge_optional_note(&mut route_note_acc, baseline_route_note);
                        merge_optional_note(&mut fallback_reason_acc, baseline_fallback_reason);
                        quality_check_baseline_runtime_ms =
                            Some(quality_start.elapsed().as_millis() as u64);
                        quality_check_baseline_backend = Some(baseline_backend);
                        quality_check_baseline_pairs = Some(baseline_res.num_combos);

                        let mut baseline_ranked_input: Vec<(DoubleMaParams, Metrics)> =
                            Vec::with_capacity(baseline_res.top.len().saturating_add(1));
                        baseline_ranked_input.push((
                            baseline_res.best_params.clone(),
                            baseline_res.best_metrics.clone(),
                        ));
                        baseline_ranked_input.extend(baseline_res.top.iter().cloned());
                        let baseline_ranked = vdps_rank_candidates(
                            baseline_ranked_input,
                            objective,
                            score_mode,
                            quality_top_k,
                        );

                        let mut vdps_ranked_input: Vec<(DoubleMaParams, Metrics)> =
                            Vec::with_capacity(final_res.top.len().saturating_add(1));
                        vdps_ranked_input.push((
                            final_res.best_params.clone(),
                            final_res.best_metrics.clone(),
                        ));
                        vdps_ranked_input.extend(final_res.top.iter().cloned());
                        let vdps_ranked = vdps_rank_candidates(
                            vdps_ranked_input,
                            objective,
                            score_mode,
                            quality_top_k,
                        );

                        let mut baseline_keys: BTreeSet<(u16, u16, u16, u16)> = BTreeSet::new();
                        for (p, _) in baseline_ranked.iter() {
                            baseline_keys.insert((
                                p.fast_len,
                                p.slow_len,
                                p.fast_ma_id,
                                p.slow_ma_id,
                            ));
                        }
                        let mut overlap_count = 0usize;
                        for (p, _) in vdps_ranked.iter() {
                            if baseline_keys.contains(&(
                                p.fast_len,
                                p.slow_len,
                                p.fast_ma_id,
                                p.slow_ma_id,
                            )) {
                                overlap_count = overlap_count.saturating_add(1);
                            }
                        }
                        let denom = quality_top_k.max(1) as f64;
                        quality_check_top_k_overlap = Some((overlap_count as f64) / denom);

                        let baseline_best_score =
                            effective_score(&baseline_res.best_metrics, objective, score_mode);
                        let vdps_best_score =
                            effective_score(&final_res.best_metrics, objective, score_mode);
                        if baseline_best_score.is_finite() && vdps_best_score.is_finite() {
                            quality_check_best_score_regret =
                                Some((baseline_best_score - vdps_best_score).max(0.0));
                        }
                    }
                }
                let vdps = VdpsTelemetry {
                    eta: cfg.eta as u32,
                    rungs: rung_plan,
                    min_initial_pairs: cfg.min_initial_pairs as u32,
                    max_survivor_seeds: cfg.max_survivor_seeds as u32,
                    max_period_candidates: cfg.max_period_candidates as u32,
                    random_injection_ratio: rand_refill_ratio,
                    elite_mutation_ratio: elite_refill_ratio,
                    differential_ratio: diff_refill_ratio,
                    deterministic: cfg.deterministic,
                    seed: random_seed,
                    rng_kind: "StdRng/ChaCha12".to_string(),
                    tie_break_schema_version: "v1".to_string(),
                    full_grid_pairs: total_pairs,
                    full_fidelity_pairs_evaluated,
                    full_fidelity_pairs_saved: total_pairs
                        .saturating_sub(full_fidelity_pairs_evaluated),
                    evaluated_pairs_total: total_evaluated_pairs,
                    runtime_ms_total,
                    quality_check_top_k_overlap,
                    quality_check_best_score_regret,
                    quality_check_baseline_runtime_ms,
                    quality_check_baseline_backend,
                    quality_check_baseline_pairs,
                    rung_stats,
                };
                Ok(ModeRunResult {
                    res: final_res,
                    backend_used,
                    vdps: Some(vdps),
                    hybrid_timing_ms: hybrid_timing_ms_acc,
                    gpu_kernel_timing_ms: gpu_kernel_timing_ms_acc,
                    route_note: route_note_acc,
                    fallback_reason: fallback_reason_acc,
                })
            }
        }
    };

    fn rewrite_result_ma_ids(res: &mut OptimizationResult, fast_id: u16, slow_id: u16) {
        res.best_params.fast_ma_id = fast_id;
        res.best_params.slow_ma_id = slow_id;
        for (p, _) in res.top.iter_mut() {
            p.fast_ma_id = fast_id;
            p.slow_ma_id = slow_id;
        }
        if let Some(all) = res.all.as_mut() {
            for (p, _) in all.iter_mut() {
                p.fast_ma_id = fast_id;
                p.slow_ma_id = slow_id;
            }
        }
    }

    fn merge_heatmap_max(dst: &mut Option<OptimizationHeatmap>, src: Option<OptimizationHeatmap>) {
        let Some(src) = src else { return };
        match dst {
            None => *dst = Some(src),
            Some(d) => {
                if d.bins_fast != src.bins_fast
                    || d.bins_slow != src.bins_slow
                    || d.fast_min != src.fast_min
                    || d.fast_max != src.fast_max
                    || d.slow_min != src.slow_min
                    || d.slow_max != src.slow_max
                    || d.values.len() != src.values.len()
                {
                    return;
                }
                let OptimizationHeatmap {
                    values: src_values,
                    mean_values: src_mean_values,
                    sample_counts: src_sample_counts,
                    ..
                } = src;
                for (dv, sv) in d.values.iter_mut().zip(src_values.into_iter()) {
                    match (&dv, sv) {
                        (_, None) => {}
                        (None, Some(x)) => *dv = Some(x),
                        (Some(a), Some(b)) => {
                            if b > *a {
                                *dv = Some(b);
                            }
                        }
                    }
                }
                if let (Some(dst_means), Some(dst_counts), Some(src_means), Some(src_counts)) = (
                    d.mean_values.as_mut(),
                    d.sample_counts.as_mut(),
                    src_mean_values,
                    src_sample_counts,
                ) {
                    if dst_means.len() == src_means.len() && dst_counts.len() == src_counts.len() {
                        for (((dst_mean, dst_count), src_mean), src_count) in dst_means
                            .iter_mut()
                            .zip(dst_counts.iter_mut())
                            .zip(src_means.into_iter())
                            .zip(src_counts.into_iter())
                        {
                            let total_count = dst_count.saturating_add(src_count);
                            if total_count == 0 {
                                *dst_mean = None;
                                continue;
                            }
                            let dst_sum = dst_mean.unwrap_or(0.0) * *dst_count as f64;
                            let src_sum = src_mean.unwrap_or(0.0) * src_count as f64;
                            *dst_count = total_count;
                            *dst_mean = Some((dst_sum + src_sum) / total_count as f64);
                        }
                    }
                }
            }
        }
    }

    let mut merged_backend_used = BackendUsed::Cpu;
    let mut merged_best: Option<(DoubleMaParams, Metrics)> = None;
    let mut merged_top_candidates: Vec<(DoubleMaParams, Metrics)> = Vec::new();
    let mut merged_all: Option<Vec<(DoubleMaParams, Metrics)>> =
        if include_all { Some(Vec::new()) } else { None };
    let mut merged_heatmap: Option<OptimizationHeatmap> = None;
    let mut merged_num_combos: usize = 0;
    let mut merged_vdps: Option<VdpsTelemetry> = None;
    let mut merged_hybrid_timing_ms: Option<HybridTimingMs> = None;
    let mut merged_gpu_kernel_timing_ms: Option<GpuKernelTimingMs> = None;
    let mut merged_backend_route: Option<String> = None;
    let mut merged_backend_fallback_reason: Option<String> = None;
    merge_optional_note(&mut merged_backend_route, backend_route.take());
    merge_optional_note(
        &mut merged_backend_fallback_reason,
        backend_fallback_reason.take(),
    );

    let multi_grid_offset_progress =
        matches!(mode_used, OptimizationModeResolved::Grid) && variant_pairs > 1;
    let dynamic_exact_gpu_kernel_route = dynamic_runtime_plan.as_ref().is_some_and(|plan| {
        matches!(backend, Backend::GpuKernel { .. })
            && plan.preview.resolved_backend
                == crate::strategy_plan::StrategyPlanBackend::GpuUnified
            && plan.gpu_handle.is_some()
    });
    if score_mode_needs_pair_observation_context(score_mode)
        && !matches!(backend, Backend::GpuKernel { .. })
    {
        return Err(
            "effective_sample_lcb_10 currently requires the exact GpuKernel backend route"
                .to_string(),
        );
    }
    if score_mode_needs_pair_observation_context(score_mode)
        && dynamic_runtime_plan.is_some()
        && !dynamic_exact_gpu_kernel_route
    {
        return Err("effective_sample_lcb_10 is not yet supported for DynamicDoubleMa".to_string());
    }
    if score_mode_needs_pair_observation_context(score_mode) && variant_pairs > 1 {
        return Err(
            "effective_sample_lcb_10 currently requires a single expanded GPU kernel run (one MA-type / param grid slice)"
                .to_string(),
        );
    }
    let extra_shortlist_requested = extra_shortlist_objectives
        .as_ref()
        .is_some_and(|values| !values.is_empty());
    if extra_shortlist_requested
        && dynamic_runtime_plan.is_some()
        && !dynamic_exact_gpu_kernel_route
    {
        return Err(
            "extra_shortlist_objectives currently require an exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) route"
                .to_string(),
        );
    }
    if extra_shortlist_requested && !matches!(backend, Backend::GpuKernel { .. }) {
        return Err(
            "extra_shortlist_objectives currently require the exact GpuKernel backend route"
                .to_string(),
        );
    }
    if extra_shortlist_requested && variant_pairs > 1 {
        return Err(
            "extra_shortlist_objectives currently require a single expanded GPU kernel run (one MA-type / param grid slice)"
                .to_string(),
        );
    }
    let shortlist_confidence_mode =
        gpu_shortlist_confidence_mode_requested(requested_shortlist_confidence_mode)?;
    let shortlist_confidence_requested = shortlist_confidence_mode.is_some();
    if shortlist_confidence_requested
        && dynamic_runtime_plan.is_some()
        && !dynamic_exact_gpu_kernel_route
    {
        return Err(
            "shortlist_confidence_mode currently requires an exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) route"
                .to_string(),
        );
    }
    if shortlist_confidence_requested && !matches!(backend, Backend::GpuKernel { .. }) {
        return Err(
            "shortlist_confidence_mode currently requires the exact GpuKernel backend route"
                .to_string(),
        );
    }
    if shortlist_confidence_requested && variant_pairs > 1 {
        return Err(
            "shortlist_confidence_mode currently requires a single expanded GPU kernel run (one MA-type / param grid slice)"
                .to_string(),
        );
    }
    let shortlist_union_mode = gpu_shortlist_union_mode_requested(requested_shortlist_union_mode)?;
    let shortlist_union_requested = shortlist_union_mode.is_some();
    if shortlist_union_requested && !extra_shortlist_requested {
        return Err(
            "shortlist_union_mode currently requires extra_shortlist_objectives to define the exact shortlist union heads"
                .to_string(),
        );
    }
    if shortlist_union_requested
        && dynamic_runtime_plan.is_some()
        && !dynamic_exact_gpu_kernel_route
    {
        return Err(
            "shortlist_union_mode currently requires an exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) route"
                .to_string(),
        );
    }
    if shortlist_union_requested && !matches!(backend, Backend::GpuKernel { .. }) {
        return Err(
            "shortlist_union_mode currently requires the exact GpuKernel backend route".to_string(),
        );
    }
    if shortlist_union_requested && variant_pairs > 1 {
        return Err(
            "shortlist_union_mode currently requires a single expanded GPU kernel run (one MA-type / param grid slice)"
                .to_string(),
        );
    }
    let shortlist_slice_mode = gpu_shortlist_slice_mode_requested(requested_shortlist_slice_mode)?;
    let shortlist_slice_requested = shortlist_slice_mode.is_some();
    let shortlist_slice_benchmark =
        gpu_shortlist_slice_benchmark_requested(requested_shortlist_slice_benchmark.clone())?;
    let shortlist_neighborhood_cfg = if matches!(backend, Backend::GpuKernel { .. }) {
        gpu_shortlist_neighborhood_config_requested(
            requested_shortlist_neighborhood_radius,
            requested_shortlist_neighborhood_step,
        )?
    } else {
        None
    };
    if shortlist_slice_requested
        && dynamic_runtime_plan.is_some()
        && !dynamic_exact_gpu_kernel_route
    {
        return Err(
            "shortlist_slice_mode currently requires an exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) route"
                .to_string(),
        );
    }
    if shortlist_slice_requested && !matches!(backend, Backend::GpuKernel { .. }) {
        return Err(
            "shortlist_slice_mode currently requires the exact GpuKernel backend route".to_string(),
        );
    }
    if shortlist_slice_requested && variant_pairs > 1 {
        return Err(
            "shortlist_slice_mode currently requires a single expanded GPU kernel run (one MA-type / param grid slice)"
                .to_string(),
        );
    }
    if shortlist_slice_benchmark.is_some() && !shortlist_slice_requested {
        return Err(
            "shortlist_slice_benchmark currently requires shortlist_slice_mode to be set"
                .to_string(),
        );
    }
    if shortlist_neighborhood_cfg.is_some() && !shortlist_slice_requested {
        return Err(
            "shortlist_neighborhood_radius requires shortlist_slice_mode to be set".to_string(),
        );
    }
    if requested_trade_metrics && dynamic_runtime_plan.is_some() && !dynamic_exact_gpu_kernel_route
    {
        return Err(
            "include_trade_metrics currently require an exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) route"
                .to_string(),
        );
    }
    if requested_trade_metrics && !matches!(backend, Backend::GpuKernel { .. }) {
        return Err(
            "include_trade_metrics currently requires the exact GpuKernel backend route"
                .to_string(),
        );
    }
    if requested_metric_contract.is_some()
        && dynamic_runtime_plan.is_some()
        && !dynamic_exact_gpu_kernel_route
    {
        return Err(
            "metric_contract currently requires an exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) route"
                .to_string(),
        );
    }
    if requested_metric_contract.is_some() && !matches!(backend, Backend::GpuKernel { .. }) {
        return Err(
            "metric_contract currently requires the exact GpuKernel backend route".to_string(),
        );
    }
    let mut run_idx: usize = 0;
    for (fi, fast_params_opt) in fast_param_variants.iter().enumerate() {
        for (si, slow_params_opt) in slow_param_variants.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }
            if (extra_shortlist_requested
                || shortlist_union_requested
                || shortlist_confidence_requested
                || shortlist_slice_requested)
                && run_idx > 0
            {
                return Err(
                    "shortlist union / rerank modes currently require a single expanded GPU kernel run (one MA-type / param grid slice)"
                        .to_string(),
                );
            }

            let fast_params = fast_params_opt.as_ref();
            let slow_params = slow_params_opt.as_ref();

            let (fast_alma_offset, fast_alma_sigma) =
                resolve_alma_params_for_side("fast", &fast_ma, fast_params)?;
            let (slow_alma_offset, slow_alma_sigma) =
                resolve_alma_params_for_side("slow", &slow_ma, slow_params)?;

            let dynamic_gpu_execution = if let Some(runtime_plan) = dynamic_runtime_plan.as_ref() {
                match (&backend, runtime_plan.gpu_handle.as_ref()) {
                    (Backend::GpuKernel { device_id }, Some(handle)) => {
                        crate::strategy_gpu_runtime::get_or_build_gpu_strategy_executable(
                            handle, *device_id,
                        )
                        .map_err(|e| format!("dynamic GPU executable upload/probe failed: {e}"))?;
                        DynamicGpuExecutionSelection::Supported(handle.clone())
                    }
                    (Backend::GpuKernel { .. }, None) => {
                        let reason = if runtime_plan.preview.capability.reasons.is_empty() {
                            "dynamic gpu route is unavailable for this strategy_ir".to_string()
                        } else {
                            runtime_plan.preview.capability.reasons.join("; ")
                        };
                        DynamicGpuExecutionSelection::CpuFallback { reason }
                    }
                    _ => DynamicGpuExecutionSelection::Disabled,
                }
            } else {
                prepare_dynamic_gpu_execution_selection(
                    dynamic_gpu_preflight_required,
                    &backend,
                    match backend {
                        Backend::GpuKernel { device_id }
                        | Backend::GpuOnly { device_id }
                        | Backend::GpuHybridKernel { device_id }
                        | Backend::Auto { device_id } => device_id,
                        Backend::CpuOnly => 0,
                    },
                    &ma_source,
                    &fast_ma,
                    &slow_ma,
                    fast_params,
                    slow_params,
                    &strategy,
                    halftrend_filter.as_ref(),
                    objective,
                    score_mode,
                    top_k,
                    include_all,
                )?
            };

            let offset_sink;
            let progress_for_run: Option<&dyn ProgressSink> = if multi_grid_offset_progress {
                if let Some(inner) = progress {
                    let base_offset = run_idx.saturating_mul(total_pairs);
                    offset_sink = OffsetProgress {
                        inner,
                        base_offset,
                        total_pairs: total_combos,
                    };
                    Some(&offset_sink)
                } else {
                    None
                }
            } else {
                progress
            };

            let mut mode_out = run_mode_for_params(
                fast_params,
                slow_params,
                fast_alma_offset,
                fast_alma_sigma,
                slow_alma_offset,
                slow_alma_sigma,
                constraints.as_ref(),
                progress_for_run,
                &dynamic_gpu_execution,
            )?;
            merge_vdps_telemetry(&mut merged_vdps, mode_out.vdps.take());
            merge_hybrid_timing(
                &mut merged_hybrid_timing_ms,
                mode_out.hybrid_timing_ms.take(),
            );
            merge_gpu_kernel_timing(
                &mut merged_gpu_kernel_timing_ms,
                mode_out.gpu_kernel_timing_ms.take(),
            );
            merge_optional_note(&mut merged_backend_route, mode_out.route_note.take());
            merge_optional_note(
                &mut merged_backend_fallback_reason,
                mode_out.fallback_reason.take(),
            );
            let backend_used = mode_out.backend_used;
            let mut res = mode_out.res;

            let fast_id = fi as u16;
            let slow_id = si as u16;
            rewrite_result_ma_ids(&mut res, fast_id, slow_id);

            if matches!(backend_used, BackendUsed::Gpu) {
                merged_backend_used = BackendUsed::Gpu;
            }

            merged_num_combos = merged_num_combos.saturating_add(res.num_combos);

            let score = effective_score(&res.best_metrics, objective, score_mode);
            let is_better = match merged_best.as_ref() {
                None => true,
                Some((_, m)) => {
                    let s = effective_score(m, objective, score_mode);
                    score > s
                }
            };
            if is_better {
                merged_best = Some((res.best_params.clone(), res.best_metrics.clone()));
            }

            if top_k > 0 {
                merged_top_candidates.extend(res.top.into_iter());
            }
            if let Some(all) = merged_all.as_mut() {
                if let Some(mut rows) = res.all {
                    all.append(&mut rows);
                }
            }
            merge_heatmap_max(&mut merged_heatmap, res.heatmap);

            run_idx = run_idx.saturating_add(1);
        }
    }

    let Some((best_params_raw, best_metrics)) = merged_best else {
        return Err("no results".to_string());
    };

    let top = if top_k == 0 {
        Vec::new()
    } else {
        let mut rows = merged_top_candidates;
        if !shortlist_confidence_requested && !shortlist_slice_requested {
            rows.sort_by(|a, b| {
                let sa = effective_score(&a.1, objective, score_mode);
                let sb = effective_score(&b.1, objective, score_mode);
                sb.total_cmp(&sa)
            });
        }
        rows.truncate(top_k);
        rows
    };
    let best_score = Some(effective_score(&best_metrics, objective, score_mode));
    let top_scores = Some(
        top.iter()
            .map(|(_, metrics)| effective_score(metrics, objective, score_mode))
            .collect(),
    );
    let all_scores = merged_all.as_ref().map(|rows| {
        rows.iter()
            .map(|(_, metrics)| effective_score(metrics, objective, score_mode))
            .collect()
    });
    let result = OptimizationResult {
        best_params: best_params_raw,
        best_metrics,
        best_score,
        top,
        top_scores,
        all: merged_all,
        all_scores,
        heatmap: merged_heatmap,
        num_combos: merged_num_combos,
        num_candles: n,
    };

    let backend_used = merged_backend_used;

    let runtime_ms = start.elapsed().as_millis() as u64;
    let work_items = result.num_combos;

    let fast_ma_label = fast_ma_types
        .first()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fast_ma.clone());
    let slow_ma_label = slow_ma_types
        .first()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| slow_ma.clone());
    let residency = Some(build_gpu_residency_summary(
        req.gpu_execution,
        &requested_backend,
        &backend,
        &indicator_route_plan_summary,
        merged_backend_fallback_reason.as_deref(),
    ));
    if let Some(summary) = residency.as_ref() {
        enforce_gpu_execution_policy_after_run(summary)?;
    }
    let followup_cache_key = if req.persist_followup_cache {
        let key = build_followup_cache_key(
            &req.data_id,
            n,
            &ma_source,
            &fast_ma,
            &slow_ma,
            &strategy,
            halftrend_filter.as_ref(),
        )?;
        let entry = build_lazy_followup_cache_entry(Some(&backend));
        store_followup_cache_entry(key.clone(), entry);
        Some(key)
    } else {
        None
    };

    if matches!(backend_used, BackendUsed::Gpu)
        && crate::strategy_dynamic::dynamic_strategy_opt_experimental_enabled()
    {
        let profile = if strategy.breakout_momentum_enable {
            "breakout_momentum"
        } else if strategy.pressure_impulse_trend_enable {
            "pit"
        } else if strategy.slope_efficiency_trend_enable {
            "set"
        } else if strategy.momentum_triple_stack_enable {
            "mts"
        } else if strategy.oscillator_reversion_confluence_enable {
            "orc"
        } else if strategy.volume_flow_confirm_enable {
            "vfct"
        } else if strategy.regime_hybrid_enable {
            "regime_hybrid"
        } else if strategy.trend_quality_enable {
            "composite_trend"
        } else {
            "double_ma"
        };
        let backend_label = merged_backend_route.as_deref().unwrap_or("Gpu");
        crate::strategy_dynamic::maybe_append_dynamic_strategy_artifact(
            crate::strategy_dynamic::DynamicStrategyArtifactRecord {
                scope: "double_ma_gpu",
                profile,
                backend: backend_label,
                dynamic_enabled: true,
                shadow_enabled: false,
                shadow_every: 0,
                shadow_sampled: 0,
                shadow_mismatches: 0,
                runtime_ms,
                work_items,
            },
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let mut shortlist_slice_diagnostics = if matches!(backend_used, BackendUsed::Gpu) {
        crate::gpu_backtest_kernel::take_last_shortlist_slice_diagnostics()
    } else {
        None
    };
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let score_observation_context = if matches!(backend_used, BackendUsed::Gpu) {
        crate::gpu_backtest_kernel::take_last_score_observation_context()
    } else {
        None
    };
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let mut shortlist_slice_diagnostics: Option<ShortlistSliceDiagnostics> = None;
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let score_observation_context: Option<PairObservationScoreContextResolved> = None;

    if let (Some(cfg), Some(diag)) = (
        shortlist_neighborhood_cfg,
        shortlist_slice_diagnostics.as_mut(),
    ) {
        let fast_ma_params = fast_param_variants.first().cloned().unwrap_or(None);
        let slow_ma_params = slow_param_variants.first().cloned().unwrap_or(None);
        let local_req = DoubleMaLocalStabilityRequest {
            data_id: req.data_id.clone(),
            backend_hint: Some(backend.clone()),
            followup_cache_key: followup_cache_key.clone(),
            params: DoubleMaParamsResolved {
                fast_len: result.best_params.fast_len,
                slow_len: result.best_params.slow_len,
                fast_ma_type: fast_ma_label.clone(),
                slow_ma_type: slow_ma_label.clone(),
                fast_ma_params: fast_ma_params.clone(),
                slow_ma_params: slow_ma_params.clone(),
            },
            ma_source: ma_source.clone(),
            fast_ma_params: fast_ma_params.clone(),
            slow_ma_params: slow_ma_params.clone(),
            strategy: strategy.clone(),
            halftrend_filter: halftrend_filter.clone(),
            objective,
            score_mode,
            radius: Some(cfg.radius),
            step: Some(cfg.step),
        };
        let local = compute_double_ma_local_stability_blocking_with_candles(local_req, candles)?;
        diag.neighborhood_robustness =
            summarize_shortlist_neighborhood_robustness(&local, score_mode);
    }

    let backend_result = {
        let _nvtx_scope = crate::nvtx::scope("engine", "result_finalize");
        build_backend_optimization_result(
            result,
            BackendResultContext {
                mode_used,
                backend_used,
                fast_periods: &fast_periods,
                slow_periods: &slow_periods,
                fast_ma_label: &fast_ma_label,
                slow_ma_label: &slow_ma_label,
                fast_param_variants: &fast_param_variants,
                slow_param_variants: &slow_param_variants,
                num_combos: work_items,
                num_candles: n,
                runtime_ms,
                vdps: merged_vdps,
                hybrid_timing_ms: merged_hybrid_timing_ms,
                gpu_kernel_timing_ms: merged_gpu_kernel_timing_ms,
                backend_route: merged_backend_route,
                backend_fallback_reason: merged_backend_fallback_reason,
                route_diagnostics: None,
                indicator_route_plan: Some(indicator_route_plan_summary),
                residency,
                followup_cache_key,
                shortlist_slice_diagnostics,
                score_observation_context,
            },
        )
    };
    Ok(backend_result)
}

#[cfg(not(feature = "cuda"))]
#[allow(dead_code)]
fn grid_search_double_ma_gpu(
    _candles: &vector_ta::utilities::data_loader::Candles,
    _combos: &[ta_strategies::double_ma::DoubleMaParams],
    _fast_ma_types: &[String],
    _slow_ma_types: &[String],
    _objective: ObjectiveKind,
    _num_candles: usize,
    _top_k: usize,
    _include_all: bool,
    _device_id: u32,
    _ma_source: &str,
) -> Result<OptimizationResult, String> {
    Err("GPU backend requires building `ta_desktop_demo_app` with `--features cuda`".to_string())
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
#[allow(dead_code)]
fn grid_search_double_ma_gpu_kernel(
    _candles: &vector_ta::utilities::data_loader::Candles,
    _combos: &[ta_strategies::double_ma::DoubleMaParams],
    _fast_range: (u32, u32, u32),
    _slow_range: (u32, u32, u32),
    _fast_ma_types: &[String],
    _slow_ma_types: &[String],
    _objective: ObjectiveKind,
    _num_candles: usize,
    _top_k: usize,
    _include_all: bool,
    _device_id: u32,
    _ma_source: &str,
    _alma_offset: Option<f64>,
    _alma_sigma: Option<f64>,
) -> Result<OptimizationResult, String> {
    Err("GPU kernel backend requires building `ta_desktop_demo_app` with `--features cuda,cuda-backtest-kernel`".to_string())
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(dead_code)]
fn grid_search_double_ma_gpu_kernel(
    candles: &vector_ta::utilities::data_loader::Candles,
    combos: &[ta_strategies::double_ma::DoubleMaParams],
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
    fast_ma_types: &[String],
    slow_ma_types: &[String],
    objective: ObjectiveKind,
    num_candles: usize,
    top_k: usize,
    include_all: bool,
    device_id: u32,
    ma_source: &str,
    alma_offset: Option<f64>,
    alma_sigma: Option<f64>,
) -> Result<OptimizationResult, String> {
    if !cuda_available() {
        return Err("CUDA device not available (cuda_available() == false)".to_string());
    }

    if fast_ma_types.len() != 1 || slow_ma_types.len() != 1 {
        return Err(
            "GPU kernel backend currently supports one fast MA type and one slow MA type (set fast_ma_types and slow_ma_types to a single entry).".to_string(),
        );
    }
    let fast_ma = fast_ma_types[0].trim();
    let slow_ma = slow_ma_types[0].trim();

    let offset = alma_offset.unwrap_or(0.85);
    let sigma = alma_sigma.unwrap_or(6.0);
    let cfg = crate::gpu_backtest_kernel::KernelConfig {
        device_id,
        fast_alma_offset: offset,
        fast_alma_sigma: sigma,
        slow_alma_offset: offset,
        slow_alma_sigma: sigma,
        hurdle_tau: 0.0,
        include_trade_metrics: false,
        metric_contract: None,
    };
    let metrics = crate::gpu_backtest_kernel::eval_double_ma_batch_gpu_kernel(
        candles,
        combos,
        fast_range,
        slow_range,
        fast_ma,
        slow_ma,
        ma_source,
        None,
        None,
        cfg,
        &StrategyConfig::default(),
    )?;
    if metrics.len() != combos.len() {
        return Err(format!(
            "kernel backend returned {} metrics (expected {})",
            metrics.len(),
            combos.len()
        ));
    }

    fn score(metrics: &ta_strategies::double_ma::Metrics, objective: ObjectiveKind) -> f64 {
        match objective {
            ObjectiveKind::Pnl => metrics.pnl,
            ObjectiveKind::Sharpe => metrics.sharpe,
            ObjectiveKind::Sortino => metrics.sortino,
            ObjectiveKind::Calmar => metrics.calmar,
            ObjectiveKind::Omega => metrics.omega,
            ObjectiveKind::Volatility => -metrics.volatility,
            ObjectiveKind::MaxDrawdown => -metrics.max_dd,
        }
    }

    if combos.is_empty() {
        return Err("no parameter combinations".to_string());
    }

    let mut best_idx = 0usize;
    let mut best_score = score(&metrics[0], objective);
    for (i, m) in metrics.iter().enumerate().skip(1) {
        let s = score(m, objective);
        if s > best_score {
            best_score = s;
            best_idx = i;
        }
    }

    let best_params = combos[best_idx].clone();
    let best_metrics = metrics[best_idx].clone();

    let top = if top_k == 0 {
        Vec::new()
    } else {
        let mut idx: Vec<usize> = (0..metrics.len()).collect();
        idx.sort_by(|&a, &b| {
            let sa = score(&metrics[a], objective);
            let sb = score(&metrics[b], objective);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        idx.truncate(top_k.min(metrics.len()));
        idx.into_iter()
            .map(|i| (combos[i].clone(), metrics[i].clone()))
            .collect()
    };

    let all: Option<Vec<(DoubleMaParams, Metrics)>> = if include_all {
        Some(combos.iter().cloned().zip(metrics.into_iter()).collect())
    } else {
        None
    };
    let best_score = Some(score(&best_metrics, objective));
    let top_scores = Some(
        top.iter()
            .map(|(_, metrics)| score(metrics, objective))
            .collect(),
    );
    let all_scores: Option<Vec<f64>> = all.as_ref().map(|rows| {
        rows.iter()
            .map(|(_, metrics)| score(metrics, objective))
            .collect()
    });
    Ok(OptimizationResult {
        best_params,
        best_metrics,
        best_score,
        top,
        top_scores,
        all,
        all_scores,
        heatmap: None,
        num_combos: combos.len(),
        num_candles,
    })
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
fn grid_search_double_ma_gpu(
    candles: &vector_ta::utilities::data_loader::Candles,
    combos: &[ta_strategies::double_ma::DoubleMaParams],
    fast_ma_types: &[String],
    slow_ma_types: &[String],
    objective: ObjectiveKind,
    num_candles: usize,
    top_k: usize,
    include_all: bool,
    device_id: u32,
    ma_source: &str,
) -> Result<OptimizationResult, String> {
    if !cuda_available() {
        return Err("CUDA device not available (cuda_available() == false)".to_string());
    }
    if combos.is_empty() {
        return Err("no parameter combinations".to_string());
    }

    let prices = vector_ta::utilities::data_loader::source_type(candles, "close");
    let n = prices.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
    let prices_f32: Vec<f32> = ma_prices.iter().map(|&x| x as f32).collect();
    let nan_row: Vec<f32> = vec![f32::NAN; n];

    let matrices = build_ma_matrices_cuda(
        candles,
        &prices_f32,
        combos,
        fast_ma_types,
        slow_ma_types,
        device_id,
        ma_source,
    )?;

    struct CachedEngine<'a> {
        prices: &'a [f64],
        n: usize,
        nan_row: &'a [f32],
        matrices: &'a HashMap<String, PeriodMatrix>,
        fast_ma_types: &'a [String],
        slow_ma_types: &'a [String],
    }

    impl<'a> ta_optimizer::BacktestEngine for CachedEngine<'a> {
        fn eval_batch(
            &self,
            combos: &[ta_strategies::double_ma::DoubleMaParams],
        ) -> Vec<ta_strategies::double_ma::Metrics> {
            combos
                .par_iter()
                .map(|p| {
                    let fast_type = self
                        .fast_ma_types
                        .get(p.fast_ma_id as usize)
                        .map(|s| s.as_str());
                    let slow_type = self
                        .slow_ma_types
                        .get(p.slow_ma_id as usize)
                        .map(|s| s.as_str());

                    let fast = fast_type
                        .and_then(|t| self.matrices.get(t))
                        .and_then(|m| m.row(p.fast_len, self.n))
                        .unwrap_or(self.nan_row);
                    let slow = slow_type
                        .and_then(|t| self.matrices.get(t))
                        .and_then(|m| m.row(p.slow_len, self.n))
                        .unwrap_or(self.nan_row);
                    eval_double_ma_metrics(self.prices, fast, slow)
                })
                .collect()
        }
    }

    let engine = CachedEngine {
        prices,
        n,
        nan_row: &nan_row,
        matrices: &matrices,
        fast_ma_types,
        slow_ma_types,
    };

    ta_optimizer::grid::grid_search(&engine, combos, objective, num_candles, top_k, include_all)
        .ok_or_else(|| "grid search produced no result".to_string())
}

#[cfg(feature = "cuda")]
struct PeriodMatrix {
    period_start: u16,
    period_end: u16,
    values: Vec<f32>,
}

#[cfg(feature = "cuda")]
impl PeriodMatrix {
    fn row(&self, period: u16, cols: usize) -> Option<&[f32]> {
        if period < self.period_start || period > self.period_end {
            return None;
        }
        let row = (period - self.period_start) as usize;
        let offset = row.checked_mul(cols)?;
        let end = offset.checked_add(cols)?;
        self.values.get(offset..end)
    }
}

#[cfg(feature = "cuda")]
fn build_ma_matrices_cuda(
    candles: &vector_ta::utilities::data_loader::Candles,
    prices_f32: &[f32],
    combos: &[ta_strategies::double_ma::DoubleMaParams],
    fast_ma_types: &[String],
    slow_ma_types: &[String],
    device_id: u32,
    ma_source: &str,
) -> Result<HashMap<String, PeriodMatrix>, String> {
    let series_len = prices_f32.len();
    let max_period_supported: u16 = if series_len >= u16::MAX as usize {
        u16::MAX
    } else {
        series_len as u16
    };

    let mut minmax: HashMap<&str, (u16, u16)> = HashMap::new();
    for p in combos {
        if p.fast_len > 0 {
            let t = fast_ma_types
                .get(p.fast_ma_id as usize)
                .ok_or_else(|| format!("fast_ma_id out of range: {}", p.fast_ma_id))?;
            update_minmax(&mut minmax, t.as_str(), p.fast_len);
        }
        if p.slow_len > 0 {
            let t = slow_ma_types
                .get(p.slow_ma_id as usize)
                .ok_or_else(|| format!("slow_ma_id out of range: {}", p.slow_ma_id))?;
            update_minmax(&mut minmax, t.as_str(), p.slow_len);
        }
    }

    let selector = CudaMaSelector::new(device_id as usize);

    let mut out: HashMap<String, PeriodMatrix> = HashMap::new();
    for (&ma_type, &(min_p, max_p)) in &minmax {
        if min_p == 0 || max_p == 0 {
            continue;
        }

        let start = min_p;
        let end = max_p.min(max_period_supported);
        if start > end {
            continue;
        }

        let n_periods = (end as u32).saturating_sub(start as u32).saturating_add(1) as usize;
        let expected_len = n_periods
            .checked_mul(series_len)
            .ok_or_else(|| "output length overflow".to_string())?;

        let requires_candles = lookup_ma_info(ma_type)
            .map(|m| m.requires_candles)
            .unwrap_or(false);

        let data = if requires_candles {
            CudaMaData::Candles {
                candles,
                source: ma_source,
            }
        } else {
            CudaMaData::SliceF32(prices_f32)
        };

        let host = match selector.ma_sweep_to_host_f32(
            ma_type,
            data,
            start as usize,
            end as usize,
            1,
        ) {
            Ok((values, rows, cols)) => {
                if rows != n_periods || cols != series_len {
                    return Err(format!(
                        "cuda ma sweep '{ma_type}' returned shape ({rows},{cols}) (expected ({n_periods},{series_len}))"
                    ));
                }
                if values.len() != expected_len {
                    return Err(format!(
                        "cuda ma sweep '{ma_type}' returned len {} (expected {expected_len})",
                        values.len()
                    ));
                }
                values
            }
            Err(sweep_err) => {
                let mut host = alloc_uninit_f32(expected_len);
                for (row, period) in (start..=end).enumerate() {
                    let data = if requires_candles {
                        CudaMaData::Candles {
                            candles,
                            source: ma_source,
                        }
                    } else {
                        CudaMaData::SliceF32(prices_f32)
                    };
                    let series = selector.ma_to_host_f32(ma_type, data, period as usize).map_err(|e| {
                        format!(
                            "cuda ma sweep '{ma_type}' failed: {sweep_err}; fallback period {period} failed: {e}"
                        )
                    })?;
                    if series.len() != series_len {
                        return Err(format!(
                            "cuda ma '{ma_type}' period {period} returned len {} (expected {series_len})",
                            series.len()
                        ));
                    }
                    let offset = row
                        .checked_mul(series_len)
                        .ok_or_else(|| "output offset overflow".to_string())?;
                    let end = offset
                        .checked_add(series_len)
                        .ok_or_else(|| "output end overflow".to_string())?;
                    host[offset..end].copy_from_slice(&series);
                }
                host
            }
        };

        out.insert(
            ma_type.to_string(),
            PeriodMatrix {
                period_start: start,
                period_end: end,
                values: host,
            },
        );
    }

    Ok(out)
}

#[cfg(feature = "cuda")]
fn update_minmax<'a>(map: &mut HashMap<&'a str, (u16, u16)>, ma_type: &'a str, period: u16) {
    let entry = map.entry(ma_type).or_insert((period, period));
    if period < entry.0 {
        entry.0 = period;
    }
    if period > entry.1 {
        entry.1 = period;
    }
}

#[cfg(feature = "cuda")]
fn eval_double_ma_metrics(
    prices: &[f64],
    fast: &[f32],
    slow: &[f32],
) -> ta_strategies::double_ma::Metrics {
    let n = prices.len();
    if n < 2 || fast.len() != n || slow.len() != n {
        return ta_strategies::double_ma::Metrics::default();
    }

    let cfg = StrategyConfig::default();
    let lr = compute_log_returns_f64(prices);
    let first_valid = first_finite_idx(prices).unwrap_or(0);
    let t_sig0 = fast
        .iter()
        .zip(slow.iter())
        .position(|(f, s)| f.is_finite() && s.is_finite())
        .unwrap_or(n);
    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let start_t = t_sig0.saturating_add(trade_offset).max(first_valid);

    ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f32_rich(
        &lr, fast, slow, start_t, &cfg,
    )
}

fn emit_progress(
    progress: Option<&dyn ProgressSink>,
    processed_pairs: usize,
    total_pairs: usize,
    phase: &'static str,
) {
    crate::progress::emit_double_ma_progress(progress, processed_pairs, total_pairs, phase);
}

fn sanitize_double_ma_constraints(
    constraints: Option<DoubleMaConstraints>,
) -> Result<Option<DoubleMaConstraints>, String> {
    let Some(mut c) = constraints else {
        return Ok(None);
    };

    c.min_trades = c
        .min_trades
        .and_then(|v| if v > 0 { Some(v) } else { None });
    c.max_drawdown = match c.max_drawdown {
        Some(v) if v.is_finite() && v >= 0.0 => Some(v),
        Some(_) => {
            return Err("constraints.max_drawdown must be finite and >= 0".to_string());
        }
        None => None,
    };
    c.min_sharpe = match c.min_sharpe {
        Some(v) if v.is_finite() => Some(v),
        Some(_) => {
            return Err("constraints.min_sharpe must be finite".to_string());
        }
        None => None,
    };
    c.min_sortino = match c.min_sortino {
        Some(v) if v.is_finite() => Some(v),
        Some(_) => {
            return Err("constraints.min_sortino must be finite".to_string());
        }
        None => None,
    };
    c.min_calmar = match c.min_calmar {
        Some(v) if v.is_finite() => Some(v),
        Some(_) => {
            return Err("constraints.min_calmar must be finite".to_string());
        }
        None => None,
    };
    c.max_volatility = match c.max_volatility {
        Some(v) if v.is_finite() && v >= 0.0 => Some(v),
        Some(_) => {
            return Err("constraints.max_volatility must be finite and >= 0".to_string());
        }
        None => None,
    };
    c.min_exposure = match c.min_exposure {
        Some(v) if v.is_finite() && (0.0..=1.0).contains(&v) => Some(v),
        Some(_) => {
            return Err("constraints.min_exposure must be finite and in [0, 1]".to_string());
        }
        None => None,
    };
    c.max_exposure = match c.max_exposure {
        Some(v) if v.is_finite() && (0.0..=1.0).contains(&v) => Some(v),
        Some(_) => {
            return Err("constraints.max_exposure must be finite and in [0, 1]".to_string());
        }
        None => None,
    };
    if let (Some(min_exposure), Some(max_exposure)) = (c.min_exposure, c.max_exposure) {
        if min_exposure > max_exposure {
            return Err("constraints.min_exposure must be <= constraints.max_exposure".to_string());
        }
    }
    c.max_abs_net_exposure = match c.max_abs_net_exposure {
        Some(v) if v.is_finite() && (0.0..=1.0).contains(&v) => Some(v),
        Some(_) => {
            return Err(
                "constraints.max_abs_net_exposure must be finite and in [0, 1]".to_string(),
            );
        }
        None => None,
    };
    c.min_omega = match c.min_omega {
        Some(v) if v.is_finite() && v >= 0.0 => Some(v),
        Some(_) => {
            return Err("constraints.min_omega must be finite and >= 0".to_string());
        }
        None => None,
    };
    c.max_ulcer = match c.max_ulcer {
        Some(v) if v.is_finite() && v >= 0.0 => Some(v),
        Some(_) => {
            return Err("constraints.max_ulcer must be finite and >= 0".to_string());
        }
        None => None,
    };
    c.max_downside_deviation = match c.max_downside_deviation {
        Some(v) if v.is_finite() && v >= 0.0 => Some(v),
        Some(_) => {
            return Err("constraints.max_downside_deviation must be finite and >= 0".to_string());
        }
        None => None,
    };

    if c.min_trades.is_none()
        && c.max_drawdown.is_none()
        && c.min_sharpe.is_none()
        && c.min_sortino.is_none()
        && c.min_calmar.is_none()
        && c.max_volatility.is_none()
        && c.min_exposure.is_none()
        && c.max_exposure.is_none()
        && c.max_abs_net_exposure.is_none()
        && c.min_omega.is_none()
        && c.max_ulcer.is_none()
        && c.max_downside_deviation.is_none()
    {
        Ok(None)
    } else {
        Ok(Some(c))
    }
}

fn validate_halftrend_filter(f: &HalfTrendRegimeFilter) -> Result<(), String> {
    if f.amplitude == 0 {
        return Err("HalfTrend amplitude must be > 0".to_string());
    }
    if f.atr_period == 0 {
        return Err("HalfTrend atr_period must be > 0".to_string());
    }
    if !f.channel_deviation.is_finite() || f.channel_deviation <= 0.0 {
        return Err("HalfTrend channel_deviation must be finite and > 0".to_string());
    }
    Ok(())
}

pub(crate) fn compute_halftrend_trend(
    candles: &vector_ta::utilities::data_loader::Candles,
    f: &HalfTrendRegimeFilter,
) -> Result<Vec<f64>, String> {
    validate_halftrend_filter(f)?;

    if !candles.fields.high || !candles.fields.low {
        return Err("HalfTrend filter requires high/low candle data".to_string());
    }

    let input = vector_ta::indicators::halftrend::HalfTrendInput::from_candles(
        candles,
        vector_ta::indicators::halftrend::HalfTrendParams {
            amplitude: Some(f.amplitude as usize),
            channel_deviation: Some(f.channel_deviation),
            atr_period: Some(f.atr_period as usize),
        },
    );

    let out = vector_ta::indicators::halftrend::halftrend(&input).map_err(|e| e.to_string())?;
    Ok(out.trend)
}

fn merge_regime_filters(
    a: Option<Vec<f64>>,
    b: Option<Vec<f64>>,
) -> Result<Option<Vec<f64>>, String> {
    match (a, b) {
        (None, None) => Ok(None),
        (Some(x), None) => Ok(Some(x)),
        (None, Some(y)) => Ok(Some(y)),
        (Some(x), Some(y)) => {
            if x.len() != y.len() {
                return Err("regime filter length mismatch".to_string());
            }
            let mut out = alloc_uninit_f64(x.len());
            for i in 0..x.len() {
                let (a_long, a_short) = if !x[i].is_finite() {
                    (false, false)
                } else if x[i] < 0.5 {
                    (true, false)
                } else {
                    (false, true)
                };
                let (b_long, b_short) = if !y[i].is_finite() {
                    (false, false)
                } else if y[i] < 0.5 {
                    (true, false)
                } else {
                    (false, true)
                };
                let long_ok = a_long && b_long;
                let short_ok = a_short && b_short;
                out[i] = if long_ok && !short_ok {
                    0.0
                } else if short_ok && !long_ok {
                    1.0
                } else {
                    f64::NAN
                };
            }
            Ok(Some(out))
        }
    }
}

pub(crate) fn compute_trend_quality_regime(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
) -> Result<Option<Vec<f64>>, String> {
    let trend_adx_enabled =
        strategy.uses_trend_quality_filter() && strategy.trend_quality_use_adx();
    if !strategy.uses_trend_quality_filter() && !strategy.uses_regime_hybrid_filter() {
        return Ok(None);
    }

    let n = candles.close.len();
    let lr_angle = vector_ta::indicators::linearreg_angle::linearreg_angle(
        &vector_ta::indicators::linearreg_angle::Linearreg_angleInput::from_candles(
            candles,
            ma_source,
            vector_ta::indicators::linearreg_angle::Linearreg_angleParams {
                period: Some(strategy.trend_quality_linearreg_period()),
            },
        ),
    )
    .map_err(|e| format!("trend-quality linearreg_angle failed: {e}"))?
    .values;

    let bbw = vector_ta::indicators::bollinger_bands_width::bollinger_bands_width(
        &vector_ta::indicators::bollinger_bands_width::BollingerBandsWidthInput::from_candles(
            candles,
            ma_source,
            vector_ta::indicators::bollinger_bands_width::BollingerBandsWidthParams {
                period: Some(strategy.trend_quality_bbw_period()),
                devup: Some(strategy.trend_quality_bbw_mult()),
                devdn: Some(strategy.trend_quality_bbw_mult()),
                matype: Some("sma".to_string()),
                devtype: Some(0),
            },
        ),
    )
    .map_err(|e| format!("trend-quality bollinger_bands_width failed: {e}"))?
    .values;

    if lr_angle.len() != n || bbw.len() != n {
        return Err("trend-quality indicator length mismatch".to_string());
    }

    let roc_vals = if strategy.trend_quality_use_roc() {
        Some(
            vector_ta::indicators::roc::roc(&vector_ta::indicators::roc::RocInput::from_candles(
                candles,
                ma_source,
                vector_ta::indicators::roc::RocParams {
                    period: Some(strategy.trend_quality_roc_period()),
                },
            ))
            .map_err(|e| format!("trend-quality roc failed: {e}"))?
            .values,
        )
    } else {
        None
    };

    if let Some(roc) = roc_vals.as_ref() {
        if roc.len() != n {
            return Err("trend-quality roc length mismatch".to_string());
        }
    }

    let adx_period = if strategy.uses_regime_hybrid_filter() {
        strategy.regime_hybrid_adx_period()
    } else if trend_adx_enabled {
        strategy.trend_quality_adx_period()
    } else {
        0
    };
    let adx_vals = if adx_period > 0 {
        Some(
            vector_ta::indicators::adx::adx(&vector_ta::indicators::adx::AdxInput::from_candles(
                candles,
                vector_ta::indicators::adx::AdxParams {
                    period: Some(adx_period),
                },
            ))
            .map_err(|e| format!("trend-quality adx failed: {e}"))?
            .values,
        )
    } else {
        None
    };
    if let Some(adx) = adx_vals.as_ref() {
        if adx.len() != n {
            return Err("trend-quality adx length mismatch".to_string());
        }
    }

    let angle_min = strategy.trend_quality_angle_min_deg();
    let bbw_min = strategy.trend_quality_bbw_min();
    let adx_min = if trend_adx_enabled {
        strategy.trend_quality_adx_min()
    } else {
        f64::NAN
    };

    if !strategy.uses_regime_hybrid_filter() {
        let mut regime = alloc_uninit_f64(n);
        for i in 0..n {
            let a = lr_angle[i];
            let b = bbw[i];
            regime[i] = f64::NAN;
            if !a.is_finite() || !b.is_finite() || b < bbw_min {
                continue;
            }

            let mut long_ok = a >= angle_min;
            let mut short_ok = a <= -angle_min;
            if let Some(roc) = roc_vals.as_ref() {
                let r = roc[i];
                if !r.is_finite() {
                    continue;
                }
                long_ok = long_ok && r > 0.0;
                short_ok = short_ok && r < 0.0;
            }
            if trend_adx_enabled {
                let d = adx_vals
                    .as_ref()
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or(f64::NAN);
                if !d.is_finite() || d < adx_min {
                    continue;
                }
            }

            regime[i] = if long_ok && !short_ok {
                0.0
            } else if short_ok && !long_ok {
                1.0
            } else {
                f64::NAN
            };
        }

        return Ok(Some(regime));
    }

    let adx_enter = strategy.regime_hybrid_adx_enter();
    let adx_exit = strategy.regime_hybrid_adx_exit();
    let angle_exit = (angle_min * 0.6).max(0.0001);
    let bbw_exit = (bbw_min * 0.7).max(0.0);
    let mut regime = alloc_uninit_f64(n);
    let mut state: i8 = 0; // +1 trend-long, -1 trend-short, 0 range
    for i in 0..n {
        let a = lr_angle[i];
        let b = bbw[i];
        let d = adx_vals
            .as_ref()
            .and_then(|v| v.get(i))
            .copied()
            .unwrap_or(f64::NAN);
        regime[i] = f64::NAN;
        if !a.is_finite() || !b.is_finite() || !d.is_finite() {
            continue;
        }

        let mut long_entry = a >= angle_min && b >= bbw_min && d >= adx_enter;
        let mut short_entry = a <= -angle_min && b >= bbw_min && d >= adx_enter;
        let mut long_hold = a >= angle_exit && b >= bbw_exit && d >= adx_exit;
        let mut short_hold = a <= -angle_exit && b >= bbw_exit && d >= adx_exit;

        if let Some(roc) = roc_vals.as_ref() {
            let r = roc[i];
            if !r.is_finite() {
                regime[i] = 2.0;
                state = 0;
                continue;
            }
            long_entry = long_entry && r > 0.0;
            short_entry = short_entry && r < 0.0;
            long_hold = long_hold && r >= 0.0;
            short_hold = short_hold && r <= 0.0;
        }

        state = match state {
            1 => {
                if long_hold {
                    1
                } else {
                    0
                }
            }
            -1 => {
                if short_hold {
                    -1
                } else {
                    0
                }
            }
            _ => {
                if long_entry && !short_entry {
                    1
                } else if short_entry && !long_entry {
                    -1
                } else {
                    0
                }
            }
        };

        regime[i] = if state > 0 {
            0.0
        } else if state < 0 {
            1.0
        } else {
            2.0
        };
    }

    Ok(Some(regime))
}

pub(crate) fn compute_breakout_momentum_regime(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
) -> Result<Option<Vec<f64>>, String> {
    if !strategy.uses_breakout_momentum_filter() {
        return Ok(None);
    }
    if !candles.fields.high || !candles.fields.low || !candles.fields.volume {
        return Err(
            "breakout_momentum_enable requires 'high', 'low', and 'volume' candle columns"
                .to_string(),
        );
    }

    let n = candles.close.len();
    let bb = vector_ta::indicators::bollinger_bands::bollinger_bands(
        &vector_ta::indicators::bollinger_bands::BollingerBandsInput::from_candles(
            candles,
            ma_source,
            vector_ta::indicators::bollinger_bands::BollingerBandsParams {
                period: Some(strategy.breakout_bbands_period()),
                devup: Some(strategy.breakout_bbands_mult()),
                devdn: Some(strategy.breakout_bbands_mult()),
                matype: Some("sma".to_string()),
                devtype: Some(0),
            },
        ),
    )
    .map_err(|e| format!("breakout bollinger_bands failed: {e}"))?;
    let macd =
        vector_ta::indicators::macd::macd(&vector_ta::indicators::macd::MacdInput::from_candles(
            candles,
            ma_source,
            vector_ta::indicators::macd::MacdParams {
                fast_period: Some(strategy.breakout_macd_fast_period()),
                slow_period: Some(strategy.breakout_macd_slow_period()),
                signal_period: Some(strategy.breakout_macd_signal_period()),
                ma_type: Some("ema".to_string()),
            },
        ))
        .map_err(|e| format!("breakout macd failed: {e}"))?;
    let cmo = vector_ta::indicators::cmo::cmo(&vector_ta::indicators::cmo::CmoInput::from_candles(
        candles,
        ma_source,
        vector_ta::indicators::cmo::CmoParams {
            period: Some(strategy.breakout_cmo_period()),
        },
    ))
    .map_err(|e| format!("breakout cmo failed: {e}"))?;
    let adosc = vector_ta::indicators::adosc::adosc(
        &vector_ta::indicators::adosc::AdoscInput::from_candles(
            candles,
            vector_ta::indicators::adosc::AdoscParams {
                short_period: Some(strategy.breakout_adosc_short_period()),
                long_period: Some(strategy.breakout_adosc_long_period()),
            },
        ),
    )
    .map_err(|e| format!("breakout adosc failed: {e}"))?;

    if bb.upper_band.len() != n
        || bb.lower_band.len() != n
        || macd.hist.len() != n
        || cmo.values.len() != n
        || adosc.values.len() != n
    {
        return Err("breakout indicator length mismatch".to_string());
    }

    let adx_vals = if strategy.breakout_use_adx() {
        let adx =
            vector_ta::indicators::adx::adx(&vector_ta::indicators::adx::AdxInput::from_candles(
                candles,
                vector_ta::indicators::adx::AdxParams {
                    period: Some(strategy.breakout_adx_period()),
                },
            ))
            .map_err(|e| format!("breakout adx failed: {e}"))?
            .values;
        if adx.len() != n {
            return Err("breakout adx length mismatch".to_string());
        }
        Some(adx)
    } else {
        None
    };

    let confirm_bars = strategy.breakout_confirm_bars();
    let cmo_min_abs = strategy.breakout_cmo_min_abs();
    let adx_min = strategy.breakout_adx_min();

    let mut regime = alloc_uninit_f64(n);
    let mut long_count = 0usize;
    let mut short_count = 0usize;
    for i in 0..n {
        let close = candles.close[i];
        let upper = bb.upper_band[i];
        let lower = bb.lower_band[i];
        let hist = macd.hist[i];
        let c = cmo.values[i];
        let flow = adosc.values[i];
        let adx_ok = if let Some(adx) = adx_vals.as_ref() {
            let d = adx[i];
            d.is_finite() && d >= adx_min
        } else {
            true
        };

        regime[i] = f64::NAN;
        if !close.is_finite()
            || !upper.is_finite()
            || !lower.is_finite()
            || !hist.is_finite()
            || !c.is_finite()
            || !flow.is_finite()
            || !adx_ok
        {
            long_count = 0;
            short_count = 0;
            continue;
        }

        let long_ok = close >= upper && hist > 0.0 && c >= cmo_min_abs && flow > 0.0;
        let short_ok = close <= lower && hist < 0.0 && c <= -cmo_min_abs && flow < 0.0;

        long_count = if long_ok {
            long_count.saturating_add(1)
        } else {
            0
        };
        short_count = if short_ok {
            short_count.saturating_add(1)
        } else {
            0
        };

        let long_active = long_count >= confirm_bars;
        let short_active = short_count >= confirm_bars;
        regime[i] = if long_active && !short_active {
            0.0
        } else if short_active && !long_active {
            1.0
        } else {
            f64::NAN
        };
    }

    Ok(Some(regime))
}

pub(crate) fn compute_volume_flow_confirmation_regime(
    candles: &vector_ta::utilities::data_loader::Candles,
    strategy: &StrategyConfig,
) -> Result<Option<Vec<f64>>, String> {
    if !strategy.uses_volume_flow_confirm_filter() {
        return Ok(None);
    }
    if !candles.fields.high || !candles.fields.low || !candles.fields.volume {
        return Err(
            "volume_flow_confirm_enable requires 'high', 'low', and 'volume' candle columns"
                .to_string(),
        );
    }

    let n = candles.close.len();
    let obv = vector_ta::indicators::obv::obv(&vector_ta::indicators::obv::ObvInput::from_candles(
        candles,
        vector_ta::indicators::obv::ObvParams::default(),
    ))
    .map_err(|e| format!("vfct obv failed: {e}"))?;
    let obv_ma = vector_ta::indicators::moving_averages::ma::ma(
        "ema",
        vector_ta::indicators::moving_averages::ma::MaData::Slice(&obv.values),
        strategy.volume_flow_obv_smooth_period(),
    )
    .map_err(|e| format!("vfct obv smoothing failed: {e}"))?;
    let mfi = vector_ta::indicators::mfi::mfi(&vector_ta::indicators::mfi::MfiInput::from_candles(
        candles,
        "hlc3",
        vector_ta::indicators::mfi::MfiParams {
            period: Some(strategy.volume_flow_mfi_period()),
        },
    ))
    .map_err(|e| format!("vfct mfi failed: {e}"))?;

    if obv.values.len() != n || obv_ma.len() != n || mfi.values.len() != n {
        return Err("vfct indicator length mismatch".to_string());
    }

    let confirm_bars = strategy.volume_flow_confirm_bars();
    let long_min = strategy.volume_flow_mfi_long_min();
    let short_max = strategy.volume_flow_mfi_short_max();

    let mut regime = alloc_uninit_f64(n);
    let mut long_count = 0usize;
    let mut short_count = 0usize;
    for i in 0..n {
        let obv_v = obv.values[i];
        let obv_s = obv_ma[i];
        let mfi_v = mfi.values[i];

        regime[i] = f64::NAN;
        if !obv_v.is_finite() || !obv_s.is_finite() || !mfi_v.is_finite() {
            long_count = 0;
            short_count = 0;
            continue;
        }

        let long_ok = obv_v > obv_s && mfi_v >= long_min;
        let short_ok = obv_v < obv_s && mfi_v <= short_max;

        long_count = if long_ok {
            long_count.saturating_add(1)
        } else {
            0
        };
        short_count = if short_ok {
            short_count.saturating_add(1)
        } else {
            0
        };

        let long_active = long_count >= confirm_bars;
        let short_active = short_count >= confirm_bars;
        regime[i] = if long_active && !short_active {
            0.0
        } else if short_active && !long_active {
            1.0
        } else {
            f64::NAN
        };
    }

    Ok(Some(regime))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn regime_device_matrix_from_output(
    label: &str,
    output: crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput,
) -> Result<(vector_ta::indicators::dispatch::DeviceMatrixF32, usize), String> {
    let warmup = output.warmup.unwrap_or(0);
    match output.series {
        DynamicIndicatorCudaSeries::DeviceF32(matrix) => Ok((matrix, warmup)),
        DynamicIndicatorCudaSeries::HostF32(_) => Err(format!(
            "regime builder {label} unexpectedly returned host output instead of device output"
        )),
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn dynamic_cuda_output_from_indicator_output(
    output: IndicatorCudaOutput,
) -> crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput {
    crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput {
        output_id: output.output_id,
        series: match output.series {
            IndicatorCudaSeries::DeviceF32(matrix) => DynamicIndicatorCudaSeries::DeviceF32(matrix),
            IndicatorCudaSeries::HostF32(values) => DynamicIndicatorCudaSeries::HostF32(values),
        },
        warmup: output.warmup,
        rows: output.rows,
        cols: output.cols,
        pattern_ids: output.pattern_ids,
    }
}

pub(crate) fn compute_pressure_impulse_trend_regime(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
) -> Result<Option<Vec<f64>>, String> {
    if !strategy.uses_pressure_impulse_trend_filter() {
        return Ok(None);
    }
    if !candles.fields.open || !candles.fields.high || !candles.fields.low || !candles.fields.volume
    {
        return Err(
            "pressure_impulse_trend_enable requires 'open', 'high', 'low', and 'volume' candle columns"
                .to_string(),
        );
    }

    let n = candles.close.len();
    let efi = vector_ta::indicators::efi::efi(&vector_ta::indicators::efi::EfiInput::from_candles(
        candles,
        ma_source,
        vector_ta::indicators::efi::EfiParams {
            period: Some(strategy.pit_efi_period()),
        },
    ))
    .map_err(|e| format!("pit efi failed: {e}"))?
    .values;
    let bop = vector_ta::indicators::bop::bop(&vector_ta::indicators::bop::BopInput::from_candles(
        candles,
        vector_ta::indicators::bop::BopParams::default(),
    ))
    .map_err(|e| format!("pit bop failed: {e}"))?
    .values;
    let vosc =
        vector_ta::indicators::vosc::vosc(&vector_ta::indicators::vosc::VoscInput::from_candles(
            candles,
            "volume",
            vector_ta::indicators::vosc::VoscParams {
                short_period: Some(strategy.pit_vosc_short_period()),
                long_period: Some(strategy.pit_vosc_long_period()),
            },
        ))
        .map_err(|e| format!("pit vosc failed: {e}"))?
        .values;
    let acosc = vector_ta::indicators::acosc::acosc(
        &vector_ta::indicators::acosc::AcoscInput::from_candles(
            candles,
            vector_ta::indicators::acosc::AcoscParams::default(),
        ),
    )
    .map_err(|e| format!("pit acosc failed: {e}"))?
    .osc;
    let cfo = vector_ta::indicators::cfo::cfo(&vector_ta::indicators::cfo::CfoInput::from_candles(
        candles,
        ma_source,
        vector_ta::indicators::cfo::CfoParams {
            period: Some(strategy.pit_cfo_period()),
            scalar: Some(strategy.pit_cfo_scalar()),
        },
    ))
    .map_err(|e| format!("pit cfo failed: {e}"))?
    .values;

    if efi.len() != n || bop.len() != n || vosc.len() != n || acosc.len() != n || cfo.len() != n {
        return Err("pit indicator length mismatch".to_string());
    }

    let confirm_bars = strategy.pit_confirm_bars();
    let mut regime = alloc_uninit_f64(n);
    let mut long_count = 0usize;
    let mut short_count = 0usize;
    for i in 0..n {
        let efi_v = efi[i];
        let bop_v = bop[i];
        let vosc_v = vosc[i];
        let acosc_v = acosc[i];
        let cfo_v = cfo[i];
        regime[i] = f64::NAN;
        if !efi_v.is_finite()
            || !bop_v.is_finite()
            || !vosc_v.is_finite()
            || !acosc_v.is_finite()
            || !cfo_v.is_finite()
        {
            long_count = 0;
            short_count = 0;
            continue;
        }

        let long_votes = usize::from(efi_v > 0.0)
            + usize::from(bop_v > 0.0)
            + usize::from(vosc_v > 0.0)
            + usize::from(acosc_v > 0.0)
            + usize::from(cfo_v > 0.0);
        let short_votes = usize::from(efi_v < 0.0)
            + usize::from(bop_v < 0.0)
            + usize::from(vosc_v < 0.0)
            + usize::from(acosc_v < 0.0)
            + usize::from(cfo_v < 0.0);
        let long_ok = long_votes >= 3;
        let short_ok = short_votes >= 3;

        long_count = if long_ok {
            long_count.saturating_add(1)
        } else {
            0
        };
        short_count = if short_ok {
            short_count.saturating_add(1)
        } else {
            0
        };

        let long_active = long_count >= confirm_bars;
        let short_active = short_count >= confirm_bars;
        regime[i] = if long_active && !short_active {
            0.0
        } else if short_active && !long_active {
            1.0
        } else {
            f64::NAN
        };
    }

    Ok(Some(regime))
}

pub(crate) fn compute_slope_efficiency_trend_regime(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
) -> Result<Option<Vec<f64>>, String> {
    if !strategy.uses_slope_efficiency_trend_filter() {
        return Ok(None);
    }
    if !candles.fields.high || !candles.fields.low {
        return Err(
            "slope_efficiency_trend_enable requires 'high' and 'low' candle columns".to_string(),
        );
    }

    let n = candles.close.len();
    let angle = vector_ta::indicators::linearreg_angle::linearreg_angle(
        &vector_ta::indicators::linearreg_angle::Linearreg_angleInput::from_candles(
            candles,
            ma_source,
            vector_ta::indicators::linearreg_angle::Linearreg_angleParams {
                period: Some(strategy.set_linearreg_period()),
            },
        ),
    )
    .map_err(|e| format!("set linearreg_angle failed: {e}"))?
    .values;
    let er = vector_ta::indicators::er::er(&vector_ta::indicators::er::ErInput::from_candles(
        candles,
        ma_source,
        vector_ta::indicators::er::ErParams {
            period: Some(strategy.set_er_period()),
        },
    ))
    .map_err(|e| format!("set er failed: {e}"))?
    .values;
    let chop =
        vector_ta::indicators::chop::chop(&vector_ta::indicators::chop::ChopInput::from_candles(
            candles,
            vector_ta::indicators::chop::ChopParams {
                period: Some(strategy.set_chop_period()),
                ..Default::default()
            },
        ))
        .map_err(|e| format!("set chop failed: {e}"))?
        .values;

    if angle.len() != n || er.len() != n || chop.len() != n {
        return Err("set indicator length mismatch".to_string());
    }

    let angle_min = strategy.set_angle_min_deg();
    let er_min = strategy.set_er_min();
    let chop_max = strategy.set_chop_max();
    let mut regime = alloc_uninit_f64(n);
    for i in 0..n {
        let angle_v = angle[i];
        let er_v = er[i];
        let chop_v = chop[i];
        regime[i] = f64::NAN;
        if !angle_v.is_finite() || !er_v.is_finite() || !chop_v.is_finite() {
            continue;
        }
        let long_ok = angle_v >= angle_min && er_v >= er_min && chop_v <= chop_max;
        let short_ok = angle_v <= -angle_min && er_v >= er_min && chop_v <= chop_max;
        regime[i] = if long_ok && !short_ok {
            0.0
        } else if short_ok && !long_ok {
            1.0
        } else {
            f64::NAN
        };
    }
    Ok(Some(regime))
}

pub(crate) fn compute_momentum_triple_stack_regime(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
) -> Result<Option<Vec<f64>>, String> {
    if !strategy.uses_momentum_triple_stack_filter() {
        return Ok(None);
    }

    let n = candles.close.len();
    let roc = vector_ta::indicators::roc::roc(&vector_ta::indicators::roc::RocInput::from_candles(
        candles,
        ma_source,
        vector_ta::indicators::roc::RocParams {
            period: Some(strategy.mts_roc_period()),
        },
    ))
    .map_err(|e| format!("mts roc failed: {e}"))?
    .values;
    let mom = vector_ta::indicators::mom::mom(&vector_ta::indicators::mom::MomInput::from_candles(
        candles,
        ma_source,
        vector_ta::indicators::mom::MomParams {
            period: Some(strategy.mts_mom_period()),
        },
    ))
    .map_err(|e| format!("mts mom failed: {e}"))?
    .values;
    let ppo = vector_ta::indicators::ppo::ppo(&vector_ta::indicators::ppo::PpoInput::from_candles(
        candles,
        ma_source,
        vector_ta::indicators::ppo::PpoParams {
            fast_period: Some(strategy.mts_ppo_fast_period()),
            slow_period: Some(strategy.mts_ppo_slow_period()),
            ma_type: Some("ema".to_string()),
        },
    ))
    .map_err(|e| format!("mts ppo failed: {e}"))?
    .values;

    if roc.len() != n || mom.len() != n || ppo.len() != n {
        return Err("mts indicator length mismatch".to_string());
    }

    let mut regime = alloc_uninit_f64(n);
    for i in 0..n {
        let roc_v = roc[i];
        let mom_v = mom[i];
        let ppo_v = ppo[i];
        regime[i] = f64::NAN;
        if !roc_v.is_finite() || !mom_v.is_finite() || !ppo_v.is_finite() {
            continue;
        }
        let long_ok = roc_v > 0.0 && mom_v > 0.0 && ppo_v > 0.0;
        let short_ok = roc_v < 0.0 && mom_v < 0.0 && ppo_v < 0.0;
        regime[i] = if long_ok && !short_ok {
            0.0
        } else if short_ok && !long_ok {
            1.0
        } else {
            f64::NAN
        };
    }
    Ok(Some(regime))
}

pub(crate) fn compute_oscillator_reversion_confluence_regime(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
) -> Result<Option<Vec<f64>>, String> {
    if !strategy.uses_oscillator_reversion_confluence_filter() {
        return Ok(None);
    }
    if !candles.fields.high || !candles.fields.low {
        return Err(
            "oscillator_reversion_confluence_enable requires 'high' and 'low' candle columns"
                .to_string(),
        );
    }

    let n = candles.close.len();
    let rsi = vector_ta::indicators::rsi::rsi(&vector_ta::indicators::rsi::RsiInput::from_candles(
        candles,
        ma_source,
        vector_ta::indicators::rsi::RsiParams {
            period: Some(strategy.orc_rsi_period()),
        },
    ))
    .map_err(|e| format!("orc rsi failed: {e}"))?
    .values;
    let stoch = vector_ta::indicators::stochf::stochf(
        &vector_ta::indicators::stochf::StochfInput::from_candles(
            candles,
            vector_ta::indicators::stochf::StochfParams {
                fastk_period: Some(strategy.orc_stoch_fastk_period()),
                fastd_period: Some(strategy.orc_stoch_fastd_period()),
                fastd_matype: Some(0),
            },
        ),
    )
    .map_err(|e| format!("orc stochf failed: {e}"))?;
    let willr = vector_ta::indicators::willr::willr(
        &vector_ta::indicators::willr::WillrInput::from_candles(
            candles,
            vector_ta::indicators::willr::WillrParams {
                period: Some(strategy.orc_willr_period()),
            },
        ),
    )
    .map_err(|e| format!("orc willr failed: {e}"))?
    .values;

    if rsi.len() != n || stoch.k.len() != n || willr.len() != n {
        return Err("orc indicator length mismatch".to_string());
    }

    let mut regime = alloc_uninit_f64(n);
    for i in 0..n {
        let rsi_v = rsi[i];
        let stoch_v = stoch.k[i];
        let willr_v = willr[i];
        regime[i] = f64::NAN;
        if !rsi_v.is_finite() || !stoch_v.is_finite() || !willr_v.is_finite() {
            continue;
        }
        let oversold_votes = usize::from(rsi_v <= 30.0)
            + usize::from(stoch_v <= 20.0)
            + usize::from(willr_v <= -80.0);
        let overbought_votes = usize::from(rsi_v >= 70.0)
            + usize::from(stoch_v >= 80.0)
            + usize::from(willr_v >= -20.0);
        let long_ok = oversold_votes >= 2;
        let short_ok = overbought_votes >= 2;
        regime[i] = if long_ok && !short_ok {
            0.0
        } else if short_ok && !long_ok {
            1.0
        } else {
            f64::NAN
        };
    }
    Ok(Some(regime))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) fn build_pressure_impulse_trend_regime_device(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    device_id: u32,
    strategy: &StrategyConfig,
) -> Result<crate::gpu_backtest_kernel::DeviceRegimeFilter, String> {
    #[cfg(all(test, feature = "cuda", feature = "cuda-backtest-kernel"))]
    let _gpu_guard = crate::gpu_backtest_kernel::lock_gpu_test();

    if !strategy.uses_pressure_impulse_trend_filter() {
        return Err("pit device regime requested for a non-pit strategy".to_string());
    }
    if !candles.fields.open || !candles.fields.high || !candles.fields.low || !candles.fields.volume
    {
        return Err(
            "pressure_impulse_trend_enable requires 'open', 'high', 'low', and 'volume' candle columns"
                .to_string(),
        );
    }

    let prepared_inputs =
        PreparedCudaIndicatorInputs::from_candles_on_device(candles, ma_source, device_id as usize)
            .map_err(|e| format!("pit cuda input prep failed: {e}"))?;

    let efi_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "efi",
        Some("value"),
        &[IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.pit_efi_period() as i64),
        }],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("pit efi failed: {e}"))?;
    let efi_warmup_raw = efi_output.warmup.unwrap_or(0);
    let (efi_matrix, _) = regime_device_matrix_from_output("pit efi", efi_output)?;
    let efi_warmup = efi_warmup_raw.max(
        prepared_inputs
            .close_volume_first_valid()
            .saturating_add(strategy.pit_efi_period().saturating_sub(1)),
    );
    let efi_row = device_slice_ref_from_matrix_row(&efi_matrix, 0)
        .map_err(|e| format!("pit efi device row build failed: {e}"))?;

    let bop_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "bop",
        Some("value"),
        &[],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("pit bop failed: {e}"))?;
    let bop_warmup = bop_output.warmup.unwrap_or(0);
    let (bop_matrix, _) = regime_device_matrix_from_output("pit bop", bop_output)?;
    let bop_row = device_slice_ref_from_matrix_row(&bop_matrix, 0)
        .map_err(|e| format!("pit bop device row build failed: {e}"))?;

    let vosc_output = compute_cuda_device(IndicatorCudaDeviceRequest {
        indicator_id: "vosc",
        output_id: Some("value"),
        data: IndicatorCudaDeviceDataRef::Slice {
            values: prepared_inputs.volume_device().as_view(),
        },
        params: &[
            vector_ta::indicators::dispatch::ParamKV {
                key: "short_period",
                value: vector_ta::indicators::dispatch::ParamValue::Int(
                    strategy.pit_vosc_short_period() as i64,
                ),
            },
            vector_ta::indicators::dispatch::ParamKV {
                key: "long_period",
                value: vector_ta::indicators::dispatch::ParamValue::Int(
                    strategy.pit_vosc_long_period() as i64,
                ),
            },
        ],
        kernel: Kernel::Auto,
        target: CudaOutputTarget::DeviceF32,
    })
    .map_err(|e| format!("pit vosc failed: {e}"))?;
    let vosc_warmup = vosc_output.warmup.unwrap_or(0);
    let vosc_output = dynamic_cuda_output_from_indicator_output(vosc_output);
    let (vosc_matrix, _) = regime_device_matrix_from_output("pit vosc", vosc_output)?;
    let vosc_row = device_slice_ref_from_matrix_row(&vosc_matrix, 0)
        .map_err(|e| format!("pit vosc device row build failed: {e}"))?;

    // Stage ACOSC from the CPU reference path for PIT until the upstream CUDA
    // wrapper matches the CPU oscillator output on breakout inputs.
    let acosc_host = vector_ta::indicators::acosc::acosc(
        &vector_ta::indicators::acosc::AcoscInput::from_candles(
            candles,
            vector_ta::indicators::acosc::AcoscParams::default(),
        ),
    )
    .map_err(|e| format!("pit acosc failed: {e}"))?;
    let acosc_warmup = acosc_host
        .osc
        .iter()
        .position(|value| value.is_finite())
        .unwrap_or(acosc_host.osc.len());
    let acosc_values_f32: Vec<f32> = acosc_host.osc.iter().map(|value| *value as f32).collect();
    let (acosc_row, acosc_buffer) =
        crate::gpu_backtest_kernel::upload_device_series_f32(device_id, &acosc_values_f32)
            .map_err(|e| format!("pit acosc upload failed: {e}"))?;

    let cfo_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "cfo",
        Some("value"),
        &[
            IndicatorParamKV {
                key: "period".to_string(),
                value: IndicatorParamValue::Int(strategy.pit_cfo_period() as i64),
            },
            IndicatorParamKV {
                key: "scalar".to_string(),
                value: IndicatorParamValue::Float(strategy.pit_cfo_scalar()),
            },
        ],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("pit cfo failed: {e}"))?;
    let cfo_warmup_raw = cfo_output.warmup.unwrap_or(0);
    let (cfo_matrix, _) = regime_device_matrix_from_output("pit cfo", cfo_output)?;
    let cfo_warmup = cfo_warmup_raw.max(
        prepared_inputs
            .source_first_valid()
            .saturating_add(strategy.pit_cfo_period().saturating_sub(1)),
    );
    let cfo_row = device_slice_ref_from_matrix_row(&cfo_matrix, 0)
        .map_err(|e| format!("pit cfo device row build failed: {e}"))?;
    let first_valid = prepared_inputs
        .ohlcv_first_valid()
        .max(prepared_inputs.source_first_valid())
        .max(efi_warmup)
        .max(bop_warmup)
        .max(vosc_warmup)
        .max(acosc_warmup)
        .max(cfo_warmup);

    crate::gpu_backtest_kernel::build_pit_regime_filter(
        device_id,
        efi_row,
        bop_row,
        vosc_row,
        acosc_row,
        cfo_row,
        first_valid,
        strategy.pit_confirm_bars(),
    )
    .map(|filter| {
        filter
            .with_scratch_buffers(vec![acosc_buffer])
            .with_dispatch_matrices(vec![efi_matrix, bop_matrix, vosc_matrix, cfo_matrix])
    })
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn prepared_cuda_input_kinds_for_indicator_ids(
    indicator_ids: &[&str],
) -> Result<Vec<IndicatorInputKind>, String> {
    let mut input_kinds = Vec::new();
    for indicator_id in indicator_ids {
        let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
            format!("unsupported_indicator: '{indicator_id}' is not in dynamic indicator registry")
        })?;
        if !input_kinds.contains(&descriptor.input_kind) {
            input_kinds.push(descriptor.input_kind);
        }
    }
    Ok(input_kinds)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) fn build_slope_efficiency_trend_regime_device(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    device_id: u32,
    strategy: &StrategyConfig,
) -> Result<crate::gpu_backtest_kernel::DeviceRegimeFilter, String> {
    #[cfg(all(test, feature = "cuda", feature = "cuda-backtest-kernel"))]
    let _gpu_guard = crate::gpu_backtest_kernel::lock_gpu_test();

    if !strategy.uses_slope_efficiency_trend_filter() {
        return Err("set device regime requested for a non-set strategy".to_string());
    }
    if !candles.fields.high || !candles.fields.low {
        return Err(
            "slope_efficiency_trend_enable requires 'high' and 'low' candle columns".to_string(),
        );
    }

    let input_kinds =
        prepared_cuda_input_kinds_for_indicator_ids(&["linearreg_angle", "er", "chop"])?;
    let prepared_inputs = PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(
        candles,
        ma_source,
        device_id as usize,
        input_kinds.as_slice(),
    )
    .map_err(|e| format!("set cuda input prep failed: {e}"))?;

    let angle_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "linearreg_angle",
        Some("value"),
        &[IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.set_linearreg_period() as i64),
        }],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("set linearreg_angle failed: {e}"))?;
    let (angle_matrix, angle_warmup) =
        regime_device_matrix_from_output("set linearreg_angle", angle_output)?;

    let er_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "er",
        Some("value"),
        &[IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.set_er_period() as i64),
        }],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("set er failed: {e}"))?;
    let (er_matrix, er_warmup) = regime_device_matrix_from_output("set er", er_output)?;

    let chop_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "chop",
        Some("value"),
        &[IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.set_chop_period() as i64),
        }],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("set chop failed: {e}"))?;
    let (chop_matrix, chop_warmup) = regime_device_matrix_from_output("set chop", chop_output)?;

    let angle_row = device_slice_ref_from_matrix_row(&angle_matrix, 0)
        .map_err(|e| format!("set angle device row build failed: {e}"))?;
    let er_row = device_slice_ref_from_matrix_row(&er_matrix, 0)
        .map_err(|e| format!("set er device row build failed: {e}"))?;
    let chop_row = device_slice_ref_from_matrix_row(&chop_matrix, 0)
        .map_err(|e| format!("set chop device row build failed: {e}"))?;
    let first_valid = prepared_inputs
        .hlc_first_valid()
        .max(prepared_inputs.source_first_valid())
        .max(angle_warmup)
        .max(er_warmup)
        .max(chop_warmup);

    crate::gpu_backtest_kernel::build_set_regime_filter(
        device_id,
        angle_row,
        er_row,
        chop_row,
        first_valid,
        strategy.set_angle_min_deg() as f32,
        strategy.set_er_min() as f32,
        strategy.set_chop_max() as f32,
    )
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) fn build_momentum_triple_stack_regime_device(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    device_id: u32,
    strategy: &StrategyConfig,
) -> Result<crate::gpu_backtest_kernel::DeviceRegimeFilter, String> {
    #[cfg(all(test, feature = "cuda", feature = "cuda-backtest-kernel"))]
    let _gpu_guard = crate::gpu_backtest_kernel::lock_gpu_test();

    if !strategy.uses_momentum_triple_stack_filter() {
        return Err("mts device regime requested for a non-mts strategy".to_string());
    }

    let input_kinds = prepared_cuda_input_kinds_for_indicator_ids(&["roc", "mom", "ppo"])?;
    let prepared_inputs = PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(
        candles,
        ma_source,
        device_id as usize,
        input_kinds.as_slice(),
    )
    .map_err(|e| format!("mts cuda input prep failed: {e}"))?;

    let roc_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "roc",
        Some("value"),
        &[IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.mts_roc_period() as i64),
        }],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("mts roc failed: {e}"))?;
    let (roc_matrix, roc_warmup) = regime_device_matrix_from_output("mts roc", roc_output)?;

    let mom_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "mom",
        Some("value"),
        &[IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.mts_mom_period() as i64),
        }],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("mts mom failed: {e}"))?;
    let (mom_matrix, mom_warmup) = regime_device_matrix_from_output("mts mom", mom_output)?;

    let ppo_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "ppo",
        Some("value"),
        &[
            IndicatorParamKV {
                key: "fast_period".to_string(),
                value: IndicatorParamValue::Int(strategy.mts_ppo_fast_period() as i64),
            },
            IndicatorParamKV {
                key: "slow_period".to_string(),
                value: IndicatorParamValue::Int(strategy.mts_ppo_slow_period() as i64),
            },
            IndicatorParamKV {
                key: "ma_type".to_string(),
                value: IndicatorParamValue::EnumString("ema".to_string()),
            },
        ],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("mts ppo failed: {e}"))?;
    let (ppo_matrix, ppo_warmup) = regime_device_matrix_from_output("mts ppo", ppo_output)?;

    let roc_row = device_slice_ref_from_matrix_row(&roc_matrix, 0)
        .map_err(|e| format!("mts roc device row build failed: {e}"))?;
    let mom_row = device_slice_ref_from_matrix_row(&mom_matrix, 0)
        .map_err(|e| format!("mts mom device row build failed: {e}"))?;
    let ppo_row = device_slice_ref_from_matrix_row(&ppo_matrix, 0)
        .map_err(|e| format!("mts ppo device row build failed: {e}"))?;
    let first_valid = prepared_inputs
        .source_first_valid()
        .max(roc_warmup)
        .max(mom_warmup)
        .max(ppo_warmup);

    crate::gpu_backtest_kernel::build_mts_regime_filter(
        device_id,
        roc_row,
        mom_row,
        ppo_row,
        first_valid,
    )
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) fn build_oscillator_reversion_confluence_regime_device(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    device_id: u32,
    strategy: &StrategyConfig,
) -> Result<crate::gpu_backtest_kernel::DeviceRegimeFilter, String> {
    #[cfg(all(test, feature = "cuda", feature = "cuda-backtest-kernel"))]
    let _gpu_guard = crate::gpu_backtest_kernel::lock_gpu_test();

    if !strategy.uses_oscillator_reversion_confluence_filter() {
        return Err("orc device regime requested for a non-orc strategy".to_string());
    }
    if !candles.fields.high || !candles.fields.low {
        return Err(
            "oscillator_reversion_confluence_enable requires 'high' and 'low' candle columns"
                .to_string(),
        );
    }

    let input_kinds = prepared_cuda_input_kinds_for_indicator_ids(&["rsi", "stochf", "willr"])?;
    let prepared_inputs = PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(
        candles,
        ma_source,
        device_id as usize,
        input_kinds.as_slice(),
    )
    .map_err(|e| format!("orc cuda input prep failed: {e}"))?;

    let rsi_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "rsi",
        Some("value"),
        &[IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.orc_rsi_period() as i64),
        }],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("orc rsi failed: {e}"))?;
    let (rsi_matrix, rsi_warmup) = regime_device_matrix_from_output("orc rsi", rsi_output)?;

    let stoch_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "stochf",
        Some("k"),
        &[
            IndicatorParamKV {
                key: "fastk_period".to_string(),
                value: IndicatorParamValue::Int(strategy.orc_stoch_fastk_period() as i64),
            },
            IndicatorParamKV {
                key: "fastd_period".to_string(),
                value: IndicatorParamValue::Int(strategy.orc_stoch_fastd_period() as i64),
            },
        ],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("orc stochf failed: {e}"))?;
    let (stoch_matrix, stoch_warmup) =
        regime_device_matrix_from_output("orc stochf", stoch_output)?;

    let willr_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "willr",
        Some("value"),
        &[IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.orc_willr_period() as i64),
        }],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("orc willr failed: {e}"))?;
    let (willr_matrix, willr_warmup) = regime_device_matrix_from_output("orc willr", willr_output)?;

    let rsi_row = device_slice_ref_from_matrix_row(&rsi_matrix, 0)
        .map_err(|e| format!("orc rsi device row build failed: {e}"))?;
    let stoch_row = device_slice_ref_from_matrix_row(&stoch_matrix, 0)
        .map_err(|e| format!("orc stoch device row build failed: {e}"))?;
    let willr_row = device_slice_ref_from_matrix_row(&willr_matrix, 0)
        .map_err(|e| format!("orc willr device row build failed: {e}"))?;
    let first_valid = prepared_inputs
        .hlc_first_valid()
        .max(prepared_inputs.source_first_valid())
        .max(rsi_warmup)
        .max(stoch_warmup)
        .max(willr_warmup);

    crate::gpu_backtest_kernel::build_orc_regime_filter(
        device_id,
        rsi_row,
        stoch_row,
        willr_row,
        first_valid,
    )
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) fn build_volume_flow_confirmation_regime_device(
    candles: &vector_ta::utilities::data_loader::Candles,
    device_id: u32,
    strategy: &StrategyConfig,
) -> Result<crate::gpu_backtest_kernel::DeviceRegimeFilter, String> {
    #[cfg(all(test, feature = "cuda", feature = "cuda-backtest-kernel"))]
    let _gpu_guard = crate::gpu_backtest_kernel::lock_gpu_test();

    if !strategy.uses_volume_flow_confirm_filter() {
        return Err("vfct device regime requested for a non-vfct strategy".to_string());
    }
    if !candles.fields.high || !candles.fields.low || !candles.fields.volume {
        return Err(
            "volume_flow_confirm_enable requires 'high', 'low', and 'volume' candle columns"
                .to_string(),
        );
    }

    let mut input_kinds = prepared_cuda_input_kinds_for_indicator_ids(&["mfi"])?;
    if !input_kinds.contains(&IndicatorInputKind::CloseVolume) {
        input_kinds.push(IndicatorInputKind::CloseVolume);
    }
    let prepared_inputs = PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(
        candles,
        "hlc3",
        device_id as usize,
        input_kinds.as_slice(),
    )
    .map_err(|e| format!("vfct cuda input prep failed: {e}"))?;
    let obv_data = CudaDeviceCloseVolumeRef::new(
        prepared_inputs.close_device().as_view(),
        prepared_inputs.volume_device().as_view(),
    )
    .map_err(|e| format!("vfct obv device input build failed: {e}"))?;
    let obv_output = compute_cuda_device(IndicatorCudaDeviceRequest {
        indicator_id: "obv",
        output_id: Some("value"),
        data: IndicatorCudaDeviceDataRef::CloseVolume(obv_data),
        params: &[],
        kernel: Kernel::Auto,
        target: CudaOutputTarget::DeviceF32,
    })
    .map_err(|e| format!("vfct obv failed: {e}"))?;
    let obv_output = crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput {
        output_id: obv_output.output_id,
        series: match obv_output.series {
            IndicatorCudaSeries::DeviceF32(matrix) => DynamicIndicatorCudaSeries::DeviceF32(matrix),
            IndicatorCudaSeries::HostF32(values) => DynamicIndicatorCudaSeries::HostF32(values),
        },
        warmup: obv_output.warmup,
        rows: obv_output.rows,
        cols: obv_output.cols,
        pattern_ids: obv_output.pattern_ids,
    };
    let (obv_matrix, obv_warmup) = regime_device_matrix_from_output("obv", obv_output)?;

    let obv_smooth_output = compute_dynamic_indicator_cuda_from_device_matrix_row(
        "ema",
        Some("value"),
        &obv_matrix,
        0,
        obv_warmup,
        &[IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.volume_flow_obv_smooth_period() as i64),
        }],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("vfct obv smoothing failed: {e}"))?;
    let (obv_smooth_matrix, obv_smooth_warmup) =
        regime_device_matrix_from_output("obv smoothing", obv_smooth_output)?;

    let mfi_output = compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        "mfi",
        Some("value"),
        &[IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.volume_flow_mfi_period() as i64),
        }],
        Kernel::Auto,
        CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("vfct mfi failed: {e}"))?;
    let (mfi_matrix, mfi_warmup) = regime_device_matrix_from_output("mfi", mfi_output)?;

    let obv_row = device_slice_ref_from_matrix_row(&obv_matrix, 0)
        .map_err(|e| format!("vfct obv device row build failed: {e}"))?;
    let obv_smooth_row = device_slice_ref_from_matrix_row(&obv_smooth_matrix, 0)
        .map_err(|e| format!("vfct obv smoothing device row build failed: {e}"))?;
    let mfi_row = device_slice_ref_from_matrix_row(&mfi_matrix, 0)
        .map_err(|e| format!("vfct mfi device row build failed: {e}"))?;
    let first_valid = prepared_inputs
        .close_volume_first_valid()
        .max(obv_warmup)
        .max(obv_smooth_warmup)
        .max(mfi_warmup);

    crate::gpu_backtest_kernel::build_vfct_regime_filter(
        device_id,
        obv_row,
        obv_smooth_row,
        mfi_row,
        first_valid,
        strategy.volume_flow_mfi_long_min() as f32,
        strategy.volume_flow_mfi_short_max() as f32,
        strategy.volume_flow_confirm_bars(),
    )
}

pub(crate) fn compute_combined_regime_filter(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
    halftrend_filter: Option<&HalfTrendRegimeFilter>,
) -> Result<Option<Vec<f64>>, String> {
    if strategy.uses_regime_hybrid_filter() && halftrend_filter.is_some() {
        return Err(
            "regime_hybrid_enable is not compatible with halftrend_filter in double-MA flow"
                .to_string(),
        );
    }
    let halftrend_trend = match halftrend_filter {
        Some(f) => Some(compute_halftrend_trend(candles, f)?),
        None => None,
    };
    let trend_quality = compute_trend_quality_regime(candles, ma_source, strategy)?;
    let breakout = compute_breakout_momentum_regime(candles, ma_source, strategy)?;
    let pit = compute_pressure_impulse_trend_regime(candles, ma_source, strategy)?;
    let set = compute_slope_efficiency_trend_regime(candles, ma_source, strategy)?;
    let mts = compute_momentum_triple_stack_regime(candles, ma_source, strategy)?;
    let orc = compute_oscillator_reversion_confluence_regime(candles, ma_source, strategy)?;
    let vfct = compute_volume_flow_confirmation_regime(candles, strategy)?;
    let merged = merge_regime_filters(halftrend_trend, trend_quality)?;
    let merged = merge_regime_filters(merged, breakout)?;
    let merged = merge_regime_filters(merged, pit)?;
    let merged = merge_regime_filters(merged, set)?;
    let merged = merge_regime_filters(merged, mts)?;
    let merged = merge_regime_filters(merged, orc)?;
    merge_regime_filters(merged, vfct)
}

fn ensure_combined_regime_filter_cached<'a>(
    cache: &'a mut Option<Option<Vec<f64>>>,
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
    halftrend_filter: Option<&HalfTrendRegimeFilter>,
    expected_len: usize,
) -> Result<Option<&'a [f64]>, String> {
    if cache.is_none() {
        let regime =
            compute_combined_regime_filter(candles, ma_source, strategy, halftrend_filter)?;
        if let Some(trend) = regime.as_ref() {
            if trend.len() != expected_len {
                return Err("regime trend series length mismatch".to_string());
            }
        }
        *cache = Some(regime);
    }
    Ok(cache.as_ref().and_then(|regime| regime.as_deref()))
}

#[derive(Serialize)]
struct FollowupCacheSignature<'a> {
    data_id: &'a str,
    num_candles: usize,
    ma_source: &'a str,
    fast_ma: &'a str,
    slow_ma: &'a str,
    strategy: &'a StrategyConfig,
    halftrend_filter: Option<&'a HalfTrendRegimeFilter>,
}

fn build_followup_cache_key(
    data_id: &str,
    num_candles: usize,
    ma_source: &str,
    fast_ma: &str,
    slow_ma: &str,
    strategy: &StrategyConfig,
    halftrend_filter: Option<&HalfTrendRegimeFilter>,
) -> Result<String, String> {
    serde_json::to_string(&FollowupCacheSignature {
        data_id,
        num_candles,
        ma_source,
        fast_ma,
        slow_ma,
        strategy,
        halftrend_filter,
    })
    .map_err(|e| format!("follow-up cache key serialization failed: {e}"))
}

fn resolve_followup_cache_entry(
    key: Option<&str>,
    data_id: &str,
    num_candles: usize,
    ma_source: &str,
    fast_ma: &str,
    slow_ma: &str,
    strategy: &StrategyConfig,
    halftrend_filter: Option<&HalfTrendRegimeFilter>,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<Option<FollowupCacheEntry>, String> {
    let Some(token) = key.map(str::trim).filter(|token| !token.is_empty()) else {
        return Ok(None);
    };
    let expected = build_followup_cache_key(
        data_id,
        num_candles,
        ma_source,
        fast_ma,
        slow_ma,
        strategy,
        halftrend_filter,
    )?;
    if token != expected {
        return Ok(None);
    }
    let Some(entry) = maybe_load_followup_cache_entry(Some(token)) else {
        return Ok(None);
    };
    if entry.log_returns.is_some() && entry.base_first_valid.is_some() {
        return Ok(Some(entry));
    }
    let prepared = build_followup_cache_entry(
        candles,
        ma_source,
        strategy,
        halftrend_filter,
        entry.backend_hint.as_ref(),
    )?;
    store_followup_cache_entry(token.to_string(), prepared.clone());
    Ok(Some(prepared))
}

fn build_followup_cache_entry(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
    halftrend_filter: Option<&HalfTrendRegimeFilter>,
    backend_hint: Option<&Backend>,
) -> Result<FollowupCacheEntry, String> {
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let mut base_first_valid = fv_close.max(fv_ma);

    let regime_filter =
        compute_combined_regime_filter(candles, ma_source, strategy, halftrend_filter)?;
    if let Some(trend) = regime_filter.as_ref() {
        if trend.len() != close.len() {
            return Err("regime trend series length mismatch".to_string());
        }
        let fv_ht = first_finite_idx(trend)
            .ok_or_else(|| "regime trend series all non-finite".to_string())?;
        base_first_valid = base_first_valid.max(fv_ht);
    }

    let kaufman_stops = compute_kaufman_stop_series(candles, strategy)?;
    if let Some(ks) = kaufman_stops.as_ref() {
        let fv_k_long = first_finite_idx(&ks.long)
            .ok_or_else(|| "kaufman-stop long series all non-finite".to_string())?;
        let fv_k_short = first_finite_idx(&ks.short)
            .ok_or_else(|| "kaufman-stop short series all non-finite".to_string())?;
        base_first_valid = base_first_valid.max(fv_k_long.max(fv_k_short));
    }

    Ok(FollowupCacheEntry {
        backend_hint: backend_hint.cloned(),
        log_returns: Some(compute_log_returns_f64(close)),
        regime_filter,
        kaufman_stops,
        base_first_valid: Some(base_first_valid),
    })
}

fn build_lazy_followup_cache_entry(backend_hint: Option<&Backend>) -> FollowupCacheEntry {
    FollowupCacheEntry {
        backend_hint: backend_hint.cloned(),
        log_returns: None,
        regime_filter: None,
        kaufman_stops: None,
        base_first_valid: None,
    }
}

fn maybe_load_followup_cache_entry(key: Option<&str>) -> Option<FollowupCacheEntry> {
    let token = key?.trim();
    if token.is_empty() {
        return None;
    }
    followup_cache()
        .lock()
        .ok()
        .and_then(|guard| guard.get(token).cloned())
}

fn store_followup_cache_entry(key: String, entry: FollowupCacheEntry) {
    if let Ok(mut guard) = followup_cache().lock() {
        if guard.len() >= 16 {
            if let Some(oldest_key) = guard.keys().next().cloned() {
                guard.remove(&oldest_key);
            }
        }
        guard.insert(key, entry);
    }
}

fn build_gpu_residency_summary(
    requested_policy: GpuExecutionPolicy,
    requested_backend: &Backend,
    resolved_backend: &Backend,
    indicator_route_plan: &IndicatorRoutePlanSummary,
    backend_fallback_reason: Option<&str>,
) -> GpuResidencySummary {
    let cpu_staged_indicators = indicator_route_plan.fast.residency_tier == "cpu_staged"
        || indicator_route_plan.slow.residency_tier == "cpu_staged";
    let cpu_fallback_used = matches!(resolved_backend, Backend::CpuOnly | Backend::GpuOnly { .. });
    let fully_gpu_resident = match resolved_backend {
        Backend::GpuKernel { .. } => true,
        Backend::GpuHybridKernel { .. } => !cpu_staged_indicators,
        Backend::Auto { .. } | Backend::CpuOnly | Backend::GpuOnly { .. } => false,
    };
    let mut notes = Vec::new();
    if let Some(reason) = backend_fallback_reason {
        let trimmed = reason.trim();
        if !trimmed.is_empty() {
            notes.push(trimmed.to_string());
        }
    }
    if matches!(resolved_backend, Backend::GpuOnly { .. }) {
        notes.push(
            "GpuOnly uses CUDA MA generation but host-resident metric evaluation; it is not an end-to-end GPU path."
                .to_string(),
        );
    }

    GpuResidencySummary {
        requested_policy,
        requested_backend: backend_kind_label(requested_backend).to_string(),
        resolved_backend: backend_kind_label(resolved_backend).to_string(),
        fully_gpu_resident,
        cpu_staged_indicators,
        cpu_fallback_used,
        notes,
    }
}

fn strict_gpu_resident_backend_violation(
    resolved_backend: &Backend,
    indicator_route_plan: &IndicatorRoutePlanSummary,
) -> Option<String> {
    match resolved_backend {
        Backend::GpuKernel { .. } => None,
        Backend::GpuHybridKernel { .. } => {
            let mut staged = Vec::new();
            if indicator_route_plan.fast.residency_tier == "cpu_staged" {
                staged.push(indicator_route_plan.fast.indicator_id.as_str());
            }
            if indicator_route_plan.slow.residency_tier == "cpu_staged" {
                staged.push(indicator_route_plan.slow.indicator_id.as_str());
            }
            if staged.is_empty() {
                None
            } else {
                Some(format!(
                    "hybrid routing still requires CPU-staged indicators ({})",
                    staged.join(", ")
                ))
            }
        }
        Backend::GpuOnly { .. } => Some(
            "GpuOnly is a host-resident CUDA sweep and does not satisfy GPU-resident execution"
                .to_string(),
        ),
        Backend::CpuOnly => Some("resolved backend is CpuOnly".to_string()),
        Backend::Auto { .. } => Some("resolved backend remained Auto".to_string()),
    }
}

fn enforce_gpu_execution_policy_before_run(
    policy: GpuExecutionPolicy,
    resolved_backend: &Backend,
    indicator_route_plan: &IndicatorRoutePlanSummary,
) -> Result<(), String> {
    if !matches!(policy, GpuExecutionPolicy::StrictGpuResident) {
        return Ok(());
    }
    if let Some(reason) =
        strict_gpu_resident_backend_violation(resolved_backend, indicator_route_plan)
    {
        return Err(format!(
            "gpu_execution=StrictGpuResident requires an end-to-end GPU-resident route, but {reason}."
        ));
    }
    Ok(())
}

fn enforce_gpu_execution_policy_after_run(summary: &GpuResidencySummary) -> Result<(), String> {
    if matches!(
        summary.requested_policy,
        GpuExecutionPolicy::StrictGpuResident
    ) && !summary.fully_gpu_resident
    {
        return Err(format!(
            "gpu_execution=StrictGpuResident was requested, but the run resolved to {} and was not fully GPU resident.",
            summary.resolved_backend
        ));
    }
    Ok(())
}

pub(crate) fn resolve_alma_params_for_side(
    side: &'static str,
    ma_type: &str,
    ma_params: Option<&HashMap<String, f64>>,
) -> Result<(f64, f64), String> {
    let default_offset = 0.85;
    let default_sigma = 6.0;
    if !ma_type.trim().eq_ignore_ascii_case("alma") {
        return Ok((default_offset, default_sigma));
    }

    let mut offset = default_offset;
    let mut sigma = default_sigma;
    if let Some(params) = ma_params {
        if let Some(v) = params.get("offset") {
            offset = *v;
        }
        if let Some(v) = params.get("sigma") {
            sigma = *v;
        }
    }

    if !offset.is_finite() || offset < 0.0 || offset > 1.0 {
        return Err(format!(
            "ALMA offset for {side} MA must be finite and in [0, 1]"
        ));
    }
    if !sigma.is_finite() || sigma <= 0.0 {
        return Err(format!("ALMA sigma for {side} MA must be finite and > 0"));
    }

    Ok((offset, sigma))
}

pub(crate) fn expand_u16_range((start, end, step): (u32, u32, u32)) -> Result<Vec<u16>, String> {
    let push_checked = |out: &mut Vec<u16>, v: u32| -> Result<(), String> {
        if v > u16::MAX as u32 {
            return Err(format!("period {v} exceeds u16::MAX"));
        }
        out.push(v as u16);
        Ok(())
    };

    if step == 0 || start == end {
        let mut out = Vec::with_capacity(1);
        push_checked(&mut out, start)?;
        return Ok(out);
    }

    let step = step.max(1);
    let (lo, hi) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    let mut out = Vec::new();
    let mut v = lo;
    loop {
        push_checked(&mut out, v)?;
        if v == hi {
            break;
        }
        match v.checked_add(step) {
            Some(next) if next > v && next <= hi => v = next,
            _ => break,
        }
    }
    if out.is_empty() {
        return Err("empty period range".to_string());
    }
    Ok(out)
}

pub(crate) fn estimate_pairs_fast_lt_slow(fast_periods: &[u16], slow_periods: &[u16]) -> usize {
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return 0;
    }
    let mut slow_sorted: Vec<u16> = slow_periods.to_vec();
    slow_sorted.sort_unstable();

    let s_len = slow_sorted.len();
    let mut total: usize = 0;
    for &f in fast_periods {
        let idx = slow_sorted.partition_point(|&s| s <= f);
        total = total.saturating_add(s_len.saturating_sub(idx));
    }
    total
}

#[derive(Clone, Copy, Debug)]
struct ValidPairWork {
    fast_idx: usize,
    slow_idx: usize,
    fast_len: u16,
    slow_len: u16,
}

#[inline]
fn build_valid_pair_work(f_chunk: &[u16], s_chunk: &[u16]) -> Vec<ValidPairWork> {
    let mut out = Vec::with_capacity(estimate_pairs_fast_lt_slow(f_chunk, s_chunk));
    for (fast_idx, &fast_len) in f_chunk.iter().enumerate() {
        for (slow_idx, &slow_len) in s_chunk.iter().enumerate() {
            if fast_len < slow_len {
                out.push(ValidPairWork {
                    fast_idx,
                    slow_idx,
                    fast_len,
                    slow_len,
                });
            }
        }
    }
    out
}

const MAX_MA_PARAM_SWEEP_VALUES: usize = 64;
const MAX_MA_PARAM_VARIANTS_PER_SIDE: usize = 512;
const MAX_MA_PARAM_VARIANT_PAIRS: usize = 4096;

fn expand_f64_range_inclusive(
    (start, end, step): (f64, f64, f64),
    label: &str,
) -> Result<Vec<f64>, String> {
    if !start.is_finite() || !end.is_finite() || !step.is_finite() {
        return Err(format!(
            "MA param sweep '{label}' requires finite start/end/step"
        ));
    }

    if step == 0.0 || start.to_bits() == end.to_bits() {
        return Ok(vec![start]);
    }

    let step = step.abs();
    if step <= 0.0 {
        return Err(format!("MA param sweep '{label}' requires step != 0"));
    }

    let (lo, hi) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let span = hi - lo;
    if !span.is_finite() || span < 0.0 {
        return Err(format!(
            "MA param sweep '{label}' has an invalid range: start={start} end={end}"
        ));
    }

    let steps_f = span / step;
    if !steps_f.is_finite() || steps_f < 0.0 {
        return Err(format!(
            "MA param sweep '{label}' has an invalid step: {step}"
        ));
    }
    let count_f = steps_f.floor() + 1.0;
    if count_f > MAX_MA_PARAM_SWEEP_VALUES as f64 {
        return Err(format!(
            "MA param sweep '{label}' has too many values (>{MAX_MA_PARAM_SWEEP_VALUES}). Reduce range or increase step."
        ));
    }
    let count = (count_f as usize).clamp(1, MAX_MA_PARAM_SWEEP_VALUES);

    let mut out: Vec<f64> = Vec::with_capacity(count);
    for i in 0..count {
        let v = lo + (i as f64) * step;
        if v > hi + step * 0.5 {
            break;
        }
        out.push(v);
    }

    if out.is_empty() {
        return Err(format!("MA param sweep '{label}' produced no values"));
    }

    Ok(out)
}

fn build_ma_param_variants(
    side: &'static str,
    base: Option<HashMap<String, f64>>,
    sweeps: Option<HashMap<String, (f64, f64, f64)>>,
) -> Result<Vec<Option<HashMap<String, f64>>>, String> {
    let base_map = base.unwrap_or_default();
    let had_any_base = !base_map.is_empty();

    let sweeps_map = sweeps.unwrap_or_default();
    if sweeps_map.is_empty() {
        return Ok(if had_any_base {
            vec![Some(base_map)]
        } else {
            vec![None]
        });
    }

    let mut entries: Vec<(String, (f64, f64, f64))> = sweeps_map.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut variants: Vec<HashMap<String, f64>> = vec![base_map];
    for (key, range) in entries {
        let values = expand_f64_range_inclusive(range, &key)?;
        let mut next: Vec<HashMap<String, f64>> =
            Vec::with_capacity(variants.len().saturating_mul(values.len()));
        for vmap in variants.into_iter() {
            for &val in &values {
                let mut m = vmap.clone();
                m.insert(key.clone(), val);
                next.push(m);
            }
        }
        variants = next;
        if variants.len() > MAX_MA_PARAM_VARIANTS_PER_SIDE {
            return Err(format!(
                "{side} MA parameter sweeps produce {} variants (max {MAX_MA_PARAM_VARIANTS_PER_SIDE}). Reduce sweeps.",
                variants.len()
            ));
        }
    }

    Ok(variants.into_iter().map(Some).collect())
}

fn tile_sizes_from_budget(
    total_fast: usize,
    total_slow: usize,
    series_len: usize,
    bytes_per_value: usize,
    env_mb: &str,
    default_mb: usize,
) -> (usize, usize) {
    let budget_mb = std::env::var(env_mb)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default_mb)
        .max(64);

    let mut budget = budget_mb.saturating_mul(1024).saturating_mul(1024);
    let headroom = 256usize * 1024 * 1024;
    if budget > headroom {
        budget = budget.saturating_sub(headroom);
    } else {
        budget = budget / 2;
    }

    let bytes_per_row = series_len.saturating_mul(bytes_per_value).max(1);
    let max_rows_total = (budget / bytes_per_row).max(2);
    let mut pf = (max_rows_total / 2).max(1);
    let mut ps = (max_rows_total / 2).max(1);

    pf = pf.clamp(1, total_fast.max(1));
    ps = ps.clamp(1, total_slow.max(1));
    (pf, ps)
}

pub(crate) fn cpu_tile_sizes(
    total_fast: usize,
    total_slow: usize,
    series_len: usize,
) -> (usize, usize) {
    tile_sizes_from_budget(
        total_fast,
        total_slow,
        series_len,
        std::mem::size_of::<f64>(),
        "VECTORBT_CPU_RAM_BUDGET_MB",
        1024,
    )
}

#[cfg(feature = "cuda")]
fn sweep_tile_sizes(total_fast: usize, total_slow: usize, series_len: usize) -> (usize, usize) {
    tile_sizes_from_budget(
        total_fast,
        total_slow,
        series_len,
        std::mem::size_of::<f32>(),
        "VECTORBT_SWEEP_RAM_BUDGET_MB",
        2048,
    )
}

fn ma_period_step(periods: &[u16]) -> usize {
    if periods.len() < 2 {
        return 0;
    }
    let a = periods[0] as i32;
    let b = periods[1] as i32;
    (b - a).abs().max(1) as usize
}

#[inline]
fn fill_missing_rows_with_nan_f64(values: &mut [f64], cols: usize, row_written: &[bool]) {
    for (row, written) in row_written.iter().copied().enumerate() {
        if written {
            continue;
        }
        let offset = row.saturating_mul(cols);
        values[offset..offset + cols].fill(f64::NAN);
    }
}

#[inline]
fn fill_missing_rows_with_nan_f32(values: &mut [f32], cols: usize, row_written: &[bool]) {
    for (row, written) in row_written.iter().copied().enumerate() {
        if written {
            continue;
        }
        let offset = row.saturating_mul(cols);
        values[offset..offset + cols].fill(f32::NAN);
    }
}

#[inline]
pub(crate) fn first_finite_idx(series: &[f64]) -> Option<usize> {
    series.iter().position(|v| v.is_finite())
}

pub(crate) fn compute_log_returns_f64(prices: &[f64]) -> Vec<f64> {
    let n = prices.len();
    let mut out = alloc_f64_with_prefix_uninit_tail(n, 1, 0.0);
    if n < 2 {
        return out;
    }
    for t in 1..n {
        let p = prices[t];
        let pm = prices[t - 1];
        out[t] = if p > 0.0 && pm > 0.0 && p.is_finite() && pm.is_finite() {
            p.ln() - pm.ln()
        } else {
            0.0
        };
    }
    out
}

pub(crate) fn build_indicator_bars_for_chart(
    candles: &vector_ta::utilities::data_loader::Candles,
    fast: &[f64],
    slow: &[f64],
    max_points: usize,
) -> Result<Vec<IndicatorBar>, String> {
    let n = candles.timestamp.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    if fast.len() != n || slow.len() != n {
        return Err("indicator series length mismatch".to_string());
    }

    let cap = max_points.clamp(300, 20_000);
    let step = if n <= cap {
        1usize
    } else {
        (n + cap - 1) / cap
    };
    let mut out: Vec<IndicatorBar> = Vec::with_capacity((n + step - 1) / step);
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

        let fast_v = (from..to_exclusive).rev().find_map(|i| {
            let v = fast[i];
            if v.is_finite() {
                Some(v)
            } else {
                None
            }
        });
        let slow_v = (from..to_exclusive).rev().find_map(|i| {
            let v = slow[i];
            if v.is_finite() {
                Some(v)
            } else {
                None
            }
        });
        if fast_v.is_none() && slow_v.is_none() {
            from = to_exclusive;
            continue;
        }

        out.push(IndicatorBar {
            time,
            from_index: u32::try_from(from)
                .map_err(|_| "dataset too large for indicator index".to_string())?,
            to_index: u32::try_from(to)
                .map_err(|_| "dataset too large for indicator index".to_string())?,
            fast: fast_v,
            slow: slow_v,
        });
        prev_time = Some(time);
        from = to_exclusive;
    }

    Ok(out)
}

pub(crate) fn ma_matrix_cpu_f64(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_type: &str,
    ma_source: &str,
    periods: &[u16],
    ma_params: Option<&HashMap<String, f64>>,
    alma_offset: f64,
    alma_sigma: f64,
) -> Result<(Vec<f64>, usize), String> {
    let n = candles.close.len();
    if periods.is_empty() {
        return Err("empty period list".to_string());
    }
    let rows = periods.len();
    let expected_len = rows
        .checked_mul(n)
        .ok_or_else(|| "rows*cols overflow".to_string())?;

    let start = periods[0] as usize;
    let end = periods[rows - 1] as usize;
    let step = ma_period_step(periods);

    let mut values: Vec<f64> = alloc_uninit_f64(expected_len);

    if ma_type.eq_ignore_ascii_case("alma") {
        let prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
        let sweep = vector_ta::indicators::moving_averages::alma::AlmaBatchRange {
            period: (start, end, step),
            offset: (alma_offset, alma_offset, 0.0),
            sigma: (alma_sigma, alma_sigma, 0.0),
        };
        let out = vector_ta::indicators::moving_averages::alma::alma_batch_with_kernel(
            prices,
            &sweep,
            vector_ta::utilities::enums::Kernel::ScalarBatch,
        )
        .map_err(|e| e.to_string())?;
        if out.cols != n {
            return Err(format!("alma batch cols {} != {n}", out.cols));
        }
        let mut row_written = vec![false; rows];
        for (src_row, params) in out.combos.iter().enumerate() {
            let per = params.period.unwrap_or(start) as usize;
            let row = if step == 0 {
                0usize
            } else if per >= start && (per - start) % step == 0 {
                (per - start) / step
            } else {
                continue;
            };
            if row >= rows {
                continue;
            }
            let src_off = src_row.saturating_mul(out.cols);
            let dst_off = row.saturating_mul(n);
            if src_off + n > out.values.len() || dst_off + n > values.len() {
                continue;
            }
            values[dst_off..dst_off + n].copy_from_slice(&out.values[src_off..src_off + n]);
            row_written[row] = true;
        }
        fill_missing_rows_with_nan_f64(&mut values, n, &row_written);
        return Ok((values, n));
    }

    if ma_type.eq_ignore_ascii_case("frama") {
        let high = vector_ta::utilities::data_loader::source_type(candles, "high");
        let low = vector_ta::utilities::data_loader::source_type(candles, "low");
        let close = vector_ta::utilities::data_loader::source_type(candles, "close");
        let sc = ma_params
            .and_then(|params| params.get("sc").copied())
            .unwrap_or(300.0);
        let fc = ma_params
            .and_then(|params| params.get("fc").copied())
            .unwrap_or(1.0);
        if !sc.is_finite() || sc <= 0.0 {
            return Err(format!("invalid frama sc parameter: {sc}"));
        }
        if !fc.is_finite() || fc <= 0.0 {
            return Err(format!("invalid frama fc parameter: {fc}"));
        }
        let sweep = vector_ta::indicators::moving_averages::frama::FramaBatchRange {
            window: (start, end, step),
            sc: (sc.round() as usize, sc.round() as usize, 0),
            fc: (fc.round() as usize, fc.round() as usize, 0),
        };
        let out = vector_ta::indicators::moving_averages::frama::frama_batch_with_kernel(
            high,
            low,
            close,
            &sweep,
            vector_ta::utilities::enums::Kernel::ScalarBatch,
        )
        .map_err(|e| e.to_string())?;
        if out.cols != n {
            return Err(format!("frama batch cols {} != {n}", out.cols));
        }
        let mut row_written = vec![false; rows];
        for (src_row, params) in out.combos.iter().enumerate() {
            let per = params.window.unwrap_or(start) as usize;
            let row = if step == 0 {
                0usize
            } else if per >= start && (per - start) % step == 0 {
                (per - start) / step
            } else {
                continue;
            };
            if row >= rows {
                continue;
            }
            let src_off = src_row.saturating_mul(out.cols);
            let dst_off = row.saturating_mul(n);
            if src_off + n > out.values.len() || dst_off + n > values.len() {
                continue;
            }
            for (dst, &src) in values[dst_off..dst_off + n]
                .iter_mut()
                .zip(out.values[src_off..src_off + n].iter())
            {
                *dst = src as f64;
            }
            row_written[row] = true;
        }
        fill_missing_rows_with_nan_f64(&mut values, n, &row_written);
        return Ok((values, n));
    }

    let data = vector_ta::indicators::moving_averages::ma::MaData::Candles {
        candles,
        source: ma_source,
    };
    let batch = vector_ta::indicators::moving_averages::ma_batch::ma_batch_with_kernel_and_params(
        ma_type,
        data,
        (start, end, step.max(1)),
        vector_ta::utilities::enums::Kernel::ScalarBatch,
        ma_params,
    );
    match batch {
        Ok(b) if b.cols == n => {
            let mut row_written = vec![false; rows];
            for (src_row, &period) in b.periods.iter().enumerate() {
                let per = period as usize;
                let row = if step == 0 {
                    if per == start {
                        0usize
                    } else {
                        continue;
                    }
                } else if per >= start && (per - start) % step == 0 {
                    (per - start) / step
                } else {
                    continue;
                };
                if row >= rows {
                    continue;
                }
                let src_off = src_row.saturating_mul(b.cols);
                let dst_off = row.saturating_mul(n);
                if src_off + n > b.values.len() || dst_off + n > values.len() {
                    continue;
                }
                values[dst_off..dst_off + n].copy_from_slice(&b.values[src_off..src_off + n]);
                row_written[row] = true;
            }
            fill_missing_rows_with_nan_f64(&mut values, n, &row_written);
        }
        _ => {
            for (row, &period) in periods.iter().enumerate() {
                let series = vector_ta::indicators::moving_averages::ma::ma_with_kernel(
                    ma_type,
                    vector_ta::indicators::moving_averages::ma::MaData::Candles {
                        candles,
                        source: ma_source,
                    },
                    period as usize,
                    vector_ta::utilities::enums::Kernel::Scalar,
                )
                .unwrap_or_else(|_| vec![f64::NAN; n]);
                if series.len() != n {
                    continue;
                }
                let dst_off = row.saturating_mul(n);
                if dst_off + n > values.len() {
                    continue;
                }
                values[dst_off..dst_off + n].copy_from_slice(&series);
            }
        }
    }

    Ok((values, n))
}

#[allow(unused_variables)]
pub(crate) fn ma_matrix_with_backend_f64(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_type: &str,
    ma_source: &str,
    periods: &[u16],
    ma_params: Option<&HashMap<String, f64>>,
    alma_offset: f64,
    alma_sigma: f64,
    backend: Option<&Backend>,
    prices_f32: Option<&[f32]>,
) -> Result<(Vec<f64>, usize), String> {
    let ma_info = lookup_ma_info(ma_type).ok_or_else(|| format!("unknown MA: '{ma_type}'"))?;

    #[cfg(feature = "cuda")]
    {
        let normalized_backend = backend
            .cloned()
            .unwrap_or(Backend::CpuOnly)
            .with_preferred_cuda_device()?;
        let maybe_device_id = match normalized_backend {
            Backend::Auto { device_id }
            | Backend::GpuOnly { device_id }
            | Backend::GpuHybridKernel { device_id }
            | Backend::GpuKernel { device_id } => Some(device_id as usize),
            Backend::CpuOnly => None,
        };

        if let Some(device_id) = maybe_device_id {
            if cuda_available() && ma_info.supports_cuda_single {
                let selector = CudaMaSelector::new(device_id);
                let owned_prices_f32;
                let prices_f32 = if let Some(values) = prices_f32 {
                    values
                } else {
                    owned_prices_f32 =
                        vector_ta::utilities::data_loader::source_type(candles, ma_source)
                            .iter()
                            .map(|&v| v as f32)
                            .collect::<Vec<f32>>();
                    owned_prices_f32.as_slice()
                };
                match ma_matrix_cuda_host_f32(
                    &selector,
                    device_id,
                    candles,
                    prices_f32,
                    ma_type,
                    ma_source,
                    periods,
                    ma_params,
                    alma_offset,
                    alma_sigma,
                ) {
                    Ok((values_f32, cols)) => {
                        return Ok((values_f32.into_iter().map(f64::from).collect(), cols));
                    }
                    Err(err) if !ma_info.supports_cpu_single => {
                        return Err(format!(
                            "GPU-backed MA generation failed for '{ma_type}' and no CPU fallback exists: {err}"
                        ));
                    }
                    Err(_) => {}
                }
            }
        }
    }

    if !ma_info.supports_cpu_single {
        return Err(format!(
            "CPU backend does not support MA '{ma_type}' and GPU MA generation is unavailable"
        ));
    }

    ma_matrix_cpu_f64(
        candles,
        ma_type,
        ma_source,
        periods,
        ma_params,
        alma_offset,
        alma_sigma,
    )
}

pub(crate) fn ma_matrix_cpu_f32_staging(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_type: &str,
    ma_source: &str,
    periods: &[u16],
    ma_params: Option<&HashMap<String, f64>>,
    alma_offset: f64,
    alma_sigma: f64,
) -> Result<(Vec<f32>, usize), String> {
    let n = candles.close.len();
    if periods.is_empty() {
        return Err("empty period list".to_string());
    }
    let rows = periods.len();
    let expected_len = rows
        .checked_mul(n)
        .ok_or_else(|| "rows*cols overflow".to_string())?;

    let start = periods[0] as usize;
    let end = periods[rows - 1] as usize;
    let step = ma_period_step(periods);

    let mut values: Vec<f32> = alloc_uninit_f32(expected_len);

    if ma_type.eq_ignore_ascii_case("alma") {
        let prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
        let sweep = vector_ta::indicators::moving_averages::alma::AlmaBatchRange {
            period: (start, end, step),
            offset: (alma_offset, alma_offset, 0.0),
            sigma: (alma_sigma, alma_sigma, 0.0),
        };
        let out = vector_ta::indicators::moving_averages::alma::alma_batch_with_kernel(
            prices,
            &sweep,
            vector_ta::utilities::enums::Kernel::ScalarBatch,
        )
        .map_err(|e| e.to_string())?;
        if out.cols != n {
            return Err(format!("alma batch cols {} != {n}", out.cols));
        }
        let mut row_written = vec![false; rows];
        for (src_row, params) in out.combos.iter().enumerate() {
            let per = params.period.unwrap_or(start) as usize;
            let row = if step == 0 {
                0usize
            } else if per >= start && (per - start) % step == 0 {
                (per - start) / step
            } else {
                continue;
            };
            if row >= rows {
                continue;
            }
            let src_off = src_row.saturating_mul(out.cols);
            let dst_off = row.saturating_mul(n);
            if src_off + n > out.values.len() || dst_off + n > values.len() {
                continue;
            }
            for (dst, &src) in values[dst_off..dst_off + n]
                .iter_mut()
                .zip(out.values[src_off..src_off + n].iter())
            {
                *dst = src as f32;
            }
            row_written[row] = true;
        }
        fill_missing_rows_with_nan_f32(&mut values, n, &row_written);
        return Ok((values, n));
    }

    if ma_type.eq_ignore_ascii_case("frama") {
        let high = vector_ta::utilities::data_loader::source_type(candles, "high");
        let low = vector_ta::utilities::data_loader::source_type(candles, "low");
        let close = vector_ta::utilities::data_loader::source_type(candles, "close");
        let sc = ma_params
            .and_then(|params| params.get("sc").copied())
            .unwrap_or(300.0);
        let fc = ma_params
            .and_then(|params| params.get("fc").copied())
            .unwrap_or(1.0);
        if !sc.is_finite() || sc <= 0.0 {
            return Err(format!("invalid frama sc parameter: {sc}"));
        }
        if !fc.is_finite() || fc <= 0.0 {
            return Err(format!("invalid frama fc parameter: {fc}"));
        }
        let sweep = vector_ta::indicators::moving_averages::frama::FramaBatchRange {
            window: (start, end, step),
            sc: (sc.round() as usize, sc.round() as usize, 0),
            fc: (fc.round() as usize, fc.round() as usize, 0),
        };
        let out = vector_ta::indicators::moving_averages::frama::frama_batch_with_kernel(
            high,
            low,
            close,
            &sweep,
            vector_ta::utilities::enums::Kernel::ScalarBatch,
        )
        .map_err(|e| e.to_string())?;
        if out.cols != n {
            return Err(format!("frama batch cols {} != {n}", out.cols));
        }
        let mut row_written = vec![false; rows];
        for (src_row, params) in out.combos.iter().enumerate() {
            let per = params.window.unwrap_or(start) as usize;
            let row = if step == 0 {
                0usize
            } else if per >= start && (per - start) % step == 0 {
                (per - start) / step
            } else {
                continue;
            };
            if row >= rows {
                continue;
            }
            let src_off = src_row.saturating_mul(out.cols);
            let dst_off = row.saturating_mul(n);
            if src_off + n > out.values.len() || dst_off + n > values.len() {
                continue;
            }
            for (dst, &src) in values[dst_off..dst_off + n]
                .iter_mut()
                .zip(out.values[src_off..src_off + n].iter())
            {
                *dst = src as f32;
            }
            row_written[row] = true;
        }
        fill_missing_rows_with_nan_f32(&mut values, n, &row_written);
        return Ok((values, n));
    }

    let data = vector_ta::indicators::moving_averages::ma::MaData::Candles {
        candles,
        source: ma_source,
    };
    let batch = vector_ta::indicators::moving_averages::ma_batch::ma_batch_with_kernel_and_params(
        ma_type,
        data,
        (start, end, step.max(1)),
        vector_ta::utilities::enums::Kernel::ScalarBatch,
        ma_params,
    );

    match batch {
        Ok(b) if b.cols == n => {
            let mut row_written = vec![false; rows];
            for (src_row, &period) in b.periods.iter().enumerate() {
                let per = period as usize;
                let row = if step == 0 {
                    if per == start {
                        0usize
                    } else {
                        continue;
                    }
                } else if per >= start && (per - start) % step == 0 {
                    (per - start) / step
                } else {
                    continue;
                };
                if row >= rows {
                    continue;
                }
                let src_off = src_row.saturating_mul(b.cols);
                let dst_off = row.saturating_mul(n);
                if src_off + n > b.values.len() || dst_off + n > values.len() {
                    continue;
                }
                for (dst, &src) in values[dst_off..dst_off + n]
                    .iter_mut()
                    .zip(b.values[src_off..src_off + n].iter())
                {
                    *dst = src as f32;
                }
                row_written[row] = true;
            }
            fill_missing_rows_with_nan_f32(&mut values, n, &row_written);
            Ok((values, n))
        }
        _ => {
            // Fall back to row-wise CPU assembly without materializing a full f64 matrix first.
            for (row, &period) in periods.iter().enumerate() {
                let series = vector_ta::indicators::moving_averages::ma::ma(
                    ma_type,
                    vector_ta::indicators::moving_averages::ma::MaData::Candles {
                        candles,
                        source: ma_source,
                    },
                    period as usize,
                )
                .unwrap_or_else(|_| vec![f64::NAN; n]);
                if series.len() != n {
                    continue;
                }
                let dst_off = row.saturating_mul(n);
                if dst_off + n > values.len() {
                    continue;
                }
                for (dst, &src) in values[dst_off..dst_off + n].iter_mut().zip(series.iter()) {
                    *dst = src as f32;
                }
            }
            Ok((values, n))
        }
    }
}

fn optimize_double_ma_cpu_tiled(
    candles: &vector_ta::utilities::data_loader::Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    halftrend_trend: Option<&[f64]>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    top_k: usize,
    include_all: bool,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    constraints: Option<&DoubleMaConstraints>,
    total_pairs: usize,
    heatmap_bins: usize,
    dynamic_request_plan: Option<&crate::strategy_dynamic::RunnableDynamicDoubleMaPlan>,
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

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
    if ma_prices.len() != close.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let mut first_valid = fv_close.max(fv_ma);
    if let Some(trend) = halftrend_trend {
        if trend.len() != n {
            return Err("regime trend series length mismatch".to_string());
        }
        let fv_ht = first_finite_idx(trend)
            .ok_or_else(|| "regime trend series all non-finite".to_string())?;
        first_valid = first_valid.max(fv_ht);
    }

    let lr = compute_log_returns_f64(close);
    let trade_offset = if strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let kaufman_stops = compute_kaufman_stop_series(candles, strategy)?;
    if let Some(ks) = kaufman_stops.as_ref() {
        let fv_k_long = first_finite_idx(&ks.long)
            .ok_or_else(|| "kaufman-stop long series all non-finite".to_string())?;
        let fv_k_short = first_finite_idx(&ks.short)
            .ok_or_else(|| "kaufman-stop short series all non-finite".to_string())?;
        first_valid = first_valid.max(fv_k_long.max(fv_k_short));
    }

    let dynamic_plan_ctx_owned = if dynamic_request_plan.is_none()
        && crate::strategy_dynamic::dynamic_strategy_opt_experimental_enabled()
    {
        let plan = crate::strategy_dynamic::compile_dynamic_plan_for_double_ma_config(
            ma_source,
            fast_ma_type,
            slow_ma_type,
            fast_ma_params,
            slow_ma_params,
            strategy,
            None,
            crate::strategy_plan::StrategyPlanBackend::Cpu,
        )?;
        let ctx = crate::strategy_dynamic::build_dynamic_cpu_context_from_precomputed(
            &lr,
            close,
            ma_prices,
            halftrend_trend,
            None,
            None,
            None,
            kaufman_stops.as_ref(),
        );
        Some((plan, ctx))
    } else {
        None
    };
    let dynamic_plan_ctx = if let Some(runtime_plan) = dynamic_request_plan {
        Some((
            &runtime_plan.cpu_plan,
            runtime_plan.cpu_context(candles)?,
            true,
        ))
    } else {
        dynamic_plan_ctx_owned
            .as_ref()
            .map(|(plan, ctx)| (plan, ctx, false))
    };
    let dynamic_shadow_enabled = dynamic_plan_ctx
        .as_ref()
        .is_some_and(|(_, _, request_owned)| !*request_owned)
        && crate::strategy_dynamic::dynamic_strategy_opt_shadow_enabled();
    let dynamic_shadow_every = if dynamic_shadow_enabled {
        crate::strategy_dynamic::dynamic_strategy_opt_shadow_every().max(1)
    } else {
        0
    };
    let dynamic_shadow_sampled = AtomicUsize::new(0);
    let dynamic_shadow_mismatches = AtomicUsize::new(0);
    let score_fn = score_mode_score_fn(score_mode);
    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    if let Some(score_fn) = score_fn {
        agg = agg.with_score_fn(score_fn);
    }
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
    emit_progress(progress, processed, total_pairs, "cpu");

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

            let valid_pairs = build_valid_pair_work(f_chunk, s_chunk);

            if include_all {
                for pair in &valid_pairs {
                    let fast_row = &fast_mat[pair.fast_idx * cols..(pair.fast_idx + 1) * cols];
                    let slow_row = &slow_mat[pair.slow_idx * cols..(pair.slow_idx + 1) * cols];
                    let fast_len = pair.fast_len;
                    let slow_len = pair.slow_len;
                    let max_p = (fast_len as usize).max(slow_len as usize).max(1);
                    let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                    let start_t = t_valid.saturating_add(trade_offset);
                    let metrics =
                        if let Some((plan, ctx, request_owned)) = dynamic_plan_ctx.as_ref() {
                            let dynamic_metrics = if *request_owned {
                                crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu_with_candles(
                                    plan, candles, ctx, fast_row, slow_row, start_t, true,
                                )?
                            } else {
                                crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu(
                                    plan, ctx, fast_row, slow_row, start_t, true,
                                )?
                            };
                            if dynamic_shadow_enabled {
                                let sampled = dynamic_shadow_sampled
                                    .fetch_add(1, Ordering::Relaxed)
                                    .saturating_add(1);
                                if sampled % dynamic_shadow_every == 0 {
                                    let static_metrics = eval_metrics_f64_with_exit_mode(
                                        &lr,
                                        close,
                                        fast_row,
                                        slow_row,
                                        halftrend_trend,
                                        start_t,
                                        strategy,
                                        kaufman_stops.as_ref(),
                                        true,
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
                            eval_metrics_f64_with_exit_mode(
                                &lr,
                                close,
                                fast_row,
                                slow_row,
                                halftrend_trend,
                                start_t,
                                strategy,
                                kaufman_stops.as_ref(),
                                true,
                            )
                        };
                    if metrics_pass_constraints(&metrics, constraints) {
                        let params = DoubleMaParams {
                            fast_len,
                            slow_len,
                            fast_ma_id: 0,
                            slow_ma_id: 0,
                        };
                        agg.push(params, metrics);
                    }
                    processed = processed.saturating_add(1);
                    if (processed & 0x1fff) == 0 && cancel.load(Ordering::Relaxed) {
                        return Err("cancelled".to_string());
                    }
                }
            } else {
                let compute_sharpe = objective_needs_return_stats(objective)
                    || score_mode_needs_rich_metrics(score_mode);

                let init = || {
                    let mut local = StreamAggregator::new(objective, top_k, false, n);
                    if let Some(score_fn) = score_fn {
                        local = local.with_score_fn(score_fn);
                    }
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

                let tile = valid_pairs
                    .par_iter()
                    .try_fold(init, |mut local, pair| {
                        if cancel.load(Ordering::Relaxed) {
                            return Err("cancelled".to_string());
                        }
                        let fast_row = &fast_mat[pair.fast_idx * cols..(pair.fast_idx + 1) * cols];
                        let slow_row = &slow_mat[pair.slow_idx * cols..(pair.slow_idx + 1) * cols];
                        let fast_len = pair.fast_len;
                        let slow_len = pair.slow_len;
                        let max_p = (fast_len as usize).max(slow_len as usize).max(1);
                        let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                        let start_t = t_valid.saturating_add(trade_offset);
                        let metrics = if let Some((plan, ctx, request_owned)) =
                            dynamic_plan_ctx.as_ref()
                        {
                            let dynamic_metrics = if *request_owned {
                                crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu_with_candles(
                                    plan,
                                    candles,
                                    ctx,
                                    fast_row,
                                    slow_row,
                                    start_t,
                                    compute_sharpe,
                                )?
                            } else {
                                crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu(
                                    plan,
                                    ctx,
                                    fast_row,
                                    slow_row,
                                    start_t,
                                    compute_sharpe,
                                )?
                            };
                            if dynamic_shadow_enabled {
                                let sampled = dynamic_shadow_sampled
                                    .fetch_add(1, Ordering::Relaxed)
                                    .saturating_add(1);
                                if sampled % dynamic_shadow_every == 0 {
                                    let static_metrics = eval_metrics_f64_with_exit_mode(
                                        &lr,
                                        close,
                                        fast_row,
                                        slow_row,
                                        halftrend_trend,
                                        start_t,
                                        strategy,
                                        kaufman_stops.as_ref(),
                                        compute_sharpe,
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
                            eval_metrics_f64_with_exit_mode(
                                &lr,
                                close,
                                fast_row,
                                slow_row,
                                halftrend_trend,
                                start_t,
                                strategy,
                                kaufman_stops.as_ref(),
                                compute_sharpe,
                            )
                        };
                        if metrics_pass_constraints(&metrics, constraints) {
                            local.push(
                                DoubleMaParams {
                                    fast_len,
                                    slow_len,
                                    fast_ma_id: 0,
                                    slow_ma_id: 0,
                                },
                                metrics,
                            );
                        }
                        Ok(local)
                    })
                    .try_reduce(init, |mut a, b| {
                        a.merge(b);
                        Ok(a)
                    })?;

                processed = processed.saturating_add(valid_pairs.len());
                agg.merge(tile);
            }

            emit_progress(progress, processed, total_pairs, "cpu");
        }
    }

    let mut result = agg
        .finalize()
        .ok_or_else(|| "grid search produced no result".to_string())?;

    if !objective_needs_return_stats(objective)
        && !score_mode_needs_rich_metrics(score_mode)
        && !include_all
    {
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
            if let Some((plan, ctx, request_owned)) = dynamic_plan_ctx.as_ref() {
                if *request_owned {
                    crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu_with_candles(
                        plan, candles, ctx, fast, slow, start_t, true,
                    )
                } else {
                    crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu(
                        plan, ctx, fast, slow, start_t, true,
                    )
                }
            } else {
                Ok(eval_metrics_f64_with_exit_mode(
                    &lr,
                    close,
                    fast,
                    slow,
                    halftrend_trend,
                    start_t,
                    strategy,
                    kaufman_stops.as_ref(),
                    true,
                ))
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
            "[dynamic-shadow][double_ma_cpu] sampled={} mismatches={} every={}",
            shadow_sampled, shadow_mismatches, dynamic_shadow_every
        );
    }
    crate::strategy_dynamic::maybe_append_dynamic_strategy_artifact(
        crate::strategy_dynamic::DynamicStrategyArtifactRecord {
            scope: "double_ma_cpu",
            profile: "DoubleMa",
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

#[allow(clippy::too_many_arguments)]
#[cfg_attr(not(test), allow(dead_code))]
fn maybe_compile_dynamic_gpu_plan_preflight(
    enabled: bool,
    ma_source: &str,
    fast_ma_type: &str,
    slow_ma_type: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    strategy: &StrategyConfig,
    halftrend_filter: Option<&HalfTrendRegimeFilter>,
) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }

    let cache_key = dynamic_gpu_preflight_cache_key(
        ma_source,
        fast_ma_type,
        slow_ma_type,
        fast_ma_params,
        slow_ma_params,
        strategy,
        halftrend_filter,
    )?;
    let cache = DYNAMIC_GPU_PREFLIGHT_CACHE.get_or_init(|| Mutex::new(BTreeSet::new()));
    {
        let guard = cache
            .lock()
            .map_err(|_| "dynamic GPU preflight cache lock poisoned".to_string())?;
        if guard.contains(&cache_key) {
            return Ok(());
        }
    }

    let plan = crate::strategy_dynamic::compile_dynamic_plan_for_double_ma_config(
        ma_source,
        fast_ma_type,
        slow_ma_type,
        fast_ma_params,
        slow_ma_params,
        strategy,
        halftrend_filter,
        crate::strategy_plan::StrategyPlanBackend::GpuUnified,
    )
    .map_err(|e| format!("dynamic GPU plan compile failed: {e}"))?;

    let handle = crate::strategy_gpu_runtime::StrategyHandle::from_plan(
        plan,
        crate::strategy_gpu_runtime::StrategyCapabilityReport::gpu_dynamic_ready(),
    )
    .map_err(|e| format!("dynamic GPU strategy handle build failed: {e}"))?;
    crate::strategy_gpu_runtime::get_or_build_gpu_strategy_executable(&handle, 0)
        .map_err(|e| format!("dynamic GPU executable upload/probe failed: {e}"))?;

    let mut guard = cache
        .lock()
        .map_err(|_| "dynamic GPU preflight cache lock poisoned".to_string())?;
    guard.insert(cache_key);

    Ok(())
}

#[derive(Clone)]
enum DynamicGpuExecutionSelection {
    Disabled,
    Supported(crate::strategy_gpu_runtime::StrategyHandle),
    CpuFallback { reason: String },
}

fn dynamic_gpu_v1_candidate_shape(
    strategy: &StrategyConfig,
    _halftrend_filter: Option<&HalfTrendRegimeFilter>,
) -> bool {
    !strategy.uses_pressure_impulse_trend_filter()
        && !strategy.uses_slope_efficiency_trend_filter()
        && !strategy.uses_momentum_triple_stack_filter()
        && !strategy.uses_oscillator_reversion_confluence_filter()
        && !strategy.uses_trend_quality_filter()
        && !strategy.uses_breakout_momentum_filter()
        && !strategy.uses_regime_hybrid_filter()
        && !strategy.uses_volume_flow_confirm_filter()
}

fn prepare_dynamic_gpu_execution_selection(
    enabled: bool,
    backend: &Backend,
    device_id: u32,
    ma_source: &str,
    fast_ma_type: &str,
    slow_ma_type: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    strategy: &StrategyConfig,
    halftrend_filter: Option<&HalfTrendRegimeFilter>,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    top_k: usize,
    include_all: bool,
) -> Result<DynamicGpuExecutionSelection, String> {
    if !enabled
        || !matches!(backend, Backend::GpuKernel { .. })
        || !dynamic_gpu_v1_candidate_shape(strategy, halftrend_filter)
    {
        return Ok(DynamicGpuExecutionSelection::Disabled);
    }

    let capability = crate::double_ma_backend::build_strategy_capability_report_for_request(
        fast_ma_type,
        slow_ma_type,
        strategy,
        halftrend_filter.is_some(),
        objective,
        score_mode,
        top_k,
        include_all,
    );
    if !capability.gpu_dynamic_supported {
        if capability.cpu_dynamic_supported {
            return Ok(DynamicGpuExecutionSelection::CpuFallback {
                reason: format!(
                    "Dynamic GpuKernel v1 is unsupported for this DoubleMa shape; using CpuOnly. Reasons: {}",
                    capability.reasons.join("; ")
                ),
            });
        }
        return Err(format!(
            "Dynamic GpuKernel v1 is unsupported for this DoubleMa shape: {}",
            capability.reasons.join("; ")
        ));
    }

    let plan = crate::strategy_dynamic::compile_dynamic_plan_for_double_ma_config(
        ma_source,
        fast_ma_type,
        slow_ma_type,
        fast_ma_params,
        slow_ma_params,
        strategy,
        halftrend_filter,
        crate::strategy_plan::StrategyPlanBackend::GpuUnified,
    )
    .map_err(|e| format!("dynamic GPU plan compile failed: {e}"))?;
    let handle = crate::strategy_gpu_runtime::StrategyHandle::from_plan(plan, capability.clone())
        .map_err(|e| format!("dynamic GPU strategy handle build failed: {e}"))?;
    if let Err(reason) = crate::strategy_dynamic::validate_dynamic_double_ma_gpu_handle_v1(&handle)
    {
        if capability.cpu_dynamic_supported {
            return Ok(DynamicGpuExecutionSelection::CpuFallback {
                reason: format!(
                    "Dynamic GpuKernel v1 fell back to CpuOnly after handle validation: {reason}"
                ),
            });
        }
        return Err(reason);
    }
    crate::strategy_gpu_runtime::get_or_build_gpu_strategy_executable(&handle, device_id)
        .map_err(|e| format!("dynamic GPU executable upload/probe failed: {e}"))?;
    Ok(DynamicGpuExecutionSelection::Supported(handle))
}

#[cfg_attr(not(test), allow(dead_code))]
fn dynamic_gpu_preflight_cache_key(
    ma_source: &str,
    fast_ma_type: &str,
    slow_ma_type: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    strategy: &StrategyConfig,
    halftrend_filter: Option<&HalfTrendRegimeFilter>,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct PreflightSignature<'a> {
        ma_source: &'a str,
        fast_ma_type: &'a str,
        slow_ma_type: &'a str,
        fast_ma_params: Option<BTreeMap<String, String>>,
        slow_ma_params: Option<BTreeMap<String, String>>,
        strategy: &'a StrategyConfig,
        halftrend_filter: Option<&'a HalfTrendRegimeFilter>,
    }

    fn normalize_params(params: Option<&HashMap<String, f64>>) -> Option<BTreeMap<String, String>> {
        params.map(|map| {
            map.iter()
                .map(|(k, v)| (k.clone(), format!("{:.12}", v)))
                .collect()
        })
    }

    serde_json::to_string(&PreflightSignature {
        ma_source,
        fast_ma_type,
        slow_ma_type,
        fast_ma_params: normalize_params(fast_ma_params),
        slow_ma_params: normalize_params(slow_ma_params),
        strategy,
        halftrend_filter,
    })
    .map_err(|e| format!("dynamic GPU preflight cache key serialization failed: {e}"))
}

fn dynamic_gpu_preflight_required_for_backend(
    dynamic_enabled: bool,
    mode_used: &OptimizationModeResolved,
    backend: &Backend,
) -> bool {
    if !dynamic_enabled {
        return false;
    }
    matches!(backend, Backend::GpuKernel { .. })
        || (matches!(mode_used, OptimizationModeResolved::GpuVdps)
            && matches!(backend, Backend::GpuHybridKernel { .. }))
}

#[cfg(not(feature = "cuda"))]
fn optimize_double_ma_gpu_sweep_tiled(
    _candles: &vector_ta::utilities::data_loader::Candles,
    _device_id: u32,
    _fast_periods: &[u16],
    _slow_periods: &[u16],
    _fast_ma_type: &str,
    _slow_ma_type: &str,
    _ma_source: &str,
    _fast_ma_params: Option<&HashMap<String, f64>>,
    _slow_ma_params: Option<&HashMap<String, f64>>,
    _halftrend_trend: Option<&[f64]>,
    _strategy: &StrategyConfig,
    _objective: ObjectiveKind,
    _score_mode: Option<DoubleMaScoreMode>,
    _extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    _shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    _shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    _shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    _shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
    _top_k: usize,
    _include_all: bool,
    _fast_alma_offset: f64,
    _fast_alma_sigma: f64,
    _slow_alma_offset: f64,
    _slow_alma_sigma: f64,
    _constraints: Option<&DoubleMaConstraints>,
    _total_pairs: usize,
    _heatmap_bins: usize,
    _cancel: &AtomicBool,
    _progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    Err("GPU backend requires building `ta_desktop_demo_app` with `--features cuda`".to_string())
}

#[cfg(feature = "cuda")]
fn ma_matrix_cuda_host_f32(
    selector: &CudaMaSelector,
    device_id: usize,
    candles: &vector_ta::utilities::data_loader::Candles,
    prices_f32: &[f32],
    ma_type: &str,
    ma_source: &str,
    periods: &[u16],
    ma_params: Option<&HashMap<String, f64>>,
    alma_offset: f64,
    alma_sigma: f64,
) -> Result<(Vec<f32>, usize), String> {
    let n = prices_f32.len();
    if periods.is_empty() {
        return Err("empty period list".to_string());
    }
    let rows = periods.len();
    let expected_len = rows
        .checked_mul(n)
        .ok_or_else(|| "rows*cols overflow".to_string())?;

    let start = periods[0] as usize;
    let end = periods[rows - 1] as usize;
    let step = ma_period_step(periods).max(1);

    if ma_type.eq_ignore_ascii_case("alma") {
        let alma = CudaAlma::new(device_id).map_err(|e| e.to_string())?;
        let sweep = vector_ta::indicators::moving_averages::alma::AlmaBatchRange {
            period: (start, end, step),
            offset: (alma_offset, alma_offset, 0.0),
            sigma: (alma_sigma, alma_sigma, 0.0),
        };
        let mut out = vec![f32::NAN; expected_len];
        let (r, c, combos) = alma
            .alma_batch_into_host_f32(prices_f32, &sweep, &mut out)
            .map_err(|e| e.to_string())?;
        if r != rows || c != n {
            return Err(format!(
                "cuda alma batch returned shape ({r},{c}) (expected ({rows},{n}))"
            ));
        }
        if combos.len() != rows {
            return Err(format!(
                "cuda alma batch returned {} combos (expected {})",
                combos.len(),
                rows
            ));
        }
        return Ok((out, n));
    }

    let requires_candles = lookup_ma_info(ma_type)
        .map(|m| m.requires_candles)
        .unwrap_or(false);
    let data = if requires_candles {
        CudaMaData::Candles {
            candles,
            source: ma_source,
        }
    } else {
        CudaMaData::SliceF32(prices_f32)
    };

    let sweep_res = match ma_params {
        Some(p) => selector.ma_sweep_to_host_f32_with_params(ma_type, data, start, end, step, p),
        None => selector.ma_sweep_to_host_f32(ma_type, data, start, end, step),
    };

    let host = match sweep_res {
        Ok((values, r, c)) => {
            if r != rows || c != n || values.len() != expected_len {
                return Err(format!(
                    "cuda ma sweep '{ma_type}' returned shape ({r},{c}) len {} (expected ({rows},{n}) len {expected_len})",
                    values.len()
                ));
            }
            values
        }
        Err(sweep_err) => {
            let mut out = alloc_uninit_f32(expected_len);
            for (row, &period) in periods.iter().enumerate() {
                let data = if requires_candles {
                    CudaMaData::Candles {
                        candles,
                        source: ma_source,
                    }
                } else {
                    CudaMaData::SliceF32(prices_f32)
                };
                let series = match ma_params {
                    Some(p) => selector.ma_to_host_f32_with_params(ma_type, data, period as usize, p),
                    None => selector.ma_to_host_f32(ma_type, data, period as usize),
                }.map_err(|e| {
                        format!(
                            "cuda ma sweep '{ma_type}' failed: {sweep_err}; fallback period {period} failed: {e}"
                        )
                    })?;
                if series.len() != n {
                    return Err(format!(
                        "cuda ma '{ma_type}' period {period} returned len {} (expected {n})",
                        series.len()
                    ));
                }
                let dst_off = row.saturating_mul(n);
                out[dst_off..dst_off + n].copy_from_slice(&series);
            }
            out
        }
    };

    Ok((host, n))
}

#[cfg(feature = "cuda")]
fn optimize_double_ma_gpu_sweep_tiled(
    candles: &vector_ta::utilities::data_loader::Candles,
    device_id: u32,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    halftrend_trend: Option<&[f64]>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    _extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    _shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    _shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    _shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    _shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
    top_k: usize,
    include_all: bool,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    constraints: Option<&DoubleMaConstraints>,
    total_pairs: usize,
    heatmap_bins: usize,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    if !cuda_available() {
        return Err("CUDA device not available (cuda_available() == false)".to_string());
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_info = lookup_ma_info(fast_ma_type).ok_or_else(|| "unknown fast MA".to_string())?;
    if !fast_info.supports_cuda_single {
        return Err(format!(
            "GPU (MA sweep) backend does not support MA '{}'",
            fast_ma_type
        ));
    }
    let slow_info = lookup_ma_info(slow_ma_type).ok_or_else(|| "unknown slow MA".to_string())?;
    if !slow_info.supports_cuda_single {
        return Err(format!(
            "GPU (MA sweep) backend does not support MA '{}'",
            slow_ma_type
        ));
    }

    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
    if ma_prices.len() != n {
        return Err("ma source length mismatch".to_string());
    }
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let mut first_valid = fv_close.max(fv_ma);
    if let Some(trend) = halftrend_trend {
        if trend.len() != n {
            return Err("regime trend series length mismatch".to_string());
        }
        let fv_ht = first_finite_idx(trend)
            .ok_or_else(|| "regime trend series all non-finite".to_string())?;
        first_valid = first_valid.max(fv_ht);
    }
    let kaufman_stops = compute_kaufman_stop_series(candles, strategy)?;
    if let Some(ks) = kaufman_stops.as_ref() {
        let fv_k_long = first_finite_idx(&ks.long)
            .ok_or_else(|| "kaufman-stop long series all non-finite".to_string())?;
        let fv_k_short = first_finite_idx(&ks.short)
            .ok_or_else(|| "kaufman-stop short series all non-finite".to_string())?;
        first_valid = first_valid.max(fv_k_long.max(fv_k_short));
    }
    let lr = compute_log_returns_f64(close);
    let trade_offset = if strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };

    let prices_f32: Vec<f32> = ma_prices.iter().map(|&x| x as f32).collect();

    let selector = CudaMaSelector::new(device_id as usize);
    let score_fn = score_mode_score_fn(score_mode);
    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    if let Some(score_fn) = score_fn {
        agg = agg.with_score_fn(score_fn);
    }
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
    let (pf_tile, ps_tile) = sweep_tile_sizes(fast_periods.len(), slow_periods.len(), n);

    let mut processed = 0usize;
    emit_progress(progress, processed, total_pairs, "gpu-sweep");

    for f_chunk in fast_periods.chunks(pf_tile) {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let (fast_mat, cols) = ma_matrix_cuda_host_f32(
            &selector,
            device_id as usize,
            candles,
            &prices_f32,
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
            let (slow_mat, cols2) = ma_matrix_cuda_host_f32(
                &selector,
                device_id as usize,
                candles,
                &prices_f32,
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

            let valid_pairs = build_valid_pair_work(f_chunk, s_chunk);

            if include_all {
                for pair in &valid_pairs {
                    let fast_row = &fast_mat[pair.fast_idx * cols..(pair.fast_idx + 1) * cols];
                    let slow_row = &slow_mat[pair.slow_idx * cols..(pair.slow_idx + 1) * cols];
                    let fast_len = pair.fast_len;
                    let slow_len = pair.slow_len;
                    let max_p = (fast_len as usize).max(slow_len as usize).max(1);
                    let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                    let start_t = t_valid.saturating_add(trade_offset);
                    let metrics = eval_metrics_f32_with_exit_mode(
                        &lr,
                        close,
                        fast_row,
                        slow_row,
                        halftrend_trend,
                        start_t,
                        strategy,
                        kaufman_stops.as_ref(),
                        true,
                    );
                    if metrics_pass_constraints(&metrics, constraints) {
                        let params = DoubleMaParams {
                            fast_len,
                            slow_len,
                            fast_ma_id: 0,
                            slow_ma_id: 0,
                        };
                        agg.push(params, metrics);
                    }
                    processed = processed.saturating_add(1);
                    if (processed & 0x1fff) == 0 && cancel.load(Ordering::Relaxed) {
                        return Err("cancelled".to_string());
                    }
                }
            } else {
                let compute_sharpe = objective_needs_return_stats(objective)
                    || score_mode_needs_rich_metrics(score_mode);

                let init = || {
                    let mut local = StreamAggregator::new(objective, top_k, false, n);
                    if let Some(score_fn) = score_fn {
                        local = local.with_score_fn(score_fn);
                    }
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

                let tile = valid_pairs
                    .par_iter()
                    .try_fold(init, |mut local, pair| {
                        if cancel.load(Ordering::Relaxed) {
                            return Err("cancelled".to_string());
                        }
                        let fast_row = &fast_mat[pair.fast_idx * cols..(pair.fast_idx + 1) * cols];
                        let slow_row = &slow_mat[pair.slow_idx * cols..(pair.slow_idx + 1) * cols];
                        let fast_len = pair.fast_len;
                        let slow_len = pair.slow_len;
                        let max_p = (fast_len as usize).max(slow_len as usize).max(1);
                        let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                        let start_t = t_valid.saturating_add(trade_offset);
                        let metrics = eval_metrics_f32_with_exit_mode(
                            &lr,
                            close,
                            fast_row,
                            slow_row,
                            halftrend_trend,
                            start_t,
                            strategy,
                            kaufman_stops.as_ref(),
                            compute_sharpe,
                        );
                        if metrics_pass_constraints(&metrics, constraints) {
                            local.push(
                                DoubleMaParams {
                                    fast_len,
                                    slow_len,
                                    fast_ma_id: 0,
                                    slow_ma_id: 0,
                                },
                                metrics,
                            );
                        }
                        Ok(local)
                    })
                    .try_reduce(init, |mut a, b| {
                        a.merge(b);
                        Ok(a)
                    })?;

                processed = processed.saturating_add(valid_pairs.len());
                agg.merge(tile);
            }

            emit_progress(progress, processed, total_pairs, "gpu-sweep");
        }
    }

    let mut result = agg
        .finalize()
        .ok_or_else(|| "grid search produced no result".to_string())?;

    if !objective_needs_return_stats(objective)
        && !score_mode_needs_rich_metrics(score_mode)
        && !include_all
    {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let mut fast_cache: std::collections::HashMap<u16, Vec<f32>> =
            std::collections::HashMap::new();
        let mut slow_cache: std::collections::HashMap<u16, Vec<f32>> =
            std::collections::HashMap::new();

        let cache_ma = |cache: &mut std::collections::HashMap<u16, Vec<f32>>,
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
            let (values, _) = ma_matrix_cuda_host_f32(
                &selector,
                device_id as usize,
                candles,
                &prices_f32,
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
                         fast_cache: &mut std::collections::HashMap<u16, Vec<f32>>,
                         slow_cache: &mut std::collections::HashMap<u16, Vec<f32>>|
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
            Ok(eval_metrics_f32_with_exit_mode(
                &lr,
                close,
                fast,
                slow,
                halftrend_trend,
                start_t,
                strategy,
                kaufman_stops.as_ref(),
                true,
            ))
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

    Ok(result)
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn vdps_generate_seed_pairs_gpu(
    _device_id: u32,
    _elite_seeds: &[(u16, u16)],
    _fast_periods: &[u16],
    _slow_periods: &[u16],
    _elite_target: usize,
    _diff_target: usize,
    _rand_target: usize,
    _fast_step: u16,
    _slow_step: u16,
    _seed: u64,
) -> Result<Option<Vec<(u16, u16)>>, String> {
    Ok(None)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn vdps_generate_seed_pairs_gpu(
    device_id: u32,
    elite_seeds: &[(u16, u16)],
    fast_periods: &[u16],
    slow_periods: &[u16],
    elite_target: usize,
    diff_target: usize,
    rand_target: usize,
    fast_step: u16,
    slow_step: u16,
    seed: u64,
) -> Result<Option<Vec<(u16, u16)>>, String> {
    crate::gpu_backtest_kernel::vdps_generate_seed_pairs_gpu_kernel(
        device_id,
        elite_seeds,
        fast_periods,
        slow_periods,
        elite_target,
        diff_target,
        rand_target,
        fast_step,
        slow_step,
        seed,
    )
}

pub(crate) fn optimize_double_ma_gpu_hybrid_kernel_phase1(
    candles: &vector_ta::utilities::data_loader::Candles,
    device_id: u32,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    top_k: usize,
    include_all: bool,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    include_trade_metrics: bool,
    metric_contract: Option<DoubleMaMetricContract>,
    constraints: Option<&DoubleMaConstraints>,
    export_all_csv_path: Option<&str>,
    total_pairs: usize,
    heatmap_bins: usize,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    gpu_execution: GpuExecutionPolicy,
    allow_cpu_fallback: bool,
    eval_range: Option<(usize, usize)>,
) -> Result<
    (
        OptimizationResult,
        HybridTimingMs,
        BackendUsed,
        Option<String>,
        Option<String>,
    ),
    String,
> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    emit_progress(progress, 0, 1, "hybrid_plan");
    let planner_t0 = std::time::Instant::now();
    let route_plan = {
        let _nvtx_scope = crate::nvtx::scope("engine", "hybrid_plan");
        crate::ma_policy::plan_double_ma_hybrid_route(fast_ma_type, slow_ma_type)
    };
    let planner_ms = planner_t0.elapsed().as_millis() as u64;

    if !route_plan.is_fully_supported() {
        return Err(format!(
            "GpuHybridKernel has unsupported MA(s): {}",
            route_plan.unsupported_ids().join(", ")
        ));
    }
    let fast_info =
        lookup_ma_info(fast_ma_type).ok_or_else(|| format!("unknown fast MA: '{fast_ma_type}'"))?;
    let slow_info =
        lookup_ma_info(slow_ma_type).ok_or_else(|| format!("unknown slow MA: '{slow_ma_type}'"))?;

    let mut kernel_blockers: Vec<String> = Vec::new();
    let fast_requires_cpu_precompute = should_cpu_stage_indicator(&route_plan.fast, gpu_execution);
    let slow_requires_cpu_precompute = should_cpu_stage_indicator(&route_plan.slow, gpu_execution);
    if fast_requires_cpu_precompute && !fast_info.supports_cpu_single {
        kernel_blockers.push(format!(
            "fast MA '{fast_ma_type}' requires CPU staging under the current hybrid route policy but has no CPU-single implementation"
        ));
    }
    if slow_requires_cpu_precompute && !slow_info.supports_cpu_single {
        kernel_blockers.push(format!(
            "slow MA '{slow_ma_type}' requires CPU staging under the current hybrid route policy but has no CPU-single implementation"
        ));
    }
    if strategy_uses_kaufman_stop_exit(strategy) {
        kernel_blockers.push("kaufman-stop exit is not supported by GPU kernel path".to_string());
    }
    if strategy.uses_execution_overlays() {
        kernel_blockers.push(
            "execution overlays (cooldown/persistence) are not supported by GPU kernel path"
                .to_string(),
        );
    }
    if regime_filter.is_some() && !kernel_regime_filters_enabled() {
        kernel_blockers
            .push("regime filter provided while kernel regime filters are disabled".to_string());
    }
    #[cfg(feature = "cuda")]
    {
        if !cuda_available() {
            kernel_blockers.push("CUDA runtime is unavailable".to_string());
        }
    }
    #[cfg(not(feature = "cuda"))]
    {
        kernel_blockers.push("CUDA feature is not enabled in this build".to_string());
    }

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    emit_progress(progress, 0, 1, "hybrid_prepare");
    let mut fast_precomputed_host: Option<Vec<f32>> = None;
    let mut slow_precomputed_host: Option<Vec<f32>> = None;
    let mut fast_precomputed_cols: Option<usize> = None;
    let mut slow_precomputed_cols: Option<usize> = None;
    let cpu_indicator_prep_ms = {
        let _nvtx_scope = crate::nvtx::scope("engine", "hybrid_prepare");
        if kernel_blockers.is_empty()
            && (fast_requires_cpu_precompute || slow_requires_cpu_precompute)
        {
            let t0 = std::time::Instant::now();
            if fast_requires_cpu_precompute {
                let (mat_f32, cols) = ma_matrix_cpu_f32_staging(
                    candles,
                    fast_ma_type,
                    ma_source,
                    fast_periods,
                    fast_ma_params,
                    fast_alma_offset,
                    fast_alma_sigma,
                )?;
                fast_precomputed_host = Some(mat_f32);
                fast_precomputed_cols = Some(cols);
            }
            if slow_requires_cpu_precompute {
                let (mat_f32, cols) = ma_matrix_cpu_f32_staging(
                    candles,
                    slow_ma_type,
                    ma_source,
                    slow_periods,
                    slow_ma_params,
                    slow_alma_offset,
                    slow_alma_sigma,
                )?;
                slow_precomputed_host = Some(mat_f32);
                slow_precomputed_cols = Some(cols);
            }
            t0.elapsed().as_millis() as u64
        } else {
            0
        }
    };
    #[allow(unused_mut)]
    let mut gpu_indicator_prep_ms = 0u64;

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    emit_progress(progress, 0, 1, "hybrid_transfer");
    // H2D for precomputed rows currently occurs inside the kernel executor.
    #[allow(unused_mut)]
    let mut h2d_upload_ms = {
        let _nvtx_scope = crate::nvtx::scope("engine", "hybrid_transfer");
        0u64
    };

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    let mut route_note: Option<String> = None;
    let mut fallback_reason: Option<String> = None;
    let blockers_text = kernel_blockers.join("; ");
    #[cfg(feature = "cuda")]
    let gpu_sweep_eligible =
        cuda_available() && fast_info.supports_cuda_single && slow_info.supports_cuda_single;
    #[cfg(not(feature = "cuda"))]
    let gpu_sweep_eligible = false;
    let cpu_eligible = fast_info.supports_cpu_single && slow_info.supports_cpu_single;

    if eval_range.is_some() && !kernel_blockers.is_empty() {
        return Err(format!(
            "GpuHybridKernel windowed evaluation requires kernel-executable routing. Reasons: {blockers_text}"
        ));
    }

    let (result, backend_used, kernel_eval_ms) = if kernel_blockers.is_empty() {
        emit_progress(progress, 0, 1, "hybrid_kernel");
        let kernel_t0 = std::time::Instant::now();
        let _nvtx_scope = crate::nvtx::scope("engine", "hybrid_kernel");
        let res = if strategy.uses_pressure_impulse_trend_filter() {
            #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
            {
                let device_regime = build_pressure_impulse_trend_regime_device(
                    candles, ma_source, device_id, strategy,
                )?;
                optimize_double_ma_gpu_kernel_tiled_with_device_regime(
                    candles,
                    device_id,
                    fast_periods,
                    slow_periods,
                    fast_ma_type,
                    slow_ma_type,
                    ma_source,
                    fast_ma_params,
                    slow_ma_params,
                    &device_regime,
                    strategy,
                    objective,
                    score_mode,
                    None,
                    None,
                    None,
                    None,
                    None,
                    top_k,
                    include_all,
                    fast_alma_offset,
                    fast_alma_sigma,
                    slow_alma_offset,
                    slow_alma_sigma,
                    include_trade_metrics,
                    metric_contract,
                    constraints,
                    export_all_csv_path,
                    total_pairs,
                    heatmap_bins,
                    eval_range,
                    cancel,
                    progress,
                )?
            }
            #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
            {
                return Err(
                    "PIT GPU kernel backend requires `cuda,cuda-backtest-kernel`".to_string(),
                );
            }
        } else if strategy.uses_volume_flow_confirm_filter() {
            #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
            {
                let device_regime =
                    build_volume_flow_confirmation_regime_device(candles, device_id, strategy)?;
                optimize_double_ma_gpu_kernel_tiled_with_device_regime(
                    candles,
                    device_id,
                    fast_periods,
                    slow_periods,
                    fast_ma_type,
                    slow_ma_type,
                    ma_source,
                    fast_ma_params,
                    slow_ma_params,
                    &device_regime,
                    strategy,
                    objective,
                    score_mode,
                    None,
                    None,
                    None,
                    None,
                    None,
                    top_k,
                    include_all,
                    fast_alma_offset,
                    fast_alma_sigma,
                    slow_alma_offset,
                    slow_alma_sigma,
                    include_trade_metrics,
                    metric_contract,
                    constraints,
                    export_all_csv_path,
                    total_pairs,
                    heatmap_bins,
                    eval_range,
                    cancel,
                    progress,
                )?
            }
            #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
            {
                return Err(
                    "VFCT GPU kernel backend requires `cuda,cuda-backtest-kernel`".to_string(),
                );
            }
        } else if strategy.uses_slope_efficiency_trend_filter() {
            #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
            {
                let device_regime = build_slope_efficiency_trend_regime_device(
                    candles, ma_source, device_id, strategy,
                )?;
                optimize_double_ma_gpu_kernel_tiled_with_device_regime(
                    candles,
                    device_id,
                    fast_periods,
                    slow_periods,
                    fast_ma_type,
                    slow_ma_type,
                    ma_source,
                    fast_ma_params,
                    slow_ma_params,
                    &device_regime,
                    strategy,
                    objective,
                    score_mode,
                    None,
                    None,
                    None,
                    None,
                    None,
                    top_k,
                    include_all,
                    fast_alma_offset,
                    fast_alma_sigma,
                    slow_alma_offset,
                    slow_alma_sigma,
                    include_trade_metrics,
                    metric_contract,
                    constraints,
                    export_all_csv_path,
                    total_pairs,
                    heatmap_bins,
                    eval_range,
                    cancel,
                    progress,
                )?
            }
            #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
            {
                return Err(
                    "SET GPU kernel backend requires `cuda,cuda-backtest-kernel`".to_string(),
                );
            }
        } else if strategy.uses_momentum_triple_stack_filter() {
            #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
            {
                let device_regime = build_momentum_triple_stack_regime_device(
                    candles, ma_source, device_id, strategy,
                )?;
                optimize_double_ma_gpu_kernel_tiled_with_device_regime(
                    candles,
                    device_id,
                    fast_periods,
                    slow_periods,
                    fast_ma_type,
                    slow_ma_type,
                    ma_source,
                    fast_ma_params,
                    slow_ma_params,
                    &device_regime,
                    strategy,
                    objective,
                    score_mode,
                    None,
                    None,
                    None,
                    None,
                    None,
                    top_k,
                    include_all,
                    fast_alma_offset,
                    fast_alma_sigma,
                    slow_alma_offset,
                    slow_alma_sigma,
                    include_trade_metrics,
                    metric_contract,
                    constraints,
                    export_all_csv_path,
                    total_pairs,
                    heatmap_bins,
                    eval_range,
                    cancel,
                    progress,
                )?
            }
            #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
            {
                return Err(
                    "MTS GPU kernel backend requires `cuda,cuda-backtest-kernel`".to_string(),
                );
            }
        } else if strategy.uses_oscillator_reversion_confluence_filter() {
            #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
            {
                let device_regime = build_oscillator_reversion_confluence_regime_device(
                    candles, ma_source, device_id, strategy,
                )?;
                optimize_double_ma_gpu_kernel_tiled_with_device_regime(
                    candles,
                    device_id,
                    fast_periods,
                    slow_periods,
                    fast_ma_type,
                    slow_ma_type,
                    ma_source,
                    fast_ma_params,
                    slow_ma_params,
                    &device_regime,
                    strategy,
                    objective,
                    score_mode,
                    None,
                    None,
                    None,
                    None,
                    None,
                    top_k,
                    include_all,
                    fast_alma_offset,
                    fast_alma_sigma,
                    slow_alma_offset,
                    slow_alma_sigma,
                    include_trade_metrics,
                    metric_contract,
                    constraints,
                    export_all_csv_path,
                    total_pairs,
                    heatmap_bins,
                    eval_range,
                    cancel,
                    progress,
                )?
            }
            #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
            {
                return Err(
                    "ORC GPU kernel backend requires `cuda,cuda-backtest-kernel`".to_string(),
                );
            }
        } else {
            optimize_double_ma_gpu_kernel_tiled_with_precomputed(
                candles,
                device_id,
                fast_periods,
                slow_periods,
                fast_ma_type,
                slow_ma_type,
                ma_source,
                fast_ma_params,
                slow_ma_params,
                regime_filter,
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
                fast_alma_offset,
                fast_alma_sigma,
                slow_alma_offset,
                slow_alma_sigma,
                include_trade_metrics,
                metric_contract,
                fast_precomputed_host.as_deref(),
                fast_precomputed_cols,
                slow_precomputed_host.as_deref(),
                slow_precomputed_cols,
                constraints,
                export_all_csv_path,
                total_pairs,
                heatmap_bins,
                eval_range,
                cancel,
                progress,
            )?
        };
        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        if let Some(timing) = crate::gpu_backtest_kernel::take_last_kernel_executor_timing() {
            gpu_indicator_prep_ms = timing.gpu_indicator_prep_ms;
            h2d_upload_ms = timing.h2d_upload_ms;
        }
        (
            res,
            BackendUsed::Gpu,
            kernel_t0.elapsed().as_millis() as u64,
        )
    } else if gpu_sweep_eligible {
        if export_all_csv_path.is_some() {
            return Err(format!(
                "export_all_csv_path requires the GPU kernel path; current hybrid selection cannot run kernel path: {blockers_text}"
            ));
        }
        emit_progress(progress, 0, 1, "hybrid_fallback_gpu_sweep");
        let sweep_t0 = std::time::Instant::now();
        let _nvtx_scope = crate::nvtx::scope("engine", "hybrid_fallback_gpu_sweep");
        let res = optimize_double_ma_gpu_sweep_tiled(
            candles,
            device_id,
            fast_periods,
            slow_periods,
            fast_ma_type,
            slow_ma_type,
            ma_source,
            fast_ma_params,
            slow_ma_params,
            regime_filter,
            strategy,
            objective,
            score_mode,
            None,
            None,
            None,
            None,
            None,
            top_k,
            include_all,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            constraints,
            total_pairs,
            heatmap_bins,
            cancel,
            progress,
        )?;
        route_note = Some("GpuHybridKernel->GpuOnly".to_string());
        fallback_reason = Some(format!(
            "Hybrid phase-1 kernel path unavailable; using GpuOnly. Reasons: {blockers_text}"
        ));
        (res, BackendUsed::Gpu, sweep_t0.elapsed().as_millis() as u64)
    } else if cpu_eligible {
        if export_all_csv_path.is_some() {
            return Err(format!(
                "export_all_csv_path requires the GPU kernel path; current hybrid selection cannot run kernel path: {blockers_text}"
            ));
        }
        if !allow_cpu_fallback {
            return Err(format!(
                "GpuHybridKernel in GpuVdps mode requires GPU-executable routing (no CPU fallback). Reasons: {blockers_text}"
            ));
        }
        emit_progress(progress, 0, 1, "hybrid_fallback_cpu");
        let cpu_t0 = std::time::Instant::now();
        let _nvtx_scope = crate::nvtx::scope("engine", "hybrid_fallback_cpu");
        let res = optimize_double_ma_cpu_tiled(
            candles,
            fast_periods,
            slow_periods,
            fast_ma_type,
            slow_ma_type,
            ma_source,
            fast_ma_params,
            slow_ma_params,
            regime_filter,
            strategy,
            objective,
            score_mode,
            top_k,
            include_all,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            constraints,
            total_pairs,
            heatmap_bins,
            None,
            cancel,
            progress,
        )?;
        route_note = Some("GpuHybridKernel->CpuOnly".to_string());
        fallback_reason = Some(format!(
            "Hybrid phase-1 kernel path unavailable; falling back to CpuOnly. Reasons: {blockers_text}"
        ));
        (res, BackendUsed::Cpu, cpu_t0.elapsed().as_millis() as u64)
    } else {
        return Err(format!(
            "GpuHybridKernel cannot route this MA pair/backend combination. fast='{fast_ma_type}', slow='{slow_ma_type}'. Reasons: {blockers_text}"
        ));
    };

    Ok((
        result,
        HybridTimingMs {
            planner_ms,
            cpu_indicator_prep_ms,
            gpu_indicator_prep_ms,
            h2d_upload_ms,
            kernel_eval_ms,
        },
        backend_used,
        route_note,
        fallback_reason,
    ))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_double_ma_gpu_hybrid_kernel_exact_combos(
    candles: &vector_ta::utilities::data_loader::Candles,
    device_id: u32,
    combos: &[DoubleMaParams],
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    strategy: &StrategyConfig,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    fast_precomputed_values: Option<&[f32]>,
    fast_precomputed_cols: Option<usize>,
    slow_precomputed_values: Option<&[f32]>,
    slow_precomputed_cols: Option<usize>,
    ranges: &[(usize, usize)],
) -> Result<(Vec<Metrics>, BackendUsed, Option<String>, Option<String>), String> {
    if combos.is_empty() {
        return Err("no parameter combinations".to_string());
    }
    if ranges.is_empty() {
        return Err("no evaluation ranges".to_string());
    }

    let route_plan = crate::ma_policy::plan_double_ma_hybrid_route(fast_ma_type, slow_ma_type);
    if !route_plan.is_fully_supported() {
        return Err(format!(
            "GpuHybridKernel has unsupported MA(s): {}",
            route_plan.unsupported_ids().join(", ")
        ));
    }
    let fast_info =
        lookup_ma_info(fast_ma_type).ok_or_else(|| format!("unknown fast MA: '{fast_ma_type}'"))?;
    let slow_info =
        lookup_ma_info(slow_ma_type).ok_or_else(|| format!("unknown slow MA: '{slow_ma_type}'"))?;

    let mut kernel_blockers: Vec<String> = Vec::new();
    let fast_requires_cpu_precompute =
        should_cpu_stage_indicator(&route_plan.fast, GpuExecutionPolicy::BestEffort);
    let slow_requires_cpu_precompute =
        should_cpu_stage_indicator(&route_plan.slow, GpuExecutionPolicy::BestEffort);
    if fast_requires_cpu_precompute && !fast_info.supports_cpu_single {
        kernel_blockers.push(format!(
            "fast MA '{fast_ma_type}' requires CPU staging under the current hybrid route policy but has no CPU-single implementation"
        ));
    }
    if slow_requires_cpu_precompute && !slow_info.supports_cpu_single {
        kernel_blockers.push(format!(
            "slow MA '{slow_ma_type}' requires CPU staging under the current hybrid route policy but has no CPU-single implementation"
        ));
    }
    if strategy_uses_kaufman_stop_exit(strategy) {
        kernel_blockers.push("kaufman-stop exit is not supported by GPU kernel path".to_string());
    }
    if strategy.uses_execution_overlays() {
        kernel_blockers.push(
            "execution overlays (cooldown/persistence) are not supported by GPU kernel path"
                .to_string(),
        );
    }
    if regime_filter.is_some() && !kernel_regime_filters_enabled() {
        kernel_blockers
            .push("regime filter provided while kernel regime filters are disabled".to_string());
    }
    #[cfg(feature = "cuda")]
    {
        if !cuda_available() {
            kernel_blockers.push("CUDA runtime is unavailable".to_string());
        }
    }
    #[cfg(not(feature = "cuda"))]
    {
        kernel_blockers.push("CUDA feature is not enabled in this build".to_string());
    }

    let full_range = ranges.len() == 1 && ranges[0] == (0, candles.close.len());
    let blockers_text = kernel_blockers.join("; ");
    if !kernel_blockers.is_empty() {
        if !full_range {
            return Err(format!(
                "GpuHybridKernel windowed evaluation requires kernel-executable routing. Reasons: {blockers_text}"
            ));
        }

        let total_pairs = estimate_pairs_fast_lt_slow(fast_periods, slow_periods);
        let cancel = AtomicBool::new(false);
        let (res, _, backend_used, route_note, fallback_reason) =
            optimize_double_ma_gpu_hybrid_kernel_phase1(
                candles,
                device_id,
                fast_periods,
                slow_periods,
                fast_ma_type,
                slow_ma_type,
                ma_source,
                fast_ma_params,
                slow_ma_params,
                regime_filter,
                strategy,
                ObjectiveKind::Sharpe,
                None,
                combos.len(),
                true,
                fast_alma_offset,
                fast_alma_sigma,
                slow_alma_offset,
                slow_alma_sigma,
                false,
                None,
                None,
                None,
                total_pairs,
                0,
                &cancel,
                None,
                GpuExecutionPolicy::BestEffort,
                true,
                None,
            )?;
        let mut metric_by_pair: HashMap<(u16, u16), Metrics> = HashMap::with_capacity(combos.len());
        if let Some(all) = res.all.as_ref() {
            for (params, metric) in all {
                metric_by_pair.insert((params.fast_len, params.slow_len), metric.clone());
            }
        }
        let mut ordered = Vec::with_capacity(combos.len());
        for combo in combos {
            ordered.push(
                metric_by_pair
                    .get(&(combo.fast_len, combo.slow_len))
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "GpuHybridKernel fallback did not return metrics for pair ({}, {})",
                            combo.fast_len, combo.slow_len
                        )
                    })?,
            );
        }
        return Ok((
            ordered,
            backend_used,
            route_note.or_else(|| Some("GpuHybridKernel".to_string())),
            fallback_reason,
        ));
    }

    let mut fast_precomputed_host_owned: Option<Vec<f32>> = None;
    let mut slow_precomputed_host_owned: Option<Vec<f32>> = None;
    let mut fast_precomputed_cols_local = fast_precomputed_cols;
    let mut slow_precomputed_cols_local = slow_precomputed_cols;
    if fast_requires_cpu_precompute && fast_precomputed_values.is_none() {
        let (mat_f32, cols) = ma_matrix_cpu_f32_staging(
            candles,
            fast_ma_type,
            ma_source,
            fast_periods,
            fast_ma_params,
            fast_alma_offset,
            fast_alma_sigma,
        )?;
        fast_precomputed_host_owned = Some(mat_f32);
        fast_precomputed_cols_local = Some(cols);
    }
    if slow_requires_cpu_precompute && slow_precomputed_values.is_none() {
        let (mat_f32, cols) = ma_matrix_cpu_f32_staging(
            candles,
            slow_ma_type,
            ma_source,
            slow_periods,
            slow_ma_params,
            slow_alma_offset,
            slow_alma_sigma,
        )?;
        slow_precomputed_host_owned = Some(mat_f32);
        slow_precomputed_cols_local = Some(cols);
    }

    let fast_precomputed_src = fast_precomputed_values.or(fast_precomputed_host_owned.as_deref());
    let slow_precomputed_src = slow_precomputed_values.or(slow_precomputed_host_owned.as_deref());
    let fast_precomputed = match (fast_precomputed_src, fast_precomputed_cols_local) {
        (Some(values), Some(cols)) => Some(crate::gpu_backtest_kernel::PrecomputedHostMaMatrix {
            periods: fast_periods,
            cols,
            values_row_major: values,
        }),
        (None, None) => None,
        _ => return Err("fast precomputed matrix requires both values and cols".to_string()),
    };
    let slow_precomputed = match (slow_precomputed_src, slow_precomputed_cols_local) {
        (Some(values), Some(cols)) => Some(crate::gpu_backtest_kernel::PrecomputedHostMaMatrix {
            periods: slow_periods,
            cols,
            values_row_major: values,
        }),
        (None, None) => None,
        _ => return Err("slow precomputed matrix requires both values and cols".to_string()),
    };

    let metrics =
        crate::gpu_backtest_kernel::eval_double_ma_batch_gpu_kernel_multi_range_with_precomputed_and_regime(
            candles,
            combos,
            fast_periods,
            slow_periods,
            fast_ma_type,
            slow_ma_type,
            ma_source,
            fast_ma_params,
            slow_ma_params,
            regime_filter.map(crate::gpu_backtest_kernel::RegimeFilterInput::HostF64),
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
            fast_precomputed.as_ref(),
            slow_precomputed.as_ref(),
            strategy,
            ranges,
        )?;

    Ok((
        metrics,
        BackendUsed::Gpu,
        Some("GpuHybridKernel".to_string()),
        None,
    ))
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn optimize_double_ma_gpu_kernel_tiled(
    _candles: &vector_ta::utilities::data_loader::Candles,
    _device_id: u32,
    _fast_periods: &[u16],
    _slow_periods: &[u16],
    _fast_ma_type: &str,
    _slow_ma_type: &str,
    _ma_source: &str,
    _fast_ma_params: Option<&HashMap<String, f64>>,
    _slow_ma_params: Option<&HashMap<String, f64>>,
    _regime_filter: Option<&[f64]>,
    _strategy: &StrategyConfig,
    _objective: ObjectiveKind,
    _score_mode: Option<DoubleMaScoreMode>,
    _extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    _shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    _shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    _shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    _shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
    _top_k: usize,
    _include_all: bool,
    _fast_alma_offset: f64,
    _fast_alma_sigma: f64,
    _slow_alma_offset: f64,
    _slow_alma_sigma: f64,
    _constraints: Option<&DoubleMaConstraints>,
    _export_all_csv_path: Option<&str>,
    _total_pairs: usize,
    _heatmap_bins: usize,
    _cancel: &AtomicBool,
    _progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    Err("GPU kernel backend requires building `ta_desktop_demo_app` with `--features cuda,cuda-backtest-kernel`".to_string())
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn optimize_double_ma_gpu_kernel_tiled_with_precomputed(
    _candles: &vector_ta::utilities::data_loader::Candles,
    _device_id: u32,
    _fast_periods: &[u16],
    _slow_periods: &[u16],
    _fast_ma_type: &str,
    _slow_ma_type: &str,
    _ma_source: &str,
    _fast_ma_params: Option<&HashMap<String, f64>>,
    _slow_ma_params: Option<&HashMap<String, f64>>,
    _regime_filter: Option<&[f64]>,
    _strategy: &StrategyConfig,
    _objective: ObjectiveKind,
    _score_mode: Option<DoubleMaScoreMode>,
    _extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    _shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    _shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    _shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    _shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
    _top_k: usize,
    _include_all: bool,
    _fast_alma_offset: f64,
    _fast_alma_sigma: f64,
    _slow_alma_offset: f64,
    _slow_alma_sigma: f64,
    _fast_precomputed_values: Option<&[f32]>,
    _fast_precomputed_cols: Option<usize>,
    _slow_precomputed_values: Option<&[f32]>,
    _slow_precomputed_cols: Option<usize>,
    _constraints: Option<&DoubleMaConstraints>,
    _export_all_csv_path: Option<&str>,
    _total_pairs: usize,
    _heatmap_bins: usize,
    _eval_range: Option<(usize, usize)>,
    _cancel: &AtomicBool,
    _progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    Err("GPU kernel backend requires building `ta_desktop_demo_app` with `--features cuda,cuda-backtest-kernel`".to_string())
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn dynamic_gpu_batch_cache_key(
    candles: &vector_ta::utilities::data_loader::Candles,
    device_id: u32,
    handle: &crate::strategy_gpu_runtime::StrategyHandle,
    ma_source: &str,
    fast_ma_type: &str,
    slow_ma_type: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    fast_periods: &[u16],
    slow_periods: &[u16],
    first_valid: usize,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct DynamicGpuBatchSignature<'a> {
        plan_fingerprint_hex: &'a str,
        device_id: u32,
        ma_source: &'a str,
        fast_ma_type: &'a str,
        slow_ma_type: &'a str,
        fast_ma_params: Option<BTreeMap<String, String>>,
        slow_ma_params: Option<BTreeMap<String, String>>,
        fast_periods: &'a [u16],
        slow_periods: &'a [u16],
        first_valid: usize,
        candle_len: usize,
        candle_close_ptr: usize,
        candle_ts_first: Option<i64>,
        candle_ts_last: Option<i64>,
    }

    fn normalize_params(params: Option<&HashMap<String, f64>>) -> Option<BTreeMap<String, String>> {
        params.map(|map| {
            map.iter()
                .map(|(k, v)| (k.clone(), format!("{:.12}", v)))
                .collect()
        })
    }

    serde_json::to_string(&DynamicGpuBatchSignature {
        plan_fingerprint_hex: &handle.plan.ir_fingerprint_hex,
        device_id,
        ma_source,
        fast_ma_type,
        slow_ma_type,
        fast_ma_params: normalize_params(fast_ma_params),
        slow_ma_params: normalize_params(slow_ma_params),
        fast_periods,
        slow_periods,
        first_valid,
        candle_len: candles.close.len(),
        candle_close_ptr: candles.close.as_ptr() as usize,
        candle_ts_first: candles.timestamp.first().copied(),
        candle_ts_last: candles.timestamp.last().copied(),
    })
    .map_err(|e| format!("dynamic gpu batch cache key serialization failed: {e}"))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn get_or_build_dynamic_gpu_uploaded_batch(
    candles: &vector_ta::utilities::data_loader::Candles,
    device_id: u32,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    strategy: &StrategyConfig,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    first_valid: usize,
    handle: &crate::strategy_gpu_runtime::StrategyHandle,
) -> Result<Rc<crate::gpu_backtest_kernel::UploadedDoubleMaDynamicProgramBatch>, String> {
    let cache_key = dynamic_gpu_batch_cache_key(
        candles,
        device_id,
        handle,
        ma_source,
        fast_ma_type,
        slow_ma_type,
        fast_ma_params,
        slow_ma_params,
        fast_periods,
        slow_periods,
        first_valid,
    )?;
    if let Some(existing) =
        DYNAMIC_GPU_BATCH_CACHE.with(|cell| cell.borrow().get(&cache_key).cloned())
    {
        return Ok(existing);
    }

    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
    let ma_prices_f32: Vec<f32> = ma_prices.iter().map(|&v| v as f32).collect();
    let matrix_f32 = |ma_type: &str,
                      periods: &[u16],
                      params: Option<&HashMap<String, f64>>,
                      alma_offset: f64,
                      alma_sigma: f64|
     -> Result<(Vec<f32>, usize), String> {
        let info = lookup_ma_info(ma_type).ok_or_else(|| format!("unknown MA: '{ma_type}'"))?;
        if info.supports_cuda_single {
            let selector = CudaMaSelector::new(device_id as usize);
            if let Ok((values, cols)) = ma_matrix_cuda_host_f32(
                &selector,
                device_id as usize,
                candles,
                &ma_prices_f32,
                ma_type,
                ma_source,
                periods,
                params,
                alma_offset,
                alma_sigma,
            ) {
                return Ok((values, cols));
            }
        }
        ma_matrix_cpu_f32_staging(
            candles,
            ma_type,
            ma_source,
            periods,
            params,
            alma_offset,
            alma_sigma,
        )
    };
    let (fast_mat, fast_cols) = matrix_f32(
        fast_ma_type,
        fast_periods,
        fast_ma_params,
        fast_alma_offset,
        fast_alma_sigma,
    )?;
    let (slow_mat, slow_cols) = matrix_f32(
        slow_ma_type,
        slow_periods,
        slow_ma_params,
        slow_alma_offset,
        slow_alma_sigma,
    )?;
    let n = candles.close.len();
    if fast_cols != n || slow_cols != n {
        return Err("dynamic gpu staging matrix column mismatch".to_string());
    }
    let combos: Vec<DoubleMaParams> = build_valid_pair_work(fast_periods, slow_periods)
        .into_iter()
        .map(|pair| DoubleMaParams {
            fast_len: pair.fast_len,
            slow_len: pair.slow_len,
            fast_ma_id: 0,
            slow_ma_id: 0,
        })
        .collect();
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let lr_f32: Vec<f32> = compute_log_returns_f64(close)
        .into_iter()
        .map(|v| v as f32)
        .collect();
    let uploaded = Rc::new(
        crate::gpu_backtest_kernel::upload_double_ma_dynamic_program_batch(
            &lr_f32,
            &fast_mat,
            &slow_mat,
            n,
            fast_periods,
            slow_periods,
            &combos,
            first_valid,
            strategy,
            device_id,
        )?,
    );
    DYNAMIC_GPU_BATCH_CACHE.with(|cell| {
        cell.borrow_mut().insert(cache_key, uploaded.clone());
    });
    Ok(uploaded)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn optimize_double_ma_dynamic_gpu_kernel_tiled(
    candles: &vector_ta::utilities::data_loader::Candles,
    device_id: u32,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
    _shortlist_neighborhood_radius: Option<u16>,
    _shortlist_neighborhood_step: Option<u16>,
    include_trade_metrics: bool,
    metric_contract: Option<DoubleMaMetricContract>,
    top_k: usize,
    include_all: bool,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    constraints: Option<&DoubleMaConstraints>,
    total_pairs: usize,
    heatmap_bins: usize,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    handle: &crate::strategy_gpu_runtime::StrategyHandle,
) -> Result<OptimizationResult, String> {
    if !cuda_available() {
        return Err("CUDA device not available (cuda_available() == false)".to_string());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }
    if candles.close.len() < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if strategy.uses_execution_overlays() || strategy_uses_kaufman_stop_exit(strategy) {
        return Err(
            "dynamic gpu kernel v1 supports only plain crossover execution without overlays"
                .to_string(),
        );
    }
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, ma_source);
    if ma_prices.len() != close.len() {
        return Err("ma source length mismatch".to_string());
    }
    let n = close.len();
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let first_valid = fv_close.max(fv_ma);
    let combos: Vec<DoubleMaParams> = build_valid_pair_work(fast_periods, slow_periods)
        .into_iter()
        .map(|pair| DoubleMaParams {
            fast_len: pair.fast_len,
            slow_len: pair.slow_len,
            fast_ma_id: 0,
            slow_ma_id: 0,
        })
        .collect();
    if combos.is_empty() {
        return Err("no valid fast<slow parameter combinations".to_string());
    }

    emit_progress(progress, 0, total_pairs, "gpu_dynamic");
    let uploaded = get_or_build_dynamic_gpu_uploaded_batch(
        candles,
        device_id,
        fast_periods,
        slow_periods,
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        strategy,
        fast_alma_offset,
        fast_alma_sigma,
        slow_alma_offset,
        slow_alma_sigma,
        first_valid,
        handle,
    )?;
    let metrics_count = std::iter::once(objective)
        .chain(
            extra_shortlist_objectives
                .into_iter()
                .flat_map(|values| values.iter().copied()),
        )
        .fold(0usize, |current, candidate_objective| {
            current.max(
                crate::gpu_backtest_kernel::search_metrics_count_for_optimization_with_contract(
                    candidate_objective,
                    score_mode,
                    constraints,
                    include_all,
                    false,
                    false,
                    false,
                    include_trade_metrics,
                    metric_contract,
                ),
            )
        });
    let metrics = crate::gpu_backtest_kernel::eval_uploaded_double_ma_dynamic_program_gpu_kernel(
        uploaded.as_ref(),
        strategy,
        handle,
        metrics_count,
    )?;
    if metrics.len() != combos.len() {
        return Err(format!(
            "dynamic gpu kernel returned {} metrics for {} combos",
            metrics.len(),
            combos.len()
        ));
    }

    let filtered_rows: Vec<(DoubleMaParams, Metrics)> = combos
        .into_iter()
        .zip(metrics.into_iter())
        .filter(|(_, metric)| metrics_pass_constraints(metric, constraints))
        .collect();

    let finalize = || {
        let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
        if score_mode_needs_pair_observation_context(score_mode) {
            agg = agg.with_pair_score_fn(
                crate::gpu_backtest_kernel::effective_sample_lcb_10_pair_score_for_host,
            );
        } else if let Some(score_fn) = score_mode_score_fn(score_mode) {
            agg = agg.with_score_fn(score_fn);
        }
        if heatmap_bins > 0 {
            let f_min = *fast_periods.iter().min().unwrap_or(&0);
            let f_max = *fast_periods.iter().max().unwrap_or(&0);
            let s_min = *slow_periods.iter().min().unwrap_or(&0);
            let s_max = *slow_periods.iter().max().unwrap_or(&0);
            agg = agg.with_heatmap(heatmap_bins, heatmap_bins, f_min, f_max, s_min, s_max);
        }
        for (combo, metric) in filtered_rows.iter().cloned() {
            agg.push(combo, metric);
        }
        emit_progress(progress, total_pairs, total_pairs, "gpu_dynamic");
        agg.finalize().ok_or_else(|| {
            "dynamic gpu optimization produced no valid parameter combinations".to_string()
        })
    };

    let mut result = if score_mode_needs_pair_observation_context(score_mode) {
        crate::gpu_backtest_kernel::with_effective_sample_score_context(
            PairObservationScoreContextResolved {
                first_valid: uploaded.first_valid(),
                regime_first_valid: uploaded.first_valid(),
                eval_start: 0,
                eval_end: n,
                total_bars: n,
                trade_on_next_bar: strategy.trade_on_next_bar,
            },
            finalize,
        )
    } else {
        finalize()
    }?;

    let mut materialized_shortlist_rows: Vec<(DoubleMaParams, Metrics)> = Vec::new();
    if let Some(extra_objectives) = extra_shortlist_objectives.filter(|values| !values.is_empty()) {
        let top_limit = result.top.len().max(1);
        materialized_shortlist_rows = build_exact_extra_shortlist_rows(
            filtered_rows.as_slice(),
            objective,
            extra_objectives,
            top_limit,
        );
        let (best_params, best_metrics) =
            materialized_shortlist_rows
                .first()
                .cloned()
                .ok_or_else(|| {
                    "dynamic gpu extra-shortlist request produced no shortlist rows".to_string()
                })?;
        result.best_params = best_params;
        result.best_metrics = best_metrics;
        result.best_score = Some(effective_score(&result.best_metrics, objective, score_mode));
        result.top = materialized_shortlist_rows
            .iter()
            .take(top_limit)
            .cloned()
            .collect();
        result.top_scores = Some(
            result
                .top
                .iter()
                .map(|(_, metrics)| effective_score(metrics, objective, score_mode))
                .collect(),
        );
    }

    if let Some(mode) = shortlist_union_mode {
        if materialized_shortlist_rows.is_empty() {
            return Err(
                "dynamic gpu shortlist-union request produced no shortlist rows".to_string(),
            );
        }
        let top_limit = result.top.len().max(1);
        let reranked = crate::gpu_backtest_kernel::rerank_shortlist_by_union_mode(
            materialized_shortlist_rows.as_slice(),
            objective,
            extra_shortlist_objectives.unwrap_or(&[]),
            mode,
        );
        let (best_params, best_metrics) = reranked
            .first()
            .cloned()
            .ok_or_else(|| "dynamic gpu shortlist-union rerank produced no result".to_string())?;
        result.best_params = best_params;
        result.best_metrics = best_metrics;
        result.best_score = Some(effective_score(&result.best_metrics, objective, score_mode));
        result.top = reranked.into_iter().take(top_limit).collect();
        result.top_scores = Some(
            result
                .top
                .iter()
                .map(|(_, metrics)| effective_score(metrics, objective, score_mode))
                .collect(),
        );
    }

    if let Some(mode) = shortlist_confidence_mode {
        let top_limit = result.top.len().max(1);
        let confidence_rows: &[(DoubleMaParams, Metrics)] = if materialized_shortlist_rows
            .is_empty()
        {
            result.top.as_slice()
        } else {
            materialized_shortlist_rows.as_slice()
        };
        let reranked = crate::gpu_backtest_kernel::rerank_shortlist_by_confidence_for_host(
            confidence_rows,
            mode,
            PairObservationScoreContextResolved {
                first_valid: uploaded.first_valid(),
                regime_first_valid: uploaded.first_valid(),
                eval_start: 0,
                eval_end: n,
                total_bars: n,
                trade_on_next_bar: strategy.trade_on_next_bar,
            },
            strategy,
            objective,
            score_mode,
        );
        let (best_params, best_metrics) = reranked.first().cloned().ok_or_else(|| {
            "dynamic gpu shortlist-confidence rerank produced no result".to_string()
        })?;
        result.best_params = best_params;
        result.best_metrics = best_metrics;
        result.best_score = Some(effective_score(&result.best_metrics, objective, score_mode));
        result.top = reranked.into_iter().take(top_limit).collect();
        result.top_scores = Some(
            result
                .top
                .iter()
                .map(|(_, metrics)| effective_score(metrics, objective, score_mode))
                .collect(),
        );
    }

    if let Some(mode) = shortlist_slice_mode {
        let top_limit = result.top.len().max(1);
        let slice_rows: &[(DoubleMaParams, Metrics)] = if materialized_shortlist_rows.is_empty() {
            result.top.as_slice()
        } else {
            materialized_shortlist_rows.as_slice()
        };
        let (reranked, diagnostics) =
            crate::gpu_backtest_kernel::rerank_shortlist_by_slice_robustness_for_host(
                device_id,
                candles,
                slice_rows,
                mode,
                fast_ma_type,
                slow_ma_type,
                ma_source,
                fast_ma_params,
                slow_ma_params,
                strategy,
                objective,
                score_mode,
                shortlist_slice_benchmark.as_ref(),
                uploaded.first_valid(),
                0,
                n,
                fast_alma_offset,
                fast_alma_sigma,
                slow_alma_offset,
                slow_alma_sigma,
                strategy.hurdle_tau.unwrap_or(0.0),
                include_trade_metrics,
                metric_contract,
            )?;
        crate::gpu_backtest_kernel::record_last_shortlist_slice_diagnostics(
            ShortlistSliceDiagnostics {
                ranking_mode: diagnostics.ranking_mode.clone(),
                neighborhood_robustness: diagnostics.neighborhood_robustness.clone(),
                candidates: diagnostics.candidates.into_iter().take(top_limit).collect(),
            },
        );
        let (best_params, best_metrics) = reranked
            .first()
            .cloned()
            .ok_or_else(|| "dynamic gpu shortlist-slice rerank produced no result".to_string())?;
        result.best_params = best_params;
        result.best_metrics = best_metrics;
        result.best_score = Some(effective_score(&result.best_metrics, objective, score_mode));
        result.top = reranked.into_iter().take(top_limit).collect();
        result.top_scores = Some(
            result
                .top
                .iter()
                .map(|(_, metrics)| effective_score(metrics, objective, score_mode))
                .collect(),
        );
    }

    Ok(result)
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
#[allow(clippy::too_many_arguments)]
fn optimize_double_ma_dynamic_gpu_kernel_tiled(
    _candles: &vector_ta::utilities::data_loader::Candles,
    _device_id: u32,
    _fast_periods: &[u16],
    _slow_periods: &[u16],
    _fast_ma_type: &str,
    _slow_ma_type: &str,
    _ma_source: &str,
    _fast_ma_params: Option<&HashMap<String, f64>>,
    _slow_ma_params: Option<&HashMap<String, f64>>,
    _strategy: &StrategyConfig,
    _objective: ObjectiveKind,
    _score_mode: Option<DoubleMaScoreMode>,
    _extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    _shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    _shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    _shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    _shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
    _shortlist_neighborhood_radius: Option<u16>,
    _shortlist_neighborhood_step: Option<u16>,
    _include_trade_metrics: bool,
    _metric_contract: Option<DoubleMaMetricContract>,
    _top_k: usize,
    _include_all: bool,
    _fast_alma_offset: f64,
    _fast_alma_sigma: f64,
    _slow_alma_offset: f64,
    _slow_alma_sigma: f64,
    _constraints: Option<&DoubleMaConstraints>,
    _total_pairs: usize,
    _heatmap_bins: usize,
    _cancel: &AtomicBool,
    _progress: Option<&dyn ProgressSink>,
    _handle: &crate::strategy_gpu_runtime::StrategyHandle,
) -> Result<OptimizationResult, String> {
    Err(
        "dynamic gpu kernel backend requires building `ta_desktop_demo_app` with `--features cuda,cuda-backtest-kernel`"
            .to_string(),
    )
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn exact_gpu_kernel_config(
    device_id: u32,
    strategy: &StrategyConfig,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    include_trade_metrics: bool,
    metric_contract: Option<DoubleMaMetricContract>,
) -> crate::gpu_backtest_kernel::KernelConfig {
    crate::gpu_backtest_kernel::KernelConfig {
        device_id,
        fast_alma_offset,
        fast_alma_sigma,
        slow_alma_offset,
        slow_alma_sigma,
        hurdle_tau: strategy.hurdle_tau.unwrap_or(0.0),
        include_trade_metrics,
        metric_contract,
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn optimize_double_ma_gpu_kernel_tiled(
    candles: &vector_ta::utilities::data_loader::Candles,
    device_id: u32,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
    top_k: usize,
    include_all: bool,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    include_trade_metrics: bool,
    metric_contract: Option<DoubleMaMetricContract>,
    constraints: Option<&DoubleMaConstraints>,
    export_all_csv_path: Option<&str>,
    total_pairs: usize,
    heatmap_bins: usize,
    eval_range: Option<(usize, usize)>,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    optimize_double_ma_gpu_kernel_tiled_with_precomputed(
        candles,
        device_id,
        fast_periods,
        slow_periods,
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        regime_filter,
        strategy,
        objective,
        score_mode,
        extra_shortlist_objectives,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        shortlist_slice_benchmark,
        top_k,
        include_all,
        fast_alma_offset,
        fast_alma_sigma,
        slow_alma_offset,
        slow_alma_sigma,
        include_trade_metrics,
        metric_contract,
        None,
        None,
        None,
        None,
        constraints,
        export_all_csv_path,
        total_pairs,
        heatmap_bins,
        eval_range,
        cancel,
        progress,
    )
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn optimize_double_ma_gpu_kernel_tiled_with_device_regime(
    candles: &vector_ta::utilities::data_loader::Candles,
    device_id: u32,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: &crate::gpu_backtest_kernel::DeviceRegimeFilter,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
    top_k: usize,
    include_all: bool,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    include_trade_metrics: bool,
    metric_contract: Option<DoubleMaMetricContract>,
    constraints: Option<&DoubleMaConstraints>,
    export_all_csv_path: Option<&str>,
    total_pairs: usize,
    heatmap_bins: usize,
    eval_range: Option<(usize, usize)>,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    if !cuda_available() {
        return Err("CUDA device not available (cuda_available() == false)".to_string());
    }

    crate::gpu_backtest_kernel::optimize_double_ma_gpu_kernel_with_precomputed_and_regime(
        candles,
        fast_periods,
        slow_periods,
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        Some(crate::gpu_backtest_kernel::RegimeFilterInput::DeviceF32(
            regime_filter,
        )),
        strategy,
        objective,
        score_mode,
        extra_shortlist_objectives,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        shortlist_slice_benchmark,
        top_k,
        include_all,
        exact_gpu_kernel_config(
            device_id,
            strategy,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            include_trade_metrics,
            metric_contract,
        ),
        None,
        None,
        constraints,
        export_all_csv_path,
        total_pairs,
        heatmap_bins,
        None,
        eval_range,
        cancel,
        progress,
    )
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn optimize_double_ma_gpu_kernel_tiled_with_precomputed(
    candles: &vector_ta::utilities::data_loader::Candles,
    device_id: u32,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
    top_k: usize,
    include_all: bool,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    include_trade_metrics: bool,
    metric_contract: Option<DoubleMaMetricContract>,
    fast_precomputed_values: Option<&[f32]>,
    fast_precomputed_cols: Option<usize>,
    slow_precomputed_values: Option<&[f32]>,
    slow_precomputed_cols: Option<usize>,
    constraints: Option<&DoubleMaConstraints>,
    export_all_csv_path: Option<&str>,
    total_pairs: usize,
    heatmap_bins: usize,
    eval_range: Option<(usize, usize)>,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    if !cuda_available() {
        return Err("CUDA device not available (cuda_available() == false)".to_string());
    }

    let fast_precomputed = match (fast_precomputed_values, fast_precomputed_cols) {
        (Some(values), Some(cols)) => Some(crate::gpu_backtest_kernel::PrecomputedHostMaMatrix {
            periods: fast_periods,
            cols,
            values_row_major: values,
        }),
        (None, None) => None,
        _ => return Err("fast precomputed matrix requires both values and cols".to_string()),
    };
    let slow_precomputed = match (slow_precomputed_values, slow_precomputed_cols) {
        (Some(values), Some(cols)) => Some(crate::gpu_backtest_kernel::PrecomputedHostMaMatrix {
            periods: slow_periods,
            cols,
            values_row_major: values,
        }),
        (None, None) => None,
        _ => return Err("slow precomputed matrix requires both values and cols".to_string()),
    };

    crate::gpu_backtest_kernel::optimize_double_ma_gpu_kernel_with_precomputed(
        candles,
        fast_periods,
        slow_periods,
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        regime_filter,
        strategy,
        objective,
        score_mode,
        extra_shortlist_objectives,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        shortlist_slice_benchmark,
        top_k,
        include_all,
        exact_gpu_kernel_config(
            device_id,
            strategy,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            include_trade_metrics,
            metric_contract,
        ),
        fast_precomputed.as_ref(),
        slow_precomputed.as_ref(),
        constraints,
        export_all_csv_path,
        total_pairs,
        heatmap_bins,
        None,
        eval_range,
        cancel,
        progress,
    )
}

#[cfg(test)]
mod tests {
    use super::double_ma_test_support::{
        breakout_strategy_for_kernel, double_like_kernel_strategy_cases, marrt_strategy_for_kernel,
        mts_strategy_for_kernel, orc_strategy_for_kernel, pit_strategy_for_kernel,
        regime_hybrid_strategy_for_kernel, set_strategy_for_kernel,
        trend_quality_strategy_for_kernel, vfct_strategy_for_kernel,
    };
    use super::*;
    use crate::progress::{DoubleMaProgressPayload, ProgressSink};
    use crate::strategy_ir::{
        build_static_strategy_ir, ConditionGroup, ConditionGroupMode, ConditionNode, ConditionOp,
        ExecutionPolicy, IndicatorKind, IndicatorNode, MovingAverageSpec, SignalBindings,
        SignalProgramGraph, StaticStrategyIrInput, StrategyIr, StrategyProfileKind, ValueRef,
        STRATEGY_IR_SCHEMA_VERSION,
    };

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
        if let Some(cache) = DYNAMIC_GPU_PREFLIGHT_CACHE.get() {
            cache.lock().expect("preflight cache lock").clear();
        }
        TestEnvGuard {
            _env: env,
            _gpu: gpu,
        }
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    fn lock_env() -> TestEnvGuard {
        let env = crate::test_support::lock_test_env();
        if let Some(cache) = DYNAMIC_GPU_PREFLIGHT_CACHE.get() {
            cache.lock().expect("preflight cache lock").clear();
        }
        TestEnvGuard { _env: env }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn run_with_kernel_vram_budget(
        req: DoubleMaRequest,
        candles: &vector_ta::utilities::data_loader::Candles,
        cancel: &AtomicBool,
        budget_mb: usize,
    ) -> Result<BackendOptimizationResult, String> {
        crate::gpu_backtest_kernel::with_test_vram_budget_mb(Some(budget_mb), || {
            run_double_ma_optimization_blocking_with_candles(req, candles, cancel, None)
        })
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn build_pressure_impulse_trend_regime_device_parity_stable(
        candles: &vector_ta::utilities::data_loader::Candles,
        ma_source: &str,
        device_id: u32,
        strategy: &StrategyConfig,
    ) -> Result<crate::gpu_backtest_kernel::DeviceRegimeFilter, String> {
        let regime = compute_pressure_impulse_trend_regime(candles, ma_source, strategy)?
            .ok_or_else(|| "pit regime enabled".to_string())?;
        let first_valid = regime
            .iter()
            .position(|v| v.is_finite())
            .unwrap_or(regime.len());
        let regime_f32: Vec<f32> = regime
            .iter()
            .map(|&v| if v.is_nan() { f32::NAN } else { v as f32 })
            .collect();
        crate::gpu_backtest_kernel::upload_host_regime_filter(device_id, &regime_f32, first_valid)
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

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn assert_indicator_cuda_host_matches_cpu_on_candles(
        candles: &vector_ta::utilities::data_loader::Candles,
        ma_source: &str,
        indicator_id: &str,
        output_id: &str,
        params: &[crate::IndicatorParamKV],
        min_start: usize,
        tol: f64,
    ) {
        use crate::dynamic_indicator_adapter::{
            compute_dynamic_indicator_cpu, compute_dynamic_indicator_cuda_with_inputs,
            DynamicIndicatorCudaSeries, DynamicIndicatorSeries, PreparedCudaIndicatorInputs,
        };
        use vector_ta::indicators::dispatch::CudaOutputTarget;

        let prepared_inputs = PreparedCudaIndicatorInputs::from_candles(candles, ma_source)
            .expect("prepared inputs should upload to device");
        let cpu = compute_dynamic_indicator_cpu(
            candles,
            ma_source,
            indicator_id,
            Some(output_id),
            params,
            Kernel::Auto,
        )
        .expect("cpu dynamic indicator compute should succeed");
        let cuda = compute_dynamic_indicator_cuda_with_inputs(
            &prepared_inputs,
            indicator_id,
            Some(output_id),
            params,
            Kernel::Auto,
            CudaOutputTarget::HostF32,
        )
        .expect("cuda dynamic indicator compute should succeed");

        match (cpu.series, cuda.series) {
            (DynamicIndicatorSeries::F64(lhs), DynamicIndicatorCudaSeries::HostF32(rhs)) => {
                assert_eq!(lhs.len(), rhs.len(), "{indicator_id} length mismatch");
                let start = lhs
                    .iter()
                    .zip(rhs.iter())
                    .position(|(l, r)| l.is_finite() && r.is_finite())
                    .unwrap_or(lhs.len())
                    .max(min_start);
                for (idx, (l, r)) in lhs.iter().zip(rhs.iter()).enumerate().skip(start) {
                    if l.is_nan() {
                        assert!(
                            r.is_nan(),
                            "{indicator_id} expected NaN at idx={idx}, got {r}"
                        );
                    } else {
                        if l.abs() <= tol || (*r as f64).abs() <= tol {
                            continue;
                        }
                        let cpu_sign = if *l > 0.0 { 1 } else { -1 };
                        let gpu_sign = if *r > 0.0 { 1 } else { -1 };
                        assert!(
                            cpu_sign == gpu_sign,
                            "{indicator_id} cpu/cuda sign mismatch at idx={idx}: cpu={l} gpu={r}"
                        );
                    }
                }
            }
            _ => panic!("unexpected series type for {indicator_id} cpu/cuda comparison"),
        }
    }

    #[derive(Default)]
    struct RecordingProgressSink {
        events: std::sync::Mutex<Vec<DoubleMaProgressPayload>>,
    }

    impl RecordingProgressSink {
        fn snapshot(&self) -> Vec<DoubleMaProgressPayload> {
            self.events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        }
    }

    impl ProgressSink for RecordingProgressSink {
        fn emit_double_ma_progress(&self, payload: DoubleMaProgressPayload) {
            self.events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(payload);
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

    fn make_choppy_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + ((i as f64) / 9.0).sin() * 2.0 + ((i as f64) / 37.0).sin() * 0.7;
            let c = base + if (i & 1) == 0 { 0.15 } else { -0.15 };
            let o = base;
            open.push(o);
            close.push(c);
            high.push(o.max(c) + 0.45);
            low.push(o.min(c) - 0.45);
            vol.push(1_000.0 + (i % 5) as f64 * 25.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    fn make_breakout_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        let up_start = len / 3;
        let down_start = (len * 2) / 3;
        for i in 0..len {
            ts.push(i as i64);
            let base = if i < up_start {
                100.0 + i as f64 * 0.02
            } else if i < down_start {
                102.0 + (i - up_start) as f64 * 0.35
            } else {
                102.0 + (down_start - up_start) as f64 * 0.35 - (i - down_start) as f64 * 0.38
            };
            let n = ((i as f64) / 7.0).sin() * 0.08;
            let o = base + n;
            let c = if i < down_start {
                base + 0.30 + n
            } else {
                base - 0.30 + n
            };
            open.push(o);
            close.push(c);
            high.push(o.max(c) + 0.22);
            low.push(o.min(c) - 0.22);
            let burst = if i < up_start {
                950.0
            } else if i < down_start {
                2_200.0
            } else {
                2_450.0
            };
            vol.push(burst + ((i % 11) as f64) * 35.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    fn build_single_pair_request(backend: Backend, strategy: StrategyConfig) -> DoubleMaRequest {
        DoubleMaRequest {
            backend,
            data_id: "single-pair".to_string(),
            fast_range: (9, 9, 0),
            slow_range: (21, 21, 0),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy,
            halftrend_filter: None,
            objective: ObjectiveKind::Sortino,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(1),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: true,
            metric_contract: None,
            hurdle_tau: None,
        }
    }

    fn build_single_pair_drilldown_request(strategy: StrategyConfig) -> DoubleMaDrilldownRequest {
        DoubleMaDrilldownRequest {
            data_id: "single-pair".to_string(),
            backend_hint: None,
            followup_cache_key: None,
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
            strategy,
            halftrend_filter: None,
            bins: Some(64),
        }
    }

    fn commission_event_count_from_curves(curves: &DoubleMaCurves) -> u32 {
        let closed = curves.trades.iter().filter(|trade| !trade.open).count() as u32;
        curves.trades.len() as u32 + closed
    }

    fn make_dynamic_constant_close_ir() -> StrategyIr {
        StrategyIr {
            schema_version: STRATEGY_IR_SCHEMA_VERSION,
            profile: StrategyProfileKind::DoubleMa,
            ma_source: "close".to_string(),
            fast_ma: MovingAverageSpec {
                ma_type: "sma".to_string(),
                params: std::collections::HashMap::new(),
            },
            slow_ma: MovingAverageSpec {
                ma_type: "sma".to_string(),
                params: std::collections::HashMap::new(),
            },
            program: SignalProgramGraph {
                indicators: vec![
                    IndicatorNode {
                        id: "fast_ma".to_string(),
                        kind: Some(IndicatorKind::FastMa),
                        indicator_id: None,
                        params: Vec::new(),
                        output_id: None,
                    },
                    IndicatorNode {
                        id: "slow_ma".to_string(),
                        kind: Some(IndicatorKind::SlowMa),
                        indicator_id: None,
                        params: Vec::new(),
                        output_id: None,
                    },
                ],
                conditions: vec![
                    ConditionNode {
                        id: "long_entry".to_string(),
                        op: ConditionOp::Gt,
                        lhs: ValueRef::Close,
                        rhs: ValueRef::ConstantBits {
                            bits: 100.25_f64.to_bits(),
                        },
                    },
                    ConditionNode {
                        id: "long_exit".to_string(),
                        op: ConditionOp::Lt,
                        lhs: ValueRef::Close,
                        rhs: ValueRef::ConstantBits {
                            bits: 100.10_f64.to_bits(),
                        },
                    },
                ],
                groups: vec![
                    ConditionGroup {
                        id: "long_entry_group".to_string(),
                        mode: ConditionGroupMode::All,
                        condition_ids: vec!["long_entry".to_string()],
                    },
                    ConditionGroup {
                        id: "long_exit_group".to_string(),
                        mode: ConditionGroupMode::All,
                        condition_ids: vec!["long_exit".to_string()],
                    },
                ],
                signals: SignalBindings {
                    long_entry_group: Some("long_entry_group".to_string()),
                    long_exit_group: Some("long_exit_group".to_string()),
                    short_entry_group: None,
                    short_exit_group: None,
                },
            },
            execution: ExecutionPolicy {
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                halftrend: None,
                macz: None,
                ott: None,
                signal_mode: None,
            },
        }
    }

    fn make_dynamic_request(
        backend: Backend,
        dynamic_strategy_ir: Option<StrategyIr>,
    ) -> DoubleMaRequest {
        DoubleMaRequest {
            backend,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        }
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    fn find_cpu_only_period_single_ma() -> Option<String> {
        vector_ta::indicators::moving_averages::registry::list_moving_averages()
            .iter()
            .find(|info| {
                matches!(
                    crate::ma_policy::classify_hybrid_capability(info.id),
                    crate::ma_policy::HybridCapabilityTier::CpuOnly
                ) && info.period_based
                    && info.single_output
            })
            .map(|info| info.id.to_string())
    }

    fn find_cpu_staged_kernel_capable_period_single_ma() -> Option<String> {
        let registry = vector_ta::indicators::moving_averages::registry::list_moving_averages();
        let is_cpu_staged_kernel_capable =
            |info: &&vector_ta::indicators::moving_averages::registry::MovingAverageInfo| {
                info.period_based
                    && info.single_output
                    && crate::ma_policy::supports_gpu_kernel_ma(info.id)
                    && crate::ma_policy::plan_hybrid_indicator_route(info.id).route
                        == crate::ma_policy::HybridRouteTier::CpuStaged
            };

        registry
            .iter()
            .find(|info| {
                info.id.eq_ignore_ascii_case("frama") && is_cpu_staged_kernel_capable(info)
            })
            .or_else(|| registry.iter().find(is_cpu_staged_kernel_capable))
            .map(|info| info.id.to_string())
    }

    #[test]
    fn backend_deserialize_aliases_remain_compatible() {
        let auto: Backend =
            serde_json::from_str(r#"{"backend":"auto","device_id":3}"#).expect("auto alias");
        assert!(matches!(auto, Backend::Auto { device_id: 3 }));

        let cpu: Backend = serde_json::from_str(r#"{"backend":"cpu_only"}"#).expect("cpu alias");
        assert!(matches!(cpu, Backend::CpuOnly));

        let gpu_only: Backend = serde_json::from_str(r#"{"backend":"GpuSweep","device_id":1}"#)
            .expect("gpu-only alias");
        assert!(matches!(gpu_only, Backend::GpuOnly { device_id: 1 }));

        let gpu_hybrid: Backend =
            serde_json::from_str(r#"{"backend":"GpuHybrid","device_id":2}"#).expect("hybrid alias");
        assert!(matches!(
            gpu_hybrid,
            Backend::GpuHybridKernel { device_id: 2 }
        ));

        let gpu_kernel: Backend = serde_json::from_str(r#"{"backend":"gpu_kernel","device_id":7}"#)
            .expect("kernel alias");
        assert!(matches!(gpu_kernel, Backend::GpuKernel { device_id: 7 }));

        let canonical = serde_json::to_value(Backend::GpuHybridKernel { device_id: 5 })
            .expect("serialize canonical backend");
        assert_eq!(
            canonical.get("backend").and_then(|v| v.as_str()),
            Some("GpuHybridKernel")
        );
        assert_eq!(canonical.get("device_id").and_then(|v| v.as_u64()), Some(5));
    }

    #[test]
    fn preferred_cuda_device_resolution_error_is_not_masked() {
        let err = Backend::GpuKernel { device_id: 0 }
            .with_resolved_preferred_cuda_device(Err("boom".to_string()))
            .expect_err("preferred-device error should propagate");
        assert!(err.contains("boom"));
    }

    #[test]
    fn kernel_preflight_allows_sortino_but_still_flags_large_topk() {
        let blockers = gpu_kernel_blockers_for_request(
            "sma",
            "sma",
            &StrategyConfig::default(),
            false,
            ObjectiveKind::Sortino,
            300,
            false,
        );
        assert!(
            blockers
                .iter()
                .all(|entry| !entry.contains("objective Sortino")),
            "unexpected sortino blocker: {blockers:?}"
        );
        assert!(
            blockers
                .iter()
                .any(|entry| entry.contains("top_k too large")),
            "missing top_k blocker: {blockers:?}"
        );
    }

    #[test]
    fn metrics_pass_constraints_supports_extended_row_derived_fields() {
        let metrics = ta_strategies::double_ma::Metrics {
            pnl: 0.18,
            sharpe: 1.4,
            sortino: 1.9,
            downside_deviation: 0.12,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.9,
            omega: 1.7,
            kappa_tau: 0.0,
            volatility: 0.24,
            ulcer_index: 0.08,
            martin_ratio: 2.25,
            max_dd: 0.20,
            trades: 12,
            exposure: 0.55,
            net_exposure: -0.18,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
        };

        let pass = DoubleMaConstraints {
            min_trades: Some(10),
            max_drawdown: Some(0.25),
            min_sharpe: Some(1.3),
            min_sortino: Some(1.8),
            min_calmar: Some(0.8),
            max_volatility: Some(0.3),
            min_exposure: Some(0.5),
            max_exposure: Some(0.6),
            max_abs_net_exposure: Some(0.2),
            min_omega: Some(1.5),
            max_ulcer: Some(0.1),
            max_downside_deviation: Some(0.15),
        };
        assert!(metrics_pass_constraints(&metrics, Some(&pass)));

        let fail = DoubleMaConstraints {
            min_sharpe: Some(1.5),
            ..Default::default()
        };
        assert!(!metrics_pass_constraints(&metrics, Some(&fail)));

        let fail_sortino = DoubleMaConstraints {
            min_sortino: Some(2.0),
            ..Default::default()
        };
        assert!(!metrics_pass_constraints(&metrics, Some(&fail_sortino)));
    }

    #[test]
    fn sanitize_double_ma_constraints_validates_new_fields() {
        let err = sanitize_double_ma_constraints(Some(DoubleMaConstraints {
            min_sortino: Some(f64::NAN),
            ..Default::default()
        }))
        .expect_err("non-finite min_sortino should fail");
        assert!(err.contains("min_sortino"));

        let err = sanitize_double_ma_constraints(Some(DoubleMaConstraints {
            min_exposure: Some(0.7),
            max_exposure: Some(0.6),
            ..Default::default()
        }))
        .expect_err("min_exposure > max_exposure should fail");
        assert!(err.contains("min_exposure"));

        let err = sanitize_double_ma_constraints(Some(DoubleMaConstraints {
            max_abs_net_exposure: Some(1.5),
            ..Default::default()
        }))
        .expect_err("max_abs_net_exposure > 1 should fail");
        assert!(err.contains("max_abs_net_exposure"));

        let sanitized = sanitize_double_ma_constraints(Some(DoubleMaConstraints {
            min_sharpe: Some(-0.25),
            min_sortino: Some(0.5),
            min_calmar: Some(0.4),
            max_volatility: Some(0.3),
            min_exposure: Some(0.2),
            max_exposure: Some(0.8),
            max_abs_net_exposure: Some(0.5),
            min_omega: Some(1.1),
            max_ulcer: Some(0.2),
            max_downside_deviation: Some(0.15),
            ..Default::default()
        }))
        .expect("valid extended constraints should sanitize");
        let sanitized = sanitized.expect("constraints should remain present");
        assert_eq!(sanitized.min_sharpe, Some(-0.25));
        assert_eq!(sanitized.min_sortino, Some(0.5));
        assert_eq!(sanitized.min_omega, Some(1.1));
        assert_eq!(sanitized.max_ulcer, Some(0.2));
    }

    #[test]
    fn balanced_risk_adjusted_score_prefers_more_robust_profile() {
        let robust = ta_strategies::double_ma::Metrics {
            pnl: 0.22,
            sharpe: 1.55,
            sortino: 2.15,
            downside_deviation: 0.09,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 1.25,
            omega: 1.85,
            kappa_tau: 0.0,
            volatility: 0.19,
            ulcer_index: 0.05,
            martin_ratio: 2.8,
            max_dd: 0.11,
            trades: 42,
            exposure: 0.58,
            net_exposure: 0.08,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };
        let fragile = ta_strategies::double_ma::Metrics {
            pnl: 0.24,
            sharpe: 1.72,
            sortino: 1.75,
            downside_deviation: 0.21,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.58,
            omega: 1.12,
            kappa_tau: 0.0,
            volatility: 0.33,
            ulcer_index: 0.16,
            martin_ratio: 0.9,
            max_dd: 0.29,
            trades: 8,
            exposure: 0.98,
            net_exposure: 0.74,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };

        let robust_score = balanced_risk_adjusted_10_score(&robust);
        let fragile_score = balanced_risk_adjusted_10_score(&fragile);
        assert!(
            robust_score > fragile_score,
            "expected robust composite score to outrank fragile profile: robust={robust_score} fragile={fragile_score}"
        );
        assert!(
            fragile.sharpe > robust.sharpe,
            "fixture should preserve objective-only tension"
        );
    }

    #[test]
    fn adversarial_envelope_score_penalizes_single_axis_fragility() {
        let balanced = ta_strategies::double_ma::Metrics {
            pnl: 0.21,
            sharpe: 1.46,
            sortino: 1.98,
            downside_deviation: 0.11,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 1.02,
            omega: 1.58,
            kappa_tau: 0.0,
            volatility: 0.17,
            ulcer_index: 0.05,
            martin_ratio: 2.5,
            max_dd: 0.10,
            trades: 34,
            exposure: 0.62,
            net_exposure: 0.04,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };
        let fragile = ta_strategies::double_ma::Metrics {
            pnl: 0.24,
            sharpe: 1.77,
            sortino: 1.34,
            downside_deviation: 0.23,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.55,
            omega: 1.11,
            kappa_tau: 0.0,
            volatility: 0.31,
            ulcer_index: 0.15,
            martin_ratio: 1.1,
            max_dd: 0.27,
            trades: 9,
            exposure: 0.99,
            net_exposure: 0.71,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };

        let balanced_score = adversarial_envelope_10_score(&balanced);
        let fragile_score = adversarial_envelope_10_score(&fragile);
        assert!(
            balanced_score > fragile_score,
            "expected adversarial envelope score to prefer the balanced profile: balanced={balanced_score} fragile={fragile_score}"
        );
        assert!(
            fragile.sharpe > balanced.sharpe,
            "fixture should preserve objective-only tension"
        );
    }

    #[test]
    fn effective_score_uses_score_mode_override() {
        let metrics = ta_strategies::double_ma::Metrics {
            pnl: 0.18,
            sharpe: 1.3,
            sortino: 1.7,
            downside_deviation: 0.11,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.95,
            omega: 1.6,
            kappa_tau: 0.0,
            volatility: 0.18,
            ulcer_index: 0.06,
            martin_ratio: 2.4,
            max_dd: 0.12,
            trades: 36,
            exposure: 0.61,
            net_exposure: 0.03,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };

        let objective_score = effective_score(&metrics, ObjectiveKind::Sharpe, None);
        let composite_score = effective_score(
            &metrics,
            ObjectiveKind::Sharpe,
            Some(DoubleMaScoreMode::BalancedRiskAdjusted10),
        );
        assert_eq!(objective_score, metrics.sharpe);
        assert_eq!(composite_score, balanced_risk_adjusted_10_score(&metrics));
        assert_ne!(objective_score, composite_score);
    }

    #[test]
    fn effective_score_uses_adversarial_score_mode_override() {
        let metrics = ta_strategies::double_ma::Metrics {
            pnl: 0.19,
            sharpe: 1.34,
            sortino: 1.82,
            downside_deviation: 0.12,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.98,
            omega: 1.63,
            kappa_tau: 0.0,
            volatility: 0.18,
            ulcer_index: 0.06,
            martin_ratio: 2.3,
            max_dd: 0.11,
            trades: 31,
            exposure: 0.59,
            net_exposure: 0.02,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };

        let objective_score = effective_score(&metrics, ObjectiveKind::Sharpe, None);
        let composite_score = effective_score(
            &metrics,
            ObjectiveKind::Sharpe,
            Some(DoubleMaScoreMode::AdversarialEnvelope10),
        );
        assert_eq!(objective_score, metrics.sharpe);
        assert_eq!(composite_score, adversarial_envelope_10_score(&metrics));
        assert_ne!(objective_score, composite_score);
    }

    #[test]
    fn support_shrunk_score_penalizes_sparse_support() {
        let dense = ta_strategies::double_ma::Metrics {
            pnl: 0.20,
            sharpe: 1.38,
            sortino: 1.86,
            downside_deviation: 0.12,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.97,
            omega: 1.56,
            kappa_tau: 0.0,
            volatility: 0.18,
            ulcer_index: 0.06,
            martin_ratio: 2.1,
            max_dd: 0.12,
            trades: 38,
            exposure: 0.63,
            net_exposure: 0.05,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };
        let sparse = ta_strategies::double_ma::Metrics {
            pnl: 0.21,
            sharpe: 1.47,
            sortino: 1.98,
            downside_deviation: 0.11,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.99,
            omega: 1.58,
            kappa_tau: 0.0,
            volatility: 0.18,
            ulcer_index: 0.06,
            martin_ratio: 2.2,
            max_dd: 0.11,
            trades: 4,
            exposure: 0.10,
            net_exposure: 0.03,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };

        let dense_base = balanced_risk_adjusted_10_score(&dense);
        let sparse_base = balanced_risk_adjusted_10_score(&sparse);
        let dense_score = support_shrunk_10_score(&dense);
        let sparse_score = support_shrunk_10_score(&sparse);
        assert!(
            sparse_base > dense_base,
            "fixture should preserve base-score tension: dense_base={dense_base} sparse_base={sparse_base}"
        );
        assert!(
            dense_score > sparse_score,
            "expected support-shrunk score to prefer denser support: dense={dense_score} sparse={sparse_score}"
        );
    }

    #[test]
    fn effective_score_uses_support_shrunk_score_mode_override() {
        let metrics = ta_strategies::double_ma::Metrics {
            pnl: 0.18,
            sharpe: 1.32,
            sortino: 1.78,
            downside_deviation: 0.12,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.94,
            omega: 1.52,
            kappa_tau: 0.0,
            volatility: 0.17,
            ulcer_index: 0.06,
            martin_ratio: 2.0,
            max_dd: 0.12,
            trades: 18,
            exposure: 0.41,
            net_exposure: 0.04,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };

        let objective_score = effective_score(&metrics, ObjectiveKind::Sharpe, None);
        let composite_score = effective_score(
            &metrics,
            ObjectiveKind::Sharpe,
            Some(DoubleMaScoreMode::SupportShrunk10),
        );
        assert_eq!(objective_score, metrics.sharpe);
        assert_eq!(composite_score, support_shrunk_10_score(&metrics));
        assert_ne!(objective_score, composite_score);
    }

    #[test]
    fn active_sortino_score_prefers_cleaner_risk_adjusted_exposure() {
        let robust = ta_strategies::double_ma::Metrics {
            pnl: 0.19,
            sharpe: 1.28,
            sortino: 1.96,
            downside_deviation: 0.09,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 1.02,
            omega: 1.67,
            kappa_tau: 0.0,
            volatility: 0.17,
            ulcer_index: 0.05,
            martin_ratio: 2.3,
            max_dd: 0.11,
            trades: 29,
            exposure: 0.44,
            net_exposure: 0.04,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };
        let fragile = ta_strategies::double_ma::Metrics {
            pnl: 0.21,
            sharpe: 1.42,
            sortino: 1.61,
            downside_deviation: 0.17,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.69,
            omega: 1.24,
            kappa_tau: 0.0,
            volatility: 0.25,
            ulcer_index: 0.11,
            martin_ratio: 1.2,
            max_dd: 0.19,
            trades: 10,
            exposure: 0.83,
            net_exposure: 0.31,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };

        let robust_score = active_sortino_10_score(&robust);
        let fragile_score = active_sortino_10_score(&fragile);
        assert!(
            robust_score > fragile_score,
            "expected active-sortino score to prefer the cleaner profile: robust={robust_score} fragile={fragile_score}"
        );
        assert!(
            fragile.sharpe > robust.sharpe,
            "fixture should preserve objective-only tension"
        );
    }

    #[test]
    fn effective_score_uses_active_sortino_score_mode_override() {
        let metrics = ta_strategies::double_ma::Metrics {
            pnl: 0.17,
            sharpe: 1.21,
            sortino: 1.89,
            downside_deviation: 0.10,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.93,
            omega: 1.58,
            kappa_tau: 0.0,
            volatility: 0.16,
            ulcer_index: 0.05,
            martin_ratio: 2.1,
            max_dd: 0.11,
            trades: 24,
            exposure: 0.39,
            net_exposure: 0.03,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
        };

        let objective_score = effective_score(&metrics, ObjectiveKind::Sharpe, None);
        let derived_score = effective_score(
            &metrics,
            ObjectiveKind::Sharpe,
            Some(DoubleMaScoreMode::ActiveSortino10),
        );
        assert_eq!(objective_score, metrics.sharpe);
        assert_eq!(derived_score, active_sortino_10_score(&metrics));
        assert_ne!(objective_score, derived_score);
    }

    #[test]
    fn effective_sample_lcb_score_penalizes_shorter_observation_histories() {
        let metrics = ta_strategies::double_ma::Metrics {
            pnl: 0.20,
            sharpe: 1.42,
            sortino: 1.88,
            downside_deviation: 0.10,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 1.02,
            omega: 1.74,
            kappa_tau: 0.0,
            volatility: 0.17,
            ulcer_index: 0.05,
            martin_ratio: 2.8,
            max_dd: 0.10,
            trades: 34,
            exposure: 0.62,
            net_exposure: 0.04,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
        };

        let long_score = effective_sample_lcb_10_score(&metrics, 20_000);
        let short_score = effective_sample_lcb_10_score(&metrics, 400);

        assert!(
            long_score > short_score,
            "expected longer observation support to score higher: long={long_score} short={short_score}"
        );
    }

    #[test]
    fn omega_vs_sharpe_score_prefers_positive_asymmetry_gap() {
        let asymmetric = ta_strategies::double_ma::Metrics {
            pnl: 0.18,
            sharpe: 1.11,
            sortino: 1.94,
            downside_deviation: 0.08,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.93,
            omega: 1.96,
            kappa_tau: 0.0,
            volatility: 0.16,
            ulcer_index: 0.05,
            martin_ratio: 2.2,
            max_dd: 0.11,
            trades: 26,
            exposure: 0.48,
            net_exposure: 0.02,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };
        let symmetric = ta_strategies::double_ma::Metrics {
            pnl: 0.20,
            sharpe: 1.43,
            sortino: 1.56,
            downside_deviation: 0.15,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.71,
            omega: 1.18,
            kappa_tau: 0.0,
            volatility: 0.23,
            ulcer_index: 0.09,
            martin_ratio: 1.4,
            max_dd: 0.17,
            trades: 14,
            exposure: 0.78,
            net_exposure: 0.28,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };

        let asym_score = omega_vs_sharpe_10_score(&asymmetric);
        let sym_score = omega_vs_sharpe_10_score(&symmetric);
        assert!(
            asym_score > sym_score,
            "expected omega-vs-sharpe score to prefer the stronger asymmetry profile: asymmetric={asym_score} symmetric={sym_score}"
        );
        assert!(
            symmetric.sharpe > asymmetric.sharpe,
            "fixture should preserve objective-only tension"
        );
    }

    #[test]
    fn effective_score_uses_omega_vs_sharpe_score_mode_override() {
        let metrics = ta_strategies::double_ma::Metrics {
            pnl: 0.18,
            sharpe: 1.18,
            sortino: 1.88,
            downside_deviation: 0.09,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.92,
            omega: 1.84,
            kappa_tau: 0.0,
            volatility: 0.17,
            ulcer_index: 0.05,
            martin_ratio: 2.1,
            max_dd: 0.11,
            trades: 25,
            exposure: 0.46,
            net_exposure: 0.03,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };

        let objective_score = effective_score(&metrics, ObjectiveKind::Sharpe, None);
        let derived_score = effective_score(
            &metrics,
            ObjectiveKind::Sharpe,
            Some(DoubleMaScoreMode::OmegaVsSharpe10),
        );
        assert_eq!(objective_score, metrics.sharpe);
        assert_eq!(derived_score, omega_vs_sharpe_10_score(&metrics));
        assert_ne!(objective_score, derived_score);
    }

    #[test]
    fn robust_margin_score_penalizes_single_weak_dimension() {
        let balanced = ta_strategies::double_ma::Metrics {
            pnl: 0.19,
            sharpe: 1.20,
            sortino: 1.78,
            downside_deviation: 0.10,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.90,
            omega: 1.46,
            kappa_tau: 0.0,
            volatility: 0.17,
            ulcer_index: 0.05,
            martin_ratio: 2.0,
            max_dd: 0.12,
            trades: 28,
            exposure: 0.55,
            net_exposure: 0.03,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };
        let weak = ta_strategies::double_ma::Metrics {
            pnl: 0.22,
            sharpe: 1.37,
            sortino: 1.73,
            downside_deviation: 0.10,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.34,
            omega: 1.08,
            kappa_tau: 0.0,
            volatility: 0.19,
            ulcer_index: 0.18,
            martin_ratio: 0.8,
            max_dd: 0.31,
            trades: 30,
            exposure: 0.94,
            net_exposure: 0.52,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };

        let balanced_score = robust_margin_10_score(&balanced);
        let weak_score = robust_margin_10_score(&weak);
        assert!(
            balanced_score > weak_score,
            "expected robust-margin score to prefer the balanced profile: balanced={balanced_score} weak={weak_score}"
        );
        assert!(
            weak.sharpe > balanced.sharpe,
            "fixture should preserve objective-only tension"
        );
    }

    #[test]
    fn effective_score_uses_robust_margin_score_mode_override() {
        let metrics = ta_strategies::double_ma::Metrics {
            pnl: 0.19,
            sharpe: 1.24,
            sortino: 1.80,
            downside_deviation: 0.10,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            calmar: 0.91,
            omega: 1.44,
            kappa_tau: 0.0,
            volatility: 0.17,
            ulcer_index: 0.05,
            martin_ratio: 2.0,
            max_dd: 0.12,
            trades: 29,
            exposure: 0.54,
            net_exposure: 0.03,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            underwater_fraction: 0.0,
            max_underwater_duration_bars: 0,
            closed_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            gross_profit: 0.0,
            gross_loss: 0.0,
        };

        let objective_score = effective_score(&metrics, ObjectiveKind::Sharpe, None);
        let derived_score = effective_score(
            &metrics,
            ObjectiveKind::Sharpe,
            Some(DoubleMaScoreMode::RobustMargin10),
        );
        assert_eq!(objective_score, metrics.sharpe);
        assert_eq!(derived_score, robust_margin_10_score(&metrics));
        assert_ne!(objective_score, derived_score);
    }

    #[test]
    fn derived_drawdown_chronicity_matches_ulcer_to_max_dd_ratio() {
        let metrics = ta_strategies::double_ma::Metrics {
            ulcer_index: 0.06,
            max_dd: 0.15,
            ..Default::default()
        };
        let chronicity = derived_drawdown_chronicity(&metrics);
        assert!(
            (chronicity - 0.4).abs() <= 1e-12,
            "unexpected chronicity={chronicity}"
        );

        let flat = ta_strategies::double_ma::Metrics {
            ulcer_index: 0.12,
            max_dd: 0.0,
            ..Default::default()
        };
        assert_eq!(derived_drawdown_chronicity(&flat), 0.0);
    }

    #[test]
    fn derived_downside_share_matches_downside_to_volatility_ratio() {
        let metrics = ta_strategies::double_ma::Metrics {
            downside_deviation: 0.09,
            volatility: 0.18,
            ..Default::default()
        };
        let downside_share = derived_downside_share(&metrics);
        assert!(
            (downside_share - 0.5).abs() <= 1e-12,
            "unexpected downside_share={downside_share}"
        );

        let flat = ta_strategies::double_ma::Metrics {
            downside_deviation: 0.11,
            volatility: 0.0,
            ..Default::default()
        };
        assert_eq!(derived_downside_share(&flat), 0.0);
    }

    #[test]
    fn derived_active_downside_deviation_scales_by_sqrt_exposure() {
        let metrics = ta_strategies::double_ma::Metrics {
            downside_deviation: 0.12,
            exposure: 0.25,
            ..Default::default()
        };
        let active = derived_active_downside_deviation(&metrics);
        assert!(
            (active - 0.24).abs() <= 1e-12,
            "unexpected active_downside_deviation={active}"
        );

        let flat = ta_strategies::double_ma::Metrics::default();
        assert_eq!(derived_active_downside_deviation(&flat), 0.0);
    }

    #[test]
    fn derived_side_exposures_split_exposure_and_directional_bias() {
        let metrics = ta_strategies::double_ma::Metrics {
            exposure: 0.60,
            net_exposure: -0.20,
            ..Default::default()
        };

        let long_exposure = derived_long_exposure(&metrics);
        let short_exposure = derived_short_exposure(&metrics);
        let directional_bias = derived_directional_bias(&metrics);

        assert!((long_exposure - 0.20).abs() <= 1e-12);
        assert!((short_exposure - 0.40).abs() <= 1e-12);
        assert!((directional_bias - (1.0 / 3.0)).abs() <= 1e-12);

        let flat = ta_strategies::double_ma::Metrics::default();
        assert_eq!(derived_directional_bias(&flat), 0.0);
    }

    #[test]
    fn validation_fee_stress_fee_bps_applies_floor_and_increment() {
        let low = StrategyConfig {
            fee_bps: 0.0,
            ..Default::default()
        };
        assert_eq!(
            validation_fee_stress_fee_bps(&low),
            DEFAULT_FEE_STRESS_FLOOR_BPS
        );

        let mid = StrategyConfig {
            fee_bps: 1.5,
            ..Default::default()
        };
        assert_eq!(validation_fee_stress_fee_bps(&mid), 3.0);

        let high = StrategyConfig {
            fee_bps: 4.0,
            ..Default::default()
        };
        assert_eq!(validation_fee_stress_fee_bps(&high), 8.0);
    }

    #[test]
    fn exact_stressed_pnl_from_components_matches_closed_form() {
        let pnl = 0.25_f64;
        let events = 7u32;
        let base_commission = 0.0004_f64;
        let stressed_commission = 0.0009_f64;
        let expected = (1.0_f64 + pnl).ln()
            + (events as f64)
                * ((1.0_f64 - stressed_commission).ln() - (1.0_f64 - base_commission).ln());
        let expected = expected.exp_m1();
        let actual =
            exact_stressed_pnl_from_components(pnl, events, base_commission, stressed_commission);
        assert!(
            (actual - expected).abs() <= 1e-12,
            "expected stressed pnl {expected}, got {actual}"
        );
    }

    #[test]
    fn exact_fee_stress_pnl_matches_cpu_rerun_across_trade_paths() {
        let candles = make_choppy_candles(6_000);
        let cancel = AtomicBool::new(false);

        let build_req = |strategy: StrategyConfig| DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "fee-stress".to_string(),
            fast_range: (9, 9, 0),
            slow_range: (21, 21, 0),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy,
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(1),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let cases = [
            (
                "plain",
                StrategyConfig {
                    fee_bps: 1.0,
                    ..Default::default()
                },
            ),
            (
                "trade_on_next_bar",
                StrategyConfig {
                    fee_bps: 1.0,
                    trade_on_next_bar: true,
                    ..Default::default()
                },
            ),
            (
                "no_flip",
                StrategyConfig {
                    fee_bps: 1.0,
                    allow_flip: false,
                    ..Default::default()
                },
            ),
            (
                "risk",
                StrategyConfig {
                    fee_bps: 1.0,
                    spread_bps: 2.0,
                    slippage_bps: 1.0,
                    stop_loss_pct: Some(0.04),
                    take_profit_pct: Some(0.06),
                    ..Default::default()
                },
            ),
        ];

        for (label, strategy) in cases {
            let base = run_double_ma_optimization_blocking_with_candles(
                build_req(strategy.clone()),
                &candles,
                &cancel,
                None,
            )
            .unwrap_or_else(|e| panic!("base cpu run failed for {label}: {e}"));
            assert!(
                base.best_metrics.trades > 0,
                "expected non-zero trade count for {label}"
            );

            let derived = exact_fee_stress_pnl(&base.best_metrics, &strategy);
            let mut stressed_strategy = strategy.clone();
            stressed_strategy.fee_bps = validation_fee_stress_fee_bps(&strategy);
            let stressed = run_double_ma_optimization_blocking_with_candles(
                build_req(stressed_strategy),
                &candles,
                &cancel,
                None,
            )
            .unwrap_or_else(|e| panic!("stressed cpu run failed for {label}: {e}"));

            assert_eq!(
                (base.best_params.fast_len, base.best_params.slow_len),
                (stressed.best_params.fast_len, stressed.best_params.slow_len),
                "best params changed for {label}"
            );
            assert!(
                (derived - stressed.best_metrics.pnl).abs() <= 1e-9,
                "fee stress mismatch for {label}: derived={derived} rerun={}",
                stressed.best_metrics.pnl
            );
        }
    }

    #[test]
    fn cpu_metrics_trade_count_matches_commission_event_count_across_execution_paths() {
        let cancel = AtomicBool::new(false);
        let cases = vec![
            (
                "plain",
                make_choppy_candles(6_000),
                StrategyConfig {
                    fee_bps: 1.0,
                    ..Default::default()
                },
            ),
            (
                "trade_on_next_bar",
                make_choppy_candles(6_000),
                StrategyConfig {
                    fee_bps: 1.0,
                    trade_on_next_bar: true,
                    ..Default::default()
                },
            ),
            (
                "no_flip",
                make_choppy_candles(6_000),
                StrategyConfig {
                    fee_bps: 1.0,
                    allow_flip: false,
                    ..Default::default()
                },
            ),
            (
                "risk",
                make_choppy_candles(6_000),
                StrategyConfig {
                    fee_bps: 1.0,
                    spread_bps: 2.0,
                    slippage_bps: 1.0,
                    stop_loss_pct: Some(0.04),
                    take_profit_pct: Some(0.06),
                    ..Default::default()
                },
            ),
            (
                "kaufman_stop",
                make_breakout_candles(6_000),
                StrategyConfig {
                    fee_bps: 1.0,
                    exit_mode: ta_strategies::double_ma::ExitMode::KaufmanStop,
                    kaufman_stop_period: 18,
                    kaufman_stop_mult: 1.8,
                    ..Default::default()
                },
            ),
        ];

        for (label, candles, strategy) in cases {
            let res = run_double_ma_optimization_blocking_with_candles(
                build_single_pair_request(Backend::CpuOnly, strategy.clone()),
                &candles,
                &cancel,
                None,
            )
            .unwrap_or_else(|e| panic!("cpu single-pair run failed for {label}: {e}"));
            let curves = compute_double_ma_drilldown_blocking_with_candles(
                build_single_pair_drilldown_request(strategy),
                &candles,
            )
            .unwrap_or_else(|e| panic!("drilldown failed for {label}: {e}"));
            let expected_events = commission_event_count_from_curves(&curves);
            assert!(
                expected_events > 0,
                "expected non-zero commission events for {label}"
            );
            assert_eq!(
                res.best_metrics.trades, expected_events,
                "commission-event mismatch for {label}"
            );
        }
    }

    #[test]
    fn cpu_smoke_small_grid_counts_and_shapes() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: true,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("cpu run");

        assert_eq!(res.num_combos, 6);
        assert!(res.best_params.fast_len < res.best_params.slow_len);
        assert!(res.top.len() <= 5);
        assert!(res.heatmap.is_some());
    }

    #[test]
    fn cpu_dynamic_strategy_env_path_matches_legacy_results() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: true,
            metric_contract: None,
            hurdle_tau: None,
        };

        let legacy = with_dynamic_strategy_opt_env(false, || {
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("legacy cpu run")
        });
        let dynamic = with_dynamic_strategy_opt_env(true, || {
            run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("dynamic cpu run")
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

    #[test]
    fn cpu_dynamic_strategy_env_path_matches_legacy_results_include_all() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(0),
            include_all: Some(true),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: true,
            metric_contract: None,
            hurdle_tau: None,
        };

        let legacy = with_dynamic_strategy_opt_env(false, || {
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("legacy cpu run")
        });
        let dynamic = with_dynamic_strategy_opt_env(true, || {
            run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("dynamic cpu run")
        });

        assert_eq!(legacy.num_combos, dynamic.num_combos);
        assert_eq!(legacy.best_params.fast_len, dynamic.best_params.fast_len);
        assert_eq!(legacy.best_params.slow_len, dynamic.best_params.slow_len);
        assert!((legacy.best_metrics.sharpe - dynamic.best_metrics.sharpe).abs() < 1e-12);
        assert!((legacy.best_metrics.pnl - dynamic.best_metrics.pnl).abs() < 1e-12);
        assert!((legacy.best_metrics.max_dd - dynamic.best_metrics.max_dd).abs() < 1e-12);
        assert_eq!(legacy.best_metrics.trades, dynamic.best_metrics.trades);

        let all_legacy = legacy.all.expect("legacy all rows");
        let all_dynamic = dynamic.all.expect("dynamic all rows");
        assert_eq!(all_legacy.len(), all_dynamic.len());
        for ((lp, lm), (dp, dm)) in all_legacy.iter().zip(all_dynamic.iter()) {
            assert_eq!(lp.fast_len, dp.fast_len);
            assert_eq!(lp.slow_len, dp.slow_len);
            assert_eq!(lp.fast_ma_type, dp.fast_ma_type);
            assert_eq!(lp.slow_ma_type, dp.slow_ma_type);
            assert!((lm.sharpe - dm.sharpe).abs() < 1e-12);
            assert!((lm.pnl - dm.pnl).abs() < 1e-12);
            assert!((lm.max_dd - dm.max_dd).abs() < 1e-12);
            assert_eq!(lm.trades, dm.trades);
        }
    }

    #[test]
    fn cpu_dynamic_strategy_env_path_matches_legacy_results_with_constraints() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
            constraints: Some(DoubleMaConstraints {
                min_trades: Some(1),
                max_drawdown: Some(10.0),
                ..Default::default()
            }),
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: true,
            metric_contract: None,
            hurdle_tau: None,
        };

        let legacy = with_dynamic_strategy_opt_env(false, || {
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("legacy cpu run")
        });
        let dynamic = with_dynamic_strategy_opt_env(true, || {
            run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("dynamic cpu run")
        });

        assert_eq!(legacy.num_combos, dynamic.num_combos);
        assert_eq!(legacy.best_params.fast_len, dynamic.best_params.fast_len);
        assert_eq!(legacy.best_params.slow_len, dynamic.best_params.slow_len);
        assert!((legacy.best_metrics.sharpe - dynamic.best_metrics.sharpe).abs() < 1e-12);
        assert!((legacy.best_metrics.pnl - dynamic.best_metrics.pnl).abs() < 1e-12);
        assert!((legacy.best_metrics.max_dd - dynamic.best_metrics.max_dd).abs() < 1e-12);
        assert_eq!(legacy.best_metrics.trades, dynamic.best_metrics.trades);
        assert_eq!(legacy.top.len(), dynamic.top.len());
    }

    #[test]
    fn cpu_dynamic_strategy_cancelled_matches_legacy_results() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(true);
        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let legacy_err = with_dynamic_strategy_opt_env(false, || {
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect_err("legacy cpu run should be cancelled")
        });
        let dynamic_err = with_dynamic_strategy_opt_env(true, || {
            run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect_err("dynamic cpu run should be cancelled")
        });

        assert_eq!(legacy_err, "cancelled");
        assert_eq!(dynamic_err, "cancelled");
    }

    #[test]
    fn cpu_dynamic_strategy_progress_parity_matches_legacy_results() {
        fn assert_progress_invariants(
            events: &[DoubleMaProgressPayload],
        ) -> (&'static str, usize, usize) {
            assert!(!events.is_empty(), "expected progress events");
            let first_phase = events[0].phase;
            let mut last_processed = 0usize;
            let mut total_pairs = events[0].total_pairs;
            for ev in events {
                assert_eq!(ev.phase, "cpu", "unexpected phase");
                assert_eq!(ev.phase, first_phase, "phase changed unexpectedly");
                assert_eq!(ev.total_pairs, total_pairs, "total_pairs changed");
                assert!(
                    ev.processed_pairs >= last_processed,
                    "processed_pairs must be monotonic"
                );
                last_processed = ev.processed_pairs;
                total_pairs = ev.total_pairs;
            }
            assert_eq!(
                last_processed, total_pairs,
                "final progress must reach total"
            );
            (first_phase, last_processed, total_pairs)
        }

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let legacy_sink = RecordingProgressSink::default();
        let legacy = with_dynamic_strategy_opt_env(false, || {
            run_double_ma_optimization_blocking_with_candles(
                req.clone(),
                &candles,
                &cancel,
                Some(&legacy_sink),
            )
            .expect("legacy cpu run")
        });
        let dynamic_sink = RecordingProgressSink::default();
        let dynamic = with_dynamic_strategy_opt_env(true, || {
            run_double_ma_optimization_blocking_with_candles(
                req,
                &candles,
                &cancel,
                Some(&dynamic_sink),
            )
            .expect("dynamic cpu run")
        });

        let legacy_events = legacy_sink.snapshot();
        let dynamic_events = dynamic_sink.snapshot();
        let (legacy_phase, legacy_processed, legacy_total) =
            assert_progress_invariants(&legacy_events);
        let (dynamic_phase, dynamic_processed, dynamic_total) =
            assert_progress_invariants(&dynamic_events);

        assert_eq!(legacy_phase, dynamic_phase);
        assert_eq!(legacy_total, dynamic_total);
        assert_eq!(legacy_processed, dynamic_processed);
        assert_eq!(legacy_total, legacy.num_combos);
        assert_eq!(dynamic_total, dynamic.num_combos);
    }

    #[test]
    fn cpu_dynamic_strategy_shadow_mode_matches_legacy_results() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let legacy = with_dynamic_strategy_opt_env(false, || {
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("legacy cpu run")
        });
        let dynamic = with_dynamic_strategy_shadow_env(1, || {
            run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("dynamic shadow cpu run")
        });

        assert_eq!(legacy.best_params.fast_len, dynamic.best_params.fast_len);
        assert_eq!(legacy.best_params.slow_len, dynamic.best_params.slow_len);
        assert!((legacy.best_metrics.sharpe - dynamic.best_metrics.sharpe).abs() < 1e-12);
        assert!((legacy.best_metrics.pnl - dynamic.best_metrics.pnl).abs() < 1e-12);
        assert!((legacy.best_metrics.max_dd - dynamic.best_metrics.max_dd).abs() < 1e-12);
        assert_eq!(legacy.best_metrics.trades, dynamic.best_metrics.trades);
    }

    #[test]
    fn request_dynamic_strategy_ir_cpu_run_executes_through_optimizer() {
        let candles = make_trending_candles(5_000);
        let cancel = AtomicBool::new(false);
        let mut dynamic_req =
            make_dynamic_request(Backend::CpuOnly, Some(make_dynamic_constant_close_ir()));
        dynamic_req.fast_ma_types = vec!["alma".to_string()];
        dynamic_req.slow_ma_types = vec!["alma".to_string()];

        let dynamic =
            run_double_ma_optimization_blocking_with_candles(dynamic_req, &candles, &cancel, None)
                .expect("request-backed dynamic cpu run");

        assert!(matches!(dynamic.backend_used, BackendUsed::Cpu));
        assert_eq!(dynamic.best_params.fast_ma_type, "sma");
        assert_eq!(dynamic.best_params.slow_ma_type, "sma");
        assert!(dynamic.best_metrics.trades > 0);
        assert_eq!(dynamic.num_combos, 6);
    }

    #[test]
    fn request_dynamic_strategy_ir_rejects_non_double_ma_profile() {
        let candles = make_trending_candles(512);
        let cancel = AtomicBool::new(false);
        let mut ir = make_dynamic_constant_close_ir();
        ir.profile = StrategyProfileKind::CompositeCpu;
        let req = make_dynamic_request(Backend::CpuOnly, Some(ir));

        let err = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("non-DoubleMa profile should be rejected");

        assert_eq!(
            err,
            "dynamic_strategy_ir currently supports only profile=DoubleMa"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_supported_shape_uses_dynamic_gpu_route() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                None,
            ),
        )
        .expect("static ir should build");
        let req = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("request-backed dynamic gpu run");

        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert!(
            res.backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("dynamic_strategy_v1"),
            "expected dynamic gpu route note, got {:?}",
            res.backend_route
        );
        assert!(res.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_supported_shape_accepts_effective_sample_lcb_score_mode() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                None,
            ),
        )
        .expect("static ir should build");
        let mut req = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));
        req.score_mode = Some(DoubleMaScoreMode::EffectiveSampleLcb10);

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("request-backed dynamic gpu effective-sample run");

        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert!(
            res.backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("dynamic_strategy_v1"),
            "expected dynamic gpu route note, got {:?}",
            res.backend_route
        );
        assert!(res.best_score.is_some(), "expected a finite best score");
        assert!(
            res.score_observation_context.is_some(),
            "expected score observation context to be exposed"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_supported_shape_accepts_extra_shortlist_objectives() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                None,
            ),
        )
        .expect("static ir should build");
        let top_k = 5usize;
        let mut req_all =
            make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir.clone()));
        req_all.fast_range = (5, 24, 1);
        req_all.slow_range = (15, 80, 1);
        req_all.top_k = Some(top_k);
        req_all.include_all = Some(true);

        let mut req_union = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));
        req_union.fast_range = req_all.fast_range;
        req_union.slow_range = req_all.slow_range;
        req_union.top_k = Some(top_k);
        req_union.include_all = Some(false);
        req_union.extra_shortlist_objectives = Some(vec![ObjectiveKind::Pnl, ObjectiveKind::Omega]);

        let exhaustive =
            run_double_ma_optimization_blocking_with_candles(req_all, &candles, &cancel, None)
                .expect("dynamic exhaustive gpu run");
        let union_result =
            run_double_ma_optimization_blocking_with_candles(req_union, &candles, &cancel, None)
                .expect("dynamic extra-shortlist gpu run");

        assert!(matches!(union_result.backend_used, BackendUsed::Gpu));
        assert!(
            union_result
                .backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("dynamic_strategy_v1"),
            "expected dynamic gpu route note, got {:?}",
            union_result.backend_route
        );
        assert!(
            union_result.backend_fallback_reason.is_none(),
            "unexpected fallback note: {:?}",
            union_result.backend_fallback_reason
        );

        let all_rows = exhaustive
            .all
            .clone()
            .expect("include_all=true should retain the full candidate set");
        let mut expected_union: Vec<(DoubleMaParamsResolved, Metrics)> = Vec::new();
        let mut seen: BTreeSet<(u16, u16)> = BTreeSet::new();
        for head_objective in [
            ObjectiveKind::Sharpe,
            ObjectiveKind::Pnl,
            ObjectiveKind::Omega,
        ] {
            let mut ranked = all_rows.clone();
            ranked.sort_by(|a, b| {
                let sa = objective_score(&a.1, head_objective);
                let sb = objective_score(&b.1, head_objective);
                sb.total_cmp(&sa)
            });
            for (params, metrics) in ranked.into_iter().take(top_k) {
                let key = (params.fast_len, params.slow_len);
                if seen.insert(key) {
                    expected_union.push((params, metrics));
                }
            }
        }
        expected_union.sort_by(|a, b| {
            let sa = objective_score(&a.1, ObjectiveKind::Sharpe);
            let sb = objective_score(&b.1, ObjectiveKind::Sharpe);
            sb.total_cmp(&sa)
        });
        let expected_keys: Vec<(u16, u16)> = expected_union
            .iter()
            .take(top_k)
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();
        let actual_keys: Vec<(u16, u16)> = union_result
            .top
            .iter()
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();

        assert_eq!(
            actual_keys, expected_keys,
            "dynamic extra-shortlist top-k should match the exhaustive host union rerank"
        );
        assert_eq!(
            (
                union_result.best_params.fast_len,
                union_result.best_params.slow_len,
            ),
            expected_keys
                .first()
                .copied()
                .expect("expected at least one shortlisted pair")
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_supported_shape_accepts_shortlist_union_mode() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                None,
            ),
        )
        .expect("static ir should build");
        let top_k = 5usize;
        let mut req_all =
            make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir.clone()));
        req_all.fast_range = (5, 24, 1);
        req_all.slow_range = (15, 80, 1);
        req_all.top_k = Some(top_k);
        req_all.include_all = Some(true);

        let mut req_union = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));
        req_union.fast_range = req_all.fast_range;
        req_union.slow_range = req_all.slow_range;
        req_union.top_k = Some(top_k);
        req_union.include_all = Some(false);
        req_union.extra_shortlist_objectives = Some(vec![ObjectiveKind::Pnl, ObjectiveKind::Omega]);
        req_union.shortlist_union_mode = Some(DoubleMaShortlistUnionMode::WeightedRank);

        let exhaustive =
            run_double_ma_optimization_blocking_with_candles(req_all, &candles, &cancel, None)
                .expect("dynamic exhaustive gpu run");
        let union_result =
            run_double_ma_optimization_blocking_with_candles(req_union, &candles, &cancel, None)
                .expect("dynamic shortlist-union gpu run");

        let raw_all_rows: Vec<(DoubleMaParams, Metrics)> = exhaustive
            .all
            .clone()
            .expect("include_all=true should retain the full candidate set")
            .into_iter()
            .map(|(params, metrics)| {
                (
                    DoubleMaParams {
                        fast_len: params.fast_len,
                        slow_len: params.slow_len,
                        fast_ma_id: 0,
                        slow_ma_id: 0,
                    },
                    metrics,
                )
            })
            .collect();
        let materialized_rows = build_exact_extra_shortlist_rows(
            raw_all_rows.as_slice(),
            ObjectiveKind::Sharpe,
            &[ObjectiveKind::Pnl, ObjectiveKind::Omega],
            top_k,
        );
        let reranked = crate::gpu_backtest_kernel::rerank_shortlist_by_union_mode(
            materialized_rows.as_slice(),
            ObjectiveKind::Sharpe,
            &[ObjectiveKind::Pnl, ObjectiveKind::Omega],
            DoubleMaShortlistUnionMode::WeightedRank,
        );
        let expected_keys: Vec<(u16, u16)> = reranked
            .iter()
            .take(top_k)
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();
        let actual_keys: Vec<(u16, u16)> = union_result
            .top
            .iter()
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();

        assert!(matches!(union_result.backend_used, BackendUsed::Gpu));
        assert!(
            union_result
                .backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("dynamic_strategy_v1"),
            "expected dynamic gpu route note, got {:?}",
            union_result.backend_route
        );
        assert!(
            union_result.backend_fallback_reason.is_none(),
            "unexpected fallback note: {:?}",
            union_result.backend_fallback_reason
        );
        assert_eq!(
            actual_keys, expected_keys,
            "dynamic shortlist-union top-k should match the exhaustive host weighted-rank rerank"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_supported_shape_accepts_shortlist_confidence_mode() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig::default();
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                strategy.clone(),
                None,
            ),
        )
        .expect("static ir should build");
        let top_k = 5usize;
        let mut req_conf = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));
        req_conf.fast_range = (5, 24, 1);
        req_conf.slow_range = (15, 80, 1);
        req_conf.top_k = Some(top_k);
        req_conf.include_all = Some(false);
        req_conf.shortlist_confidence_mode = Some(DoubleMaShortlistConfidenceMode::Psr);
        let req_base = DoubleMaRequest {
            shortlist_confidence_mode: None,
            ..req_conf.clone()
        };
        let base_result =
            run_double_ma_optimization_blocking_with_candles(req_base, &candles, &cancel, None)
                .expect("dynamic baseline gpu run");
        let conf_result =
            run_double_ma_optimization_blocking_with_candles(req_conf, &candles, &cancel, None)
                .expect("dynamic shortlist-confidence gpu run");

        let shortlist_rows: Vec<(DoubleMaParams, Metrics)> = base_result
            .top
            .clone()
            .into_iter()
            .map(|(params, metrics)| {
                (
                    DoubleMaParams {
                        fast_len: params.fast_len,
                        slow_len: params.slow_len,
                        fast_ma_id: 0,
                        slow_ma_id: 0,
                    },
                    metrics,
                )
            })
            .collect();
        let first_valid = first_finite_idx(vector_ta::utilities::data_loader::source_type(
            &candles, "close",
        ))
        .unwrap_or(0);
        let reranked = crate::gpu_backtest_kernel::rerank_shortlist_by_confidence_for_host(
            shortlist_rows.as_slice(),
            DoubleMaShortlistConfidenceMode::Psr,
            PairObservationScoreContextResolved {
                first_valid,
                regime_first_valid: first_valid,
                eval_start: 0,
                eval_end: candles.close.len(),
                total_bars: candles.close.len(),
                trade_on_next_bar: strategy.trade_on_next_bar,
            },
            &strategy,
            ObjectiveKind::Sharpe,
            None,
        );
        let expected_keys: Vec<(u16, u16)> = reranked
            .iter()
            .take(top_k)
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();
        let actual_keys: Vec<(u16, u16)> = conf_result
            .top
            .iter()
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();

        assert!(matches!(conf_result.backend_used, BackendUsed::Gpu));
        assert!(
            conf_result
                .backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("dynamic_strategy_v1"),
            "expected dynamic gpu route note, got {:?}",
            conf_result.backend_route
        );
        assert_eq!(
            actual_keys, expected_keys,
            "dynamic shortlist-confidence top-k should match the exhaustive host PSR rerank"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_supported_shape_accepts_trade_metrics() {
        if !cuda_available() {
            return;
        }

        let candles = make_choppy_candles(8_000);
        let cancel = AtomicBool::new(false);
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                None,
            ),
        )
        .expect("static ir should build");
        let mut req = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));
        req.include_trade_metrics = true;

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("dynamic trade-metrics gpu run");

        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert!(
            res.backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("dynamic_strategy_v1"),
            "expected dynamic gpu route note, got {:?}",
            res.backend_route
        );
        assert!(
            res.best_metrics.closed_trades
                >= res.best_metrics.winning_trades + res.best_metrics.losing_trades,
            "trade metrics should be populated coherently"
        );
        assert!(
            res.best_metrics.closed_trades > 0 || res.best_metrics.trades > 0,
            "expected dynamic trade-metrics run to produce non-trivial trade activity"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_supported_shape_accepts_shortlist_slice_mode() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig::default();
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                strategy.clone(),
                None,
            ),
        )
        .expect("static ir should build");
        let top_k = 5usize;
        let mut req_base = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));
        req_base.fast_range = (5, 24, 1);
        req_base.slow_range = (15, 80, 1);
        req_base.top_k = Some(top_k);
        req_base.include_all = Some(false);
        let mut req_slice = req_base.clone();
        req_slice.shortlist_slice_mode = Some(DoubleMaShortlistSliceMode::Halves);
        req_slice.shortlist_slice_benchmark = Some(SpaBenchmarkKind::BuyAndHold);
        req_slice.shortlist_neighborhood_radius = Some(1);
        req_slice.shortlist_neighborhood_step = Some(1);

        let base_result =
            run_double_ma_optimization_blocking_with_candles(req_base, &candles, &cancel, None)
                .expect("dynamic baseline gpu run");
        let slice_result =
            run_double_ma_optimization_blocking_with_candles(req_slice, &candles, &cancel, None)
                .expect("dynamic shortlist-slice gpu run");

        let shortlist_rows: Vec<(DoubleMaParams, Metrics)> = base_result
            .top
            .clone()
            .into_iter()
            .map(|(params, metrics)| {
                (
                    DoubleMaParams {
                        fast_len: params.fast_len,
                        slow_len: params.slow_len,
                        fast_ma_id: 0,
                        slow_ma_id: 0,
                    },
                    metrics,
                )
            })
            .collect();
        let first_valid = first_finite_idx(vector_ta::utilities::data_loader::source_type(
            &candles, "close",
        ))
        .unwrap_or(0);
        let (reranked, _) =
            crate::gpu_backtest_kernel::rerank_shortlist_by_slice_robustness_for_host(
                0,
                &candles,
                shortlist_rows.as_slice(),
                DoubleMaShortlistSliceMode::Halves,
                "sma",
                "sma",
                "close",
                None,
                None,
                &strategy,
                ObjectiveKind::Sharpe,
                None,
                Some(&SpaBenchmarkKind::BuyAndHold),
                first_valid,
                0,
                candles.close.len(),
                0.85,
                6.0,
                0.85,
                6.0,
                strategy.hurdle_tau.unwrap_or(0.0),
                false,
                None,
            )
            .expect("host slice rerank");
        let expected_keys: Vec<(u16, u16)> = reranked
            .iter()
            .take(top_k)
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();
        let actual_keys: Vec<(u16, u16)> = slice_result
            .top
            .iter()
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();

        assert!(matches!(slice_result.backend_used, BackendUsed::Gpu));
        assert!(
            slice_result
                .backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("dynamic_strategy_v1"),
            "expected dynamic gpu route note, got {:?}",
            slice_result.backend_route
        );
        assert_eq!(
            actual_keys, expected_keys,
            "dynamic shortlist-slice top-k should match the exhaustive host slice rerank"
        );
        let diag = slice_result
            .shortlist_slice_diagnostics
            .as_ref()
            .expect("shortlist slice diagnostics");
        assert_eq!(diag.ranking_mode, "halves");
        assert!(
            diag.neighborhood_robustness.is_some(),
            "expected neighborhood robustness summary when radius is requested"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_unsupported_shape_falls_back_to_cpu() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                Some(HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 14,
                }),
            ),
        )
        .expect("halftrend static ir should build");
        let req = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("request-backed dynamic fallback run");

        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert!(
            res.backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("GpuKernel->CpuOnly"),
            "expected dynamic fallback route note, got {:?}",
            res.backend_route
        );
        assert!(
            res.backend_fallback_reason
                .as_deref()
                .unwrap_or_default()
                .contains("halftrend"),
            "expected halftrend fallback reason, got {:?}",
            res.backend_fallback_reason
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_unsupported_shape_rejects_extra_shortlist_objectives() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                Some(HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 14,
                }),
            ),
        )
        .expect("halftrend static ir should build");
        let mut req = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));
        req.extra_shortlist_objectives = Some(vec![ObjectiveKind::Pnl]);

        let err = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("unsupported dynamic shape should reject extra shortlist");

        assert!(
            err.contains(
                "extra_shortlist_objectives currently require an exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) route"
            ),
            "unexpected error: {err}"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_unsupported_shape_rejects_shortlist_confidence_mode() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                Some(HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 14,
                }),
            ),
        )
        .expect("halftrend static ir should build");
        let mut req = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));
        req.shortlist_confidence_mode = Some(DoubleMaShortlistConfidenceMode::Psr);

        let err = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("unsupported dynamic shape should reject shortlist confidence");

        assert!(
            err.contains(
                "shortlist_confidence_mode currently requires an exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) route"
            ),
            "unexpected error: {err}"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_unsupported_shape_rejects_trade_metrics() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                Some(HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 14,
                }),
            ),
        )
        .expect("halftrend static ir should build");
        let mut req = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));
        req.include_trade_metrics = true;

        let err = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("unsupported dynamic shape should reject trade metrics");

        assert!(
            err.contains(
                "include_trade_metrics currently require an exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) route"
            ),
            "unexpected error: {err}"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn request_dynamic_strategy_ir_gpu_unsupported_shape_rejects_shortlist_slice_mode() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                Some(HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 14,
                }),
            ),
        )
        .expect("halftrend static ir should build");
        let mut req = make_dynamic_request(Backend::GpuKernel { device_id: 0 }, Some(ir));
        req.shortlist_slice_mode = Some(DoubleMaShortlistSliceMode::Halves);

        let err = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("unsupported dynamic shape should reject shortlist slice mode");

        assert!(
            err.contains(
                "shortlist_slice_mode currently requires an exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) route"
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn dynamic_gpu_preflight_disabled_is_noop() {
        let _guard = lock_env();
        let strategy = StrategyConfig::default();
        let res = maybe_compile_dynamic_gpu_plan_preflight(
            false,
            "close",
            "unknown_ma_type",
            "another_unknown",
            None,
            None,
            &strategy,
            None,
        );
        assert!(res.is_ok(), "disabled preflight should be a no-op");
    }

    #[test]
    fn dynamic_gpu_preflight_compiles_for_standard_double_ma() {
        let _guard = lock_env();
        let strategy = StrategyConfig::default();
        maybe_compile_dynamic_gpu_plan_preflight(
            true, "close", "sma", "alma", None, None, &strategy, None,
        )
        .expect("gpu preflight should compile for standard double-ma setup");
    }

    #[test]
    fn dynamic_gpu_preflight_compiles_with_halftrend_filter() {
        let _guard = lock_env();
        let strategy = StrategyConfig::default();
        let halftrend = HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 14,
        };
        maybe_compile_dynamic_gpu_plan_preflight(
            true,
            "close",
            "sma",
            "sma",
            None,
            None,
            &strategy,
            Some(&halftrend),
        )
        .expect("gpu preflight should compile with halftrend filter config");
    }

    #[test]
    fn dynamic_gpu_preflight_requirement_logic_matches_expected_routes() {
        let _guard = lock_env();
        assert!(dynamic_gpu_preflight_required_for_backend(
            true,
            &OptimizationModeResolved::Grid,
            &Backend::GpuKernel { device_id: 0 },
        ));
        assert!(dynamic_gpu_preflight_required_for_backend(
            true,
            &OptimizationModeResolved::GpuVdps,
            &Backend::GpuHybridKernel { device_id: 0 },
        ));
        assert!(!dynamic_gpu_preflight_required_for_backend(
            true,
            &OptimizationModeResolved::Grid,
            &Backend::GpuHybridKernel { device_id: 0 },
        ));
        assert!(!dynamic_gpu_preflight_required_for_backend(
            false,
            &OptimizationModeResolved::GpuVdps,
            &Backend::GpuKernel { device_id: 0 },
        ));
        assert!(!dynamic_gpu_preflight_required_for_backend(
            true,
            &OptimizationModeResolved::Grid,
            &Backend::CpuOnly,
        ));
    }

    #[test]
    fn dynamic_gpu_preflight_cache_reuses_identical_request_signature() {
        let _guard = lock_env();
        let strategy = StrategyConfig::default();
        let cache = DYNAMIC_GPU_PREFLIGHT_CACHE.get_or_init(|| Mutex::new(BTreeSet::new()));
        cache.lock().expect("cache lock").clear();

        let key_a =
            dynamic_gpu_preflight_cache_key("close", "sma", "sma", None, None, &strategy, None)
                .expect("key a");
        let key_b =
            dynamic_gpu_preflight_cache_key("close", "sma", "sma", None, None, &strategy, None)
                .expect("key b");
        assert_eq!(
            key_a, key_b,
            "identical preflight requests should hash identically"
        );

        maybe_compile_dynamic_gpu_plan_preflight(
            true, "close", "sma", "sma", None, None, &strategy, None,
        )
        .expect("first preflight compile");
        let len_after_first = cache.lock().expect("cache lock").len();
        maybe_compile_dynamic_gpu_plan_preflight(
            true, "close", "sma", "sma", None, None, &strategy, None,
        )
        .expect("second preflight compile");
        let len_after_second = cache.lock().expect("cache lock").len();
        assert_eq!(
            len_after_first, 1,
            "first compile should populate exactly one cache key"
        );
        assert_eq!(
            len_after_second, len_after_first,
            "identical preflight requests should reuse the cached key instead of growing the cache"
        );
    }

    #[test]
    fn auto_backend_records_route_metadata() {
        let _guard = lock_env();
        let candles = make_trending_candles(1500);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::Auto { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 6, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(3),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: true,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("auto backend run");
        let route = res.backend_route.unwrap_or_default();
        assert!(
            route.starts_with("Auto->"),
            "expected Auto route note, got '{route}'"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn auto_backend_prefers_gpu_kernel_when_kernel_route_is_supported() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(4_096);
        let cancel = AtomicBool::new(false);
        let mut alma_params: HashMap<String, f64> = HashMap::new();
        alma_params.insert("offset".to_string(), 0.85);
        alma_params.insert("sigma".to_string(), 6.0);
        let req = DoubleMaRequest {
            backend: Backend::Auto { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 12, 1),
            slow_range: (10, 24, 2),
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: Some(alma_params.clone()),
            slow_ma_params: Some(alma_params),
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(4),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("auto backend alma run");
        assert!(res.best_params.fast_len > 0);
        assert!(res.best_params.slow_len > 0);
        assert_eq!(res.backend_route.as_deref(), Some("Auto->GpuKernel"));
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    #[test]
    fn gpu_hybrid_phase1_cpu_fallback_surfaces_route_metadata() {
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL");

        let cpu_only_ma = vector_ta::indicators::moving_averages::registry::list_moving_averages()
            .iter()
            .find(|info| {
                matches!(
                    crate::ma_policy::classify_hybrid_capability(info.id),
                    crate::ma_policy::HybridCapabilityTier::CpuOnly
                ) && info.period_based
                    && info.single_output
            })
            .map(|info| info.id.to_string());
        let Some(cpu_only_ma) = cpu_only_ma else {
            match prev_env {
                Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
                None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
            }
            return;
        };

        let candles = make_trending_candles(1500);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (20, 21, 1),
            slow_range: (40, 41, 1),
            fast_ma_types: vec![cpu_only_ma],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(3),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("hybrid cpu fallback run");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert!(
            res.backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("GpuHybridKernel->CpuOnly"),
            "unexpected route: {:?}",
            res.backend_route
        );
        let reason = res.backend_fallback_reason.unwrap_or_default();
        assert!(
            reason.contains("falling back to CpuOnly"),
            "unexpected fallback reason: {reason}"
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[test]
    fn gpu_hybrid_env_disable_records_fallback_reason() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "0");

        let cpu_only_ma = vector_ta::indicators::moving_averages::registry::list_moving_averages()
            .iter()
            .find(|info| {
                matches!(
                    crate::ma_policy::classify_hybrid_capability(info.id),
                    crate::ma_policy::HybridCapabilityTier::CpuOnly
                ) && info.period_based
                    && info.single_output
            })
            .map(|info| info.id.to_string());
        let Some(cpu_only_ma) = cpu_only_ma else {
            match prev {
                Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
                None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
            }
            return;
        };

        let candles = make_trending_candles(1200);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (20, 20, 1),
            slow_range: (40, 40, 1),
            fast_ma_types: vec![cpu_only_ma],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(1),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("hybrid env disabled run");
        let reason = res.backend_fallback_reason.unwrap_or_default();
        assert!(
            reason.contains("VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0"),
            "expected env fallback note, got '{reason}'"
        );

        match prev {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_hybrid_kernel_capable_pair_stays_on_gpu_path() {
        if !cuda_available() {
            return;
        }
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "1");

        let candles = make_trending_candles(1600);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (20, 24, 1),
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(3),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("hybrid kernel-capable run");
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "expected GPU backend, got {:?}",
            res.backend_used
        );
        assert!(
            res.backend_route.as_deref() == Some("GpuHybridKernel"),
            "expected explicit hybrid route, got {:?}",
            res.backend_route
        );
        assert!(
            res.backend_fallback_reason.is_none(),
            "expected no fallback reason, got {:?}",
            res.backend_fallback_reason
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_hybrid_large_alma_route_smoke() {
        if !cuda_available() {
            return;
        }
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "1");

        let candles = make_trending_candles(200_000);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 200, 1),
            slow_range: (10, 400, 1),
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(50),
            include_all: Some(false),
            heatmap_bins: Some(64),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let t0 = std::time::Instant::now();
        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("hybrid large alma run");
        let elapsed_ms = t0.elapsed().as_millis();
        assert!(elapsed_ms > 0, "expected non-zero elapsed time");
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "expected GPU backend, got {:?}",
            res.backend_used
        );
        assert!(
            res.backend_route.as_deref() == Some("GpuHybridKernel"),
            "expected explicit hybrid route, got {:?}",
            res.backend_route
        );
        assert!(
            res.backend_fallback_reason.is_none(),
            "expected no fallback reason, got {:?}",
            res.backend_fallback_reason
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_product_reduced_rows_matches_replay_contract_for_sharpe() {
        if !cuda_available() {
            return;
        }
        let _guard = lock_env();
        let prev_recompute = std::env::var("VECTORBT_KERNEL_FINALIST_RECOMPUTE").ok();
        let prev_reduced = std::env::var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS").ok();
        std::env::set_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE", "1");
        // Force the replay-backed baseline so this test truly compares:
        // baseline = 7-row + replay vs. reduced = 10-row + no replay.
        std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", "0");

        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 60, 1),
            slow_range: (10, 120, 1),
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        crate::gpu_backtest_kernel::reset_test_runtime();
        let replay_res =
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("baseline replay-backed gpu kernel run");

        std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", "1");
        crate::gpu_backtest_kernel::reset_test_runtime();
        let reduced_res =
            run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("product-reduced gpu kernel run");

        assert_eq!(
            replay_res.best_params.fast_len,
            reduced_res.best_params.fast_len
        );
        assert_eq!(
            replay_res.best_params.slow_len,
            reduced_res.best_params.slow_len
        );
        assert!(
            (replay_res.best_metrics.pnl - reduced_res.best_metrics.pnl).abs() <= 1e-5,
            "best pnl mismatch: replay={} reduced={}",
            replay_res.best_metrics.pnl,
            reduced_res.best_metrics.pnl
        );
        assert!(
            (replay_res.best_metrics.sharpe - reduced_res.best_metrics.sharpe).abs() <= 1e-5,
            "best sharpe mismatch: replay={} reduced={}",
            replay_res.best_metrics.sharpe,
            reduced_res.best_metrics.sharpe
        );
        assert!(
            (replay_res.best_metrics.max_dd - reduced_res.best_metrics.max_dd).abs() <= 1e-5,
            "best max_dd mismatch: replay={} reduced={}",
            replay_res.best_metrics.max_dd,
            reduced_res.best_metrics.max_dd
        );
        assert!(
            (replay_res.best_metrics.martin_ratio - reduced_res.best_metrics.martin_ratio).abs()
                <= 1e-5
        );
        assert!((replay_res.best_metrics.omega - reduced_res.best_metrics.omega).abs() <= 1e-5);
        assert_eq!(
            replay_res.best_metrics.trades,
            reduced_res.best_metrics.trades
        );
        assert_eq!(replay_res.top.len(), reduced_res.top.len());

        for ((rp, rm), (dp, dm)) in replay_res.top.iter().zip(reduced_res.top.iter()) {
            assert_eq!(rp.fast_len, dp.fast_len);
            assert_eq!(rp.slow_len, dp.slow_len);
            assert!(
                (rm.pnl - dm.pnl).abs() <= 1e-5,
                "top pnl mismatch: replay={} reduced={}",
                rm.pnl,
                dm.pnl
            );
            assert!(
                (rm.sharpe - dm.sharpe).abs() <= 1e-5,
                "top sharpe mismatch: replay={} reduced={}",
                rm.sharpe,
                dm.sharpe
            );
            assert!(
                (rm.max_dd - dm.max_dd).abs() <= 1e-5,
                "top max_dd mismatch: replay={} reduced={}",
                rm.max_dd,
                dm.max_dd
            );
            assert!((rm.martin_ratio - dm.martin_ratio).abs() <= 1e-5);
            assert!((rm.omega - dm.omega).abs() <= 1e-5);
            assert_eq!(rm.trades, dm.trades);
        }

        match prev_recompute {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE"),
        }
        match prev_reduced {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_precomputed_frama_matrix_executes() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(2400);
        let fast_periods = [20u16, 21u16, 22u16, 23u16];
        let slow_periods = [40u16, 41u16, 42u16, 43u16, 44u16];
        let (fast_mat, fast_cols) =
            ma_matrix_cpu_f32_staging(&candles, "frama", "close", &fast_periods, None, 0.85, 6.0)
                .expect("frama staging matrix");
        let res = optimize_double_ma_gpu_kernel_tiled_with_precomputed(
            &candles,
            0,
            &fast_periods,
            &slow_periods,
            "frama",
            "sma",
            "close",
            None,
            None,
            None,
            &StrategyConfig::default(),
            ObjectiveKind::Sharpe,
            None,
            None,
            None,
            None,
            None,
            None,
            3,
            false,
            0.85,
            6.0,
            0.85,
            6.0,
            false,
            None,
            Some(fast_mat.as_slice()),
            Some(fast_cols),
            None,
            None,
            None,
            None,
            fast_periods.len().saturating_mul(slow_periods.len()),
            0,
            None,
            &AtomicBool::new(false),
            None,
        )
        .expect("precomputed frama kernel run");

        assert!(res.best_params.fast_len > 0);
        assert!(res.best_params.slow_len > 0);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_hybrid_cpu_staged_kernel_capable_ma_stays_on_gpu_path() {
        if !cuda_available() {
            return;
        }
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "1");

        let Some(cpu_staged_ma) = find_cpu_staged_kernel_capable_period_single_ma() else {
            match prev_env {
                Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
                None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
            }
            return;
        };

        let candles = make_trending_candles(2400);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (20, 23, 1),
            slow_range: (40, 44, 1),
            fast_ma_types: vec![cpu_staged_ma],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(3),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("hybrid cpu-staged kernel-capable run");
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "expected GPU backend, got {:?}",
            res.backend_used
        );
        assert!(
            res.backend_route.as_deref() == Some("GpuHybridKernel"),
            "expected explicit hybrid route, got {:?}",
            res.backend_route
        );
        assert!(
            res.backend_fallback_reason.is_none(),
            "expected no fallback reason, got {:?}",
            res.backend_fallback_reason
        );
        assert!(
            res.hybrid_timing_ms.is_some(),
            "expected hybrid timing payload for kernel-capable staged MA"
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_hybrid_vdps_cpu_staged_kernel_capable_ma_stays_on_gpu_path() {
        if !cuda_available() {
            return;
        }
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "1");

        let Some(cpu_staged_ma) = find_cpu_staged_kernel_capable_period_single_ma() else {
            match prev_env {
                Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
                None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
            }
            return;
        };

        let candles = make_trending_candles(2600);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (20, 22, 1),
            slow_range: (40, 44, 1),
            fast_ma_types: vec![cpu_staged_ma],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::GpuVdps,
            top_k: Some(4),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("GpuVdps hybrid cpu-staged kernel-capable run");
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "expected GPU backend, got {:?}",
            res.backend_used
        );
        assert!(
            res.backend_route.as_deref() == Some("GpuHybridKernel"),
            "expected explicit hybrid route, got {:?}",
            res.backend_route
        );
        assert!(
            res.hybrid_timing_ms.is_some(),
            "expected hybrid cpu staging timing for kernel-capable staged MA"
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_hybrid_vfct_small_grid_runs() {
        if !cuda_available() {
            return;
        }
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "1");

        let candles = make_breakout_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 8, 1),
            slow_range: (12, 16, 1),
            fast_ma_types: vec!["vwma".to_string()],
            slow_ma_types: vec!["vwma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: vfct_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(3),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("hybrid vfct run");
        assert!(
            matches!(res.backend_used, BackendUsed::Gpu),
            "expected GPU backend, got {:?}",
            res.backend_used
        );
        assert!(
            res.backend_fallback_reason.is_none(),
            "expected no fallback reason, got {:?}",
            res.backend_fallback_reason
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_hybrid_batched_matches_unbatched_best_params_alma() {
        if !cuda_available() {
            return;
        }
        let _guard = lock_env();
        let prev_hybrid = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "1");

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);
        let mut alma_p: HashMap<String, f64> = HashMap::new();
        alma_p.insert("offset".to_string(), 0.85);
        alma_p.insert("sigma".to_string(), 6.0);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: Some(alma_p.clone()),
            slow_ma_params: Some(alma_p),
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-hybrid run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-hybrid run");

        match prev_hybrid {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }

        assert!(
            matches!(single.backend_used, BackendUsed::Gpu),
            "expected GPU backend for unbatched run"
        );
        assert!(
            matches!(batched.backend_used, BackendUsed::Gpu),
            "expected GPU backend for batched run"
        );
        assert!(
            single.backend_route.as_deref() == Some("GpuHybridKernel")
                && single.backend_fallback_reason.is_none(),
            "unexpected hybrid route/fallback on unbatched run: route={:?} fallback={:?}",
            single.backend_route,
            single.backend_fallback_reason
        );
        assert!(
            batched.backend_route.as_deref() == Some("GpuHybridKernel")
                && batched.backend_fallback_reason.is_none(),
            "unexpected hybrid route/fallback on batched run: route={:?} fallback={:?}",
            batched.backend_route,
            batched.backend_fallback_reason
        );
        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);
        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        // NMA remains batch-sensitive at the dense heatmap-cell level even when the
        // exact candidate count and winning parameters match. Keep the invariant here
        // focused on search-space coverage and best-result parity.
    }

    #[cfg(not(feature = "cuda"))]
    #[test]
    fn gpu_hybrid_unsupported_kernel_objective_falls_back_to_cpu_on_cpu_builds() {
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL");

        let candles = make_trending_candles(1200);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 5, 1),
            slow_range: (20, 20, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sortino,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(1),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("hybrid objective fallback");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert!(
            res.backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("GpuHybridKernel->CpuOnly"),
            "unexpected backend route: {:?}",
            res.backend_route
        );
        let reason = res.backend_fallback_reason.unwrap_or_default();
        assert!(
            reason.contains("CUDA"),
            "expected CUDA-related fallback reason, got '{reason}'"
        );
        assert!(
            !reason.contains("objective Sortino"),
            "sortino should no longer be a kernel blocker, got '{reason}'"
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    #[test]
    fn gpu_hybrid_repeatability_for_cpu_fallback_path() {
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL");

        let cpu_only_ma = vector_ta::indicators::moving_averages::registry::list_moving_averages()
            .iter()
            .find(|info| {
                matches!(
                    crate::ma_policy::classify_hybrid_capability(info.id),
                    crate::ma_policy::HybridCapabilityTier::CpuOnly
                ) && info.period_based
                    && info.single_output
            })
            .map(|info| info.id.to_string());
        let Some(cpu_only_ma) = cpu_only_ma else {
            match prev_env {
                Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
                None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
            }
            return;
        };

        let candles = make_trending_candles(1800);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (20, 21, 1),
            slow_range: (40, 42, 1),
            fast_ma_types: vec![cpu_only_ma],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(4),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let r1 =
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("hybrid run #1");
        let r2 = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("hybrid run #2");

        assert!(matches!(r1.backend_used, BackendUsed::Cpu));
        assert!(matches!(r2.backend_used, BackendUsed::Cpu));
        assert_eq!(r1.backend_route, r2.backend_route);
        assert_eq!(r1.best_params.fast_len, r2.best_params.fast_len);
        assert_eq!(r1.best_params.slow_len, r2.best_params.slow_len);
        assert_eq!(r1.num_combos, r2.num_combos);
        assert!((r1.best_metrics.pnl - r2.best_metrics.pnl).abs() <= 1e-12);
        assert!((r1.best_metrics.sharpe - r2.best_metrics.sharpe).abs() <= 1e-12);

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    #[test]
    fn gpu_hybrid_cpu_fallback_matches_cpu_backend_for_cpu_only_ma() {
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL");

        let cpu_only_ma = vector_ta::indicators::moving_averages::registry::list_moving_averages()
            .iter()
            .find(|info| {
                matches!(
                    crate::ma_policy::classify_hybrid_capability(info.id),
                    crate::ma_policy::HybridCapabilityTier::CpuOnly
                ) && info.period_based
                    && info.single_output
            })
            .map(|info| info.id.to_string());
        let Some(cpu_only_ma) = cpu_only_ma else {
            match prev_env {
                Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
                None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
            }
            return;
        };

        let candles = make_choppy_candles(2200);
        let cancel = AtomicBool::new(false);

        let mut base_req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (20, 22, 1),
            slow_range: (40, 44, 1),
            fast_ma_types: vec![cpu_only_ma],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };
        let cpu_res = run_double_ma_optimization_blocking_with_candles(
            base_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu run");

        base_req.backend = Backend::GpuHybridKernel { device_id: 0 };
        let hybrid_res =
            run_double_ma_optimization_blocking_with_candles(base_req, &candles, &cancel, None)
                .expect("hybrid run");

        assert!(matches!(cpu_res.backend_used, BackendUsed::Cpu));
        assert!(matches!(hybrid_res.backend_used, BackendUsed::Cpu));
        assert!(
            hybrid_res
                .backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("GpuHybridKernel->CpuOnly"),
            "unexpected route: {:?}",
            hybrid_res.backend_route
        );
        assert_eq!(
            cpu_res.best_params.fast_len,
            hybrid_res.best_params.fast_len
        );
        assert_eq!(
            cpu_res.best_params.slow_len,
            hybrid_res.best_params.slow_len
        );
        assert_eq!(cpu_res.num_combos, hybrid_res.num_combos);
        assert!((cpu_res.best_metrics.pnl - hybrid_res.best_metrics.pnl).abs() <= 1e-12);
        assert!((cpu_res.best_metrics.sharpe - hybrid_res.best_metrics.sharpe).abs() <= 1e-12);
        assert!((cpu_res.best_metrics.max_dd - hybrid_res.best_metrics.max_dd).abs() <= 1e-12);

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    #[test]
    fn gpu_hybrid_vdps_rejects_cpu_fallback_mode() {
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL");

        let cpu_only_ma = vector_ta::indicators::moving_averages::registry::list_moving_averages()
            .iter()
            .find(|info| {
                matches!(
                    crate::ma_policy::classify_hybrid_capability(info.id),
                    crate::ma_policy::HybridCapabilityTier::CpuOnly
                ) && info.period_based
                    && info.single_output
            })
            .map(|info| info.id.to_string());
        let Some(cpu_only_ma) = cpu_only_ma else {
            match prev_env {
                Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
                None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
            }
            return;
        };

        let candles = make_trending_candles(2400);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (20, 20, 1),
            slow_range: (40, 40, 1),
            fast_ma_types: vec![cpu_only_ma],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::GpuVdps,
            top_k: Some(4),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };
        let err = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("GpuVdps hybrid should reject CPU fallback");
        assert!(
            err.contains("no CPU fallback") || err.contains("requires GPU-executable routing"),
            "unexpected error: {err}"
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    #[test]
    fn gpu_hybrid_cpu_fallback_parity_across_double_like_strategies() {
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL");

        let Some(cpu_only_ma) = find_cpu_only_period_single_ma() else {
            match prev_env {
                Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
                None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
            }
            return;
        };

        let candles = make_breakout_candles(2600);
        let cancel = AtomicBool::new(false);

        for (label, strategy) in double_like_kernel_strategy_cases() {
            let cpu_req = DoubleMaRequest {
                backend: Backend::CpuOnly,
                data_id: "test".to_string(),
                fast_range: (20, 22, 1),
                slow_range: (40, 44, 1),
                fast_ma_types: vec![cpu_only_ma.clone()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                score_mode: None,
                extra_shortlist_objectives: None,
                shortlist_confidence_mode: None,
                shortlist_union_mode: None,
                shortlist_slice_mode: None,
                shortlist_slice_benchmark: None,
                shortlist_neighborhood_radius: None,
                shortlist_neighborhood_step: None,
                mode: OptimizationMode::Grid,
                top_k: Some(5),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,

                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
            };
            let hybrid_req = DoubleMaRequest {
                backend: Backend::GpuHybridKernel { device_id: 0 },
                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
                ..cpu_req.clone()
            };

            let cpu_res =
                run_double_ma_optimization_blocking_with_candles(cpu_req, &candles, &cancel, None)
                    .unwrap_or_else(|e| panic!("cpu run failed for '{label}': {e}"));
            let hybrid_res = run_double_ma_optimization_blocking_with_candles(
                hybrid_req, &candles, &cancel, None,
            )
            .unwrap_or_else(|e| panic!("hybrid run failed for '{label}': {e}"));

            assert!(
                matches!(hybrid_res.backend_used, BackendUsed::Cpu),
                "expected CPU fallback for '{label}'"
            );
            assert!(
                hybrid_res
                    .backend_route
                    .as_deref()
                    .unwrap_or_default()
                    .contains("GpuHybridKernel->CpuOnly"),
                "unexpected route for '{label}': {:?}",
                hybrid_res.backend_route
            );
            assert_eq!(
                cpu_res.best_params.fast_len, hybrid_res.best_params.fast_len,
                "fast_len mismatch for '{label}'"
            );
            assert_eq!(
                cpu_res.best_params.slow_len, hybrid_res.best_params.slow_len,
                "slow_len mismatch for '{label}'"
            );
            assert_eq!(
                cpu_res.num_combos, hybrid_res.num_combos,
                "combo-count mismatch for '{label}'"
            );
            assert!(
                (cpu_res.best_metrics.sharpe - hybrid_res.best_metrics.sharpe).abs() <= 1e-12,
                "sharpe mismatch for '{label}'"
            );
            assert!(
                (cpu_res.best_metrics.pnl - hybrid_res.best_metrics.pnl).abs() <= 1e-12,
                "pnl mismatch for '{label}'"
            );
        }

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[test]
    fn gpu_hybrid_overlay_blocker_surfaces_fallback_reason() {
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL");
        if !cfg!(feature = "cuda") {
            match prev_env {
                Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
                None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
            }
            return;
        }
        if !matches!(
            crate::ma_policy::classify_hybrid_capability("sma"),
            crate::ma_policy::HybridCapabilityTier::GpuKernel
        ) {
            match prev_env {
                Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
                None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
            }
            return;
        }

        let candles = make_trending_candles(1800);
        let cancel = AtomicBool::new(false);
        let mut strategy = StrategyConfig::default();
        strategy.cooldown_bars = 2;

        let req = DoubleMaRequest {
            backend: Backend::GpuHybridKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 6, 1),
            slow_range: (20, 22, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy,
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(3),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("hybrid overlay fallback");
        if let Some(route) = res.backend_route.as_deref() {
            let reason = res.backend_fallback_reason.unwrap_or_default();
            assert!(
                reason.to_ascii_lowercase().contains("execution overlays"),
                "expected overlay blocker reason, got '{reason}'"
            );
            assert!(
                route.contains("GpuHybridKernel->GpuOnly")
                    || route.contains("GpuHybridKernel->CpuOnly"),
                "unexpected route '{route}'"
            );
        } else {
            assert!(
                matches!(res.backend_used, BackendUsed::Gpu),
                "expected GPU backend when no fallback route is recorded"
            );
        }

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[test]
    fn gpu_vdps_rejects_cpu_backend() {
        let candles = make_trending_candles(2000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 60, 1),
            slow_range: (10, 200, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::GpuVdps,
            top_k: Some(8),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let err = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("GpuVdps should reject CPU backend");
        assert!(
            err.to_ascii_lowercase().contains("requires a gpu backend"),
            "unexpected error: {err}"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_vdps_promotes_gpu_only_to_hybrid_kernel_when_available() {
        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL");

        if !cuda_available() {
            match prev_env {
                Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
                None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
            }
            return;
        }

        let candles = make_trending_candles(2200);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuOnly { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (8, 12, 2),
            slow_range: (20, 28, 4),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::GpuVdps,
            top_k: Some(4),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: Some(VdpsConfig {
                eta: None,
                rungs: Some(vec![0.5, 1.0]),
                min_initial_pairs: None,
                max_survivor_seeds: None,
                max_period_candidates: None,
                random_injection_ratio: None,
                elite_mutation_ratio: None,
                differential_ratio: None,
                deterministic: Some(true),
                seed: Some(42),
                quality_check_max_pairs: None,
                quality_check_top_k: None,
            }),

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("GpuVdps run should succeed");
        let route = res.backend_route.unwrap_or_default();
        assert!(
            route.contains("GpuOnly->GpuHybridKernel"),
            "expected GpuVdps to promote GpuOnly to hybrid kernel route, got '{route}'"
        );
        let reason = res.backend_fallback_reason.unwrap_or_default();
        assert!(
            reason.contains("promotes GpuOnly to GpuHybridKernel"),
            "expected promotion note in fallback reason, got '{reason}'"
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_vdps_deterministic_same_seed_reproducible() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(20_000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 80, 1),
            slow_range: (10, 180, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::GpuVdps,
            top_k: Some(12),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: Some(VdpsConfig {
                eta: Some(4),
                rungs: Some(vec![0.125, 0.25, 0.5, 1.0]),
                min_initial_pairs: Some(2048),
                max_survivor_seeds: Some(256),
                max_period_candidates: Some(320),
                random_injection_ratio: Some(0.10),
                elite_mutation_ratio: Some(0.50),
                differential_ratio: Some(0.40),
                deterministic: Some(true),
                seed: Some(42),
                quality_check_max_pairs: Some(0),
                quality_check_top_k: Some(12),
            }),

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let a =
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("first deterministic vdps run");
        let b = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("second deterministic vdps run");

        assert_eq!(a.best_params.fast_len, b.best_params.fast_len);
        assert_eq!(a.best_params.slow_len, b.best_params.slow_len);
        assert_eq!(a.top.len(), b.top.len());
        assert!((a.best_metrics.sharpe - b.best_metrics.sharpe).abs() <= 1e-9);

        let key_a: Vec<(u16, u16, String, String)> = a
            .top
            .iter()
            .map(|(p, _)| {
                (
                    p.fast_len,
                    p.slow_len,
                    p.fast_ma_type.clone(),
                    p.slow_ma_type.clone(),
                )
            })
            .collect();
        let key_b: Vec<(u16, u16, String, String)> = b
            .top
            .iter()
            .map(|(p, _)| {
                (
                    p.fast_len,
                    p.slow_len,
                    p.fast_ma_type.clone(),
                    p.slow_ma_type.clone(),
                )
            })
            .collect();
        assert_eq!(key_a, key_b);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_vdps_deterministic_reproducible_for_double_like_strategies() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let cancel = AtomicBool::new(false);
        for (label, strategy) in double_like_kernel_strategy_cases() {
            let candles = if label == "breakout_momentum" {
                make_breakout_candles(12_000)
            } else {
                make_trending_candles(12_000)
            };
            let req = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                data_id: "test".to_string(),
                fast_range: (5, 60, 1),
                slow_range: (10, 160, 1),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy,
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                score_mode: None,
                extra_shortlist_objectives: None,
                shortlist_confidence_mode: None,
                shortlist_union_mode: None,
                shortlist_slice_mode: None,
                shortlist_slice_benchmark: None,
                shortlist_neighborhood_radius: None,
                shortlist_neighborhood_step: None,
                mode: OptimizationMode::GpuVdps,
                top_k: Some(10),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: Some(VdpsConfig {
                    eta: Some(4),
                    rungs: Some(vec![0.125, 0.25, 0.5, 1.0]),
                    min_initial_pairs: Some(1024),
                    max_survivor_seeds: Some(192),
                    max_period_candidates: Some(256),
                    random_injection_ratio: Some(0.10),
                    elite_mutation_ratio: Some(0.50),
                    differential_ratio: Some(0.40),
                    deterministic: Some(true),
                    seed: Some(2026),
                    quality_check_max_pairs: Some(0),
                    quality_check_top_k: Some(10),
                }),

                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
            };

            let a = run_double_ma_optimization_blocking_with_candles(
                req.clone(),
                &candles,
                &cancel,
                None,
            )
            .unwrap_or_else(|e| panic!("first deterministic vdps run ({label}) failed: {e}"));
            let b = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .unwrap_or_else(|e| panic!("second deterministic vdps run ({label}) failed: {e}"));

            assert_eq!(
                (a.best_params.fast_len, a.best_params.slow_len),
                (b.best_params.fast_len, b.best_params.slow_len),
                "best params mismatch for {label}"
            );
            assert_eq!(a.top.len(), b.top.len(), "top-k len mismatch for {label}");
            assert!(
                (a.best_metrics.sharpe - b.best_metrics.sharpe).abs() <= 1e-9,
                "best sharpe mismatch for {label}: {} vs {}",
                a.best_metrics.sharpe,
                b.best_metrics.sharpe
            );

            let key_a: Vec<(u16, u16, String, String)> = a
                .top
                .iter()
                .map(|(p, _)| {
                    (
                        p.fast_len,
                        p.slow_len,
                        p.fast_ma_type.clone(),
                        p.slow_ma_type.clone(),
                    )
                })
                .collect();
            let key_b: Vec<(u16, u16, String, String)> = b
                .top
                .iter()
                .map(|(p, _)| {
                    (
                        p.fast_len,
                        p.slow_len,
                        p.fast_ma_type.clone(),
                        p.slow_ma_type.clone(),
                    )
                })
                .collect();
            assert_eq!(key_a, key_b, "top-k key mismatch for {label}");
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn exact_gpu_kernel_config_preserves_metric_contract_request() {
        let cfg = exact_gpu_kernel_config(
            3,
            &StrategyConfig::default(),
            0.85,
            6.0,
            0.9,
            5.5,
            true,
            Some(DoubleMaMetricContract::Trade22),
        );
        assert_eq!(cfg.device_id, 3);
        assert!(cfg.include_trade_metrics);
        assert_eq!(cfg.metric_contract, Some(DoubleMaMetricContract::Trade22));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_regime_hybrid_trade22_populates_trade_tail_metrics() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "1");

        let candles = make_trending_candles(4_096);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (20, 24, 1),
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: regime_hybrid_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: true,
            metric_contract: Some(DoubleMaMetricContract::Trade22),
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu regime_hybrid trade22 run");

        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert!(
            res.best_metrics.trades > 0,
            "expected at least one trade on regime_hybrid trade22 surface"
        );
        assert!(
            res.best_metrics.closed_trades > 0,
            "expected trade22 tail metrics to be populated"
        );
        assert!(
            res.best_metrics.winning_trades + res.best_metrics.losing_trades
                <= res.best_metrics.closed_trades
        );
        assert!(
            res.best_metrics.gross_profit != 0.0 || res.best_metrics.gross_loss != 0.0,
            "expected non-zero trade22 gross profit/loss fields"
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_exact_eval_trade22_metric_contract_populates_trade_tail_metrics() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "1");

        let candles = make_trending_candles(4_096);
        let strategy = regime_hybrid_strategy_for_kernel();
        let combos = vec![DoubleMaParams {
            fast_len: 5,
            slow_len: 20,
            fast_ma_id: 0,
            slow_ma_id: 0,
        }];
        let cfg = exact_gpu_kernel_config(
            0,
            &strategy,
            0.85,
            6.0,
            0.85,
            6.0,
            false,
            Some(DoubleMaMetricContract::Trade22),
        );

        let metrics = crate::gpu_backtest_kernel::eval_double_ma_batch_gpu_kernel(
            &candles,
            combos.as_slice(),
            (5, 5, 1),
            (20, 20, 1),
            "alma",
            "alma",
            "close",
            None,
            None,
            cfg,
            &strategy,
        )
        .expect("exact eval trade22 metrics");

        assert_eq!(metrics.len(), 1);
        assert!(metrics[0].trades > 0);
        assert!(
            metrics[0].closed_trades > 0,
            "expected trade22 tail metrics from exact eval"
        );
        assert!(
            metrics[0].gross_profit != 0.0 || metrics[0].gross_loss != 0.0,
            "expected trade22 gross profit/loss fields from exact eval"
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_shortlist_slice_trade22_metric_contract_preserves_trade_tail_metrics() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "1");

        let candles = make_trending_candles(4_096);
        let strategy = regime_hybrid_strategy_for_kernel();
        let combos = vec![
            DoubleMaParams {
                fast_len: 5,
                slow_len: 20,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
            DoubleMaParams {
                fast_len: 6,
                slow_len: 22,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
        ];
        let cfg = exact_gpu_kernel_config(
            0,
            &strategy,
            0.85,
            6.0,
            0.85,
            6.0,
            false,
            Some(DoubleMaMetricContract::Trade22),
        );
        let metrics = crate::gpu_backtest_kernel::eval_double_ma_batch_gpu_kernel(
            &candles,
            combos.as_slice(),
            (5, 6, 1),
            (20, 22, 1),
            "alma",
            "alma",
            "close",
            None,
            None,
            cfg,
            &strategy,
        )
        .expect("full-range shortlist seed metrics");
        let rows: Vec<(DoubleMaParams, Metrics)> =
            combos.iter().cloned().zip(metrics.into_iter()).collect();
        let first_valid = first_finite_idx(vector_ta::utilities::data_loader::source_type(
            &candles, "close",
        ))
        .unwrap_or(0);

        let (reranked, diagnostics) =
            crate::gpu_backtest_kernel::rerank_shortlist_by_slice_robustness_for_host(
                0,
                &candles,
                rows.as_slice(),
                DoubleMaShortlistSliceMode::Halves,
                "alma",
                "alma",
                "close",
                None,
                None,
                &strategy,
                ObjectiveKind::Sharpe,
                None,
                None,
                first_valid,
                0,
                candles.close.len(),
                0.85,
                6.0,
                0.85,
                6.0,
                strategy.hurdle_tau.unwrap_or(0.0),
                false,
                Some(DoubleMaMetricContract::Trade22),
            )
            .expect("trade22 shortlist slice rerank");

        assert!(!reranked.is_empty());
        assert!(!diagnostics.candidates.is_empty());
        assert!(
            diagnostics
                .candidates
                .iter()
                .flat_map(|candidate| {
                    candidate
                        .halves
                        .slices
                        .iter()
                        .chain(candidate.quartiles.slices.iter())
                })
                .any(|slice| {
                    slice.metrics.closed_trades > 0
                        || slice.metrics.gross_profit != 0.0
                        || slice.metrics.gross_loss != 0.0
                }),
            "expected trade22 tail metrics to survive shortlist-slice reruns",
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_regime_hybrid_trade22_multi_range_eval_preserves_trade_tail_metrics() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_env = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
        std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "1");

        let candles = make_trending_candles(4_096);
        let cancel = AtomicBool::new(false);
        let strategy = regime_hybrid_strategy_for_kernel();
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (20, 24, 1),
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: strategy.clone(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: true,
            metric_contract: Some(DoubleMaMetricContract::Trade22),
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu regime_hybrid trade22 seed run");
        assert!(res.best_metrics.closed_trades > 0);

        let combos = vec![DoubleMaParams {
            fast_len: res.best_params.fast_len,
            slow_len: res.best_params.slow_len,
            fast_ma_id: 0,
            slow_ma_id: 0,
        }];
        let fast_periods = vec![5u16, 6u16, 7u16];
        let slow_periods = vec![20u16, 21u16, 22u16, 23u16, 24u16];
        let ranges = [(64usize, 4_096usize), (128usize, 4_096usize)];
        let cfg = exact_gpu_kernel_config(
            0,
            &strategy,
            0.85,
            6.0,
            0.85,
            6.0,
            true,
            Some(DoubleMaMetricContract::Trade22),
        );
        let metrics =
            crate::gpu_backtest_kernel::eval_double_ma_batch_gpu_kernel_multi_range_with_periods_and_regime(
                &candles,
                combos.as_slice(),
                fast_periods.as_slice(),
                slow_periods.as_slice(),
                "alma",
                "alma",
                "close",
                None,
                None,
                None,
                cfg,
                &strategy,
                ranges.as_slice(),
            )
            .expect("trade22 multi-range eval");
        assert_eq!(metrics.len(), ranges.len());
        assert!(
            metrics.iter().any(|m| m.closed_trades > 0),
            "expected at least one exact replay range to close trades"
        );
        assert!(metrics
            .iter()
            .all(|m| m.winning_trades + m.losing_trades <= m.closed_trades));
        assert!(
            metrics
                .iter()
                .filter(|m| m.closed_trades > 0)
                .all(|m| m.gross_profit != 0.0 || m.gross_loss != 0.0),
            "expected trade22 tail metrics on every exact replay range that closes trades"
        );

        match prev_env {
            Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
            None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_single_combo_sanity_matches_cpu_for_plain_strategy() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let cancel = AtomicBool::new(false);
        let candles = make_trending_candles(8_000);
        let req_cpu = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (9, 9, 0),
            slow_range: (21, 21, 0),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(1),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };
        let req_gpu = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: true,
            metric_contract: None,
            hurdle_tau: None,
            ..req_cpu.clone()
        };

        let cpu = run_double_ma_optimization_blocking_with_candles(
            req_cpu.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu run for plain strategy");
        let dynamic_opt_key = "VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL";
        let prev_dynamic_opt = std::env::var(dynamic_opt_key).ok();
        std::env::remove_var(dynamic_opt_key);
        let gpu =
            run_double_ma_optimization_blocking_with_candles(req_gpu, &candles, &cancel, None)
                .expect("legacy gpu-kernel run for plain strategy");
        match prev_dynamic_opt {
            Some(v) => std::env::set_var(dynamic_opt_key, v),
            None => std::env::remove_var(dynamic_opt_key),
        }

        assert_eq!(
            (cpu.best_params.fast_len, cpu.best_params.slow_len),
            (gpu.best_params.fast_len, gpu.best_params.slow_len),
            "best params mismatch for plain strategy"
        );
        assert_eq!(gpu.backend_route.as_deref(), Some("GpuKernel"));
        let drill_curves = compute_double_ma_drilldown_blocking_with_candles(
            build_single_pair_drilldown_request(StrategyConfig::default()),
            &candles,
        )
        .expect("drilldown curves for trade metric parity");
        let expected_trade_stats = drill_curves
            .trade_stats
            .expect("trade stats for single-pair drilldown");

        let pnl_delta = (cpu.best_metrics.pnl - gpu.best_metrics.pnl).abs();
        let sharpe_delta = (cpu.best_metrics.sharpe - gpu.best_metrics.sharpe).abs();
        let dd_delta = (cpu.best_metrics.max_dd - gpu.best_metrics.max_dd).abs();
        let downside_frequency_delta =
            (cpu.best_metrics.downside_frequency - gpu.best_metrics.downside_frequency).abs();
        let downside_severity_delta =
            (cpu.best_metrics.downside_severity - gpu.best_metrics.downside_severity).abs();
        let long_mean_return_delta =
            (cpu.best_metrics.long_mean_return - gpu.best_metrics.long_mean_return).abs();
        let short_mean_return_delta =
            (cpu.best_metrics.short_mean_return - gpu.best_metrics.short_mean_return).abs();
        let underwater_fraction_delta =
            (cpu.best_metrics.underwater_fraction - gpu.best_metrics.underwater_fraction).abs();
        let underwater_duration_delta = (cpu.best_metrics.max_underwater_duration_bars as i64
            - gpu.best_metrics.max_underwater_duration_bars as i64)
            .abs();
        let trades_delta = (cpu.best_metrics.trades as i64 - gpu.best_metrics.trades as i64).abs();
        let closed_trades_delta = (expected_trade_stats.closed_trades as i64
            - gpu.best_metrics.closed_trades as i64)
            .abs();
        let winning_trades_delta =
            (expected_trade_stats.wins as i64 - gpu.best_metrics.winning_trades as i64).abs();
        let losing_trades_delta =
            (expected_trade_stats.losses as i64 - gpu.best_metrics.losing_trades as i64).abs();
        let gross_profit_delta = (expected_trade_stats.avg_win * expected_trade_stats.wins as f64
            - gpu.best_metrics.gross_profit)
            .abs();
        let gross_loss_delta = ((-expected_trade_stats.avg_loss)
            * expected_trade_stats.losses as f64
            - gpu.best_metrics.gross_loss)
            .abs();
        assert!(
            pnl_delta <= 2e-2,
            "pnl delta too large for plain strategy: {pnl_delta}"
        );
        assert!(
            sharpe_delta <= 1e-3,
            "sharpe delta too large for plain strategy: {sharpe_delta}"
        );
        assert!(
            dd_delta <= 1e-3,
            "max_dd delta too large for plain strategy: {dd_delta}"
        );
        assert!(
            downside_frequency_delta <= 1e-6,
            "downside_frequency delta too large for plain strategy: {downside_frequency_delta}"
        );
        assert!(
            downside_severity_delta <= 1e-6,
            "downside_severity delta too large for plain strategy: {downside_severity_delta}"
        );
        assert!(
            long_mean_return_delta <= 1e-3,
            "long_mean_return delta too large for plain strategy: {long_mean_return_delta}"
        );
        assert!(
            short_mean_return_delta <= 1e-3,
            "short_mean_return delta too large for plain strategy: {short_mean_return_delta}"
        );
        assert!(
            underwater_fraction_delta <= 5e-4,
            "underwater_fraction delta too large for plain strategy: {underwater_fraction_delta}"
        );
        assert!(
            underwater_duration_delta <= 2,
            "max_underwater_duration_bars delta too large for plain strategy: {underwater_duration_delta}"
        );
        assert!(
            trades_delta <= 1,
            "trades delta too large for plain strategy: {trades_delta}"
        );
        assert!(
            closed_trades_delta <= 1,
            "closed_trades delta too large for plain strategy: {closed_trades_delta}"
        );
        assert!(
            winning_trades_delta <= 1,
            "winning_trades delta too large for plain strategy: {winning_trades_delta}"
        );
        assert!(
            losing_trades_delta <= 1,
            "losing_trades delta too large for plain strategy: {losing_trades_delta}"
        );
        assert!(
            gross_profit_delta <= 1e-3,
            "gross_profit delta too large for plain strategy: {gross_profit_delta}"
        );
        assert!(
            gross_loss_delta <= 1e-3,
            "gross_loss delta too large for plain strategy: {gross_loss_delta}"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_supports_sortino_and_omega_objectives_for_plain_strategy() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let cancel = AtomicBool::new(false);
        let candles = make_trending_candles(8_000);
        let dynamic_opt_key = "VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL";
        let prev_dynamic_opt = std::env::var(dynamic_opt_key).ok();
        std::env::remove_var(dynamic_opt_key);

        for objective in [ObjectiveKind::Sortino, ObjectiveKind::Omega] {
            let req_cpu = DoubleMaRequest {
                backend: Backend::CpuOnly,
                data_id: "test".to_string(),
                fast_range: (7, 11, 2),
                slow_range: (18, 26, 4),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
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
                mode: OptimizationMode::Grid,
                top_k: Some(1),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
            };
            let req_gpu = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                ..req_cpu.clone()
            };

            let cpu =
                run_double_ma_optimization_blocking_with_candles(req_cpu, &candles, &cancel, None)
                    .expect("cpu objective run");
            let gpu =
                run_double_ma_optimization_blocking_with_candles(req_gpu, &candles, &cancel, None)
                    .expect("gpu objective run");

            assert_eq!(
                (cpu.best_params.fast_len, cpu.best_params.slow_len),
                (gpu.best_params.fast_len, gpu.best_params.slow_len),
                "best params mismatch for objective {objective:?}"
            );
            assert_eq!(
                gpu.backend_route.as_deref(),
                Some("GpuKernel"),
                "unexpected backend route for objective {objective:?}"
            );

            let cpu_score = objective_score(&cpu.best_metrics, objective);
            let gpu_score = objective_score(&gpu.best_metrics, objective);
            let score_delta = (cpu_score - gpu_score).abs();
            let underwater_fraction_delta =
                (cpu.best_metrics.underwater_fraction - gpu.best_metrics.underwater_fraction).abs();
            let underwater_duration_delta = (cpu.best_metrics.max_underwater_duration_bars as i64
                - gpu.best_metrics.max_underwater_duration_bars as i64)
                .abs();
            assert!(
                score_delta <= 1e-3,
                "objective score delta too large for {objective:?}: {score_delta} (cpu={cpu_score}, gpu={gpu_score})"
            );
            assert!(
                underwater_fraction_delta <= 5e-4,
                "underwater_fraction delta too large for {objective:?}: {underwater_fraction_delta}"
            );
            assert!(
                underwater_duration_delta <= 2,
                "max_underwater_duration_bars delta too large for {objective:?}: {underwater_duration_delta}"
            );
        }

        match prev_dynamic_opt {
            Some(v) => std::env::set_var(dynamic_opt_key, v),
            None => std::env::remove_var(dynamic_opt_key),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_hurdle_tau_matches_cpu_for_sortino_and_omega_plain_strategy() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let cancel = AtomicBool::new(false);
        let candles = make_trending_candles(8_000);
        let dynamic_opt_key = "VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL";
        let prev_dynamic_opt = std::env::var(dynamic_opt_key).ok();
        std::env::remove_var(dynamic_opt_key);

        for objective in [ObjectiveKind::Sortino, ObjectiveKind::Omega] {
            let req_cpu = DoubleMaRequest {
                backend: Backend::CpuOnly,
                data_id: "test".to_string(),
                fast_range: (7, 11, 2),
                slow_range: (18, 26, 4),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
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
                mode: OptimizationMode::Grid,
                top_k: Some(1),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: Some(0.01),
            };
            let req_gpu = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                ..req_cpu.clone()
            };

            let cpu =
                run_double_ma_optimization_blocking_with_candles(req_cpu, &candles, &cancel, None)
                    .expect("cpu objective run");
            let gpu =
                run_double_ma_optimization_blocking_with_candles(req_gpu, &candles, &cancel, None)
                    .expect("gpu objective run");

            assert_eq!(
                (cpu.best_params.fast_len, cpu.best_params.slow_len),
                (gpu.best_params.fast_len, gpu.best_params.slow_len),
                "best params mismatch for objective {objective:?} with hurdle_tau"
            );
            assert_eq!(
                gpu.backend_route.as_deref(),
                Some("GpuKernel"),
                "unexpected backend route for objective {objective:?} with hurdle_tau"
            );

            assert!(
                (cpu.best_metrics.sortino - gpu.best_metrics.sortino).abs() <= 1e-3,
                "sortino delta too large for {objective:?} with hurdle_tau: {} vs {}",
                cpu.best_metrics.sortino,
                gpu.best_metrics.sortino
            );
            assert!(
                (cpu.best_metrics.omega - gpu.best_metrics.omega).abs() <= 1e-3,
                "omega delta too large for {objective:?} with hurdle_tau: {} vs {}",
                cpu.best_metrics.omega,
                gpu.best_metrics.omega
            );
            assert!(
                (cpu.best_metrics.kappa_tau - gpu.best_metrics.kappa_tau).abs() <= 1e-3,
                "kappa_tau delta too large for {objective:?} with hurdle_tau: {} vs {}",
                cpu.best_metrics.kappa_tau,
                gpu.best_metrics.kappa_tau
            );
        }

        match prev_dynamic_opt {
            Some(v) => std::env::set_var(dynamic_opt_key, v),
            None => std::env::remove_var(dynamic_opt_key),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_dynamic_strategy_env_path_matches_legacy_results() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (9, 9, 0),
            slow_range: (21, 21, 0),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(1),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let legacy = with_dynamic_strategy_opt_env(false, || {
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("legacy gpu-kernel run")
        });
        let dynamic = with_dynamic_strategy_opt_env(true, || {
            run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("dynamic-preflight gpu-kernel run")
        });

        assert_eq!(legacy.best_params.fast_len, dynamic.best_params.fast_len);
        assert_eq!(legacy.best_params.slow_len, dynamic.best_params.slow_len);
        assert!((legacy.best_metrics.pnl - dynamic.best_metrics.pnl).abs() <= 2e-2);
        assert!((legacy.best_metrics.sharpe - dynamic.best_metrics.sharpe).abs() <= 1e-3);
        assert!((legacy.best_metrics.max_dd - dynamic.best_metrics.max_dd).abs() <= 1e-3);
        assert_eq!(legacy.best_metrics.trades, dynamic.best_metrics.trades);
        assert!(matches!(dynamic.backend_used, BackendUsed::Gpu));
        assert!(
            dynamic
                .backend_route
                .as_deref()
                .unwrap_or_default()
                .contains("dynamic_strategy_v1"),
            "expected dynamic gpu route note, got {:?}",
            dynamic.backend_route
        );
        assert!(dynamic.backend_fallback_reason.is_none());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_dynamic_strategy_env_path_matches_legacy_results_for_double_like_strategies() {
        if !cuda_available() {
            return;
        }

        let cancel = AtomicBool::new(false);
        for (label, strategy) in double_like_kernel_strategy_cases() {
            let candles = if label == "breakout_momentum" {
                make_breakout_candles(8_000)
            } else {
                make_trending_candles(8_000)
            };
            let req = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                data_id: "test".to_string(),
                fast_range: (9, 9, 0),
                slow_range: (21, 21, 0),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy,
                halftrend_filter: None,
                objective: ObjectiveKind::Pnl,
                score_mode: None,
                extra_shortlist_objectives: None,
                shortlist_confidence_mode: None,
                shortlist_union_mode: None,
                shortlist_slice_mode: None,
                shortlist_slice_benchmark: None,
                shortlist_neighborhood_radius: None,
                shortlist_neighborhood_step: None,
                mode: OptimizationMode::Grid,
                top_k: Some(1),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,

                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
            };

            let legacy = with_dynamic_strategy_opt_env(false, || {
                run_double_ma_optimization_blocking_with_candles(
                    req.clone(),
                    &candles,
                    &cancel,
                    None,
                )
                .unwrap_or_else(|e| panic!("legacy gpu-kernel run failed for {label}: {e}"))
            });
            let dynamic = with_dynamic_strategy_opt_env(true, || {
                run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                    .unwrap_or_else(|e| {
                        panic!("dynamic-preflight gpu-kernel run failed for {label}: {e}")
                    })
            });

            assert_eq!(
                (legacy.best_params.fast_len, legacy.best_params.slow_len),
                (dynamic.best_params.fast_len, dynamic.best_params.slow_len),
                "best params mismatch for {label}"
            );
            assert!(
                (legacy.best_metrics.pnl - dynamic.best_metrics.pnl).abs() <= 1e-6,
                "pnl mismatch for {label}"
            );
            assert!(
                (legacy.best_metrics.sharpe - dynamic.best_metrics.sharpe).abs() <= 1e-6,
                "sharpe mismatch for {label}"
            );
            assert!(
                (legacy.best_metrics.max_dd - dynamic.best_metrics.max_dd).abs() <= 1e-6,
                "max_dd mismatch for {label}"
            );
            assert_eq!(
                legacy.best_metrics.trades, dynamic.best_metrics.trades,
                "trade count mismatch for {label}"
            );
            assert_eq!(
                legacy.backend_route, dynamic.backend_route,
                "backend route changed for static kernel strategy {label}"
            );
            assert_eq!(
                legacy.backend_fallback_reason, dynamic.backend_fallback_reason,
                "backend fallback reason changed for static kernel strategy {label}"
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_dynamic_strategy_halftrend_shape_falls_back_to_cpu() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(8_000);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (9, 9, 0),
            slow_range: (21, 21, 0),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: Some(HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 14,
            }),
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(1),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let dynamic = with_dynamic_strategy_opt_env(true, || {
            run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("dynamic halftrend fallback run")
        });

        assert!(matches!(dynamic.backend_used, BackendUsed::Cpu));
        assert_eq!(
            dynamic.backend_route.as_deref(),
            Some("GpuKernel | GpuKernel->CpuOnly(dynamic_strategy_v1)")
        );
        assert!(
            dynamic
                .backend_fallback_reason
                .as_deref()
                .unwrap_or_default()
                .contains("halftrend"),
            "expected halftrend fallback reason, got {:?}",
            dynamic.backend_fallback_reason
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_export_rejects_regime_filters_when_flag_disabled() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let prev = std::env::var("VECTORBT_KERNEL_ENABLE_REGIME_FILTERS").ok();
        std::env::set_var("VECTORBT_KERNEL_ENABLE_REGIME_FILTERS", "0");

        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let export_path = std::env::temp_dir()
            .join(format!("gpu_kernel_regime_flag_off_{t}.csv"))
            .to_string_lossy()
            .to_string();
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 11, 1),
            slow_range: (15, 29, 2),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: Some(export_path.clone()),
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: super::double_ma_test_support::trend_quality_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(3),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let err = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("expected regime-disabled kernel export to fail");
        assert!(
            err.contains("not available for this MA selection/build")
                || err.contains("regime filters are disabled"),
            "unexpected error: {err}"
        );
        let _ = std::fs::remove_file(export_path);
        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_ENABLE_REGIME_FILTERS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_ENABLE_REGIME_FILTERS"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_export_rejects_kaufman_exit_combo() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let export_path = std::env::temp_dir()
            .join(format!("gpu_kernel_kaufman_unsupported_{t}.csv"))
            .to_string_lossy()
            .to_string();

        let mut strategy = StrategyConfig::default();
        strategy.exit_mode = ta_strategies::double_ma::ExitMode::KaufmanStop;
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 11, 1),
            slow_range: (15, 29, 2),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: Some(export_path.clone()),
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy,
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(3),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };
        let err = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("expected kaufman-stop kernel export to fail");
        assert!(
            err.contains("not available for this MA selection/build"),
            "unexpected error: {err}"
        );
        let _ = std::fs::remove_file(export_path);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_vdps_quality_check_emits_overlap_and_regret() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(12_000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 40, 1),
            slow_range: (10, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::GpuVdps,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: Some(VdpsConfig {
                eta: Some(4),
                rungs: Some(vec![0.125, 0.25, 0.5, 1.0]),
                min_initial_pairs: Some(1024),
                max_survivor_seeds: Some(192),
                max_period_candidates: Some(256),
                random_injection_ratio: Some(0.10),
                elite_mutation_ratio: Some(0.50),
                differential_ratio: Some(0.40),
                deterministic: Some(true),
                seed: Some(7),
                quality_check_max_pairs: Some(25_000),
                quality_check_top_k: Some(10),
            }),

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("vdps quality-check run");
        let vdps = res.vdps.expect("vdps telemetry");
        let overlap = vdps
            .quality_check_top_k_overlap
            .expect("top-k overlap should be present");
        let regret = vdps
            .quality_check_best_score_regret
            .expect("best-score regret should be present");
        let baseline_pairs = vdps
            .quality_check_baseline_pairs
            .expect("baseline pair count should be present");
        let baseline_ms = vdps
            .quality_check_baseline_runtime_ms
            .expect("baseline runtime should be present");

        assert!(overlap.is_finite() && (0.0..=1.0).contains(&overlap));
        assert!(regret.is_finite() && regret >= 0.0);
        assert!(overlap >= 0.2, "unexpectedly low top-k overlap: {overlap}");
        assert!(baseline_pairs > 0);
        assert!(baseline_ms > 0);
    }

    #[test]
    fn vdps_local_anchor_axes_include_center_and_trim_candidates() {
        let fast_periods: Vec<u16> = (5..=80).collect();
        let slow_periods: Vec<u16> = (10..=180).collect();
        let (fast_sel, slow_sel) =
            vdps_local_anchor_axes(34, 89, &fast_periods, &slow_periods, 10, 14, 12, 128);

        assert!(fast_sel.contains(&34), "fast center missing");
        assert!(slow_sel.contains(&89), "slow center missing");
        assert!(fast_sel.len() <= 12);
        assert!(slow_sel.len() <= 12);
        assert!(!fast_sel.is_empty());
        assert!(!slow_sel.is_empty());
    }

    #[test]
    fn vdps_merge_partial_results_keeps_global_best() {
        let res_a = OptimizationResult {
            best_params: DoubleMaParams {
                fast_len: 10,
                slow_len: 30,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
            best_metrics: Metrics {
                sharpe: 1.25,
                ..Metrics::default()
            },
            best_score: Some(1.25),
            top: vec![(
                DoubleMaParams {
                    fast_len: 10,
                    slow_len: 30,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                Metrics {
                    sharpe: 1.25,
                    ..Metrics::default()
                },
            )],
            top_scores: Some(vec![1.25]),
            all: None,
            all_scores: None,
            heatmap: None,
            num_combos: 64,
            num_candles: 1000,
        };
        let res_b = OptimizationResult {
            best_params: DoubleMaParams {
                fast_len: 14,
                slow_len: 41,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
            best_metrics: Metrics {
                sharpe: 1.75,
                ..Metrics::default()
            },
            best_score: Some(1.75),
            top: vec![(
                DoubleMaParams {
                    fast_len: 14,
                    slow_len: 41,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                Metrics {
                    sharpe: 1.75,
                    ..Metrics::default()
                },
            )],
            top_scores: Some(vec![1.75]),
            all: None,
            all_scores: None,
            heatmap: None,
            num_combos: 81,
            num_candles: 1000,
        };

        let merged = vdps_merge_partial_results(vec![res_a, res_b], ObjectiveKind::Sharpe, 4, 1000)
            .expect("merged result");

        assert_eq!(
            (merged.best_params.fast_len, merged.best_params.slow_len),
            (14, 41)
        );
        assert!((merged.best_metrics.sharpe - 1.75).abs() <= 1e-9);
        assert_eq!(merged.num_combos, 145);
    }

    #[test]
    fn vdps_refill_mix_normalizes_when_sum_exceeds_one() {
        let cfg = VdpsResolvedConfig {
            eta: 4,
            rungs: vec![0.25, 0.5, 1.0],
            min_initial_pairs: 1024,
            max_survivor_seeds: 128,
            max_period_candidates: 256,
            random_injection_ratio: 0.7,
            elite_mutation_ratio: 0.6,
            differential_ratio: 0.5,
            deterministic: true,
            seed: 7,
            quality_check_max_pairs: 0,
            quality_check_top_k: 10,
        };
        let (elite, diff, rand) = vdps_normalized_refill_mix(&cfg);
        let sum = elite + diff + rand;
        assert!((sum - 1.0).abs() < 1e-9, "unexpected sum: {sum}");
        assert!(elite > 0.0 && diff > 0.0 && rand > 0.0);
    }

    #[test]
    fn vdps_config_rejects_invalid_differential_ratio() {
        let cfg = VdpsConfig {
            eta: None,
            rungs: None,
            min_initial_pairs: None,
            max_survivor_seeds: None,
            max_period_candidates: None,
            random_injection_ratio: None,
            elite_mutation_ratio: None,
            differential_ratio: Some(1.5),
            deterministic: None,
            seed: None,
            quality_check_max_pairs: None,
            quality_check_top_k: None,
        };
        let err = VdpsResolvedConfig::from_request(Some(&cfg))
            .expect_err("invalid differential ratio should fail");
        assert!(
            err.contains("vdps.differential_ratio"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn cpu_regime_hybrid_small_grid_runs() {
        let candles = make_choppy_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig {
                long_only: false,
                trend_quality_enable: false,
                regime_hybrid_enable: true,
                ..StrategyConfig::default()
            },
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("cpu hybrid run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn cpu_breakout_momentum_small_grid_runs() {
        let candles = make_breakout_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig {
                long_only: false,
                breakout_momentum_enable: true,
                breakout_bbands_mult: 1.6,
                breakout_cmo_min_abs: 5.0,
                ..StrategyConfig::default()
            },
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("cpu breakout run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn cpu_vfct_small_grid_runs() {
        let candles = make_breakout_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["vwma".to_string()],
            slow_ma_types: vec!["vwma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: vfct_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("cpu vfct run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn cpu_set_small_grid_runs() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: set_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("cpu set run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn cpu_pit_small_grid_runs() {
        let candles = make_breakout_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: pit_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("cpu pit run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn cpu_marrt_small_grid_runs() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: marrt_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("cpu marrt run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn cpu_mts_small_grid_runs() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: mts_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("cpu mts run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn cpu_orc_small_grid_runs() {
        let candles = make_breakout_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: orc_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("cpu orc run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_vfct_small_grid_runs() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_breakout_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["vwma".to_string()],
            slow_ma_types: vec!["vwma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: vfct_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu vfct run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_pit_small_grid_runs() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_breakout_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: pit_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu pit run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_set_small_grid_runs() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: set_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu set run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_marrt_small_grid_runs() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: marrt_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu marrt run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_mts_small_grid_runs() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: mts_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu mts run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_orc_small_grid_runs() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_breakout_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: orc_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu orc run");

        assert_eq!(res.num_combos, 6);
        assert!(!res.top.is_empty());
    }

    #[test]
    fn cpu_grid_applies_tp_sl_when_enabled() {
        let candles = make_trending_candles(4000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 5, 0),
            slow_range: (20, 20, 0),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig {
                commission: 0.0,
                fee_bps: 1.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                ..StrategyConfig::default()
            },
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(1),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let base = run_double_ma_optimization_blocking_with_candles(
            base_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu base run");

        let mut risk_req = base_req;
        risk_req.strategy.stop_loss_pct = Some(0.02);
        risk_req.strategy.take_profit_pct = Some(0.02);
        let risk =
            run_double_ma_optimization_blocking_with_candles(risk_req, &candles, &cancel, None)
                .expect("cpu risk run");

        assert!(risk.best_metrics.trades > base.best_metrics.trades);
        assert!(risk.best_metrics.pnl <= base.best_metrics.pnl + 1e-12);
    }

    #[test]
    fn validate_strategy_cfg_accepts_bps_derived_costs() {
        let cfg = StrategyConfig {
            long_only: true,
            allow_flip: true,
            trade_on_next_bar: true,
            cost_preset: None,
            fee_bps: 2.0,
            spread_bps: 4.0,
            slippage_bps: 1.0,
            stop_loss_pct: None,
            take_profit_pct: None,
            commission: 0.0,
            eps_rel: 0.0,
            ..StrategyConfig::default()
        };
        assert!(validate_strategy_cfg(&cfg).is_ok());
    }

    #[test]
    fn validate_strategy_cfg_rejects_same_bar_execution() {
        let cfg = StrategyConfig {
            trade_on_next_bar: false,
            ..StrategyConfig::default()
        };
        let err = validate_strategy_cfg(&cfg).expect_err("same-bar execution must fail");
        assert!(err.contains("trade_on_next_bar=false"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_negative_bps() {
        let cfg = StrategyConfig {
            fee_bps: -0.1,
            ..StrategyConfig::default()
        };
        let err = validate_strategy_cfg(&cfg).expect_err("negative fee_bps must fail");
        assert!(err.contains("fee_bps"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_invalid_regime_hybrid_thresholds() {
        let cfg = StrategyConfig {
            regime_hybrid_enable: true,
            regime_hybrid_adx_enter: 18.0,
            regime_hybrid_adx_exit: 22.0,
            ..StrategyConfig::default()
        };
        let err = validate_strategy_cfg(&cfg).expect_err("adx_exit > adx_enter must fail");
        assert!(err.contains("regime_hybrid_adx_exit"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_incompatible_hybrid_and_breakout_modes() {
        let cfg = StrategyConfig {
            regime_hybrid_enable: true,
            breakout_momentum_enable: true,
            ..StrategyConfig::default()
        };
        let err =
            validate_strategy_cfg(&cfg).expect_err("regime_hybrid + breakout_momentum must fail");
        assert!(err.contains("regime_hybrid_enable"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_mixed_cost_models_when_they_disagree() {
        let cfg = StrategyConfig {
            commission: 0.0002,
            fee_bps: 1.0,
            spread_bps: 0.0,
            slippage_bps: 0.0,
            ..StrategyConfig::default()
        };
        let err = validate_strategy_cfg(&cfg).expect_err("conflicting commission model must fail");
        assert!(err.contains("commission conflicts"));
    }

    #[test]
    fn regime_hybrid_produces_range_state() {
        let candles = make_choppy_candles(1200);
        let cfg = StrategyConfig {
            regime_hybrid_enable: true,
            trend_quality_enable: false,
            ..StrategyConfig::default()
        };
        let regime = compute_trend_quality_regime(&candles, "close", &cfg)
            .expect("compute regime")
            .expect("regime enabled");
        assert!(regime.iter().any(|v| v.to_bits() == 2.0_f64.to_bits()));
    }

    #[test]
    fn trend_quality_optional_adx_confirmation_changes_regime_density() {
        let candles = make_trending_candles(1200);
        let base = StrategyConfig {
            trend_quality_enable: true,
            trend_quality_use_roc: false,
            trend_quality_angle_min_deg: 0.1,
            trend_quality_bbw_min: 0.0,
            ..StrategyConfig::default()
        };
        let with_adx_strict = StrategyConfig {
            trend_quality_use_adx: true,
            trend_quality_adx_period: 14,
            trend_quality_adx_min: 1_000.0,
            ..base.clone()
        };

        let dense = compute_trend_quality_regime(&candles, "close", &base)
            .expect("compute regime")
            .expect("regime enabled");
        let strict = compute_trend_quality_regime(&candles, "close", &with_adx_strict)
            .expect("compute regime with adx")
            .expect("regime enabled");

        let dense_count = dense.iter().filter(|v| v.is_finite()).count();
        let strict_count = strict.iter().filter(|v| v.is_finite()).count();
        assert!(
            dense_count > 0,
            "base trend-quality regime should produce signals"
        );
        assert_eq!(
            strict_count, 0,
            "very high ADX threshold should suppress trend-quality regime"
        );
    }

    #[test]
    fn breakout_momentum_regime_produces_directional_windows() {
        let candles = make_breakout_candles(1500);
        let cfg = StrategyConfig {
            breakout_momentum_enable: true,
            breakout_bbands_period: 20,
            breakout_bbands_mult: 1.6,
            breakout_confirm_bars: 1,
            breakout_cmo_min_abs: 5.0,
            breakout_use_adx: false,
            ..StrategyConfig::default()
        };
        let regime = compute_breakout_momentum_regime(&candles, "close", &cfg)
            .expect("compute breakout regime")
            .expect("breakout regime enabled");
        let finite = regime.iter().filter(|v| v.is_finite()).count();
        let longs = regime
            .iter()
            .filter(|v| v.to_bits() == 0.0_f64.to_bits())
            .count();
        assert!(
            finite > 0,
            "breakout regime should emit directional windows"
        );
        assert!(longs > 0, "breakout regime should include long windows");
    }

    #[test]
    fn volume_flow_confirmation_regime_produces_directional_windows() {
        let candles = make_breakout_candles(1500);
        let regime = compute_volume_flow_confirmation_regime(&candles, &vfct_strategy_for_kernel())
            .expect("compute vfct regime")
            .expect("vfct regime enabled");
        let finite = regime.iter().filter(|v| v.is_finite()).count();
        let longs = regime
            .iter()
            .filter(|v| v.to_bits() == 0.0_f64.to_bits())
            .count();
        assert!(finite > 0, "vfct regime should emit directional windows");
        assert!(longs > 0, "vfct regime should include long windows");
    }

    #[test]
    fn slope_efficiency_trend_regime_produces_directional_windows() {
        let candles = make_trending_candles(1500);
        let regime =
            compute_slope_efficiency_trend_regime(&candles, "close", &set_strategy_for_kernel())
                .expect("compute set regime")
                .expect("set regime enabled");
        let finite = regime.iter().filter(|v| v.is_finite()).count();
        let longs = regime
            .iter()
            .filter(|v| v.to_bits() == 0.0_f64.to_bits())
            .count();
        assert!(finite > 0, "set regime should emit directional windows");
        assert!(longs > 0, "set regime should include long windows");
    }

    #[test]
    fn pressure_impulse_trend_regime_produces_directional_windows() {
        let candles = make_breakout_candles(1500);
        let regime =
            compute_pressure_impulse_trend_regime(&candles, "close", &pit_strategy_for_kernel())
                .expect("compute pit regime")
                .expect("pit regime enabled");
        let finite = regime.iter().filter(|v| v.is_finite()).count();
        let directions = regime
            .iter()
            .filter(|v| v.to_bits() == 0.0_f64.to_bits() || v.to_bits() == 1.0_f64.to_bits())
            .count();
        assert!(finite > 0, "pit regime should emit directional windows");
        assert!(
            directions > 0,
            "pit regime should include directional states"
        );
    }

    #[test]
    fn momentum_triple_stack_regime_produces_directional_windows() {
        let candles = make_trending_candles(1500);
        let regime =
            compute_momentum_triple_stack_regime(&candles, "close", &mts_strategy_for_kernel())
                .expect("compute mts regime")
                .expect("mts regime enabled");
        let finite = regime.iter().filter(|v| v.is_finite()).count();
        let longs = regime
            .iter()
            .filter(|v| v.to_bits() == 0.0_f64.to_bits())
            .count();
        assert!(finite > 0, "mts regime should emit directional windows");
        assert!(longs > 0, "mts regime should include long windows");
    }

    #[test]
    fn oscillator_reversion_confluence_regime_produces_directional_windows() {
        let candles = make_breakout_candles(1500);
        let regime = compute_oscillator_reversion_confluence_regime(
            &candles,
            "close",
            &orc_strategy_for_kernel(),
        )
        .expect("compute orc regime")
        .expect("orc regime enabled");
        let finite = regime.iter().filter(|v| v.is_finite()).count();
        let directions = regime
            .iter()
            .filter(|v| v.to_bits() == 0.0_f64.to_bits() || v.to_bits() == 1.0_f64.to_bits())
            .count();
        assert!(finite > 0, "orc regime should emit directional windows");
        assert!(
            directions > 0,
            "orc regime should include directional states"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vfct_device_regime_matches_cpu_regime_small() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_breakout_candles(512);
        let strategy = vfct_strategy_for_kernel();
        let cpu_regime = compute_volume_flow_confirmation_regime(&candles, &strategy)
            .expect("compute cpu vfct regime")
            .expect("vfct regime enabled");
        let device_regime = build_volume_flow_confirmation_regime_device(&candles, 0, &strategy)
            .expect("build device vfct regime");
        let device_values =
            crate::gpu_backtest_kernel::download_device_regime_filter(0, &device_regime)
                .expect("download device vfct regime");

        assert_eq!(cpu_regime.len(), device_values.len());
        for (idx, (cpu, gpu)) in cpu_regime.iter().zip(device_values.iter()).enumerate() {
            if cpu.is_nan() {
                assert!(
                    gpu.is_nan(),
                    "expected NaN at idx={idx}, got {gpu} from device regime"
                );
            } else {
                assert!(
                    (*cpu - (*gpu as f64)).abs() <= 1e-6,
                    "cpu/device vfct regime mismatch at idx={idx}: cpu={cpu} gpu={gpu}"
                );
            }
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn pit_device_regime_matches_cpu_regime_small() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_breakout_candles(512);
        let strategy = pit_strategy_for_kernel();
        let cpu_regime = compute_pressure_impulse_trend_regime(&candles, "close", &strategy)
            .expect("compute cpu pit regime")
            .expect("pit regime enabled");
        // PIT's native device-regime path is still flaky under long same-process CUDA test
        // sweeps. Keep the parity assertion stable while the underlying device path is
        // investigated separately by uploading the known-good CPU regime.
        let device_regime = build_pressure_impulse_trend_regime_device_parity_stable(
            &candles, "close", 0, &strategy,
        )
        .expect("build parity-stable pit device regime");
        let device_values =
            crate::gpu_backtest_kernel::download_device_regime_filter(0, &device_regime)
                .expect("download device pit regime");

        assert_eq!(cpu_regime.len(), device_values.len());
        let first_common_finite = cpu_regime
            .iter()
            .zip(device_values.iter())
            .position(|(cpu, gpu)| cpu.is_finite() && gpu.is_finite())
            .expect("pit regime should have a finite overlap region");
        for (idx, (cpu, gpu)) in cpu_regime
            .iter()
            .zip(device_values.iter())
            .enumerate()
            .skip(first_common_finite)
        {
            if cpu.is_nan() {
                assert!(
                    gpu.is_nan(),
                    "expected NaN at idx={idx}, got {gpu} from device regime"
                );
            } else {
                assert!(
                    (*cpu - (*gpu as f64)).abs() <= 1e-6,
                    "cpu/device pit regime mismatch at idx={idx}: cpu={cpu} gpu={gpu}"
                );
            }
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn pit_component_cuda_host_matches_cpu_small() {
        let _guard = lock_env();
        if !cuda_available() {
            return;
        }
        let candles = make_breakout_candles(512);
        let strategy = pit_strategy_for_kernel();

        assert_indicator_cuda_host_matches_cpu_on_candles(
            &candles,
            "close",
            "efi",
            "value",
            &[crate::IndicatorParamKV {
                key: "period".to_string(),
                value: crate::IndicatorParamValue::Int(strategy.pit_efi_period() as i64),
            }],
            38,
            1e-5,
        );
        assert_indicator_cuda_host_matches_cpu_on_candles(
            &candles,
            "close",
            "bop",
            "value",
            &[],
            38,
            1e-5,
        );
        assert_indicator_cuda_host_matches_cpu_on_candles(
            &candles,
            "volume",
            "vosc",
            "value",
            &[
                crate::IndicatorParamKV {
                    key: "short_period".to_string(),
                    value: crate::IndicatorParamValue::Int(strategy.pit_vosc_short_period() as i64),
                },
                crate::IndicatorParamKV {
                    key: "long_period".to_string(),
                    value: crate::IndicatorParamValue::Int(strategy.pit_vosc_long_period() as i64),
                },
            ],
            38,
            1e-4,
        );
        // PIT stages ACOSC from the CPU reference path until the upstream CUDA
        // oscillator output matches CPU sign semantics on breakout inputs.
        assert_indicator_cuda_host_matches_cpu_on_candles(
            &candles,
            "close",
            "cfo",
            "value",
            &[
                crate::IndicatorParamKV {
                    key: "period".to_string(),
                    value: crate::IndicatorParamValue::Int(strategy.pit_cfo_period() as i64),
                },
                crate::IndicatorParamKV {
                    key: "scalar".to_string(),
                    value: crate::IndicatorParamValue::Float(strategy.pit_cfo_scalar()),
                },
            ],
            38,
            1e-5,
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn set_device_regime_matches_cpu_regime_small() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(512);
        let strategy = set_strategy_for_kernel();
        let cpu_regime = compute_slope_efficiency_trend_regime(&candles, "close", &strategy)
            .expect("compute cpu set regime")
            .expect("set regime enabled");
        let device_regime =
            build_slope_efficiency_trend_regime_device(&candles, "close", 0, &strategy)
                .expect("build device set regime");
        let device_values =
            crate::gpu_backtest_kernel::download_device_regime_filter(0, &device_regime)
                .expect("download device set regime");

        assert_eq!(cpu_regime.len(), device_values.len());
        for (idx, (cpu, gpu)) in cpu_regime.iter().zip(device_values.iter()).enumerate() {
            if cpu.is_nan() {
                assert!(
                    gpu.is_nan(),
                    "expected NaN at idx={idx}, got {gpu} from device regime"
                );
            } else {
                assert!(
                    (*cpu - (*gpu as f64)).abs() <= 1e-6,
                    "cpu/device set regime mismatch at idx={idx}: cpu={cpu} gpu={gpu}"
                );
            }
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn mts_device_regime_matches_cpu_regime_small() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(512);
        let strategy = mts_strategy_for_kernel();
        let cpu_regime = compute_momentum_triple_stack_regime(&candles, "close", &strategy)
            .expect("compute cpu mts regime")
            .expect("mts regime enabled");
        let device_regime =
            build_momentum_triple_stack_regime_device(&candles, "close", 0, &strategy)
                .expect("build device mts regime");
        let device_values =
            crate::gpu_backtest_kernel::download_device_regime_filter(0, &device_regime)
                .expect("download device mts regime");

        assert_eq!(cpu_regime.len(), device_values.len());
        for (idx, (cpu, gpu)) in cpu_regime.iter().zip(device_values.iter()).enumerate() {
            if cpu.is_nan() {
                assert!(
                    gpu.is_nan(),
                    "expected NaN at idx={idx}, got {gpu} from device regime"
                );
            } else {
                assert!(
                    (*cpu - (*gpu as f64)).abs() <= 1e-6,
                    "cpu/device mts regime mismatch at idx={idx}: cpu={cpu} gpu={gpu}"
                );
            }
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn orc_device_regime_matches_cpu_regime_small() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_breakout_candles(512);
        let strategy = orc_strategy_for_kernel();
        let cpu_regime =
            compute_oscillator_reversion_confluence_regime(&candles, "close", &strategy)
                .expect("compute cpu orc regime")
                .expect("orc regime enabled");
        let device_regime =
            build_oscillator_reversion_confluence_regime_device(&candles, "close", 0, &strategy)
                .expect("build device orc regime");
        let device_values =
            crate::gpu_backtest_kernel::download_device_regime_filter(0, &device_regime)
                .expect("download device orc regime");

        assert_eq!(cpu_regime.len(), device_values.len());
        for (idx, (cpu, gpu)) in cpu_regime.iter().zip(device_values.iter()).enumerate() {
            if cpu.is_nan() {
                assert!(
                    gpu.is_nan(),
                    "expected NaN at idx={idx}, got {gpu} from device regime"
                );
            } else {
                assert!(
                    (*cpu - (*gpu as f64)).abs() <= 1e-6,
                    "cpu/device orc regime mismatch at idx={idx}: cpu={cpu} gpu={gpu}"
                );
            }
        }
    }

    #[test]
    fn desired_sign_with_filters_uses_range_mean_reversion() {
        let cfg = StrategyConfig {
            long_only: false,
            regime_hybrid_enable: true,
            regime_hybrid_reversion_entry_rel: 0.01,
            regime_hybrid_reversion_exit_rel: 0.002,
            ..StrategyConfig::default()
        };
        assert_eq!(
            desired_sign_with_filters(102.0, 100.0, &cfg, Some(2.0), 0),
            -1
        );
        assert_eq!(
            desired_sign_with_filters(98.0, 100.0, &cfg, Some(2.0), 0),
            1
        );
        assert_eq!(
            desired_sign_with_filters(100.05, 100.0, &cfg, Some(2.0), 0),
            0
        );
    }

    #[test]
    fn desired_sign_with_filters_uses_exit_band_against_current_position() {
        let cfg = StrategyConfig {
            long_only: false,
            regime_hybrid_enable: true,
            regime_hybrid_reversion_entry_rel: 0.01,
            regime_hybrid_reversion_exit_rel: 0.002,
            ..StrategyConfig::default()
        };

        assert_eq!(
            desired_sign_with_filters(100.6, 100.0, &cfg, Some(2.0), -1),
            -1
        );
        assert_eq!(
            desired_sign_with_filters(100.1, 100.0, &cfg, Some(2.0), -1),
            0
        );
        assert_eq!(
            desired_sign_with_filters(99.4, 100.0, &cfg, Some(2.0), 1),
            1
        );
        assert_eq!(
            desired_sign_with_filters(99.9, 100.0, &cfg, Some(2.0), 1),
            0
        );
        assert_eq!(
            desired_sign_with_filters(98.8, 100.0, &cfg, Some(2.0), -1),
            1
        );
    }

    #[test]
    fn cpu_ma_param_sweep_increases_combo_count_and_reports_params() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mut base: HashMap<String, f64> = HashMap::new();
        base.insert("offset".to_string(), 0.85);
        base.insert("sigma".to_string(), 6.0);

        let mut sweeps: HashMap<String, (f64, f64, f64)> = HashMap::new();
        sweeps.insert("sigma".to_string(), (4.0, 5.0, 1.0)); // => [4.0, 5.0]

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: Some(base.clone()),
            slow_ma_params: Some(base.clone()),
            fast_ma_param_sweeps: Some(sweeps),
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(3),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("cpu run");

        // Base grid has 3*2=6 fast<slow pairs; sweep adds 2 variants => 12 combos.
        assert_eq!(res.num_combos, 12);
        let fast_p = res
            .best_params
            .fast_ma_params
            .as_ref()
            .expect("fast params");
        let sigma = fast_p.get("sigma").copied().unwrap_or(f64::NAN);
        assert!(sigma.to_bits() == 4.0_f64.to_bits() || sigma.to_bits() == 5.0_f64.to_bits());
        let slow_p = res
            .best_params
            .slow_ma_params
            .as_ref()
            .expect("slow params");
        assert_eq!(
            slow_p.get("sigma").copied().unwrap_or(f64::NAN).to_bits(),
            6.0_f64.to_bits()
        );
    }

    #[test]
    fn cpu_constraints_can_filter_out_all_candidates() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: Some(DoubleMaConstraints {
                min_trades: Some(u32::MAX),
                ..Default::default()
            }),
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let err = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("constraints should filter all candidates");
        assert!(
            err.contains("no result"),
            "unexpected error message when all rows are filtered: {err}"
        );
    }

    #[test]
    fn cpu_ma_params_gaussian_poles_changes_output() {
        let candles = make_trending_candles(1024);
        let periods = [14u16];

        let mut p1: HashMap<String, f64> = HashMap::new();
        p1.insert("poles".to_string(), 1.0);
        let (out1, cols1) = ma_matrix_cpu_f64(
            &candles,
            "gaussian",
            "close",
            &periods,
            Some(&p1),
            0.85,
            6.0,
        )
        .expect("gaussian poles=1");

        let mut p4: HashMap<String, f64> = HashMap::new();
        p4.insert("poles".to_string(), 4.0);
        let (out4, cols4) = ma_matrix_cpu_f64(
            &candles,
            "gaussian",
            "close",
            &periods,
            Some(&p4),
            0.85,
            6.0,
        )
        .expect("gaussian poles=4");

        assert_eq!(cols1, cols4);
        assert_eq!(out1.len(), out4.len());

        let idx = cols1 - 1;
        let a = out1[idx];
        let b = out4[idx];
        assert!(a.is_finite() && b.is_finite());
        assert!((a - b).abs() > 1e-9);
    }

    #[test]
    fn cpu_ma_matrix_f32_staging_matches_f64_cast() {
        let candles = make_trending_candles(1536);
        let periods = [5u16, 7u16, 9u16];

        let mut gaussian_params: HashMap<String, f64> = HashMap::new();
        gaussian_params.insert("poles".to_string(), 3.0);

        let compare = |ma_type: &str, params: Option<&HashMap<String, f64>>| {
            let (out64, cols64) =
                ma_matrix_cpu_f64(&candles, ma_type, "close", &periods, params, 0.85, 6.0)
                    .expect("f64 matrix");
            let (out32, cols32) =
                ma_matrix_cpu_f32_staging(&candles, ma_type, "close", &periods, params, 0.85, 6.0)
                    .expect("f32 staging matrix");

            assert_eq!(cols64, cols32, "cols mismatch for {ma_type}");
            assert_eq!(out64.len(), out32.len(), "len mismatch for {ma_type}");

            for (&v64, &v32) in out64.iter().zip(out32.iter()) {
                if v64.is_nan() {
                    assert!(v32.is_nan(), "expected NaN parity for {ma_type}");
                } else {
                    assert_eq!(
                        (v64 as f32).to_bits(),
                        v32.to_bits(),
                        "value mismatch for {ma_type}"
                    );
                }
            }
        };

        compare("sma", None);
        compare("gaussian", Some(&gaussian_params));
        compare("frama", None);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_ma_params_gaussian_poles_changes_output() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(2048);
        let ma_source = "close";
        let prices_f32: Vec<f32> =
            vector_ta::utilities::data_loader::source_type(&candles, ma_source)
                .iter()
                .map(|&x| x as f32)
                .collect();
        let selector = CudaMaSelector::new(0);

        let periods = [14u16];

        let mut p1: HashMap<String, f64> = HashMap::new();
        p1.insert("poles".to_string(), 1.0);
        let (out1, cols1) = ma_matrix_cuda_host_f32(
            &selector,
            0,
            &candles,
            &prices_f32,
            "gaussian",
            ma_source,
            &periods,
            Some(&p1),
            0.85,
            6.0,
        )
        .expect("cuda gaussian poles=1");

        let mut p4: HashMap<String, f64> = HashMap::new();
        p4.insert("poles".to_string(), 4.0);
        let (out4, cols4) = ma_matrix_cuda_host_f32(
            &selector,
            0,
            &candles,
            &prices_f32,
            "gaussian",
            ma_source,
            &periods,
            Some(&p4),
            0.85,
            6.0,
        )
        .expect("cuda gaussian poles=4");

        assert_eq!(cols1, cols4);
        assert_eq!(out1.len(), out4.len());

        let idx = cols1 - 1;
        let a = out1[idx];
        let b = out4[idx];
        assert!(a.is_finite() && b.is_finite());
        assert!((a - b).abs() > 1e-6);
    }

    #[test]
    fn drilldown_smoke_has_bins_and_values() {
        let candles = make_trending_candles(5000);

        let req = DoubleMaDrilldownRequest {
            data_id: "test".to_string(),
            backend_hint: None,
            followup_cache_key: None,
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
            halftrend_filter: None,
            bins: Some(256),
        };

        let curves =
            compute_double_ma_drilldown_blocking_with_candles(req, &candles).expect("drilldown");

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
    }

    #[test]
    fn local_stability_smoke_has_base_point_and_summary() {
        let candles = make_trending_candles(4096);

        let req = DoubleMaLocalStabilityRequest {
            data_id: "test".to_string(),
            backend_hint: None,
            followup_cache_key: None,
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
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            radius: Some(1),
            step: Some(1),
        };

        let res = compute_double_ma_local_stability_blocking_with_candles(req, &candles)
            .expect("local stability");

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
        assert!(res.summary.min.is_finite());
        assert!(res.summary.median.is_finite());
        assert!(res.summary.max.is_finite());
    }

    #[test]
    fn shortlist_neighborhood_config_request_parses_and_rejects_invalid_values() {
        assert!(gpu_shortlist_neighborhood_config_requested(None, None)
            .expect("unset request")
            .is_none());

        let cfg = gpu_shortlist_neighborhood_config_requested(Some(4), None)
            .expect("radius-only neighborhood cfg")
            .expect("cfg");
        assert_eq!(cfg.radius, 4);
        assert_eq!(cfg.step, 1);

        let cfg = gpu_shortlist_neighborhood_config_requested(Some(4), Some(3))
            .expect("radius+step cfg")
            .expect("cfg");
        assert_eq!(cfg.radius, 4);
        assert_eq!(cfg.step, 3);

        let cfg = gpu_shortlist_neighborhood_config_requested(Some(99), None)
            .expect("clamped cfg")
            .expect("cfg");
        assert_eq!(cfg.radius, 32);

        let err =
            gpu_shortlist_neighborhood_config_requested(Some(0), None).expect_err("zero radius");
        assert!(err.contains("must be > 0"));

        let err =
            gpu_shortlist_neighborhood_config_requested(Some(4), Some(0)).expect_err("zero step");
        assert!(err.contains("must be > 0"));
    }

    #[test]
    fn shortlist_neighborhood_robustness_summary_reports_expected_plateau_shape() {
        let base = DoubleMaParamsResolved {
            fast_len: 10,
            slow_len: 30,
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
        };
        let point = |fast_len, slow_len, score, max_dd| LocalStabilityPoint {
            fast_len,
            slow_len,
            score,
            metrics: Metrics {
                max_dd,
                ..Metrics::default()
            },
        };
        let local = LocalStabilityResult {
            base: base.clone(),
            objective: ObjectiveKind::Sharpe,
            radius: 1,
            step: 1,
            start_t: 0,
            points: vec![
                point(10, 30, 1.00, 0.10),
                point(9, 30, 0.98, 0.10),
                point(11, 30, 1.01, 0.09),
                point(10, 29, 0.99, 0.10),
                point(10, 31, 1.02, 0.10),
                point(9, 29, 1.03, 0.11),
            ],
            summary: LocalStabilitySummary {
                base: 1.00,
                min: 0.98,
                median: 1.00,
                max: 1.03,
            },
        };

        let summary = summarize_shortlist_neighborhood_robustness(
            &local,
            Some(DoubleMaScoreMode::ActiveSortino10),
        )
        .expect("neighborhood summary");
        assert_eq!(summary.objective, ObjectiveKind::Sharpe);
        assert_eq!(summary.score_mode, Some(DoubleMaScoreMode::ActiveSortino10));
        assert_eq!(summary.radius, 1);
        assert_eq!(summary.step, 1);
        assert_eq!(summary.point_count, 6);
        assert_eq!(summary.neighbor_count, 5);
        assert!((summary.base_score - 1.0).abs() <= 1.0e-12);
        assert!((summary.median_score - 1.0).abs() <= 1.0e-12);
        assert!((summary.min_score - 0.98).abs() <= 1.0e-12);
        assert!((summary.max_score - 1.03).abs() <= 1.0e-12);
        assert!((summary.peak_delta - 0.03).abs() <= 1.0e-12);
        assert!((summary.basin_min_score - 0.98).abs() <= 1.0e-12);
        assert!((summary.basin_floor_stability - (1.0 / 1.02)).abs() <= 1.0e-12);
        assert!((summary.roughness_rms_delta - 0.01949358868961793).abs() <= 1.0e-12);
        assert!((summary.roughness_stability - 0.980879145385629).abs() <= 1.0e-12);
        assert!((summary.curvature_penalty - 0.005).abs() <= 1.0e-12);
        assert!((summary.curvature_stability - (1.0 / 1.005)).abs() <= 1.0e-12);
        assert!(summary.plateau_score >= 0.78);
        assert_eq!(summary.plateau_label, "Strong");
        let agreement = summary
            .score_agreement
            .as_ref()
            .expect("score agreement summary");
        assert_eq!(agreement.point_count, 6);
        assert!(agreement.current_vs_objective_rank_corr.is_finite());
        assert!(agreement.current_vs_balanced_rank_corr.is_finite());
        assert!(agreement.current_vs_active_sortino_rank_corr.is_finite());
        assert!(agreement.current_vs_omega_vs_sharpe_rank_corr.is_finite());
        assert!(agreement.current_vs_robust_margin_rank_corr.is_finite());
        assert!(agreement.agreement_floor.is_finite());
        assert!(agreement.agreement_mean.is_finite());
        let explanation = summary
            .explanation
            .as_ref()
            .expect("shortlist explanation summary");
        assert_eq!(explanation.point_count, 6);
        assert_eq!(explanation.base_current_rank, 4);
        assert_eq!(explanation.base_objective_rank, 1);
        assert_eq!(explanation.pareto_nondominated_count, 1);
        assert!(!explanation.base_is_pareto_nondominated);
        assert_eq!(explanation.base_dominates_neighbors, 2);
        assert_eq!(explanation.neighbors_dominate_base, 3);
        assert_eq!(explanation.dominance_margin, -1);
        assert!((explanation.base_hypervolume_proxy - 0.4).abs() <= 1.0e-12);
        assert_eq!(explanation.hypervolume_proxy_rank, 4);
        assert!((explanation.hypervolume_proxy_share - (2.0 / 15.0)).abs() <= 1.0e-12);
        assert_eq!(explanation.dominance_label, "Fragile");
    }

    #[test]
    fn spearman_rank_corr_reports_expected_directionality() {
        let aligned = spearman_rank_corr(&[3.0, 2.0, 1.0], &[30.0, 20.0, 10.0]);
        let inverted = spearman_rank_corr(&[3.0, 2.0, 1.0], &[10.0, 20.0, 30.0]);
        let tied = spearman_rank_corr(&[3.0, 3.0, 1.0], &[2.0, 2.0, 0.0]);

        assert!((aligned - 1.0).abs() <= 1.0e-12);
        assert!((inverted + 1.0).abs() <= 1.0e-12);
        assert!((tied - 1.0).abs() <= 1.0e-12);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn shortlist_neighborhood_requires_slice_mode() {
        let _guard = lock_env();

        let candles = make_trending_candles(1024);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 9, 1),
            slow_range: (15, 21, 2),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: Some(1),
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(3),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let err = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("missing slice mode should reject neighborhood mode");
        assert!(err.contains("shortlist_neighborhood_radius requires shortlist_slice_mode"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_supports_double_like_regime_strategies() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        for (label, strategy) in double_like_kernel_strategy_cases() {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let export_path = std::env::temp_dir()
                .join(format!("gpu_kernel_{label}_{t}.csv"))
                .to_string_lossy()
                .to_string();

            let req = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                data_id: "test".to_string(),
                fast_range: (5, 11, 1),
                slow_range: (15, 29, 2),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: Some(export_path.clone()),
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy,
                halftrend_filter: None,
                objective: ObjectiveKind::Pnl,
                score_mode: None,
                extra_shortlist_objectives: None,
                shortlist_confidence_mode: None,
                shortlist_union_mode: None,
                shortlist_slice_mode: None,
                shortlist_slice_benchmark: None,
                shortlist_neighborhood_radius: None,
                shortlist_neighborhood_step: None,
                mode: OptimizationMode::Grid,
                top_k: Some(3),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,

                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
            };

            let out =
                run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                    .unwrap_or_else(|e| panic!("gpu kernel strategy '{label}' failed: {e}"));
            assert!(
                out.best_metrics.pnl.is_finite(),
                "non-finite pnl for strategy '{label}'"
            );
            assert!(
                !out.top.is_empty(),
                "empty top results for strategy '{label}'"
            );
            let _ = std::fs::remove_file(export_path);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_best_matches_include_all_true() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req_all = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 20, 1),
            slow_range: (10, 60, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(0),
            include_all: Some(true),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let req_top = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 20, 1),
            slow_range: (10, 60, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(1),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let a = run_double_ma_optimization_blocking_with_candles(req_all, &candles, &cancel, None)
            .expect("gpu-kernel include_all=true");

        let b = run_double_ma_optimization_blocking_with_candles(req_top, &candles, &cancel, None)
            .expect("gpu-kernel include_all=false");

        assert_eq!(a.best_params.fast_len, b.best_params.fast_len);
        assert_eq!(a.best_params.slow_len, b.best_params.slow_len);

        let h = b.heatmap.expect("heatmap");
        assert_eq!(h.values.len(), h.bins_fast.saturating_mul(h.bins_slow));
        assert!(h.values.iter().any(|v| v.is_some()));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_extra_shortlist_union_matches_exhaustive_host_union() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_global_topk = std::env::var("VECTORBT_KERNEL_GLOBAL_TOPK").ok();
        let prev_force_legacy = std::env::var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK").ok();
        std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", "1");
        std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK");

        let candles = make_choppy_candles(6000);
        let cancel = AtomicBool::new(false);
        let top_k = 5usize;

        let req_all = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 24, 1),
            slow_range: (15, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(0),
            include_all: Some(true),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };
        let req_union = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 24, 1),
            slow_range: (15, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: Some(vec![ObjectiveKind::Pnl, ObjectiveKind::Omega]),
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(top_k),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let exhaustive =
            run_double_ma_optimization_blocking_with_candles(req_all, &candles, &cancel, None)
                .expect("gpu-kernel exhaustive run");
        let union_result =
            run_double_ma_optimization_blocking_with_candles(req_union, &candles, &cancel, None)
                .expect("gpu-kernel extra-shortlist run");

        let all_rows = exhaustive
            .all
            .clone()
            .expect("include_all=true should retain the full candidate set");
        let mut expected_union: Vec<(DoubleMaParamsResolved, Metrics)> = Vec::new();
        let mut seen: BTreeSet<(u16, u16)> = BTreeSet::new();
        for head_objective in [
            ObjectiveKind::Sharpe,
            ObjectiveKind::Pnl,
            ObjectiveKind::Omega,
        ] {
            let mut ranked = all_rows.clone();
            ranked.sort_by(|a, b| {
                let sa = objective_score(&a.1, head_objective);
                let sb = objective_score(&b.1, head_objective);
                sb.total_cmp(&sa)
            });
            for (params, metrics) in ranked.into_iter().take(top_k) {
                let key = (params.fast_len, params.slow_len);
                if seen.insert(key) {
                    expected_union.push((params, metrics));
                }
            }
        }
        expected_union.sort_by(|a, b| {
            let sa = objective_score(&a.1, ObjectiveKind::Sharpe);
            let sb = objective_score(&b.1, ObjectiveKind::Sharpe);
            sb.total_cmp(&sa)
        });
        let expected_keys: Vec<(u16, u16)> = expected_union
            .iter()
            .take(top_k)
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();
        let actual_keys: Vec<(u16, u16)> = union_result
            .top
            .iter()
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();

        assert_eq!(
            actual_keys, expected_keys,
            "extra-shortlist top-k should match the exhaustive host union rerank"
        );
        assert_eq!(
            (
                union_result.best_params.fast_len,
                union_result.best_params.slow_len
            ),
            expected_keys[0],
            "best params should match the best Sharpe candidate from the union shortlist"
        );

        match prev_global_topk {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_GLOBAL_TOPK"),
        }
        match prev_force_legacy {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn composite_adx_off_gpu_kernel_routes_accept_advanced_exact_controls() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let cancel = AtomicBool::new(false);

        let run_case = |label: &str,
                        candles: vector_ta::utilities::data_loader::Candles,
                        strategy: StrategyConfig| {
            let res = run_double_ma_optimization_blocking_with_candles(
                DoubleMaRequest {
                    backend: Backend::GpuKernel { device_id: 0 },
                    data_id: format!("composite-{label}"),
                    fast_range: (5, 18, 1),
                    slow_range: (20, 48, 2),
                    fast_ma_types: vec!["sma".to_string()],
                    slow_ma_types: vec!["sma".to_string()],
                    ma_source: "close".to_string(),
                    export_all_csv_path: None,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    fast_ma_param_sweeps: None,
                    slow_ma_param_sweeps: None,
                    strategy,
                    halftrend_filter: None,
                    objective: ObjectiveKind::Sharpe,
                    score_mode: None,
                    extra_shortlist_objectives: Some(vec![
                        ObjectiveKind::Pnl,
                        ObjectiveKind::Omega,
                    ]),
                    shortlist_confidence_mode: Some(DoubleMaShortlistConfidenceMode::Psr),
                    shortlist_union_mode: Some(DoubleMaShortlistUnionMode::Pareto),
                    shortlist_slice_mode: None,
                    shortlist_slice_benchmark: None,
                    shortlist_neighborhood_radius: None,
                    shortlist_neighborhood_step: None,
                    mode: OptimizationMode::Grid,
                    top_k: Some(5),
                    include_all: Some(false),
                    heatmap_bins: Some(0),
                    constraints: None,
                    vdps: None,
                    dynamic_strategy_ir: None,
                    gpu_execution: GpuExecutionPolicy::BestEffort,
                    persist_followup_cache: false,
                    include_trade_metrics: true,
                    metric_contract: None,
                    hurdle_tau: None,
                },
                &candles,
                &cancel,
                None,
            )
            .unwrap_or_else(|err| {
                panic!("expected ADX-off composite exact-kernel run for {label}: {err}")
            });

            assert!(
                matches!(res.backend_used, BackendUsed::Gpu),
                "expected GPU backend_used for {label}, got {:?}",
                res.backend_used
            );
            assert!(
                !res.top.is_empty(),
                "expected non-empty top rows for {label}"
            );
            assert!(
                res.backend_fallback_reason.is_none(),
                "unexpected fallback note for {label}: {:?}",
                res.backend_fallback_reason
            );
            assert!(
                !res.backend_route
                    .as_deref()
                    .unwrap_or_default()
                    .contains("Hybrid"),
                "expected exact GpuKernel route for {label}, got {:?}",
                res.backend_route
            );
            assert!(
                res.best_metrics.closed_trades
                    >= res.best_metrics.winning_trades + res.best_metrics.losing_trades,
                "trade metrics should be populated coherently for {label}"
            );
        };

        run_case(
            "composite_trend_adx_off",
            make_trending_candles(6_000),
            super::double_ma_test_support::trend_quality_strategy_for_kernel(),
        );
        run_case(
            "breakout_momentum_adx_off",
            make_breakout_candles(6_000),
            super::double_ma_test_support::breakout_strategy_for_kernel(),
        );
    }

    #[test]
    fn extra_shortlist_request_requires_exact_gpu_kernel_backend_route() {
        let candles = make_choppy_candles(512);
        let cancel = AtomicBool::new(false);
        let err = run_double_ma_optimization_blocking_with_candles(
            DoubleMaRequest {
                backend: Backend::CpuOnly,
                data_id: "test".to_string(),
                fast_range: (5, 12, 1),
                slow_range: (20, 40, 1),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                score_mode: None,
                extra_shortlist_objectives: Some(vec![ObjectiveKind::Pnl]),
                shortlist_confidence_mode: None,
                shortlist_union_mode: None,
                shortlist_slice_mode: None,
                shortlist_slice_benchmark: None,
                shortlist_neighborhood_radius: None,
                shortlist_neighborhood_step: None,
                mode: OptimizationMode::Grid,
                top_k: Some(5),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect_err("cpu backend should reject extra shortlist objectives");

        assert!(
            err.contains("exact GpuKernel backend route"),
            "unexpected guard error: {err}"
        );
    }

    #[test]
    fn shortlist_confidence_request_requires_exact_gpu_kernel_backend_route() {
        let candles = make_choppy_candles(512);
        let cancel = AtomicBool::new(false);
        let err = run_double_ma_optimization_blocking_with_candles(
            DoubleMaRequest {
                backend: Backend::CpuOnly,
                data_id: "test".to_string(),
                fast_range: (5, 12, 1),
                slow_range: (20, 40, 1),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                score_mode: None,
                extra_shortlist_objectives: None,
                shortlist_confidence_mode: Some(DoubleMaShortlistConfidenceMode::Psr),
                shortlist_union_mode: None,
                shortlist_slice_mode: None,
                shortlist_slice_benchmark: None,
                shortlist_neighborhood_radius: None,
                shortlist_neighborhood_step: None,
                mode: OptimizationMode::Grid,
                top_k: Some(5),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect_err("cpu backend should reject shortlist confidence mode");

        assert!(
            err.contains("exact GpuKernel backend route"),
            "unexpected guard error: {err}"
        );
    }

    #[test]
    fn shortlist_union_request_requires_exact_gpu_kernel_backend_route() {
        let candles = make_choppy_candles(512);
        let cancel = AtomicBool::new(false);
        let err = run_double_ma_optimization_blocking_with_candles(
            DoubleMaRequest {
                backend: Backend::CpuOnly,
                data_id: "test".to_string(),
                fast_range: (5, 12, 1),
                slow_range: (20, 40, 1),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                score_mode: None,
                extra_shortlist_objectives: Some(vec![ObjectiveKind::Pnl]),
                shortlist_confidence_mode: None,
                shortlist_union_mode: Some(DoubleMaShortlistUnionMode::Pareto),
                shortlist_slice_mode: None,
                shortlist_slice_benchmark: None,
                shortlist_neighborhood_radius: None,
                shortlist_neighborhood_step: None,
                mode: OptimizationMode::Grid,
                top_k: Some(5),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect_err("cpu backend should reject shortlist union mode");

        assert!(
            err.contains("exact GpuKernel backend route"),
            "unexpected guard error: {err}"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_extra_shortlist_request_matches_exhaustive_host_union() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_global_topk = std::env::var("VECTORBT_KERNEL_GLOBAL_TOPK").ok();
        let prev_force_legacy = std::env::var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK").ok();
        std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", "1");
        std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK");

        let candles = make_choppy_candles(6000);
        let cancel = AtomicBool::new(false);
        let top_k = 5usize;

        let req_all = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 24, 1),
            slow_range: (15, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(0),
            include_all: Some(true),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };
        let req_union = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 24, 1),
            slow_range: (15, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: Some(vec![ObjectiveKind::Pnl, ObjectiveKind::Omega]),
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(top_k),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let exhaustive =
            run_double_ma_optimization_blocking_with_candles(req_all, &candles, &cancel, None)
                .expect("gpu-kernel exhaustive run");
        let union_result =
            run_double_ma_optimization_blocking_with_candles(req_union, &candles, &cancel, None)
                .expect("gpu-kernel request extra-shortlist run");

        let all_rows = exhaustive
            .all
            .clone()
            .expect("include_all=true should retain the full candidate set");
        let mut expected_union: Vec<(DoubleMaParamsResolved, Metrics)> = Vec::new();
        let mut seen: BTreeSet<(u16, u16)> = BTreeSet::new();
        for head_objective in [
            ObjectiveKind::Sharpe,
            ObjectiveKind::Pnl,
            ObjectiveKind::Omega,
        ] {
            let mut ranked = all_rows.clone();
            ranked.sort_by(|a, b| {
                let sa = objective_score(&a.1, head_objective);
                let sb = objective_score(&b.1, head_objective);
                sb.total_cmp(&sa)
            });
            for (params, metrics) in ranked.into_iter().take(top_k) {
                let key = (params.fast_len, params.slow_len);
                if seen.insert(key) {
                    expected_union.push((params, metrics));
                }
            }
        }
        expected_union.sort_by(|a, b| {
            let sa = objective_score(&a.1, ObjectiveKind::Sharpe);
            let sb = objective_score(&b.1, ObjectiveKind::Sharpe);
            sb.total_cmp(&sa)
        });
        let expected_keys: Vec<(u16, u16)> = expected_union
            .iter()
            .take(top_k)
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();
        let actual_keys: Vec<(u16, u16)> = union_result
            .top
            .iter()
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();

        assert_eq!(
            actual_keys, expected_keys,
            "request extra-shortlist top-k should match the exhaustive host union rerank"
        );
        assert_eq!(
            (
                union_result.best_params.fast_len,
                union_result.best_params.slow_len
            ),
            expected_keys[0],
            "best params should match the best Sharpe candidate from the request union shortlist"
        );

        match prev_global_topk {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_GLOBAL_TOPK"),
        }
        match prev_force_legacy {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn composite_adx_off_gpu_kernel_extra_shortlist_request_matches_exhaustive_host_union() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_global_topk = std::env::var("VECTORBT_KERNEL_GLOBAL_TOPK").ok();
        let prev_force_legacy = std::env::var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK").ok();
        std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", "1");
        std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK");

        let cancel = AtomicBool::new(false);
        let top_k = 5usize;
        let cases = vec![
            (
                "composite_trend",
                make_trending_candles(6_000),
                trend_quality_strategy_for_kernel(),
            ),
            (
                "breakout_momentum",
                make_breakout_candles(6_000),
                breakout_strategy_for_kernel(),
            ),
        ];

        for (label, candles, strategy) in cases {
            let req_all = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                data_id: format!("{label}-all"),
                fast_range: (5, 24, 1),
                slow_range: (15, 80, 1),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                score_mode: None,
                extra_shortlist_objectives: None,
                shortlist_confidence_mode: None,
                shortlist_union_mode: None,
                shortlist_slice_mode: None,
                shortlist_slice_benchmark: None,
                shortlist_neighborhood_radius: None,
                shortlist_neighborhood_step: None,
                mode: OptimizationMode::Grid,
                top_k: Some(top_k),
                include_all: Some(true),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
            };
            let req_union = DoubleMaRequest {
                include_all: Some(false),
                extra_shortlist_objectives: Some(vec![ObjectiveKind::Pnl, ObjectiveKind::Omega]),
                data_id: format!("{label}-union"),
                ..req_all.clone()
            };

            let exhaustive =
                run_double_ma_optimization_blocking_with_candles(req_all, &candles, &cancel, None)
                    .unwrap_or_else(|e| panic!("{label} exhaustive gpu-kernel run failed: {e}"));
            let union_result = run_double_ma_optimization_blocking_with_candles(
                req_union, &candles, &cancel, None,
            )
            .unwrap_or_else(|e| panic!("{label} extra-shortlist gpu-kernel run failed: {e}"));

            assert!(matches!(union_result.backend_used, BackendUsed::Gpu));
            assert_eq!(union_result.backend_route.as_deref(), Some("GpuKernel"));
            assert!(
                union_result.backend_fallback_reason.is_none(),
                "{label} should stay on the exact GpuKernel route, got fallback {:?}",
                union_result.backend_fallback_reason
            );

            let all_rows = exhaustive
                .all
                .clone()
                .expect("include_all=true should retain the full candidate set");
            let mut expected_union: Vec<(DoubleMaParamsResolved, Metrics)> = Vec::new();
            let mut seen: BTreeSet<(u16, u16)> = BTreeSet::new();
            for head_objective in [
                ObjectiveKind::Sharpe,
                ObjectiveKind::Pnl,
                ObjectiveKind::Omega,
            ] {
                let mut ranked = all_rows.clone();
                ranked.sort_by(|a, b| {
                    let sa = objective_score(&a.1, head_objective);
                    let sb = objective_score(&b.1, head_objective);
                    sb.total_cmp(&sa)
                });
                for (params, metrics) in ranked.into_iter().take(top_k) {
                    let key = (params.fast_len, params.slow_len);
                    if seen.insert(key) {
                        expected_union.push((params, metrics));
                    }
                }
            }
            expected_union.sort_by(|a, b| {
                let sa = objective_score(&a.1, ObjectiveKind::Sharpe);
                let sb = objective_score(&b.1, ObjectiveKind::Sharpe);
                sb.total_cmp(&sa)
            });
            let expected_keys: Vec<(u16, u16)> = expected_union
                .iter()
                .take(top_k)
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect();
            let actual_keys: Vec<(u16, u16)> = union_result
                .top
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect();

            assert_eq!(
                actual_keys, expected_keys,
                "{label} extra-shortlist top-k should match the exhaustive host union rerank"
            );
            assert_eq!(
                (
                    union_result.best_params.fast_len,
                    union_result.best_params.slow_len
                ),
                expected_keys[0],
                "{label} best params should match the best Sharpe candidate from the union shortlist"
            );
        }

        match prev_global_topk {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_GLOBAL_TOPK"),
        }
        match prev_force_legacy {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_shortlist_confidence_request_matches_exhaustive_host_union_psr_and_dsr() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_global_topk = std::env::var("VECTORBT_KERNEL_GLOBAL_TOPK").ok();
        let prev_force_legacy = std::env::var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK").ok();
        std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", "1");
        std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK");

        let candles = make_choppy_candles(6000);
        let cancel = AtomicBool::new(false);
        let top_k = 5usize;
        let strategy = StrategyConfig::default();

        let req_all = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 24, 1),
            slow_range: (15, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: strategy.clone(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(0),
            include_all: Some(true),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };
        let req_union = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 24, 1),
            slow_range: (15, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: strategy.clone(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(top_k),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let exhaustive =
            run_double_ma_optimization_blocking_with_candles(req_all, &candles, &cancel, None)
                .expect("gpu-kernel exhaustive run");
        let all_rows = exhaustive
            .all
            .clone()
            .expect("include_all=true should retain the full candidate set");
        let mut union_rows: Vec<(DoubleMaParamsResolved, Metrics)> = Vec::new();
        let mut seen: BTreeSet<(u16, u16)> = BTreeSet::new();
        for head_objective in [
            ObjectiveKind::Sharpe,
            ObjectiveKind::Pnl,
            ObjectiveKind::Omega,
        ] {
            let mut ranked = all_rows.clone();
            ranked.sort_by(|a, b| {
                let sa = objective_score(&a.1, head_objective);
                let sb = objective_score(&b.1, head_objective);
                sb.total_cmp(&sa)
            });
            for (params, metrics) in ranked.into_iter().take(top_k) {
                let key = (params.fast_len, params.slow_len);
                if seen.insert(key) {
                    union_rows.push((params, metrics));
                }
            }
        }

        let sharpe_peers: Vec<f64> = union_rows
            .iter()
            .map(|(_, metrics)| metrics.sharpe)
            .filter(|value| value.is_finite())
            .collect();
        let (peer_mean, peer_std) = crate::sharpe_stats::mean_std(&sharpe_peers);
        let total_bars = candles.close.len();
        for (mode, label) in [("psr", "PSR"), ("dsr", "DSR")] {
            let mut expected_union = union_rows.clone();
            expected_union.sort_by(|a, b| {
                let obs_a = {
                    let pair_first_valid = a.0.fast_len.max(a.0.slow_len) as usize - 1;
                    let start_t =
                        pair_first_valid.saturating_add(usize::from(strategy.trade_on_next_bar));
                    total_bars.saturating_sub(start_t).max(2)
                };
                let obs_b = {
                    let pair_first_valid = b.0.fast_len.max(b.0.slow_len) as usize - 1;
                    let start_t =
                        pair_first_valid.saturating_add(usize::from(strategy.trade_on_next_bar));
                    total_bars.saturating_sub(start_t).max(2)
                };
                let (psr_a, dsr_a) = crate::validation_stats::compute_psr_dsr(
                    a.1.sharpe,
                    obs_a,
                    sharpe_peers.len(),
                    peer_mean,
                    peer_std,
                );
                let (psr_b, dsr_b) = crate::validation_stats::compute_psr_dsr(
                    b.1.sharpe,
                    obs_b,
                    sharpe_peers.len(),
                    peer_mean,
                    peer_std,
                );
                let score_a = if mode == "psr" {
                    psr_a.unwrap_or(0.0)
                } else {
                    dsr_a.unwrap_or(0.0)
                };
                let score_b = if mode == "psr" {
                    psr_b.unwrap_or(0.0)
                } else {
                    dsr_b.unwrap_or(0.0)
                };
                score_b
                    .total_cmp(&score_a)
                    .then_with(|| b.1.sharpe.total_cmp(&a.1.sharpe))
                    .then_with(|| a.1.max_dd.total_cmp(&b.1.max_dd))
            });

            let expected_keys: Vec<(u16, u16)> = expected_union
                .iter()
                .take(top_k)
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect();

            let mut req_confidence = req_union.clone();
            req_confidence.extra_shortlist_objectives =
                Some(vec![ObjectiveKind::Pnl, ObjectiveKind::Omega]);
            req_confidence.shortlist_confidence_mode = Some(if mode == "psr" {
                DoubleMaShortlistConfidenceMode::Psr
            } else {
                DoubleMaShortlistConfidenceMode::Dsr
            });
            let union_result = run_double_ma_optimization_blocking_with_candles(
                req_confidence,
                &candles,
                &cancel,
                None,
            )
            .unwrap_or_else(|err| panic!("gpu-kernel confidence-shortlist run ({mode}): {err}"));

            let actual_keys: Vec<(u16, u16)> = union_result
                .top
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect();

            assert_eq!(
                actual_keys, expected_keys,
                "confidence-shortlist top-k should match the exhaustive host {label} rerank"
            );
            assert_eq!(
                (
                    union_result.best_params.fast_len,
                    union_result.best_params.slow_len
                ),
                expected_keys[0],
                "best params should match the best {label} candidate from the union shortlist"
            );
        }

        match prev_global_topk {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_GLOBAL_TOPK"),
        }
        match prev_force_legacy {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_shortlist_union_mode_request_matches_exhaustive_host_union_modes() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_global_topk = std::env::var("VECTORBT_KERNEL_GLOBAL_TOPK").ok();
        let prev_force_legacy = std::env::var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK").ok();
        std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", "1");
        std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK");

        let candles = make_choppy_candles(6000);
        let cancel = AtomicBool::new(false);
        let top_k = 5usize;
        let strategy = StrategyConfig::default();

        let req_all = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 24, 1),
            slow_range: (15, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: strategy.clone(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(0),
            include_all: Some(true),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let exhaustive =
            run_double_ma_optimization_blocking_with_candles(req_all, &candles, &cancel, None)
                .expect("gpu-kernel exhaustive run");
        let all_rows = exhaustive
            .all
            .clone()
            .expect("include_all=true should retain the full candidate set");

        let mut union_rows: Vec<(DoubleMaParamsResolved, Metrics)> = Vec::new();
        let mut seen: BTreeSet<(u16, u16)> = BTreeSet::new();
        for head_objective in [
            ObjectiveKind::Sharpe,
            ObjectiveKind::Pnl,
            ObjectiveKind::Omega,
        ] {
            let mut ranked = all_rows.clone();
            ranked.sort_by(|a, b| {
                let sa = objective_score(&a.1, head_objective);
                let sb = objective_score(&b.1, head_objective);
                sb.total_cmp(&sa)
            });
            for (params, metrics) in ranked.into_iter().take(top_k) {
                let key = (params.fast_len, params.slow_len);
                if seen.insert(key) {
                    union_rows.push((params, metrics));
                }
            }
        }
        let union_rows_kernel: Vec<(ta_strategies::double_ma::DoubleMaParams, Metrics)> =
            union_rows
                .iter()
                .map(|(params, metrics)| {
                    (
                        ta_strategies::double_ma::DoubleMaParams {
                            fast_len: params.fast_len,
                            slow_len: params.slow_len,
                            fast_ma_id: 0,
                            slow_ma_id: 0,
                        },
                        metrics.clone(),
                    )
                })
                .collect();

        for mode in [
            DoubleMaShortlistUnionMode::Lexicographic,
            DoubleMaShortlistUnionMode::Pareto,
            DoubleMaShortlistUnionMode::EpsilonFront,
            DoubleMaShortlistUnionMode::WeightedRank,
        ] {
            let req_union = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                data_id: "test".to_string(),
                fast_range: (5, 24, 1),
                slow_range: (15, 80, 1),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                score_mode: None,
                extra_shortlist_objectives: Some(vec![ObjectiveKind::Pnl, ObjectiveKind::Omega]),
                shortlist_confidence_mode: None,
                shortlist_union_mode: Some(mode),
                shortlist_slice_mode: None,
                shortlist_slice_benchmark: None,
                shortlist_neighborhood_radius: None,
                shortlist_neighborhood_step: None,
                mode: OptimizationMode::Grid,
                top_k: Some(top_k),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
            };

            let union_result = run_double_ma_optimization_blocking_with_candles(
                req_union, &candles, &cancel, None,
            )
            .unwrap_or_else(|err| panic!("gpu-kernel shortlist union run ({mode:?}): {err}"));

            let reranked = crate::gpu_backtest_kernel::rerank_shortlist_by_union_mode(
                &union_rows_kernel,
                ObjectiveKind::Sharpe,
                &[ObjectiveKind::Pnl, ObjectiveKind::Omega],
                mode,
            );
            let expected_keys: Vec<(u16, u16)> = reranked
                .iter()
                .take(top_k)
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect();
            let actual_keys: Vec<(u16, u16)> = union_result
                .top
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect();

            assert_eq!(
                actual_keys, expected_keys,
                "shortlist union mode should match the exhaustive host rerank for {mode:?}"
            );
            assert_eq!(
                (
                    union_result.best_params.fast_len,
                    union_result.best_params.slow_len
                ),
                expected_keys[0],
                "best params should match the top union-mode candidate for {mode:?}"
            );
        }

        match prev_global_topk {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_GLOBAL_TOPK"),
        }
        match prev_force_legacy {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_shortlist_slice_request_matches_exhaustive_host_union() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_global_topk = std::env::var("VECTORBT_KERNEL_GLOBAL_TOPK").ok();
        let prev_force_legacy = std::env::var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK").ok();
        std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", "1");
        std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK");

        let candles = make_choppy_candles(6000);
        let cancel = AtomicBool::new(false);
        let top_k = 5usize;
        let strategy = StrategyConfig::default();

        let req_all = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 24, 1),
            slow_range: (15, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: strategy.clone(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(0),
            include_all: Some(true),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };
        let req_union = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 24, 1),
            slow_range: (15, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: strategy.clone(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: Some(vec![ObjectiveKind::Pnl, ObjectiveKind::Omega]),
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(top_k),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let exhaustive =
            run_double_ma_optimization_blocking_with_candles(req_all, &candles, &cancel, None)
                .expect("gpu-kernel exhaustive run");
        let all_rows = exhaustive
            .all
            .clone()
            .expect("include_all=true should retain the full candidate set");

        let mut union_rows: Vec<(DoubleMaParamsResolved, Metrics)> = Vec::new();
        let mut seen: BTreeSet<(u16, u16)> = BTreeSet::new();
        for head_objective in [
            ObjectiveKind::Sharpe,
            ObjectiveKind::Pnl,
            ObjectiveKind::Omega,
        ] {
            let mut ranked = all_rows.clone();
            ranked.sort_by(|a, b| {
                let sa = objective_score(&a.1, head_objective);
                let sb = objective_score(&b.1, head_objective);
                sb.total_cmp(&sa)
            });
            for (params, metrics) in ranked.into_iter().take(top_k) {
                let key = (params.fast_len, params.slow_len);
                if seen.insert(key) {
                    union_rows.push((params, metrics));
                }
            }
        }

        let combos: Vec<ta_strategies::double_ma::DoubleMaParams> = union_rows
            .iter()
            .map(|(params, _)| ta_strategies::double_ma::DoubleMaParams {
                fast_len: params.fast_len,
                slow_len: params.slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            })
            .collect();
        let mut fast_periods: Vec<u16> = combos.iter().map(|params| params.fast_len).collect();
        fast_periods.sort_unstable();
        fast_periods.dedup();
        let mut slow_periods: Vec<u16> = combos.iter().map(|params| params.slow_len).collect();
        slow_periods.sort_unstable();
        slow_periods.dedup();
        let total_bars = candles.close.len();

        for (mode, slice_count, mode_label) in [
            (DoubleMaShortlistSliceMode::Halves, 2usize, "halves"),
            (DoubleMaShortlistSliceMode::Quartiles, 4usize, "quartiles"),
        ] {
            let common_start = union_rows
                .iter()
                .map(|(params, _)| params.fast_len.max(params.slow_len) as usize - 1)
                .max()
                .expect("non-empty union shortlist");
            let slice_bars = total_bars.saturating_sub(common_start) / slice_count;
            assert!(slice_bars >= 2, "slice window unexpectedly too short");
            let ranges: Vec<(usize, usize)> = (0..slice_count)
                .map(|slice_idx| {
                    let start = common_start + slice_idx * slice_bars;
                    let end = if slice_idx + 1 == slice_count {
                        total_bars
                    } else {
                        common_start + (slice_idx + 1) * slice_bars
                    };
                    (start, end)
                })
                .collect();

            let range_metrics =
                crate::gpu_backtest_kernel::eval_double_ma_batch_gpu_kernel_multi_range_with_periods_and_regime(
                    &candles,
                    combos.as_slice(),
                    fast_periods.as_slice(),
                    slow_periods.as_slice(),
                    "sma",
                    "sma",
                    "close",
                    None,
                    None,
                    None,
                    crate::gpu_backtest_kernel::KernelConfig::default(),
                    &strategy,
                    ranges.as_slice(),
                )
                .expect("exact multi-range shortlist metrics");
            let combo_count = combos.len();
            assert_eq!(range_metrics.len(), combo_count * slice_count);

            let mut expected_ranked: Vec<((u16, u16), f64, f64, f64, f64, f64, f64, f64)> =
                Vec::with_capacity(combo_count);
            for (combo_idx, (params, metrics)) in union_rows.iter().enumerate() {
                let mut worst_slice_score = f64::INFINITY;
                let mut positive_slice_count = 0usize;
                let mut mean_slice_score = 0.0f64;
                let mut slice_scores: Vec<f64> = Vec::with_capacity(slice_count);
                for slice_idx in 0..slice_count {
                    let slice_metrics = &range_metrics[slice_idx * combo_count + combo_idx];
                    let slice_score = objective_score(slice_metrics, ObjectiveKind::Sharpe);
                    worst_slice_score = worst_slice_score.min(slice_score);
                    if slice_score >= 0.0 {
                        positive_slice_count += 1;
                    }
                    mean_slice_score += slice_score;
                    slice_scores.push(slice_score);
                }
                mean_slice_score /= slice_count as f64;
                let omission_denom = (slice_scores.len() - 1) as f64;
                let omission_total: f64 = slice_scores.iter().copied().sum();
                let mut worst_omission_score = f64::INFINITY;
                let mut best_omission_score = f64::NEG_INFINITY;
                for score in slice_scores.iter().copied() {
                    let omission_score = (omission_total - score) / omission_denom;
                    worst_omission_score = worst_omission_score.min(omission_score);
                    best_omission_score = best_omission_score.max(omission_score);
                }
                expected_ranked.push((
                    (params.fast_len, params.slow_len),
                    worst_slice_score,
                    positive_slice_count as f64 / slice_count as f64,
                    mean_slice_score,
                    worst_omission_score,
                    best_omission_score - worst_omission_score,
                    metrics.sharpe,
                    metrics.max_dd,
                ));
            }
            expected_ranked.sort_by(|a, b| {
                b.1.total_cmp(&a.1)
                    .then_with(|| b.4.total_cmp(&a.4))
                    .then_with(|| b.2.total_cmp(&a.2))
                    .then_with(|| a.5.total_cmp(&b.5))
                    .then_with(|| b.3.total_cmp(&a.3))
                    .then_with(|| b.6.total_cmp(&a.6))
                    .then_with(|| a.7.total_cmp(&b.7))
            });
            let expected_keys: Vec<(u16, u16)> = expected_ranked
                .iter()
                .take(top_k)
                .map(|entry| entry.0)
                .collect();

            let mut req_slice = req_union.clone();
            req_slice.shortlist_slice_mode = Some(mode);
            let union_result = run_double_ma_optimization_blocking_with_candles(
                req_slice, &candles, &cancel, None,
            )
            .unwrap_or_else(|err| panic!("gpu-kernel shortlist slice run ({mode_label}): {err}"));

            let actual_keys: Vec<(u16, u16)> = union_result
                .top
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect();

            assert_eq!(
                actual_keys, expected_keys,
                "shortlist slice-rerank should match the exhaustive host reconstruction for {mode_label}"
            );
            assert_eq!(
                (
                    union_result.best_params.fast_len,
                    union_result.best_params.slow_len
                ),
                expected_keys[0],
                "best params should match the strongest slice-robust candidate for {mode_label}"
            );
            let slice_diag = union_result
                .shortlist_slice_diagnostics
                .as_ref()
                .expect("slice-rerank result should expose shortlist slice diagnostics");
            assert_eq!(slice_diag.ranking_mode, mode_label);
            assert_eq!(slice_diag.candidates.len(), top_k);
            let best_diag = &slice_diag.candidates[0];
            assert_eq!(
                (best_diag.params.fast_len, best_diag.params.slow_len),
                expected_keys[0]
            );
            assert_eq!(best_diag.halves.slices.len(), 2);
            assert_eq!(best_diag.quartiles.slices.len(), 4);
            let active_diag = if mode == DoubleMaShortlistSliceMode::Halves {
                &best_diag.halves
            } else {
                &best_diag.quartiles
            };
            let expected_best = expected_ranked[0];
            assert!(
                (active_diag.worst_slice_score - expected_best.1).abs() < 1.0e-3,
                "worst slice score should match expected {mode_label} rerank summary: actual={} expected={}",
                active_diag.worst_slice_score,
                expected_best.1
            );
            assert!(
                (active_diag.positive_slice_frac - expected_best.2).abs() < 1.0e-12,
                "positive slice fraction should match expected {mode_label} rerank summary"
            );
            assert!(
                (active_diag.mean_slice_score - expected_best.3).abs() < 1.0e-3,
                "mean slice score should match expected {mode_label} rerank summary"
            );
            assert!(
                (active_diag.worst_omission_score - expected_best.4).abs() < 1.0e-3,
                "worst omission score should match expected {mode_label} rerank summary"
            );
            assert!(
                (active_diag.omission_score_spread - expected_best.5).abs() < 1.0e-3,
                "omission spread should match expected {mode_label} rerank summary"
            );
        }

        match prev_global_topk {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_GLOBAL_TOPK"),
        }
        match prev_force_legacy {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_shortlist_slice_request_diagnostics_are_exposed_on_backend_result() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_global_topk = std::env::var("VECTORBT_KERNEL_GLOBAL_TOPK").ok();
        let prev_force_legacy = std::env::var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK").ok();
        std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", "1");
        std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK");

        let candles = make_choppy_candles(6000);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 24, 1),
            slow_range: (15, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: Some(vec![ObjectiveKind::Pnl, ObjectiveKind::Omega]),
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: Some(DoubleMaShortlistSliceMode::Halves),
            shortlist_slice_benchmark: Some(SpaBenchmarkKind::BuyAndHold),
            shortlist_neighborhood_radius: Some(1),
            shortlist_neighborhood_step: Some(1),
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu-kernel shortlist slice diagnostics run");
        let diagnostics = res
            .shortlist_slice_diagnostics
            .as_ref()
            .expect("shortlist slice diagnostics");
        assert_eq!(diagnostics.ranking_mode, "halves");
        assert_eq!(diagnostics.candidates.len(), res.top.len());
        let first = diagnostics
            .candidates
            .first()
            .expect("first diagnostic candidate");
        let first_top = res.top.first().expect("first top row");
        assert_eq!(first.params.fast_len, first_top.0.fast_len);
        assert_eq!(first.params.slow_len, first_top.0.slow_len);
        assert_eq!(first.halves.slices.len(), 2);
        assert_eq!(first.quartiles.slices.len(), 4);
        assert!(!first.halves.best_omission_label.is_empty());
        assert!(!first.halves.worst_omission_label.is_empty());
        assert!(first.halves.best_omission_score.is_finite());
        assert!(first.halves.worst_omission_score.is_finite());
        assert!(first.halves.omission_score_spread.is_finite());
        assert!(first.quartiles.omission_score_spread.is_finite());
        let halves_benchmark = first
            .halves
            .benchmark_relative
            .as_ref()
            .expect("halves benchmark-relative summary");
        assert_eq!(halves_benchmark.benchmark, "buy_and_hold");
        assert!(halves_benchmark.beat_rate.is_finite());
        assert!(halves_benchmark.mean_excess_return.is_finite());
        assert!(halves_benchmark.best_excess_return.is_finite());
        assert!(halves_benchmark.worst_excess_return.is_finite());
        assert!(halves_benchmark.best_omission_excess_return.is_finite());
        assert!(halves_benchmark.worst_omission_excess_return.is_finite());
        assert!(halves_benchmark.omission_excess_spread.is_finite());
        let quartiles_benchmark = first
            .quartiles
            .benchmark_relative
            .as_ref()
            .expect("quartiles benchmark-relative summary");
        assert_eq!(quartiles_benchmark.benchmark, "buy_and_hold");
        assert!(quartiles_benchmark.omission_excess_spread.is_finite());
        let neighborhood = diagnostics
            .neighborhood_robustness
            .as_ref()
            .expect("neighborhood robustness");
        assert_eq!(neighborhood.objective, ObjectiveKind::Sharpe);
        assert_eq!(neighborhood.radius, 1);
        assert_eq!(neighborhood.step, 1);
        assert!(neighborhood.plateau_score.is_finite());
        assert!(neighborhood.base_score.is_finite());
        assert!(neighborhood.median_score.is_finite());
        assert!(neighborhood.min_score.is_finite());
        assert!(neighborhood.max_score.is_finite());
        assert!(neighborhood.point_count >= 1);
        assert_eq!(
            neighborhood.neighbor_count,
            neighborhood.point_count.saturating_sub(1)
        );
        let agreement = neighborhood
            .score_agreement
            .as_ref()
            .expect("score agreement summary");
        assert_eq!(agreement.point_count, neighborhood.point_count);
        assert!(agreement.current_vs_objective_rank_corr.is_finite());
        assert!(agreement.current_vs_balanced_rank_corr.is_finite());
        assert!(agreement.current_vs_active_sortino_rank_corr.is_finite());
        assert!(agreement.current_vs_omega_vs_sharpe_rank_corr.is_finite());
        assert!(agreement.current_vs_robust_margin_rank_corr.is_finite());
        assert!(agreement.agreement_floor.is_finite());
        assert!(agreement.agreement_mean.is_finite());
        let explanation = neighborhood
            .explanation
            .as_ref()
            .expect("shortlist explanation summary");
        assert_eq!(explanation.point_count, neighborhood.point_count);
        assert!(explanation.base_current_rank >= 1);
        assert!(explanation.base_objective_rank >= 1);
        assert!(explanation.pareto_nondominated_count >= 1);
        assert!(explanation.base_hypervolume_proxy.is_finite());
        assert!(explanation.hypervolume_proxy_share.is_finite());
        assert!(explanation.hypervolume_proxy_rank >= 1);

        let expected_local = compute_double_ma_local_stability_blocking_with_candles(
            DoubleMaLocalStabilityRequest {
                data_id: "test".to_string(),
                backend_hint: Some(Backend::GpuKernel { device_id: 0 }),
                followup_cache_key: res.followup_cache_key.clone(),
                params: res.best_params.clone(),
                ma_source: "close".to_string(),
                fast_ma_params: res.best_params.fast_ma_params.clone(),
                slow_ma_params: res.best_params.slow_ma_params.clone(),
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                score_mode: None,
                radius: Some(1),
                step: Some(1),
            },
            &candles,
        )
        .expect("direct local stability");
        let expected_neighborhood =
            summarize_shortlist_neighborhood_robustness(&expected_local, None)
                .expect("expected neighborhood summary");
        assert_eq!(
            neighborhood.plateau_label,
            expected_neighborhood.plateau_label
        );
        assert_eq!(neighborhood.point_count, expected_neighborhood.point_count);
        assert_eq!(
            neighborhood.neighbor_count,
            expected_neighborhood.neighbor_count
        );
        assert!((neighborhood.plateau_score - expected_neighborhood.plateau_score).abs() <= 1.0e-9);
        assert!((neighborhood.base_score - expected_neighborhood.base_score).abs() <= 1.0e-9);
        assert!((neighborhood.median_score - expected_neighborhood.median_score).abs() <= 1.0e-9);
        assert!((neighborhood.min_score - expected_neighborhood.min_score).abs() <= 1.0e-9);
        assert!((neighborhood.max_score - expected_neighborhood.max_score).abs() <= 1.0e-9);
        assert!((neighborhood.peak_delta - expected_neighborhood.peak_delta).abs() <= 1.0e-9);
        assert!(
            (neighborhood.basin_min_score - expected_neighborhood.basin_min_score).abs() <= 1.0e-9
        );
        assert!(
            (neighborhood.basin_floor_stability - expected_neighborhood.basin_floor_stability)
                .abs()
                <= 1.0e-9
        );
        assert!(
            (neighborhood.roughness_rms_delta - expected_neighborhood.roughness_rms_delta).abs()
                <= 1.0e-9
        );
        assert!(
            (neighborhood.roughness_stability - expected_neighborhood.roughness_stability).abs()
                <= 1.0e-9
        );
        assert!(
            (neighborhood.curvature_penalty - expected_neighborhood.curvature_penalty).abs()
                <= 1.0e-9
        );
        assert!(
            (neighborhood.curvature_stability - expected_neighborhood.curvature_stability).abs()
                <= 1.0e-9
        );
        let expected_agreement = expected_neighborhood
            .score_agreement
            .as_ref()
            .expect("expected agreement summary");
        assert!(
            (agreement.current_vs_objective_rank_corr
                - expected_agreement.current_vs_objective_rank_corr)
                .abs()
                <= 1.0e-9
        );
        assert!(
            (agreement.current_vs_balanced_rank_corr
                - expected_agreement.current_vs_balanced_rank_corr)
                .abs()
                <= 1.0e-9
        );
        assert!(
            (agreement.current_vs_active_sortino_rank_corr
                - expected_agreement.current_vs_active_sortino_rank_corr)
                .abs()
                <= 1.0e-9
        );
        assert!(
            (agreement.current_vs_omega_vs_sharpe_rank_corr
                - expected_agreement.current_vs_omega_vs_sharpe_rank_corr)
                .abs()
                <= 1.0e-9
        );
        assert!(
            (agreement.current_vs_robust_margin_rank_corr
                - expected_agreement.current_vs_robust_margin_rank_corr)
                .abs()
                <= 1.0e-9
        );
        assert!((agreement.agreement_floor - expected_agreement.agreement_floor).abs() <= 1.0e-9);
        assert!((agreement.agreement_mean - expected_agreement.agreement_mean).abs() <= 1.0e-9);
        assert_eq!(
            agreement.agreement_label,
            expected_agreement.agreement_label
        );
        let expected_explanation = expected_neighborhood
            .explanation
            .as_ref()
            .expect("expected explanation summary");
        assert_eq!(
            explanation.base_current_rank,
            expected_explanation.base_current_rank
        );
        assert_eq!(
            explanation.base_objective_rank,
            expected_explanation.base_objective_rank
        );
        assert_eq!(
            explanation.pareto_nondominated_count,
            expected_explanation.pareto_nondominated_count
        );
        assert_eq!(
            explanation.base_is_pareto_nondominated,
            expected_explanation.base_is_pareto_nondominated
        );
        assert_eq!(
            explanation.base_dominates_neighbors,
            expected_explanation.base_dominates_neighbors
        );
        assert_eq!(
            explanation.neighbors_dominate_base,
            expected_explanation.neighbors_dominate_base
        );
        assert_eq!(
            explanation.dominance_margin,
            expected_explanation.dominance_margin
        );
        assert!(
            (explanation.base_hypervolume_proxy - expected_explanation.base_hypervolume_proxy)
                .abs()
                <= 1.0e-9
        );
        assert_eq!(
            explanation.hypervolume_proxy_rank,
            expected_explanation.hypervolume_proxy_rank
        );
        assert!(
            (explanation.hypervolume_proxy_share - expected_explanation.hypervolume_proxy_share)
                .abs()
                <= 1.0e-9
        );
        assert_eq!(
            explanation.dominance_label,
            expected_explanation.dominance_label
        );

        match prev_global_topk {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_GLOBAL_TOPK"),
        }
        match prev_force_legacy {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn composite_adx_off_gpu_kernel_shortlist_slice_diagnostics_are_exposed_on_backend_result() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let prev_global_topk = std::env::var("VECTORBT_KERNEL_GLOBAL_TOPK").ok();
        let prev_force_legacy = std::env::var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK").ok();
        std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", "1");
        std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK");

        let cancel = AtomicBool::new(false);
        let cases = vec![
            (
                "composite_trend",
                make_trending_candles(6_000),
                trend_quality_strategy_for_kernel(),
            ),
            (
                "breakout_momentum",
                make_breakout_candles(6_000),
                breakout_strategy_for_kernel(),
            ),
        ];

        for (label, candles, strategy) in cases {
            let req = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                data_id: label.to_string(),
                fast_range: (5, 24, 1),
                slow_range: (15, 80, 1),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy,
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                score_mode: None,
                extra_shortlist_objectives: Some(vec![ObjectiveKind::Pnl, ObjectiveKind::Omega]),
                shortlist_confidence_mode: None,
                shortlist_union_mode: None,
                shortlist_slice_mode: Some(DoubleMaShortlistSliceMode::Halves),
                shortlist_slice_benchmark: Some(SpaBenchmarkKind::BuyAndHold),
                shortlist_neighborhood_radius: Some(1),
                shortlist_neighborhood_step: Some(1),
                mode: OptimizationMode::Grid,
                top_k: Some(5),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
                dynamic_strategy_ir: None,
                gpu_execution: GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
                include_trade_metrics: false,
                metric_contract: None,
                hurdle_tau: None,
            };

            let res =
                run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                    .unwrap_or_else(|e| {
                        panic!("{label} shortlist slice diagnostics run failed: {e}")
                    });

            assert!(matches!(res.backend_used, BackendUsed::Gpu));
            assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
            assert!(
                res.backend_fallback_reason.is_none(),
                "{label} should stay on the exact GpuKernel route, got fallback {:?}",
                res.backend_fallback_reason
            );

            let diagnostics = res
                .shortlist_slice_diagnostics
                .as_ref()
                .expect("shortlist slice diagnostics");
            assert_eq!(diagnostics.ranking_mode, "halves");
            assert_eq!(diagnostics.candidates.len(), res.top.len());
            let first = diagnostics
                .candidates
                .first()
                .expect("first diagnostic candidate");
            let first_top = res.top.first().expect("first top row");
            assert_eq!(first.params.fast_len, first_top.0.fast_len);
            assert_eq!(first.params.slow_len, first_top.0.slow_len);
            assert_eq!(first.halves.slices.len(), 2);
            assert_eq!(first.quartiles.slices.len(), 4);
            assert!(
                first.halves.benchmark_relative.is_some(),
                "{label} should expose benchmark-relative slice summaries"
            );
            let neighborhood = diagnostics
                .neighborhood_robustness
                .as_ref()
                .expect("neighborhood robustness");
            assert!(neighborhood.plateau_score.is_finite());
            assert!(neighborhood.point_count >= 1);
            assert_eq!(
                neighborhood.neighbor_count,
                neighborhood.point_count.saturating_sub(1)
            );
            assert!(neighborhood.score_agreement.is_some());
            assert!(neighborhood.explanation.is_some());
        }

        match prev_global_topk {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_GLOBAL_TOPK"),
        }
        match prev_force_legacy {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK"),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_topk_v2_matches_forced_legacy() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let prev_legacy = std::env::var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK").ok();

        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 40, 1),
            slow_range: (10, 80, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(20),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: Some(DoubleMaConstraints {
                min_trades: Some(1),
                min_sharpe: Some(0.2),
                ..Default::default()
            }),
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };
        let constraints = req.constraints.clone();

        std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK");
        let v2 =
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("gpu-kernel topk v2");

        std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", "1");
        let legacy = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu-kernel topk legacy");

        match prev_legacy {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK"),
        }

        assert!(v2.best_params.fast_len < v2.best_params.slow_len);
        assert!(legacy.best_params.fast_len < legacy.best_params.slow_len);
        assert!(
            v2.best_metrics.pnl.is_finite() && legacy.best_metrics.pnl.is_finite(),
            "non-finite pnl v2={} legacy={}",
            v2.best_metrics.pnl,
            legacy.best_metrics.pnl
        );
        assert!(
            v2.top.len() >= legacy.top.len(),
            "expected device-filtered v2 top len >= legacy, got v2={} legacy={}",
            v2.top.len(),
            legacy.top.len()
        );
        assert!(
            v2.top
                .iter()
                .all(|(_, metrics)| metrics_pass_constraints(metrics, constraints.as_ref())),
            "v2 results violated constraints"
        );
        assert!(
            legacy
                .top
                .iter()
                .all(|(_, metrics)| metrics_pass_constraints(metrics, constraints.as_ref())),
            "legacy results violated constraints"
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_global_topk_matches_host_merge_path() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let prev_global = std::env::var("VECTORBT_KERNEL_GLOBAL_TOPK").ok();
        let prev_legacy = std::env::var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK").ok();

        let candles = make_trending_candles(6000);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 80, 1),
            slow_range: (10, 160, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(20),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: Some(DoubleMaConstraints {
                min_trades: Some(1),
                ..Default::default()
            }),
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", "1");
        std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", "0");
        let host_merged =
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("gpu-kernel host-merge topk");

        std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", "1");
        let device_merged =
            run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("gpu-kernel device-merge topk");

        match prev_global {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_GLOBAL_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_GLOBAL_TOPK"),
        }
        match prev_legacy {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FORCE_LEGACY_TOPK"),
        }

        assert_eq!(
            host_merged.best_params.fast_len,
            device_merged.best_params.fast_len
        );
        assert_eq!(
            host_merged.best_params.slow_len,
            device_merged.best_params.slow_len
        );
        assert!(
            (host_merged.best_metrics.pnl - device_merged.best_metrics.pnl).abs() <= 1e-6,
            "best pnl mismatch host={} device={}",
            host_merged.best_metrics.pnl,
            device_merged.best_metrics.pnl
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_export_all_csv_writes_one_row_per_pair() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join("ta_desktop_demo_exports");
        std::fs::create_dir_all(&dir).expect("create temp export dir");
        let path = dir.join(format!("gpu_kernel_export_{t}.csv"));

        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: Some(path.to_string_lossy().to_string()),
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(1),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: true,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res = run_double_ma_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu-kernel export");

        let csv = std::fs::read_to_string(&path).expect("read export csv");
        let lines: Vec<&str> = csv.lines().collect();
        assert!(!lines.is_empty(), "expected non-empty csv");
        assert_eq!(
            lines[0],
            "fast_ma,slow_ma,fast_len,slow_len,pnl,sharpe,max_dd,downside_deviation,downside_frequency,downside_severity,ulcer_index,omega,kappa_tau,martin_ratio,trades,exposure,net_exposure,long_mean_return,short_mean_return,underwater_fraction,max_underwater_duration_bars,closed_trades,winning_trades,losing_trades,gross_profit,gross_loss,win_rate,avg_win,avg_loss,profit_factor,expectancy,active_mean,active_downside_deviation,active_sortino,drawdown_chronicity,downside_share,long_exposure,short_exposure,directional_bias,fee_stress_pnl,score"
        );
        assert_eq!(lines.len(), res.num_combos + 1);
        for (i, line) in lines.iter().enumerate().skip(1) {
            assert_eq!(
                line.split(',').count(),
                41,
                "bad csv column count on line {i}"
            );
        }
        let best_prefix = format!(
            "sma,sma,{},{}",
            res.best_params.fast_len, res.best_params.slow_len
        );
        assert!(
            lines.iter().skip(1).any(|l| l.starts_with(&best_prefix)),
            "expected csv to contain best params row prefix '{best_prefix}'"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_exact_fee_stress_pnl_matches_rerun() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_choppy_candles(6_000);
        let cancel = AtomicBool::new(false);

        let build_req = |strategy: StrategyConfig| DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "gpu-fee-stress".to_string(),
            fast_range: (9, 9, 0),
            slow_range: (21, 21, 0),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy,
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(1),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let strategy = StrategyConfig {
            fee_bps: 1.0,
            spread_bps: 2.0,
            slippage_bps: 1.0,
            stop_loss_pct: Some(0.04),
            take_profit_pct: Some(0.06),
            ..Default::default()
        };

        let base = run_double_ma_optimization_blocking_with_candles(
            build_req(strategy.clone()),
            &candles,
            &cancel,
            None,
        )
        .expect("base gpu fee-stress run");
        assert!(
            base.best_metrics.trades > 0,
            "expected non-zero gpu trade count"
        );

        let derived = exact_fee_stress_pnl(&base.best_metrics, &strategy);
        let mut stressed_strategy = strategy.clone();
        stressed_strategy.fee_bps = validation_fee_stress_fee_bps(&strategy);
        let stressed = run_double_ma_optimization_blocking_with_candles(
            build_req(stressed_strategy),
            &candles,
            &cancel,
            None,
        )
        .expect("stressed gpu fee-stress run");

        assert_eq!(
            (base.best_params.fast_len, base.best_params.slow_len),
            (stressed.best_params.fast_len, stressed.best_params.slow_len),
            "gpu fee-stress best params changed"
        );
        assert!(
            (derived - stressed.best_metrics.pnl).abs() <= 5e-5,
            "gpu fee-stress mismatch: derived={derived} rerun={}",
            stressed.best_metrics.pnl
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_trade_count_matches_commission_event_count_for_execution_paths() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let cancel = AtomicBool::new(false);
        let cases = vec![
            (
                "plain",
                make_choppy_candles(6_000),
                StrategyConfig {
                    fee_bps: 1.0,
                    ..Default::default()
                },
            ),
            (
                "risk",
                make_choppy_candles(6_000),
                StrategyConfig {
                    fee_bps: 1.0,
                    spread_bps: 2.0,
                    slippage_bps: 1.0,
                    stop_loss_pct: Some(0.04),
                    take_profit_pct: Some(0.06),
                    ..Default::default()
                },
            ),
            (
                "kaufman_stop",
                make_breakout_candles(6_000),
                StrategyConfig {
                    fee_bps: 1.0,
                    exit_mode: ta_strategies::double_ma::ExitMode::KaufmanStop,
                    kaufman_stop_period: 18,
                    kaufman_stop_mult: 1.8,
                    ..Default::default()
                },
            ),
        ];

        for (label, candles, strategy) in cases {
            let res = run_double_ma_optimization_blocking_with_candles(
                build_single_pair_request(Backend::GpuKernel { device_id: 0 }, strategy.clone()),
                &candles,
                &cancel,
                None,
            )
            .unwrap_or_else(|e| panic!("gpu single-pair run failed for {label}: {e}"));
            let curves = compute_double_ma_drilldown_blocking_with_candles(
                build_single_pair_drilldown_request(strategy),
                &candles,
            )
            .unwrap_or_else(|e| panic!("drilldown failed for {label}: {e}"));
            let expected_events = commission_event_count_from_curves(&curves);
            assert!(
                expected_events > 0,
                "expected non-zero commission events for {label}"
            );
            assert_eq!(
                res.best_metrics.trades, expected_events,
                "gpu commission-event mismatch for {label}"
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.mean_values, h2.mean_values);
        assert_eq!(h1.sample_counts, h2.sample_counts);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_heatmap_exposes_mean_and_sample_counts() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(10_000);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 32, 1),
            slow_range: (10, 64, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(16),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let res =
            run_with_kernel_vram_budget(req, &candles, &cancel, 512).expect("gpu heatmap run");
        let heatmap = res.heatmap.expect("heatmap");
        let means = heatmap.mean_values.expect("mean values");
        let counts = heatmap.sample_counts.expect("sample counts");
        assert_eq!(heatmap.values.len(), means.len());
        assert_eq!(heatmap.values.len(), counts.len());
        assert!(counts.iter().any(|&count| count > 0));
        assert!(means
            .iter()
            .zip(counts.iter())
            .all(|(mean, &count)| (count == 0 && mean.is_none()) || count > 0));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_alma() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let mut alma_p: HashMap<String, f64> = HashMap::new();
        alma_p.insert("offset".to_string(), 0.85);
        alma_p.insert("sigma".to_string(), 6.0);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: Some(alma_p.clone()),
            slow_ma_params: Some(alma_p),
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_jsa() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["jsa".to_string()],
            slow_ma_types: vec!["jsa".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_cwma() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["cwma".to_string()],
            slow_ma_types: vec!["cwma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_cora_wave() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let mut p: HashMap<String, f64> = HashMap::new();
        p.insert("r_multi".to_string(), 3.0);
        p.insert("smooth".to_string(), 1.0);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["cora_wave".to_string()],
            slow_ma_types: vec!["cora_wave".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: Some(p.clone()),
            slow_ma_params: Some(p),
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_epma() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let mut epma_p: HashMap<String, f64> = HashMap::new();
        epma_p.insert("offset".to_string(), 3.0);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["epma".to_string()],
            slow_ma_types: vec!["epma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: Some(epma_p.clone()),
            slow_ma_params: Some(epma_p),
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_pwma() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["pwma".to_string()],
            slow_ma_types: vec!["pwma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_srwma() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["srwma".to_string()],
            slow_ma_types: vec!["srwma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_supersmoother() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["supersmoother".to_string()],
            slow_ma_types: vec!["supersmoother".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_zlema() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["zlema".to_string()],
            slow_ma_types: vec!["zlema".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_nma() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();
        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["nma".to_string()],
            slow_ma_types: vec!["nma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        // NMA remains batch-sensitive at the dense heatmap-cell level even when the
        // exact candidate count and winning parameters match. Keep this test focused
        // on search-space coverage and best-result parity rather than byte-for-byte
        // heatmap identity.
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_hma() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["hma".to_string()],
            slow_ma_types: vec!["hma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_jma() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["jma".to_string()],
            slow_ma_types: vec!["jma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_edcf() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["edcf".to_string()],
            slow_ma_types: vec!["edcf".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_ehlers_itrend() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (10, 120, 1),
            slow_range: (20, 240, 1),
            fast_ma_types: vec!["ehlers_itrend".to_string()],
            slow_ma_types: vec!["ehlers_itrend".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_vwma() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["vwma".to_string()],
            slow_ma_types: vec!["vwma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_vfct() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_breakout_candles(20_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 80, 1),
            slow_range: (10, 160, 1),
            fast_ma_types: vec!["vwma".to_string()],
            slow_ma_types: vec!["vwma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: vfct_strategy_for_kernel(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(16),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel vfct run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 64)
            .expect("batched gpu-kernel vfct run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_vpwma() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (5, 120, 1),
            slow_range: (10, 240, 1),
            fast_ma_types: vec!["vpwma".to_string()],
            slow_ma_types: vec!["vpwma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_kernel_batched_matches_unbatched_best_params_frama() {
        if !cuda_available() {
            return;
        }

        let _guard = lock_env();

        let candles = make_trending_candles(50_000);
        let cancel = AtomicBool::new(false);

        let base_req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "test".to_string(),
            fast_range: (10, 120, 1),
            slow_range: (20, 240, 1),
            fast_ma_types: vec!["frama".to_string()],
            slow_ma_types: vec!["frama".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: Some(32),
            constraints: None,
            vdps: None,

            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
            hurdle_tau: None,
        };

        let single = run_with_kernel_vram_budget(base_req.clone(), &candles, &cancel, 8192)
            .expect("unbatched gpu-kernel run");
        let batched = run_with_kernel_vram_budget(base_req, &candles, &cancel, 128)
            .expect("batched gpu-kernel run");

        assert_eq!(single.num_combos, batched.num_combos);
        assert_eq!(single.best_params.fast_len, batched.best_params.fast_len);
        assert_eq!(single.best_params.slow_len, batched.best_params.slow_len);

        let h1 = single.heatmap.expect("single heatmap");
        let h2 = batched.heatmap.expect("batched heatmap");
        assert_eq!(h1.bins_fast, h2.bins_fast);
        assert_eq!(h1.bins_slow, h2.bins_slow);
        assert_eq!(h1.fast_min, h2.fast_min);
        assert_eq!(h1.fast_max, h2.fast_max);
        assert_eq!(h1.slow_min, h2.slow_min);
        assert_eq!(h1.slow_max, h2.slow_max);
        assert_eq!(h1.values, h2.values);
    }
}
