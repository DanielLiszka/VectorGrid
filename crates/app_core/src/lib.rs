pub(crate) mod alloc_policy;
pub mod asr;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod asr_gpu_kernel;
pub mod carm;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod carm_gpu_kernel;
pub mod cdmc;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod cdmc_gpu_kernel;
pub mod composite_cpu;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) mod composite_gpu_metric_decode;
pub mod composite_mo;
pub(crate) mod composite_profile_route;
pub mod composite_validate;
pub mod cscv_pbo;
pub mod csv_compat;
pub mod cuda_device;
pub mod double_ma;
pub(crate) mod double_ma_backend;
pub(crate) mod double_ma_ma_kind;
pub(crate) mod double_ma_mode;
pub(crate) mod double_ma_result;
pub(crate) mod double_ma_strategy;
pub mod double_ma_validate;
pub(crate) mod double_ma_vdps;
pub mod dynamic_indicator_adapter;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod gpu_backtest_kernel;
pub mod indicator_catalog;
pub mod indicator_factory;
pub mod iterative_sim;
pub mod kqsr;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod kqsr_gpu_kernel;
pub mod kvdf;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod kvdf_gpu_kernel;
pub mod ma_policy;
pub mod matrix_index;
pub mod meta_select;
pub mod mkcs;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod mkcs_gpu_kernel;
pub mod ml_confidence;
pub mod ml_signal;
pub mod nvtx;
pub(crate) mod prescan_predictive;
pub mod progress;
pub(crate) mod request_normalization;
pub mod rocb;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod rocb_gpu_kernel;
pub mod sharpe_stats;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) mod standalone_gpu_runtime;
pub mod stats_common;
pub mod strategy_dynamic;
pub mod strategy_gpu_program;
pub mod strategy_gpu_runtime;
pub mod strategy_ir;
pub mod strategy_plan;
pub mod strategy_presets;
#[cfg(test)]
pub(crate) mod test_support;
pub mod timeframe;
pub(crate) mod validation_core;
pub mod validation_pipeline;
pub mod validation_shared;
pub mod validation_stats;
pub mod vcc;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod vcc_gpu_kernel;
pub mod vcet;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod vcet_gpu_kernel;
pub mod vemr;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod vemr_gpu_kernel;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) mod vram_ma;
pub use ta_strategies::composite_cpu::CompositeSignalMode;

pub use ta_strategies::double_ma::DoubleMaCurves;
pub use ta_strategies::records::{TradeRecord, TradeRecords};

pub use csv_compat::read_candles_from_csv_compat;
pub use cuda_device::{preferred_cuda_device, preferred_cuda_device_id, PreferredCudaDevice};
pub use timeframe::{parse_timeframe_label, resample_candles, TimeframeSpec};

pub use double_ma::{
    compute_double_ma_drilldown_blocking_with_candles,
    compute_double_ma_local_stability_blocking_with_candles, reset_dynamic_gpu_preflight_cache,
    run_double_ma_optimization_blocking_with_candles,
    run_double_ma_optimization_blocking_with_candles_and_eval_range, Backend,
    BackendOptimizationResult, BackendUsed, DoubleMaCampaignVramPrefetchOutcome,
    DoubleMaDrilldownRequest, DoubleMaLocalStabilityRequest, DoubleMaParamsResolved,
    DoubleMaRequest, LocalStabilityResult, OptimizationModeResolved,
    stage_double_ma_campaign_vram_prefetch,
};

pub use composite_cpu::{
    compute_composite_cpu_drilldown_blocking_with_candles,
    compute_composite_cpu_local_stability_blocking_with_candles,
    run_composite_cpu_optimization_blocking_with_candles, CompositeCpuDrilldownRequest,
    CompositeCpuDrilldownResult, CompositeCpuLocalStabilityRequest, CompositeCpuRequest,
};

pub use asr::AsrStrategyParams;
pub use carm::CarmStrategyParams;
pub use cdmc::CdmcStrategyParams;
pub use kqsr::KqsrStrategyParams;
pub use kvdf::KvdfStrategyParams;
pub use mkcs::MkcsStrategyParams;
pub use rocb::RocbStrategyParams;
pub use vcc::VccStrategyParams;
pub use vcet::VcetStrategyParams;
pub use vemr::VemrStrategyParams;

