use serde::{Deserialize, Serialize};

pub const OPTIONS_DATASET_MANIFEST_VERSION: u32 = 1;
pub const OPTIONS_SIGNAL_MANIFEST_VERSION: u32 = 1;
pub const OPTIONS_EXPRESSION_ARTIFACT_VERSION: u32 = 1;
pub const OPTIONS_EXPRESSION_VALIDATION_ARTIFACT_VERSION: u32 = 1;

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_quote_resolution() -> String {
    "1m".to_string()
}

fn default_provider_name() -> String {
    "polygon.io".to_string()
}

fn default_provider_source_surface() -> String {
    "minute_aggregate".to_string()
}

fn default_contract_reference_source() -> String {
    "polygon_contract_reference".to_string()
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

fn default_structure_family() -> OptionsStructureFamily {
    OptionsStructureFamily::LongSingle
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

fn default_validation_tuple_selection() -> OptionsExpressionValidationTupleSelection {
    OptionsExpressionValidationTupleSelection::default()
}

fn default_validation_plan() -> OptionsExpressionValidationPlan {
    OptionsExpressionValidationPlan::default()
}

fn default_validation_dataset_policy() -> OptionsExpressionValidationDatasetPolicy {
    OptionsExpressionValidationDatasetPolicy::RequireExactReplay
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OptionsPriceSemantics {
    MinuteAggregate,
    TopOfBookQuote,
    DerivedMidSnapshot,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptionsExactnessClass {
    QuoteClass,
    #[default]
    AggregateClass,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptionsMedianReplayContract {
    #[default]
    RankingIrrelevantDeferred,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptionsRetainedFrontierCertification {
    CertifiedExact,
    ProvisionalMixedPrecision,
    #[default]
    ProvisionalMissingExtremeLane,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptionsExactPruningContract {
    ExactCertified,
    RequiresIntervalCertificate,
    #[default]
    RequiresExactExtremeLaneAndIntervalCertificate,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct OptionsExpressionExactnessSemantics {
    pub exactness_class: OptionsExactnessClass,
    pub median_replay_contract: OptionsMedianReplayContract,
    pub retained_frontier_certification: OptionsRetainedFrontierCertification,
    pub exact_pruning_contract: OptionsExactPruningContract,
}

impl OptionsPriceSemantics {
    pub fn exactness_class(self) -> OptionsExactnessClass {
        match self {
            OptionsPriceSemantics::TopOfBookQuote => OptionsExactnessClass::QuoteClass,
            OptionsPriceSemantics::MinuteAggregate | OptionsPriceSemantics::DerivedMidSnapshot => {
                OptionsExactnessClass::AggregateClass
            }
        }
    }

    pub fn exactness_semantics_for_backend(
        self,
        backend_used: &str,
    ) -> OptionsExpressionExactnessSemantics {
        match self.exactness_class() {
            OptionsExactnessClass::QuoteClass => {
                let cpu_exact = backend_used.trim().eq_ignore_ascii_case("cpu_reference");
                OptionsExpressionExactnessSemantics {
                    exactness_class: OptionsExactnessClass::QuoteClass,
                    median_replay_contract: OptionsMedianReplayContract::RankingIrrelevantDeferred,
                    retained_frontier_certification: if cpu_exact {
                        OptionsRetainedFrontierCertification::CertifiedExact
                    } else {
                        OptionsRetainedFrontierCertification::ProvisionalMixedPrecision
                    },
                    exact_pruning_contract: if cpu_exact {
                        OptionsExactPruningContract::ExactCertified
                    } else {
                        OptionsExactPruningContract::RequiresIntervalCertificate
                    },
                }
            }
            OptionsExactnessClass::AggregateClass => OptionsExpressionExactnessSemantics {
                exactness_class: OptionsExactnessClass::AggregateClass,
                median_replay_contract: OptionsMedianReplayContract::RankingIrrelevantDeferred,
                retained_frontier_certification:
                    OptionsRetainedFrontierCertification::ProvisionalMissingExtremeLane,
                exact_pruning_contract:
                    OptionsExactPruningContract::RequiresExactExtremeLaneAndIntervalCertificate,
            },
        }
    }

    pub fn supports_fill_model(self, fill_model: OptionsFillModelFamily) -> bool {
        match (self, fill_model) {
            (OptionsPriceSemantics::MinuteAggregate, OptionsFillModelFamily::AggregateResearch) => {
                true
            }
            (
                OptionsPriceSemantics::DerivedMidSnapshot,
                OptionsFillModelFamily::AggregateResearch,
            ) => true,
            (OptionsPriceSemantics::TopOfBookQuote, _) => true,
            _ => false,
        }
    }

    pub fn recommended_fill_model(self) -> OptionsFillModelFamily {
        match self {
            OptionsPriceSemantics::TopOfBookQuote => OptionsFillModelFamily::QuoteConservative,
            OptionsPriceSemantics::MinuteAggregate | OptionsPriceSemantics::DerivedMidSnapshot => {
                OptionsFillModelFamily::AggregateResearch
            }
        }
    }

    pub fn allowed_fill_models_label(self) -> &'static str {
        match self {
            OptionsPriceSemantics::MinuteAggregate | OptionsPriceSemantics::DerivedMidSnapshot => {
                "aggregate_research"
            }
            OptionsPriceSemantics::TopOfBookQuote => {
                "aggregate_research, quote_conservative, quote_mid"
            }
        }
    }
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
pub enum OptionsStructureFamily {
    LongSingle,
    DebitVertical,
    LongPair,
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
    NoCompanionLegInStructure,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_quote_age_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_spread_bps: Option<f64>,
}

impl Default for OptionsStaticFilterBundle {
    fn default() -> Self {
        Self {
            max_entry_lag_minutes: Some(5),
            max_premium: None,
            min_premium: None,
            min_volume: None,
            min_open_interest: None,
            max_quote_age_ms: None,
            max_spread_bps: None,
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
    #[serde(default = "default_structure_family")]
    pub structure_family: OptionsStructureFamily,
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
    #[serde(default)]
    pub mean_return_on_premium: f64,
    #[serde(default)]
    pub return_volatility_on_premium: f64,
    pub median_return_on_premium: f64,
    pub win_rate: f64,
    pub total_pnl_per_contract: f64,
}

impl OptionsExpressionMetrics {
    pub fn total_events(&self) -> usize {
        self.valid_trades.saturating_add(self.invalid_events)
    }

    pub fn has_supported_trades(&self) -> bool {
        self.valid_trades > 0
    }

    pub fn support_rate(&self) -> Option<f64> {
        let total_events = self.total_events();
        if total_events == 0 {
            None
        } else {
            Some(self.valid_trades as f64 / total_events as f64)
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptionsExpressionValidationVerdict {
    #[default]
    SearchOnly,
    HoldoutSupported,
    SealedFinalist,
    StressFragile,
}

pub const OPTIONS_VALIDATION_STRESS_DROP_THRESHOLD: f64 = 0.03;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extreme_stress_metrics: Option<OptionsExpressionMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sealed_extreme_stress_metrics: Option<OptionsExpressionMetrics>,
    #[serde(default)]
    pub validation_verdict: OptionsExpressionValidationVerdict,
    #[serde(default)]
    pub robust_score: f64,
}

impl OptionsExpressionTupleResult {
    fn supported_metrics(
        metrics: Option<&OptionsExpressionMetrics>,
    ) -> Option<&OptionsExpressionMetrics> {
        metrics.filter(|metrics| metrics.has_supported_trades())
    }

    pub fn supported_holdout_metrics(&self) -> Option<&OptionsExpressionMetrics> {
        Self::supported_metrics(self.holdout_metrics.as_ref())
    }

    pub fn supported_sealed_evaluation_metrics(&self) -> Option<&OptionsExpressionMetrics> {
        Self::supported_metrics(self.sealed_evaluation_metrics.as_ref())
    }

    pub fn strongest_validation_metrics(&self) -> Option<&OptionsExpressionMetrics> {
        self.supported_sealed_evaluation_metrics()
            .or(self.supported_holdout_metrics())
    }

    pub fn strongest_validation_stress_metrics(&self) -> Option<&OptionsExpressionMetrics> {
        Self::supported_metrics(self.sealed_extreme_stress_metrics.as_ref())
            .or(Self::supported_metrics(
                self.sealed_harsher_stress_metrics.as_ref(),
            ))
            .or(Self::supported_metrics(self.sealed_stress_metrics.as_ref()))
            .or(Self::supported_metrics(
                self.extreme_stress_metrics.as_ref(),
            ))
            .or(Self::supported_metrics(
                self.harsher_stress_metrics.as_ref(),
            ))
            .or(Self::supported_metrics(self.stress_metrics.as_ref()))
    }

    pub fn inferred_validation_verdict(&self) -> OptionsExpressionValidationVerdict {
        let has_supported_sealed_validation = self.supported_sealed_evaluation_metrics().is_some();
        let has_supported_holdout_validation = self.supported_holdout_metrics().is_some();
        if has_supported_sealed_validation {
            if self.is_stress_fragile() {
                OptionsExpressionValidationVerdict::StressFragile
            } else {
                OptionsExpressionValidationVerdict::SealedFinalist
            }
        } else if has_supported_holdout_validation {
            if self.is_stress_fragile() {
                OptionsExpressionValidationVerdict::StressFragile
            } else {
                OptionsExpressionValidationVerdict::HoldoutSupported
            }
        } else {
            OptionsExpressionValidationVerdict::SearchOnly
        }
    }

    pub fn is_stress_fragile(&self) -> bool {
        let Some(base) = self.strongest_validation_metrics() else {
            return false;
        };
        let Some(stressed) = self.strongest_validation_stress_metrics() else {
            return false;
        };
        stressed.expectancy_per_premium < 0.0
            || (base.expectancy_per_premium - stressed.expectancy_per_premium)
                >= OPTIONS_VALIDATION_STRESS_DROP_THRESHOLD
    }
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
    pub companion_contract_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strike: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_strike: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiplier: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_option_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_entry_option_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_underlier_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_option_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_exit_option_price: Option<f64>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exactify_buffered_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exactify_final_top_k: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exactify_rescued_from_buffer_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provisional_final_robust_score_drift_mean_abs: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provisional_final_robust_score_drift_max_abs: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionFeasibilityTimingStats {
    #[serde(default)]
    pub prepare_ms: u64,
    #[serde(default)]
    pub prepare_us: u64,
    pub tuple_compile_ms: u64,
    pub baseline_candidate_compile_ms: u64,
    pub baseline_exit_summary_compile_ms: u64,
    pub conservative_candidate_compile_ms: u64,
    pub conservative_exit_summary_compile_ms: u64,
    pub harsher_candidate_compile_ms: u64,
    pub harsher_exit_summary_compile_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_device_resolution_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_device_resolution_us: Option<u64>,
    pub cpu_ranking_ms: u64,
    #[serde(default)]
    pub cpu_ranking_us: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_plan_probe_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_plan_probe_us: Option<u64>,
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
    pub gpu_exactify_summary_overwrite_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_exactify_summary_overwrite_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_exactify_enrichment_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_exactify_enrichment_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_exactify_extreme_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_exactify_extreme_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_sealed_stage_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_sealed_stage_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_sealed_non_extreme_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_sealed_non_extreme_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_sealed_extreme_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_sealed_extreme_us: Option<u64>,
    #[serde(default)]
    pub report_finalize_ms: u64,
    #[serde(default)]
    pub report_finalize_us: u64,
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
pub struct OptionsExpressionOverlapWindowSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_reference_as_of_utc: Option<String>,
    pub total_events_in_dataset_window: usize,
    pub search_events_in_dataset_window: usize,
    pub holdout_events_in_dataset_window: usize,
    pub sealed_test_events_in_dataset_window: usize,
    pub total_events_before_dataset_window: usize,
    pub total_events_after_dataset_window: usize,
    pub search_events_before_dataset_window: usize,
    pub search_events_after_dataset_window: usize,
    pub holdout_events_before_dataset_window: usize,
    pub holdout_events_after_dataset_window: usize,
    pub sealed_test_events_before_dataset_window: usize,
    pub sealed_test_events_after_dataset_window: usize,
    #[serde(default)]
    pub total_events_before_contract_reference_as_of: usize,
    #[serde(default)]
    pub search_events_before_contract_reference_as_of: usize,
    #[serde(default)]
    pub holdout_events_before_contract_reference_as_of: usize,
    #[serde(default)]
    pub sealed_test_events_before_contract_reference_as_of: usize,
    pub total_overlap_rate: f64,
    pub search_overlap_rate: f64,
    pub holdout_overlap_rate: f64,
    pub sealed_test_overlap_rate: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionSplitInvalidReasonSummary {
    #[serde(default)]
    pub search_invalid_reasons: Vec<OptionsCandidateInvalidReasonCount>,
    #[serde(default)]
    pub holdout_invalid_reasons: Vec<OptionsCandidateInvalidReasonCount>,
    #[serde(default)]
    pub sealed_test_invalid_reasons: Vec<OptionsCandidateInvalidReasonCount>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptionsExpressionCoverageAdequacyVerdict {
    #[default]
    Unknown,
    Healthy,
    WeakDatasetOverlap,
    SearchOverlapMissing,
    SearchCoverageWeak,
    HoldoutCoverageWeak,
    SealedCoverageWeak,
    ContractEligibilityFragile,
    LateEntryFragile,
    StrikeWindowFragile,
    MixedFragile,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionCoverageAdequacySummary {
    #[serde(default)]
    pub verdict: OptionsExpressionCoverageAdequacyVerdict,
    #[serde(default)]
    pub non_promotable: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub search_candidate_coverage_rate: f64,
    #[serde(default)]
    pub holdout_candidate_coverage_rate: f64,
    #[serde(default)]
    pub sealed_test_candidate_coverage_rate: f64,
    #[serde(default)]
    pub search_baseline_outcome_coverage_rate: f64,
    #[serde(default)]
    pub holdout_baseline_outcome_coverage_rate: f64,
    #[serde(default)]
    pub sealed_test_baseline_outcome_coverage_rate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dominant_invalid_reason: Option<OptionsCandidateInvalidReason>,
    #[serde(default)]
    pub dominant_invalid_reason_share: f64,
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
    #[serde(default)]
    pub exactness_semantics: OptionsExpressionExactnessSemantics,
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
    #[serde(default)]
    pub overlap_window: OptionsExpressionOverlapWindowSummary,
    #[serde(default)]
    pub split_invalid_reasons: OptionsExpressionSplitInvalidReasonSummary,
    #[serde(default)]
    pub coverage_adequacy: OptionsExpressionCoverageAdequacySummary,
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
    #[serde(default = "default_structure_family")]
    pub structure_family: OptionsStructureFamily,
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
    #[serde(default)]
    pub exactness_semantics: OptionsExpressionExactnessSemantics,
    pub tuple_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging: Option<OptionsExpressionStagingStats>,
    #[serde(default)]
    pub overlap_window: OptionsExpressionOverlapWindowSummary,
    #[serde(default)]
    pub coverage_adequacy: OptionsExpressionCoverageAdequacySummary,
    #[serde(default)]
    pub split_invalid_reasons: OptionsExpressionSplitInvalidReasonSummary,
    #[serde(default)]
    pub ranked_results: Vec<OptionsExpressionTupleResult>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptionsExpressionValidationDatasetPolicy {
    #[default]
    RequireExactReplay,
    AllowSameRootReplacement,
    AllowExplicitCrossLaneComparison,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptionsExpressionValidationPlanMode {
    #[default]
    OriginalEmbeddedSplits,
    SessionChronologicalReplay,
    AnchoredWalkForward,
    RollingSessionWalkForward,
    DateRangeHoldout,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OptionsExpressionValidationPlan {
    #[serde(default)]
    pub mode: OptionsExpressionValidationPlanMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_timestamp_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_timestamp_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub train_session_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_session_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_session_count: Option<usize>,
}

impl Default for OptionsExpressionValidationPlan {
    fn default() -> Self {
        Self {
            mode: OptionsExpressionValidationPlanMode::OriginalEmbeddedSplits,
            start_timestamp_ms: None,
            end_timestamp_ms: None,
            train_session_count: None,
            validation_session_count: None,
            step_session_count: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionsExpressionValidationTupleSelection {
    #[serde(default = "default_true")]
    pub include_sealed_finalists: bool,
    #[serde(default = "default_true")]
    pub include_robust_leader: bool,
    #[serde(default = "default_true")]
    pub include_holdout_leader: bool,
    #[serde(default)]
    pub tuple_indexes: Vec<usize>,
}

impl Default for OptionsExpressionValidationTupleSelection {
    fn default() -> Self {
        Self {
            include_sealed_finalists: true,
            include_robust_leader: true,
            include_holdout_leader: true,
            tuple_indexes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionValidationRequest {
    pub artifact_path: String,
    #[serde(default)]
    pub options_dataset_id: String,
    #[serde(default = "default_validation_dataset_policy")]
    pub dataset_policy: OptionsExpressionValidationDatasetPolicy,
    #[serde(default = "default_validation_tuple_selection")]
    pub tuple_selection: OptionsExpressionValidationTupleSelection,
    #[serde(default = "default_validation_plan")]
    pub validation_plan: OptionsExpressionValidationPlan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_model_override: Option<OptionsFillModelFamily>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_valid_trades: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionValidationTupleDriftSummary {
    pub selection_expectancy_delta: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holdout_expectancy_delta: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sealed_expectancy_delta: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stress_expectancy_delta: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extreme_stress_expectancy_delta: Option<f64>,
    pub robust_score_delta: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionValidationTupleResult {
    pub source_tuple_index: usize,
    pub source_validation_verdict: OptionsExpressionValidationVerdict,
    pub source_robust_score: f64,
    pub source_tuple: OptionsTupleSpec,
    pub validated_tuple: OptionsExpressionTupleResult,
    pub support_deficient: bool,
    pub drift: OptionsExpressionValidationTupleDriftSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionValidationLaneSummary {
    pub lane: String,
    pub tuple_count: usize,
    pub supported_tuple_count: usize,
    pub support_deficient_tuple_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_valid_trades: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_expectancy_per_premium: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worst_expectancy_per_premium: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub median_expectancy_per_premium: Option<f64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptionsExpressionValidationAcceptanceVerdict {
    Accepted,
    Warning,
    #[default]
    SupportDeficient,
    Rejected,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsExpressionValidationAcceptanceSummary {
    pub verdict: OptionsExpressionValidationAcceptanceVerdict,
    pub selected_tuple_count: usize,
    pub validated_tuple_count: usize,
    pub accepted_tuple_count: usize,
    pub support_deficient_tuple_count: usize,
    pub stress_fragile_tuple_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_valid_trades: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_validated_expectancy_per_premium: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worst_stress_expectancy_per_premium: Option<f64>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionValidationEvaluationResult {
    pub diagnostics: OptionsExpressionDiagnostics,
    #[serde(default)]
    pub overlap_window: OptionsExpressionOverlapWindowSummary,
    #[serde(default)]
    pub coverage_adequacy: OptionsExpressionCoverageAdequacySummary,
    #[serde(default)]
    pub lane_summaries: Vec<OptionsExpressionValidationLaneSummary>,
    pub acceptance_summary: OptionsExpressionValidationAcceptanceSummary,
    pub tuple_results: Vec<OptionsExpressionValidationTupleResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionValidationResult {
    pub version: u32,
    pub kind: String,
    pub ts: String,
    pub validation_artifact_path: String,
    pub source_artifact_path: String,
    pub source_artifact_hash: String,
    pub parent_artifact_id: String,
    pub parent_artifact_hash: String,
    pub signal_manifest_id: String,
    pub root_symbol: String,
    pub options_dataset_id: String,
    pub options_dataset_hash: String,
    pub dataset_policy: OptionsExpressionValidationDatasetPolicy,
    pub replacement_dataset: bool,
    pub validation_plan: OptionsExpressionValidationPlan,
    pub tuple_selection: OptionsExpressionValidationTupleSelection,
    pub price_semantics: OptionsPriceSemantics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_provenance: Option<OptionsExpressionDatasetProvenance>,
    pub diagnostics: OptionsExpressionDiagnostics,
    #[serde(default)]
    pub overlap_window: OptionsExpressionOverlapWindowSummary,
    #[serde(default)]
    pub coverage_adequacy: OptionsExpressionCoverageAdequacySummary,
    #[serde(default)]
    pub lane_summaries: Vec<OptionsExpressionValidationLaneSummary>,
    pub acceptance_summary: OptionsExpressionValidationAcceptanceSummary,
    pub tuple_results: Vec<OptionsExpressionValidationTupleResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct OptionsExpressionDatasetProvenance {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub provider_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub provider_source_surface: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub quote_resolution: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate_price_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_reference_as_of_utc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote_quality: Option<OptionsExpressionDatasetQuoteQualitySummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct OptionsExpressionContractQuoteQualitySummary {
    pub contract_id: String,
    pub row_count: usize,
    pub any_quote_row_count: usize,
    pub bid_ask_row_count: usize,
    pub mid_row_count: usize,
    pub quote_age_row_count: usize,
    pub stale_quote_row_count: usize,
    pub any_quote_coverage_rate: f64,
    pub bid_ask_coverage_rate: f64,
    pub mid_coverage_rate: f64,
    pub stale_quote_rate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_spread_bps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_spread_bps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_quote_age_ms: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OptionsExpressionDatasetQuoteQualitySummary {
    pub price_semantics: OptionsPriceSemantics,
    pub row_count: usize,
    pub contract_count: usize,
    pub any_quote_row_count: usize,
    pub bid_row_count: usize,
    pub ask_row_count: usize,
    pub bid_ask_row_count: usize,
    pub mid_row_count: usize,
    pub quote_age_row_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_quote_threshold_ms: Option<i64>,
    pub stale_quote_row_count: usize,
    pub any_quote_coverage_rate: f64,
    pub bid_ask_coverage_rate: f64,
    pub mid_coverage_rate: f64,
    pub stale_quote_rate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_spread_bps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub median_spread_bps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_spread_bps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_quote_age_ms: Option<i64>,
    #[serde(default)]
    pub per_contract: Vec<OptionsExpressionContractQuoteQualitySummary>,
}

impl Default for OptionsExpressionDatasetQuoteQualitySummary {
    fn default() -> Self {
        Self {
            price_semantics: OptionsPriceSemantics::MinuteAggregate,
            row_count: 0,
            contract_count: 0,
            any_quote_row_count: 0,
            bid_row_count: 0,
            ask_row_count: 0,
            bid_ask_row_count: 0,
            mid_row_count: 0,
            quote_age_row_count: 0,
            stale_quote_threshold_ms: None,
            stale_quote_row_count: 0,
            any_quote_coverage_rate: 0.0,
            bid_ask_coverage_rate: 0.0,
            mid_coverage_rate: 0.0,
            stale_quote_rate: 0.0,
            mean_spread_bps: None,
            median_spread_bps: None,
            max_spread_bps: None,
            max_quote_age_ms: None,
            per_contract: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionsExpressionArtifactV1 {
    pub version: u32,
    pub kind: String,
    pub ts: String,
    pub artifact_path: String,
    pub parent_artifact_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub parent_artifact_hash: String,
    pub root_symbol: String,
    pub options_dataset_id: String,
    pub options_dataset_hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub signal_manifest_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_provenance: Option<OptionsExpressionDatasetProvenance>,
    pub backend_used: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_fallback_reason: Option<String>,
    pub price_semantics: OptionsPriceSemantics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exactness_semantics: Option<OptionsExpressionExactnessSemantics>,
    pub request: OptionsExpressionRunRequest,
    pub diagnostics: OptionsExpressionDiagnostics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging: Option<OptionsExpressionStagingStats>,
    #[serde(default)]
    pub overlap_window: OptionsExpressionOverlapWindowSummary,
    #[serde(default)]
    pub coverage_adequacy: OptionsExpressionCoverageAdequacySummary,
    #[serde(default)]
    pub split_invalid_reasons: OptionsExpressionSplitInvalidReasonSummary,
    pub selection_metrics: Vec<OptionsExpressionTupleResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics_with_counts(
        ev_per_premium: f64,
        valid_trades: usize,
        invalid_events: usize,
    ) -> OptionsExpressionMetrics {
        let total_events = valid_trades.saturating_add(invalid_events);
        OptionsExpressionMetrics {
            valid_trades,
            invalid_events,
            invalid_rate: if total_events == 0 {
                0.0
            } else {
                invalid_events as f64 / total_events as f64
            },
            expectancy_per_trade: ev_per_premium * 10.0,
            expectancy_per_premium: ev_per_premium,
            mean_return_on_premium: ev_per_premium,
            return_volatility_on_premium: 0.0,
            median_return_on_premium: ev_per_premium * 0.8,
            win_rate: 0.5,
            total_pnl_per_contract: ev_per_premium * 100.0,
        }
    }

    fn metrics(ev_per_premium: f64) -> OptionsExpressionMetrics {
        metrics_with_counts(ev_per_premium, 4, 0)
    }

    fn sample_tuple_result() -> OptionsExpressionTupleResult {
        OptionsExpressionTupleResult {
            tuple: OptionsTupleSpec {
                structure_family: OptionsStructureFamily::LongSingle,
                delay_minutes: 0,
                dte_days: 1,
                moneyness_bucket: OptionsMoneynessBucket::Atm,
                hold_minutes: 15,
                target_return: Some(0.25),
                stop_return: Some(0.25),
                exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
                fill_model: OptionsFillModelFamily::AggregateResearch,
            },
            pre_sealed_rank: 0,
            sealed_finalist: false,
            sealed_final_rank: 0,
            selection_metrics: metrics(0.20),
            holdout_metrics: None,
            sealed_evaluation_metrics: None,
            stress_metrics: None,
            sealed_stress_metrics: None,
            harsher_stress_metrics: None,
            sealed_harsher_stress_metrics: None,
            extreme_stress_metrics: None,
            sealed_extreme_stress_metrics: None,
            validation_verdict: OptionsExpressionValidationVerdict::SearchOnly,
            robust_score: 0.0,
        }
    }

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
        assert_eq!(req.structure_family, OptionsStructureFamily::LongSingle);
        assert!(!req.delay_minutes.is_empty());
    }

    #[test]
    fn run_request_omitted_filters_uses_bundle_default_shape() {
        let req: OptionsExpressionRunRequest =
            serde_json::from_str(r#"{"parent_artifact_path":"a","options_dataset_id":"b"}"#)
                .expect("request");

        assert_eq!(req.filters.max_entry_lag_minutes, Some(5));
        assert_eq!(req.filters.max_premium, None);
        assert_eq!(req.filters.min_premium, None);
        assert_eq!(req.filters.min_volume, None);
        assert_eq!(req.filters.min_open_interest, None);
        assert_eq!(req.filters.max_quote_age_ms, None);
        assert_eq!(req.filters.max_spread_bps, None);
    }

    #[test]
    fn run_request_empty_filters_object_uses_field_level_defaults() {
        let req: OptionsExpressionRunRequest = serde_json::from_str(
            r#"{"parent_artifact_path":"a","options_dataset_id":"b","filters":{}}"#,
        )
        .expect("request");

        assert_eq!(req.filters.max_entry_lag_minutes, None);
        assert_eq!(req.filters.max_premium, None);
        assert_eq!(req.filters.min_premium, None);
        assert_eq!(req.filters.min_volume, None);
        assert_eq!(req.filters.min_open_interest, None);
        assert_eq!(req.filters.max_quote_age_ms, None);
        assert_eq!(req.filters.max_spread_bps, None);
    }

    #[test]
    fn run_request_filters_null_entry_lag_preserves_explicit_none() {
        let req: OptionsExpressionRunRequest = serde_json::from_str(
            r#"{"parent_artifact_path":"a","options_dataset_id":"b","filters":{"max_entry_lag_minutes":null}}"#,
        )
        .expect("request");

        assert_eq!(req.filters.max_entry_lag_minutes, None);
    }

    #[test]
    fn feasibility_timing_stats_default_prompt12_fields_to_none_and_omit_serialization() {
        let timings = OptionsExpressionFeasibilityTimingStats::default();
        assert_eq!(timings.prepare_ms, 0);
        assert_eq!(timings.prepare_us, 0);
        assert_eq!(timings.gpu_device_resolution_ms, None);
        assert_eq!(timings.gpu_device_resolution_us, None);
        assert_eq!(timings.gpu_plan_probe_ms, None);
        assert_eq!(timings.gpu_plan_probe_us, None);
        assert_eq!(timings.gpu_exactify_summary_overwrite_ms, None);
        assert_eq!(timings.gpu_exactify_summary_overwrite_us, None);
        assert_eq!(timings.gpu_exactify_enrichment_ms, None);
        assert_eq!(timings.gpu_exactify_enrichment_us, None);
        assert_eq!(timings.gpu_exactify_extreme_ms, None);
        assert_eq!(timings.gpu_exactify_extreme_us, None);
        assert_eq!(timings.gpu_sealed_non_extreme_ms, None);
        assert_eq!(timings.gpu_sealed_non_extreme_us, None);
        assert_eq!(timings.gpu_sealed_extreme_ms, None);
        assert_eq!(timings.gpu_sealed_extreme_us, None);
        assert_eq!(timings.report_finalize_ms, 0);
        assert_eq!(timings.report_finalize_us, 0);

        let value = serde_json::to_value(&timings).expect("serialize timings");
        assert!(value.get("gpu_device_resolution_ms").is_none());
        assert!(value.get("gpu_device_resolution_us").is_none());
        assert!(value.get("gpu_plan_probe_ms").is_none());
        assert!(value.get("gpu_plan_probe_us").is_none());
        assert!(value.get("gpu_exactify_summary_overwrite_ms").is_none());
        assert!(value.get("gpu_exactify_summary_overwrite_us").is_none());
        assert!(value.get("gpu_exactify_enrichment_ms").is_none());
        assert!(value.get("gpu_exactify_enrichment_us").is_none());
        assert!(value.get("gpu_exactify_extreme_ms").is_none());
        assert!(value.get("gpu_exactify_extreme_us").is_none());
        assert!(value.get("gpu_sealed_non_extreme_ms").is_none());
        assert!(value.get("gpu_sealed_non_extreme_us").is_none());
        assert!(value.get("gpu_sealed_extreme_ms").is_none());
        assert!(value.get("gpu_sealed_extreme_us").is_none());
    }

    #[test]
    fn inferred_validation_verdict_defaults_to_search_only_without_holdout() {
        let tuple = sample_tuple_result();
        assert_eq!(
            tuple.inferred_validation_verdict(),
            OptionsExpressionValidationVerdict::SearchOnly
        );
    }

    #[test]
    fn inferred_validation_verdict_promotes_holdout_supported_rows() {
        let mut tuple = sample_tuple_result();
        tuple.holdout_metrics = Some(metrics(0.12));
        tuple.stress_metrics = Some(metrics(0.10));
        assert_eq!(
            tuple.inferred_validation_verdict(),
            OptionsExpressionValidationVerdict::HoldoutSupported
        );
    }

    #[test]
    fn inferred_validation_verdict_promotes_non_fragile_sealed_finalists() {
        let mut tuple = sample_tuple_result();
        tuple.sealed_finalist = true;
        tuple.sealed_evaluation_metrics = Some(metrics(0.11));
        tuple.sealed_stress_metrics = Some(metrics(0.10));
        tuple.sealed_harsher_stress_metrics = Some(metrics(0.09));
        tuple.sealed_extreme_stress_metrics = Some(metrics(0.09));
        assert_eq!(
            tuple.inferred_validation_verdict(),
            OptionsExpressionValidationVerdict::SealedFinalist
        );
    }

    #[test]
    fn inferred_validation_verdict_marks_material_stress_drop_as_fragile() {
        let mut tuple = sample_tuple_result();
        tuple.sealed_finalist = true;
        tuple.sealed_evaluation_metrics = Some(metrics(0.09));
        tuple.sealed_stress_metrics = Some(metrics(0.05));
        tuple.sealed_harsher_stress_metrics = Some(metrics(0.04));
        tuple.sealed_extreme_stress_metrics = Some(metrics(0.03));
        assert!(tuple.is_stress_fragile());
        assert_eq!(
            tuple.inferred_validation_verdict(),
            OptionsExpressionValidationVerdict::StressFragile
        );
    }

    #[test]
    fn metrics_deserialize_missing_single_pass_expansion_fields_to_zero() {
        let metrics: OptionsExpressionMetrics = serde_json::from_value(serde_json::json!({
            "valid_trades": 3,
            "invalid_events": 1,
            "invalid_rate": 0.25,
            "expectancy_per_trade": 5.0,
            "expectancy_per_premium": 0.10,
            "median_return_on_premium": 0.08,
            "win_rate": 0.66,
            "total_pnl_per_contract": 15.0
        }))
        .expect("deserialize legacy metrics");

        assert_eq!(metrics.mean_return_on_premium, 0.0);
        assert_eq!(metrics.return_volatility_on_premium, 0.0);
    }

    #[test]
    fn validation_metrics_helpers_report_support_counts_and_ratio() {
        let metrics = metrics_with_counts(0.10, 3, 1);
        assert_eq!(metrics.total_events(), 4);
        assert!(metrics.has_supported_trades());
        assert_eq!(metrics.support_rate(), Some(0.75));

        let zero_support = metrics_with_counts(0.05, 0, 3);
        assert_eq!(zero_support.total_events(), 3);
        assert!(!zero_support.has_supported_trades());
        assert_eq!(zero_support.support_rate(), Some(0.0));
    }

    #[test]
    fn inferred_validation_verdict_ignores_zero_support_holdout_metrics() {
        let mut tuple = sample_tuple_result();
        tuple.holdout_metrics = Some(metrics_with_counts(0.12, 0, 3));
        tuple.stress_metrics = Some(metrics(0.10));

        assert!(tuple.strongest_validation_metrics().is_none());
        assert_eq!(
            tuple.inferred_validation_verdict(),
            OptionsExpressionValidationVerdict::SearchOnly
        );
    }

    #[test]
    fn inferred_validation_verdict_requires_supported_sealed_metrics_for_promotion() {
        let mut tuple = sample_tuple_result();
        tuple.sealed_finalist = true;
        tuple.holdout_metrics = Some(metrics(0.12));
        tuple.sealed_evaluation_metrics = Some(metrics_with_counts(0.11, 0, 2));
        tuple.stress_metrics = Some(metrics(0.10));

        assert_eq!(
            tuple.inferred_validation_verdict(),
            OptionsExpressionValidationVerdict::HoldoutSupported
        );
    }

    #[test]
    fn stress_fragility_requires_supported_stress_metrics() {
        let mut tuple = sample_tuple_result();
        tuple.holdout_metrics = Some(metrics(0.09));
        tuple.stress_metrics = Some(metrics_with_counts(0.03, 0, 2));

        assert!(!tuple.is_stress_fragile());
        assert_eq!(
            tuple.inferred_validation_verdict(),
            OptionsExpressionValidationVerdict::HoldoutSupported
        );
    }

    #[test]
    fn tuple_result_defaults_missing_persisted_summary_fields() {
        let tuple: OptionsExpressionTupleResult = serde_json::from_value(serde_json::json!({
            "tuple": {
                "structure_family": "long_single",
                "delay_minutes": 0,
                "dte_days": 1,
                "moneyness_bucket": "atm",
                "hold_minutes": 15,
                "target_return": 0.25,
                "stop_return": 0.25,
                "exit_template": "fixed_hold_or_target_stop",
                "fill_model": "aggregate_research"
            },
            "selection_metrics": {
                "valid_trades": 4,
                "invalid_events": 0,
                "invalid_rate": 0.0,
                "expectancy_per_trade": 2.0,
                "expectancy_per_premium": 0.2,
                "mean_return_on_premium": 0.2,
                "return_volatility_on_premium": 0.0,
                "median_return_on_premium": 0.16,
                "win_rate": 0.5,
                "total_pnl_per_contract": 20.0
            }
        }))
        .expect("deserialize tuple");

        assert_eq!(
            tuple.validation_verdict,
            OptionsExpressionValidationVerdict::SearchOnly
        );
        assert_eq!(tuple.robust_score, 0.0);
        assert_eq!(
            tuple.inferred_validation_verdict(),
            OptionsExpressionValidationVerdict::SearchOnly
        );
    }

    #[test]
    fn inferred_validation_verdict_is_derived_independently_of_persisted_summary_fields() {
        let mut tuple = sample_tuple_result();
        tuple.validation_verdict = OptionsExpressionValidationVerdict::SearchOnly;
        tuple.robust_score = 42.0;
        tuple.sealed_finalist = true;
        tuple.sealed_evaluation_metrics = Some(metrics(0.11));
        tuple.sealed_stress_metrics = Some(metrics(0.10));
        tuple.sealed_harsher_stress_metrics = Some(metrics(0.09));
        tuple.sealed_extreme_stress_metrics = Some(metrics(0.09));

        assert_eq!(
            tuple.inferred_validation_verdict(),
            OptionsExpressionValidationVerdict::SealedFinalist
        );
        assert_eq!(
            tuple.validation_verdict,
            OptionsExpressionValidationVerdict::SearchOnly
        );
        assert_eq!(tuple.robust_score, 42.0);
    }

    #[test]
    fn run_result_preserves_explicit_tuple_count_summary() {
        let run_result: OptionsExpressionRunResult = serde_json::from_value(serde_json::json!({
            "diagnostics": {
                "parent_artifact_path": "artifact.json",
                "options_dataset_id": "dataset",
                "root_symbol": "SPY",
                "total_events": 10,
                "bullish_events": 5,
                "bearish_events": 5,
                "search_events": 6,
                "holdout_events": 2,
                "sealed_test_events": 2,
                "dataset_contracts": 3,
                "dataset_rows": 100
            },
            "backend_used": "cpu_reference",
            "price_semantics": "minute_aggregate",
            "tuple_count": 7,
            "ranked_results": [serde_json::to_value(sample_tuple_result()).expect("tuple value")]
        }))
        .expect("deserialize run result");

        assert_eq!(run_result.tuple_count, 7);
        assert_eq!(run_result.ranked_results.len(), 1);
        assert_eq!(
            run_result
                .exactness_semantics
                .retained_frontier_certification,
            OptionsRetainedFrontierCertification::ProvisionalMissingExtremeLane
        );
    }

    #[test]
    fn event_path_inspection_preserves_explicit_point_count_summary() {
        let inspection: OptionsExpressionEventPathInspectionResult =
            serde_json::from_value(serde_json::json!({
                "tuple_index": 0,
                "event_index": 1,
                "event_id": "evt-1",
                "row": {
                    "event_index": 1,
                    "event_id": "evt-1",
                    "split": "search",
                    "side": "bullish",
                    "decision_timestamp": 1000,
                    "earliest_tradable_timestamp": 1001,
                    "requested_entry_timestamp": 1002,
                    "raw_underlier_price": 500.0,
                    "status": "executed"
                },
                "point_count": 3,
                "points": [{
                    "timestamp_ms": 1002,
                    "option_open": 1.0,
                    "option_high": 1.1,
                    "option_low": 0.9,
                    "option_close": 1.05,
                    "underlying_price": 500.0
                }]
            }))
            .expect("deserialize inspection result");

        assert_eq!(inspection.point_count, 3);
        assert_eq!(inspection.points.len(), 1);
    }

    #[test]
    fn price_semantics_fill_model_compatibility_matches_current_product_matrix() {
        assert!(OptionsPriceSemantics::MinuteAggregate
            .supports_fill_model(OptionsFillModelFamily::AggregateResearch));
        assert!(!OptionsPriceSemantics::MinuteAggregate
            .supports_fill_model(OptionsFillModelFamily::QuoteConservative));
        assert!(!OptionsPriceSemantics::MinuteAggregate
            .supports_fill_model(OptionsFillModelFamily::QuoteMid));

        assert!(OptionsPriceSemantics::DerivedMidSnapshot
            .supports_fill_model(OptionsFillModelFamily::AggregateResearch));
        assert!(!OptionsPriceSemantics::DerivedMidSnapshot
            .supports_fill_model(OptionsFillModelFamily::QuoteConservative));

        assert!(OptionsPriceSemantics::TopOfBookQuote
            .supports_fill_model(OptionsFillModelFamily::AggregateResearch));
        assert!(OptionsPriceSemantics::TopOfBookQuote
            .supports_fill_model(OptionsFillModelFamily::QuoteConservative));
        assert!(OptionsPriceSemantics::TopOfBookQuote
            .supports_fill_model(OptionsFillModelFamily::QuoteMid));
        assert_eq!(
            OptionsPriceSemantics::TopOfBookQuote.recommended_fill_model(),
            OptionsFillModelFamily::QuoteConservative
        );
    }

    #[test]
    fn price_semantics_exactness_contracts_match_current_runtime_matrix() {
        assert_eq!(
            OptionsPriceSemantics::TopOfBookQuote.exactness_class(),
            OptionsExactnessClass::QuoteClass
        );
        assert_eq!(
            OptionsPriceSemantics::MinuteAggregate.exactness_class(),
            OptionsExactnessClass::AggregateClass
        );
        assert_eq!(
            OptionsPriceSemantics::DerivedMidSnapshot.exactness_class(),
            OptionsExactnessClass::AggregateClass
        );

        let quote_cpu =
            OptionsPriceSemantics::TopOfBookQuote.exactness_semantics_for_backend("cpu_reference");
        assert_eq!(
            quote_cpu.retained_frontier_certification,
            OptionsRetainedFrontierCertification::CertifiedExact
        );
        assert_eq!(
            quote_cpu.exact_pruning_contract,
            OptionsExactPruningContract::ExactCertified
        );

        let quote_gpu = OptionsPriceSemantics::TopOfBookQuote
            .exactness_semantics_for_backend("gpu_compiled_tensor");
        assert_eq!(
            quote_gpu.retained_frontier_certification,
            OptionsRetainedFrontierCertification::ProvisionalMixedPrecision
        );
        assert_eq!(
            quote_gpu.exact_pruning_contract,
            OptionsExactPruningContract::RequiresIntervalCertificate
        );
        assert_eq!(
            quote_gpu.median_replay_contract,
            OptionsMedianReplayContract::RankingIrrelevantDeferred
        );

        let aggregate_cpu =
            OptionsPriceSemantics::MinuteAggregate.exactness_semantics_for_backend("cpu_reference");
        assert_eq!(
            aggregate_cpu.retained_frontier_certification,
            OptionsRetainedFrontierCertification::ProvisionalMissingExtremeLane
        );
        assert_eq!(
            aggregate_cpu.exact_pruning_contract,
            OptionsExactPruningContract::RequiresExactExtremeLaneAndIntervalCertificate
        );
        assert_eq!(
            aggregate_cpu.median_replay_contract,
            OptionsMedianReplayContract::RankingIrrelevantDeferred
        );
    }
}
