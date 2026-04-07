use serde::{Deserialize, Serialize};

pub const OPTIONS_DATASET_MANIFEST_VERSION: u32 = 1;
pub const OPTIONS_SIGNAL_MANIFEST_VERSION: u32 = 1;
pub const OPTIONS_EXPRESSION_ARTIFACT_VERSION: u32 = 1;

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_quote_resolution() -> String {
    "1m".to_string()
}

fn default_provider_name() -> String {
    "massive".to_string()
}

fn default_provider_source_surface() -> String {
    "minute_aggregate".to_string()
}

fn default_contract_reference_source() -> String {
    "massive_contract_reference".to_string()
}

fn default_underlier_reference_source() -> String {
    "underlying_dataset".to_string()
}

fn default_calendar_id() -> String {
    "market_hours_utc".to_string()
}

fn default_root_symbol() -> String {
    String::new()
}

fn default_delay_minutes() -> Vec<u16> {
    vec![0, 1, 5]
}

fn default_dte_buckets() -> Vec<u16> {
    vec![1, 3, 5]
}

fn default_moneyness_buckets() -> Vec<OptionsMoneynessBucket> {
    vec![
        OptionsMoneynessBucket::Atm,
        OptionsMoneynessBucket::Otm1,
        OptionsMoneynessBucket::Otm2,
        OptionsMoneynessBucket::Itm1,
    ]
}

fn default_hold_minutes() -> Vec<u16> {
    vec![5, 15, 30, 60]
}

fn default_target_buckets() -> Vec<f64> {
    vec![0.25, 0.50, 1.0]
}

fn default_stop_buckets() -> Vec<f64> {
    vec![0.25, 0.50]
}

fn default_top_k() -> usize {
    25
}

fn default_exit_template() -> OptionsExitTemplateFamily {
    OptionsExitTemplateFamily::FixedHoldOrTargetStop
}

fn default_fill_model() -> OptionsFillModelFamily {
    OptionsFillModelFamily::AggregateResearch
}

fn default_backend_preference() -> OptionsExpressionBackendPreference {
    OptionsExpressionBackendPreference::Auto
}

fn default_split_ratio_search() -> f64 {
    0.60
}

fn default_split_ratio_holdout() -> f64 {
    0.20
}

fn default_sealed_finalist_count() -> usize {
    5
}