pub use composite_mo::{
    run_composite_mo_optimization_blocking_with_candles, CompositeConstraints,
    CompositeGpuPrescanConfig, CompositeGpuPrescanObjectives, CompositeMoObjectives,
    CompositeMoParams, CompositeMoPoint, CompositeMoRequest, CompositeMoResult, CompositeMoSampler,
    CompositePrescanMaPair,
};

pub use composite_validate::{
    run_composite_validation_blocking_with_candles,
    run_composite_validation_blocking_with_candles_with_aux_mask, CompositeParams,
    CompositeValidationRequest, CompositeValidationResult,
};

pub use double_ma_validate::{
    run_double_ma_validation_blocking_with_candles,
    run_double_ma_validation_blocking_with_candles_with_aux_mask, DoubleMaValidationParams,
    DoubleMaValidationRequest, DoubleMaValidationResult, ValidationExecutionProfile,
};

pub use validation_shared::{
    CrossTickerConfig, CrossTickerDataset, CrossTickerDatasetMeta, CrossTickerGateConfig,
    CrossTickerMode, CrossTickerStats, CrossTickerValidationSummary, PerturbConfig, PerturbStats,
    ReoptWalkForwardConfigOptional, ReoptWalkForwardStats, ReoptWalkForwardWindowStats,
    SpaBenchmarkKind, SpaConfigOptional, SpaValidationStats, TradeBootstrapConfig,
    TradeBootstrapStats, ValidationArtifactContext, ValidationAuxWorkMask, WalkForwardConfig,
    WalkForwardConfigOptional, WalkForwardStats,
};

pub use iterative_sim::{
    IntrabarRule, IterativeMetricsDelta, IterativeSimConfig, ValidationEngine,
};

pub use cscv_pbo::{CscvCandidatePboStats, CscvPboConfig, CscvPboResult};
pub use dynamic_indicator_adapter::{
    compute_dynamic_indicator_cpu, DynamicIndicatorAdapterError, DynamicIndicatorComputeOutput,
    DynamicIndicatorSeries,
};
#[cfg(feature = "cuda")]
pub use dynamic_indicator_adapter::{
    compute_dynamic_indicator_cuda, compute_dynamic_indicator_cuda_with_inputs,
    DynamicIndicatorCudaComputeOutput, DynamicIndicatorCudaSeries, PreparedCudaIndicatorInputs,
};
pub use indicator_catalog::{
    find_dynamic_indicator_descriptor, list_dynamic_indicator_descriptors,
    list_dynamic_indicator_descriptors_cached, normalize_and_validate_indicator_output_ref,
    normalize_and_validate_indicator_params, IndicatorDescriptor, IndicatorInputKind,
    IndicatorOutputDescriptor, IndicatorParamDescriptor, IndicatorParamKV, IndicatorParamKind,
    IndicatorParamValue, IndicatorValueType,
};

pub use meta_select::{
    meta_group_key, meta_group_key_for_request, meta_select_from_artifacts,
    refresh_meta_group_evidence_flags, HoldoutAggregate, HoldoutDatasetRow, HoldoutSummary,
    MetaCandidateAggregate, MetaClusterConfig, MetaGroupingMode, MetaHoldoutMode,
    MetaHoldoutValidationConfig, MetaScoreWeights, MetaSelectionRequest, MetaSelectionResult,
    TrainDatasetRow,
};

pub use ml_confidence::{
    extract_ml_confidence_group_features, predict_ml_confidence_for_features,
    predict_ml_confidence_for_group, prepare_ml_confidence_rows_from_meta_artifacts,
    train_ml_confidence_model_from_meta_artifacts, train_ml_confidence_model_from_prepared_rows,
    train_ml_confidence_model_from_rows, MlConfidenceCalibrationBin, MlConfidenceDatasetRow,
    MlConfidenceGroupFeatures, MlConfidenceLabelConfig, MlConfidenceModel, MlConfidenceModelFamily,
    MlConfidenceModelKind, MlConfidenceModelSummary, MlConfidencePreparedArtifactRows,
    MlConfidencePreparedRows, MlConfidenceScope, MlConfidenceTarget, MlConfidenceTrainConfig,
    MlConfidenceTrainMetrics,
};

