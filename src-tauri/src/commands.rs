#[path = "commands_analysis.rs"]
mod commands_analysis;
#[path = "commands_artifacts.rs"]
mod commands_artifacts;
#[path = "commands_campaign.rs"]
mod commands_campaign;
#[path = "commands_data.rs"]
mod commands_data;
#[path = "commands_dynamic.rs"]
mod commands_dynamic;
#[path = "commands_execution.rs"]
mod commands_execution;
#[path = "commands_meta.rs"]
mod commands_meta;
#[path = "commands_ml_signal.rs"]
mod commands_ml_signal;
#[path = "commands_optimize.rs"]
mod commands_optimize;
#[path = "commands_reports.rs"]
mod commands_reports;
#[path = "commands_reports_api.rs"]
mod commands_reports_api;
#[path = "commands_system.rs"]
mod commands_system;
#[path = "commands_validation.rs"]
mod commands_validation;
#[path = "metric_surface.rs"]
mod metric_surface;
#[path = "run_artifacts.rs"]
mod run_artifacts;
#[path = "validation_common.rs"]
mod validation_common;

pub use commands_analysis::{
    compute_composite_cpu_drilldown, compute_composite_cpu_local_stability,
    compute_double_ma_drilldown, compute_double_ma_local_stability,
};
pub use commands_artifacts::{
    delete_run_artifact, list_artifact_library, list_meta_artifacts, list_ml_policy_eval_artifacts,
    list_run_artifacts, list_stage4_artifacts, list_stage5_artifacts, load_meta_artifact_replay,
    load_ml_policy_eval_artifact_replay, load_run_artifact, load_run_artifact_detail,
    load_run_artifact_hydration, load_stage5_artifact_replay, open_runs_dir, save_batch_summary,
    save_run_artifact, update_artifact_tags,
};
pub use commands_campaign::{
    preload_research_campaign_dataset_pack_headless, run_research_campaign,
    run_research_campaign_blocking_headless, run_research_campaign_optimize_only_blocking_headless,
    run_research_campaign_optimize_only_preloaded_pack_blocking_headless,
    HeadlessPreparedResearchCampaignDatasetPack, ResearchCampaignDatasetInput,
    ResearchCampaignExecutionMode, ResearchCampaignPreloadPolicy, ResearchCampaignRequest,
    ResearchCampaignValidationConfig, RunResearchCampaignResponse,
};
pub use commands_data::{
    clear_loaded_data, clear_market_data_dir, get_market_data_dir_info, list_dynamic_indicators,
    list_gpu_speedups, list_moving_averages, list_repo_data_csvs, list_sample_csvs,
    load_price_data, pick_csv_file, pick_folder, pick_save_csv, polygon_clear_api_key,
    polygon_download_1m_and_load, polygon_get_cache_info, polygon_live_candle_stream_status,
    polygon_repair_dataset, polygon_set_api_key, polygon_start_live_candle_stream,
    polygon_stop_live_candle_stream, polygon_validate_dataset, resample_price_data,
    set_market_data_dir, ClearLoadedDataResponse, GpuSpeedupEntry, MarketDataDirInfo,
    PolygonDownloadAndLoadResponse, PolygonLiveCandleStreamStatusResponse,
    PolygonStartLiveCandleStreamRequest, PolygonStartLiveCandleStreamResponse,
    PolygonStopLiveCandleStreamResponse, RepoDataCsvEntry, RepoDataCsvList, SampleCsv,
};
pub use commands_dynamic::{
    encode_dynamic_strategy_constant_bits, format_dynamic_strategy_ir_json,
    preview_dynamic_double_ma_strategy_ir_json, DynamicDoubleMaStrategyIrJsonPreviewResponse,
    DynamicStrategyConstantBitsResponse,
};
pub use commands_execution::{
    execute_strategy_signal_once, get_live_execution_snapshot, live_execution_runner_status,
    start_live_execution_runner, stop_live_execution_runner, test_alpaca_connection,
    ExecuteStrategySignalRequest, ExecuteStrategySignalResponse, GetLiveExecutionSnapshotRequest,
    GetLiveExecutionSnapshotResponse, LiveExecutionMode, LiveExecutionRunnerConfig,
    LiveExecutionRunnerStatus, LiveOrderSnapshot, LivePositionSnapshot, LiveRiskOverride,
    LiveStrategySpec, StopLiveExecutionRunnerResponse, TestAlpacaConnectionRequest,
    TestAlpacaConnectionResponse,
};
pub use commands_meta::{
    apply_ml_confidence_model, delete_ml_confidence_model, evaluate_ml_confidence_policy,
    ml_confidence_model_status, run_meta_selection, run_meta_stage4_experimental,
    run_meta_stage5_ml_confidence, train_ml_confidence_model, ApplyMlConfidenceResponse,
    MetaStage4Request, MetaStage4Result, MetaStage5Request, MetaStage5Result,
    MlConfidenceModelState, MlConfidenceModelStatus, MlConfidencePolicyArmMetrics,
    MlConfidencePolicyEvaluationRequest, MlConfidencePolicyEvaluationResult,
    RunMetaSelectionResponse, RunMetaStage4Response, RunMetaStage5Response,
    RunMlConfidencePolicyEvaluationResponse, TrainMlConfidenceResponse,
};
pub use commands_ml_signal::{
    export_ml_signal_dataset, generate_ml_signal_score_cache, list_ml_signal_model_bundles,
    ml_signal_model_status, run_ml_signal_policy_sweep, run_ml_signal_validation,
    MlSignalModelStatusResponse,
};
pub use commands_optimize::{
    cancel_background_task, cancel_current_run, cancel_double_ma_optimization,
    run_composite_cpu_optimization, run_composite_mo_optimization, run_double_ma_optimization,
};
pub(crate) use commands_optimize::{OffsetProgressSink, TauriProgressSink};
pub use commands_reports::build_report_xlsx_bytes_headless;
pub(crate) use commands_reports::{
    build_meta_report_xlsx_bytes, quantile, reveal_path_blocking, sanitize_filename_component,
};
pub use commands_reports_api::{
    build_report_xlsx, build_validation_shortlist_csv, open_path, open_reports_dir, reveal_path,
    save_report_xlsx, save_report_xlsx_from_artifact, SaveReportXlsxResponse,
};
pub use commands_system::{get_gpu_status, set_window_effect, GpuStatus};
#[allow(unused_imports)]
pub(crate) use commands_validation::apply_cross_ticker_validation;
pub use commands_validation::{
    apply_cross_ticker_validation_double_ma_headless, get_price_bars, run_composite_validation,
    run_double_ma_validation, PriceBar,
};
pub use run_artifacts::{
    ArtifactLibrarySnapshot, MetaArtifactReplayResponse, MetaArtifactSummary,
    MlPolicyEvalArtifactReplayResponse, MlPolicyEvalArtifactSummary, RunArtifactDetailResponse,
    RunArtifactSummary, SaveBatchSummaryResponse, SaveRunArtifactResponse, Stage4ArtifactSummary,
    Stage5ArtifactReplayResponse, Stage5ArtifactSummary, WorkflowArtifactsHydrationPayload,
};