fn default_split_assignment_mode() -> OptionsSplitAssignmentMode {
    OptionsSplitAssignmentMode::EventChronological
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OptionsPriceSemantics {
    MinuteAggregate,
    TopOfBookQuote,
    DerivedMidSnapshot,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OptionsFillModelFamily {
    AggregateResearch,
    QuoteConservative,
    QuoteMid,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OptionsExpressionBackendPreference {
    Auto,
    CpuReference,
    GpuCompiledTensor,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OptionsSplitAssignmentMode {
    EventChronological,
    SessionChronological,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OptionType {
    Call,
    Put,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SignalSide {
    Bullish,
    Bearish,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum OptionsMoneynessBucket {
    Atm,
    Otm1,
    Otm2,
    Itm1,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OptionsExitTemplateFamily {
    FixedHold,
    FixedHoldOrTargetStop,
    EndOfDayOrTargetStop,
    BaseExitOrTargetStop,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OptionsEvaluationSplit {
    Search,
    Holdout,
    SealedTest,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum OptionsCandidateInvalidReason {
    NoPriceRowAfterEntry,
    NoExpirationInBucket,
    NoStrikeInTolerance,
    SpreadTooWide,
    PremiumTooHigh,
    PremiumTooLow,
    InsufficientVolume,
    InsufficientOpenInterest,
    EntryPastCutoff,
    InsufficientFuturePath,
    StaleOrMissingPath,
    ContractNotEligibleAsOfEvent,
    IncompatiblePriceSemantics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsCandidateInvalidReasonCount {
    pub reason: OptionsCandidateInvalidReason,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsDelayBucketCoverage {
    pub delay_minutes: u16,
    pub slot_count: usize,
    pub valid_slot_count: usize,
    pub invalid_slot_count: usize,
    pub valid_rate: f64,
    #[serde(default)]
    pub invalid_reasons: Vec<OptionsCandidateInvalidReasonCount>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsDteBucketCoverage {
    pub dte_days: u16,
    pub slot_count: usize,
    pub valid_slot_count: usize,
    pub invalid_slot_count: usize,
    pub valid_rate: f64,
    #[serde(default)]
    pub invalid_reasons: Vec<OptionsCandidateInvalidReasonCount>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsMoneynessBucketCoverage {
    pub moneyness_bucket: OptionsMoneynessBucket,
    pub slot_count: usize,
    pub valid_slot_count: usize,
    pub invalid_slot_count: usize,
    pub valid_rate: f64,
    #[serde(default)]
    pub invalid_reasons: Vec<OptionsCandidateInvalidReasonCount>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsCandidateAxisCoverage {
    #[serde(default)]
    pub delay_minutes: Vec<OptionsDelayBucketCoverage>,
    #[serde(default)]
    pub dte_days: Vec<OptionsDteBucketCoverage>,
    #[serde(default)]
    pub moneyness_buckets: Vec<OptionsMoneynessBucketCoverage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsStaticFilterBundle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_entry_lag_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_premium: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_premium: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_volume: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_open_interest: Option<f64>,
}

impl Default for OptionsStaticFilterBundle {
    fn default() -> Self {
        Self {
            max_entry_lag_minutes: Some(5),
            max_premium: None,
            min_premium: None,
            min_volume: None,
            min_open_interest: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsDatasetManifestV1 {
    pub version: u32,
    pub options_dataset_id: String,
    #[serde(default = "default_root_symbol")]
    pub root_symbol: String,
    #[serde(default = "default_provider_name")]
    pub provider_name: String,
    #[serde(default = "default_provider_source_surface")]
    pub provider_source_surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_entitlement_profile: Option<String>,
    #[serde(default = "default_quote_resolution")]
    pub quote_resolution: String,
    pub price_semantics: OptionsPriceSemantics,
    #[serde(default = "default_calendar_id")]
    pub calendar_id: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    pub coverage_start: i64,
    pub coverage_end: i64,
    pub normalization_version: u32,
    pub quote_semantics_version: u32,
    pub corporate_action_mapping_version: u32,
    #[serde(default = "default_contract_reference_source")]
    pub contract_reference_source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_reference_as_of_utc: Option<String>,
    #[serde(default = "default_underlier_reference_source")]
    pub underlier_reference_source: String,
    pub dataset_path: String,
    pub row_count: usize,
    pub contract_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate_price_field: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsPriceRow {
    pub timestamp_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub underlying_price: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mid: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_interest: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote_age_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsContractSeries {
    pub root_symbol: String,
    pub contract_id: String,
    pub option_type: OptionType,
    pub expiration_date: String,
    pub expiration_timestamp_ms: i64,
    pub strike: f64,
    pub multiplier: f64,
    #[serde(default)]
    pub rows: Vec<OptionsPriceRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportedOptionsDataset {
    pub manifest: OptionsDatasetManifestV1,
    #[serde(default)]
    pub contracts: Vec<OptionsContractSeries>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsSignalEvent {
    pub event_id: String,
    pub parent_artifact_id: String,
    pub parent_run_kind: String,
    pub source_dataset_identity_hash: String,
    pub source_content_fingerprint: String,
    pub dataset_hash: String,
    pub timeframe: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resample_spec_hash: Option<String>,
    pub adjustment_mode: String,
    pub symbol_root: String,
    pub side: SignalSide,
    pub decision_timestamp: i64,
    pub earliest_tradable_timestamp: i64,
    pub decision_bar_index: usize,
    pub earliest_tradable_bar_index: usize,
    pub raw_underlier_price: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjusted_underlier_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_entry_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_exit_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_exit_bar_index: Option<usize>,
    pub fold: OptionsEvaluationSplit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regime_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_tag: Option<String>,
    #[serde(default = "default_calendar_id")]
    pub calendar_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsSignalEventManifestV1 {
    pub version: u32,
    pub manifest_id: String,
    pub parent_artifact_id: String,
    pub parent_run_kind: String,
    pub source_dataset_identity_hash: String,
    pub source_content_fingerprint: String,
    pub dataset_hash: String,
    pub timeframe: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resample_spec_hash: Option<String>,
    pub adjustment_mode: String,
    pub symbol_root: String,
    #[serde(default = "default_calendar_id")]
    pub calendar_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_dataset_id: Option<String>,
    #[serde(default)]
    pub events: Vec<OptionsSignalEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OptionsTupleSpec {
    pub delay_minutes: u16,
    pub dte_days: u16,
    pub moneyness_bucket: OptionsMoneynessBucket,
    pub hold_minutes: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_return: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_return: Option<f64>,
    pub exit_template: OptionsExitTemplateFamily,
    pub fill_model: OptionsFillModelFamily,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionMetrics {
    pub valid_trades: usize,
    pub invalid_events: usize,
    pub invalid_rate: f64,
    pub expectancy_per_trade: f64,
    pub expectancy_per_premium: f64,
    pub median_return_on_premium: f64,
    pub win_rate: f64,
    pub total_pnl_per_contract: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionTupleResult {
    pub tuple: OptionsTupleSpec,
    #[serde(default)]
    pub pre_sealed_rank: usize,
    #[serde(default)]
    pub sealed_finalist: bool,
    #[serde(default)]
    pub sealed_final_rank: usize,
    pub selection_metrics: OptionsExpressionMetrics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holdout_metrics: Option<OptionsExpressionMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sealed_evaluation_metrics: Option<OptionsExpressionMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stress_metrics: Option<OptionsExpressionMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sealed_stress_metrics: Option<OptionsExpressionMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harsher_stress_metrics: Option<OptionsExpressionMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sealed_harsher_stress_metrics: Option<OptionsExpressionMetrics>,
    #[serde(default)]
    pub robust_score: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OptionsExpressionEventStatus {
    Executed,
    CandidateMissing,
    OutcomeMissing,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OptionsExpressionExitReason {
    FixedHoldCutoff,
    TargetHit,
    StopHit,
    EndOfDayCutoff,
    BaseExitCutoff,
    ForcedExpiryCutoff,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionEventDetailRow {
    pub event_index: usize,
    pub event_id: String,
    pub split: OptionsEvaluationSplit,
    pub side: SignalSide,
    pub decision_timestamp: i64,
    pub earliest_tradable_timestamp: i64,
    pub requested_entry_timestamp: i64,
    pub raw_underlier_price: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_exit_timestamp: Option<i64>,
    pub status: OptionsExpressionEventStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_invalid_reason: Option<OptionsCandidateInvalidReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strike: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiplier: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_option_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_underlier_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_option_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_paid: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pnl_per_contract: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_on_premium: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_lag_minutes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes_to_expiry_at_entry: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realized_hold_minutes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_option_high: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_option_low: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_underlier_high: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_underlier_low: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_reason: Option<OptionsExpressionExitReason>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionTupleInspectionResult {
    pub tuple_index: usize,
    pub tuple_count: usize,
    pub tuple_result: OptionsExpressionTupleResult,
    pub total_events: usize,
    pub executed_events: usize,
    pub candidate_missing_events: usize,
    pub outcome_missing_events: usize,
    pub rows: Vec<OptionsExpressionEventDetailRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionEventPathPoint {
    pub timestamp_ms: i64,
    pub option_open: f64,
    pub option_high: f64,
    pub option_low: f64,
    pub option_close: f64,
    pub underlying_price: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionEventPathInspectionResult {
    pub tuple_index: usize,
    pub event_index: usize,
    pub event_id: String,
    pub row: OptionsExpressionEventDetailRow,
    pub point_count: usize,
    #[serde(default)]
    pub points: Vec<OptionsExpressionEventPathPoint>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionDiagnostics {
    pub parent_artifact_path: String,
    pub options_dataset_id: String,
    pub root_symbol: String,
    pub total_events: usize,
    pub bullish_events: usize,
    pub bearish_events: usize,
    pub search_events: usize,
    pub holdout_events: usize,
    pub sealed_test_events: usize,
    pub dataset_contracts: usize,
    pub dataset_rows: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage_start: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage_end: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionStagingStats {
    pub candidate_slot_count: usize,
    pub compiled_candidate_count: usize,
    pub baseline_exit_summary_count: usize,
    pub conservative_exit_summary_count: usize,
    pub harsher_exit_summary_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionFeasibilityTimingStats {
    pub tuple_compile_ms: u64,
    pub baseline_candidate_compile_ms: u64,
    pub baseline_exit_summary_compile_ms: u64,
    pub conservative_candidate_compile_ms: u64,
    pub conservative_exit_summary_compile_ms: u64,
    pub harsher_candidate_compile_ms: u64,
    pub harsher_exit_summary_compile_ms: u64,
    pub cpu_ranking_ms: u64,
    #[serde(default)]
    pub cpu_ranking_us: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_ranking_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_ranking_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_tensor_score_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_tensor_score_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_exactify_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_exactify_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_sealed_stage_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_sealed_stage_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_speedup_vs_cpu: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionFeasibilityEventCoverage {
    pub total_events_with_candidates: usize,
    pub search_events_with_candidates: usize,
    pub holdout_events_with_candidates: usize,
    pub sealed_test_events_with_candidates: usize,
    pub total_events_with_baseline_outcomes: usize,
    pub search_events_with_baseline_outcomes: usize,
    pub holdout_events_with_baseline_outcomes: usize,
    pub sealed_test_events_with_baseline_outcomes: usize,
    pub total_events_with_conservative_outcomes: usize,
    pub search_events_with_conservative_outcomes: usize,
    pub holdout_events_with_conservative_outcomes: usize,
    pub sealed_test_events_with_conservative_outcomes: usize,
    pub total_events_with_harsher_outcomes: usize,
    pub search_events_with_harsher_outcomes: usize,
    pub holdout_events_with_harsher_outcomes: usize,
    pub sealed_test_events_with_harsher_outcomes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionFeasibilityCacheStats {
    pub structural_cache_hits: usize,
    pub structural_cache_misses: usize,
    pub structural_cache_entries: usize,
    pub summary_cache_hits: usize,
    pub summary_cache_misses: usize,
    pub summary_cache_entries: usize,
    #[serde(default)]
    pub gpu_shared_input_cache_hits: usize,
    #[serde(default)]
    pub gpu_shared_input_cache_misses: usize,
    #[serde(default)]
    pub gpu_outcome_input_cache_hits: usize,
    #[serde(default)]
    pub gpu_outcome_input_cache_misses: usize,
    #[serde(default)]
    pub gpu_output_buffer_reuses: usize,
    #[serde(default)]
    pub gpu_output_buffer_allocations: usize,
    #[serde(default)]
    pub gpu_tuple_chunk_count: usize,
    #[serde(default)]
    pub gpu_tuple_chunk_size: usize,
    #[serde(default)]
    pub gpu_kernel_launch_count: usize,
    #[serde(default)]
    pub gpu_tuple_budget_bytes: usize,
    #[serde(default)]
    pub gpu_outcome_tensor_bytes: usize,
    #[serde(default)]
    pub gpu_shared_input_bytes_per_chunk: usize,
    #[serde(default)]
    pub gpu_output_bytes_per_chunk: usize,
    #[serde(default)]
    pub gpu_total_device_bytes_per_chunk: usize,
    #[serde(default)]
    pub gpu_device_total_memory_bytes: usize,
    #[serde(default)]
    pub gpu_device_free_memory_bytes: usize,
    #[serde(default)]
    pub gpu_device_usable_memory_bytes: usize,
    #[serde(default)]
    pub gpu_memory_budget_source: String,
    #[serde(default)]
    pub gpu_outcome_tensor_fits_in_memory: bool,
    #[serde(default)]
    pub gpu_chunk_plan_fits_in_memory: bool,
    #[serde(default)]
    pub gpu_plan_notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionAutoBackendDecision {
    pub recommended_backend: String,
    pub gpu_eligible: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionFeasibilityReport {
    pub diagnostics: OptionsExpressionDiagnostics,
    pub price_semantics: OptionsPriceSemantics,
    pub fill_model: OptionsFillModelFamily,
    pub best_available_backend: String,
    #[serde(default)]
    pub auto_backend_decision: OptionsExpressionAutoBackendDecision,
    pub compiled_tuple_count: usize,
    pub per_event_candidate_slots: usize,
    pub per_candidate_exit_summaries: usize,
    pub candidate_slot_count: usize,
    pub compiled_candidate_count: usize,
    pub candidate_density: f64,
    pub invalid_slot_count: usize,
    pub invalid_slot_rate: f64,
    pub baseline_exit_summary_count: usize,
    pub baseline_compiled_exit_summary_count: usize,
    pub baseline_exit_summary_density: f64,
    pub conservative_exit_summary_count: usize,
    pub conservative_compiled_exit_summary_count: usize,
    pub conservative_exit_summary_density: f64,
    pub harsher_exit_summary_count: usize,
    pub harsher_compiled_exit_summary_count: usize,
    pub harsher_exit_summary_density: f64,
    pub projected_event_tuple_evaluations: u64,
    pub structural_cache_bytes_estimate: usize,
    pub baseline_summary_bytes_estimate: usize,
    pub conservative_summary_bytes_estimate: usize,
    pub harsher_summary_bytes_estimate: usize,
    pub timings: OptionsExpressionFeasibilityTimingStats,
    pub event_coverage: OptionsExpressionFeasibilityEventCoverage,
    pub cache_stats: OptionsExpressionFeasibilityCacheStats,
    #[serde(default)]
    pub invalid_reasons: Vec<OptionsCandidateInvalidReasonCount>,
    #[serde(default)]
    pub axis_coverage: OptionsCandidateAxisCoverage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionRunRequest {
    pub parent_artifact_path: String,
    #[serde(default)]
    pub parent_candidate_index: Option<usize>,
    pub options_dataset_id: String,
    #[serde(default = "default_backend_preference")]
    pub backend_preference: OptionsExpressionBackendPreference,
    #[serde(default = "default_delay_minutes")]
    pub delay_minutes: Vec<u16>,
    #[serde(default = "default_dte_buckets")]
    pub dte_buckets: Vec<u16>,
    #[serde(default = "default_moneyness_buckets")]
    pub moneyness_buckets: Vec<OptionsMoneynessBucket>,
    #[serde(default = "default_hold_minutes")]
    pub hold_minutes: Vec<u16>,
    #[serde(default = "default_target_buckets")]
    pub target_return_buckets: Vec<f64>,
    #[serde(default = "default_stop_buckets")]
    pub stop_return_buckets: Vec<f64>,
    #[serde(default = "default_exit_template")]
    pub exit_template: OptionsExitTemplateFamily,
    #[serde(default = "default_fill_model")]
    pub fill_model: OptionsFillModelFamily,
    #[serde(default)]
    pub filters: OptionsStaticFilterBundle,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_valid_trades: Option<usize>,
    #[serde(default = "default_split_ratio_search")]
    pub split_ratio_search: f64,
    #[serde(default = "default_split_ratio_holdout")]
    pub split_ratio_holdout: f64,
    #[serde(default = "default_split_assignment_mode")]
    pub split_assignment_mode: OptionsSplitAssignmentMode,
    #[serde(default = "default_sealed_finalist_count")]
    pub sealed_finalist_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionRunResult {
    pub diagnostics: OptionsExpressionDiagnostics,
    pub backend_used: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_fallback_reason: Option<String>,
    pub price_semantics: OptionsPriceSemantics,
    pub tuple_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging: Option<OptionsExpressionStagingStats>,
    #[serde(default)]
    pub ranked_results: Vec<OptionsExpressionTupleResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionArtifactV1 {
    pub version: u32,
    pub kind: String,
    pub ts: String,
    pub artifact_path: String,
    pub parent_artifact_id: String,
    pub root_symbol: String,
    pub options_dataset_id: String,
    pub options_dataset_hash: String,
    pub backend_used: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_fallback_reason: Option<String>,
    pub price_semantics: OptionsPriceSemantics,
    pub request: OptionsExpressionRunRequest,
    pub diagnostics: OptionsExpressionDiagnostics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging: Option<OptionsExpressionStagingStats>,
    pub selection_metrics: Vec<OptionsExpressionTupleResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_fill_model_is_aggregate_research() {
        let req: OptionsExpressionRunRequest =
            serde_json::from_str(r#"{"parent_artifact_path":"a","options_dataset_id":"b"}"#)
                .expect("request");
        assert_eq!(req.fill_model, OptionsFillModelFamily::AggregateResearch);
        assert_eq!(
            req.exit_template,
            OptionsExitTemplateFamily::FixedHoldOrTargetStop
        );
        assert_eq!(
            req.backend_preference,
            OptionsExpressionBackendPreference::Auto
        );
        assert_eq!(
            req.split_assignment_mode,
            OptionsSplitAssignmentMode::EventChronological
        );
        assert_eq!(req.sealed_finalist_count, 5);
        assert!(!req.delay_minutes.is_empty());
    }
}