pub use ml_signal::{
    assign_ml_signal_ticker_holdout, build_ml_signal_score_cache_from_csv,
    build_ml_signal_split_windows, classify_ml_signal_manifest,
    export_ml_signal_dataset_with_candles, list_ml_signal_model_bundle_summaries,
    read_ml_signal_manifest, run_ml_signal_policy_sweep_blocking_with_candles,
    run_ml_signal_validation_blocking_with_candles, MlSignalDatasetExportManifest,
    MlSignalDatasetExportRequest, MlSignalDatasetExportResult, MlSignalFeatureSchema,
    MlSignalModelBundleManifest, MlSignalModelBundleStatus, MlSignalModelBundleSummary,
    MlSignalPolicyParams, MlSignalPolicySweepRequest, MlSignalPolicySweepResult,
    MlSignalScoreCacheBuildRequest, MlSignalScoreCacheBuildResult, MlSignalScoreCacheManifest,
    MlSignalSplitPlan, MlSignalSplitWindow, MlSignalSweepAxisF64, MlSignalSweepAxisUsize,
    MlSignalTickerHoldoutConfig, MlSignalValidationRequest, MlSignalValidationResult,
};

pub use sharpe_stats::{
    deflated_sharpe_ratio, expected_max_sharpe, mean_std, probabilistic_sharpe_ratio, SharpeStats,
};

pub use strategy_ir::{
    assess_strategy_ir_schema_compatibility, build_static_strategy_ir, validate_strategy_ir,
    ConditionGroup, ConditionGroupMode, ConditionNode, ConditionOp, ExecutionPolicy, IndicatorKind,
    IndicatorNode, MovingAverageSpec, SignalBindings, SignalProgramGraph, StaticStrategyIrInput,
    StrategyIr, StrategyIrSchemaCompatibility, StrategyProfileKind, ValueRef, IR_MAX_CONDITIONS,
    IR_MAX_GROUPS, IR_MAX_GROUP_CONDITIONS, IR_MAX_INDICATORS,
    STRATEGY_IR_MIN_SUPPORTED_SCHEMA_VERSION, STRATEGY_IR_SCHEMA_VERSION,
};

pub use strategy_gpu_program::{
    compile_gpu_predicate_program, compile_gpu_predicate_program_abi, CompiledGpuPredicateProgram,
    CompiledGpuPredicateProgramAbi, GpuConditionAbi, GpuConditionGroupAbi, GpuConditionGroupSlice,
    GpuConditionPredicate, GpuIndicatorSlot, GpuIndicatorSlotAbi, GpuSignalBindings,
    GpuSignalBindingsAbi, GpuValueOperand, GpuValueOperandAbi, GpuValueSourceKind,
    STRATEGY_GPU_EXECUTABLE_ABI_VERSION, STRATEGY_GPU_PROGRAM_SCHEMA_VERSION,
};

pub use strategy_gpu_runtime::{
    get_or_build_gpu_strategy_executable, gpu_strategy_executable_cache_stats,
    reset_gpu_strategy_executable_cache, GpuStrategyExecutable, GpuStrategyExecutableCacheKey,
    StrategyCapabilityReport, StrategyExecutableCacheStats, StrategyHandle,
};

pub use strategy_plan::{
    compile_strategy_plan, CompiledConditionGroup, CompiledConditionOp, CompiledSignalBindings,
    CompiledStrategyPlan, CompiledValueRef, IndicatorComputePlan, IndicatorRouteKind,
    StrategyPlanBackend, STRATEGY_PLAN_SCHEMA_VERSION,
};

pub use strategy_dynamic::{
    compile_dynamic_plan_for_composite_cpu_config, compile_dynamic_plan_for_double_ma_config,
    eval_dynamic_plan_metrics_cpu, prepare_dynamic_cpu_context,
    prepare_dynamic_indicator_slot_series, prepare_dynamic_indicator_slot_series_cpu,
    prepare_dynamic_indicator_slot_series_cpu_with_controls,
    prepare_dynamic_indicator_slot_series_with_controls, preview_dynamic_double_ma_strategy_ir,
    DynamicDoubleMaStrategyIrPreview, DynamicIndicatorStagingProgressFn, PreparedDynamicCpuContext,
};

#[cfg(feature = "cuda")]
pub use dynamic_indicator_adapter::take_prepared_cuda_dispatch_stats;

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub use gpu_backtest_kernel::{
    eval_double_ma_dynamic_program_precomputed_gpu_kernel,
    eval_uploaded_double_ma_dynamic_program_gpu_kernel, upload_double_ma_dynamic_program_batch,
    UploadedDoubleMaDynamicProgramBatch,
};
