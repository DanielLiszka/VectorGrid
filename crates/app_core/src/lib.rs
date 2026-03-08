pub(crate) mod alloc_policy;
pub mod asr;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod asr_gpu_kernel;
pub mod composite_cpu;
pub mod composite_mo;
pub mod composite_validate;
pub mod cscv_pbo;
pub mod csv_compat;
pub mod double_ma;
pub mod double_ma_validate;
pub mod dynamic_indicator_adapter;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod gpu_backtest_kernel;
pub mod indicator_catalog;
pub mod indicator_factory;
pub mod iterative_sim;
pub mod ma_policy;
pub mod matrix_index;
pub mod meta_select;
pub mod ml_confidence;
pub(crate) mod prescan_predictive;
pub mod progress;
pub(crate) mod request_normalization;
pub mod sharpe_stats;
pub mod stats_common;
pub mod strategy_dynamic;
pub mod strategy_gpu_program;
pub mod strategy_ir;
pub mod strategy_plan;
pub mod timeframe;
pub(crate) mod validation_core;
pub mod validation_pipeline;
pub mod validation_stats;
pub mod vcc;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod vcc_gpu_kernel;
pub mod vemr;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod vemr_gpu_kernel;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) mod vram_ma;
pub use ta_strategies::composite_cpu::CompositeSignalMode;

pub use ta_strategies::double_ma::DoubleMaCurves;
pub use ta_strategies::records::{TradeRecord, TradeRecords};

pub use csv_compat::read_candles_from_csv_compat;
pub use timeframe::{parse_timeframe_label, resample_candles, TimeframeSpec};

pub use double_ma::{
    compute_double_ma_drilldown_blocking_with_candles,
    compute_double_ma_local_stability_blocking_with_candles,
    run_double_ma_optimization_blocking_with_candles, Backend, BackendOptimizationResult,
    BackendUsed, DoubleMaDrilldownRequest, DoubleMaLocalStabilityRequest, DoubleMaParamsResolved,
    DoubleMaRequest, LocalStabilityResult, OptimizationModeResolved,
};

pub use composite_cpu::{
    compute_composite_cpu_drilldown_blocking_with_candles,
    compute_composite_cpu_local_stability_blocking_with_candles,
    run_composite_cpu_optimization_blocking_with_candles, CompositeCpuDrilldownRequest,
    CompositeCpuDrilldownResult, CompositeCpuLocalStabilityRequest, CompositeCpuRequest,
};

pub use asr::AsrStrategyParams;
pub use vcc::VccStrategyParams;
pub use vemr::VemrStrategyParams;

pub use composite_mo::{
    run_composite_mo_optimization_blocking_with_candles, CompositeConstraints,
    CompositeGpuPrescanConfig, CompositeGpuPrescanObjectives, CompositeMoObjectives,
    CompositeMoParams, CompositeMoPoint, CompositeMoRequest, CompositeMoResult, CompositeMoSampler,
    CompositePrescanMaPair,
};

pub use composite_validate::{
    run_composite_validation_blocking_with_candles, CompositeParams, CompositeValidationRequest,
    CompositeValidationResult, CrossTickerConfig, CrossTickerDataset, CrossTickerDatasetMeta,
    CrossTickerGateConfig, CrossTickerStats, CrossTickerValidationSummary, PerturbConfig,
    TradeBootstrapConfig, WalkForwardConfigOptional,
};

pub use double_ma_validate::{
    run_double_ma_validation_blocking_with_candles, DoubleMaValidationParams,
    DoubleMaValidationRequest, DoubleMaValidationResult,
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
    meta_group_key, meta_group_key_for_request, meta_select_from_artifacts, HoldoutAggregate,
    HoldoutDatasetRow, HoldoutSummary, MetaCandidateAggregate, MetaClusterConfig, MetaGroupingMode,
    MetaHoldoutMode, MetaHoldoutValidationConfig, MetaScoreWeights, MetaSelectionRequest,
    MetaSelectionResult, TrainDatasetRow,
};

pub use ml_confidence::{
    predict_ml_confidence_for_group, train_ml_confidence_model_from_meta_artifacts,
    MlConfidenceCalibrationBin, MlConfidenceLabelConfig, MlConfidenceModel,
    MlConfidenceModelFamily, MlConfidenceModelKind, MlConfidenceScope, MlConfidenceTrainConfig,
    MlConfidenceTrainMetrics,
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
    compile_gpu_predicate_program, CompiledGpuPredicateProgram, GpuConditionGroupSlice,
    GpuConditionPredicate, GpuIndicatorSlot, GpuSignalBindings, GpuValueOperand,
    GpuValueSourceKind, STRATEGY_GPU_PROGRAM_SCHEMA_VERSION,
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
    prepare_dynamic_indicator_slot_series_with_controls, DynamicIndicatorStagingProgressFn,
    PreparedDynamicCpuContext,
};
