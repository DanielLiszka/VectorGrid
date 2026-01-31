pub mod composite_cpu;
pub mod composite_validate;
pub mod composite_mo;
pub mod cscv_pbo;
pub mod double_ma;
pub mod double_ma_validate;
pub mod indicator_factory;
pub mod matrix_index;
pub mod meta_select;
pub mod ml_confidence;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod gpu_backtest_kernel;
pub mod progress;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) mod vram_ma;

pub use ta_strategies::double_ma::DoubleMaCurves;
pub use ta_strategies::records::{TradeRecord, TradeRecords};

pub use double_ma::{
    compute_double_ma_drilldown_blocking_with_candles,
    compute_double_ma_local_stability_blocking_with_candles,
    run_double_ma_optimization_blocking_with_candles, Backend, BackendOptimizationResult,
    BackendUsed, DoubleMaDrilldownRequest, DoubleMaParamsResolved, DoubleMaRequest,
    DoubleMaLocalStabilityRequest, LocalStabilityResult,
    OptimizationModeResolved,
};

pub use composite_cpu::{
    compute_composite_cpu_drilldown_blocking_with_candles,
    compute_composite_cpu_local_stability_blocking_with_candles,
    run_composite_cpu_optimization_blocking_with_candles, CompositeCpuDrilldownRequest,
    CompositeCpuLocalStabilityRequest, CompositeCpuRequest,
};

pub use composite_mo::{
    run_composite_mo_optimization_blocking_with_candles, CompositeConstraints,
    CompositeGpuPrescanConfig, CompositeGpuPrescanObjectives, CompositeMoObjectives,
    CompositeMoParams, CompositeMoPoint, CompositeMoRequest, CompositeMoResult, CompositeMoSampler,
    CompositePrescanMaPair,
};

pub use composite_validate::{
    run_composite_validation_blocking_with_candles, CompositeParams, CompositeValidationRequest,
    CompositeValidationResult, CrossTickerConfig, CrossTickerDataset, CrossTickerDatasetMeta,
    CrossTickerStats, CrossTickerValidationSummary, PerturbConfig, TradeBootstrapConfig,
    WalkForwardConfigOptional,
};

pub use double_ma_validate::{
    run_double_ma_validation_blocking_with_candles, DoubleMaValidationParams,
    DoubleMaValidationRequest, DoubleMaValidationResult,
};

pub use cscv_pbo::{CscvCandidatePboStats, CscvPboConfig, CscvPboResult};

pub use meta_select::{
    meta_group_key, meta_group_key_for_request, meta_select_from_artifacts, HoldoutAggregate,
    HoldoutDatasetRow, HoldoutSummary, MetaCandidateAggregate, MetaClusterConfig, MetaGroupingMode,
    MetaScoreWeights, MetaHoldoutMode, MetaHoldoutValidationConfig, MetaSelectionRequest,
    MetaSelectionResult,
};

pub use ml_confidence::{
    predict_ml_confidence_for_group, train_ml_confidence_model_from_meta_artifacts,
    MlConfidenceCalibrationBin, MlConfidenceLabelConfig, MlConfidenceModel, MlConfidenceScope,
    MlConfidenceTrainConfig, MlConfidenceTrainMetrics,
};