pub use ta_app_core::{
    Backend, BackendOptimizationResult, BackendUsed, CompositeCpuDrilldownRequest,
    CompositeCpuDrilldownResult, CompositeCpuLocalStabilityRequest, CompositeCpuRequest,
    CompositeMoRequest, CompositeMoResult, CompositeValidationRequest, CompositeValidationResult,
    DoubleMaCurves, DoubleMaDrilldownRequest, DoubleMaLocalStabilityRequest,
    DoubleMaParamsResolved, DoubleMaRequest, DoubleMaValidationRequest, DoubleMaValidationResult,
    LocalStabilityResult, MetaSelectionRequest, MetaSelectionResult, MlConfidenceLabelConfig,
    MlConfidenceModel, MlConfidenceScope, MlConfidenceTrainConfig, MlConfidenceTrainMetrics,
    MlSignalDatasetExportRequest, MlSignalDatasetExportResult, MlSignalModelBundleManifest,
    MlSignalModelBundleStatus, MlSignalModelBundleSummary, MlSignalPolicyParams,
    MlSignalPolicySweepRequest, MlSignalPolicySweepResult, MlSignalScoreCacheBuildRequest,
    MlSignalScoreCacheBuildResult, MlSignalValidationRequest, MlSignalValidationResult,
    OptimizationModeResolved,
};
