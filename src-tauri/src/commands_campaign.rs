use super::commands_meta::{
    run_meta_selection_blocking, run_meta_stage4_experimental_blocking, RunMetaSelectionResponse,
    RunMetaStage5Response,
};
use super::commands_optimize::{
    normalize_product_composite_cpu_optimization_mode,
    normalize_product_double_ma_optimization_mode, TauriProgressSink,
};
use super::run_artifacts;
use super::{
    BackendOptimizationResult, CompositeCpuRequest, CompositeMoRequest, CompositeMoResult,
    CompositeValidationRequest, DoubleMaRequest, DoubleMaValidationRequest, MetaSelectionRequest,
    MetaStage5Request,
};
use crate::data_cache::{self, LoadOutcome};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use ta_app_core::{
    progress::{DoubleMaCampaignVramPrefetchFeedback, DoubleMaProgressPayload, ProgressSink},
    Backend, BackendUsed, CrossTickerConfig, CscvPboConfig, MonteCarloValidationConfig,
    PerturbConfig, ReoptWalkForwardConfigOptional, SpaConfigOptional, TradeBootstrapConfig,
    ValidationAdmissionHeadProvenance, ValidationAdmissionPolicy, ValidationAdmissionProvenance,
    ValidationArtifactContext, WalkForwardConfigOptional,
};
use tauri::{Emitter, Manager};
use vector_ta::utilities::data_loader::Candles;

#[cfg(test)]
static CAMPAIGN_SUMMARY_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchCampaignPreloadPolicy {
    Disabled,
    HostOnly,
    HostPlusDoublemaVram,
}

impl Default for ResearchCampaignPreloadPolicy {
    fn default() -> Self {
        Self::HostOnly
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchCampaignExecutionMode {
    Full,
    OptimizeOnly,
    ValidateOnly,
    EvaluateCandidates,
}

impl Default for ResearchCampaignExecutionMode {
    fn default() -> Self {
        Self::Full
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ResearchCampaignDatasetInput {
    pub csv_path: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub timeframe: Option<String>,
    #[serde(default)]
    pub source_size_bytes: Option<u64>,
    #[serde(default)]
    pub source_mtime_utc_ms: Option<i64>,
    #[serde(default)]
    pub source_change_marker: Option<i64>,
    #[serde(default)]
    pub source_content_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResearchCampaignValidationConfig {
    #[serde(default = "default_validation_max_candidates")]
    pub max_candidates: usize,
    #[serde(default = "default_validation_seed")]
    pub seed: u64,
    #[serde(default = "default_validation_admission_policy")]
    pub admission_policy: ValidationAdmissionPolicy,
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
    pub monte_carlo: Option<MonteCarloValidationConfig>,
    #[serde(default)]
    pub cross_ticker: Option<CrossTickerConfig>,
    #[serde(default)]
    pub cscv_pbo: Option<CscvPboConfig>,
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
    pub fixed_candidates: Vec<ResearchCampaignVcetFixedCandidate>,
    /// Optional bounded multidimensional design for VCET. This is mutually
    /// exclusive with the legacy Fast/Slow fixed-candidate list.
    #[serde(default)]
    pub vcet_search: Option<ta_app_core::vcet_search::VcetSearchConfig>,
    #[serde(default)]
    pub require_gpu_kernel: bool,
    #[serde(default)]
    pub cross_universe_admission: ResearchCampaignCrossUniverseAdmissionPolicy,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct ResearchCampaignCrossUniverseAdmissionPolicy {
    pub min_profitable_share_requested: f64,
    pub min_hard_gate_pass_share_requested: f64,
    pub min_pnl_p10: f64,
    pub min_sharpe_p10: f64,
}

impl Default for ResearchCampaignCrossUniverseAdmissionPolicy {
    fn default() -> Self {
        Self {
            min_profitable_share_requested: 0.55,
            min_hard_gate_pass_share_requested: 0.50,
            min_pnl_p10: 0.0,
            min_sharpe_p10: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResearchCampaignVcetFixedCandidate {
    pub fast_len: u16,
    pub slow_len: u16,
}

fn default_validation_max_candidates() -> usize {
    32
}

fn default_validation_seed() -> u64 {
    1
}

fn default_validation_admission_policy() -> ValidationAdmissionPolicy {
    ValidationAdmissionPolicy::LegacyTopk
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResearchCampaignRequest {
    pub strategy_kind: String,
    pub datasets: Vec<ResearchCampaignDatasetInput>,
    #[serde(default)]
    pub timeframe_override: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub holdout_pct: u32,
    #[serde(default)]
    pub holdout_tag: Option<String>,
    #[serde(default)]
    pub skip_existing: bool,
    #[serde(default)]
    pub auto_meta: bool,
    #[serde(default)]
    pub auto_stage5: bool,
    #[serde(default)]
    pub execution_mode: ResearchCampaignExecutionMode,
    pub optimization_template: Value,
    #[serde(default)]
    pub validation_config: ResearchCampaignValidationConfig,
    #[serde(default)]
    pub meta_request: Option<MetaSelectionRequest>,
    #[serde(default)]
    pub stage5_request: Option<MetaStage5Request>,
    #[serde(default)]
    pub preload_policy: ResearchCampaignPreloadPolicy,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResearchCampaignCounts {
    pub total: usize,
    pub optimized: usize,
    pub validated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResearchCampaignItemDiagnostics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepare_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepare_wait_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_prefetch_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_prefetch_note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_prefetch_wall_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_prefetch_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_prefetch_attempted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_prefetch_success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_prefetch_failure_risk_event: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_prefetch_stage_cost_vs_optimize_frac: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram_prefetch_stage_cost_vs_prepare_wait_frac: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimize_wall_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimize_artifact_save_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_save_ms: Option<f64>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CampaignEvidenceState {
    None,
    Partial,
    Complete,
    Stale,
    Unsafe,
    LegacyDegraded,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CampaignResumeState {
    Resumable,
    RestartOnly,
    Complete,
    BlockedStale,
    BlockedUnsafe,
    LegacyDegraded,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResearchCampaignDatasetIdentitySummary {
    pub csv_path: String,
    pub label: String,
    pub timeframe: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_timeframe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_timeframe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_mtime_utc_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_change_marker: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_content_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ResearchCampaignManifestV2 {
    pub manifest_version: u32,
    pub manifest_hash: String,
    pub dataset_universe_hash: String,
    pub optimization_template_hash: String,
    pub validation_config_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_manifest_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_request_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage5_request_hash: Option<String>,
    pub holdout_split_hash: String,
    pub strategy_kind: String,
    pub execution_mode: ResearchCampaignExecutionMode,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holdout_tag: Option<String>,
    pub holdout_pct: u32,
    pub preload_policy: ResearchCampaignPreloadPolicy,
    pub strict_identity_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResearchCampaignRetentionSummary {
    pub class: String,
    pub cohort_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResearchCampaignItemSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    pub csv_path: String,
    pub label: String,
    pub timeframe: String,
    pub is_holdout: bool,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset_identity: Option<ResearchCampaignDatasetIdentitySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_state: Option<CampaignEvidenceState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_state: Option<CampaignResumeState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reuse_decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_xlsx_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<ResearchCampaignItemDiagnostics>,
    #[serde(skip)]
    fixed_candidate_result: Option<CampaignFixedCandidateDatasetResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResearchCampaignFixedCandidateAggregate {
    pub candidate_id: String,
    pub fast_len: u16,
    pub slow_len: u16,
    pub requested_datasets: usize,
    pub evaluated_datasets: usize,
    pub missing_or_failed_datasets: usize,
    pub profitable_datasets: usize,
    pub hard_gate_pass_datasets: usize,
    pub profitable_share_requested: f64,
    pub hard_gate_pass_share_requested: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pnl_p10: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pnl_median: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharpe_p10: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharpe_median: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_drawdown_p90: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trades_median: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ResearchCampaignFixedCandidateCohortAggregate {
    pub requested_datasets: usize,
    pub evaluated_datasets: usize,
    pub gpu_kernel_datasets: usize,
    pub missing_or_failed_datasets: usize,
    pub candidates: Vec<ResearchCampaignFixedCandidateAggregate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResearchCampaignFixedCandidateEvaluationSummary {
    pub schema_version: u32,
    pub candidate_manifest_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_manifest: Option<ta_app_core::vcet_search::VcetSearchManifest>,
    pub require_gpu_kernel: bool,
    /// Compatibility counts for the complete requested universe.
    pub requested_datasets: usize,
    pub evaluated_datasets: usize,
    pub gpu_kernel_datasets: usize,
    pub missing_or_failed_datasets: usize,
    /// Selection-safe rows. In schema v2 these are training-only.
    pub candidates: Vec<ResearchCampaignFixedCandidateAggregate>,
    #[serde(default)]
    pub selection_cohort: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_id: Option<String>,
    #[serde(default)]
    pub cross_universe_admission: ResearchCampaignCrossUniverseAdmissionPolicy,
    #[serde(default)]
    pub admitted_candidate_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub admitted_candidate_ids: Vec<String>,
    #[serde(default)]
    pub selection_status: String,
    #[serde(default)]
    pub all_universe: ResearchCampaignFixedCandidateCohortAggregate,
    #[serde(default)]
    pub training: ResearchCampaignFixedCandidateCohortAggregate,
    #[serde(default)]
    pub holdout: ResearchCampaignFixedCandidateCohortAggregate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResearchCampaignSummary {
    pub version: u32,
    pub kind: String,
    pub ts: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention: Option<ResearchCampaignRetentionSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<ResearchCampaignManifestV2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_state: Option<CampaignEvidenceState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_state: Option<CampaignResumeState>,
    pub strategy_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holdout_tag: Option<String>,
    pub holdout_pct: u32,
    pub request: Value,
    pub counts: ResearchCampaignCounts,
    pub items: Vec<ResearchCampaignItemSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_candidate_evaluation: Option<ResearchCampaignFixedCandidateEvaluationSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_json_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage5_json_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub runtime_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunResearchCampaignResponse {
    pub ts: String,
    pub summary_json_path: String,
    pub summary_json_bytes: usize,
    pub summary: ResearchCampaignSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<RunMetaSelectionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage5: Option<RunMetaStage5Response>,
}

impl ResearchCampaignItemSummary {
    fn with_campaign_metadata(
        mut self,
        item_id: Option<String>,
        dataset_identity: Option<ResearchCampaignDatasetIdentitySummary>,
        evidence_state: Option<CampaignEvidenceState>,
        resume_state: Option<CampaignResumeState>,
        reuse_decision: Option<String>,
    ) -> Self {
        self.item_id = item_id;
        self.dataset_identity = dataset_identity;
        self.evidence_state = evidence_state;
        self.resume_state = resume_state;
        self.reuse_decision = reuse_decision;
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchCampaignProgressPayload {
    pub stage: String,
    pub dataset_index: usize,
    pub dataset_total: usize,
    pub label: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_json_path: Option<String>,
    pub optimized: usize,
    pub validated: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CampaignOptimizationFamily {
    DoubleLike,
    CompositeCpu,
    CompositeMo,
}

enum CampaignOptimizationResult {
    Backend(BackendOptimizationResult),
    CompositeMo(CompositeMoResult),
    CompositeMoArtifact(Value),
}

#[derive(Clone, Debug)]
struct CampaignVcetFixedCandidatePlan {
    candidates: Vec<Value>,
    candidate_identities: Vec<CampaignVcetFixedCandidateIdentity>,
    candidate_manifest_hash: String,
    search_manifest: Option<ta_app_core::vcet_search::VcetSearchManifest>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CampaignVcetFixedCandidateIdentity {
    vidya_short_period: u16,
    vidya_long_period: u16,
    vidya_alpha_bits: u64,
    ce_period: u16,
    ce_mult_bits: u64,
    ce_use_close: bool,
    require_vidya_slope: bool,
    max_trade_bars: u16,
}

#[derive(Clone, Debug)]
pub(super) struct CampaignVcetFixedCandidateArtifactContract {
    plan: CampaignVcetFixedCandidatePlan,
    require_gpu_kernel: bool,
}

#[derive(Clone, Debug)]
pub(super) struct VcetTemporalCampaignCandidateSourceV1 {
    pub(super) request: ResearchCampaignRequest,
    pub(super) optimization_request: DoubleMaRequest,
    pub(super) sealed_candidate_manifest:
        super::commands_vcet_temporal::VcetTemporalSealedCandidateManifestV1,
    pub(super) cross_universe_admission: ResearchCampaignCrossUniverseAdmissionPolicy,
}

#[derive(Clone, Debug)]
struct CampaignFixedCandidateObservation {
    identity: CampaignVcetFixedCandidateIdentity,
    pnl: f64,
    sharpe: f64,
    max_drawdown: f64,
    trades: u32,
    hard_gate_passed: bool,
}

#[derive(Clone, Debug)]
struct CampaignFixedCandidateDatasetResult {
    gpu_kernel_used: bool,
    candidates: Vec<CampaignFixedCandidateObservation>,
}

enum CampaignFixedCandidateArtifactReuse {
    Reuse(CampaignFixedCandidateDatasetResult),
    Rerun { warning: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CampaignArtifactMatchDecision {
    Exact,
    Mismatch,
    Stale,
    Unsafe,
    LegacyDegraded,
}

impl CampaignArtifactMatchDecision {
    fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Mismatch => "mismatch",
            Self::Stale => "stale",
            Self::Unsafe => "unsafe",
            Self::LegacyDegraded => "legacy_degraded",
        }
    }

    fn blocks_reuse(self) -> bool {
        matches!(self, Self::Stale | Self::Unsafe | Self::LegacyDegraded)
    }
}

struct CampaignExistingArtifactMatch<'a> {
    summary: &'a run_artifacts::RunArtifactSummary,
    decision: CampaignArtifactMatchDecision,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum CampaignArtifactSourceIdentityKey {
    ContentFingerprint(String),
    SizeAndMtime(u64, i64),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct CampaignArtifactReuseKey {
    strategy_kind: String,
    config_fingerprint: String,
    csv_path: String,
    timeframe: String,
    cohort_tags: Vec<String>,
    source_identity: CampaignArtifactSourceIdentityKey,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct CampaignArtifactReuseCoarseKey {
    strategy_kind: String,
    config_fingerprint: String,
    csv_path: String,
    cohort_tags: Vec<String>,
}

struct CampaignArtifactReuseIndex<'a> {
    artifacts: &'a [run_artifacts::RunArtifactSummary],
    exact: HashMap<CampaignArtifactReuseKey, Vec<usize>>,
    coarse: HashMap<CampaignArtifactReuseCoarseKey, Vec<usize>>,
}

struct CampaignRunArtifactPruneGuard {
    app: Option<tauri::AppHandle>,
}

impl CampaignRunArtifactPruneGuard {
    fn new(app: Option<tauri::AppHandle>) -> Self {
        Self { app }
    }

    fn finish(&mut self) -> Result<(), String> {
        let Some(app) = self.app.take() else {
            return Ok(());
        };
        run_artifacts::prune_run_artifacts_to_retention_blocking(&app)
    }
}

impl Drop for CampaignRunArtifactPruneGuard {
    fn drop(&mut self) {
        if self.app.take().is_some() {
            log::warn!(
                "deferred campaign run-artifact retention was not finalized because no final campaign summary was published; leaving artifacts intact for recovery"
            );
        }
    }
}

fn campaign_defers_run_artifact_pruning(
    strategy_kind: &str,
    _execution_mode: ResearchCampaignExecutionMode,
) -> bool {
    normalize_strategy_kind(strategy_kind) == "Vcet"
}

#[cfg(test)]
fn planned_campaign_run_artifact_prunes(
    strategy_kind: &str,
    execution_mode: ResearchCampaignExecutionMode,
    artifact_saves: usize,
) -> usize {
    if campaign_defers_run_artifact_pruning(strategy_kind, execution_mode) {
        1
    } else {
        artifact_saves
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CampaignValidationObjective {
    Pnl,
    Sharpe,
    Sortino,
    Calmar,
    Omega,
    Volatility,
    MaxDrawdown,
}

impl CampaignValidationObjective {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pnl => "Pnl",
            Self::Sharpe => "Sharpe",
            Self::Sortino => "Sortino",
            Self::Calmar => "Calmar",
            Self::Omega => "Omega",
            Self::Volatility => "Volatility",
            Self::MaxDrawdown => "MaxDrawdown",
        }
    }
}

#[derive(Clone, Debug)]
struct CampaignValidationAdmissionRow {
    params_value: Value,
    metrics: Option<ta_strategies::double_ma::Metrics>,
    maximize_value: Option<f64>,
    minimize_value: Option<f64>,
    violation: Option<f64>,
    input_index: usize,
    source_kind: &'static str,
}

struct CampaignValidationAdmissionSelection {
    rows: Vec<CampaignValidationAdmissionRow>,
    provenance: ValidationAdmissionProvenance,
}

pub(super) struct PreparedCampaignDataset {
    pub(super) input: ResearchCampaignDatasetInput,
    pub(super) candles: vector_ta::utilities::data_loader::Candles,
    pub(super) dataset_meta: Value,
    pub(super) timeframe: String,
    pub(super) label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DatasetPrepareSource {
    PreloadedPack,
    BackgroundPreloadReady,
    BackgroundPreloadWaited,
    SyncPrepare,
}

impl DatasetPrepareSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::PreloadedPack => "preloaded_pack",
            Self::BackgroundPreloadReady => "background_preload_ready",
            Self::BackgroundPreloadWaited => "background_preload_waited",
            Self::SyncPrepare => "sync_prepare",
        }
    }
}

#[derive(Debug, Clone)]
struct CampaignPrefetchDiagnostics {
    outcome: &'static str,
    note: Option<String>,
    wall_ms: f64,
    policy: &'static str,
    attempted: bool,
    success: bool,
    failure_risk_event: bool,
}

impl CampaignPrefetchDiagnostics {
    fn not_attempted() -> Self {
        Self {
            outcome: "not_attempted",
            note: None,
            wall_ms: 0.0,
            policy: "heuristic_observational",
            attempted: false,
            success: false,
            failure_risk_event: false,
        }
    }
}

pub struct HeadlessPreparedResearchCampaignDatasetPack {
    datasets: VecDeque<PreparedCampaignDataset>,
}

struct DatasetPreloadState {
    receiver: mpsc::Receiver<Result<PreparedCampaignDataset, String>>,
    cached: Option<Result<PreparedCampaignDataset, String>>,
}

type SharedDatasetPreloadState = Arc<Mutex<DatasetPreloadState>>;

struct DatasetPreloadJob {
    state: SharedDatasetPreloadState,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl DatasetPreloadJob {
    fn spawn(
        input: ResearchCampaignDatasetInput,
        timeframe_override: Option<String>,
        app_cache_dir: PathBuf,
    ) -> Self {
        Self::spawn_with_task(move || {
            prepare_campaign_dataset(input, timeframe_override.as_deref(), &app_cache_dir)
        })
    }

    fn spawn_with_task<F>(task: F) -> Self
    where
        F: FnOnce() -> Result<PreparedCampaignDataset, String> + Send + 'static,
    {
        let (tx, rx) = mpsc::sync_channel(1);
        let handle = std::thread::spawn(move || {
            let _ = tx.send(task());
        });
        Self {
            state: Arc::new(Mutex::new(DatasetPreloadState {
                receiver: rx,
                cached: None,
            })),
            handle: Some(handle),
        }
    }

    fn shared_state(&self) -> SharedDatasetPreloadState {
        Arc::clone(&self.state)
    }

    fn join(mut self) -> Result<PreparedCampaignDataset, String> {
        self.join_worker()?;
        self.take_result()
    }

    fn join_with_prepare_metrics(self) -> (Result<PreparedCampaignDataset, String>, f64, bool) {
        let cached_ready = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            try_fill_preload_state(&mut state);
            state.cached.is_some()
        };
        let wait_started = std::time::Instant::now();
        let result = self.join();
        (
            result,
            wait_started.elapsed().as_secs_f64() * 1000.0,
            cached_ready,
        )
    }

    fn join_worker(&mut self) -> Result<(), String> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };
        handle
            .join()
            .map_err(|_| "campaign dataset preload worker panicked".to_string())
    }

    fn take_result(&self) -> Result<PreparedCampaignDataset, String> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(result) = state.cached.take() {
            return result;
        }
        match state.receiver.recv() {
            Ok(result) => result,
            Err(_) => Err("campaign dataset preload disconnected".to_string()),
        }
    }
}

impl Drop for DatasetPreloadJob {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn try_fill_preload_state(state: &mut DatasetPreloadState) {
    if state.cached.is_some() {
        return;
    }
    match state.receiver.try_recv() {
        Ok(result) => state.cached = Some(result),
        Err(mpsc::TryRecvError::Empty) => {}
        Err(mpsc::TryRecvError::Disconnected) => {
            state.cached = Some(Err("campaign dataset preload disconnected".to_string()))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CampaignDoubleMaVramPrefetchConfig {
    device_id: u32,
    fast_ma_type: String,
    slow_ma_type: String,
    ma_source: String,
}

struct CampaignDoubleMaVramPrefetch {
    preload_state: SharedDatasetPreloadState,
    config: CampaignDoubleMaVramPrefetchConfig,
    completed: AtomicBool,
    feedback: Mutex<Option<DoubleMaCampaignVramPrefetchFeedback>>,
}

impl CampaignDoubleMaVramPrefetch {
    fn with_request_if_ready(&self, f: &mut dyn FnMut(u32, &Candles, &str, &str, &str)) {
        if self.completed.load(Ordering::Relaxed) {
            return;
        }
        let mut preload = self.preload_state.lock().unwrap_or_else(|e| e.into_inner());
        try_fill_preload_state(&mut preload);
        let Some(result) = preload.cached.as_ref() else {
            return;
        };
        let Ok(prepared) = result else {
            self.completed.store(true, Ordering::Relaxed);
            return;
        };
        f(
            self.config.device_id,
            &prepared.candles,
            &self.config.fast_ma_type,
            &self.config.slow_ma_type,
            &self.config.ma_source,
        );
        self.completed.store(true, Ordering::Relaxed);
    }

    fn record_feedback(&self, feedback: DoubleMaCampaignVramPrefetchFeedback) {
        *self.feedback.lock().unwrap_or_else(|e| e.into_inner()) = Some(feedback);
    }

    fn take_feedback(&self) -> Option<DoubleMaCampaignVramPrefetchFeedback> {
        self.feedback
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }
}

struct CampaignProgressSink {
    app: tauri::AppHandle,
    vram_prefetch: Option<CampaignDoubleMaVramPrefetch>,
}

impl CampaignProgressSink {
    fn take_prefetch_feedback(&self) -> Option<DoubleMaCampaignVramPrefetchFeedback> {
        self.vram_prefetch
            .as_ref()
            .and_then(CampaignDoubleMaVramPrefetch::take_feedback)
    }
}

impl ProgressSink for CampaignProgressSink {
    fn emit_double_ma_progress(&self, payload: DoubleMaProgressPayload) {
        let _ = self.app.emit("double_ma_progress", payload);
    }

    fn with_double_ma_campaign_vram_prefetch_request(
        &self,
        f: &mut dyn FnMut(u32, &Candles, &str, &str, &str),
    ) {
        if let Some(prefetch) = self.vram_prefetch.as_ref() {
            prefetch.with_request_if_ready(f);
        }
    }

    fn on_double_ma_campaign_vram_prefetch_feedback(
        &self,
        feedback: DoubleMaCampaignVramPrefetchFeedback,
    ) {
        if let Some(prefetch) = self.vram_prefetch.as_ref() {
            prefetch.record_feedback(feedback);
        }
    }
}

fn normalize_text(value: &str) -> String {
    value.trim().to_string()
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => {
            if *value {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(values) => {
            let mut out = String::from("[");
            for (idx, item) in values.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json(item));
            }
            out.push(']');
            out
        }
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort_unstable();
            let mut out = String::from("{");
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(':');
                out.push_str(&canonical_json(map.get(*key).unwrap_or(&Value::Null)));
            }
            out.push('}');
            out
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn sha256_text_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    format!("sha256:{}", sha256_hex(&digest))
}

fn sha256_json_hash(value: &Value) -> String {
    sha256_text_hash(&canonical_json(value))
}

fn normalize_strategy_kind(raw: &str) -> String {
    match raw.trim() {
        "" => "DoubleMa".to_string(),
        "DoubleMa" => "DoubleMa".to_string(),
        "DynamicDoubleMa" => "DynamicDoubleMa".to_string(),
        "MlSignal" => "MlSignal".to_string(),
        "Pit" => "Pit".to_string(),
        "Set" => "Set".to_string(),
        "Mts" => "Mts".to_string(),
        "Orc" => "Orc".to_string(),
        "Marrt" => "Marrt".to_string(),
        "Vfct" => "Vfct".to_string(),
        "Rocb" => "Rocb".to_string(),
        "Vcet" => "Vcet".to_string(),
        "Mkcs" => "Mkcs".to_string(),
        "Kqsr" => "Kqsr".to_string(),
        "Carm" => "Carm".to_string(),
        "Kvdf" => "Kvdf".to_string(),
        "Cdmc" => "Cdmc".to_string(),
        "Asr" => "Asr".to_string(),
        "Vcc" => "Vcc".to_string(),
        "Vemr" => "Vemr".to_string(),
        "CompositeCpu" => "CompositeCpu".to_string(),
        "CompositeMo" => "CompositeMo".to_string(),
        "CompositeTrend" => "CompositeTrend".to_string(),
        "RegimeHybrid" => "RegimeHybrid".to_string(),
        "BreakoutMomentum" => "BreakoutMomentum".to_string(),
        other => other.to_string(),
    }
}

fn resolve_campaign_strategy_kind(raw: &str) -> Result<String, String> {
    let normalized = normalize_strategy_kind(raw);
    match normalized.as_str() {
        "DoubleMa"
        | "Pit"
        | "Set"
        | "Mts"
        | "Orc"
        | "Marrt"
        | "Vfct"
        | "Rocb"
        | "Vcet"
        | "Mkcs"
        | "Kqsr"
        | "Carm"
        | "Kvdf"
        | "Cdmc"
        | "Asr"
        | "Vcc"
        | "Vemr"
        | "CompositeCpu"
        | "CompositeMo"
        | "CompositeTrend"
        | "RegimeHybrid"
        | "BreakoutMomentum" => Ok(normalized),
        "DynamicDoubleMa" => Err(
            "Research campaigns do not support DynamicDoubleMa yet because validation does not preserve its dynamic strategy IR."
                .to_string(),
        ),
        "MlSignal" => Err(
            "Research campaigns do not support MlSignal. Use the dedicated ML Signal workflow instead."
                .to_string(),
        ),
        other => Err(format!(
            "Unsupported research campaign strategy kind '{other}'."
        )),
    }
}

fn is_double_like_kind(kind: &str) -> bool {
    matches!(
        normalize_strategy_kind(kind).as_str(),
        "DoubleMa"
            | "Pit"
            | "Set"
            | "Mts"
            | "Orc"
            | "Marrt"
            | "Vfct"
            | "CompositeTrend"
            | "RegimeHybrid"
            | "BreakoutMomentum"
    )
}

fn optimization_family_for_kind(kind: &str) -> CampaignOptimizationFamily {
    match normalize_strategy_kind(kind).as_str() {
        "CompositeMo" => CampaignOptimizationFamily::CompositeMo,
        "CompositeCpu" | "Asr" | "Vcc" | "Vemr" | "Rocb" | "Vcet" | "Mkcs" | "Kqsr" | "Carm"
        | "Kvdf" | "Cdmc" => CampaignOptimizationFamily::CompositeCpu,
        _ => CampaignOptimizationFamily::DoubleLike,
    }
}

fn vcet_fixed_candidate_identity(
    candidate: &Value,
) -> Result<CampaignVcetFixedCandidateIdentity, String> {
    let typed: ta_app_core::CompositeParams = serde_json::from_value(candidate.clone())
        .map_err(|error| format!("invalid expanded VCET fixed candidate: {error}"))?;
    let vcet = typed
        .vcet
        .as_ref()
        .ok_or_else(|| "expanded VCET fixed candidate is missing its VCET controls".to_string())?;
    let resolved =
        ta_app_core::vcet::resolve_vcet_periods_checked(vcet, typed.fast_len, typed.slow_len)?;
    Ok(CampaignVcetFixedCandidateIdentity {
        vidya_short_period: resolved.vidya_short,
        vidya_long_period: resolved.vidya_long,
        vidya_alpha_bits: vcet.vidya_alpha.to_bits(),
        ce_period: resolved.ce,
        ce_mult_bits: vcet.ce_mult.to_bits(),
        ce_use_close: vcet.ce_use_close,
        require_vidya_slope: vcet.require_vidya_slope,
        max_trade_bars: vcet.max_trade_bars,
    })
}

fn vcet_fixed_candidate_id(identity: CampaignVcetFixedCandidateIdentity) -> String {
    let hash = sha256_json_hash(&json!({
        "identity_schema_version": 1,
        "strategy_kind": "Vcet",
        "vidya_short_period": identity.vidya_short_period,
        "vidya_long_period": identity.vidya_long_period,
        "vidya_alpha": f64::from_bits(identity.vidya_alpha_bits),
        "ce_period": identity.ce_period,
        "ce_mult": f64::from_bits(identity.ce_mult_bits),
        "ce_use_close": identity.ce_use_close,
        "require_vidya_slope": identity.require_vidya_slope,
        "max_trade_bars": identity.max_trade_bars,
    }));
    format!("vcet_candidate_{}", hash.trim_start_matches("sha256:"))
}

fn build_vcet_fixed_candidate_plan(
    req: &ResearchCampaignRequest,
    strategy_kind: &str,
    execution_mode: ResearchCampaignExecutionMode,
) -> Result<Option<CampaignVcetFixedCandidatePlan>, String> {
    if execution_mode != ResearchCampaignExecutionMode::EvaluateCandidates {
        if !req.validation_config.fixed_candidates.is_empty()
            || req.validation_config.vcet_search.is_some()
            || req.validation_config.require_gpu_kernel
            || req.validation_config.cross_universe_admission
                != ResearchCampaignCrossUniverseAdmissionPolicy::default()
        {
            return Err(
                "validation_config.fixed_candidates, vcet_search, require_gpu_kernel, and non-default cross_universe_admission are only valid with execution_mode=evaluate_candidates"
                    .to_string(),
            );
        }
        return Ok(None);
    }
    if normalize_strategy_kind(strategy_kind) != "Vcet" {
        return Err(
            "execution_mode=evaluate_candidates currently supports only strategy_kind=Vcet"
                .to_string(),
        );
    }
    if req.validation_config.max_candidates == 0 {
        return Err(
            "evaluate_candidates requires validation_config.max_candidates to be greater than zero"
                .to_string(),
        );
    }
    validate_cross_universe_admission_policy(&req.validation_config.cross_universe_admission)?;

    let mut entries = req.validation_config.fixed_candidates.clone();
    if !entries.is_empty() && req.validation_config.vcet_search.is_some() {
        return Err(
            "evaluate_candidates accepts either validation_config.fixed_candidates or validation_config.vcet_search, not both"
                .to_string(),
        );
    }
    if entries.is_empty() && req.validation_config.vcet_search.is_none() {
        return Err(
            "evaluate_candidates requires validation_config.fixed_candidates or validation_config.vcet_search"
                .to_string(),
        );
    }
    if entries.len() > req.validation_config.max_candidates {
        return Err(format!(
            "evaluate_candidates received {} fixed candidates, exceeding validation_config.max_candidates={}; the manifest will not be truncated",
            entries.len(), req.validation_config.max_candidates
        ));
    }
    entries.sort_unstable();
    if entries.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("VCET fixed-candidate manifest contains a duplicate period pair".to_string());
    }

    let template = req
        .optimization_template
        .as_object()
        .ok_or_else(|| "campaign optimization_template must be an object".to_string())?;
    let one_ma_type = |key: &str| -> Result<String, String> {
        let values = template
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("VCET evaluate_candidates requires {key} to be an array"))?;
        if values.len() != 1 {
            return Err(format!(
                "VCET evaluate_candidates requires exactly one {key} entry"
            ));
        }
        values[0]
            .as_str()
            .map(normalize_text)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("VCET evaluate_candidates requires a non-empty {key} entry"))
    };
    let fast_ma_type = one_ma_type("fast_ma_types")?;
    let slow_ma_type = one_ma_type("slow_ma_types")?;

    for key in [
        "asr", "vcc", "vemr", "rocb", "mkcs", "kqsr", "carm", "kvdf", "cdmc",
    ] {
        if template.get(key).is_some_and(|value| !value.is_null()) {
            return Err(format!(
                "VCET evaluate_candidates optimization_template must not include the '{key}' strategy payload"
            ));
        }
    }
    if template.get("halftrend").is_none_or(Value::is_null) {
        return Err(
            "VCET evaluate_candidates optimization_template requires a halftrend configuration"
                .to_string(),
        );
    }
    let vcet_value = template
        .get("vcet")
        .filter(|value| !value.is_null())
        .ok_or_else(|| {
            "VCET evaluate_candidates optimization_template requires a vcet configuration"
                .to_string()
        })?;
    let vcet: ta_app_core::VcetStrategyParams = serde_json::from_value(vcet_value.clone())
        .map_err(|error| format!("invalid VCET strategy configuration: {error}"))?;
    ta_app_core::vcet::validate_vcet_params(&vcet)?;

    if req.validation_config.require_gpu_kernel {
        let backend = normalize_backend_value(template.get("backend"), strategy_kind);
        if backend.get("backend").and_then(Value::as_str) != Some("GpuKernel") {
            return Err(
                "require_gpu_kernel=true requires optimization_template.backend=GpuKernel for VCET fixed-candidate evaluation"
                    .to_string(),
            );
        }

        let mut non_gpu_resident_stages = Vec::new();
        if req.validation_config.walk_forward.is_some() {
            non_gpu_resident_stages.push("walk_forward");
        }
        if req.validation_config.reopt_walk_forward.is_some() {
            non_gpu_resident_stages.push("reopt_walk_forward");
        }
        if req.validation_config.spa.is_some() {
            non_gpu_resident_stages.push("spa");
        }
        if req.validation_config.perturb.is_some() {
            non_gpu_resident_stages.push("perturb");
        }
        if req.validation_config.trade_bootstrap.is_some() {
            non_gpu_resident_stages.push("trade_bootstrap");
        }
        if req.validation_config.monte_carlo.is_some() {
            non_gpu_resident_stages.push("monte_carlo");
        }
        if req.validation_config.cross_ticker.is_some() {
            non_gpu_resident_stages.push("cross_ticker");
        }
        if req.validation_config.cscv_pbo.is_some() {
            non_gpu_resident_stages.push("cscv_pbo");
        }
        if !non_gpu_resident_stages.is_empty() {
            return Err(format!(
                "require_gpu_kernel=true does not permit auxiliary validation stages that are not proven fully GPU-resident; set these validation_config fields to null: {}",
                non_gpu_resident_stages.join(", ")
            ));
        }
    }

    let mut search_manifest = req
        .validation_config
        .vcet_search
        .as_ref()
        .map(ta_app_core::vcet_search::generate_vcet_search_manifest)
        .transpose()?;
    if let Some(manifest) = search_manifest.as_ref() {
        if manifest.effective_budget > req.validation_config.max_candidates {
            return Err(format!(
                "VCET search generated {} candidates, exceeding validation_config.max_candidates={}; the manifest will not be truncated",
                manifest.effective_budget, req.validation_config.max_candidates
            ));
        }
    }

    let params = serde_json::Map::new();
    let planned_count = search_manifest
        .as_ref()
        .map(|manifest| manifest.candidates.len())
        .unwrap_or(entries.len());
    let mut candidates = Vec::with_capacity(planned_count);
    let mut candidate_identities = Vec::with_capacity(planned_count);
    let mut seen_identities = BTreeSet::new();
    let mut append_candidate = |fast_len: u16,
                                slow_len: u16,
                                vcet_override: Option<Value>|
     -> Result<(), String> {
        let mut candidate = build_composite_candidate_value(
            strategy_kind,
            &req.optimization_template,
            &params,
            fast_ma_type.clone(),
            slow_ma_type.clone(),
            u64::from(fast_len),
            u64::from(slow_len),
        );
        if let Some(vcet_override) = vcet_override {
            candidate["vcet"] = vcet_override;
        }
        let typed: ta_app_core::CompositeParams = serde_json::from_value(candidate)
            .map_err(|error| format!("invalid expanded VCET fixed candidate: {error}"))?;
        let candidate = serde_json::to_value(typed)
            .map_err(|error| format!("failed to encode VCET fixed candidate: {error}"))?;
        let identity = vcet_fixed_candidate_identity(&candidate)?;
        if !seen_identities.insert(identity) {
            return Err(format!(
                "VCET fixed-candidate manifest contains a duplicate effective strategy identity after resolving candidate ({}, {}): {}",
                fast_len,
                slow_len,
                vcet_fixed_candidate_id(identity),
            ));
        }
        candidates.push(candidate);
        candidate_identities.push(identity);
        Ok(())
    };

    if let Some(manifest) = search_manifest.as_ref() {
        for sampled in &manifest.candidates {
            let effective = &sampled.effective;
            append_candidate(
                effective.vidya_short_period,
                effective.vidya_long_period,
                Some(json!({
                    "map_fast_to_vidya_short_period": false,
                    "map_slow_to_vidya_long_period": false,
                    "map_slow_to_ce_period": false,
                    "vidya_short_period": effective.vidya_short_period,
                    "vidya_long_period": effective.vidya_long_period,
                    "vidya_alpha": effective.vidya_alpha,
                    "ce_period": effective.ce_period,
                    "ce_mult": effective.ce_mult,
                    "ce_use_close": effective.ce_use_close,
                    "require_vidya_slope": effective.require_vidya_slope,
                    "max_trade_bars": effective.max_trade_bars,
                })),
            )?;
        }
    } else {
        for entry in entries {
            append_candidate(entry.fast_len, entry.slow_len, None)?;
        }
    }
    if let Some(manifest) = search_manifest.as_mut() {
        if manifest.candidates.len() != candidate_identities.len() {
            return Err(
                "VCET search manifest and expanded candidate identity table differ in length"
                    .to_string(),
            );
        }
        for (sampled, identity) in manifest
            .candidates
            .iter_mut()
            .zip(candidate_identities.iter().copied())
        {
            sampled.evaluation_candidate_id = Some(vcet_fixed_candidate_id(identity));
        }
    }
    let candidate_manifest_hash = sha256_json_hash(&json!({
        "schema_version": 1,
        "strategy_kind": "Vcet",
        "candidates": candidates,
    }));
    Ok(Some(CampaignVcetFixedCandidatePlan {
        candidates,
        candidate_identities,
        candidate_manifest_hash,
        search_manifest,
    }))
}

fn validate_cross_universe_admission_policy(
    policy: &ResearchCampaignCrossUniverseAdmissionPolicy,
) -> Result<(), String> {
    for (name, value) in [
        (
            "min_profitable_share_requested",
            policy.min_profitable_share_requested,
        ),
        (
            "min_hard_gate_pass_share_requested",
            policy.min_hard_gate_pass_share_requested,
        ),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!(
                "validation_config.cross_universe_admission.{name} must be finite and between 0 and 1"
            ));
        }
    }
    for (name, value) in [
        ("min_pnl_p10", policy.min_pnl_p10),
        ("min_sharpe_p10", policy.min_sharpe_p10),
    ] {
        if !value.is_finite() {
            return Err(format!(
                "validation_config.cross_universe_admission.{name} must be finite"
            ));
        }
    }
    Ok(())
}

fn normalize_path_key(raw: &str) -> String {
    raw.trim().replace('/', "\\").to_ascii_lowercase()
}

fn preload_enabled(policy: ResearchCampaignPreloadPolicy) -> bool {
    !matches!(policy, ResearchCampaignPreloadPolicy::Disabled)
}

fn requested_gpu_device_id(backend: &Backend) -> Option<u32> {
    match backend {
        Backend::Auto { device_id }
        | Backend::GpuOnly { device_id }
        | Backend::GpuHybridKernel { device_id }
        | Backend::GpuKernel { device_id } => Some(*device_id),
        Backend::CpuOnly => None,
    }
}

fn supports_exact_gpu_kernel_route(result: &BackendOptimizationResult) -> bool {
    matches!(result.backend_used, BackendUsed::Gpu)
        && result
            .backend_route
            .as_deref()
            .map(|route| route.contains("GpuKernel"))
            .unwrap_or(false)
}

fn build_doublema_vram_prefetch_config(
    run_req: &DoubleMaRequest,
    learned_device_id: Option<u32>,
) -> Option<CampaignDoubleMaVramPrefetchConfig> {
    if run_req.dynamic_strategy_ir.is_some() {
        return None;
    }
    if run_req.fast_ma_types.len() != 1 || run_req.slow_ma_types.len() != 1 {
        return None;
    }
    let device_id = match &run_req.backend {
        Backend::GpuKernel { device_id } => Some(*device_id),
        Backend::Auto { .. } => learned_device_id,
        _ => None,
    }?;
    let fast_ma_type = normalize_text(&run_req.fast_ma_types[0]).to_ascii_lowercase();
    let slow_ma_type = normalize_text(&run_req.slow_ma_types[0]).to_ascii_lowercase();
    if fast_ma_type.is_empty() || slow_ma_type.is_empty() {
        return None;
    }
    let ma_source = {
        let source = normalize_text(&run_req.ma_source);
        if source.is_empty() {
            "close".to_string()
        } else {
            source
        }
    };
    Some(CampaignDoubleMaVramPrefetchConfig {
        device_id,
        fast_ma_type,
        slow_ma_type,
        ma_source,
    })
}

fn normalize_timeframe_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    ta_app_core::parse_timeframe_label(trimmed)
        .map(|tf| tf.label)
        .unwrap_or_else(|_| trimmed.to_ascii_lowercase())
}

fn push_unique_warning(warnings: &mut Vec<String>, warning: String) {
    if warning.trim().is_empty() {
        return;
    }
    if !warnings.iter().any(|existing| existing == &warning) {
        warnings.push(warning);
    }
}

fn file_label_from_path(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn dataset_label_from_input(input: &ResearchCampaignDatasetInput) -> String {
    input
        .label
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| file_label_from_path(&input.csv_path))
}

fn dataset_timeframe_hint(
    input: &ResearchCampaignDatasetInput,
    timeframe_override: Option<&str>,
) -> String {
    timeframe_override
        .map(normalize_timeframe_label)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            input
                .timeframe
                .as_deref()
                .map(normalize_timeframe_label)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
}

fn value_string(meta: &Value, key: &str) -> Option<String> {
    meta.get(key)
        .and_then(Value::as_str)
        .map(normalize_text)
        .filter(|value| !value.is_empty())
}

fn dataset_identity_summary(
    prepared: &PreparedCampaignDataset,
) -> ResearchCampaignDatasetIdentitySummary {
    ResearchCampaignDatasetIdentitySummary {
        csv_path: prepared.input.csv_path.clone(),
        label: prepared.label.clone(),
        timeframe: prepared.timeframe.clone(),
        base_timeframe: value_string(&prepared.dataset_meta, "base_timeframe"),
        derived_timeframe: value_string(&prepared.dataset_meta, "derived_timeframe"),
        source_size_bytes: prepared
            .dataset_meta
            .get("source_size_bytes")
            .and_then(Value::as_u64)
            .or(prepared.input.source_size_bytes),
        source_mtime_utc_ms: prepared
            .dataset_meta
            .get("source_mtime_utc_ms")
            .and_then(Value::as_i64)
            .or(prepared.input.source_mtime_utc_ms),
        source_change_marker: prepared
            .dataset_meta
            .get("source_change_marker")
            .and_then(Value::as_i64)
            .or(prepared.input.source_change_marker),
        source_content_fingerprint: value_string(
            &prepared.dataset_meta,
            "source_content_fingerprint",
        )
        .or_else(|| prepared.input.source_content_fingerprint.clone()),
    }
}

fn campaign_item_id(manifest_hash: &str, index: usize, csv_path: &str, timeframe: &str) -> String {
    let hash = sha256_json_hash(&json!({
        "manifest_hash": manifest_hash,
        "index": index,
        "csv_path": normalize_path_key(csv_path),
        "timeframe": normalize_timeframe_label(timeframe),
    }));
    format!("campaign_item_{}", hash.trim_start_matches("sha256:"))
}

fn normalize_campaign_datasets(
    datasets: Vec<ResearchCampaignDatasetInput>,
) -> Result<Vec<ResearchCampaignDatasetInput>, String> {
    let mut normalized = Vec::with_capacity(datasets.len());
    let mut seen_paths = HashSet::with_capacity(datasets.len());
    let mut seen_fingerprints: HashMap<String, String> = HashMap::with_capacity(datasets.len());
    for dataset in datasets {
        let csv_path = normalize_text(&dataset.csv_path);
        if csv_path.is_empty() {
            return Err("campaign dataset csv_path is empty".to_string());
        }
        let mut lexical_path = PathBuf::new();
        for component in Path::new(&csv_path).components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir if lexical_path.pop() => {}
                Component::ParentDir => lexical_path.push(component.as_os_str()),
                _ => lexical_path.push(component.as_os_str()),
            }
        }
        let path_identity = normalize_path_key(lexical_path.to_string_lossy().as_ref());
        if !seen_paths.insert(path_identity) {
            return Err(format!(
                "campaign datasets contain a duplicate normalized csv_path: {csv_path}"
            ));
        }
        let source_content_fingerprint =
            normalize_optional_text(dataset.source_content_fingerprint);
        if let Some(fingerprint) = source_content_fingerprint.as_deref() {
            let fingerprint_identity = fingerprint.to_ascii_lowercase();
            if let Some(previous_path) =
                seen_fingerprints.insert(fingerprint_identity, csv_path.clone())
            {
                return Err(format!(
                    "campaign datasets '{previous_path}' and '{csv_path}' declare the same source_content_fingerprint; duplicate source content cannot be counted twice"
                ));
            }
        }
        normalized.push(ResearchCampaignDatasetInput {
            csv_path,
            label: normalize_optional_text(dataset.label),
            timeframe: normalize_optional_text(dataset.timeframe)
                .map(|value| normalize_timeframe_label(&value))
                .filter(|value| !value.is_empty()),
            source_size_bytes: dataset.source_size_bytes,
            source_mtime_utc_ms: dataset.source_mtime_utc_ms,
            source_change_marker: dataset.source_change_marker,
            source_content_fingerprint,
        });
    }
    Ok(normalized)
}

fn refresh_campaign_dataset_source_identities(
    datasets: &mut [ResearchCampaignDatasetInput],
) -> Result<(), String> {
    for dataset in datasets {
        let csv_path = normalize_text(&dataset.csv_path);
        let source_identity = data_cache::identity_for_source(Path::new(&csv_path)).map_err(|error| {
            format!(
                "failed to seal the campaign source identity for '{csv_path}' before creating its manifest: {error}"
            )
        })?;
        if let Some(expected_size) = dataset.source_size_bytes {
            if expected_size != source_identity.size {
                return Err(format!(
                    "campaign dataset source_size_bytes is stale for '{csv_path}': expected {expected_size}, observed {}; refresh dataset discovery before running the campaign",
                    source_identity.size
                ));
            }
        }
        if let Some(expected_mtime) = dataset.source_mtime_utc_ms {
            if expected_mtime != source_identity.mtime_utc_ms {
                return Err(format!(
                    "campaign dataset source_mtime_utc_ms is stale for '{csv_path}': expected {expected_mtime}, observed {}; refresh dataset discovery before running the campaign",
                    source_identity.mtime_utc_ms
                ));
            }
        }
        if let Some(expected_change_marker) = dataset.source_change_marker {
            if expected_change_marker != source_identity.change_marker {
                return Err(format!(
                    "campaign dataset source_change_marker is stale for '{csv_path}': expected {expected_change_marker}, observed {}; refresh dataset discovery before running the campaign",
                    source_identity.change_marker
                ));
            }
        }
        dataset.source_size_bytes = Some(source_identity.size);
        dataset.source_mtime_utc_ms = Some(source_identity.mtime_utc_ms);
        dataset.source_change_marker = Some(source_identity.change_marker);
    }
    Ok(())
}

fn record_unique_prepared_campaign_source(
    prepared: &PreparedCampaignDataset,
    seen_fingerprints: &mut HashMap<String, String>,
) -> Result<(), String> {
    let Some(fingerprint) = value_string(&prepared.dataset_meta, "source_content_fingerprint")
        .or_else(|| {
            prepared
                .input
                .source_content_fingerprint
                .as_deref()
                .map(normalize_text)
                .filter(|value| !value.is_empty())
        })
    else {
        return Ok(());
    };
    let csv_path = normalize_text(&prepared.input.csv_path);
    if let Some(previous_path) =
        seen_fingerprints.insert(fingerprint.to_ascii_lowercase(), csv_path.clone())
    {
        return Err(format!(
            "campaign datasets '{previous_path}' and '{csv_path}' resolve to the same source content identity '{fingerprint}'; duplicate source content cannot be counted twice"
        ));
    }
    Ok(())
}

fn should_load_existing_artifacts(
    skip_existing: bool,
    execution_mode: ResearchCampaignExecutionMode,
) -> bool {
    skip_existing || execution_mode == ResearchCampaignExecutionMode::ValidateOnly
}

fn load_existing_campaign_artifacts<F>(
    skip_existing: bool,
    execution_mode: ResearchCampaignExecutionMode,
    load: F,
) -> Result<Vec<run_artifacts::RunArtifactSummary>, String>
where
    F: FnOnce() -> Result<Vec<run_artifacts::RunArtifactSummary>, String>,
{
    if !should_load_existing_artifacts(skip_existing, execution_mode) {
        return Ok(Vec::new());
    }

    load().map_err(|error| {
        format!("failed to load existing run artifacts for campaign reuse: {error}")
    })
}

fn failed_item(
    csv_path: String,
    label: String,
    timeframe: String,
    is_holdout: bool,
    artifact_path: Option<String>,
    stage: &str,
    error: impl Into<String>,
) -> ResearchCampaignItemSummary {
    ResearchCampaignItemSummary {
        item_id: None,
        csv_path,
        label,
        timeframe,
        is_holdout,
        status: "failed".to_string(),
        dataset_identity: None,
        evidence_state: Some(CampaignEvidenceState::Partial),
        resume_state: Some(CampaignResumeState::Resumable),
        reuse_decision: None,
        artifact_path,
        artifact_sha256: None,
        report_xlsx_path: None,
        error: Some(format!("{stage}: {}", error.into())),
        diagnostics: None,
        fixed_candidate_result: None,
    }
}

fn build_item_diagnostics(
    prepare_source: DatasetPrepareSource,
    prepare_wait_ms: f64,
    prefetch: &CampaignPrefetchDiagnostics,
    optimize_wall_ms: Option<f64>,
    optimize_artifact_save_ms: Option<f64>,
    checkpoint_save_ms: Option<f64>,
) -> ResearchCampaignItemDiagnostics {
    let stage_cost_vs_optimize_frac = optimize_wall_ms
        .filter(|value| *value > 0.0)
        .map(|value| prefetch.wall_ms / value);
    let stage_cost_vs_prepare_wait_frac =
        (prepare_wait_ms > 0.0).then_some(prefetch.wall_ms / prepare_wait_ms);
    ResearchCampaignItemDiagnostics {
        prepare_source: Some(prepare_source.as_str().to_string()),
        prepare_wait_ms: Some(prepare_wait_ms),
        vram_prefetch_outcome: Some(prefetch.outcome.to_string()),
        vram_prefetch_note: prefetch.note.clone(),
        vram_prefetch_wall_ms: Some(prefetch.wall_ms),
        vram_prefetch_policy: Some(prefetch.policy.to_string()),
        vram_prefetch_attempted: Some(prefetch.attempted),
        vram_prefetch_success: Some(prefetch.success),
        vram_prefetch_failure_risk_event: Some(prefetch.failure_risk_event),
        vram_prefetch_stage_cost_vs_optimize_frac: stage_cost_vs_optimize_frac,
        vram_prefetch_stage_cost_vs_prepare_wait_frac: stage_cost_vs_prepare_wait_frac,
        optimize_wall_ms,
        optimize_artifact_save_ms,
        checkpoint_save_ms,
    }
}

fn record_prefetch_feedback(
    warnings: &mut Vec<String>,
    doublema_vram_prefetch_disabled: &mut bool,
    feedback: Option<DoubleMaCampaignVramPrefetchFeedback>,
) -> CampaignPrefetchDiagnostics {
    let Some(feedback) = feedback else {
        return CampaignPrefetchDiagnostics::not_attempted();
    };
    match feedback {
        DoubleMaCampaignVramPrefetchFeedback::Staged { stage_wall_ms } => {
            CampaignPrefetchDiagnostics {
                outcome: "staged",
                note: None,
                wall_ms: stage_wall_ms,
                policy: "heuristic_observational",
                attempted: true,
                success: true,
                failure_risk_event: false,
            }
        }
        DoubleMaCampaignVramPrefetchFeedback::AlreadyResident { stage_wall_ms } => {
            CampaignPrefetchDiagnostics {
                outcome: "already_resident",
                note: None,
                wall_ms: stage_wall_ms,
                policy: "heuristic_observational",
                attempted: true,
                success: true,
                failure_risk_event: false,
            }
        }
        DoubleMaCampaignVramPrefetchFeedback::Warning {
            note,
            stage_wall_ms,
        } => {
            push_unique_warning(warnings, note.clone());
            CampaignPrefetchDiagnostics {
                outcome: "warning",
                note: Some(note),
                wall_ms: stage_wall_ms,
                policy: "heuristic_observational",
                attempted: true,
                success: false,
                failure_risk_event: true,
            }
        }
        DoubleMaCampaignVramPrefetchFeedback::Fatal {
            note,
            stage_wall_ms,
        } => {
            push_unique_warning(warnings, note.clone());
            *doublema_vram_prefetch_disabled = true;
            CampaignPrefetchDiagnostics {
                outcome: "fatal",
                note: Some(note),
                wall_ms: stage_wall_ms,
                policy: "heuristic_observational",
                attempted: true,
                success: false,
                failure_risk_event: true,
            }
        }
    }
}

fn default_campaign_tag(ts: &str) -> String {
    let mut compact = String::with_capacity(ts.len());
    for ch in ts.chars() {
        if ch.is_ascii_alphanumeric() {
            compact.push(ch);
        }
    }
    if compact.is_empty() {
        "campaign".to_string()
    } else {
        format!("campaign-{compact}")
    }
}

fn dedup_tags(tags: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for tag in tags {
        let next = normalize_text(&tag);
        if next.is_empty() {
            continue;
        }
        let key = next.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(next);
        }
    }
    out
}

fn validate_holdout_tag_isolation(
    campaign_tags: &[String],
    datasets: &[ResearchCampaignDatasetInput],
    holdout_pct: u32,
    holdout_tag: Option<&str>,
) -> Result<(), String> {
    if holdout_pct == 0 {
        return Ok(());
    }
    let holdout_tag = holdout_tag.map(normalize_text).unwrap_or_default();
    if holdout_tag.is_empty() {
        return Err(
            "campaign holdout_tag must be non-empty when holdout_pct is positive".to_string(),
        );
    }
    if campaign_tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case(&holdout_tag))
    {
        return Err(format!(
            "campaign holdout_tag '{holdout_tag}' must not also appear in base campaign tags; training and holdout artifact cohorts require disjoint tags"
        ));
    }
    if datasets
        .iter()
        .any(|dataset| dataset_label_from_input(dataset).eq_ignore_ascii_case(&holdout_tag))
    {
        return Err(format!(
            "campaign holdout_tag '{holdout_tag}' must not match a dataset label; legacy tag-based readers could misclassify a training artifact as holdout"
        ));
    }
    Ok(())
}

fn hash_fnv1a32(text: &str) -> u32 {
    let mut hash: u32 = 2166136261;
    for ch in text.chars() {
        hash ^= ch as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

fn split_holdout_paths(
    paths: &[String],
    holdout_pct: u32,
    seed: &str,
) -> (Vec<String>, HashSet<String>) {
    let pct = holdout_pct.min(50);
    if paths.is_empty() || pct == 0 {
        return (paths.to_vec(), HashSet::new());
    }
    let total = paths.len();
    let holdout_count = ((total as f64) * (pct as f64 / 100.0)).floor() as usize;
    if holdout_count == 0 {
        return (paths.to_vec(), HashSet::new());
    }
    let seed32 = hash_fnv1a32(seed);
    let mut scored: Vec<(String, u32)> = paths
        .iter()
        .cloned()
        .map(|path| {
            let score = hash_fnv1a32(&path) ^ seed32;
            (path, score)
        })
        .collect();
    scored.sort_by(|left, right| left.1.cmp(&right.1));
    let holdout: HashSet<String> = scored
        .iter()
        .take(holdout_count)
        .map(|(path, _)| path.clone())
        .collect();
    let train: Vec<String> = scored
        .into_iter()
        .filter_map(|(path, _)| (!holdout.contains(&path)).then_some(path))
        .collect();
    (train, holdout)
}

fn emit_campaign_progress(
    app: &tauri::AppHandle,
    stage: &str,
    dataset_index: usize,
    dataset_total: usize,
    label: &str,
    message: impl Into<String>,
    summary_json_path: Option<&str>,
    counts: &ResearchCampaignCounts,
) {
    let _ = app.emit(
        "research_campaign_progress",
        ResearchCampaignProgressPayload {
            stage: stage.to_string(),
            dataset_index,
            dataset_total,
            label: label.to_string(),
            message: message.into(),
            summary_json_path: summary_json_path.map(|value| value.to_string()),
            optimized: counts.optimized,
            validated: counts.validated,
            skipped: counts.skipped,
            failed: counts.failed,
        },
    );
}

pub(super) fn prepare_campaign_dataset(
    mut input: ResearchCampaignDatasetInput,
    timeframe_override: Option<&str>,
    app_cache_dir: &Path,
) -> Result<PreparedCampaignDataset, String> {
    let csv_path = normalize_text(&input.csv_path);
    if csv_path.is_empty() {
        return Err("campaign dataset csv_path is empty".to_string());
    }

    let LoadOutcome {
        mut candles,
        source_identity,
        source_content_fingerprint,
        ..
    } = data_cache::load_or_build(Path::new(&csv_path), app_cache_dir)?;

    if let Some(expected_size) = input.source_size_bytes {
        if expected_size != source_identity.size {
            return Err(format!(
                "campaign dataset source_size_bytes is stale for '{csv_path}': expected {expected_size}, observed {}; refresh dataset discovery before running the campaign",
                source_identity.size
            ));
        }
    }
    if let Some(expected_mtime_utc_ms) = input.source_mtime_utc_ms {
        if expected_mtime_utc_ms != source_identity.mtime_utc_ms {
            return Err(format!(
                "campaign dataset source_mtime_utc_ms is stale for '{csv_path}': expected {expected_mtime_utc_ms}, observed {}; refresh dataset discovery before running the campaign",
                source_identity.mtime_utc_ms
            ));
        }
    }
    if let Some(expected_change_marker) = input.source_change_marker {
        if expected_change_marker != source_identity.change_marker {
            return Err(format!(
                "campaign dataset source_change_marker is stale for '{csv_path}': expected {expected_change_marker}, observed {}; refresh dataset discovery before running the campaign",
                source_identity.change_marker
            ));
        }
    }
    if let Some(expected_fingerprint) = input
        .source_content_fingerprint
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
    {
        if expected_fingerprint != source_content_fingerprint {
            return Err(format!(
                "campaign dataset source_content_fingerprint is stale for '{csv_path}': expected '{expected_fingerprint}', observed '{source_content_fingerprint}'; refresh dataset discovery before running the campaign"
            ));
        }
    }
    input.source_size_bytes = Some(source_identity.size);
    input.source_mtime_utc_ms = Some(source_identity.mtime_utc_ms);
    input.source_change_marker = Some(source_identity.change_marker);
    input.source_content_fingerprint = Some(source_content_fingerprint.clone());

    let base_timeframe = input
        .timeframe
        .as_deref()
        .map(normalize_timeframe_label)
        .unwrap_or_default();
    let mut effective_timeframe = base_timeframe.clone();
    if let Some(raw_tf) = timeframe_override {
        let target = normalize_timeframe_label(raw_tf);
        if !target.is_empty() {
            let spec = ta_app_core::parse_timeframe_label(raw_tf)?;
            if target != base_timeframe {
                candles = ta_app_core::resample_candles(&candles, spec.interval_ms)?;
            }
            effective_timeframe = spec.label;
        }
    }

    let label = input
        .label
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| file_label_from_path(&csv_path));

    let mut dataset_meta = json!({
        "csv_path": csv_path,
        "label": label,
        "symbol": label,
        "timeframe": effective_timeframe,
        "base_timeframe": if base_timeframe.is_empty() { effective_timeframe.clone() } else { base_timeframe.clone() },
        "derived_timeframe": effective_timeframe,
        "resample_applied": !base_timeframe.is_empty() && !effective_timeframe.is_empty() && base_timeframe != effective_timeframe,
        "source_size_bytes": source_identity.size,
        "source_mtime_utc_ms": source_identity.mtime_utc_ms,
        "source_change_marker": source_identity.change_marker,
    });
    dataset_meta["source_content_fingerprint"] = Value::String(source_content_fingerprint);

    Ok(PreparedCampaignDataset {
        input,
        candles,
        dataset_meta,
        timeframe: effective_timeframe,
        label,
    })
}

fn make_campaign_data_id(index: usize, csv_path: &str, timeframe: &str) -> String {
    let label = file_label_from_path(csv_path)
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    let tf = timeframe
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    format!("campaign_{}_{}_{}", index + 1, label, tf)
}

fn patch_optimization_template(
    template: &Value,
    strategy_kind: &str,
    data_id: &str,
) -> Result<Value, String> {
    let mut req = template.clone();
    let Some(map) = req.as_object_mut() else {
        return Err("campaign optimization_template must be an object".to_string());
    };
    map.insert("data_id".to_string(), Value::String(data_id.to_string()));
    map.insert(
        "strategy_kind".to_string(),
        Value::String(normalize_strategy_kind(strategy_kind)),
    );
    Ok(req)
}

fn run_optimization_blocking(
    family: CampaignOptimizationFamily,
    req_value: &Value,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    sink: Option<&dyn ProgressSink>,
) -> Result<CampaignOptimizationResult, String> {
    match family {
        CampaignOptimizationFamily::DoubleLike => {
            let mut req: DoubleMaRequest =
                serde_json::from_value(req_value.clone()).map_err(|e| e.to_string())?;
            req.mode = normalize_product_double_ma_optimization_mode(&req);
            let res = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                req, candles, cancel, sink,
            )?;
            Ok(CampaignOptimizationResult::Backend(res))
        }
        CampaignOptimizationFamily::CompositeCpu => {
            let mut req: CompositeCpuRequest =
                serde_json::from_value(req_value.clone()).map_err(|e| e.to_string())?;
            req.mode = normalize_product_composite_cpu_optimization_mode(req.mode);
            let res = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                req, candles, cancel, sink,
            )?;
            Ok(CampaignOptimizationResult::Backend(res))
        }
        CampaignOptimizationFamily::CompositeMo => {
            let req: CompositeMoRequest =
                serde_json::from_value(req_value.clone()).map_err(|e| e.to_string())?;
            let res = ta_app_core::run_composite_mo_optimization_blocking_with_candles(
                req, candles, cancel, sink,
            )?;
            Ok(CampaignOptimizationResult::CompositeMo(res))
        }
    }
}

fn normalize_backend_value(raw: Option<&Value>, strategy_kind: &str) -> Value {
    let device_id = raw
        .and_then(Value::as_object)
        .and_then(|row| row.get("device_id"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let backend_name = raw
        .and_then(Value::as_object)
        .and_then(|row| row.get("backend"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    let preserve_vcet_gpu_kernel =
        normalize_strategy_kind(strategy_kind) == "Vcet" && backend_name == "GpuKernel";
    if !is_double_like_kind(strategy_kind) && backend_name != "CpuOnly" && !preserve_vcet_gpu_kernel
    {
        return json!({ "backend": "Auto", "device_id": device_id });
    }

    match backend_name {
        "Auto" => json!({ "backend": "Auto", "device_id": device_id }),
        "GpuOnly" => json!({ "backend": "GpuOnly", "device_id": device_id }),
        "GpuHybridKernel" => json!({ "backend": "GpuHybridKernel", "device_id": device_id }),
        "GpuKernel" => json!({ "backend": "GpuKernel", "device_id": device_id }),
        _ => json!({ "backend": "CpuOnly" }),
    }
}

#[cfg(test)]
fn candidate_values_from_optimization(
    strategy_kind: &str,
    family: CampaignOptimizationFamily,
    run_req: &Value,
    run_res: &CampaignOptimizationResult,
    max_candidates: usize,
) -> Result<Vec<Value>, String> {
    let validation_cfg = ResearchCampaignValidationConfig {
        max_candidates,
        ..ResearchCampaignValidationConfig::default()
    };
    let (candidates, _) = candidate_selection_from_optimization(
        strategy_kind,
        family,
        run_req,
        run_res,
        &validation_cfg,
    )?;
    Ok(candidates)
}

fn parse_validation_objective(value: Option<&str>) -> Option<CampaignValidationObjective> {
    let normalized = value?.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    match normalized.as_str() {
        "pnl" => Some(CampaignValidationObjective::Pnl),
        "sharpe" => Some(CampaignValidationObjective::Sharpe),
        "sortino" => Some(CampaignValidationObjective::Sortino),
        "calmar" => Some(CampaignValidationObjective::Calmar),
        "omega" => Some(CampaignValidationObjective::Omega),
        "volatility" | "vol" => Some(CampaignValidationObjective::Volatility),
        "maxdrawdown" | "max_drawdown" | "drawdown" | "dd" => {
            Some(CampaignValidationObjective::MaxDrawdown)
        }
        _ => None,
    }
}

fn objective_from_run_req(run_req: &Value) -> CampaignValidationObjective {
    parse_validation_objective(run_req.get("objective").and_then(Value::as_str))
        .or_else(|| {
            run_req
                .get("objectives")
                .and_then(Value::as_object)
                .and_then(|row| row.get("maximize"))
                .and_then(Value::as_str)
                .and_then(|value| parse_validation_objective(Some(value)))
        })
        .unwrap_or(CampaignValidationObjective::Sharpe)
}

fn collect_scalar_union_objectives(run_req: &Value) -> Vec<CampaignValidationObjective> {
    let primary = objective_from_run_req(run_req);
    let mut out = vec![primary];
    for candidate in [
        CampaignValidationObjective::MaxDrawdown,
        CampaignValidationObjective::Omega,
        CampaignValidationObjective::Calmar,
    ] {
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    }
    out
}

fn score_metrics_for_validation_objective(
    metrics: &ta_strategies::double_ma::Metrics,
    objective: CampaignValidationObjective,
) -> f64 {
    match objective {
        CampaignValidationObjective::Pnl => metrics.pnl,
        CampaignValidationObjective::Sharpe => metrics.sharpe,
        CampaignValidationObjective::Sortino => metrics.sortino,
        CampaignValidationObjective::Calmar => metrics.calmar,
        CampaignValidationObjective::Omega => metrics.omega,
        CampaignValidationObjective::Volatility => -metrics.volatility,
        CampaignValidationObjective::MaxDrawdown => -metrics.max_dd,
    }
}

fn build_validation_admission_rows(
    family: CampaignOptimizationFamily,
    run_res: &CampaignOptimizationResult,
) -> Result<Vec<CampaignValidationAdmissionRow>, String> {
    let mut rows = Vec::new();
    match (family, run_res) {
        (CampaignOptimizationFamily::DoubleLike, CampaignOptimizationResult::Backend(res))
        | (CampaignOptimizationFamily::CompositeCpu, CampaignOptimizationResult::Backend(res)) => {
            if !res.top.is_empty() {
                for (index, (params, metrics)) in res.top.iter().enumerate() {
                    rows.push(CampaignValidationAdmissionRow {
                        params_value: serde_json::to_value(params).map_err(|e| e.to_string())?,
                        metrics: Some(metrics.clone()),
                        maximize_value: None,
                        minimize_value: None,
                        violation: None,
                        input_index: index,
                        source_kind: "top_rows",
                    });
                }
            } else {
                rows.push(CampaignValidationAdmissionRow {
                    params_value: serde_json::to_value(&res.best_params)
                        .map_err(|e| e.to_string())?,
                    metrics: Some(res.best_metrics.clone()),
                    maximize_value: None,
                    minimize_value: None,
                    violation: None,
                    input_index: 0,
                    source_kind: "best_params_fallback",
                });
            }
        }
        (CampaignOptimizationFamily::CompositeMo, CampaignOptimizationResult::CompositeMo(res)) => {
            for (index, point) in res.pareto.iter().enumerate() {
                rows.push(CampaignValidationAdmissionRow {
                    params_value: serde_json::to_value(&point.params).map_err(|e| e.to_string())?,
                    metrics: Some(point.metrics.clone()),
                    maximize_value: Some(point.maximize_value),
                    minimize_value: Some(point.minimize_value),
                    violation: Some(point.violation),
                    input_index: index,
                    source_kind: "pareto_frontier",
                });
            }
        }
        (
            CampaignOptimizationFamily::CompositeMo,
            CampaignOptimizationResult::CompositeMoArtifact(run_res),
        ) => {
            let pareto = run_res
                .get("pareto")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    "campaign validation could not read pareto candidates from the saved CompositeMo optimization artifact"
                        .to_string()
                })?;
            for (index, point) in pareto.iter().enumerate() {
                let Some(params) = point.get("params").cloned() else {
                    continue;
                };
                rows.push(CampaignValidationAdmissionRow {
                    params_value: params,
                    metrics: point
                        .get("metrics")
                        .cloned()
                        .and_then(|value| serde_json::from_value(value).ok()),
                    maximize_value: point.get("maximize_value").and_then(Value::as_f64),
                    minimize_value: point.get("minimize_value").and_then(Value::as_f64),
                    violation: point.get("violation").and_then(Value::as_f64),
                    input_index: index,
                    source_kind: "pareto_frontier",
                });
            }
        }
        _ => return Err("campaign optimization result family mismatch".to_string()),
    }

    if family == CampaignOptimizationFamily::CompositeMo {
        rows.sort_by(|left, right| {
            let left_feasible = left.violation.unwrap_or(0.0) <= 0.0;
            let right_feasible = right.violation.unwrap_or(0.0) <= 0.0;
            if left_feasible != right_feasible {
                return if left_feasible {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }
            match (
                left.maximize_value.unwrap_or(f64::NEG_INFINITY),
                right.maximize_value.unwrap_or(f64::NEG_INFINITY),
            ) {
                (a, b) if a != b => b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal),
                _ => match (
                    left.minimize_value.unwrap_or(f64::INFINITY),
                    right.minimize_value.unwrap_or(f64::INFINITY),
                ) {
                    (a, b) if a != b => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
                    _ => left.input_index.cmp(&right.input_index),
                },
            }
        });
    }

    Ok(rows)
}

fn dedupe_validation_admission_rows(
    rows: &[CampaignValidationAdmissionRow],
) -> Result<Vec<CampaignValidationAdmissionRow>, String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for row in rows {
        let key = serde_json::to_string(&row.params_value).map_err(|e| e.to_string())?;
        if seen.insert(key) {
            out.push(row.clone());
        }
    }
    Ok(out)
}

fn admission_trim_telemetry(
    deduped_count: usize,
    post_cap_count: usize,
    candidate_cap_limit: usize,
) -> (usize, f64, bool) {
    let trimmed_count = deduped_count.saturating_sub(post_cap_count);
    let trim_ratio = if deduped_count == 0 {
        0.0
    } else {
        trimmed_count as f64 / deduped_count as f64
    };
    let cap_applied = deduped_count > post_cap_count && candidate_cap_limit > 0;
    (trimmed_count, trim_ratio, cap_applied)
}

fn build_legacy_validation_admission_selection(
    rows: &[CampaignValidationAdmissionRow],
    source_kind: &str,
    requested_policy: ValidationAdmissionPolicy,
    max_candidates: usize,
    note: Option<&str>,
) -> Result<CampaignValidationAdmissionSelection, String> {
    let deduped = dedupe_validation_admission_rows(rows)?;
    let post_cap_count = deduped.len().min(max_candidates);
    let (trimmed_count, trim_ratio, cap_applied) =
        admission_trim_telemetry(deduped.len(), post_cap_count, max_candidates);
    Ok(CampaignValidationAdmissionSelection {
        rows: deduped.iter().take(max_candidates).cloned().collect(),
        provenance: ValidationAdmissionProvenance {
            requested_policy,
            applied_policy: ValidationAdmissionPolicy::LegacyTopk,
            source_kind: source_kind.to_string(),
            source_candidate_count: rows.len(),
            pre_dedupe_count: rows.len(),
            post_dedupe_count: deduped.len(),
            post_cap_count,
            candidate_cap_limit: max_candidates,
            trimmed_count,
            trim_ratio,
            cap_applied,
            heads: vec![ValidationAdmissionHeadProvenance {
                head: "legacy_topk".to_string(),
                objective: None,
                candidate_count: rows.len(),
                note: note.map(str::to_string),
            }],
            selection_bias_note: cap_applied.then_some("candidate_cap_trim".to_string()),
            note: note.map(str::to_string),
        },
    })
}

fn build_scalar_union_validation_admission_selection(
    rows: &[CampaignValidationAdmissionRow],
    run_req: &Value,
    source_kind: &str,
    requested_policy: ValidationAdmissionPolicy,
    max_candidates: usize,
) -> Result<CampaignValidationAdmissionSelection, String> {
    fn ranked_finite_rows_for_objective(
        rows: &[CampaignValidationAdmissionRow],
        objective: CampaignValidationObjective,
    ) -> Vec<usize> {
        let mut scored: Vec<(usize, f64)> = Vec::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            let score = row
                .metrics
                .as_ref()
                .map(|metrics| score_metrics_for_validation_objective(metrics, objective))
                .unwrap_or(f64::NAN);
            if score.is_finite() {
                scored.push((index, score));
            }
        }
        scored.sort_by(|(left_index, left_score), (right_index, right_score)| {
            if left_score != right_score {
                return right_score
                    .partial_cmp(left_score)
                    .unwrap_or(std::cmp::Ordering::Equal);
            }
            rows[*left_index]
                .input_index
                .cmp(&rows[*right_index].input_index)
        });
        scored.into_iter().map(|(index, _)| index).collect()
    }

    let objectives = collect_scalar_union_objectives(run_req);
    let mut ordered_heads: Vec<Vec<usize>> = Vec::new();
    let mut heads = Vec::new();
    for (index, objective) in objectives.iter().copied().enumerate() {
        let ranked = ranked_finite_rows_for_objective(rows, objective);
        if ranked.is_empty() {
            heads.push(ValidationAdmissionHeadProvenance {
                head: if index == 0 { "primary" } else { "secondary" }.to_string(),
                objective: Some(objective.as_str().to_string()),
                candidate_count: 0,
                note: Some("no_finite_metrics".to_string()),
            });
            continue;
        }
        heads.push(ValidationAdmissionHeadProvenance {
            head: if index == 0 { "primary" } else { "secondary" }.to_string(),
            objective: Some(objective.as_str().to_string()),
            candidate_count: ranked.len(),
            note: None,
        });
        ordered_heads.push(ranked);
    }

    if ordered_heads.is_empty() {
        return build_legacy_validation_admission_selection(
            rows,
            source_kind,
            requested_policy,
            max_candidates,
            Some("scalar_union_fallback_missing_metrics"),
        );
    }

    let longest_head = ordered_heads.iter().map(Vec::len).max().unwrap_or(0);
    let pre_dedupe_count: usize = ordered_heads.iter().map(Vec::len).sum();
    let mut selected_rows = Vec::with_capacity(max_candidates.min(rows.len()));
    let mut seen = HashSet::new();
    let mut params_keys: Vec<Option<String>> = vec![None; rows.len()];
    let mut post_dedupe_count = 0usize;
    for rank in 0..longest_head {
        for head_rows in &ordered_heads {
            if let Some(row_index) = head_rows.get(rank).copied() {
                let key = match &params_keys[row_index] {
                    Some(key) => key.clone(),
                    None => {
                        let key = serde_json::to_string(&rows[row_index].params_value)
                            .map_err(|e| e.to_string())?;
                        params_keys[row_index] = Some(key.clone());
                        key
                    }
                };
                if seen.insert(key) {
                    post_dedupe_count += 1;
                    if selected_rows.len() < max_candidates {
                        selected_rows.push(rows[row_index].clone());
                    }
                }
            }
        }
    }
    let post_cap_count = post_dedupe_count.min(max_candidates);
    let (trimmed_count, trim_ratio, cap_applied) =
        admission_trim_telemetry(post_dedupe_count, post_cap_count, max_candidates);
    Ok(CampaignValidationAdmissionSelection {
        rows: selected_rows,
        provenance: ValidationAdmissionProvenance {
            requested_policy,
            applied_policy: ValidationAdmissionPolicy::ScalarUnion,
            source_kind: source_kind.to_string(),
            source_candidate_count: rows.len(),
            pre_dedupe_count,
            post_dedupe_count,
            post_cap_count,
            candidate_cap_limit: max_candidates,
            trimmed_count,
            trim_ratio,
            cap_applied,
            heads,
            selection_bias_note: cap_applied.then_some("candidate_cap_trim".to_string()),
            note: None,
        },
    })
}

fn build_pareto_frontier_validation_admission_selection(
    rows: &[CampaignValidationAdmissionRow],
    source_kind: &str,
    requested_policy: ValidationAdmissionPolicy,
    max_candidates: usize,
) -> Result<CampaignValidationAdmissionSelection, String> {
    if !rows.iter().any(|row| row.maximize_value.is_some()) {
        return build_legacy_validation_admission_selection(
            rows,
            source_kind,
            requested_policy,
            max_candidates,
            Some("pareto_frontier_fallback_missing_frontier"),
        );
    }
    let deduped = dedupe_validation_admission_rows(rows)?;
    let select_evenly_spaced_indices =
        |source_indices: &[usize], keep_count: usize| -> Vec<usize> {
            if source_indices.len() <= keep_count {
                return source_indices.to_vec();
            }
            if keep_count <= 1 {
                return source_indices.iter().take(1).copied().collect();
            }
            let mut indices = BTreeSet::new();
            for slot in 0..keep_count {
                let raw_index = (slot as f64 * (source_indices.len().saturating_sub(1) as f64))
                    / ((keep_count - 1) as f64);
                indices.insert(raw_index.round() as usize);
            }
            for index in 0..source_indices.len() {
                if indices.len() >= keep_count {
                    break;
                }
                indices.insert(index);
            }
            indices
                .into_iter()
                .take(keep_count)
                .filter_map(|index| source_indices.get(index).copied())
                .collect()
        };
    let feasible: Vec<usize> = deduped
        .iter()
        .enumerate()
        .filter(|(_, row)| row.violation.unwrap_or(0.0) <= 0.0)
        .map(|(index, _)| index)
        .collect();
    let infeasible: Vec<usize> = deduped
        .iter()
        .enumerate()
        .filter(|(_, row)| row.violation.unwrap_or(0.0) > 0.0)
        .map(|(index, _)| index)
        .collect();
    let trimmed_indices = if deduped.len() > max_candidates {
        if feasible.len() >= max_candidates {
            select_evenly_spaced_indices(&feasible, max_candidates)
        } else {
            let mut selected = feasible;
            selected.extend(select_evenly_spaced_indices(
                &infeasible,
                max_candidates.saturating_sub(selected.len()),
            ));
            selected
        }
    } else {
        (0..deduped.len().min(max_candidates)).collect()
    };
    let selected_rows: Vec<CampaignValidationAdmissionRow> = trimmed_indices
        .iter()
        .filter_map(|index| deduped.get(*index).cloned())
        .collect();
    let trim_note = if deduped.len() > max_candidates {
        Some("pareto_diversity_trim".to_string())
    } else {
        None
    };
    let post_cap_count = selected_rows.len();
    let (trimmed_count, trim_ratio, cap_applied) =
        admission_trim_telemetry(deduped.len(), post_cap_count, max_candidates);
    Ok(CampaignValidationAdmissionSelection {
        rows: selected_rows,
        provenance: ValidationAdmissionProvenance {
            requested_policy,
            applied_policy: ValidationAdmissionPolicy::ParetoFrontier,
            source_kind: source_kind.to_string(),
            source_candidate_count: rows.len(),
            pre_dedupe_count: rows.len(),
            post_dedupe_count: deduped.len(),
            post_cap_count,
            candidate_cap_limit: max_candidates,
            trimmed_count,
            trim_ratio,
            cap_applied,
            heads: vec![ValidationAdmissionHeadProvenance {
                head: "pareto_frontier".to_string(),
                objective: None,
                candidate_count: rows.len(),
                note: trim_note.clone(),
            }],
            selection_bias_note: cap_applied.then_some("heuristic_frontier_trim".to_string()),
            note: trim_note,
        },
    })
}

fn select_validation_admission_policy(
    requested_policy: ValidationAdmissionPolicy,
    strategy_kind: &str,
    rows: &[CampaignValidationAdmissionRow],
) -> ValidationAdmissionPolicy {
    if requested_policy == ValidationAdmissionPolicy::Auto {
        return if normalize_strategy_kind(strategy_kind) == "CompositeMo"
            && rows.iter().any(|row| row.maximize_value.is_some())
        {
            ValidationAdmissionPolicy::ParetoFrontier
        } else {
            ValidationAdmissionPolicy::ScalarUnion
        };
    }
    requested_policy
}

fn build_candidate_value_from_params(
    strategy_kind: &str,
    family: CampaignOptimizationFamily,
    run_req: &Value,
    params_value: &Value,
) -> Option<Value> {
    if family == CampaignOptimizationFamily::CompositeMo {
        return Some(params_value.clone());
    }
    let p = params_value.as_object()?;
    let fast_ma_type = p.get("fast_ma_type").and_then(Value::as_str)?.to_string();
    let slow_ma_type = p.get("slow_ma_type").and_then(Value::as_str)?.to_string();
    let fast_len = p.get("fast_len").and_then(Value::as_u64)?;
    let slow_len = p.get("slow_len").and_then(Value::as_u64)?;
    if fast_ma_type.is_empty() || slow_ma_type.is_empty() || fast_len == 0 || slow_len == 0 {
        return None;
    }
    Some(if is_double_like_kind(strategy_kind) {
        json!({
            "fast_ma_type": fast_ma_type,
            "slow_ma_type": slow_ma_type,
            "fast_len": fast_len,
            "slow_len": slow_len,
            "fast_ma_params": p.get("fast_ma_params").cloned().unwrap_or(Value::Null),
            "slow_ma_params": p.get("slow_ma_params").cloned().unwrap_or(Value::Null),
            "strategy": run_req.get("strategy").cloned().or_else(|| run_req.get("base_strategy").cloned()).unwrap_or_else(|| json!({})),
            "halftrend_filter": run_req.get("halftrend_filter").cloned().or_else(|| run_req.get("halftrendFilter").cloned()).unwrap_or(Value::Null),
        })
    } else {
        build_composite_candidate_value(
            strategy_kind,
            run_req,
            p,
            fast_ma_type,
            slow_ma_type,
            fast_len,
            slow_len,
        )
    })
}

fn candidate_selection_from_optimization(
    strategy_kind: &str,
    family: CampaignOptimizationFamily,
    run_req: &Value,
    run_res: &CampaignOptimizationResult,
    validation_cfg: &ResearchCampaignValidationConfig,
) -> Result<(Vec<Value>, ValidationAdmissionProvenance), String> {
    let rows = build_validation_admission_rows(family, run_res)?;
    if rows.is_empty() {
        return Err(
            "campaign validation could not derive any candidates from optimization output"
                .to_string(),
        );
    }
    let source_kind = rows
        .first()
        .map(|row| row.source_kind)
        .unwrap_or("top_rows");
    let requested_policy = validation_cfg.admission_policy;
    let applied_policy = select_validation_admission_policy(requested_policy, strategy_kind, &rows);
    let mut selection = match applied_policy {
        ValidationAdmissionPolicy::ScalarUnion => {
            build_scalar_union_validation_admission_selection(
                &rows,
                run_req,
                source_kind,
                requested_policy,
                validation_cfg.max_candidates.max(1),
            )?
        }
        ValidationAdmissionPolicy::ParetoFrontier => {
            build_pareto_frontier_validation_admission_selection(
                &rows,
                source_kind,
                requested_policy,
                validation_cfg.max_candidates.max(1),
            )?
        }
        ValidationAdmissionPolicy::Auto => unreachable!("auto should resolve before selection"),
        ValidationAdmissionPolicy::LegacyTopk => build_legacy_validation_admission_selection(
            &rows,
            source_kind,
            requested_policy,
            validation_cfg.max_candidates.max(1),
            None,
        )?,
    };

    let candidate_cap = validation_cfg.max_candidates.max(1);
    if family == CampaignOptimizationFamily::CompositeMo {
        let candidates: Vec<Value> = selection
            .rows
            .iter()
            .take(candidate_cap)
            .map(|row| row.params_value.clone())
            .collect();
        if candidates.is_empty() {
            return Err(
                "campaign validation could not derive any candidates from optimization output"
                    .to_string(),
            );
        }
        selection.provenance.post_cap_count = candidates.len();
        return Ok((candidates, selection.provenance));
    }

    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for row in &selection.rows {
        let Some(candidate) =
            build_candidate_value_from_params(strategy_kind, family, run_req, &row.params_value)
        else {
            continue;
        };
        let key = serde_json::to_string(&candidate).map_err(|e| e.to_string())?;
        if seen.insert(key) {
            candidates.push(candidate);
            if candidates.len() >= candidate_cap {
                break;
            }
        }
    }
    if candidates.is_empty() {
        return Err(
            "campaign validation could not derive any candidates from optimization output"
                .to_string(),
        );
    }
    selection.provenance.post_cap_count = candidates.len();
    Ok((candidates, selection.provenance))
}

fn build_composite_candidate_value(
    strategy_kind: &str,
    run_req: &Value,
    params: &serde_json::Map<String, Value>,
    fast_ma_type: String,
    slow_ma_type: String,
    fast_len: u64,
    slow_len: u64,
) -> Value {
    let base = json!({
        "fast_ma_type": fast_ma_type,
        "slow_ma_type": slow_ma_type,
        "fast_len": fast_len,
        "slow_len": slow_len,
        "fast_ma_params": params.get("fast_ma_params").cloned().unwrap_or(Value::Null),
        "slow_ma_params": params.get("slow_ma_params").cloned().unwrap_or(Value::Null),
        "strategy": run_req
            .get("strategy")
            .cloned()
            .or_else(|| run_req.get("base_strategy").cloned())
            .unwrap_or_else(|| {
                serde_json::to_value(ta_strategies::double_ma::StrategyConfig::default())
                    .unwrap_or_else(|_| json!({}))
            }),
        "halftrend": run_req.get("halftrend").cloned().unwrap_or(Value::Null),
    });
    match normalize_strategy_kind(strategy_kind).as_str() {
        "Asr" => merge_json(
            base,
            json!({ "asr": run_req.get("asr").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Vcc" => merge_json(
            base,
            json!({ "vcc": run_req.get("vcc").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Vemr" => merge_json(
            base,
            json!({ "vemr": run_req.get("vemr").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Rocb" => merge_json(
            base,
            json!({ "rocb": run_req.get("rocb").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Vcet" => merge_json(
            base,
            json!({ "vcet": run_req.get("vcet").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Mkcs" => merge_json(
            base,
            json!({ "mkcs": run_req.get("mkcs").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Kqsr" => merge_json(
            base,
            json!({ "kqsr": run_req.get("kqsr").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Carm" => merge_json(
            base,
            json!({ "carm": run_req.get("carm").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Kvdf" => merge_json(
            base,
            json!({ "kvdf": run_req.get("kvdf").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Cdmc" => merge_json(
            base,
            json!({ "cdmc": run_req.get("cdmc").cloned().unwrap_or_else(|| json!({})) }),
        ),
        _ => merge_json(
            base,
            json!({
                "halftrend": run_req.get("halftrend").cloned().unwrap_or(Value::Null),
                "macz": run_req.get("macz").cloned().unwrap_or_else(|| json!({})),
                "ott": run_req.get("ott").cloned().unwrap_or_else(|| json!({})),
                "signal_mode": run_req.get("signal_mode").cloned().or_else(|| run_req.get("signalMode").cloned()).unwrap_or_else(|| Value::String("hard_gate".to_string())),
            }),
        ),
    }
}

fn merge_json(mut left: Value, right: Value) -> Value {
    if let (Some(left_map), Some(right_map)) = (left.as_object_mut(), right.as_object()) {
        for (key, value) in right_map {
            left_map.insert(key.clone(), value.clone());
        }
    }
    left
}

fn optimization_trials_from_result(run_res: &CampaignOptimizationResult) -> Option<usize> {
    let trials = match run_res {
        CampaignOptimizationResult::Backend(result) => result.num_combos,
        CampaignOptimizationResult::CompositeMo(result) => result.total_evaluations,
        CampaignOptimizationResult::CompositeMoArtifact(result) => result
            .get("total_evaluations")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .or_else(|| {
                result
                    .pointer("/telemetry/search/actual_evaluations")
                    .and_then(Value::as_u64)
                    .filter(|value| *value > 0)
            })
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0),
    };
    (trials > 0).then_some(trials)
}

fn search_distribution_from_result(
    run_res: &CampaignOptimizationResult,
) -> Option<ta_optimizer::SearchDistributionSummary> {
    match run_res {
        CampaignOptimizationResult::Backend(result) => result
            .search_semantics
            .as_ref()
            .and_then(|semantics| semantics.search_distribution.clone()),
        CampaignOptimizationResult::CompositeMo(result) => result
            .search_semantics
            .as_ref()
            .and_then(|semantics| semantics.search_distribution.clone()),
        CampaignOptimizationResult::CompositeMoArtifact(result) => result
            .pointer("/search_semantics/search_distribution")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
    }
}

fn build_validation_request_value(
    strategy_kind: &str,
    data_id: &str,
    run_req: &Value,
    run_res: &CampaignOptimizationResult,
    dataset_meta: &Value,
    validation_cfg: &ResearchCampaignValidationConfig,
    family: CampaignOptimizationFamily,
) -> Result<Value, String> {
    let optimization_trials = optimization_trials_from_result(run_res);
    let search_distribution = search_distribution_from_result(run_res);
    let (candidates, admission_provenance) = candidate_selection_from_optimization(
        strategy_kind,
        family,
        run_req,
        run_res,
        validation_cfg,
    )?;
    let dataset = dataset_meta.as_object();
    let csv_path = dataset
        .and_then(|row| row.get("csv_path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let timeframe = dataset
        .and_then(|row| row.get("timeframe"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let label = dataset
        .and_then(|row| row.get("symbol").or_else(|| row.get("label")))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    Ok(json!({
        "data_id": data_id,
        "backend": normalize_backend_value(run_req.get("backend"), strategy_kind),
        "ma_source": run_req.get("ma_source").cloned().unwrap_or_else(|| Value::String("close".to_string())),
        "fast_ma_params": run_req.get("fast_ma_params").cloned().unwrap_or(Value::Null),
        "slow_ma_params": run_req.get("slow_ma_params").cloned().unwrap_or(Value::Null),
        "candidates": candidates,
        "walk_forward": validation_cfg.walk_forward,
        "reopt_walk_forward": validation_cfg.reopt_walk_forward,
        "spa": validation_cfg.spa,
        "perturb": validation_cfg.perturb,
        "trade_bootstrap": validation_cfg.trade_bootstrap,
        "monte_carlo": validation_cfg.monte_carlo,
        "cross_ticker": validation_cfg.cross_ticker,
        "cscv_pbo": cscv_pbo_to_value(validation_cfg.cscv_pbo.as_ref()),
        "seed": validation_cfg.seed,
        "max_candidates": validation_cfg.max_candidates.max(1),
        "admission_policy": admission_provenance.applied_policy,
        "admission_provenance": admission_provenance,
        "min_psr": validation_cfg.min_psr,
        "min_dsr": validation_cfg.min_dsr,
        "min_cscv_oos_rank_frac": validation_cfg.min_cscv_oos_rank_frac,
        "min_cscv_oos_metric": validation_cfg.min_cscv_oos_metric,
        "max_pbo": validation_cfg.max_pbo,
        "validation_context": ValidationArtifactContext {
            strategy_kind: normalize_strategy_kind(strategy_kind),
            csv_path,
            timeframe,
            label,
            optimization_trials,
            search_distribution,
            data_realism: None,
        }
    }))
}

fn build_vcet_fixed_candidate_validation_request_value(
    data_id: &str,
    run_req: &Value,
    dataset_meta: &Value,
    validation_cfg: &ResearchCampaignValidationConfig,
    plan: &CampaignVcetFixedCandidatePlan,
) -> Value {
    let dataset = dataset_meta.as_object();
    let csv_path = dataset
        .and_then(|row| row.get("csv_path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let timeframe = dataset
        .and_then(|row| row.get("timeframe"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let label = dataset
        .and_then(|row| row.get("symbol").or_else(|| row.get("label")))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    json!({
        "data_id": data_id,
        "backend": normalize_backend_value(run_req.get("backend"), "Vcet"),
        "ma_source": run_req.get("ma_source").cloned().unwrap_or_else(|| Value::String("close".to_string())),
        "fast_ma_params": run_req.get("fast_ma_params").cloned().unwrap_or(Value::Null),
        "slow_ma_params": run_req.get("slow_ma_params").cloned().unwrap_or(Value::Null),
        "candidates": plan.candidates,
        "walk_forward": validation_cfg.walk_forward,
        "reopt_walk_forward": validation_cfg.reopt_walk_forward,
        "spa": validation_cfg.spa,
        "perturb": validation_cfg.perturb,
        "trade_bootstrap": validation_cfg.trade_bootstrap,
        "monte_carlo": validation_cfg.monte_carlo,
        "cross_ticker": validation_cfg.cross_ticker,
        "cscv_pbo": cscv_pbo_to_value(validation_cfg.cscv_pbo.as_ref()),
        "seed": validation_cfg.seed,
        "max_candidates": plan.candidates.len(),
        "min_psr": validation_cfg.min_psr,
        "min_dsr": validation_cfg.min_dsr,
        "min_cscv_oos_rank_frac": validation_cfg.min_cscv_oos_rank_frac,
        "min_cscv_oos_metric": validation_cfg.min_cscv_oos_metric,
        "max_pbo": validation_cfg.max_pbo,
        "evaluation_mode": "evaluate_candidates",
        "candidate_manifest_hash": plan.candidate_manifest_hash,
        "candidate_search_id": plan.search_manifest.as_ref().map(|manifest| manifest.search_id.as_str()),
        "candidate_search_schema_version": plan.search_manifest.as_ref().map(|manifest| manifest.schema_version),
        "require_gpu_kernel": validation_cfg.require_gpu_kernel,
        "validation_context": ValidationArtifactContext {
            strategy_kind: "Vcet".to_string(),
            csv_path,
            timeframe,
            label,
            optimization_trials: None,
            search_distribution: None,
            data_realism: None,
        }
    })
}

fn vcet_fixed_candidate_run_result_value(plan: &CampaignVcetFixedCandidatePlan) -> Value {
    json!({
        "evaluation_mode": "evaluate_candidates",
        "candidate_manifest_hash": plan.candidate_manifest_hash,
        "candidate_search_id": plan.search_manifest.as_ref().map(|manifest| manifest.search_id.as_str()),
        "candidate_search_schema_version": plan.search_manifest.as_ref().map(|manifest| manifest.schema_version),
        "num_combos": plan.candidates.len(),
    })
}

fn parse_vcet_fixed_candidate_dataset_result(
    validation_res: &Value,
    plan: &CampaignVcetFixedCandidatePlan,
    require_gpu_kernel: bool,
) -> Result<CampaignFixedCandidateDatasetResult, String> {
    let backend_used = validation_res
        .get("backend_used")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let backend_route = validation_res
        .get("backend_route")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let requested_backend = validation_res
        .get("requested_backend")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let fallback_reason = validation_res
        .get("backend_fallback_reason")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let validation_residency_tier = validation_res
        .get("validation_residency_tier")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let gpu_kernel_used = backend_used == "Gpu"
        && backend_route.contains("GpuKernel")
        && fallback_reason.is_none()
        && validation_residency_tier == "gpu_kernel";
    let exact_gpu_kernel_route =
        requested_backend == "GpuKernel" && backend_route == "GpuKernel" && gpu_kernel_used;
    if require_gpu_kernel && !exact_gpu_kernel_route {
        return Err(format!(
            "VCET fixed-candidate validation required the exact GpuKernel route but used requested_backend='{requested_backend}', backend_used='{backend_used}', backend_route='{backend_route}', validation_residency_tier='{validation_residency_tier}', fallback_reason='{}'",
            fallback_reason.unwrap_or("none")
        ));
    }

    if plan.candidate_identities.len() != plan.candidates.len() {
        return Err("VCET fixed-candidate plan contained an invalid identity table".to_string());
    }
    let expected_by_identity: std::collections::BTreeMap<
        CampaignVcetFixedCandidateIdentity,
        &Value,
    > = plan
        .candidate_identities
        .iter()
        .copied()
        .zip(plan.candidates.iter())
        .collect();
    if expected_by_identity.len() != plan.candidates.len() {
        return Err(
            "VCET fixed-candidate plan contained a duplicate effective strategy identity"
                .to_string(),
        );
    }
    let rows = validation_res
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "VCET fixed-candidate validation result is missing candidates".to_string()
        })?;
    if rows.len() != plan.candidates.len() {
        return Err(format!(
            "VCET fixed-candidate validation returned {} candidates for a {}-candidate manifest",
            rows.len(),
            plan.candidates.len()
        ));
    }

    let mut seen = BTreeSet::new();
    let mut candidates = Vec::with_capacity(rows.len());
    let mut strict_pareto_points: Vec<(f64, f64, f64, bool)> = Vec::with_capacity(rows.len());
    for row in rows {
        let params = row.get("params").ok_or_else(|| {
            "VCET fixed-candidate validation result contains a candidate without params".to_string()
        })?;
        let fast_len = params
            .get("fast_len")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| "VCET fixed-candidate result has an invalid fast_len".to_string())?;
        let slow_len = params
            .get("slow_len")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| "VCET fixed-candidate result has an invalid slow_len".to_string())?;
        let identity = vcet_fixed_candidate_identity(params).map_err(|error| {
            format!(
                "VCET fixed-candidate result has an invalid effective identity for ({fast_len}, {slow_len}): {error}"
            )
        })?;
        let expected = expected_by_identity.get(&identity).ok_or_else(|| {
            format!(
                "VCET fixed-candidate validation returned unexpected effective strategy identity for ({fast_len}, {slow_len})"
            )
        })?;
        if canonical_json(params) != canonical_json(expected) {
            return Err(format!(
                "VCET fixed-candidate validation returned parameters that do not match the manifest for ({fast_len}, {slow_len})"
            ));
        }
        if !seen.insert(identity) {
            return Err(format!(
                "VCET fixed-candidate validation returned duplicate effective strategy identity for ({fast_len}, {slow_len})"
            ));
        }
        let metrics = row.get("in_sample").ok_or_else(|| {
            "VCET fixed-candidate validation result is missing in_sample metrics".to_string()
        })?;
        let typed_metrics: ta_strategies::double_ma::Metrics =
            serde_json::from_value(metrics.clone()).map_err(|error| {
                format!(
                    "VCET fixed-candidate validation contains an invalid typed in_sample metric surface: {error}"
                )
            })?;
        let metric = |name: &str| -> Result<f64, String> {
            metrics.get(name).and_then(Value::as_f64).ok_or_else(|| {
                format!("VCET fixed-candidate validation metric '{name}' is missing or invalid")
            })
        };
        let trades = metrics
            .get("trades")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| {
                "VCET fixed-candidate validation metric 'trades' is missing or invalid".to_string()
            })?;
        let hard_gate_passed = row
            .pointer("/validation_verdict/passed")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                "VCET fixed-candidate validation result is missing its hard-gate verdict"
                    .to_string()
            })?;
        let robust_score = row
            .get("robust_score")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
            .ok_or_else(|| {
                "VCET fixed-candidate validation result is missing a finite robust_score"
                    .to_string()
            })?;
        let robust_pareto = row
            .get("robust_pareto")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                "VCET fixed-candidate validation result is missing robust_pareto".to_string()
            })?;
        for field in ["psr", "dsr"] {
            if row.get(field).is_some_and(|value| {
                !value.is_null()
                    && value
                        .as_f64()
                        .is_none_or(|number| !number.is_finite() || !(0.0..=1.0).contains(&number))
            }) {
                return Err(format!(
                    "VCET fixed-candidate validation result contains an invalid {field} probability"
                ));
            }
        }
        if require_gpu_kernel {
            for field in [
                "iterative_in_sample",
                "iterative_delta",
                "walk_forward",
                "perturb",
                "trade_bootstrap",
                "monte_carlo",
                "cross_ticker",
                "cross_ticker_pareto",
                "cscv_pbo",
            ] {
                if row.get(field).is_some_and(|value| !value.is_null()) {
                    return Err(format!(
                        "strict GPU VCET fixed-candidate validation unexpectedly contains auxiliary field '{field}'"
                    ));
                }
            }
            let expected_robust_score =
                ta_app_core::validation_pipeline::robust_score_from_metrics(&typed_metrics, None);
            if (robust_score - expected_robust_score).abs() > 1.0e-12 {
                return Err(
                    "VCET fixed-candidate validation robust_score does not match its in_sample metrics"
                        .to_string(),
                );
            }
            strict_pareto_points.push((
                expected_robust_score,
                typed_metrics.ulcer_index,
                typed_metrics.max_dd,
                robust_pareto,
            ));
        }
        candidates.push(CampaignFixedCandidateObservation {
            identity,
            pnl: metric("pnl")?,
            sharpe: metric("sharpe")?,
            max_drawdown: metric("max_dd")?,
            trades,
            hard_gate_passed,
        });
    }
    if require_gpu_kernel {
        for (index, (score, ulcer, dd, actual_pareto)) in
            strict_pareto_points.iter().copied().enumerate()
        {
            let dominated = strict_pareto_points.iter().enumerate().any(
                |(other_index, (other_score, other_ulcer, other_dd, _))| {
                    other_index != index
                        && *other_score >= score
                        && *other_ulcer <= ulcer
                        && *other_dd <= dd
                        && (*other_score > score || *other_ulcer < ulcer || *other_dd < dd)
                },
            );
            if actual_pareto == dominated {
                return Err(
                    "VCET fixed-candidate validation robust_pareto does not match its candidate metric frontier"
                        .to_string(),
                );
            }
        }
    }
    candidates.sort_by_key(|candidate| candidate.identity);
    Ok(CampaignFixedCandidateDatasetResult {
        gpu_kernel_used,
        candidates,
    })
}

fn parse_vcet_fixed_candidate_artifact_result(
    artifact: &Value,
    plan: &CampaignVcetFixedCandidatePlan,
    require_gpu_kernel: bool,
) -> Result<CampaignFixedCandidateDatasetResult, String> {
    let manifest_hash = artifact
        .pointer("/validation/req/candidate_manifest_hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if manifest_hash != plan.candidate_manifest_hash {
        return Err(
            "existing VCET fixed-candidate artifact does not match the candidate manifest hash"
                .to_string(),
        );
    }
    let expected_search_id = plan
        .search_manifest
        .as_ref()
        .map(|manifest| manifest.search_id.as_str());
    let expected_search_schema = plan
        .search_manifest
        .as_ref()
        .map(|manifest| u64::from(manifest.schema_version));
    for (surface, id_pointer, schema_pointer) in [
        (
            "validation request",
            "/validation/req/candidate_search_id",
            "/validation/req/candidate_search_schema_version",
        ),
        (
            "run result",
            "/run/res/candidate_search_id",
            "/run/res/candidate_search_schema_version",
        ),
    ] {
        let actual_search_id = match artifact.pointer(id_pointer) {
            None | Some(Value::Null) => None,
            Some(Value::String(value)) if !value.trim().is_empty() => Some(value.as_str()),
            Some(_) => {
                return Err(format!(
                    "existing VCET fixed-candidate artifact {surface} has an invalid candidate_search_id type or value"
                ))
            }
        };
        let actual_search_schema = match artifact.pointer(schema_pointer) {
            None | Some(Value::Null) => None,
            Some(value) => Some(value.as_u64().ok_or_else(|| {
                format!(
                    "existing VCET fixed-candidate artifact {surface} has an invalid candidate_search_schema_version type or value"
                )
            })?),
        };
        if actual_search_id != expected_search_id || actual_search_schema != expected_search_schema
        {
            return Err(format!(
                "existing VCET fixed-candidate artifact {surface} does not match the sealed candidate search lineage"
            ));
        }
    }
    let validation_res = artifact.pointer("/validation/res").ok_or_else(|| {
        "existing VCET fixed-candidate artifact is missing validation.res".to_string()
    })?;
    parse_vcet_fixed_candidate_dataset_result(validation_res, plan, require_gpu_kernel)
}

pub(super) fn build_vcet_fixed_candidate_artifact_contract(
    summary: &ResearchCampaignSummary,
) -> Result<CampaignVcetFixedCandidateArtifactContract, String> {
    let req: ResearchCampaignRequest = serde_json::from_value(summary.request.clone())
        .map_err(|error| format!("campaign summary request is not replayable: {error}"))?;
    let strategy_kind = resolve_campaign_strategy_kind(&req.strategy_kind)?;
    let plan = build_vcet_fixed_candidate_plan(&req, &strategy_kind, req.execution_mode)?
        .ok_or_else(|| {
            "campaign summary does not define a VCET evaluate_candidates manifest".to_string()
        })?;
    Ok(CampaignVcetFixedCandidateArtifactContract {
        plan,
        require_gpu_kernel: req.validation_config.require_gpu_kernel,
    })
}

fn vcet_temporal_identity_micros(label: &str, value: f64) -> Result<i64, String> {
    if !value.is_finite() {
        return Err(format!(
            "campaign VCET {label} is non-finite and cannot be promoted into an exact temporal identity"
        ));
    }
    let scaled = value * 1_000_000.0;
    if scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
        return Err(format!(
            "campaign VCET {label} is outside the temporal identity range"
        ));
    }
    let rounded = scaled.round();
    let tolerance = 1.0e-7_f64.max(scaled.abs() * f64::EPSILON * 8.0);
    if (scaled - rounded).abs() > tolerance {
        return Err(format!(
            "campaign VCET {label} must align to 0.000001 precision before temporal promotion"
        ));
    }
    Ok(if rounded == 0.0 { 0 } else { rounded as i64 })
}

fn vcet_temporal_identity_from_campaign_identity(
    identity: CampaignVcetFixedCandidateIdentity,
) -> Result<ta_app_core::vcet_search::VcetCandidateIdentityInputs, String> {
    Ok(ta_app_core::vcet_search::VcetCandidateIdentityInputs {
        vidya_short_period: identity.vidya_short_period,
        vidya_long_period: identity.vidya_long_period,
        vidya_alpha_micros: vcet_temporal_identity_micros(
            "vidya_alpha",
            f64::from_bits(identity.vidya_alpha_bits),
        )?,
        ce_period: identity.ce_period,
        ce_mult_micros: vcet_temporal_identity_micros(
            "ce_mult",
            f64::from_bits(identity.ce_mult_bits),
        )?,
        ce_use_close: identity.ce_use_close,
        require_vidya_slope: identity.require_vidya_slope,
        max_trade_bars: identity.max_trade_bars,
    })
}

/// Replays the sealed terminal campaign request and promotes only its admitted
/// VCET candidates into the exact integer identity contract used by temporal
/// validation. No candidate is re-sampled or reconstructed from display text.
pub(super) fn build_vcet_temporal_campaign_candidate_source(
    summary: &ResearchCampaignSummary,
) -> Result<VcetTemporalCampaignCandidateSourceV1, String> {
    if !matches!(
        summary.state.trim(),
        "completed" | "completed_with_failures" | "completed_with_warnings"
    ) || summary.counts.cancelled
    {
        return Err(
            "VCET temporal promotion requires a terminal, non-cancelled campaign summary"
                .to_string(),
        );
    }
    let request: ResearchCampaignRequest = serde_json::from_value(summary.request.clone())
        .map_err(|error| format!("campaign summary request is not replayable: {error}"))?;
    if request.execution_mode != ResearchCampaignExecutionMode::EvaluateCandidates
        || normalize_strategy_kind(&request.strategy_kind) != "Vcet"
    {
        return Err(
            "VCET temporal promotion requires a VCET evaluate_candidates campaign".to_string(),
        );
    }
    let manifest = summary
        .manifest
        .as_ref()
        .ok_or_else(|| "VCET temporal promotion requires a v2 campaign manifest".to_string())?;
    let aggregate = summary.fixed_candidate_evaluation.as_ref().ok_or_else(|| {
        "VCET temporal promotion requires fixed-candidate admission evidence".to_string()
    })?;
    if aggregate.schema_version != 4 {
        return Err(
            "VCET temporal promotion requires sealed cross-universe admission schema v4"
                .to_string(),
        );
    }
    if aggregate.admitted_candidate_ids.is_empty()
        || aggregate.admitted_candidate_count != aggregate.admitted_candidate_ids.len()
    {
        return Err(
            "VCET temporal promotion requires at least one consistently admitted candidate"
                .to_string(),
        );
    }
    let admitted = aggregate
        .admitted_candidate_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if admitted.len() != aggregate.admitted_candidate_ids.len() {
        return Err(
            "VCET temporal promotion rejected duplicate admitted candidate IDs".to_string(),
        );
    }

    let strategy_kind = resolve_campaign_strategy_kind(&request.strategy_kind)?;
    let plan = build_vcet_fixed_candidate_plan(
        &request,
        &strategy_kind,
        ResearchCampaignExecutionMode::EvaluateCandidates,
    )?
    .ok_or_else(|| "campaign did not replay an evaluate_candidates plan".to_string())?;
    if aggregate.candidate_manifest_hash != plan.candidate_manifest_hash
        || manifest.candidate_manifest_hash.as_deref()
            != Some(plan.candidate_manifest_hash.as_str())
    {
        return Err(
            "VCET temporal promotion rejected mismatched campaign candidate lineage".to_string(),
        );
    }
    let mut optimization_template = request.optimization_template.clone();
    let optimization_object = optimization_template.as_object_mut().ok_or_else(|| {
        "VCET temporal promotion requires an object optimization_template".to_string()
    })?;
    // Campaign evaluate-candidates templates historically use the UI-only
    // `Exact` mode. Temporal promotion needs only the backend/source/strategy
    // contract, so normalize the otherwise-unused optimizer mode before
    // decoding the shared request shape.
    optimization_object.insert("mode".to_string(), Value::String("Grid".to_string()));
    let optimization_request: DoubleMaRequest =
        serde_json::from_value(optimization_template).map_err(|error| {
            format!(
                "VCET temporal promotion could not replay optimization_template as DoubleMaRequest: {error}"
            )
        })?;

    let mut admitted_identities = Vec::with_capacity(admitted.len());
    let mut observed_admitted_ids = BTreeSet::new();
    for (ordinal, campaign_identity) in plan.candidate_identities.iter().copied().enumerate() {
        let evaluation_candidate_id = vcet_fixed_candidate_id(campaign_identity);
        if !admitted.contains(&evaluation_candidate_id) {
            continue;
        }
        let identity = if let Some(search_manifest) = plan.search_manifest.as_ref() {
            let sampled = search_manifest.candidates.get(ordinal).ok_or_else(|| {
                "VCET search manifest is shorter than its expanded campaign plan".to_string()
            })?;
            if sampled.evaluation_candidate_id.as_deref() != Some(evaluation_candidate_id.as_str())
            {
                return Err(
                    "VCET temporal promotion rejected a search/evaluation candidate identity mismatch"
                        .to_string(),
                );
            }
            let reconstructed = vcet_temporal_identity_from_campaign_identity(campaign_identity)?;
            if sampled.identity_inputs != reconstructed {
                return Err(
                    "VCET temporal promotion rejected drift between exact search identity and expanded campaign identity"
                        .to_string(),
                );
            }
            sampled.identity_inputs.clone()
        } else {
            vcet_temporal_identity_from_campaign_identity(campaign_identity)?
        };
        let campaign_candidate: ta_app_core::CompositeParams =
            serde_json::from_value(plan.candidates.get(ordinal).cloned().ok_or_else(|| {
                "VCET expanded candidate table is shorter than its identity table".to_string()
            })?)
            .map_err(|error| format!("decode admitted VCET campaign candidate: {error}"))?;
        let temporal_candidate =
            super::commands_vcet_temporal_execute::candidate_params_from_manifest(
                &identity,
                &optimization_request.strategy,
            );
        let campaign_strategy_bytes = serde_json::to_vec(&campaign_candidate.strategy)
            .map_err(|error| format!("encode admitted VCET execution strategy: {error}"))?;
        let temporal_strategy_bytes = serde_json::to_vec(&temporal_candidate.strategy)
            .map_err(|error| format!("encode promoted VCET execution strategy: {error}"))?;
        // campaign_identity above is computed from the fully resolved VCET
        // periods/controls (including any legacy fast/slow mapping flags) and
        // is the exact input sealed into the temporal manifest. The strict
        // window executor additionally consumes StrategyConfig. MA types,
        // HalfTrend, MACZ/OTT, signal mode, and raw mapping placeholders are
        // intentionally canonicalized and are not VCET execution inputs.
        if campaign_strategy_bytes != temporal_strategy_bytes
            || campaign_candidate.asr.is_some()
            || campaign_candidate.vcc.is_some()
            || campaign_candidate.vemr.is_some()
            || campaign_candidate.rocb.is_some()
            || campaign_candidate.mkcs.is_some()
            || campaign_candidate.kqsr.is_some()
            || campaign_candidate.carm.is_some()
            || campaign_candidate.kvdf.is_some()
            || campaign_candidate.cdmc.is_some()
        {
            return Err(format!(
                "VCET temporal promotion rejected admitted candidate '{evaluation_candidate_id}' because its standalone family or execution strategy differs from the temporal executor contract"
            ));
        }
        observed_admitted_ids.insert(evaluation_candidate_id);
        admitted_identities.push(identity);
    }
    if observed_admitted_ids != admitted {
        return Err(
            "VCET temporal promotion could not resolve every admitted campaign candidate"
                .to_string(),
        );
    }

    let source_campaign_manifest_hash = manifest.manifest_hash.clone();
    let source_candidate_manifest_hash = plan.candidate_manifest_hash.clone();
    let source_search_id = plan
        .search_manifest
        .as_ref()
        .map(|search| search.search_id.clone());
    let sealed_candidate_manifest =
        super::commands_vcet_temporal::seal_vcet_temporal_candidate_manifest(
            source_campaign_manifest_hash,
            source_candidate_manifest_hash,
            source_search_id,
            admitted_identities,
        )?;
    Ok(VcetTemporalCampaignCandidateSourceV1 {
        cross_universe_admission: request.validation_config.cross_universe_admission,
        request,
        optimization_request,
        sealed_candidate_manifest,
    })
}

pub(super) fn validate_vcet_fixed_candidate_artifact_contract(
    artifact: &Value,
    contract: &CampaignVcetFixedCandidateArtifactContract,
) -> Result<(), String> {
    parse_vcet_fixed_candidate_artifact_result(
        artifact,
        &contract.plan,
        contract.require_gpu_kernel,
    )
    .map(|_| ())
}

pub(super) fn validate_vcet_fixed_candidate_artifact_request_contract(
    artifact: &Value,
    contract: &CampaignVcetFixedCandidateArtifactContract,
) -> Result<(), String> {
    let validation_req = artifact.pointer("/validation/req").ok_or_else(|| {
        "existing VCET fixed-candidate artifact is missing validation.req".to_string()
    })?;
    let run_res = artifact
        .pointer("/run/res")
        .ok_or_else(|| "existing VCET fixed-candidate artifact is missing run.res".to_string())?;
    let expected_candidates = Value::Array(contract.plan.candidates.clone());
    if validation_req
        .get("evaluation_mode")
        .and_then(Value::as_str)
        != Some("evaluate_candidates")
        || run_res.get("evaluation_mode").and_then(Value::as_str) != Some("evaluate_candidates")
        || validation_req
            .get("require_gpu_kernel")
            .and_then(Value::as_bool)
            != Some(contract.require_gpu_kernel)
        || validation_req.get("candidates") != Some(&expected_candidates)
        || validation_req.get("max_candidates").and_then(Value::as_u64)
            != u64::try_from(contract.plan.candidates.len()).ok()
        || run_res
            .get("candidate_manifest_hash")
            .and_then(Value::as_str)
            != Some(contract.plan.candidate_manifest_hash.as_str())
        || run_res.get("num_combos").and_then(Value::as_u64)
            != u64::try_from(contract.plan.candidates.len()).ok()
    {
        return Err(
            "existing VCET fixed-candidate artifact request/result envelope differs from the sealed evaluate-candidates contract"
                .to_string(),
        );
    }
    Ok(())
}

pub(super) fn validate_vcet_fixed_candidate_training_evidence<'a>(
    summary: &ResearchCampaignSummary,
    training_artifacts: impl IntoIterator<Item = &'a Value>,
) -> Result<(), String> {
    let req: ResearchCampaignRequest = serde_json::from_value(summary.request.clone())
        .map_err(|error| format!("campaign summary request is not replayable: {error}"))?;
    let strategy_kind = resolve_campaign_strategy_kind(&req.strategy_kind)?;
    let plan = build_vcet_fixed_candidate_plan(&req, &strategy_kind, req.execution_mode)?
        .ok_or_else(|| {
            "campaign summary does not define a VCET evaluate_candidates manifest".to_string()
        })?;
    let aggregate = summary.fixed_candidate_evaluation.as_ref().ok_or_else(|| {
        "campaign summary is missing its VCET fixed-candidate aggregate".to_string()
    })?;
    if aggregate.schema_version != 4 {
        return Err(
            "campaign-bound Meta requires sealed VCET fixed-candidate admission schema v4"
                .to_string(),
        );
    }

    let parsed_results: Vec<CampaignFixedCandidateDatasetResult> = training_artifacts
        .into_iter()
        .map(|artifact| {
            parse_vcet_fixed_candidate_artifact_result(
                artifact,
                &plan,
                req.validation_config.require_gpu_kernel,
            )
        })
        .collect::<Result<_, _>>()?;
    let parsed_refs: Vec<&CampaignFixedCandidateDatasetResult> = parsed_results.iter().collect();
    let observed = build_fixed_candidate_cohort_aggregate_from_results(
        &plan,
        aggregate.training.requested_datasets,
        &parsed_refs,
    )
    .ok_or_else(|| "failed to aggregate exact VCET training artifacts".to_string())?;
    if serde_json::to_value(&observed).ok() != serde_json::to_value(&aggregate.training).ok() {
        return Err(
            "campaign-bound Meta rejected the summary because its VCET training aggregate does not match the exact referenced artifacts"
                .to_string(),
        );
    }

    let policy = &req.validation_config.cross_universe_admission;
    let admitted_candidate_ids: Vec<String> = observed
        .candidates
        .iter()
        .filter(|row| fixed_candidate_meets_cross_universe_admission(row, policy))
        .map(|row| row.candidate_id.clone())
        .collect();
    let selected_candidate_id = select_fixed_candidate_id(&observed.candidates, policy);
    if aggregate.admitted_candidate_ids != admitted_candidate_ids
        || aggregate.admitted_candidate_count != admitted_candidate_ids.len()
        || aggregate.selected_candidate_id != selected_candidate_id
    {
        return Err(
            "campaign-bound Meta rejected inconsistent VCET admission evidence".to_string(),
        );
    }
    Ok(())
}

fn evaluate_vcet_fixed_candidate_artifact_reuse(
    existing_path: &str,
    loaded_artifact: Result<Value, String>,
    plan: &CampaignVcetFixedCandidatePlan,
    require_gpu_kernel: bool,
) -> CampaignFixedCandidateArtifactReuse {
    match loaded_artifact.and_then(|artifact| {
        parse_vcet_fixed_candidate_artifact_result(&artifact, plan, require_gpu_kernel)
    }) {
        Ok(result) => CampaignFixedCandidateArtifactReuse::Reuse(result),
        Err(error) => CampaignFixedCandidateArtifactReuse::Rerun {
            warning: format!(
                "Rejected incompatible fixed-candidate validation artifact '{existing_path}' and reran the dataset: {error}"
            ),
        },
    }
}

fn cscv_pbo_to_value(value: Option<&CscvPboConfig>) -> Value {
    match value {
        Some(cfg) => json!({
            "slices": cfg.slices,
            "max_combinations": cfg.max_combinations,
            "threshold": cfg.threshold,
        }),
        None => Value::Null,
    }
}

fn validation_config_summary_value(cfg: &ResearchCampaignValidationConfig) -> Value {
    json!({
        "max_candidates": cfg.max_candidates,
        "seed": cfg.seed,
        "admission_policy": cfg.admission_policy,
        "walk_forward": cfg.walk_forward,
        "reopt_walk_forward": cfg.reopt_walk_forward,
        "spa": cfg.spa,
        "perturb": cfg.perturb,
        "trade_bootstrap": cfg.trade_bootstrap,
        "monte_carlo": cfg.monte_carlo,
        "cross_ticker": cfg.cross_ticker,
        "cscv_pbo": cscv_pbo_to_value(cfg.cscv_pbo.as_ref()),
        "min_psr": cfg.min_psr,
        "min_dsr": cfg.min_dsr,
        "min_cscv_oos_rank_frac": cfg.min_cscv_oos_rank_frac,
        "min_cscv_oos_metric": cfg.min_cscv_oos_metric,
        "max_pbo": cfg.max_pbo,
        "fixed_candidates": cfg.fixed_candidates,
        "vcet_search": cfg.vcet_search,
        "require_gpu_kernel": cfg.require_gpu_kernel,
        "cross_universe_admission": cfg.cross_universe_admission,
    })
}

fn optimization_template_manifest_value(req: &ResearchCampaignRequest) -> Value {
    let mut template = req.optimization_template.clone();
    if let Value::Object(map) = &mut template {
        map.remove("data_id");
        map.remove("dataId");
    }
    json!({
        "strategy_kind": normalize_strategy_kind(&req.strategy_kind),
        "optimization_template": template,
    })
}

fn build_campaign_manifest(
    req: &ResearchCampaignRequest,
    strategy_kind: &str,
    timeframe: Option<&str>,
    tags: &[String],
    holdout_tag: Option<&str>,
    execution_mode: ResearchCampaignExecutionMode,
) -> ResearchCampaignManifestV2 {
    let dataset_paths: Vec<String> = req
        .datasets
        .iter()
        .map(|entry| normalize_text(&entry.csv_path))
        .filter(|entry| !entry.is_empty())
        .collect();
    let primary_tag = tags.first().map(String::as_str).unwrap_or_default();
    let (_train_paths, holdout_paths) =
        split_holdout_paths(&dataset_paths, req.holdout_pct, primary_tag);
    let dataset_rows: Vec<Value> = req
        .datasets
        .iter()
        .enumerate()
        .map(|(index, dataset)| {
            let csv_path = normalize_text(&dataset.csv_path);
            let timeframe = dataset_timeframe_hint(dataset, timeframe);
            let mut row = json!({
                "index": index,
                "csv_path": csv_path,
                "csv_path_key": normalize_path_key(&csv_path),
                "label": dataset.label,
                "timeframe": timeframe,
                "is_holdout": holdout_paths.contains(&csv_path),
                "source_size_bytes": dataset.source_size_bytes,
                "source_mtime_utc_ms": dataset.source_mtime_utc_ms,
                "source_content_fingerprint": dataset.source_content_fingerprint,
            });
            let has_content_fingerprint = dataset
                .source_content_fingerprint
                .as_deref()
                .map(normalize_text)
                .is_some_and(|value| !value.is_empty());
            if !has_content_fingerprint {
                if let Some(change_marker) = dataset.source_change_marker {
                    row["source_change_marker"] = Value::from(change_marker);
                }
            }
            row
        })
        .collect();
    let strict_identity_required = req.datasets.iter().all(|dataset| {
        dataset
            .source_content_fingerprint
            .as_deref()
            .map(normalize_text)
            .is_some_and(|value| !value.is_empty())
            || (dataset.source_size_bytes.is_some() && dataset.source_mtime_utc_ms.is_some())
    });
    let dataset_universe_hash = sha256_json_hash(&json!({
        "datasets": dataset_rows,
        "timeframe_override": timeframe,
    }));
    let optimization_template_hash = sha256_json_hash(&optimization_template_manifest_value(req));
    let validation_config_hash =
        sha256_json_hash(&validation_config_summary_value(&req.validation_config));
    let candidate_manifest_hash =
        build_vcet_fixed_candidate_plan(req, strategy_kind, execution_mode)
            .ok()
            .flatten()
            .map(|plan| plan.candidate_manifest_hash);
    let meta_request_hash = req
        .meta_request
        .as_ref()
        .and_then(|value| serde_json::to_value(value).ok())
        .map(|value| sha256_json_hash(&value));
    let stage5_request_hash = req
        .stage5_request
        .as_ref()
        .and_then(|value| serde_json::to_value(value).ok())
        .map(|value| sha256_json_hash(&value));
    let mut holdout_paths_vec: Vec<_> = holdout_paths.iter().cloned().collect();
    holdout_paths_vec.sort();
    let holdout_split_hash = sha256_json_hash(&json!({
        "algorithm": "fnv1a32_xor_primary_tag_floor_pct",
        "primary_tag": primary_tag,
        "holdout_pct": req.holdout_pct.min(50),
        "holdout_tag": holdout_tag,
        "holdout_paths": holdout_paths_vec,
    }));
    let mut manifest = ResearchCampaignManifestV2 {
        manifest_version: 2,
        manifest_hash: String::new(),
        dataset_universe_hash,
        optimization_template_hash,
        validation_config_hash,
        candidate_manifest_hash,
        meta_request_hash,
        stage5_request_hash,
        holdout_split_hash,
        strategy_kind: normalize_strategy_kind(strategy_kind),
        execution_mode,
        tags: tags.to_vec(),
        holdout_tag: holdout_tag.map(str::to_string),
        holdout_pct: req.holdout_pct.min(50),
        preload_policy: req.preload_policy,
        strict_identity_required,
    };
    let manifest_value = json!({
        "manifest_version": manifest.manifest_version,
        "dataset_universe_hash": &manifest.dataset_universe_hash,
        "optimization_template_hash": &manifest.optimization_template_hash,
        "validation_config_hash": &manifest.validation_config_hash,
        "candidate_manifest_hash": &manifest.candidate_manifest_hash,
        "meta_request_hash": &manifest.meta_request_hash,
        "stage5_request_hash": &manifest.stage5_request_hash,
        "holdout_split_hash": &manifest.holdout_split_hash,
        "strategy_kind": &manifest.strategy_kind,
        "execution_mode": manifest.execution_mode,
        "tags": &manifest.tags,
        "holdout_tag": &manifest.holdout_tag,
        "holdout_pct": manifest.holdout_pct,
        "preload_policy": manifest.preload_policy,
        "strict_identity_required": manifest.strict_identity_required,
    });
    manifest.manifest_hash = sha256_json_hash(&manifest_value);
    manifest
}

fn campaign_summary_evidence_state(
    counts: &ResearchCampaignCounts,
    items: &[ResearchCampaignItemSummary],
) -> CampaignEvidenceState {
    if items
        .iter()
        .any(|item| item.evidence_state == Some(CampaignEvidenceState::Unsafe))
    {
        return CampaignEvidenceState::Unsafe;
    }
    if items
        .iter()
        .any(|item| item.evidence_state == Some(CampaignEvidenceState::Stale))
    {
        return CampaignEvidenceState::Stale;
    }
    if counts.total > 0
        && counts.failed == 0
        && !counts.cancelled
        && items.iter().all(|item| {
            item.evidence_state == Some(CampaignEvidenceState::Complete)
                || matches!(item.status.as_str(), "ok" | "optimized")
        })
    {
        return CampaignEvidenceState::Complete;
    }
    if counts.optimized > 0 || counts.validated > 0 || counts.skipped > 0 || counts.failed > 0 {
        CampaignEvidenceState::Partial
    } else {
        CampaignEvidenceState::None
    }
}

fn campaign_summary_resume_state(
    counts: &ResearchCampaignCounts,
    items: &[ResearchCampaignItemSummary],
) -> CampaignResumeState {
    if items
        .iter()
        .any(|item| item.resume_state == Some(CampaignResumeState::BlockedUnsafe))
    {
        return CampaignResumeState::BlockedUnsafe;
    }
    if items
        .iter()
        .any(|item| item.resume_state == Some(CampaignResumeState::BlockedStale))
    {
        return CampaignResumeState::BlockedStale;
    }
    if items
        .iter()
        .any(|item| item.resume_state == Some(CampaignResumeState::RestartOnly))
    {
        return CampaignResumeState::RestartOnly;
    }
    if items
        .iter()
        .any(|item| item.resume_state == Some(CampaignResumeState::LegacyDegraded))
    {
        return CampaignResumeState::LegacyDegraded;
    }
    if counts.total > 0
        && counts.failed == 0
        && !counts.cancelled
        && items
            .iter()
            .all(|item| item.resume_state == Some(CampaignResumeState::Complete))
    {
        return CampaignResumeState::Complete;
    }
    CampaignResumeState::Resumable
}

fn normalize_campaign_auto_meta_request(
    mut meta_req: MetaSelectionRequest,
    strategy_kind: &str,
    timeframe: Option<&str>,
    primary_tag: &str,
    holdout_tag: Option<&str>,
    has_holdout: bool,
    warnings: &mut Vec<String>,
) -> Result<MetaSelectionRequest, String> {
    let normalized_strategy = normalize_strategy_kind(strategy_kind);
    if normalize_strategy_kind(&meta_req.strategy_kind) != normalized_strategy {
        push_unique_warning(
            warnings,
            format!(
                "auto_meta strategy_kind was patched from '{}' to '{}' to match the campaign.",
                meta_req.strategy_kind, normalized_strategy
            ),
        );
        meta_req.strategy_kind = normalized_strategy;
    }
    let required_tag = meta_req
        .required_tag
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty());
    if required_tag.as_deref() != Some(primary_tag) {
        push_unique_warning(
            warnings,
            format!(
                "auto_meta required_tag was patched to campaign primary tag '{}'.",
                primary_tag
            ),
        );
        meta_req.required_tag = Some(primary_tag.to_string());
    }
    if let Some(timeframe) = timeframe
        .map(normalize_timeframe_label)
        .filter(|value| !value.is_empty())
    {
        let meta_timeframe = meta_req
            .timeframe
            .as_deref()
            .map(normalize_timeframe_label)
            .filter(|value| !value.is_empty());
        if meta_timeframe.as_deref() != Some(timeframe.as_str()) {
            push_unique_warning(
                warnings,
                format!(
                    "auto_meta timeframe was patched to campaign timeframe '{}'.",
                    timeframe
                ),
            );
            meta_req.timeframe = Some(timeframe);
        }
    }
    if has_holdout {
        let holdout_tag = holdout_tag
            .map(normalize_text)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "auto_meta holdout evaluation was blocked because no holdout tag was available"
                    .to_string()
            })?;
        if primary_tag.eq_ignore_ascii_case(&holdout_tag) {
            return Err(
                "auto_meta holdout evaluation was blocked because primary_tag and holdout_tag are identical"
                    .to_string(),
            );
        }
        let current_holdout_tag = meta_req
            .holdout_tag
            .as_deref()
            .map(normalize_text)
            .filter(|value| !value.is_empty());
        if current_holdout_tag.as_deref() != Some(holdout_tag.as_str()) {
            push_unique_warning(
                warnings,
                format!(
                    "auto_meta holdout_tag was patched to campaign holdout tag '{}'.",
                    holdout_tag
                ),
            );
            meta_req.holdout_tag = Some(holdout_tag);
        }
        if !meta_req.eval_holdout {
            push_unique_warning(
                warnings,
                "auto_meta eval_holdout was enabled because the campaign has a holdout split."
                    .to_string(),
            );
            meta_req.eval_holdout = true;
        }
    } else if meta_req.eval_holdout {
        push_unique_warning(
            warnings,
            "auto_meta eval_holdout was disabled because this campaign has no holdout split."
                .to_string(),
        );
        meta_req.eval_holdout = false;
        meta_req.holdout_tag = None;
    }
    Ok(meta_req)
}

fn build_request_summary_value(
    req: &ResearchCampaignRequest,
    resolved_tags: &[String],
    execution_mode: ResearchCampaignExecutionMode,
) -> Value {
    json!({
        "strategy_kind": normalize_strategy_kind(&req.strategy_kind),
        "execution_mode": execution_mode,
        "datasets": req.datasets.iter().map(|dataset| {
            json!({
                "csv_path": dataset.csv_path,
                "label": dataset.label,
                "timeframe": dataset.timeframe,
                "source_size_bytes": dataset.source_size_bytes,
                "source_mtime_utc_ms": dataset.source_mtime_utc_ms,
                "source_change_marker": dataset.source_change_marker,
                "source_content_fingerprint": dataset.source_content_fingerprint,
            })
        }).collect::<Vec<_>>(),
        "timeframe_override": req.timeframe_override,
        "tags": resolved_tags,
        "holdout_pct": req.holdout_pct,
        "holdout_tag": req.holdout_tag,
        "skip_existing": req.skip_existing,
        "auto_meta": req.auto_meta,
        "auto_stage5": req.auto_stage5,
        "optimization_template": req.optimization_template,
        "validation_config": validation_config_summary_value(&req.validation_config),
        "meta_request": req.meta_request,
        "stage5_request": req.stage5_request,
        "preload_policy": req.preload_policy,
    })
}

fn finite_percentile(mut values: Vec<f64>, percentile: f64) -> Option<f64> {
    values.retain(|value| value.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    if values.len() == 1 {
        return values.first().copied();
    }
    let position = percentile.clamp(0.0, 1.0) * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        return values.get(lower).copied();
    }
    let weight = position - lower as f64;
    Some(values[lower] * (1.0 - weight) + values[upper] * weight)
}

/// Computes a percentile against the complete requested cohort. Missing or
/// failed observations occupy the lowest ranks, below every finite value. If
/// the percentile position itself lands inside that missing mass, the result is
/// unavailable so admission fails closed. At the first finite rank and above,
/// the usual linearly interpolated percentile is evaluated after subtracting
/// the missing-rank offset.
fn finite_percentile_over_requested(
    mut values: Vec<f64>,
    requested_datasets: usize,
    percentile: f64,
) -> Option<f64> {
    values.retain(|value| value.is_finite());
    if requested_datasets == 0 || values.is_empty() || values.len() > requested_datasets {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let missing = requested_datasets.saturating_sub(values.len());
    let global_position = percentile.clamp(0.0, 1.0) * (requested_datasets - 1) as f64;
    if global_position < missing as f64 {
        return None;
    }
    let observed_position = global_position - missing as f64;
    let lower = observed_position.floor() as usize;
    let upper = observed_position.ceil() as usize;
    let lower_value = *values.get(lower)?;
    let upper_value = *values.get(upper)?;
    if lower == upper {
        return Some(lower_value);
    }
    let weight = observed_position - lower as f64;
    Some(lower_value * (1.0 - weight) + upper_value * weight)
}

fn build_fixed_candidate_cohort_aggregate<'a>(
    plan: &CampaignVcetFixedCandidatePlan,
    cohort_items: impl Iterator<Item = &'a ResearchCampaignItemSummary>,
) -> Option<ResearchCampaignFixedCandidateCohortAggregate> {
    let cohort_items: Vec<&ResearchCampaignItemSummary> = cohort_items.collect();
    let requested_datasets = cohort_items.len();
    let dataset_results: Vec<&CampaignFixedCandidateDatasetResult> = cohort_items
        .iter()
        .filter_map(|item| item.fixed_candidate_result.as_ref())
        .collect();
    build_fixed_candidate_cohort_aggregate_from_results(plan, requested_datasets, &dataset_results)
}

fn build_fixed_candidate_cohort_aggregate_from_results(
    plan: &CampaignVcetFixedCandidatePlan,
    requested_datasets: usize,
    dataset_results: &[&CampaignFixedCandidateDatasetResult],
) -> Option<ResearchCampaignFixedCandidateCohortAggregate> {
    if plan.candidate_identities.len() != plan.candidates.len() {
        return None;
    }
    let evaluated_datasets = dataset_results.len();
    let gpu_kernel_datasets = dataset_results
        .iter()
        .filter(|result| result.gpu_kernel_used)
        .count();
    let requested_denominator = requested_datasets as f64;

    let mut candidates = Vec::with_capacity(plan.candidates.len());
    for (candidate_index, (candidate, identity)) in plan
        .candidates
        .iter()
        .zip(plan.candidate_identities.iter().copied())
        .enumerate()
    {
        let fast_len = candidate
            .get("fast_len")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())?;
        let slow_len = candidate
            .get("slow_len")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())?;
        let observations: Vec<&CampaignFixedCandidateObservation> = dataset_results
            .iter()
            .filter_map(|result| {
                result
                    .candidates
                    .iter()
                    .find(|row| row.identity == identity)
            })
            .collect();
        let expected_search_evaluation_id = plan.search_manifest.as_ref().and_then(|manifest| {
            manifest
                .candidates
                .get(candidate_index)
                .and_then(|sampled| sampled.evaluation_candidate_id.as_deref())
        });
        if expected_search_evaluation_id
            .is_some_and(|candidate_id| candidate_id != vcet_fixed_candidate_id(identity))
        {
            return None;
        }
        let evaluated = observations.len();
        let profitable = observations.iter().filter(|row| row.pnl > 0.0).count();
        let passed = observations
            .iter()
            .filter(|row| row.hard_gate_passed)
            .count();
        let pnl: Vec<f64> = observations.iter().map(|row| row.pnl).collect();
        let sharpe: Vec<f64> = observations.iter().map(|row| row.sharpe).collect();
        let max_drawdown: Vec<f64> = observations.iter().map(|row| row.max_drawdown).collect();
        let trades: Vec<f64> = observations
            .iter()
            .map(|row| f64::from(row.trades))
            .collect();
        candidates.push(ResearchCampaignFixedCandidateAggregate {
            candidate_id: vcet_fixed_candidate_id(identity),
            fast_len,
            slow_len,
            requested_datasets,
            evaluated_datasets: evaluated,
            missing_or_failed_datasets: requested_datasets.saturating_sub(evaluated),
            profitable_datasets: profitable,
            hard_gate_pass_datasets: passed,
            profitable_share_requested: if requested_datasets == 0 {
                0.0
            } else {
                profitable as f64 / requested_denominator
            },
            hard_gate_pass_share_requested: if requested_datasets == 0 {
                0.0
            } else {
                passed as f64 / requested_denominator
            },
            pnl_p10: finite_percentile_over_requested(pnl.clone(), requested_datasets, 0.10),
            pnl_median: finite_percentile(pnl, 0.50),
            sharpe_p10: finite_percentile_over_requested(sharpe.clone(), requested_datasets, 0.10),
            sharpe_median: finite_percentile(sharpe, 0.50),
            max_drawdown_p90: finite_percentile(max_drawdown, 0.90),
            trades_median: finite_percentile(trades, 0.50),
        });
    }

    Some(ResearchCampaignFixedCandidateCohortAggregate {
        requested_datasets,
        evaluated_datasets,
        gpu_kernel_datasets,
        missing_or_failed_datasets: requested_datasets.saturating_sub(evaluated_datasets),
        candidates,
    })
}

fn compare_fixed_candidate_rank(
    left: &ResearchCampaignFixedCandidateAggregate,
    right: &ResearchCampaignFixedCandidateAggregate,
) -> std::cmp::Ordering {
    left.profitable_share_requested
        .total_cmp(&right.profitable_share_requested)
        .then_with(|| {
            left.hard_gate_pass_share_requested
                .total_cmp(&right.hard_gate_pass_share_requested)
        })
        .then_with(|| {
            left.sharpe_p10
                .unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&right.sharpe_p10.unwrap_or(f64::NEG_INFINITY))
        })
        .then_with(|| {
            left.pnl_p10
                .unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&right.pnl_p10.unwrap_or(f64::NEG_INFINITY))
        })
        .then_with(|| right.candidate_id.cmp(&left.candidate_id))
}

fn fixed_candidate_meets_cross_universe_admission(
    row: &ResearchCampaignFixedCandidateAggregate,
    policy: &ResearchCampaignCrossUniverseAdmissionPolicy,
) -> bool {
    row.evaluated_datasets > 0
        && row.profitable_share_requested >= policy.min_profitable_share_requested
        && row.hard_gate_pass_share_requested >= policy.min_hard_gate_pass_share_requested
        && row.pnl_p10.is_some_and(|value| value >= policy.min_pnl_p10)
        && row
            .sharpe_p10
            .is_some_and(|value| value >= policy.min_sharpe_p10)
}

fn select_fixed_candidate_id_legacy(
    candidates: &[ResearchCampaignFixedCandidateAggregate],
) -> Option<String> {
    candidates
        .iter()
        .filter(|row| row.evaluated_datasets > 0)
        .max_by(|left, right| compare_fixed_candidate_rank(left, right))
        .map(|row| row.candidate_id.clone())
}

fn select_fixed_candidate_id(
    candidates: &[ResearchCampaignFixedCandidateAggregate],
    policy: &ResearchCampaignCrossUniverseAdmissionPolicy,
) -> Option<String> {
    candidates
        .iter()
        .filter(|row| fixed_candidate_meets_cross_universe_admission(row, policy))
        .max_by(|left, right| compare_fixed_candidate_rank(left, right))
        .map(|row| row.candidate_id.clone())
}

fn build_fixed_candidate_evaluation_summary(
    req: &ResearchCampaignRequest,
    execution_mode: ResearchCampaignExecutionMode,
    items: &[ResearchCampaignItemSummary],
) -> Option<ResearchCampaignFixedCandidateEvaluationSummary> {
    let plan = build_vcet_fixed_candidate_plan(req, &req.strategy_kind, execution_mode)
        .ok()
        .flatten()?;
    let all_universe = build_fixed_candidate_cohort_aggregate(&plan, items.iter())?;
    let training = build_fixed_candidate_cohort_aggregate(
        &plan,
        items.iter().filter(|item| !item.is_holdout),
    )?;
    let holdout =
        build_fixed_candidate_cohort_aggregate(&plan, items.iter().filter(|item| item.is_holdout))?;
    let candidates = training.candidates.clone();
    let cross_universe_admission = req.validation_config.cross_universe_admission;
    let admitted_candidate_ids: Vec<String> = candidates
        .iter()
        .filter(|row| {
            fixed_candidate_meets_cross_universe_admission(row, &cross_universe_admission)
        })
        .map(|row| row.candidate_id.clone())
        .collect();
    let admitted_candidate_count = admitted_candidate_ids.len();
    let selected_candidate_id = select_fixed_candidate_id(&candidates, &cross_universe_admission);
    let selection_status = if selected_candidate_id.is_some() {
        "selected"
    } else {
        "no_candidate_met_admission"
    }
    .to_string();
    Some(ResearchCampaignFixedCandidateEvaluationSummary {
        schema_version: 4,
        candidate_manifest_hash: plan.candidate_manifest_hash.clone(),
        search_manifest: plan.search_manifest.clone(),
        require_gpu_kernel: req.validation_config.require_gpu_kernel,
        requested_datasets: all_universe.requested_datasets,
        evaluated_datasets: all_universe.evaluated_datasets,
        gpu_kernel_datasets: all_universe.gpu_kernel_datasets,
        missing_or_failed_datasets: all_universe.missing_or_failed_datasets,
        candidates,
        selection_cohort: "training".to_string(),
        selected_candidate_id,
        cross_universe_admission,
        admitted_candidate_count,
        admitted_candidate_ids,
        selection_status,
        all_universe,
        training,
        holdout,
    })
}

fn build_campaign_summary(
    ts: &str,
    state: &str,
    strategy_kind: &str,
    timeframe: Option<String>,
    req: &ResearchCampaignRequest,
    execution_mode: ResearchCampaignExecutionMode,
    tags: &[String],
    holdout_tag: Option<String>,
    counts: &ResearchCampaignCounts,
    items: &[ResearchCampaignItemSummary],
    meta_response: Option<&RunMetaSelectionResponse>,
    stage5_response: Option<&RunMetaStage5Response>,
    warnings: &[String],
    runtime_ms: u64,
) -> ResearchCampaignSummary {
    let manifest = build_campaign_manifest(
        req,
        strategy_kind,
        timeframe.as_deref(),
        tags,
        holdout_tag.as_deref(),
        execution_mode,
    );
    let cohort_id = manifest.manifest_hash.clone();
    ResearchCampaignSummary {
        version: 2,
        kind: "vectorgrid_campaign_summary".to_string(),
        ts: ts.to_string(),
        state: state.to_string(),
        retention: Some(ResearchCampaignRetentionSummary {
            class: "campaign_summary".to_string(),
            cohort_id,
        }),
        manifest: Some(manifest),
        evidence_state: Some(campaign_summary_evidence_state(counts, items)),
        resume_state: Some(campaign_summary_resume_state(counts, items)),
        strategy_kind: strategy_kind.to_string(),
        timeframe,
        tags: tags.to_vec(),
        holdout_tag,
        holdout_pct: req.holdout_pct.min(50),
        request: build_request_summary_value(req, tags, execution_mode),
        counts: counts.clone(),
        // Per-dataset candidate metrics are needed transiently to build the
        // aggregate above, but are intentionally not serialized on each item.
        // Clear them before cloning the checkpoint/response projection so a
        // large campaign does not retain a second hidden copy of the complete
        // candidate matrix for every summary checkpoint.
        items: items
            .iter()
            .cloned()
            .map(|mut item| {
                item.fixed_candidate_result = None;
                item
            })
            .collect(),
        fixed_candidate_evaluation: build_fixed_candidate_evaluation_summary(
            req,
            execution_mode,
            items,
        ),
        meta_json_path: meta_response.map(|value| value.meta_json_path.clone()),
        stage5_json_path: stage5_response.map(|value| value.stage5_json_path.clone()),
        warnings: warnings.to_vec(),
        runtime_ms,
    }
}

fn is_campaign_summary_kind(kind: &str) -> bool {
    matches!(
        kind.trim(),
        "vectorgrid_campaign_summary" | "vectorbt_campaign_summary"
    )
}

fn fixed_candidate_rows_are_consistent(
    rows: &[ResearchCampaignFixedCandidateAggregate],
    plan: &CampaignVcetFixedCandidatePlan,
    requested_datasets: usize,
) -> bool {
    if rows.len() != plan.candidates.len()
        || plan.candidate_identities.len() != plan.candidates.len()
    {
        return false;
    }
    rows.iter().enumerate().all(|(index, row)| {
        let expected = &plan.candidates[index];
        let identity = plan.candidate_identities[index];
        let Some(fast_len) = expected
            .get("fast_len")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
        else {
            return false;
        };
        let Some(slow_len) = expected
            .get("slow_len")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
        else {
            return false;
        };
        let denominator = requested_datasets as f64;
        let profitable_share = if requested_datasets == 0 {
            0.0
        } else {
            row.profitable_datasets as f64 / denominator
        };
        let hard_gate_share = if requested_datasets == 0 {
            0.0
        } else {
            row.hard_gate_pass_datasets as f64 / denominator
        };
        row.fast_len == fast_len
            && row.slow_len == slow_len
            && row.candidate_id == vcet_fixed_candidate_id(identity)
            && row.requested_datasets == requested_datasets
            && row.evaluated_datasets <= row.requested_datasets
            && row.missing_or_failed_datasets
                == row
                    .requested_datasets
                    .saturating_sub(row.evaluated_datasets)
            && row.profitable_datasets <= row.evaluated_datasets
            && row.hard_gate_pass_datasets <= row.evaluated_datasets
            && (row.profitable_share_requested - profitable_share).abs() <= 1e-12
            && (row.hard_gate_pass_share_requested - hard_gate_share).abs() <= 1e-12
    })
}

fn fixed_candidate_cohort_is_consistent(
    cohort: &ResearchCampaignFixedCandidateCohortAggregate,
    plan: &CampaignVcetFixedCandidatePlan,
    expected_requested_datasets: usize,
) -> bool {
    cohort.requested_datasets == expected_requested_datasets
        && cohort.evaluated_datasets <= cohort.requested_datasets
        && cohort.gpu_kernel_datasets <= cohort.evaluated_datasets
        && cohort.missing_or_failed_datasets
            == cohort
                .requested_datasets
                .saturating_sub(cohort.evaluated_datasets)
        && fixed_candidate_rows_are_consistent(&cohort.candidates, plan, cohort.requested_datasets)
}

pub(super) fn validate_campaign_summary_artifact_value(value: &Value) -> Result<(), String> {
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !is_campaign_summary_kind(kind) {
        return Ok(());
    }

    let version = value.get("version").and_then(Value::as_u64).unwrap_or(0);
    if version < 2 {
        // Legacy summaries remain viewable, but the frontend treats them as
        // restart-only because they do not carry the v2 manifest contract.
        return Ok(());
    }
    if version != 2 {
        return Err(format!(
            "unsupported campaign summary version {version}; restart the campaign with this build"
        ));
    }

    let summary: ResearchCampaignSummary = serde_json::from_value(value.clone())
        .map_err(|error| format!("invalid campaign summary v2 structure: {error}"))?;
    let manifest = summary
        .manifest
        .as_ref()
        .ok_or_else(|| "campaign summary v2 is missing its integrity manifest".to_string())?;
    if summary.retention.as_ref().is_some_and(|retention| {
        retention.class != "campaign_summary" || retention.cohort_id != manifest.manifest_hash
    }) {
        return Err(
            "campaign summary retention cohort does not match its integrity manifest".to_string(),
        );
    }
    for item in &summary.items {
        let output_complete = item.status.trim().eq_ignore_ascii_case("ok")
            || item.evidence_state == Some(CampaignEvidenceState::Complete)
            || item.resume_state == Some(CampaignResumeState::Complete);
        if output_complete
            && item
                .artifact_path
                .as_deref()
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .is_none()
        {
            return Err(format!(
                "campaign summary item '{}' is marked complete but has no run-artifact path",
                item.label
            ));
        }
    }
    let req: ResearchCampaignRequest = serde_json::from_value(summary.request.clone())
        .map_err(|error| format!("campaign summary request is not resumable: {error}"))?;
    let request_declares_cross_universe_admission = summary
        .request
        .pointer("/validation_config/cross_universe_admission")
        .is_some();

    let strategy_kind = resolve_campaign_strategy_kind(&req.strategy_kind)
        .map_err(|error| format!("campaign summary request is not resumable: {error}"))?;
    let fixed_candidate_plan =
        build_vcet_fixed_candidate_plan(&req, &strategy_kind, req.execution_mode)
            .map_err(|error| format!("campaign summary request is not resumable: {error}"))?;
    if normalize_strategy_kind(&summary.strategy_kind) != strategy_kind {
        return Err(
            "campaign summary strategy does not match its resumable request; restart the campaign"
                .to_string(),
        );
    }
    let expected_timeframe = req
        .timeframe_override
        .as_deref()
        .map(normalize_timeframe_label)
        .filter(|entry| !entry.is_empty());
    let summary_timeframe = summary
        .timeframe
        .as_deref()
        .map(normalize_timeframe_label)
        .filter(|entry| !entry.is_empty());
    if expected_timeframe != summary_timeframe {
        return Err(
            "campaign summary timeframe does not match its resumable request; restart the campaign"
                .to_string(),
        );
    }
    if summary.tags != req.tags {
        return Err(
            "campaign summary tags do not match its resumable request; restart the campaign"
                .to_string(),
        );
    }
    if summary.holdout_pct != req.holdout_pct.min(50) {
        return Err(
            "campaign summary holdout split does not match its resumable request; restart the campaign"
                .to_string(),
        );
    }

    let expected_manifest = build_campaign_manifest(
        &req,
        &strategy_kind,
        summary_timeframe.as_deref(),
        &summary.tags,
        summary.holdout_tag.as_deref(),
        req.execution_mode,
    );
    if manifest != &expected_manifest {
        return Err(
            "campaign summary integrity manifest does not match its resumable request; the summary is corrupt or was modified"
                .to_string(),
        );
    }

    match (
        fixed_candidate_plan.as_ref(),
        summary.fixed_candidate_evaluation.as_ref(),
    ) {
        (None, None) => {}
        (None, Some(_)) => {
            return Err(
                "campaign summary contains fixed-candidate results outside evaluate_candidates mode"
                    .to_string(),
            );
        }
        (Some(_), None) => {
            return Err(
                "evaluate_candidates campaign summary is missing its fixed-candidate aggregate"
                    .to_string(),
            );
        }
        (Some(plan), Some(aggregate)) => {
            let supported_aggregate_schema = if request_declares_cross_universe_admission {
                aggregate.schema_version == 4
            } else {
                matches!(aggregate.schema_version, 1 | 2)
            };
            if !supported_aggregate_schema
                || aggregate.candidate_manifest_hash != plan.candidate_manifest_hash
                || aggregate.search_manifest != plan.search_manifest
                || manifest.candidate_manifest_hash.as_deref()
                    != Some(plan.candidate_manifest_hash.as_str())
                || aggregate.require_gpu_kernel != req.validation_config.require_gpu_kernel
                || aggregate.requested_datasets != req.datasets.len()
                || aggregate.evaluated_datasets > aggregate.requested_datasets
                || aggregate.gpu_kernel_datasets > aggregate.evaluated_datasets
                || aggregate.missing_or_failed_datasets
                    != aggregate
                        .requested_datasets
                        .saturating_sub(aggregate.evaluated_datasets)
            {
                return Err(
                    "campaign fixed-candidate aggregate does not match its resumable manifest"
                        .to_string(),
                );
            }
            let cohorts_consistent = if aggregate.schema_version == 1 {
                fixed_candidate_rows_are_consistent(
                    &aggregate.candidates,
                    plan,
                    aggregate.requested_datasets,
                )
            } else {
                let training_requested =
                    summary.items.iter().filter(|item| !item.is_holdout).count();
                let holdout_requested = summary.items.iter().filter(|item| item.is_holdout).count();
                summary.retention.is_some()
                    && aggregate.selection_cohort == "training"
                    && aggregate.requested_datasets == aggregate.all_universe.requested_datasets
                    && aggregate.evaluated_datasets == aggregate.all_universe.evaluated_datasets
                    && aggregate.gpu_kernel_datasets == aggregate.all_universe.gpu_kernel_datasets
                    && aggregate.missing_or_failed_datasets
                        == aggregate.all_universe.missing_or_failed_datasets
                    && serde_json::to_value(&aggregate.candidates).ok()
                        == serde_json::to_value(&aggregate.training.candidates).ok()
                    && if aggregate.schema_version == 2 {
                        aggregate.selected_candidate_id
                            == select_fixed_candidate_id_legacy(&aggregate.training.candidates)
                    } else {
                        let policy = &req.validation_config.cross_universe_admission;
                        let admitted_candidate_count = aggregate
                            .training
                            .candidates
                            .iter()
                            .filter(|row| {
                                fixed_candidate_meets_cross_universe_admission(row, policy)
                            })
                            .count();
                        let admitted_candidate_ids: Vec<String> = aggregate
                            .training
                            .candidates
                            .iter()
                            .filter(|row| {
                                fixed_candidate_meets_cross_universe_admission(row, policy)
                            })
                            .map(|row| row.candidate_id.clone())
                            .collect();
                        let selected_candidate_id =
                            select_fixed_candidate_id(&aggregate.training.candidates, policy);
                        aggregate.cross_universe_admission == *policy
                            && aggregate.admitted_candidate_count == admitted_candidate_count
                            && aggregate.admitted_candidate_ids == admitted_candidate_ids
                            && aggregate.selected_candidate_id == selected_candidate_id
                            && aggregate.selection_status
                                == if selected_candidate_id.is_some() {
                                    "selected"
                                } else {
                                    "no_candidate_met_admission"
                                }
                    }
                    && fixed_candidate_cohort_is_consistent(
                        &aggregate.all_universe,
                        plan,
                        req.datasets.len(),
                    )
                    && fixed_candidate_cohort_is_consistent(
                        &aggregate.training,
                        plan,
                        training_requested,
                    )
                    && fixed_candidate_cohort_is_consistent(
                        &aggregate.holdout,
                        plan,
                        holdout_requested,
                    )
            };
            if !cohorts_consistent {
                return Err(
                    "campaign fixed-candidate aggregate contains inconsistent candidate counts or cohort separation"
                        .to_string(),
                );
            }
            let terminal = matches!(
                summary.state.trim(),
                "completed" | "completed_with_failures" | "completed_with_warnings"
            );
            if aggregate.schema_version >= 4 && terminal {
                for item in &summary.items {
                    if item.artifact_path.is_some()
                        && item
                            .artifact_sha256
                            .as_deref()
                            .map(str::trim)
                            .is_none_or(|sha256| {
                                sha256.len() != 64
                                    || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
                            })
                    {
                        return Err(format!(
                            "terminal VCET campaign summary item '{}' is missing its sealed raw run-artifact SHA-256",
                            item.label
                        ));
                    }
                }
            }
        }
    }

    if summary.counts.total != req.datasets.len() || summary.items.len() != req.datasets.len() {
        return Err(
            "campaign summary item count does not match its resumable request; restart the campaign"
                .to_string(),
        );
    }
    for count in [
        summary.counts.optimized,
        summary.counts.validated,
        summary.counts.skipped,
        summary.counts.failed,
    ] {
        if count > summary.counts.total {
            return Err(
                "campaign summary progress counts exceed the dataset total; the summary is corrupt"
                    .to_string(),
            );
        }
    }

    let dataset_paths = req
        .datasets
        .iter()
        .map(|dataset| normalize_text(&dataset.csv_path))
        .collect::<Vec<_>>();
    let primary_tag = summary.tags.first().map(String::as_str).unwrap_or_default();
    let (_, holdout_paths) = split_holdout_paths(&dataset_paths, req.holdout_pct, primary_tag);
    let mut item_ids = HashSet::new();
    let mut artifact_paths = HashSet::new();
    for (index, (dataset, item)) in req.datasets.iter().zip(&summary.items).enumerate() {
        let csv_path = normalize_text(&dataset.csv_path);
        let timeframe = dataset_timeframe_hint(dataset, summary_timeframe.as_deref());
        let label = dataset_label_from_input(dataset);
        if normalize_path_key(&item.csv_path) != normalize_path_key(&csv_path)
            || normalize_timeframe_label(&item.timeframe) != normalize_timeframe_label(&timeframe)
            || normalize_text(&item.label) != normalize_text(&label)
            || item.is_holdout != holdout_paths.contains(&csv_path)
        {
            return Err(format!(
                "campaign summary item {} does not match its resumable dataset identity",
                index + 1
            ));
        }

        let expected_item_id =
            campaign_item_id(&manifest.manifest_hash, index, &csv_path, &timeframe);
        match item.item_id.as_deref().map(str::trim) {
            Some(actual) if actual == expected_item_id => {
                if !item_ids.insert(actual.to_string()) {
                    return Err(
                        "campaign summary contains duplicate stable item identities".to_string()
                    );
                }
            }
            Some(_) => {
                return Err(format!(
                    "campaign summary item {} has a mismatched stable identity",
                    index + 1
                ));
            }
            None if item.status == "failed" => {}
            None => {
                return Err(format!(
                    "campaign summary item {} is missing its stable identity",
                    index + 1
                ));
            }
        }

        if let Some(path) = item
            .artifact_path
            .as_deref()
            .map(normalize_text)
            .filter(|path| !path.is_empty())
        {
            let key = normalize_path_key(&path);
            if !artifact_paths.insert(key) {
                return Err("campaign summary contains duplicate run-artifact paths".to_string());
            }
        }
    }

    let expected_evidence = campaign_summary_evidence_state(&summary.counts, &summary.items);
    if summary.evidence_state != Some(expected_evidence) {
        return Err(
            "campaign summary evidence state is inconsistent with its item checkpoints".to_string(),
        );
    }
    let expected_resume = campaign_summary_resume_state(&summary.counts, &summary.items);
    if summary.resume_state != Some(expected_resume) {
        return Err(
            "campaign summary resume state is inconsistent with its item checkpoints".to_string(),
        );
    }
    if (summary.state == "cancelled") != summary.counts.cancelled {
        return Err(
            "campaign summary cancellation state is inconsistent with its progress counts"
                .to_string(),
        );
    }
    if matches!(
        summary.state.as_str(),
        "completed" | "completed_with_failures" | "completed_with_warnings"
    ) && summary.counts.cancelled
    {
        return Err(
            "campaign summary reports completion after cancellation; restart the campaign"
                .to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
fn campaign_summary_temp_path(path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("campaign summary path has no file name: {}", path.display()))?;
    let counter = CAMPAIGN_SUMMARY_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(parent.join(format!(
        ".{}.{}.{}.{}.tmp",
        file_name.to_string_lossy(),
        std::process::id(),
        nanos,
        counter
    )))
}

#[cfg(test)]
fn existing_campaign_summary_target(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(format!(
                    "refusing to replace non-regular campaign summary target {}",
                    path.display()
                ));
            }
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "failed to inspect campaign summary target {}: {error}",
            path.display()
        )),
    }
}

#[cfg(test)]
fn replace_existing_campaign_summary(staged_path: &Path, final_path: &Path) -> Result<(), String> {
    // The staged file lives beside the target, so this is a same-filesystem atomic
    // rename. On Windows, Rust implements this with MOVEFILE_REPLACE_EXISTING.
    fs::rename(staged_path, final_path).map_err(|error| {
        format!(
            "failed to atomically replace campaign summary {}: {error}",
            final_path.display()
        )
    })
}

#[cfg(test)]
fn publish_campaign_summary(
    staged_path: &Path,
    final_path: &Path,
    target_exists: bool,
) -> Result<(), String> {
    if target_exists {
        replace_existing_campaign_summary(staged_path, final_path)
    } else {
        fs::rename(staged_path, final_path).map_err(|error| {
            format!(
                "failed to publish campaign summary {}: {error}",
                final_path.display()
            )
        })
    }
}

#[cfg(test)]
fn write_campaign_summary_bytes_with_publish(
    path: &Path,
    bytes: &[u8],
    publish: impl FnOnce(&Path, &Path, bool) -> Result<(), String>,
) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create campaign summary directory {}: {error}",
            parent.display()
        )
    })?;
    let target_exists = existing_campaign_summary_target(path)?;

    let (staged_path, mut staged_file) = {
        let mut staged = None;
        for _ in 0..64 {
            let candidate = campaign_summary_temp_path(path)?;
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(file) => {
                    staged = Some((candidate, file));
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!(
                        "failed to create staged campaign summary {}: {error}",
                        candidate.display()
                    ));
                }
            }
        }
        staged.ok_or_else(|| {
            format!(
                "failed to allocate a unique staged campaign summary for {}",
                path.display()
            )
        })?
    };

    let stage_result = (|| -> Result<(), String> {
        staged_file.write_all(bytes).map_err(|error| {
            format!(
                "failed to write staged campaign summary {}: {error}",
                staged_path.display()
            )
        })?;
        staged_file.sync_all().map_err(|error| {
            format!(
                "failed to sync staged campaign summary {}: {error}",
                staged_path.display()
            )
        })
    })();
    drop(staged_file);
    if let Err(error) = stage_result {
        let _ = fs::remove_file(&staged_path);
        return Err(error);
    }

    if let Err(error) = publish(&staged_path, path, target_exists) {
        let _ = fs::remove_file(&staged_path);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
fn write_campaign_summary_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_campaign_summary_bytes_with_publish(path, bytes, publish_campaign_summary)
}

fn save_campaign_summary_blocking<R: tauri::Runtime>(
    summary: &ResearchCampaignSummary,
    app: &tauri::AppHandle<R>,
    existing_path: Option<&str>,
) -> Result<(String, usize), String> {
    let bytes = serde_json::to_vec_pretty(summary).map_err(|e| e.to_string())?;
    let _artifact_library_lock = run_artifacts::acquire_artifact_library_process_lock(app)?;
    let replace_existing = existing_path
        .map(PathBuf::from)
        .is_some_and(|value| !value.as_os_str().is_empty());
    let path = match existing_path
        .map(PathBuf::from)
        .filter(|value| !value.as_os_str().is_empty())
    {
        Some(path) => path,
        None => run_artifacts::suggest_campaign_summary_path(&summary.ts, app)?,
    };
    run_artifacts::invalidate_artifact_catalog_under_process_lock(app)?;
    if replace_existing {
        run_artifacts::publish_artifact_file_atomically(&path, &bytes)?;
    } else {
        run_artifacts::publish_new_artifact_file_atomically(&path, &bytes)?;
    }
    if !replace_existing {
        run_artifacts::prune_old_campaign_summary_artifacts_under_process_lock(
            app,
            run_artifacts::artifact_retention_policy_for_app(app).campaign_summary_keep_newest,
        );
    }
    Ok((path.to_string_lossy().to_string(), bytes.len()))
}

const MAX_INTERMEDIATE_FULL_SUMMARY_CHECKPOINTS: usize = 100;

fn campaign_checkpoint_stride(dataset_total: usize) -> usize {
    dataset_total
        .max(1)
        .div_ceil(MAX_INTERMEDIATE_FULL_SUMMARY_CHECKPOINTS)
}

fn should_persist_campaign_item_checkpoint(item_index: usize, dataset_total: usize) -> bool {
    let completed = item_index.saturating_add(1);
    completed == dataset_total
        || completed.is_multiple_of(campaign_checkpoint_stride(dataset_total))
}

#[cfg(test)]
fn planned_campaign_full_summary_builds(dataset_total: usize) -> usize {
    let intermediate = (0..dataset_total)
        .filter(|&index| should_persist_campaign_item_checkpoint(index, dataset_total))
        .count();
    // One immutable initial snapshot and one forced final snapshot bracket the
    // bounded item checkpoints.
    intermediate.saturating_add(2)
}

fn artifact_stub(run_req: &Value, validation_req: Option<&Value>) -> Value {
    json!({
        "run": { "req": run_req },
        "validation": validation_req.map(|req| json!({ "req": req })),
    })
}

fn normalized_campaign_artifact_tags(tags: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = tags
        .iter()
        .map(|tag| normalize_text(tag).to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn campaign_artifact_tags(
    campaign_tags: &[String],
    label: &str,
    is_holdout: bool,
    holdout_tag: Option<&str>,
) -> Vec<String> {
    let mut tags = campaign_tags.to_vec();
    tags.push(label.to_string());
    if is_holdout {
        if let Some(tag) = holdout_tag {
            tags.push(tag.to_string());
        }
    }
    dedup_tags(tags)
}

fn campaign_manifest_cohort_tag(manifest_hash: &str) -> String {
    format!(
        "campaign-cohort:{}",
        manifest_hash.trim().trim_start_matches("sha256:")
    )
}

fn campaign_artifact_config_fingerprint(
    base_config_fingerprint: &str,
    cohort_tags: &[String],
    is_holdout: bool,
    holdout_tag: Option<&str>,
) -> String {
    sha256_json_hash(&json!({
        "schema_version": 2,
        "base_config_fingerprint": base_config_fingerprint.trim(),
        "cohort_role": if is_holdout { "holdout" } else { "training" },
        "cohort_tag": is_holdout.then(|| holdout_tag.map(normalize_text).unwrap_or_default()),
        "cohort_tags": normalized_campaign_artifact_tags(cohort_tags),
    }))
}

fn prepared_artifact_source_identity(
    prepared: &PreparedCampaignDataset,
) -> Option<CampaignArtifactSourceIdentityKey> {
    if let Some(fingerprint) = prepared
        .dataset_meta
        .get("source_content_fingerprint")
        .and_then(Value::as_str)
        .map(normalize_text)
        .filter(|value| !value.is_empty())
    {
        return Some(CampaignArtifactSourceIdentityKey::ContentFingerprint(
            fingerprint,
        ));
    }
    prepared
        .dataset_meta
        .get("source_size_bytes")
        .and_then(Value::as_u64)
        .zip(
            prepared
                .dataset_meta
                .get("source_mtime_utc_ms")
                .and_then(Value::as_i64),
        )
        .map(|(size, mtime)| CampaignArtifactSourceIdentityKey::SizeAndMtime(size, mtime))
}

fn artifact_source_identities(
    row: &run_artifacts::RunArtifactSummary,
) -> Vec<CampaignArtifactSourceIdentityKey> {
    let mut identities = Vec::with_capacity(2);
    if let Some(fingerprint) = row
        .source_content_fingerprint
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
    {
        identities.push(CampaignArtifactSourceIdentityKey::ContentFingerprint(
            fingerprint,
        ));
    }
    if let (Some(size), Some(mtime)) = (row.source_size_bytes, row.source_mtime_utc_ms) {
        identities.push(CampaignArtifactSourceIdentityKey::SizeAndMtime(size, mtime));
    }
    identities
}

impl<'a> CampaignArtifactReuseIndex<'a> {
    fn new(artifacts: &'a [run_artifacts::RunArtifactSummary]) -> Self {
        let mut exact = HashMap::<CampaignArtifactReuseKey, Vec<usize>>::new();
        let mut coarse = HashMap::<CampaignArtifactReuseCoarseKey, Vec<usize>>::new();
        for (index, row) in artifacts.iter().enumerate() {
            let strategy_kind = normalize_strategy_kind(&row.strategy_kind);
            let config_fingerprint = row.config_fingerprint.trim().to_string();
            let csv_path = normalize_path_key(&row.csv_path);
            let timeframe = normalize_timeframe_label(&row.timeframe);
            let cohort_tags = normalized_campaign_artifact_tags(&row.tags);
            coarse
                .entry(CampaignArtifactReuseCoarseKey {
                    strategy_kind: strategy_kind.clone(),
                    config_fingerprint: config_fingerprint.clone(),
                    csv_path: csv_path.clone(),
                    cohort_tags: cohort_tags.clone(),
                })
                .or_default()
                .push(index);
            for source_identity in artifact_source_identities(row) {
                exact
                    .entry(CampaignArtifactReuseKey {
                        strategy_kind: strategy_kind.clone(),
                        config_fingerprint: config_fingerprint.clone(),
                        csv_path: csv_path.clone(),
                        timeframe: timeframe.clone(),
                        cohort_tags: cohort_tags.clone(),
                        source_identity,
                    })
                    .or_default()
                    .push(index);
            }
        }
        Self {
            artifacts,
            exact,
            coarse,
        }
    }

    fn exact_candidate_indices(
        &self,
        strategy_kind: &str,
        config_fingerprint: &str,
        expected_tags: &[String],
        prepared: &PreparedCampaignDataset,
    ) -> &[usize] {
        let Some(source_identity) = prepared_artifact_source_identity(prepared) else {
            return &[];
        };
        let key = CampaignArtifactReuseKey {
            strategy_kind: normalize_strategy_kind(strategy_kind),
            config_fingerprint: config_fingerprint.trim().to_string(),
            csv_path: normalize_path_key(&prepared.input.csv_path),
            timeframe: normalize_timeframe_label(&prepared.timeframe),
            cohort_tags: normalized_campaign_artifact_tags(expected_tags),
            source_identity,
        };
        self.exact.get(&key).map(Vec::as_slice).unwrap_or(&[])
    }

    fn coarse_candidate_indices(
        &self,
        strategy_kind: &str,
        config_fingerprint: &str,
        expected_tags: &[String],
        prepared: &PreparedCampaignDataset,
    ) -> &[usize] {
        let key = CampaignArtifactReuseCoarseKey {
            strategy_kind: normalize_strategy_kind(strategy_kind),
            config_fingerprint: config_fingerprint.trim().to_string(),
            csv_path: normalize_path_key(&prepared.input.csv_path),
            cohort_tags: normalized_campaign_artifact_tags(expected_tags),
        };
        self.coarse.get(&key).map(Vec::as_slice).unwrap_or(&[])
    }
}

fn classify_run_artifact_for_campaign(
    row: &run_artifacts::RunArtifactSummary,
    strategy_kind: &str,
    config_fingerprint: &str,
    expected_tags: &[String],
    prepared: &PreparedCampaignDataset,
) -> CampaignArtifactMatchDecision {
    let dataset_tf = normalize_timeframe_label(&prepared.timeframe);
    let dataset_path = normalize_path_key(&prepared.input.csv_path);
    let strict_fp = prepared
        .dataset_meta
        .get("source_content_fingerprint")
        .and_then(Value::as_str)
        .map(normalize_text)
        .filter(|value| !value.is_empty());
    let prepared_size = prepared
        .dataset_meta
        .get("source_size_bytes")
        .and_then(Value::as_u64);
    let prepared_mtime = prepared
        .dataset_meta
        .get("source_mtime_utc_ms")
        .and_then(Value::as_i64);
    if normalize_strategy_kind(&row.strategy_kind) != normalize_strategy_kind(strategy_kind) {
        return CampaignArtifactMatchDecision::Mismatch;
    }
    if row.config_fingerprint.trim() != config_fingerprint.trim() {
        return CampaignArtifactMatchDecision::Mismatch;
    }
    if normalized_campaign_artifact_tags(&row.tags)
        != normalized_campaign_artifact_tags(expected_tags)
    {
        return CampaignArtifactMatchDecision::Mismatch;
    }
    if normalize_path_key(&row.csv_path) != dataset_path {
        return CampaignArtifactMatchDecision::Mismatch;
    }
    if !dataset_tf.is_empty() && normalize_timeframe_label(&row.timeframe) != dataset_tf {
        return CampaignArtifactMatchDecision::Mismatch;
    }
    if let Some(expected_fp) = strict_fp.as_deref() {
        return match row
            .source_content_fingerprint
            .as_deref()
            .map(normalize_text)
        {
            Some(actual) if actual == expected_fp => CampaignArtifactMatchDecision::Exact,
            Some(_) => CampaignArtifactMatchDecision::Stale,
            None => CampaignArtifactMatchDecision::LegacyDegraded,
        };
    }
    if let (Some(prepared_size), Some(prepared_mtime)) = (prepared_size, prepared_mtime) {
        return match (row.source_size_bytes, row.source_mtime_utc_ms) {
            (Some(size), Some(mtime)) if prepared_size == size && prepared_mtime == mtime => {
                CampaignArtifactMatchDecision::Exact
            }
            (Some(_), Some(_)) => CampaignArtifactMatchDecision::Stale,
            _ => CampaignArtifactMatchDecision::LegacyDegraded,
        };
    }
    CampaignArtifactMatchDecision::Unsafe
}

fn push_reuse_decision_warning(
    warnings: &mut Vec<String>,
    decision: CampaignArtifactMatchDecision,
    row: &run_artifacts::RunArtifactSummary,
    prepared: &PreparedCampaignDataset,
) {
    if !decision.blocks_reuse() {
        return;
    }
    push_unique_warning(
        warnings,
        format!(
            "campaign artifact reuse rejected {} for '{}' because dataset identity classified as {}",
            row.path,
            prepared.input.csv_path,
            decision.as_str()
        ),
    );
}

fn existing_validation_match_indexed<'a>(
    existing: &CampaignArtifactReuseIndex<'a>,
    strategy_kind: &str,
    config_fingerprint: &str,
    expected_tags: &[String],
    prepared: &PreparedCampaignDataset,
    warnings: &mut Vec<String>,
) -> Option<CampaignExistingArtifactMatch<'a>> {
    for &index in
        existing.exact_candidate_indices(strategy_kind, config_fingerprint, expected_tags, prepared)
    {
        let row = &existing.artifacts[index];
        if !row.has_validation {
            continue;
        }
        let decision = classify_run_artifact_for_campaign(
            row,
            strategy_kind,
            config_fingerprint,
            expected_tags,
            prepared,
        );
        if decision == CampaignArtifactMatchDecision::Exact {
            return Some(CampaignExistingArtifactMatch {
                summary: row,
                decision,
            });
        }
    }
    for &index in existing.coarse_candidate_indices(
        strategy_kind,
        config_fingerprint,
        expected_tags,
        prepared,
    ) {
        let row = &existing.artifacts[index];
        if row.has_validation {
            let decision = classify_run_artifact_for_campaign(
                row,
                strategy_kind,
                config_fingerprint,
                expected_tags,
                prepared,
            );
            push_reuse_decision_warning(warnings, decision, row, prepared);
        }
    }
    None
}

fn existing_optimization_match_indexed<'a>(
    existing: &CampaignArtifactReuseIndex<'a>,
    strategy_kind: &str,
    config_fingerprint: &str,
    expected_tags: &[String],
    prepared: &PreparedCampaignDataset,
    warnings: &mut Vec<String>,
) -> Option<CampaignExistingArtifactMatch<'a>> {
    for &index in
        existing.exact_candidate_indices(strategy_kind, config_fingerprint, expected_tags, prepared)
    {
        let row = &existing.artifacts[index];
        let decision = classify_run_artifact_for_campaign(
            row,
            strategy_kind,
            config_fingerprint,
            expected_tags,
            prepared,
        );
        if decision == CampaignArtifactMatchDecision::Exact {
            return Some(CampaignExistingArtifactMatch {
                summary: row,
                decision,
            });
        }
    }
    for &index in existing.coarse_candidate_indices(
        strategy_kind,
        config_fingerprint,
        expected_tags,
        prepared,
    ) {
        let row = &existing.artifacts[index];
        let decision = classify_run_artifact_for_campaign(
            row,
            strategy_kind,
            config_fingerprint,
            expected_tags,
            prepared,
        );
        push_reuse_decision_warning(warnings, decision, row, prepared);
    }
    None
}

#[cfg(test)]
fn existing_validation_match<'a>(
    existing: &'a [run_artifacts::RunArtifactSummary],
    strategy_kind: &str,
    config_fingerprint: &str,
    primary_tag: &str,
    prepared: &PreparedCampaignDataset,
    warnings: &mut Vec<String>,
) -> Option<CampaignExistingArtifactMatch<'a>> {
    let index = CampaignArtifactReuseIndex::new(existing);
    existing_validation_match_indexed(
        &index,
        strategy_kind,
        config_fingerprint,
        &[primary_tag.to_string()],
        prepared,
        warnings,
    )
}

#[cfg(test)]
fn existing_optimization_match<'a>(
    existing: &'a [run_artifacts::RunArtifactSummary],
    strategy_kind: &str,
    config_fingerprint: &str,
    primary_tag: &str,
    prepared: &PreparedCampaignDataset,
    warnings: &mut Vec<String>,
) -> Option<CampaignExistingArtifactMatch<'a>> {
    let index = CampaignArtifactReuseIndex::new(existing);
    existing_optimization_match_indexed(
        &index,
        strategy_kind,
        config_fingerprint,
        &[primary_tag.to_string()],
        prepared,
        warnings,
    )
}

fn record_checkpoint_save_ms(
    items: &mut [ResearchCampaignItemSummary],
    summary: &mut ResearchCampaignSummary,
    item_index: usize,
    checkpoint_save_ms: f64,
) {
    if let Some(diagnostics) = items
        .get_mut(item_index)
        .and_then(|item| item.diagnostics.as_mut())
    {
        diagnostics.checkpoint_save_ms = Some(checkpoint_save_ms);
    }
    if let Some(diagnostics) = summary
        .items
        .get_mut(item_index)
        .and_then(|item| item.diagnostics.as_mut())
    {
        diagnostics.checkpoint_save_ms = Some(checkpoint_save_ms);
    }
}

fn save_campaign_summary_checkpoint_blocking<R: tauri::Runtime>(
    summary: &mut ResearchCampaignSummary,
    items: &mut [ResearchCampaignItemSummary],
    item_index: Option<usize>,
    app: &tauri::AppHandle<R>,
    existing_path: Option<&str>,
) -> Result<(String, usize), String> {
    let started = std::time::Instant::now();
    let saved = save_campaign_summary_blocking(summary, app, existing_path)?;
    let checkpoint_save_ms = started.elapsed().as_secs_f64() * 1000.0;
    if let Some(item_index) = item_index {
        let has_diagnostics = items
            .get(item_index)
            .and_then(|item| item.diagnostics.as_ref())
            .is_some();
        if has_diagnostics {
            record_checkpoint_save_ms(items, summary, item_index, checkpoint_save_ms);
        }
    }
    Ok(saved)
}

fn seal_vcet_campaign_artifact_hashes<R: tauri::Runtime>(
    items: &mut [ResearchCampaignItemSummary],
    app: &tauri::AppHandle<R>,
) -> Result<(), String> {
    let paths: Vec<PathBuf> = items
        .iter()
        .filter_map(|item| {
            item.artifact_path
                .as_deref()
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
        })
        .collect();
    let hashes = run_artifacts::hash_artifact_files_checked_bulk_blocking(&paths, app)
        .map_err(|error| format!("failed to seal VCET campaign run-artifact hashes: {error}"))?;
    let mut hash_by_path = HashMap::with_capacity(hashes.len());
    for (path, sha256) in hashes {
        let key = normalize_path_key(path.to_string_lossy().as_ref());
        if hash_by_path.insert(key, sha256).is_some() {
            return Err(
                "failed to seal VCET campaign run-artifact hashes: duplicate artifact path"
                    .to_string(),
            );
        }
    }
    for item in items {
        let Some(path) = item
            .artifact_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        else {
            item.artifact_sha256 = None;
            continue;
        };
        item.artifact_sha256 = Some(hash_by_path.remove(&normalize_path_key(path)).ok_or_else(
            || {
                format!(
                    "failed to seal VCET campaign run-artifact hash for '{}'",
                    item.label
                )
            },
        )?);
    }
    if !hash_by_path.is_empty() {
        return Err(
            "failed to seal VCET campaign run-artifact hashes: unclaimed artifact hash".to_string(),
        );
    }
    Ok(())
}

fn should_seal_vcet_campaign_artifacts(
    execution_mode: ResearchCampaignExecutionMode,
    cancelled: bool,
) -> bool {
    execution_mode == ResearchCampaignExecutionMode::EvaluateCandidates && !cancelled
}

fn can_reuse_optimization_artifact_for_validation(family: CampaignOptimizationFamily) -> bool {
    matches!(
        family,
        CampaignOptimizationFamily::DoubleLike
            | CampaignOptimizationFamily::CompositeCpu
            | CampaignOptimizationFamily::CompositeMo
    )
}

fn optimization_result_from_artifact_value(
    family: CampaignOptimizationFamily,
    artifact: &Value,
) -> Result<CampaignOptimizationResult, String> {
    let run_res = artifact
        .pointer("/run/res")
        .cloned()
        .ok_or_else(|| "existing optimization artifact was missing run.res".to_string())?;
    match family {
        CampaignOptimizationFamily::DoubleLike | CampaignOptimizationFamily::CompositeCpu => {
            let res: BackendOptimizationResult =
                serde_json::from_value(run_res).map_err(|e| e.to_string())?;
            Ok(CampaignOptimizationResult::Backend(res))
        }
        CampaignOptimizationFamily::CompositeMo => {
            let pareto = run_res
                .get("pareto")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    "existing CompositeMo optimization artifact was missing pareto candidates"
                        .to_string()
                })?;
            if pareto.is_empty() {
                return Err(
                    "existing CompositeMo optimization artifact had an empty pareto frontier"
                        .to_string(),
                );
            }
            Ok(CampaignOptimizationResult::CompositeMoArtifact(run_res))
        }
    }
}

fn optimization_request_from_artifact_value(artifact: &Value) -> Result<Value, String> {
    artifact
        .pointer("/run/req")
        .cloned()
        .ok_or_else(|| "existing optimization artifact was missing run.req".to_string())
}

fn build_run_artifact_value(
    ts: &str,
    config_fingerprint: &str,
    campaign_cohort_id: &str,
    is_holdout: bool,
    holdout_tag: Option<&str>,
    csv_path: &str,
    dataset_meta: &Value,
    tags: &[String],
    run_req: &Value,
    run_res: &Value,
    validation_req: Option<&Value>,
    validation_res: Option<&Value>,
) -> Value {
    json!({
        "version": 1,
        "kind": "vectorgrid_run_artifact",
        "ts": ts,
        "config_fingerprint": config_fingerprint,
        "run": { "req": run_req, "res": run_res },
        "validation": validation_req.zip(validation_res).map(|(req, res)| json!({ "req": req, "res": res })),
        "report_xlsx_path": "",
        "data": { "csv_path": csv_path },
        "dataset_meta": dataset_meta,
        "tags": tags,
        "retention": {
            "class": "campaign_cohort",
            "cohort_id": campaign_cohort_id,
        },
        "campaign": {
            "manifest_hash": campaign_cohort_id,
            "cohort_id": campaign_cohort_id,
            "cohort_tag": campaign_manifest_cohort_tag(campaign_cohort_id),
            "cohort_role": if is_holdout { "holdout" } else { "training" },
            "holdout_tag": holdout_tag,
        },
    })
}

fn save_campaign_run_artifact_blocking<R: tauri::Runtime>(
    artifact: Value,
    app: tauri::AppHandle<R>,
    existing_path: Option<String>,
    strategy_kind: &str,
    execution_mode: ResearchCampaignExecutionMode,
) -> Result<run_artifacts::SaveRunArtifactResponse, String> {
    if campaign_defers_run_artifact_pruning(strategy_kind, execution_mode) {
        run_artifacts::save_run_artifact_without_retention_prune_blocking(
            artifact,
            app,
            existing_path,
        )
    } else {
        run_artifacts::save_run_artifact_blocking(artifact, app, existing_path)
    }
}

fn latch_campaign_cancellation(cancel: &AtomicBool, counts: &mut ResearchCampaignCounts) {
    if cancel.load(Ordering::Relaxed) {
        counts.cancelled = true;
    }
}

fn run_research_campaign_blocking_with_mode(
    mut req: ResearchCampaignRequest,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
    execution_mode: ResearchCampaignExecutionMode,
    mut preloaded_pack: Option<HeadlessPreparedResearchCampaignDatasetPack>,
) -> Result<RunResearchCampaignResponse, String> {
    req.datasets = normalize_campaign_datasets(req.datasets)?;
    // Seal the O(1) filesystem identity into the request before the manifest and
    // cohort tag are derived. This keeps manifest identity source-bound even
    // when discovery did not supply hints, without adding a second full-file
    // content scan ahead of the normal dataset load.
    refresh_campaign_dataset_source_identities(&mut req.datasets)?;
    req.execution_mode = execution_mode;
    if execution_mode == ResearchCampaignExecutionMode::EvaluateCandidates {
        req.validation_config.fixed_candidates.sort_unstable();
    }
    let started = std::time::Instant::now();
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let app_cache_dir = data_cache::app_data_cache_dir(&base);
    let strategy_kind = resolve_campaign_strategy_kind(&req.strategy_kind)?;
    if req.datasets.is_empty() {
        return Err("campaign requires at least one dataset".to_string());
    }
    let fixed_candidate_plan =
        build_vcet_fixed_candidate_plan(&req, &strategy_kind, execution_mode)?;

    let family = optimization_family_for_kind(&strategy_kind);
    let ts = chrono::Utc::now().to_rfc3339();
    let mut tags = dedup_tags(req.tags.clone());
    if tags.is_empty() {
        tags.push(default_campaign_tag(&ts));
    }
    let primary_tag = tags[0].clone();
    let holdout_tag = req
        .holdout_tag
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
        .or_else(|| (req.holdout_pct > 0).then_some("holdout".to_string()));
    validate_holdout_tag_isolation(
        &tags,
        &req.datasets,
        req.holdout_pct,
        holdout_tag.as_deref(),
    )?;
    let timeframe_override = req
        .timeframe_override
        .as_deref()
        .map(normalize_timeframe_label)
        .filter(|value| !value.is_empty());

    let dataset_paths: Vec<String> = req
        .datasets
        .iter()
        .map(|entry| normalize_text(&entry.csv_path))
        .filter(|entry| !entry.is_empty())
        .collect();
    if dataset_paths.is_empty() {
        return Err("campaign datasets resolved to zero csv_path entries".to_string());
    }

    let (_train_paths, holdout_paths) =
        split_holdout_paths(&dataset_paths, req.holdout_pct, &primary_tag);
    let mut counts = ResearchCampaignCounts {
        total: dataset_paths.len(),
        optimized: 0,
        validated: 0,
        skipped: 0,
        failed: 0,
        cancelled: false,
    };
    let mut warnings = Vec::new();
    if req.preload_policy == ResearchCampaignPreloadPolicy::HostPlusDoublemaVram
        && strategy_kind == "DoubleMa"
    {
        warnings.push(
            "host_plus_doublema_vram is experimental, limited to the plain DoubleMa exact GpuKernel route, and falls back to host preload when GPU staging is unavailable."
                .to_string(),
        );
    }
    let initial_manifest = build_campaign_manifest(
        &req,
        &strategy_kind,
        timeframe_override.as_deref(),
        &tags,
        holdout_tag.as_deref(),
        execution_mode,
    );
    if !initial_manifest.strict_identity_required {
        push_unique_warning(
            &mut warnings,
            "campaign summary manifest is missing strict source identity for at least one dataset; exact artifact reuse will fail closed unless the current prepared dataset and saved artifact both expose matching source identity."
                .to_string(),
        );
    }
    let manifest_hash = initial_manifest.manifest_hash.clone();
    let campaign_cohort_tag = campaign_manifest_cohort_tag(&manifest_hash);

    let existing = load_existing_campaign_artifacts(req.skip_existing, execution_mode, || {
        run_artifacts::list_run_artifacts_blocking(app.clone())
    })?;
    let existing_reuse_index = CampaignArtifactReuseIndex::new(&existing);
    let mut deferred_run_artifact_prune = CampaignRunArtifactPruneGuard::new(
        campaign_defers_run_artifact_pruning(&strategy_kind, execution_mode).then(|| app.clone()),
    );

    let dataset_total = req.datasets.len();
    let using_preloaded_pack = preloaded_pack.is_some();
    if let Some(pack) = preloaded_pack.as_ref() {
        if pack.datasets.len() != req.datasets.len() {
            return Err(format!(
                "preloaded campaign dataset pack length {} did not match request dataset length {}",
                pack.datasets.len(),
                req.datasets.len()
            ));
        }
    }
    if using_preloaded_pack {
        warnings.push(
            "headless benchmark supplied a preloaded dataset pack; request.preload_policy remains unchanged because full_pack_host is benchmark-only."
                .to_string(),
        );
    }
    let mut preload = if !using_preloaded_pack && preload_enabled(req.preload_policy) {
        Some(DatasetPreloadJob::spawn(
            req.datasets[0].clone(),
            timeframe_override.clone(),
            app_cache_dir.clone(),
        ))
    } else {
        None
    };
    let sink = TauriProgressSink { app: app.clone() };
    let mut learned_doublema_vram_device: Option<u32> = None;
    let mut doublema_vram_prefetch_disabled = false;
    let mut seen_prepared_source_fingerprints: HashMap<String, String> =
        HashMap::with_capacity(dataset_total);
    let mut items: Vec<ResearchCampaignItemSummary> = req
        .datasets
        .iter()
        .enumerate()
        .map(|(index, dataset)| {
            let csv_path = normalize_text(&dataset.csv_path);
            let timeframe = dataset_timeframe_hint(dataset, timeframe_override.as_deref());
            ResearchCampaignItemSummary {
                item_id: Some(campaign_item_id(
                    &manifest_hash,
                    index,
                    &csv_path,
                    &timeframe,
                )),
                is_holdout: holdout_paths.contains(&csv_path),
                csv_path,
                label: dataset_label_from_input(dataset),
                timeframe,
                status: "pending".to_string(),
                dataset_identity: None,
                evidence_state: Some(CampaignEvidenceState::None),
                resume_state: Some(CampaignResumeState::Resumable),
                reuse_decision: None,
                artifact_path: None,
                artifact_sha256: None,
                report_xlsx_path: None,
                error: None,
                diagnostics: None,
                fixed_candidate_result: None,
            }
        })
        .collect();
    let mut summary = build_campaign_summary(
        &ts,
        "running",
        &strategy_kind,
        timeframe_override.clone(),
        &req,
        execution_mode,
        &tags,
        holdout_tag.clone(),
        &counts,
        &items,
        None,
        None,
        &warnings,
        started.elapsed().as_millis() as u64,
    );
    let (summary_json_path, _) =
        save_campaign_summary_checkpoint_blocking(&mut summary, &mut items, None, &app, None)?;
    // Full summaries contain the immutable request/manifest plus every item and,
    // for VCET replay, cross-dataset aggregates. Rebuild and serialize them at a
    // bounded cadence so a 10k-dataset campaign performs at most 100 intermediate
    // full snapshots. A crash can lose at most stride-1 item checkpoints; each
    // already-published run artifact remains reusable on resume.
    macro_rules! persist_bounded_item_checkpoint {
        ($item_index:expr) => {{
            let item_index = $item_index;
            if should_persist_campaign_item_checkpoint(item_index, dataset_total) {
                summary = build_campaign_summary(
                    &ts,
                    "running",
                    &strategy_kind,
                    timeframe_override.clone(),
                    &req,
                    execution_mode,
                    &tags,
                    holdout_tag.clone(),
                    &counts,
                    &items,
                    None,
                    None,
                    &warnings,
                    started.elapsed().as_millis() as u64,
                );
                let _ = save_campaign_summary_checkpoint_blocking(
                    &mut summary,
                    &mut items,
                    Some(item_index),
                    &app,
                    Some(&summary_json_path),
                )?;
            }
        }};
    }
    emit_campaign_progress(
        &app,
        "checkpoint",
        0,
        dataset_total,
        &strategy_kind,
        "campaign initialized",
        Some(&summary_json_path),
        &counts,
    );

    for (index, dataset) in req.datasets.iter().cloned().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            counts.cancelled = true;
            break;
        }

        let dataset_csv_path = normalize_text(&dataset.csv_path);
        let is_holdout = holdout_paths.contains(&dataset_csv_path);
        let fallback_label = dataset_label_from_input(&dataset);
        let fallback_timeframe = dataset_timeframe_hint(&dataset, timeframe_override.as_deref());
        let item_id = items
            .get(index)
            .and_then(|item| item.item_id.clone())
            .or_else(|| {
                Some(campaign_item_id(
                    &manifest_hash,
                    index,
                    &dataset_csv_path,
                    &fallback_timeframe,
                ))
            });
        let (prepared_result, prepare_source, prepare_wait_ms) = match preloaded_pack.as_mut() {
            Some(pack) => {
                let prepare_started = std::time::Instant::now();
                (
                    pack.datasets.pop_front().ok_or_else(|| {
                        "preloaded campaign dataset pack was missing an entry".to_string()
                    }),
                    DatasetPrepareSource::PreloadedPack,
                    prepare_started.elapsed().as_secs_f64() * 1000.0,
                )
            }
            None => match preload.take() {
                Some(job) => {
                    let (result, wait_ms, cached_ready) = job.join_with_prepare_metrics();
                    (
                        result,
                        if cached_ready {
                            DatasetPrepareSource::BackgroundPreloadReady
                        } else {
                            DatasetPrepareSource::BackgroundPreloadWaited
                        },
                        wait_ms,
                    )
                }
                None => {
                    let prepare_started = std::time::Instant::now();
                    (
                        prepare_campaign_dataset(
                            dataset.clone(),
                            timeframe_override.as_deref(),
                            &app_cache_dir,
                        ),
                        DatasetPrepareSource::SyncPrepare,
                        prepare_started.elapsed().as_secs_f64() * 1000.0,
                    )
                }
            },
        };
        if !using_preloaded_pack
            && preload_enabled(req.preload_policy)
            && index + 1 < req.datasets.len()
        {
            preload = Some(DatasetPreloadJob::spawn(
                req.datasets[index + 1].clone(),
                timeframe_override.clone(),
                app_cache_dir.clone(),
            ));
        }
        let prepared = match prepared_result {
            Ok(prepared) => prepared,
            Err(error) => {
                counts.failed = counts.failed.saturating_add(1);
                items[index] = failed_item(
                    dataset_csv_path.clone(),
                    fallback_label.clone(),
                    fallback_timeframe.clone(),
                    is_holdout,
                    None,
                    "prepare",
                    error,
                )
                .with_campaign_metadata(
                    item_id.clone(),
                    None,
                    Some(CampaignEvidenceState::Partial),
                    Some(CampaignResumeState::Resumable),
                    None,
                );
                persist_bounded_item_checkpoint!(index);
                continue;
            }
        };
        let mut prefetch_diagnostics = CampaignPrefetchDiagnostics::not_attempted();
        let mut optimize_wall_ms = None;
        let mut optimize_artifact_save_ms = None;
        if using_preloaded_pack {
            let expected_csv_path = normalize_text(&dataset.csv_path);
            let prepared_csv_path = normalize_text(&prepared.input.csv_path);
            if prepared_csv_path != expected_csv_path {
                return Err(format!(
                    "preloaded campaign dataset pack order mismatch at dataset {}: expected '{}' but got '{}'",
                    index + 1,
                    expected_csv_path,
                    prepared_csv_path
                ));
            }
            let expected_timeframe =
                dataset_timeframe_hint(&dataset, timeframe_override.as_deref());
            if prepared.timeframe != expected_timeframe {
                return Err(format!(
                    "preloaded campaign dataset pack timeframe mismatch for '{}': expected '{}' but got '{}'",
                    expected_csv_path,
                    expected_timeframe,
                    prepared.timeframe
                ));
            }
        }

        record_unique_prepared_campaign_source(&prepared, &mut seen_prepared_source_fingerprints)?;

        let label = prepared.label.clone();
        let prepared_identity = Some(dataset_identity_summary(&prepared));
        emit_campaign_progress(
            &app,
            "prepare",
            index + 1,
            dataset_total,
            &label,
            "dataset ready",
            Some(&summary_json_path),
            &counts,
        );

        let run_ts = chrono::Utc::now().to_rfc3339();
        let data_id = make_campaign_data_id(index, &prepared.input.csv_path, &prepared.timeframe);
        let mut run_req_value =
            patch_optimization_template(&req.optimization_template, &strategy_kind, &data_id)?;
        let parsed_doublema_req = if strategy_kind == "DoubleMa" {
            serde_json::from_value::<DoubleMaRequest>(run_req_value.clone()).ok()
        } else {
            None
        };
        let mut per_run_tags =
            campaign_artifact_tags(&tags, &label, is_holdout, holdout_tag.as_deref());
        per_run_tags.push(campaign_cohort_tag.clone());
        let per_run_tags = dedup_tags(per_run_tags);
        let run_only_base_fingerprint = run_artifacts::compute_config_fingerprint_from_artifact(
            &artifact_stub(&run_req_value, None),
        );
        let run_only_fingerprint = campaign_artifact_config_fingerprint(
            &run_only_base_fingerprint,
            &per_run_tags,
            is_holdout,
            holdout_tag.as_deref(),
        );

        if let Some(plan) = fixed_candidate_plan.as_ref() {
            let validation_req_value = build_vcet_fixed_candidate_validation_request_value(
                &data_id,
                &run_req_value,
                &prepared.dataset_meta,
                &req.validation_config,
                plan,
            );
            let validation_base_fingerprint =
                run_artifacts::compute_config_fingerprint_from_artifact(&artifact_stub(
                    &run_req_value,
                    Some(&validation_req_value),
                ));
            let validation_fingerprint = campaign_artifact_config_fingerprint(
                &validation_base_fingerprint,
                &per_run_tags,
                is_holdout,
                holdout_tag.as_deref(),
            );
            if req.skip_existing {
                if let Some(existing_validation) = existing_validation_match_indexed(
                    &existing_reuse_index,
                    &strategy_kind,
                    &validation_fingerprint,
                    &per_run_tags,
                    &prepared,
                    &mut warnings,
                ) {
                    let existing_path = existing_validation.summary.path.clone();
                    let reuse = evaluate_vcet_fixed_candidate_artifact_reuse(
                        &existing_path,
                        run_artifacts::load_run_artifact_blocking(
                            existing_path.clone(),
                            app.clone(),
                        ),
                        plan,
                        req.validation_config.require_gpu_kernel,
                    );
                    match reuse {
                        CampaignFixedCandidateArtifactReuse::Reuse(fixed_candidate_result) => {
                            counts.skipped = counts.skipped.saturating_add(1);
                            items[index] = ResearchCampaignItemSummary {
                                item_id: item_id.clone(),
                                csv_path: prepared.input.csv_path.clone(),
                                label,
                                timeframe: prepared.timeframe.clone(),
                                is_holdout,
                                status: "skipped".to_string(),
                                dataset_identity: prepared_identity.clone(),
                                evidence_state: Some(CampaignEvidenceState::Complete),
                                resume_state: Some(CampaignResumeState::Complete),
                                reuse_decision: Some(format!(
                                    "{}_fixed_candidate_validation_artifact",
                                    existing_validation.decision.as_str()
                                )),
                                artifact_path: Some(existing_path),
                                artifact_sha256: None,
                                report_xlsx_path: None,
                                error: None,
                                diagnostics: Some(build_item_diagnostics(
                                    prepare_source,
                                    prepare_wait_ms,
                                    &prefetch_diagnostics,
                                    None,
                                    None,
                                    None,
                                )),
                                fixed_candidate_result: Some(fixed_candidate_result),
                            };
                            persist_bounded_item_checkpoint!(index);
                            continue;
                        }
                        CampaignFixedCandidateArtifactReuse::Rerun { warning } => {
                            push_unique_warning(&mut warnings, warning);
                        }
                    }
                }
            }

            emit_campaign_progress(
                &app,
                "validate",
                index + 1,
                dataset_total,
                &prepared.label,
                "evaluating fixed VCET candidate manifest",
                Some(&summary_json_path),
                &counts,
            );
            let validation_req: CompositeValidationRequest =
                match serde_json::from_value(validation_req_value.clone()) {
                    Ok(value) => value,
                    Err(error) => {
                        counts.failed = counts.failed.saturating_add(1);
                        items[index] = failed_item(
                            prepared.input.csv_path.clone(),
                            prepared.label.clone(),
                            prepared.timeframe.clone(),
                            is_holdout,
                            None,
                            "fixed-candidate validation request decode",
                            error.to_string(),
                        )
                        .with_campaign_metadata(
                            item_id.clone(),
                            prepared_identity.clone(),
                            Some(CampaignEvidenceState::Partial),
                            Some(CampaignResumeState::Resumable),
                            None,
                        );
                        persist_bounded_item_checkpoint!(index);
                        continue;
                    }
                };
            let validation_res_value =
                match ta_app_core::run_composite_validation_blocking_with_candles(
                    validation_req,
                    &prepared.candles,
                    cancel.as_ref(),
                    Some(&sink),
                ) {
                    Ok(result) => match serde_json::to_value(result) {
                        Ok(value) => value,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                prepared.label.clone(),
                                prepared.timeframe.clone(),
                                is_holdout,
                                None,
                                "fixed-candidate validation encode",
                                error.to_string(),
                            );
                            persist_bounded_item_checkpoint!(index);
                            continue;
                        }
                    },
                    Err(error) => {
                        counts.failed = counts.failed.saturating_add(1);
                        items[index] = failed_item(
                            prepared.input.csv_path.clone(),
                            prepared.label.clone(),
                            prepared.timeframe.clone(),
                            is_holdout,
                            None,
                            "fixed-candidate validation",
                            error,
                        );
                        persist_bounded_item_checkpoint!(index);
                        continue;
                    }
                };
            let fixed_candidate_result = match parse_vcet_fixed_candidate_dataset_result(
                &validation_res_value,
                plan,
                req.validation_config.require_gpu_kernel,
            ) {
                Ok(value) => value,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        prepared.label.clone(),
                        prepared.timeframe.clone(),
                        is_holdout,
                        None,
                        "fixed-candidate route/result policy",
                        error,
                    );
                    persist_bounded_item_checkpoint!(index);
                    continue;
                }
            };
            let run_res_value = vcet_fixed_candidate_run_result_value(plan);
            let validation_artifact = build_run_artifact_value(
                &run_ts,
                &validation_fingerprint,
                &manifest_hash,
                is_holdout,
                holdout_tag.as_deref(),
                &prepared.input.csv_path,
                &prepared.dataset_meta,
                &per_run_tags,
                &run_req_value,
                &run_res_value,
                Some(&validation_req_value),
                Some(&validation_res_value),
            );
            let artifact_path = match save_campaign_run_artifact_blocking(
                validation_artifact,
                app.clone(),
                None,
                &strategy_kind,
                execution_mode,
            ) {
                Ok(saved) => saved.path,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        prepared.label.clone(),
                        prepared.timeframe.clone(),
                        is_holdout,
                        None,
                        "fixed-candidate validation artifact",
                        error,
                    );
                    persist_bounded_item_checkpoint!(index);
                    continue;
                }
            };
            counts.validated = counts.validated.saturating_add(1);
            items[index] = ResearchCampaignItemSummary {
                item_id: item_id.clone(),
                csv_path: prepared.input.csv_path.clone(),
                label,
                timeframe: prepared.timeframe.clone(),
                is_holdout,
                status: "ok".to_string(),
                dataset_identity: prepared_identity.clone(),
                evidence_state: Some(CampaignEvidenceState::Complete),
                resume_state: Some(CampaignResumeState::Complete),
                reuse_decision: None,
                artifact_path: Some(artifact_path),
                artifact_sha256: None,
                report_xlsx_path: None,
                error: None,
                diagnostics: Some(build_item_diagnostics(
                    prepare_source,
                    prepare_wait_ms,
                    &prefetch_diagnostics,
                    None,
                    None,
                    None,
                )),
                fixed_candidate_result: Some(fixed_candidate_result),
            };
            persist_bounded_item_checkpoint!(index);
            continue;
        }

        let mut existing_optimize_path: Option<String> = None;
        let optimize_result = if execution_mode == ResearchCampaignExecutionMode::ValidateOnly {
            let Some(existing_run) = existing_optimization_match_indexed(
                &existing_reuse_index,
                &strategy_kind,
                &run_only_fingerprint,
                &per_run_tags,
                &prepared,
                &mut warnings,
            ) else {
                counts.skipped = counts.skipped.saturating_add(1);
                items[index] = ResearchCampaignItemSummary {
                    item_id: item_id.clone(),
                    csv_path: prepared.input.csv_path.clone(),
                    label,
                    timeframe: prepared.timeframe.clone(),
                    is_holdout,
                    status: "skipped".to_string(),
                    dataset_identity: prepared_identity.clone(),
                    evidence_state: Some(CampaignEvidenceState::None),
                    resume_state: Some(CampaignResumeState::RestartOnly),
                    reuse_decision: Some("no_exact_optimization_artifact".to_string()),
                    artifact_path: None,
                    artifact_sha256: None,
                    report_xlsx_path: None,
                    error: Some(
                        "no matching optimization artifact was found for validate_only mode"
                            .to_string(),
                    ),
                    diagnostics: Some(build_item_diagnostics(
                        prepare_source,
                        prepare_wait_ms,
                        &prefetch_diagnostics,
                        None,
                        None,
                        None,
                    )),
                    fixed_candidate_result: None,
                };
                persist_bounded_item_checkpoint!(index);
                continue;
            };
            existing_optimize_path = Some(existing_run.summary.path.clone());
            emit_campaign_progress(
                &app,
                "optimize",
                index + 1,
                dataset_total,
                &label,
                "loading existing optimization artifact for validation",
                Some(&summary_json_path),
                &counts,
            );
            let artifact = match run_artifacts::load_run_artifact_blocking(
                existing_run.summary.path.clone(),
                app.clone(),
            ) {
                Ok(value) => value,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        label,
                        prepared.timeframe.clone(),
                        is_holdout,
                        existing_optimize_path.clone(),
                        "load optimization artifact",
                        error,
                    );
                    persist_bounded_item_checkpoint!(index);
                    continue;
                }
            };
            run_req_value = match optimization_request_from_artifact_value(&artifact) {
                Ok(value) => value,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        label,
                        prepared.timeframe.clone(),
                        is_holdout,
                        existing_optimize_path.clone(),
                        "decode optimization request",
                        error,
                    );
                    persist_bounded_item_checkpoint!(index);
                    continue;
                }
            };
            match optimization_result_from_artifact_value(family, &artifact) {
                Ok(value) => value,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        label,
                        prepared.timeframe.clone(),
                        is_holdout,
                        existing_optimize_path.clone(),
                        "decode optimization artifact",
                        error,
                    );
                    persist_bounded_item_checkpoint!(index);
                    continue;
                }
            }
        } else if req.skip_existing {
            if let Some(existing_run) = existing_optimization_match_indexed(
                &existing_reuse_index,
                &strategy_kind,
                &run_only_fingerprint,
                &per_run_tags,
                &prepared,
                &mut warnings,
            ) {
                if execution_mode == ResearchCampaignExecutionMode::OptimizeOnly {
                    existing_optimize_path = Some(existing_run.summary.path.clone());
                    counts.skipped = counts.skipped.saturating_add(1);
                    items[index] = ResearchCampaignItemSummary {
                        item_id: item_id.clone(),
                        csv_path: prepared.input.csv_path.clone(),
                        label,
                        timeframe: prepared.timeframe.clone(),
                        is_holdout,
                        status: "skipped".to_string(),
                        dataset_identity: prepared_identity.clone(),
                        evidence_state: Some(CampaignEvidenceState::Complete),
                        resume_state: Some(CampaignResumeState::Complete),
                        reuse_decision: Some(format!(
                            "{}_optimization_artifact",
                            existing_run.decision.as_str()
                        )),
                        artifact_path: existing_optimize_path.clone(),
                        artifact_sha256: None,
                        report_xlsx_path: None,
                        error: None,
                        diagnostics: Some(build_item_diagnostics(
                            prepare_source,
                            prepare_wait_ms,
                            &prefetch_diagnostics,
                            None,
                            None,
                            None,
                        )),
                        fixed_candidate_result: None,
                    };
                    persist_bounded_item_checkpoint!(index);
                    continue;
                }

                if !can_reuse_optimization_artifact_for_validation(family) {
                    push_unique_warning(
                        &mut warnings,
                        "skip_existing found a matching CompositeMo optimization artifact, but full campaign reuse still reruns optimization because saved CompositeMo results are not deserializable for downstream validation yet."
                            .to_string(),
                    );
                    emit_campaign_progress(
                        &app,
                        "optimize",
                        index + 1,
                        dataset_total,
                        &label,
                        "existing optimization artifact found, rerunning optimization",
                        Some(&summary_json_path),
                        &counts,
                    );
                    let optimize_progress_sink = if req.preload_policy
                        == ResearchCampaignPreloadPolicy::HostPlusDoublemaVram
                        && strategy_kind == "DoubleMa"
                        && !doublema_vram_prefetch_disabled
                        && !using_preloaded_pack
                    {
                        match (
                            parsed_doublema_req.as_ref().and_then(|run_req| {
                                build_doublema_vram_prefetch_config(
                                    run_req,
                                    learned_doublema_vram_device,
                                )
                            }),
                            preload.as_ref().map(DatasetPreloadJob::shared_state),
                        ) {
                            (Some(config), Some(preload_state)) => Some(CampaignProgressSink {
                                app: app.clone(),
                                vram_prefetch: Some(CampaignDoubleMaVramPrefetch {
                                    preload_state,
                                    config,
                                    completed: AtomicBool::new(false),
                                    feedback: Mutex::new(None),
                                }),
                            }),
                            _ => Some(CampaignProgressSink {
                                app: app.clone(),
                                vram_prefetch: None,
                            }),
                        }
                    } else {
                        Some(CampaignProgressSink {
                            app: app.clone(),
                            vram_prefetch: None,
                        })
                    };
                    let optimize_started = std::time::Instant::now();
                    let optimize_result = match run_optimization_blocking(
                        family,
                        &run_req_value,
                        &prepared.candles,
                        cancel.as_ref(),
                        optimize_progress_sink
                            .as_ref()
                            .map(|value| value as &dyn ProgressSink),
                    ) {
                        Ok(value) => value,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                label,
                                prepared.timeframe.clone(),
                                is_holdout,
                                None,
                                "optimization",
                                error,
                            );
                            persist_bounded_item_checkpoint!(index);
                            continue;
                        }
                    };
                    optimize_wall_ms = Some(optimize_started.elapsed().as_secs_f64() * 1000.0);
                    prefetch_diagnostics = record_prefetch_feedback(
                        &mut warnings,
                        &mut doublema_vram_prefetch_disabled,
                        optimize_progress_sink
                            .as_ref()
                            .and_then(CampaignProgressSink::take_prefetch_feedback),
                    );
                    counts.optimized = counts.optimized.saturating_add(1);
                    optimize_result
                } else {
                    existing_optimize_path = Some(existing_run.summary.path.clone());
                    emit_campaign_progress(
                        &app,
                        "optimize",
                        index + 1,
                        dataset_total,
                        &label,
                        "reusing optimization artifact",
                        Some(&summary_json_path),
                        &counts,
                    );
                    let artifact = match run_artifacts::load_run_artifact_blocking(
                        existing_run.summary.path.clone(),
                        app.clone(),
                    ) {
                        Ok(value) => value,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                label,
                                prepared.timeframe.clone(),
                                is_holdout,
                                existing_optimize_path.clone(),
                                "load optimization artifact",
                                error,
                            );
                            persist_bounded_item_checkpoint!(index);
                            continue;
                        }
                    };
                    run_req_value = match optimization_request_from_artifact_value(&artifact) {
                        Ok(value) => value,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                label,
                                prepared.timeframe.clone(),
                                is_holdout,
                                existing_optimize_path.clone(),
                                "decode optimization request",
                                error,
                            );
                            persist_bounded_item_checkpoint!(index);
                            continue;
                        }
                    };
                    match optimization_result_from_artifact_value(family, &artifact) {
                        Ok(value) => value,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                label,
                                prepared.timeframe.clone(),
                                is_holdout,
                                existing_optimize_path.clone(),
                                "decode optimization artifact",
                                error,
                            );
                            persist_bounded_item_checkpoint!(index);
                            continue;
                        }
                    }
                }
            } else {
                emit_campaign_progress(
                    &app,
                    "optimize",
                    index + 1,
                    dataset_total,
                    &label,
                    "running optimization",
                    Some(&summary_json_path),
                    &counts,
                );
                let optimize_progress_sink = if req.preload_policy
                    == ResearchCampaignPreloadPolicy::HostPlusDoublemaVram
                    && strategy_kind == "DoubleMa"
                    && !doublema_vram_prefetch_disabled
                    && !using_preloaded_pack
                {
                    match (
                        parsed_doublema_req.as_ref().and_then(|run_req| {
                            build_doublema_vram_prefetch_config(
                                run_req,
                                learned_doublema_vram_device,
                            )
                        }),
                        preload.as_ref().map(DatasetPreloadJob::shared_state),
                    ) {
                        (Some(config), Some(preload_state)) => Some(CampaignProgressSink {
                            app: app.clone(),
                            vram_prefetch: Some(CampaignDoubleMaVramPrefetch {
                                preload_state,
                                config,
                                completed: AtomicBool::new(false),
                                feedback: Mutex::new(None),
                            }),
                        }),
                        _ => Some(CampaignProgressSink {
                            app: app.clone(),
                            vram_prefetch: None,
                        }),
                    }
                } else {
                    Some(CampaignProgressSink {
                        app: app.clone(),
                        vram_prefetch: None,
                    })
                };
                let optimize_started = std::time::Instant::now();
                let optimize_result = match run_optimization_blocking(
                    family,
                    &run_req_value,
                    &prepared.candles,
                    cancel.as_ref(),
                    optimize_progress_sink
                        .as_ref()
                        .map(|value| value as &dyn ProgressSink),
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        counts.failed = counts.failed.saturating_add(1);
                        items[index] = failed_item(
                            prepared.input.csv_path.clone(),
                            label,
                            prepared.timeframe.clone(),
                            is_holdout,
                            None,
                            "optimization",
                            error,
                        );
                        persist_bounded_item_checkpoint!(index);
                        continue;
                    }
                };
                optimize_wall_ms = Some(optimize_started.elapsed().as_secs_f64() * 1000.0);
                prefetch_diagnostics = record_prefetch_feedback(
                    &mut warnings,
                    &mut doublema_vram_prefetch_disabled,
                    optimize_progress_sink
                        .as_ref()
                        .and_then(CampaignProgressSink::take_prefetch_feedback),
                );
                counts.optimized = counts.optimized.saturating_add(1);
                optimize_result
            }
        } else {
            emit_campaign_progress(
                &app,
                "optimize",
                index + 1,
                dataset_total,
                &label,
                "running optimization",
                Some(&summary_json_path),
                &counts,
            );
            let optimize_progress_sink = if req.preload_policy
                == ResearchCampaignPreloadPolicy::HostPlusDoublemaVram
                && strategy_kind == "DoubleMa"
                && !doublema_vram_prefetch_disabled
                && !using_preloaded_pack
            {
                match (
                    parsed_doublema_req.as_ref().and_then(|run_req| {
                        build_doublema_vram_prefetch_config(run_req, learned_doublema_vram_device)
                    }),
                    preload.as_ref().map(DatasetPreloadJob::shared_state),
                ) {
                    (Some(config), Some(preload_state)) => Some(CampaignProgressSink {
                        app: app.clone(),
                        vram_prefetch: Some(CampaignDoubleMaVramPrefetch {
                            preload_state,
                            config,
                            completed: AtomicBool::new(false),
                            feedback: Mutex::new(None),
                        }),
                    }),
                    _ => Some(CampaignProgressSink {
                        app: app.clone(),
                        vram_prefetch: None,
                    }),
                }
            } else {
                Some(CampaignProgressSink {
                    app: app.clone(),
                    vram_prefetch: None,
                })
            };
            let optimize_started = std::time::Instant::now();
            let optimize_result = match run_optimization_blocking(
                family,
                &run_req_value,
                &prepared.candles,
                cancel.as_ref(),
                optimize_progress_sink
                    .as_ref()
                    .map(|value| value as &dyn ProgressSink),
            ) {
                Ok(value) => value,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        label,
                        prepared.timeframe.clone(),
                        is_holdout,
                        None,
                        "optimization",
                        error,
                    );
                    persist_bounded_item_checkpoint!(index);
                    continue;
                }
            };
            optimize_wall_ms = Some(optimize_started.elapsed().as_secs_f64() * 1000.0);
            prefetch_diagnostics = record_prefetch_feedback(
                &mut warnings,
                &mut doublema_vram_prefetch_disabled,
                optimize_progress_sink
                    .as_ref()
                    .and_then(CampaignProgressSink::take_prefetch_feedback),
            );
            counts.optimized = counts.optimized.saturating_add(1);
            optimize_result
        };
        if strategy_kind == "DoubleMa" {
            learned_doublema_vram_device = match (&optimize_result, parsed_doublema_req.as_ref()) {
                (CampaignOptimizationResult::Backend(res), Some(run_req))
                    if supports_exact_gpu_kernel_route(res) =>
                {
                    requested_gpu_device_id(&run_req.backend)
                }
                _ => None,
            };
        }
        let run_res_value = match &optimize_result {
            CampaignOptimizationResult::Backend(res) => {
                serde_json::to_value(res).map_err(|e| e.to_string())?
            }
            CampaignOptimizationResult::CompositeMo(res) => {
                serde_json::to_value(res).map_err(|e| e.to_string())?
            }
            CampaignOptimizationResult::CompositeMoArtifact(res) => res.clone(),
        };

        let reused_optimization_artifact = existing_optimize_path.is_some();
        let mut artifact_path = if let Some(path) = existing_optimize_path.clone() {
            path
        } else {
            let optimize_artifact = build_run_artifact_value(
                &run_ts,
                &run_only_fingerprint,
                &manifest_hash,
                is_holdout,
                holdout_tag.as_deref(),
                &prepared.input.csv_path,
                &prepared.dataset_meta,
                &per_run_tags,
                &run_req_value,
                &run_res_value,
                None,
                None,
            );
            let save_started = std::time::Instant::now();
            match save_campaign_run_artifact_blocking(
                optimize_artifact,
                app.clone(),
                None,
                &strategy_kind,
                execution_mode,
            ) {
                Ok(saved) => {
                    optimize_artifact_save_ms = Some(save_started.elapsed().as_secs_f64() * 1000.0);
                    saved.path
                }
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        prepared.label.clone(),
                        prepared.timeframe.clone(),
                        is_holdout,
                        None,
                        "optimization artifact",
                        error,
                    );
                    persist_bounded_item_checkpoint!(index);
                    continue;
                }
            }
        };

        if execution_mode == ResearchCampaignExecutionMode::OptimizeOnly {
            items[index] = ResearchCampaignItemSummary {
                item_id: item_id.clone(),
                csv_path: prepared.input.csv_path.clone(),
                label,
                timeframe: prepared.timeframe.clone(),
                is_holdout,
                status: "optimized".to_string(),
                dataset_identity: prepared_identity.clone(),
                evidence_state: Some(CampaignEvidenceState::Complete),
                resume_state: Some(CampaignResumeState::Complete),
                reuse_decision: None,
                artifact_path: Some(artifact_path),
                artifact_sha256: None,
                report_xlsx_path: None,
                error: None,
                diagnostics: Some(build_item_diagnostics(
                    prepare_source,
                    prepare_wait_ms,
                    &prefetch_diagnostics,
                    optimize_wall_ms,
                    optimize_artifact_save_ms,
                    None,
                )),
                fixed_candidate_result: None,
            };
            persist_bounded_item_checkpoint!(index);
            emit_campaign_progress(
                &app,
                "checkpoint",
                index + 1,
                dataset_total,
                &items[index].label,
                "dataset checkpoint saved",
                Some(&summary_json_path),
                &counts,
            );
            continue;
        }

        let validation_req_value = match build_validation_request_value(
            &strategy_kind,
            &data_id,
            &run_req_value,
            &optimize_result,
            &prepared.dataset_meta,
            &req.validation_config,
            family,
        ) {
            Ok(value) => value,
            Err(error) => {
                counts.failed = counts.failed.saturating_add(1);
                items[index] = failed_item(
                    prepared.input.csv_path.clone(),
                    prepared.label.clone(),
                    prepared.timeframe.clone(),
                    is_holdout,
                    Some(artifact_path.clone()),
                    "validation request",
                    error,
                );
                persist_bounded_item_checkpoint!(index);
                continue;
            }
        };
        let validation_base_fingerprint = run_artifacts::compute_config_fingerprint_from_artifact(
            &artifact_stub(&run_req_value, Some(&validation_req_value)),
        );
        let validation_fingerprint = campaign_artifact_config_fingerprint(
            &validation_base_fingerprint,
            &per_run_tags,
            is_holdout,
            holdout_tag.as_deref(),
        );
        let existing_validation = if req.skip_existing {
            existing_validation_match_indexed(
                &existing_reuse_index,
                &strategy_kind,
                &validation_fingerprint,
                &per_run_tags,
                &prepared,
                &mut warnings,
            )
        } else {
            None
        };
        if let Some(existing_validation) = existing_validation {
            counts.skipped = counts.skipped.saturating_add(1);
            items[index] = ResearchCampaignItemSummary {
                item_id: item_id.clone(),
                csv_path: prepared.input.csv_path.clone(),
                label,
                timeframe: prepared.timeframe.clone(),
                is_holdout,
                status: "skipped".to_string(),
                dataset_identity: prepared_identity.clone(),
                evidence_state: Some(CampaignEvidenceState::Complete),
                resume_state: Some(CampaignResumeState::Complete),
                reuse_decision: Some(format!(
                    "{}_validation_artifact",
                    existing_validation.decision.as_str()
                )),
                artifact_path: Some(existing_validation.summary.path.clone()),
                artifact_sha256: None,
                report_xlsx_path: None,
                error: None,
                diagnostics: Some(build_item_diagnostics(
                    prepare_source,
                    prepare_wait_ms,
                    &prefetch_diagnostics,
                    optimize_wall_ms,
                    optimize_artifact_save_ms,
                    None,
                )),
                fixed_candidate_result: None,
            };
            persist_bounded_item_checkpoint!(index);
            continue;
        }

        emit_campaign_progress(
            &app,
            "validate",
            index + 1,
            dataset_total,
            &prepared.label,
            "running validation",
            Some(&summary_json_path),
            &counts,
        );
        let validation_res_value = match family {
            CampaignOptimizationFamily::DoubleLike => {
                let validation_req: DoubleMaValidationRequest =
                    match serde_json::from_value(validation_req_value.clone())
                        .map_err(|e| e.to_string())
                    {
                        Ok(req) => req,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                prepared.label.clone(),
                                prepared.timeframe.clone(),
                                is_holdout,
                                Some(artifact_path.clone()),
                                "validation request decode",
                                error,
                            );
                            persist_bounded_item_checkpoint!(index);
                            continue;
                        }
                    };
                match ta_app_core::run_double_ma_validation_blocking_with_candles(
                    validation_req,
                    &prepared.candles,
                    cancel.as_ref(),
                    Some(&sink),
                ) {
                    Ok(validation_res) => {
                        match serde_json::to_value(validation_res).map_err(|e| e.to_string()) {
                            Ok(value) => value,
                            Err(error) => {
                                counts.failed = counts.failed.saturating_add(1);
                                items[index] = failed_item(
                                    prepared.input.csv_path.clone(),
                                    prepared.label.clone(),
                                    prepared.timeframe.clone(),
                                    is_holdout,
                                    Some(artifact_path.clone()),
                                    "validation encode",
                                    error,
                                );
                                persist_bounded_item_checkpoint!(index);
                                continue;
                            }
                        }
                    }
                    Err(error) => {
                        counts.failed = counts.failed.saturating_add(1);
                        items[index] = failed_item(
                            prepared.input.csv_path.clone(),
                            prepared.label.clone(),
                            prepared.timeframe.clone(),
                            is_holdout,
                            Some(artifact_path.clone()),
                            "validation",
                            error,
                        );
                        persist_bounded_item_checkpoint!(index);
                        continue;
                    }
                }
            }
            CampaignOptimizationFamily::CompositeCpu | CampaignOptimizationFamily::CompositeMo => {
                let validation_req: CompositeValidationRequest =
                    match serde_json::from_value(validation_req_value.clone())
                        .map_err(|e| e.to_string())
                    {
                        Ok(req) => req,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                prepared.label.clone(),
                                prepared.timeframe.clone(),
                                is_holdout,
                                Some(artifact_path.clone()),
                                "validation request decode",
                                error,
                            );
                            persist_bounded_item_checkpoint!(index);
                            continue;
                        }
                    };
                match ta_app_core::run_composite_validation_blocking_with_candles(
                    validation_req,
                    &prepared.candles,
                    cancel.as_ref(),
                    Some(&sink),
                ) {
                    Ok(validation_res) => {
                        match serde_json::to_value(validation_res).map_err(|e| e.to_string()) {
                            Ok(value) => value,
                            Err(error) => {
                                counts.failed = counts.failed.saturating_add(1);
                                items[index] = failed_item(
                                    prepared.input.csv_path.clone(),
                                    prepared.label.clone(),
                                    prepared.timeframe.clone(),
                                    is_holdout,
                                    Some(artifact_path.clone()),
                                    "validation encode",
                                    error,
                                );
                                persist_bounded_item_checkpoint!(index);
                                continue;
                            }
                        }
                    }
                    Err(error) => {
                        counts.failed = counts.failed.saturating_add(1);
                        items[index] = failed_item(
                            prepared.input.csv_path.clone(),
                            prepared.label.clone(),
                            prepared.timeframe.clone(),
                            is_holdout,
                            Some(artifact_path.clone()),
                            "validation",
                            error,
                        );
                        persist_bounded_item_checkpoint!(index);
                        continue;
                    }
                }
            }
        };
        counts.validated = counts.validated.saturating_add(1);
        let validation_artifact = build_run_artifact_value(
            &run_ts,
            &validation_fingerprint,
            &manifest_hash,
            is_holdout,
            holdout_tag.as_deref(),
            &prepared.input.csv_path,
            &prepared.dataset_meta,
            &per_run_tags,
            &run_req_value,
            &run_res_value,
            Some(&validation_req_value),
            Some(&validation_res_value),
        );
        artifact_path = match save_campaign_run_artifact_blocking(
            validation_artifact,
            app.clone(),
            None,
            &strategy_kind,
            execution_mode,
        ) {
            Ok(saved) => saved.path,
            Err(error) => {
                counts.failed = counts.failed.saturating_add(1);
                items[index] = failed_item(
                    prepared.input.csv_path.clone(),
                    prepared.label.clone(),
                    prepared.timeframe.clone(),
                    is_holdout,
                    Some(artifact_path.clone()),
                    "validation artifact",
                    error,
                );
                persist_bounded_item_checkpoint!(index);
                continue;
            }
        };
        items[index] = ResearchCampaignItemSummary {
            item_id: item_id.clone(),
            csv_path: prepared.input.csv_path.clone(),
            label,
            timeframe: prepared.timeframe.clone(),
            is_holdout,
            status: "ok".to_string(),
            dataset_identity: prepared_identity.clone(),
            evidence_state: Some(CampaignEvidenceState::Complete),
            resume_state: Some(CampaignResumeState::Complete),
            reuse_decision: reused_optimization_artifact
                .then(|| "exact_optimization_artifact_for_validation".to_string()),
            artifact_path: Some(artifact_path),
            artifact_sha256: None,
            report_xlsx_path: None,
            error: None,
            diagnostics: Some(build_item_diagnostics(
                prepare_source,
                prepare_wait_ms,
                &prefetch_diagnostics,
                optimize_wall_ms,
                optimize_artifact_save_ms,
                None,
            )),
            fixed_candidate_result: None,
        };
        persist_bounded_item_checkpoint!(index);
    }

    // The final (or only) dataset may observe cancellation inside its optimizer
    // and return before there is another dataset boundary to latch the flag.
    latch_campaign_cancellation(cancel.as_ref(), &mut counts);
    if should_seal_vcet_campaign_artifacts(execution_mode, counts.cancelled) {
        seal_vcet_campaign_artifact_hashes(&mut items, &app)?;
    }

    let mut meta_response = None;
    let mut stage5_response = None;
    if execution_mode == ResearchCampaignExecutionMode::OptimizeOnly {
        if req.auto_meta || req.auto_stage5 {
            warnings.push(
                "execution_mode=optimize_only ignores auto_meta and auto_stage5 follow-up stages."
                    .to_string(),
            );
        }
    } else if execution_mode == ResearchCampaignExecutionMode::EvaluateCandidates {
        if req.auto_meta || req.auto_stage5 {
            warnings.push(
                "execution_mode=evaluate_candidates defers Meta and Stage 5 until its terminal campaign summary is saved; run Meta from the completed campaign cohort."
                    .to_string(),
            );
        }
    } else if !counts.cancelled && req.auto_meta {
        if let Some(meta_req) = req.meta_request.clone() {
            match normalize_campaign_auto_meta_request(
                meta_req,
                &strategy_kind,
                timeframe_override.as_deref(),
                &campaign_cohort_tag,
                holdout_tag.as_deref(),
                !holdout_paths.is_empty(),
                &mut warnings,
            ) {
                Ok(meta_req) => {
                    emit_campaign_progress(
                        &app,
                        "meta",
                        counts.total,
                        counts.total,
                        &strategy_kind,
                        "running meta selection",
                        Some(&summary_json_path),
                        &counts,
                    );
                    match run_meta_selection_blocking(meta_req, None, app.clone(), cancel.clone()) {
                        Ok(response) => {
                            if req.auto_stage5 {
                                if let Some(mut stage5_req) = req.stage5_request.clone() {
                                    if stage5_req.source_meta_path.is_none() {
                                        stage5_req.source_meta_path =
                                            Some(response.meta_json_path.clone());
                                    }
                                    emit_campaign_progress(
                                        &app,
                                        "stage5",
                                        counts.total,
                                        counts.total,
                                        &strategy_kind,
                                        "running stage5",
                                        Some(&summary_json_path),
                                        &counts,
                                    );
                                    match run_meta_stage4_experimental_blocking(
                                        stage5_req,
                                        app.clone(),
                                    ) {
                                        Ok(stage5) => {
                                            stage5_response = Some(stage5);
                                        }
                                        Err(error) => {
                                            warnings.push(format!("stage5: {error}"));
                                        }
                                    }
                                }
                            }
                            meta_response = Some(response);
                        }
                        Err(error) => {
                            warnings.push(format!("meta: {error}"));
                        }
                    }
                }
                Err(error) => {
                    warnings.push(format!("meta: {error}"));
                }
            }
        } else {
            warnings.push("auto_meta was requested without a meta_request payload".to_string());
        }
    }

    let final_state = if counts.cancelled {
        "cancelled"
    } else if warnings
        .iter()
        .any(|entry| entry.starts_with("meta:") || entry.starts_with("stage5:"))
    {
        "completed_with_warnings"
    } else if counts.failed > 0 {
        "completed_with_failures"
    } else {
        "completed"
    };
    summary = build_campaign_summary(
        &ts,
        final_state,
        &strategy_kind,
        timeframe_override,
        &req,
        execution_mode,
        &tags,
        holdout_tag,
        &counts,
        &items,
        meta_response.as_ref(),
        stage5_response.as_ref(),
        &warnings,
        started.elapsed().as_millis() as u64,
    );
    let (_, mut summary_json_bytes) = save_campaign_summary_checkpoint_blocking(
        &mut summary,
        &mut items,
        None,
        &app,
        Some(&summary_json_path),
    )?;
    if let Err(error) = deferred_run_artifact_prune.finish() {
        let warning = format!("campaign run-artifact retention finalization failed: {error}");
        push_unique_warning(&mut warnings, warning.clone());
        push_unique_warning(&mut summary.warnings, warning.clone());
        if summary.state == "completed" {
            summary.state = "completed_with_warnings".to_string();
        }
        log::warn!("{warning}");
        let (_, persisted_bytes) = save_campaign_summary_checkpoint_blocking(
            &mut summary,
            &mut items,
            None,
            &app,
            Some(&summary_json_path),
        )
        .map_err(|persist_error| {
            format!(
                "{warning}; additionally failed to persist that retention warning to the campaign summary: {persist_error}"
            )
        })?;
        summary_json_bytes = persisted_bytes;
    }
    Ok(RunResearchCampaignResponse {
        ts,
        summary_json_path,
        summary_json_bytes,
        summary,
        meta: meta_response,
        stage5: stage5_response,
    })
}

fn run_research_campaign_blocking(
    req: ResearchCampaignRequest,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<RunResearchCampaignResponse, String> {
    let execution_mode = req.execution_mode;
    run_research_campaign_blocking_with_mode(req, app, cancel, execution_mode, None)
}

pub fn run_research_campaign_blocking_headless(
    req: ResearchCampaignRequest,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<RunResearchCampaignResponse, String> {
    run_research_campaign_blocking(req, app, cancel)
}

pub fn run_research_campaign_optimize_only_blocking_headless(
    req: ResearchCampaignRequest,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<RunResearchCampaignResponse, String> {
    run_research_campaign_blocking_with_mode(
        req,
        app,
        cancel,
        ResearchCampaignExecutionMode::OptimizeOnly,
        None,
    )
}

pub fn preload_research_campaign_dataset_pack_headless(
    req: &ResearchCampaignRequest,
    app: &tauri::AppHandle,
) -> Result<HeadlessPreparedResearchCampaignDatasetPack, String> {
    let normalized_datasets = normalize_campaign_datasets(req.datasets.clone())?;
    if normalized_datasets.is_empty() {
        return Err("campaign requires at least one dataset".to_string());
    }
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let app_cache_dir = data_cache::app_data_cache_dir(&base);
    let timeframe_override = req
        .timeframe_override
        .as_deref()
        .map(normalize_timeframe_label)
        .filter(|value| !value.is_empty());
    let mut prepared = VecDeque::with_capacity(normalized_datasets.len());
    let mut seen_prepared_source_fingerprints = HashMap::with_capacity(normalized_datasets.len());
    for dataset in normalized_datasets {
        let prepared_dataset =
            prepare_campaign_dataset(dataset, timeframe_override.as_deref(), &app_cache_dir)?;
        record_unique_prepared_campaign_source(
            &prepared_dataset,
            &mut seen_prepared_source_fingerprints,
        )?;
        prepared.push_back(prepared_dataset);
    }
    Ok(HeadlessPreparedResearchCampaignDatasetPack { datasets: prepared })
}

pub fn run_research_campaign_optimize_only_preloaded_pack_blocking_headless(
    req: ResearchCampaignRequest,
    preloaded_pack: HeadlessPreparedResearchCampaignDatasetPack,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<RunResearchCampaignResponse, String> {
    run_research_campaign_blocking_with_mode(
        req,
        app,
        cancel,
        ResearchCampaignExecutionMode::OptimizeOnly,
        Some(preloaded_pack),
    )
}

#[tauri::command]
pub async fn run_research_campaign(
    req: ResearchCampaignRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunResearchCampaignResponse, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    tauri::async_runtime::spawn_blocking(move || run_research_campaign_blocking(req, app, cancel))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "commands_campaign_vcet_test_support.rs"]
pub(crate) mod vcet_test_support;

#[cfg(test)]
mod tests {
    use super::super::commands_meta::load_meta_campaign_cohort_expectation;
    use super::{
        build_campaign_manifest, build_campaign_summary, build_composite_candidate_value,
        build_doublema_vram_prefetch_config, build_fixed_candidate_evaluation_summary,
        build_item_diagnostics, build_request_summary_value, build_run_artifact_value,
        build_vcet_fixed_candidate_plan, build_vcet_fixed_candidate_validation_request_value,
        build_vcet_temporal_campaign_candidate_source, campaign_artifact_config_fingerprint,
        campaign_artifact_tags, campaign_checkpoint_stride, campaign_item_id,
        can_reuse_optimization_artifact_for_validation, candidate_selection_from_optimization,
        candidate_values_from_optimization, classify_run_artifact_for_campaign,
        dataset_identity_summary, evaluate_vcet_fixed_candidate_artifact_reuse,
        existing_optimization_match, existing_validation_match, is_double_like_kind,
        latch_campaign_cancellation, load_existing_campaign_artifacts, normalize_backend_value,
        normalize_campaign_datasets, optimization_family_for_kind,
        optimization_result_from_artifact_value, optimization_trials_from_result,
        parse_vcet_fixed_candidate_artifact_result, parse_vcet_fixed_candidate_dataset_result,
        planned_campaign_full_summary_builds, planned_campaign_run_artifact_prunes,
        prepare_campaign_dataset, push_unique_warning, record_prefetch_feedback,
        record_unique_prepared_campaign_source, refresh_campaign_dataset_source_identities,
        requested_gpu_device_id, resolve_campaign_strategy_kind,
        save_campaign_summary_checkpoint_blocking, seal_vcet_campaign_artifact_hashes,
        search_distribution_from_result, should_load_existing_artifacts, split_holdout_paths,
        validate_campaign_summary_artifact_value, validate_holdout_tag_isolation,
        validate_vcet_fixed_candidate_artifact_contract,
        validate_vcet_fixed_candidate_artifact_request_contract,
        validate_vcet_fixed_candidate_training_evidence, vcet_fixed_candidate_id,
        vcet_fixed_candidate_run_result_value, write_campaign_summary_bytes_atomically,
        write_campaign_summary_bytes_with_publish, Backend, CampaignArtifactMatchDecision,
        CampaignArtifactReuseIndex, CampaignDoubleMaVramPrefetchConfig, CampaignEvidenceState,
        CampaignFixedCandidateArtifactReuse, CampaignFixedCandidateDatasetResult,
        CampaignFixedCandidateObservation, CampaignOptimizationFamily, CampaignOptimizationResult,
        CampaignPrefetchDiagnostics, CampaignResumeState, DatasetPreloadJob, DatasetPrepareSource,
        DoubleMaRequest, MetaSelectionRequest, PreparedCampaignDataset, ResearchCampaignCounts,
        ResearchCampaignCrossUniverseAdmissionPolicy, ResearchCampaignDatasetIdentitySummary,
        ResearchCampaignDatasetInput, ResearchCampaignExecutionMode,
        ResearchCampaignItemDiagnostics, ResearchCampaignItemSummary,
        ResearchCampaignPreloadPolicy, ResearchCampaignRequest, ResearchCampaignSummary,
        ResearchCampaignValidationConfig, ResearchCampaignVcetFixedCandidate,
    };
    use crate::state::{AppState, BackgroundTaskState};
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc};
    use ta_app_core::double_ma::GpuExecutionPolicy;
    use ta_app_core::progress::DoubleMaCampaignVramPrefetchFeedback;
    use ta_app_core::ValidationAdmissionPolicy;
    use ta_optimizer::{ObjectiveKind, OptimizationMode};
    use ta_strategies::double_ma::{Metrics, StrategyConfig};

    const SUPPORTED_CAMPAIGN_STRATEGIES: &[&str] = &[
        "DoubleMa",
        "Pit",
        "Set",
        "Mts",
        "Orc",
        "Marrt",
        "Vfct",
        "Rocb",
        "Vcet",
        "Mkcs",
        "Kqsr",
        "Carm",
        "Kvdf",
        "Cdmc",
        "Asr",
        "Vcc",
        "Vemr",
        "CompositeCpu",
        "CompositeMo",
        "CompositeTrend",
        "RegimeHybrid",
        "BreakoutMomentum",
    ];

    fn build_mock_app() -> tauri::App<tauri::test::MockRuntime> {
        tauri::test::mock_builder()
            .manage(AppState::default())
            .manage(BackgroundTaskState::default())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("failed to build tauri mock app")
    }

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        path.push(format!("{prefix}_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn vcet_fixed_campaign_request(
        fixed_candidates: &[(u16, u16)],
        dataset_count: usize,
    ) -> ResearchCampaignRequest {
        ResearchCampaignRequest {
            strategy_kind: "Vcet".to_string(),
            datasets: (0..dataset_count)
                .map(|index| ResearchCampaignDatasetInput {
                    csv_path: format!("C:/data/VCET_{index}.csv"),
                    label: Some(format!("VCET_{index}")),
                    timeframe: Some("1m".to_string()),
                    source_size_bytes: Some(1_024 + index as u64),
                    source_mtime_utc_ms: Some(1_720_000_000_000 + index as i64),
                    source_change_marker: Some(2_720_000_000_000 + index as i64),
                    source_content_fingerprint: Some(format!("sha256:vcet-{index}")),
                })
                .collect(),
            timeframe_override: Some("1m".to_string()),
            tags: vec!["vcet-fixed-campaign".to_string()],
            holdout_pct: 0,
            holdout_tag: None,
            skip_existing: true,
            auto_meta: false,
            auto_stage5: false,
            execution_mode: ResearchCampaignExecutionMode::EvaluateCandidates,
            optimization_template: json!({
                "strategy_kind": "Vcet",
                "data_id": "vcet-fixed-template",
                "backend": { "backend": "GpuKernel", "device_id": 0 },
                "fast_range": [4, 32, 1],
                "slow_range": [8, 96, 1],
                "fast_ma_types": ["vidya"],
                "slow_ma_types": ["vidya"],
                "ma_source": "close",
                "fast_ma_params": null,
                "slow_ma_params": null,
                "strategy": StrategyConfig::default(),
                "halftrend": {
                    "amplitude": 2,
                    "channel_deviation": 2.0,
                    "atr_period": 100
                },
                "vcet": {},
                "objective": "Sharpe",
                "mode": "Exact",
                "top_k": 32,
                "include_all": false,
                "heatmap_bins": 32
            }),
            validation_config: ResearchCampaignValidationConfig {
                max_candidates: 32,
                fixed_candidates: fixed_candidates
                    .iter()
                    .map(|&(fast_len, slow_len)| ResearchCampaignVcetFixedCandidate {
                        fast_len,
                        slow_len,
                    })
                    .collect(),
                require_gpu_kernel: true,
                ..ResearchCampaignValidationConfig::default()
            },
            meta_request: None,
            stage5_request: None,
            preload_policy: ResearchCampaignPreloadPolicy::HostOnly,
        }
    }

    fn vcet_fixed_validation_result_value(
        plan: &super::CampaignVcetFixedCandidatePlan,
        requested_backend: &str,
        backend_used: &str,
        backend_route: &str,
        pnls: &[f64],
    ) -> Value {
        let robust_scores: Vec<f64> = (0..plan.candidates.len())
            .map(|index| pnls.get(index).copied().unwrap_or_default() / 10.0 - 0.20)
            .collect();
        let max_robust_score = robust_scores
            .iter()
            .copied()
            .max_by(f64::total_cmp)
            .unwrap_or_default();
        let candidates: Vec<Value> = plan
            .candidates
            .iter()
            .enumerate()
            .map(|(index, params)| {
                let pnl = pnls.get(index).copied().unwrap_or_default();
                let robust_score = robust_scores[index];
                json!({
                    "params": params,
                    "in_sample": {
                        "pnl": pnl,
                        "sharpe": pnl / 10.0,
                        "max_dd": 0.10,
                        "trades": 20 + index as u32,
                        "exposure": 0.5,
                        "net_exposure": 0.5,
                    },
                    "validation_verdict": { "passed": pnl > 0.0 },
                    "robust_score": robust_score,
                    "robust_pareto": robust_score == max_robust_score,
                })
            })
            .collect();
        json!({
            "requested_backend": requested_backend,
            "backend_used": backend_used,
            "backend_route": backend_route,
            "backend_fallback_reason": null,
            "validation_residency_tier": if backend_used == "Gpu" && backend_route.contains("GpuKernel") {
                "gpu_kernel"
            } else {
                "cpu_fallback"
            },
            "candidates": candidates,
        })
    }

    fn prepared_dataset_for_matching() -> PreparedCampaignDataset {
        PreparedCampaignDataset {
            input: ResearchCampaignDatasetInput {
                csv_path: "C:/data/AAPL.csv".to_string(),
                label: Some("AAPL".to_string()),
                timeframe: Some("15m".to_string()),
                source_size_bytes: Some(10),
                source_mtime_utc_ms: Some(20),
                source_change_marker: Some(30),
                source_content_fingerprint: Some("stale-request-hint".to_string()),
            },
            candles: vector_ta::utilities::data_loader::Candles::new(
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            dataset_meta: json!({
                "csv_path": "C:/data/AAPL.csv",
                "timeframe": "15m",
                "source_size_bytes": 2048,
                "source_mtime_utc_ms": 1712000000000i64,
                "source_change_marker": 1712000000001i64,
                "source_content_fingerprint": "sha256:prepared"
            }),
            timeframe: "15m".to_string(),
            label: "AAPL".to_string(),
        }
    }

    fn run_artifact_summary(
        path: &str,
        has_validation: bool,
    ) -> super::run_artifacts::RunArtifactSummary {
        super::run_artifacts::RunArtifactSummary {
            ts: "2026-04-15T00:00:00Z".to_string(),
            path: path.to_string(),
            bytes: 1024,
            kind: "vectorgrid_run_artifact".to_string(),
            artifact_compatibility: super::run_artifacts::ArtifactCompatibilitySummary::default(),
            strategy_kind: "DoubleMa".to_string(),
            config_fingerprint: "cfg-1".to_string(),
            config_fingerprint_version: Some(1),
            has_validation,
            replay_class: "exact_evidence_review".to_string(),
            replay_reasons: Vec::new(),
            report_xlsx_path: String::new(),
            report_xlsx_exists: None,
            csv_path: "C:/data/AAPL.csv".to_string(),
            timeframe: "15m".to_string(),
            base_timeframe: "15m".to_string(),
            derived_timeframe: "15m".to_string(),
            resample_applied: Some(false),
            resample_rule: String::new(),
            resample_anchor: String::new(),
            source_size_bytes: Some(2048),
            source_mtime_utc_ms: Some(1712000000000),
            source_content_fingerprint: Some("sha256:prepared".to_string()),
            route_provenance: None,
            search_semantics_approx_mode: None,
            search_semantics_workload_family: None,
            search_semantics_contract_summary: None,
            search_semantics_declared_domain: None,
            search_semantics_base_ranker: None,
            search_semantics_rerank_operator: None,
            search_semantics_heatmap_semantics: None,
            vdps_quality_check_top_k_overlap: None,
            vdps_quality_check_best_score_regret: None,
            vdps_quality_check_exact_winner_rank: None,
            vdps_quality_check_holdout_regret: None,
            vdps_quality_check_speedup_vs_exact: None,
            lineage: None,
            tags: vec!["campaign-main".to_string()],
            ml_feature_schema_hash: None,
            ml_calibration_state: None,
            ml_threshold_policy: None,
            ml_producer_kind: None,
            ml_native_backend_used: None,
            ml_native_prediction_batch_size: None,
            ml_native_fallback_reason: None,
            ml_score_contract_version: None,
            ml_source_bundle_manifest_hash: None,
            ml_score_space: None,
            ml_score_scope: None,
            ml_score_contract_summary: None,
            ml_score_note: None,
        }
    }

    fn prepared_dataset_for_resume(
        symbol: &str,
        source_size_bytes: u64,
        source_mtime_utc_ms: i64,
        source_content_fingerprint: &str,
    ) -> PreparedCampaignDataset {
        let csv_path = format!("C:/data/{symbol}.csv");
        PreparedCampaignDataset {
            input: ResearchCampaignDatasetInput {
                csv_path: csv_path.clone(),
                label: Some(symbol.to_string()),
                timeframe: Some("15m".to_string()),
                source_size_bytes: Some(source_size_bytes),
                source_mtime_utc_ms: Some(source_mtime_utc_ms),
                source_change_marker: Some(source_mtime_utc_ms.saturating_add(1)),
                source_content_fingerprint: Some(source_content_fingerprint.to_string()),
            },
            candles: vector_ta::utilities::data_loader::Candles::new(
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            dataset_meta: json!({
                "csv_path": csv_path,
                "timeframe": "15m",
                "base_timeframe": "15m",
                "derived_timeframe": "15m",
                "source_size_bytes": source_size_bytes,
                "source_mtime_utc_ms": source_mtime_utc_ms,
                "source_content_fingerprint": source_content_fingerprint,
            }),
            timeframe: "15m".to_string(),
            label: symbol.to_string(),
        }
    }

    fn pending_resume_items(
        manifest_hash: &str,
        prepared: &[PreparedCampaignDataset],
    ) -> Vec<ResearchCampaignItemSummary> {
        prepared
            .iter()
            .enumerate()
            .map(|(index, dataset)| ResearchCampaignItemSummary {
                item_id: Some(campaign_item_id(
                    manifest_hash,
                    index,
                    &dataset.input.csv_path,
                    &dataset.timeframe,
                )),
                csv_path: dataset.input.csv_path.clone(),
                label: dataset.label.clone(),
                timeframe: dataset.timeframe.clone(),
                is_holdout: false,
                status: "pending".to_string(),
                dataset_identity: Some(dataset_identity_summary(dataset)),
                evidence_state: Some(CampaignEvidenceState::None),
                resume_state: Some(CampaignResumeState::Resumable),
                reuse_decision: None,
                artifact_path: None,
                artifact_sha256: None,
                report_xlsx_path: None,
                error: None,
                diagnostics: None,
                fixed_candidate_result: None,
            })
            .collect()
    }

    fn resume_artifact_summary(
        artifact_dir: &std::path::Path,
        item_id: &str,
        prepared: &PreparedCampaignDataset,
    ) -> super::run_artifacts::RunArtifactSummary {
        let path = artifact_dir.join(format!("{item_id}.json"));
        let mut artifact = run_artifact_summary(path.to_string_lossy().as_ref(), false);
        artifact.config_fingerprint = "cfg-resume-v1".to_string();
        artifact.csv_path = prepared.input.csv_path.clone();
        artifact.timeframe = prepared.timeframe.clone();
        artifact.base_timeframe = prepared.timeframe.clone();
        artifact.derived_timeframe = prepared.timeframe.clone();
        artifact.source_size_bytes = prepared.input.source_size_bytes;
        artifact.source_mtime_utc_ms = prepared.input.source_mtime_utc_ms;
        artifact.source_content_fingerprint = prepared.input.source_content_fingerprint.clone();
        artifact.tags = vec!["campaign-resume".to_string()];
        artifact
    }

    fn complete_resume_item(
        item: &mut ResearchCampaignItemSummary,
        status: &str,
        artifact_path: String,
        reuse_decision: Option<String>,
    ) {
        item.status = status.to_string();
        item.evidence_state = Some(CampaignEvidenceState::Complete);
        item.resume_state = Some(CampaignResumeState::Complete);
        item.reuse_decision = reuse_decision;
        item.artifact_path = Some(artifact_path);
    }

    fn write_test_json_atomically(path: &std::path::Path, value: &impl serde::Serialize) {
        let bytes = serde_json::to_vec_pretty(value).expect("serialize test checkpoint");
        write_campaign_summary_bytes_atomically(path, &bytes)
            .expect("publish test checkpoint atomically");
    }

    fn read_test_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> T {
        serde_json::from_slice(&std::fs::read(path).expect("read test checkpoint"))
            .expect("deserialize test checkpoint")
    }

    fn resumable_summary_value() -> Value {
        let req = ResearchCampaignRequest {
            strategy_kind: "DoubleMa".to_string(),
            datasets: vec![ResearchCampaignDatasetInput {
                csv_path: "C:/data/AAPL.csv".to_string(),
                label: Some("AAPL".to_string()),
                timeframe: Some("15m".to_string()),
                source_size_bytes: Some(2_048),
                source_mtime_utc_ms: Some(1_712_000_000_000),
                source_change_marker: Some(1_712_000_000_001),
                source_content_fingerprint: Some("fnv1a64-full-aabbccdd".to_string()),
            }],
            timeframe_override: Some("15m".to_string()),
            tags: vec!["campaign-resume-contract".to_string()],
            holdout_pct: 0,
            holdout_tag: None,
            skip_existing: true,
            auto_meta: false,
            auto_stage5: false,
            execution_mode: ResearchCampaignExecutionMode::OptimizeOnly,
            optimization_template: json!({
                "strategy_kind": "DoubleMa",
                "data_id": "campaign_resume_contract",
            }),
            validation_config: ResearchCampaignValidationConfig::default(),
            meta_request: None,
            stage5_request: None,
            preload_policy: ResearchCampaignPreloadPolicy::Disabled,
        };
        let manifest = build_campaign_manifest(
            &req,
            "DoubleMa",
            Some("15m"),
            &req.tags,
            None,
            ResearchCampaignExecutionMode::OptimizeOnly,
        );
        let items = vec![ResearchCampaignItemSummary {
            item_id: Some(campaign_item_id(
                &manifest.manifest_hash,
                0,
                "C:/data/AAPL.csv",
                "15m",
            )),
            csv_path: "C:/data/AAPL.csv".to_string(),
            label: "AAPL".to_string(),
            timeframe: "15m".to_string(),
            is_holdout: false,
            status: "pending".to_string(),
            dataset_identity: None,
            evidence_state: Some(CampaignEvidenceState::None),
            resume_state: Some(CampaignResumeState::Resumable),
            reuse_decision: None,
            artifact_path: None,
            artifact_sha256: None,
            report_xlsx_path: None,
            error: None,
            diagnostics: None,
            fixed_candidate_result: None,
        }];
        let counts = ResearchCampaignCounts {
            total: 1,
            optimized: 0,
            validated: 0,
            skipped: 0,
            failed: 0,
            cancelled: false,
        };
        serde_json::to_value(build_campaign_summary(
            "2026-07-25T12:00:00Z",
            "running",
            "DoubleMa",
            Some("15m".to_string()),
            &req,
            ResearchCampaignExecutionMode::OptimizeOnly,
            &req.tags,
            None,
            &counts,
            &items,
            None,
            None,
            &[],
            5,
        ))
        .expect("serialize resumable campaign summary")
    }

    #[test]
    fn post_loop_cancellation_latches_for_a_single_failed_dataset() {
        let cancel = AtomicBool::new(true);
        let mut counts = ResearchCampaignCounts {
            total: 1,
            optimized: 0,
            validated: 0,
            skipped: 0,
            failed: 1,
            cancelled: false,
        };

        latch_campaign_cancellation(&cancel, &mut counts);

        assert!(counts.cancelled);
        assert_eq!(counts.failed, 1);
    }

    #[test]
    fn vcet_artifact_sealing_policy_skips_cancelled_checkpoints_only() {
        assert!(!super::should_seal_vcet_campaign_artifacts(
            ResearchCampaignExecutionMode::EvaluateCandidates,
            true,
        ));
        assert!(super::should_seal_vcet_campaign_artifacts(
            ResearchCampaignExecutionMode::EvaluateCandidates,
            false,
        ));
        assert!(!super::should_seal_vcet_campaign_artifacts(
            ResearchCampaignExecutionMode::Full,
            false,
        ));
    }

    #[test]
    fn campaign_summary_resume_contract_accepts_untampered_v2_summary() {
        validate_campaign_summary_artifact_value(&resumable_summary_value())
            .expect("valid v2 campaign summary");
    }

    #[test]
    fn campaign_summary_resume_contract_rejects_request_manifest_tampering() {
        let mut summary = resumable_summary_value();
        summary["request"]["optimization_template"]["fast_range"] = json!([3, 30, 3]);
        let error = validate_campaign_summary_artifact_value(&summary)
            .expect_err("tampered request must not remain resumable");
        assert!(error.contains("integrity manifest"));
    }

    #[test]
    fn campaign_summary_resume_contract_keeps_legacy_summary_viewable() {
        validate_campaign_summary_artifact_value(&json!({
            "version": 1,
            "kind": "vectorgrid_campaign_summary",
            "state": "cancelled",
            "request": {"datasets": []}
        }))
        .expect("legacy summaries remain viewable but are blocked by the UI from resume");
    }

    #[test]
    fn dynamic_double_ma_is_rejected_for_campaigns() {
        let err = resolve_campaign_strategy_kind("DynamicDoubleMa").unwrap_err();
        assert!(
            err.contains("do not support DynamicDoubleMa"),
            "unexpected error: {err}"
        );
        assert!(!is_double_like_kind("DynamicDoubleMa"));
    }

    #[test]
    fn ml_signal_is_rejected_for_campaigns() {
        let err = resolve_campaign_strategy_kind("MlSignal").unwrap_err();
        assert!(
            err.contains("do not support MlSignal"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn unknown_strategy_is_rejected_for_campaigns() {
        let err = resolve_campaign_strategy_kind("UnknownFutureStrategy").unwrap_err();
        assert!(err.contains("Unsupported research campaign strategy kind"));
    }

    #[test]
    fn supported_campaign_strategy_matrix_is_accepted() {
        for kind in SUPPORTED_CAMPAIGN_STRATEGIES {
            assert_eq!(
                resolve_campaign_strategy_kind(kind).as_deref(),
                Ok(*kind),
                "expected {kind} to stay supported by campaign orchestration"
            );
        }
    }

    #[test]
    fn campaign_strategy_matrix_maps_to_expected_optimization_families() {
        for kind in [
            "DoubleMa",
            "Pit",
            "Set",
            "Mts",
            "Orc",
            "Marrt",
            "Vfct",
            "CompositeTrend",
            "RegimeHybrid",
            "BreakoutMomentum",
        ] {
            assert_eq!(
                optimization_family_for_kind(kind),
                CampaignOptimizationFamily::DoubleLike,
                "expected {kind} to stay on the double-like campaign optimization path"
            );
        }

        for kind in [
            "CompositeCpu",
            "Asr",
            "Vcc",
            "Vemr",
            "Rocb",
            "Vcet",
            "Mkcs",
            "Kqsr",
            "Carm",
            "Kvdf",
            "Cdmc",
        ] {
            assert_eq!(
                optimization_family_for_kind(kind),
                CampaignOptimizationFamily::CompositeCpu,
                "expected {kind} to stay on the composite CPU campaign optimization path"
            );
        }

        assert_eq!(
            optimization_family_for_kind("CompositeMo"),
            CampaignOptimizationFamily::CompositeMo
        );
    }

    #[test]
    fn optimization_artifact_reuse_policy_supports_all_campaign_families() {
        assert!(can_reuse_optimization_artifact_for_validation(
            CampaignOptimizationFamily::DoubleLike
        ));
        assert!(can_reuse_optimization_artifact_for_validation(
            CampaignOptimizationFamily::CompositeCpu
        ));
        assert!(can_reuse_optimization_artifact_for_validation(
            CampaignOptimizationFamily::CompositeMo
        ));
    }

    #[test]
    fn composite_mo_artifact_reuse_reads_pareto_candidates_from_raw_json() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeMo",
                    "strategy": { "fee_bps": 1.5 }
                },
                "res": {
                    "data_id": "campaign_1",
                    "pareto": [
                        {
                            "params": {
                                "fast_ma_type": "alma",
                                "slow_ma_type": "alma",
                                "fast_len": 12,
                                "slow_len": 48,
                                "strategy": { "fee_bps": 1.5 }
                            }
                        }
                    ],
                    "total_evaluations": 1,
                    "runtime_ms": 10,
                    "telemetry": {
                        "prescan": null,
                        "search": {
                            "sampler": "ExactGrid",
                            "requested_evaluations": 1,
                            "actual_evaluations": 1,
                            "unique_evaluations": 1,
                            "duplicate_evaluations_avoided": 0,
                            "feasible_points": 1,
                            "rejected_decode_or_order": 0,
                            "rejected_prescan_pair": 0,
                            "rejected_other_constraints": 0,
                            "exact_grid_used": true,
                            "exact_grid_total_combinations": 1
                        }
                    }
                }
            }
        });
        let run_req = artifact.pointer("/run/req").cloned().expect("run.req");
        let optimization = optimization_result_from_artifact_value(
            CampaignOptimizationFamily::CompositeMo,
            &artifact,
        )
        .expect("decode CompositeMo artifact");
        assert!(matches!(
            optimization,
            CampaignOptimizationResult::CompositeMoArtifact(_)
        ));

        let candidates = candidate_values_from_optimization(
            "CompositeMo",
            CampaignOptimizationFamily::CompositeMo,
            &run_req,
            &optimization,
            8,
        )
        .expect("candidates from raw artifact");
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].get("fast_ma_type").and_then(Value::as_str),
            Some("alma")
        );
        assert_eq!(
            candidates[0].get("slow_len").and_then(Value::as_u64),
            Some(48)
        );
    }

    #[test]
    fn composite_campaign_validation_defaults_missing_backend_to_auto() {
        assert_eq!(
            normalize_backend_value(None, "CompositeMo"),
            json!({ "backend": "Auto", "device_id": 0 })
        );
    }

    #[test]
    fn double_like_campaign_validation_defaults_missing_backend_to_cpu_only() {
        assert_eq!(
            normalize_backend_value(None, "DoubleMa"),
            json!({ "backend": "CpuOnly" })
        );
    }

    #[test]
    fn campaign_validation_backend_normalization_preserves_or_collapses_by_family() {
        let gpu_kernel = json!({ "backend": "GpuKernel", "device_id": 7 });
        let gpu_hybrid = json!({ "backend": "GpuHybridKernel", "device_id": 5 });
        let cpu_only = json!({ "backend": "CpuOnly" });

        for kind in [
            "DoubleMa",
            "Pit",
            "Set",
            "Mts",
            "Orc",
            "Marrt",
            "Vfct",
            "CompositeTrend",
            "RegimeHybrid",
            "BreakoutMomentum",
        ] {
            assert_eq!(
                normalize_backend_value(Some(&gpu_kernel), kind),
                gpu_kernel,
                "expected {kind} to preserve kernel-capable validation backend requests"
            );
            assert_eq!(
                normalize_backend_value(Some(&gpu_hybrid), kind),
                gpu_hybrid,
                "expected {kind} to preserve hybrid validation backend requests"
            );
        }

        for kind in [
            "CompositeCpu",
            "Asr",
            "Vcc",
            "Vemr",
            "Rocb",
            "Mkcs",
            "Kqsr",
            "Carm",
            "Kvdf",
            "Cdmc",
            "CompositeMo",
        ] {
            assert_eq!(
                normalize_backend_value(Some(&gpu_kernel), kind),
                json!({ "backend": "Auto", "device_id": 7 }),
                "expected {kind} to collapse GPU kernel requests to Auto for composite-family validation"
            );
            assert_eq!(
                normalize_backend_value(Some(&gpu_hybrid), kind),
                json!({ "backend": "Auto", "device_id": 5 }),
                "expected {kind} to collapse GPU hybrid requests to Auto for composite-family validation"
            );
            assert_eq!(
                normalize_backend_value(Some(&cpu_only), kind),
                cpu_only,
                "expected {kind} to preserve CpuOnly requests"
            );
        }

        assert_eq!(
            normalize_backend_value(Some(&gpu_kernel), "Vcet"),
            gpu_kernel,
            "VCET fixed-candidate validation must preserve its explicit GpuKernel route"
        );
        assert_eq!(
            normalize_backend_value(Some(&gpu_hybrid), "Vcet"),
            json!({ "backend": "Auto", "device_id": 5 }),
            "VCET does not expose a hybrid fixed-candidate route"
        );
    }

    #[test]
    fn vcet_evaluate_candidates_builds_canonical_typed_manifest_and_distinct_fingerprint() {
        let req = vcet_fixed_campaign_request(&[(12, 48), (6, 24)], 1);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid fixed-candidate plan")
        .expect("evaluate mode plan");
        assert_eq!(plan.candidates.len(), 2);
        assert_eq!(
            plan.candidates[0].get("fast_len").and_then(Value::as_u64),
            Some(6)
        );
        assert_eq!(
            plan.candidates[1].get("slow_len").and_then(Value::as_u64),
            Some(48)
        );
        for candidate in &plan.candidates {
            let decoded: ta_app_core::CompositeParams =
                serde_json::from_value(candidate.clone()).expect("typed CompositeParams candidate");
            assert_eq!(decoded.halftrend.amplitude, 2);
            assert!(decoded.vcet.is_some());
        }

        let manifest = build_campaign_manifest(
            &req,
            "Vcet",
            Some("1m"),
            &req.tags,
            None,
            ResearchCampaignExecutionMode::EvaluateCandidates,
        );
        assert_eq!(
            manifest.candidate_manifest_hash.as_deref(),
            Some(plan.candidate_manifest_hash.as_str())
        );
        let validation_req = build_vcet_fixed_candidate_validation_request_value(
            "vcet-data-1",
            &req.optimization_template,
            &json!({
                "csv_path": "C:/data/VCET_0.csv",
                "timeframe": "1m",
                "label": "VCET_0"
            }),
            &req.validation_config,
            &plan,
        );
        assert_eq!(
            validation_req
                .pointer("/backend/backend")
                .and_then(Value::as_str),
            Some("GpuKernel")
        );
        assert_eq!(
            validation_req
                .get("candidate_manifest_hash")
                .and_then(Value::as_str),
            Some(plan.candidate_manifest_hash.as_str())
        );
        let decoded_validation: ta_app_core::CompositeValidationRequest =
            serde_json::from_value(validation_req.clone())
                .expect("fixed-candidate request must decode for composite validation");
        assert_eq!(decoded_validation.candidates.len(), 2);
        let fingerprint = super::run_artifacts::compute_config_fingerprint_from_artifact(
            &super::artifact_stub(&req.optimization_template, Some(&validation_req)),
        );

        let changed_req = vcet_fixed_campaign_request(&[(6, 24), (12, 49)], 1);
        let changed_plan = build_vcet_fixed_candidate_plan(
            &changed_req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("changed fixed-candidate plan")
        .expect("changed evaluate mode plan");
        let changed_validation_req = build_vcet_fixed_candidate_validation_request_value(
            "vcet-data-1",
            &changed_req.optimization_template,
            &json!({
                "csv_path": "C:/data/VCET_0.csv",
                "timeframe": "1m",
                "label": "VCET_0"
            }),
            &changed_req.validation_config,
            &changed_plan,
        );
        let changed_fingerprint =
            super::run_artifacts::compute_config_fingerprint_from_artifact(&super::artifact_stub(
                &changed_req.optimization_template,
                Some(&changed_validation_req),
            ));
        assert_ne!(
            plan.candidate_manifest_hash,
            changed_plan.candidate_manifest_hash
        );
        assert_ne!(fingerprint, changed_fingerprint);
    }

    #[test]
    fn vcet_fixed_candidate_identity_rejects_semantic_period_aliases() {
        let mut req = vcet_fixed_campaign_request(&[(6, 24), (12, 48)], 1);
        req.optimization_template["vcet"] = json!({
            "map_fast_to_vidya_short_period": false,
            "map_slow_to_vidya_long_period": false,
            "map_slow_to_ce_period": false,
            "vidya_short_period": 7,
            "vidya_long_period": 31,
            "vidya_alpha": 0.25,
            "ce_period": 19,
            "ce_mult": 2.75,
            "ce_use_close": true,
            "require_vidya_slope": true,
            "max_trade_bars": 90
        });

        let error = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect_err("different period pairs resolving to one VCET strategy must be rejected");
        assert!(error.contains("duplicate effective strategy identity"));
    }

    #[test]
    fn vcet_fixed_candidate_identity_hashes_every_effective_vcet_control() {
        let base = json!({
            "map_fast_to_vidya_short_period": false,
            "map_slow_to_vidya_long_period": false,
            "map_slow_to_ce_period": false,
            "vidya_short_period": 6,
            "vidya_long_period": 24,
            "vidya_alpha": 0.2,
            "ce_period": 22,
            "ce_mult": 3.0,
            "ce_use_close": true,
            "require_vidya_slope": true,
            "max_trade_bars": 120
        });
        let candidate_id_for = |vcet: Value| {
            let mut req = vcet_fixed_campaign_request(&[(6, 24)], 1);
            req.optimization_template["vcet"] = vcet;
            let plan = build_vcet_fixed_candidate_plan(
                &req,
                "Vcet",
                ResearchCampaignExecutionMode::EvaluateCandidates,
            )
            .expect("valid fixed-candidate plan")
            .expect("evaluate mode plan");
            vcet_fixed_candidate_id(plan.candidate_identities[0])
        };
        let base_id = candidate_id_for(base.clone());

        let mut variants = Vec::new();
        for (field, value) in [
            ("vidya_short_period", json!(7)),
            ("vidya_long_period", json!(25)),
            ("vidya_alpha", json!(0.3)),
            ("ce_period", json!(23)),
            ("ce_mult", json!(3.5)),
            ("ce_use_close", json!(false)),
            ("require_vidya_slope", json!(false)),
            ("max_trade_bars", json!(121)),
        ] {
            let mut variant = base.clone();
            variant[field] = value;
            variants.push((field, variant));
        }
        for (field, variant) in variants {
            assert_ne!(
                candidate_id_for(variant),
                base_id,
                "changing effective VCET control '{field}' must change candidate identity"
            );
        }

        let mut equivalent_mapping = base;
        equivalent_mapping["map_fast_to_vidya_short_period"] = Value::Bool(true);
        equivalent_mapping["map_slow_to_vidya_long_period"] = Value::Bool(true);
        assert_eq!(
            candidate_id_for(equivalent_mapping),
            base_id,
            "mapping mechanics must not change identity when the resolved strategy is unchanged"
        );
    }

    #[test]
    fn vcet_qmc_search_expands_to_a_sealed_multidimensional_candidate_manifest() {
        let mut req = vcet_fixed_campaign_request(&[(6, 24)], 1);
        req.validation_config.fixed_candidates.clear();
        req.validation_config.max_candidates = 12;
        req.validation_config.vcet_search = Some(ta_app_core::vcet_search::VcetSearchConfig {
            budget: 12,
            seed: 42,
            vidya_short_period: ta_app_core::vcet_search::VcetU16Range {
                min: 4,
                max: 8,
                step: 2,
            },
            vidya_long_period: ta_app_core::vcet_search::VcetU16Range {
                min: 16,
                max: 24,
                step: 4,
            },
            vidya_alpha: ta_app_core::vcet_search::VcetQuantizedFloatRange {
                min: 0.1,
                max: 0.3,
                quantum: 0.1,
            },
            ce_period: ta_app_core::vcet_search::VcetU16Range {
                min: 12,
                max: 20,
                step: 4,
            },
            ce_mult: ta_app_core::vcet_search::VcetQuantizedFloatRange {
                min: 2.0,
                max: 3.0,
                quantum: 0.5,
            },
            ce_use_close: ta_app_core::vcet_search::VcetBoolChoices::both(),
            require_vidya_slope: ta_app_core::vcet_search::VcetBoolChoices::both(),
            max_trade_bars: ta_app_core::vcet_search::VcetU16Range {
                min: 60,
                max: 120,
                step: 30,
            },
        });

        let first = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid bounded VCET search")
        .expect("evaluate-candidate plan");
        let second = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("deterministic bounded VCET search")
        .expect("evaluate-candidate plan");

        assert_eq!(first.candidates.len(), 12);
        assert_eq!(first.candidate_identities.len(), 12);
        assert_eq!(
            first.candidate_manifest_hash,
            second.candidate_manifest_hash
        );
        assert_eq!(first.search_manifest, second.search_manifest);
        let search = first
            .search_manifest
            .as_ref()
            .expect("sealed search manifest");
        assert_eq!(search.requested_budget, 12);
        assert_eq!(search.effective_budget, 12);
        assert!(search
            .candidates
            .iter()
            .zip(first.candidate_identities.iter().copied())
            .all(
                |(sampled, identity)| sampled.evaluation_candidate_id.as_deref()
                    == Some(vcet_fixed_candidate_id(identity).as_str())
            ));
        assert!(first.candidates.iter().all(|candidate| {
            candidate
                .pointer("/vcet/map_fast_to_vidya_short_period")
                .and_then(Value::as_bool)
                == Some(false)
                && candidate
                    .pointer("/vcet/map_slow_to_vidya_long_period")
                    .and_then(Value::as_bool)
                    == Some(false)
                && candidate
                    .pointer("/vcet/map_slow_to_ce_period")
                    .and_then(Value::as_bool)
                    == Some(false)
        }));

        let validation_res = vcet_fixed_validation_result_value(
            &first,
            "GpuKernel",
            "Gpu",
            "GpuKernel",
            &vec![1.0; first.candidates.len()],
        );
        let mut artifact = json!({
            "run": { "res": vcet_fixed_candidate_run_result_value(&first) },
            "validation": {
                "req": {
                    "candidate_manifest_hash": first.candidate_manifest_hash,
                    "candidate_search_id": search.search_id.as_str(),
                    "candidate_search_schema_version": search.schema_version,
                },
                "res": validation_res,
            },
        });
        parse_vcet_fixed_candidate_artifact_result(&artifact, &first, true)
            .expect("exact search lineage is replayable");
        artifact["validation"]["req"]["candidate_search_id"] =
            Value::String("vcet_search_v1_tampered".to_string());
        assert!(
            parse_vcet_fixed_candidate_artifact_result(&artifact, &first, true)
                .expect_err("candidate-equivalent artifact with different search lineage must fail")
                .contains("search lineage")
        );
        artifact["validation"]["req"]["candidate_search_id"] =
            Value::String(search.search_id.clone());
        artifact["run"]["res"]["candidate_search_schema_version"] = Value::from(99_u64);
        assert!(
            parse_vcet_fixed_candidate_artifact_result(&artifact, &first, true)
                .expect_err("run-result search lineage must also be sealed")
                .contains("search lineage")
        );
    }

    #[test]
    fn vcet_qmc_aggregation_keeps_same_period_pairs_with_different_controls_distinct() {
        let mut req = vcet_fixed_campaign_request(&[(6, 24)], 1);
        req.validation_config.fixed_candidates.clear();
        req.validation_config.max_candidates = 2;
        let mut search = ta_app_core::vcet_search::VcetSearchConfig::default();
        search.budget = 2;
        search.vidya_short_period = ta_app_core::vcet_search::VcetU16Range {
            min: 6,
            max: 6,
            step: 1,
        };
        search.vidya_long_period = ta_app_core::vcet_search::VcetU16Range {
            min: 24,
            max: 24,
            step: 1,
        };
        search.vidya_alpha = ta_app_core::vcet_search::VcetQuantizedFloatRange {
            min: 0.2,
            max: 0.3,
            quantum: 0.1,
        };
        search.ce_period = ta_app_core::vcet_search::VcetU16Range {
            min: 20,
            max: 20,
            step: 1,
        };
        search.ce_mult = ta_app_core::vcet_search::VcetQuantizedFloatRange {
            min: 3.0,
            max: 3.0,
            quantum: 0.5,
        };
        search.ce_use_close = ta_app_core::vcet_search::VcetBoolChoices::fixed(true);
        search.require_vidya_slope = ta_app_core::vcet_search::VcetBoolChoices::fixed(true);
        search.max_trade_bars = ta_app_core::vcet_search::VcetU16Range {
            min: 120,
            max: 120,
            step: 1,
        };
        req.validation_config.vcet_search = Some(search);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid search")
        .expect("candidate plan");
        assert_eq!(plan.candidate_identities.len(), 2);
        assert_eq!(
            (
                plan.candidate_identities[0].vidya_short_period,
                plan.candidate_identities[0].vidya_long_period
            ),
            (
                plan.candidate_identities[1].vidya_short_period,
                plan.candidate_identities[1].vidya_long_period
            )
        );
        assert_ne!(plan.candidate_identities[0], plan.candidate_identities[1]);

        let result = CampaignFixedCandidateDatasetResult {
            gpu_kernel_used: true,
            candidates: vec![
                CampaignFixedCandidateObservation {
                    identity: plan.candidate_identities[0],
                    pnl: -7.0,
                    sharpe: -0.7,
                    max_drawdown: 0.4,
                    trades: 3,
                    hard_gate_passed: false,
                },
                CampaignFixedCandidateObservation {
                    identity: plan.candidate_identities[1],
                    pnl: 11.0,
                    sharpe: 1.1,
                    max_drawdown: 0.1,
                    trades: 30,
                    hard_gate_passed: true,
                },
            ],
        };
        let aggregate =
            super::build_fixed_candidate_cohort_aggregate_from_results(&plan, 1, &[&result])
                .expect("identity-safe aggregate");
        assert_eq!(aggregate.candidates[0].pnl_median, Some(-7.0));
        assert_eq!(aggregate.candidates[0].hard_gate_pass_datasets, 0);
        assert_eq!(aggregate.candidates[1].pnl_median, Some(11.0));
        assert_eq!(aggregate.candidates[1].hard_gate_pass_datasets, 1);
    }

    #[test]
    fn vcet_qmc_search_is_mutually_exclusive_with_legacy_period_pairs_and_respects_cap() {
        let mut req = vcet_fixed_campaign_request(&[(6, 24)], 1);
        let mut search = ta_app_core::vcet_search::VcetSearchConfig::default();
        search.budget = 4;
        req.validation_config.vcet_search = Some(search.clone());
        let error = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect_err("two candidate-design contracts must be rejected");
        assert!(error.contains("either validation_config.fixed_candidates"));

        req.validation_config.fixed_candidates.clear();
        req.validation_config.max_candidates = 3;
        let error = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect_err("search manifest must not be silently truncated");
        assert!(error.contains("generated 4 candidates"));
        assert!(error.contains("max_candidates=3"));
    }

    #[test]
    fn vcet_frontend_like_evaluate_candidates_request_decodes_complete_builder_contract() {
        let request_value = json!({
            "strategy_kind": "Vcet",
            "datasets": [{
                "csv_path": "C:/data/VCET_FRONTEND.csv",
                "label": "VCET_FRONTEND",
                "timeframe": "1m",
                "source_size_bytes": 1234,
                "source_mtime_utc_ms": 1720000000000_i64,
                "source_content_fingerprint": "sha256:frontend-vcet"
            }],
            "timeframe_override": "1m",
            "tags": ["vcet-frontend-contract"],
            "holdout_pct": 20,
            "holdout_tag": "vcet-holdout",
            "skip_existing": true,
            "auto_meta": false,
            "auto_stage5": false,
            "execution_mode": "evaluate_candidates",
            "optimization_template": {
                "strategy_kind": "Vcet",
                "data_id": "frontend-vcet-template",
                "backend": { "backend": "GpuKernel", "device_id": 0 },
                "fast_range": [4, 32, 1],
                "slow_range": [8, 96, 1],
                "fast_ma_types": ["vidya"],
                "slow_ma_types": ["vidya"],
                "ma_source": "close",
                "fast_ma_params": null,
                "slow_ma_params": null,
                "strategy": StrategyConfig::default(),
                "halftrend": {
                    "amplitude": 2,
                    "channel_deviation": 2.0,
                    "atr_period": 100
                },
                "vcet": {},
                "objective": "Sharpe",
                "mode": "Exact",
                "top_k": 32,
                "include_all": false,
                "heatmap_bins": 32
            },
            "validation_config": {
                "max_candidates": 2,
                "seed": 7,
                "admission_policy": "legacy_topk",
                "fixed_candidates": [
                    { "fast_len": 6, "slow_len": 24 },
                    { "fast_len": 12, "slow_len": 48 }
                ],
                "require_gpu_kernel": true
            },
            "preload_policy": "host_only"
        });

        let req: ResearchCampaignRequest =
            serde_json::from_value(request_value).expect("decode frontend campaign request");
        validate_holdout_tag_isolation(
            &req.tags,
            &req.datasets,
            req.holdout_pct,
            req.holdout_tag.as_deref(),
        )
        .expect("frontend cohort tags remain disjoint");
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("validate frontend VCET request")
        .expect("fixed candidate plan");
        assert_eq!(plan.candidates.len(), 2);
        for candidate in plan.candidates {
            let decoded: ta_app_core::CompositeParams =
                serde_json::from_value(candidate).expect("canonical typed VCET candidate");
            assert!(decoded.vcet.is_some());
            assert_eq!(decoded.halftrend.amplitude, 2);
        }
    }

    #[test]
    fn vcet_evaluate_candidates_rejects_ambiguous_or_truncated_manifests() {
        let duplicate = vcet_fixed_campaign_request(&[(6, 24), (6, 24)], 1);
        assert!(build_vcet_fixed_candidate_plan(
            &duplicate,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect_err("duplicate manifest must fail")
        .contains("duplicate"));

        let invalid_mapped_short = vcet_fixed_campaign_request(&[(1, 24)], 1);
        assert!(build_vcet_fixed_candidate_plan(
            &invalid_mapped_short,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect_err("mapped Fast=1 must fail the effective VIDYA contract")
        .contains("effective vidya_short_period must be >= 2"));

        let mut over_cap = vcet_fixed_campaign_request(&[(6, 24), (12, 48)], 1);
        over_cap.validation_config.max_candidates = 1;
        assert!(build_vcet_fixed_candidate_plan(
            &over_cap,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect_err("over-cap manifest must fail instead of truncating")
        .contains("will not be truncated"));

        let mut wrong_backend = vcet_fixed_campaign_request(&[(6, 24)], 1);
        wrong_backend.optimization_template["backend"] =
            json!({ "backend": "Auto", "device_id": 0 });
        assert!(build_vcet_fixed_candidate_plan(
            &wrong_backend,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect_err("strict route requires explicit GpuKernel")
        .contains("require_gpu_kernel=true"));

        let decoded: ResearchCampaignExecutionMode =
            serde_json::from_str("\"evaluate_candidates\"").expect("deserialize mode");
        assert_eq!(decoded, ResearchCampaignExecutionMode::EvaluateCandidates);
    }

    #[test]
    fn vcet_strict_gpu_evaluate_candidates_rejects_auxiliary_cpu_stages() {
        let stages: [(&str, fn(&mut ResearchCampaignValidationConfig)); 8] = [
            ("walk_forward", |cfg| {
                cfg.walk_forward = Some(
                    serde_json::from_value(json!({})).expect("walk-forward defaults deserialize"),
                );
            }),
            ("reopt_walk_forward", |cfg| {
                cfg.reopt_walk_forward = Some(
                    serde_json::from_value(json!({}))
                        .expect("reopt walk-forward defaults deserialize"),
                );
            }),
            ("spa", |cfg| {
                cfg.spa =
                    Some(serde_json::from_value(json!({})).expect("SPA defaults deserialize"));
            }),
            ("perturb", |cfg| {
                cfg.perturb =
                    Some(serde_json::from_value(json!({})).expect("perturb defaults deserialize"));
            }),
            ("trade_bootstrap", |cfg| {
                cfg.trade_bootstrap = Some(
                    serde_json::from_value(json!({}))
                        .expect("trade bootstrap defaults deserialize"),
                );
            }),
            ("monte_carlo", |cfg| {
                cfg.monte_carlo = Some(
                    serde_json::from_value(json!({})).expect("Monte Carlo defaults deserialize"),
                );
            }),
            ("cross_ticker", |cfg| {
                cfg.cross_ticker = Some(
                    serde_json::from_value(json!({})).expect("cross-ticker defaults deserialize"),
                );
            }),
            ("cscv_pbo", |cfg| {
                cfg.cscv_pbo =
                    Some(serde_json::from_value(json!({})).expect("CSCV defaults deserialize"));
            }),
        ];

        for (stage, enable_stage) in stages {
            let mut req = vcet_fixed_campaign_request(&[(6, 24)], 1);
            enable_stage(&mut req.validation_config);
            let error = build_vcet_fixed_candidate_plan(
                &req,
                "Vcet",
                ResearchCampaignExecutionMode::EvaluateCandidates,
            )
            .expect_err("strict GPU fixed-candidate evaluation must reject CPU auxiliary work");
            assert!(error.contains("not proven fully GPU-resident"));
            assert!(error.contains(stage), "missing stage '{stage}' in: {error}");
        }
    }

    #[test]
    fn vcet_fixed_candidate_result_policy_fails_closed_on_non_kernel_route() {
        let req = vcet_fixed_campaign_request(&[(6, 24)], 1);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid plan")
        .expect("evaluate plan");
        let gpu =
            vcet_fixed_validation_result_value(&plan, "GpuKernel", "Gpu", "GpuKernel", &[1.25]);
        let accepted = parse_vcet_fixed_candidate_dataset_result(&gpu, &plan, true)
            .expect("exact GPU kernel route");
        assert!(accepted.gpu_kernel_used);

        let auto_gpu =
            vcet_fixed_validation_result_value(&plan, "Auto", "Gpu", "Auto->GpuKernel", &[1.25]);
        let advisory = parse_vcet_fixed_candidate_dataset_result(&auto_gpu, &plan, false)
            .expect("non-strict mode may use Auto->GpuKernel");
        assert!(advisory.gpu_kernel_used);
        assert!(parse_vcet_fixed_candidate_dataset_result(&auto_gpu, &plan, true).is_err());

        let fallback = vcet_fixed_validation_result_value(
            &plan,
            "GpuKernel",
            "Cpu",
            "GpuKernel->Cpu",
            &[1.25],
        );
        let error = parse_vcet_fixed_candidate_dataset_result(&fallback, &plan, true)
            .expect_err("strict fixed-candidate route must reject CPU fallback");
        assert!(error.contains("required the exact GpuKernel route"));
    }

    #[test]
    fn vcet_fixed_candidate_reuse_errors_fall_through_to_fresh_evaluation() {
        let req = vcet_fixed_campaign_request(&[(6, 24)], 1);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid plan")
        .expect("evaluate plan");
        let validation_res =
            vcet_fixed_validation_result_value(&plan, "GpuKernel", "Gpu", "GpuKernel", &[1.0]);
        let valid_artifact = json!({
            "validation": {
                "req": { "candidate_manifest_hash": plan.candidate_manifest_hash },
                "res": validation_res,
            }
        });
        assert!(matches!(
            evaluate_vcet_fixed_candidate_artifact_reuse(
                "C:/runs/valid.json",
                Ok(valid_artifact.clone()),
                &plan,
                true,
            ),
            CampaignFixedCandidateArtifactReuse::Reuse(_)
        ));

        for (path, loaded) in [
            (
                "C:/runs/corrupt.json",
                Err("artifact JSON was corrupt".to_string()),
            ),
            (
                "C:/runs/incompatible.json",
                Ok({
                    let mut artifact = valid_artifact.clone();
                    artifact["validation"]["req"]["candidate_manifest_hash"] =
                        Value::String("sha256:other-manifest".to_string());
                    artifact
                }),
            ),
            (
                "C:/runs/cpu-fallback.json",
                Ok({
                    let mut artifact = valid_artifact.clone();
                    artifact["validation"]["res"] = vcet_fixed_validation_result_value(
                        &plan,
                        "GpuKernel",
                        "Cpu",
                        "GpuKernel->Cpu",
                        &[1.0],
                    );
                    artifact
                }),
            ),
        ] {
            match evaluate_vcet_fixed_candidate_artifact_reuse(path, loaded, &plan, true) {
                CampaignFixedCandidateArtifactReuse::Rerun { warning } => {
                    assert!(warning.contains(path));
                    assert!(warning.contains("reran the dataset"));
                }
                CampaignFixedCandidateArtifactReuse::Reuse(_) => {
                    panic!("invalid artifact '{path}' must not suppress fresh evaluation")
                }
            }
        }
    }

    #[test]
    fn vcet_fixed_candidate_artifact_contract_rejects_tampered_result_rows() {
        let req = vcet_fixed_campaign_request(&[(6, 24), (12, 48)], 1);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid plan")
        .expect("evaluate plan");
        let contract = super::CampaignVcetFixedCandidateArtifactContract {
            plan: plan.clone(),
            require_gpu_kernel: true,
        };
        let valid = json!({
            "validation": {
                "req": { "candidate_manifest_hash": plan.candidate_manifest_hash },
                "res": vcet_fixed_validation_result_value(
                    &plan,
                    "GpuKernel",
                    "Gpu",
                    "GpuKernel",
                    &[1.0, 2.0],
                ),
            }
        });
        validate_vcet_fixed_candidate_artifact_contract(&valid, &contract)
            .expect("untampered artifact satisfies fixed-candidate contract");

        let mut cases: Vec<(&str, Value)> = Vec::new();

        let mut missing_row = valid.clone();
        missing_row
            .pointer_mut("/validation/res/candidates")
            .and_then(Value::as_array_mut)
            .expect("candidate rows")
            .pop();
        cases.push(("missing row", missing_row));

        let mut extra_row = valid.clone();
        let rows = extra_row
            .pointer_mut("/validation/res/candidates")
            .and_then(Value::as_array_mut)
            .expect("candidate rows");
        rows.push(rows[0].clone());
        cases.push(("extra row", extra_row));

        let mut duplicate_row = valid.clone();
        let rows = duplicate_row
            .pointer_mut("/validation/res/candidates")
            .and_then(Value::as_array_mut)
            .expect("candidate rows");
        rows[1] = rows[0].clone();
        cases.push(("duplicate row", duplicate_row));

        let mut unexpected_periods = valid.clone();
        unexpected_periods["validation"]["res"]["candidates"][0]["params"]["fast_len"] =
            Value::from(7_u64);
        cases.push(("unexpected periods", unexpected_periods));

        let mut altered_params = valid.clone();
        altered_params["validation"]["res"]["candidates"][0]["params"]
            .as_object_mut()
            .expect("candidate params")
            .insert("tampered".to_string(), Value::Bool(true));
        cases.push(("altered params", altered_params));

        let mut missing_metric = valid.clone();
        missing_metric["validation"]["res"]["candidates"][0]["in_sample"]
            .as_object_mut()
            .expect("in-sample metrics")
            .remove("sharpe");
        cases.push(("missing metric", missing_metric));

        let mut missing_verdict = valid.clone();
        missing_verdict["validation"]["res"]["candidates"][0]["validation_verdict"] = Value::Null;
        cases.push(("missing verdict", missing_verdict));

        let mut altered_robust_score = valid.clone();
        altered_robust_score["validation"]["res"]["candidates"][0]["robust_score"] =
            Value::from(999.0);
        cases.push(("altered robust score", altered_robust_score));

        let mut altered_pareto = valid.clone();
        let pareto = altered_pareto["validation"]["res"]["candidates"][0]["robust_pareto"]
            .as_bool()
            .expect("robust pareto");
        altered_pareto["validation"]["res"]["candidates"][0]["robust_pareto"] =
            Value::Bool(!pareto);
        cases.push(("altered robust pareto", altered_pareto));

        let mut missing_exposure = valid.clone();
        missing_exposure["validation"]["res"]["candidates"][0]["in_sample"]
            .as_object_mut()
            .expect("in-sample metrics")
            .remove("exposure");
        cases.push(("missing exposure", missing_exposure));

        let mut invalid_psr = valid.clone();
        invalid_psr["validation"]["res"]["candidates"][0]["psr"] = Value::from(1.5);
        cases.push(("invalid PSR probability", invalid_psr));

        let mut wrong_type_search_id = valid.clone();
        wrong_type_search_id["validation"]["req"]["candidate_search_id"] = Value::from(7_u64);
        cases.push(("legacy wrong-type search id", wrong_type_search_id));

        let mut partial_search_lineage = valid.clone();
        partial_search_lineage["validation"]["req"]["candidate_search_schema_version"] =
            Value::from(1_u64);
        cases.push(("legacy partial search lineage", partial_search_lineage));

        let mut injected_walk_forward = valid.clone();
        injected_walk_forward["validation"]["res"]["candidates"][0]["walk_forward"] =
            json!({ "sharpe_median": 100.0 });
        cases.push(("injected walk forward", injected_walk_forward));

        let mut wrong_route = valid.clone();
        wrong_route["validation"]["res"]["backend_route"] =
            Value::String("GpuKernel->Cpu".to_string());
        wrong_route["validation"]["res"]["backend_used"] = Value::String("Cpu".to_string());
        cases.push(("wrong route", wrong_route));

        let mut wrong_hash = valid;
        wrong_hash["validation"]["req"]["candidate_manifest_hash"] =
            Value::String("sha256:other".to_string());
        cases.push(("wrong hash", wrong_hash));

        for (label, artifact) in cases {
            assert!(
                validate_vcet_fixed_candidate_artifact_contract(&artifact, &contract).is_err(),
                "{label} must fail the exact campaign-to-Meta candidate contract"
            );
        }
    }

    #[test]
    fn vcet_fixed_candidate_request_envelope_is_exactly_bound() {
        let req = vcet_fixed_campaign_request(&[(6, 24), (12, 48)], 1);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid plan")
        .expect("evaluate plan");
        let contract = super::CampaignVcetFixedCandidateArtifactContract {
            plan: plan.clone(),
            require_gpu_kernel: true,
        };
        let valid = json!({
            "run": {
                "res": {
                    "evaluation_mode": "evaluate_candidates",
                    "candidate_manifest_hash": plan.candidate_manifest_hash,
                    "num_combos": plan.candidates.len(),
                }
            },
            "validation": {
                "req": {
                    "evaluation_mode": "evaluate_candidates",
                    "candidate_manifest_hash": plan.candidate_manifest_hash,
                    "require_gpu_kernel": true,
                    "candidates": plan.candidates,
                    "max_candidates": plan.candidates.len(),
                }
            }
        });
        validate_vcet_fixed_candidate_artifact_request_contract(&valid, &contract)
            .expect("exact request envelope");

        for tampered in [
            {
                let mut value = valid.clone();
                value["validation"]["req"]["require_gpu_kernel"] = Value::Bool(false);
                value
            },
            {
                let mut value = valid.clone();
                value["validation"]["req"]["candidates"] = json!([]);
                value
            },
            {
                let mut value = valid.clone();
                value["run"]["res"]["candidate_manifest_hash"] =
                    Value::String("sha256:foreign".to_string());
                value
            },
        ] {
            assert!(
                validate_vcet_fixed_candidate_artifact_request_contract(&tampered, &contract)
                    .is_err()
            );
        }
    }

    #[test]
    fn campaign_bound_meta_recomputes_admission_from_exact_training_artifacts() {
        let req = vcet_fixed_campaign_request(&[(6, 24)], 1);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid plan")
        .expect("evaluate plan");
        let item = ResearchCampaignItemSummary {
            item_id: Some("training-item".to_string()),
            csv_path: req.datasets[0].csv_path.clone(),
            label: "VCET_0".to_string(),
            timeframe: "1m".to_string(),
            is_holdout: false,
            status: "ok".to_string(),
            dataset_identity: None,
            evidence_state: Some(CampaignEvidenceState::Complete),
            resume_state: Some(CampaignResumeState::Complete),
            reuse_decision: None,
            artifact_path: Some("C:/runs/vcet-training.json".to_string()),
            artifact_sha256: Some("a".repeat(64)),
            report_xlsx_path: None,
            error: None,
            diagnostics: None,
            fixed_candidate_result: Some(CampaignFixedCandidateDatasetResult {
                gpu_kernel_used: true,
                candidates: vec![CampaignFixedCandidateObservation {
                    identity: plan.candidate_identities[0],
                    pnl: 1.0,
                    sharpe: 0.1,
                    max_drawdown: 0.1,
                    trades: 20,
                    hard_gate_passed: true,
                }],
            }),
        };
        let counts = ResearchCampaignCounts {
            total: 1,
            optimized: 0,
            validated: 1,
            skipped: 0,
            failed: 0,
            cancelled: false,
        };
        let summary = build_campaign_summary(
            "2026-07-31T12:00:00Z",
            "completed",
            "Vcet",
            Some("1m".to_string()),
            &req,
            ResearchCampaignExecutionMode::EvaluateCandidates,
            &req.tags,
            None,
            &counts,
            &[item],
            None,
            None,
            &[],
            1,
        );
        assert!(
            summary.items[0].fixed_candidate_result.is_none(),
            "persisted summary projections must not retain the hidden per-dataset candidate matrix"
        );
        assert!(summary.fixed_candidate_evaluation.is_some());
        let artifact = json!({
            "validation": {
                "req": { "candidate_manifest_hash": plan.candidate_manifest_hash },
                "res": vcet_fixed_validation_result_value(
                    &plan,
                    "GpuKernel",
                    "Gpu",
                    "GpuKernel",
                    &[1.0],
                ),
            }
        });
        validate_vcet_fixed_candidate_training_evidence(&summary, std::iter::once(&artifact))
            .expect("exact training artifact reproduces the campaign admission aggregate");
        let promoted = build_vcet_temporal_campaign_candidate_source(&summary)
            .expect("admitted campaign candidates promote into exact temporal identities");
        assert_eq!(
            promoted.sealed_candidate_manifest.payload.candidates.len(),
            summary
                .fixed_candidate_evaluation
                .as_ref()
                .expect("fixed candidate aggregate")
                .admitted_candidate_count
        );

        let mut contradicted = artifact;
        contradicted["validation"]["res"]["candidates"][0]["in_sample"]["pnl"] = Value::from(-5.0);
        contradicted["validation"]["res"]["candidates"][0]["in_sample"]["sharpe"] =
            Value::from(-0.5);
        contradicted["validation"]["res"]["candidates"][0]["robust_score"] = Value::from(-0.7);
        contradicted["validation"]["res"]["candidates"][0]["validation_verdict"]["passed"] =
            Value::Bool(false);
        assert!(validate_vcet_fixed_candidate_training_evidence(
            &summary,
            std::iter::once(&contradicted),
        )
        .expect_err("Meta must reject artifact evidence that contradicts the summary")
        .contains("does not match the exact referenced artifacts"));
    }

    #[test]
    fn vcet_fixed_candidate_summary_uses_all_requested_datasets_as_denominator() {
        let req = vcet_fixed_campaign_request(&[(6, 24), (12, 48)], 3);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid plan")
        .expect("evaluate plan");
        let make_result = |first_pnl: f64, second_pnl: f64| CampaignFixedCandidateDatasetResult {
            gpu_kernel_used: true,
            candidates: vec![
                CampaignFixedCandidateObservation {
                    identity: plan.candidate_identities[0],
                    pnl: first_pnl,
                    sharpe: first_pnl / 10.0,
                    max_drawdown: 0.10,
                    trades: 20,
                    hard_gate_passed: first_pnl > 0.0,
                },
                CampaignFixedCandidateObservation {
                    identity: plan.candidate_identities[1],
                    pnl: second_pnl,
                    sharpe: second_pnl / 10.0,
                    max_drawdown: 0.20,
                    trades: 10,
                    hard_gate_passed: second_pnl > 0.0,
                },
            ],
        };
        let items: Vec<ResearchCampaignItemSummary> = req
            .datasets
            .iter()
            .enumerate()
            .map(|(index, dataset)| ResearchCampaignItemSummary {
                item_id: Some(format!("item-{index}")),
                csv_path: dataset.csv_path.clone(),
                label: dataset.label.clone().unwrap_or_default(),
                timeframe: "1m".to_string(),
                is_holdout: false,
                status: if index < 2 { "ok" } else { "failed" }.to_string(),
                dataset_identity: None,
                evidence_state: Some(if index < 2 {
                    CampaignEvidenceState::Complete
                } else {
                    CampaignEvidenceState::Partial
                }),
                resume_state: Some(CampaignResumeState::Complete),
                reuse_decision: None,
                artifact_path: None,
                artifact_sha256: None,
                report_xlsx_path: None,
                error: None,
                diagnostics: None,
                fixed_candidate_result: match index {
                    0 => Some(make_result(1.0, -1.0)),
                    1 => Some(make_result(3.0, 2.0)),
                    _ => None,
                },
            })
            .collect();
        let aggregate = build_fixed_candidate_evaluation_summary(
            &req,
            ResearchCampaignExecutionMode::EvaluateCandidates,
            &items,
        )
        .expect("fixed-candidate aggregate");
        assert_eq!(aggregate.requested_datasets, 3);
        assert_eq!(aggregate.evaluated_datasets, 2);
        assert_eq!(aggregate.missing_or_failed_datasets, 1);
        assert_eq!(aggregate.gpu_kernel_datasets, 2);
        assert_eq!(aggregate.schema_version, 4);
        assert_eq!(aggregate.selection_cohort, "training");
        assert_eq!(
            aggregate.cross_universe_admission,
            ResearchCampaignCrossUniverseAdmissionPolicy::default()
        );
        assert_eq!(aggregate.admitted_candidate_count, 0);
        assert!(aggregate.admitted_candidate_ids.is_empty());
        assert_eq!(aggregate.selection_status, "no_candidate_met_admission");
        assert_eq!(aggregate.selected_candidate_id, None);
        assert_eq!(aggregate.all_universe.requested_datasets, 3);
        assert_eq!(aggregate.training.requested_datasets, 3);
        assert_eq!(aggregate.holdout.requested_datasets, 0);
        assert_eq!(aggregate.candidates[0].profitable_datasets, 2);
        assert_eq!(aggregate.candidates[0].missing_or_failed_datasets, 1);
        assert!((aggregate.candidates[0].profitable_share_requested - 2.0 / 3.0).abs() < 1e-12);
        assert_eq!(aggregate.candidates[0].pnl_p10, None);
        assert_eq!(aggregate.candidates[0].sharpe_p10, None);
        assert_eq!(aggregate.candidates[0].pnl_median, Some(2.0));
        assert_eq!(aggregate.candidates[1].profitable_datasets, 1);
        assert!((aggregate.candidates[1].profitable_share_requested - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn requested_cohort_percentile_fails_closed_at_missing_mass_boundary() {
        assert_eq!(
            super::finite_percentile_over_requested((1..=9).map(f64::from).collect(), 10, 0.10),
            None,
            "N=10 with one missing observation leaves p10 inside missing mass"
        );
        assert_eq!(
            super::finite_percentile_over_requested((1..=10).map(f64::from).collect(), 11, 0.10),
            Some(1.0),
            "N=11 with one missing observation places p10 at the first finite rank"
        );
        let interpolated = super::finite_percentile_over_requested(
            (1..=19).map(|value| f64::from(value) * 10.0).collect(),
            20,
            0.10,
        )
        .expect("p10 should land above the missing rank");
        assert!(
            (interpolated - 19.0).abs() <= 1e-12,
            "interpolation must subtract the missing-rank offset"
        );
        assert_eq!(
            super::finite_percentile_over_requested(vec![10.0, 20.0], 2, 0.10),
            Some(11.0),
            "a complete cohort preserves ordinary percentile interpolation"
        );
    }

    #[test]
    fn vcet_fixed_candidate_selection_fails_closed_when_relative_best_is_unprofitable() {
        let req = vcet_fixed_campaign_request(&[(6, 24), (12, 48)], 3);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid plan")
        .expect("evaluate plan");
        let items: Vec<ResearchCampaignItemSummary> = req
            .datasets
            .iter()
            .enumerate()
            .map(|(index, dataset)| ResearchCampaignItemSummary {
                item_id: Some(format!("item-{index}")),
                csv_path: dataset.csv_path.clone(),
                label: dataset.label.clone().unwrap_or_default(),
                timeframe: "1m".to_string(),
                is_holdout: false,
                status: "ok".to_string(),
                dataset_identity: None,
                evidence_state: Some(CampaignEvidenceState::Complete),
                resume_state: Some(CampaignResumeState::Complete),
                reuse_decision: None,
                artifact_path: None,
                artifact_sha256: None,
                report_xlsx_path: None,
                error: None,
                diagnostics: None,
                fixed_candidate_result: Some(CampaignFixedCandidateDatasetResult {
                    gpu_kernel_used: true,
                    candidates: vec![
                        CampaignFixedCandidateObservation {
                            identity: plan.candidate_identities[0],
                            pnl: -0.1 - index as f64,
                            sharpe: -0.01,
                            max_drawdown: 0.20,
                            trades: 20,
                            hard_gate_passed: false,
                        },
                        CampaignFixedCandidateObservation {
                            identity: plan.candidate_identities[1],
                            pnl: -10.0 - index as f64,
                            sharpe: -1.0,
                            max_drawdown: 0.40,
                            trades: 10,
                            hard_gate_passed: false,
                        },
                    ],
                }),
            })
            .collect();

        let aggregate = build_fixed_candidate_evaluation_summary(
            &req,
            ResearchCampaignExecutionMode::EvaluateCandidates,
            &items,
        )
        .expect("fixed-candidate aggregate");
        assert_eq!(aggregate.admitted_candidate_count, 0);
        assert!(aggregate.admitted_candidate_ids.is_empty());
        assert_eq!(aggregate.selected_candidate_id, None);
        assert_eq!(aggregate.selection_status, "no_candidate_met_admission");
    }

    #[test]
    fn vcet_cross_universe_admission_is_configurable_and_validated() {
        for (name, configure) in [
            (
                "profitable_share",
                (|policy: &mut ResearchCampaignCrossUniverseAdmissionPolicy| {
                    policy.min_profitable_share_requested = 1.01;
                }) as fn(&mut ResearchCampaignCrossUniverseAdmissionPolicy),
            ),
            ("hard_gate_share", |policy| {
                policy.min_hard_gate_pass_share_requested = -0.01;
            }),
            ("pnl_p10", |policy| {
                policy.min_pnl_p10 = f64::NAN;
            }),
            ("sharpe_p10", |policy| {
                policy.min_sharpe_p10 = f64::INFINITY;
            }),
        ] {
            let mut req = vcet_fixed_campaign_request(&[(6, 24)], 1);
            configure(&mut req.validation_config.cross_universe_admission);
            let error = build_vcet_fixed_candidate_plan(
                &req,
                "Vcet",
                ResearchCampaignExecutionMode::EvaluateCandidates,
            )
            .expect_err("invalid admission policy must fail before evaluation");
            assert!(
                error.contains("cross_universe_admission"),
                "{name}: {error}"
            );
        }
    }

    #[test]
    fn vcet_fixed_candidate_summary_separates_training_selection_from_conflicting_holdout() {
        let req = vcet_fixed_campaign_request(&[(6, 24), (12, 48)], 3);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid plan")
        .expect("evaluate plan");
        let result = |first_pnl: f64, second_pnl: f64| CampaignFixedCandidateDatasetResult {
            gpu_kernel_used: true,
            candidates: vec![
                CampaignFixedCandidateObservation {
                    identity: plan.candidate_identities[0],
                    pnl: first_pnl,
                    sharpe: first_pnl,
                    max_drawdown: 0.10,
                    trades: 10,
                    hard_gate_passed: first_pnl > 0.0,
                },
                CampaignFixedCandidateObservation {
                    identity: plan.candidate_identities[1],
                    pnl: second_pnl,
                    sharpe: second_pnl,
                    max_drawdown: 0.10,
                    trades: 10,
                    hard_gate_passed: second_pnl > 0.0,
                },
            ],
        };
        let items: Vec<ResearchCampaignItemSummary> = req
            .datasets
            .iter()
            .enumerate()
            .map(|(index, dataset)| ResearchCampaignItemSummary {
                item_id: Some(format!("item-{index}")),
                csv_path: dataset.csv_path.clone(),
                label: dataset.label.clone().unwrap_or_default(),
                timeframe: "1m".to_string(),
                is_holdout: index > 0,
                status: "ok".to_string(),
                dataset_identity: None,
                evidence_state: Some(CampaignEvidenceState::Complete),
                resume_state: Some(CampaignResumeState::Complete),
                reuse_decision: None,
                artifact_path: None,
                artifact_sha256: None,
                report_xlsx_path: None,
                error: None,
                diagnostics: None,
                fixed_candidate_result: Some(if index == 0 {
                    result(1.0, -1.0)
                } else {
                    result(-10.0, 10.0)
                }),
            })
            .collect();

        let aggregate = build_fixed_candidate_evaluation_summary(
            &req,
            ResearchCampaignExecutionMode::EvaluateCandidates,
            &items,
        )
        .expect("cohort-separated fixed-candidate aggregate");

        let training_winner = &aggregate.training.candidates[0];
        let holdout_winner = &aggregate.holdout.candidates[1];
        assert_eq!(training_winner.profitable_datasets, 1);
        assert_eq!(aggregate.training.candidates[1].profitable_datasets, 0);
        assert_eq!(holdout_winner.profitable_datasets, 2);
        assert_eq!(aggregate.holdout.candidates[0].profitable_datasets, 0);
        assert_eq!(aggregate.all_universe.candidates[1].profitable_datasets, 2);
        assert_eq!(aggregate.candidates[0].requested_datasets, 1);
        assert_eq!(aggregate.candidates[1].requested_datasets, 1);
        assert_eq!(
            aggregate.selected_candidate_id.as_deref(),
            Some(training_winner.candidate_id.as_str()),
            "holdout performance must never influence the selected training candidate"
        );
    }

    #[test]
    fn vcet_fixed_candidate_summary_never_selects_without_training_evidence() {
        let req = vcet_fixed_campaign_request(&[(6, 24)], 1);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid plan")
        .expect("evaluate plan");
        let items = vec![ResearchCampaignItemSummary {
            item_id: Some("holdout-only".to_string()),
            csv_path: req.datasets[0].csv_path.clone(),
            label: "VCET_0".to_string(),
            timeframe: "1m".to_string(),
            is_holdout: true,
            status: "ok".to_string(),
            dataset_identity: None,
            evidence_state: Some(CampaignEvidenceState::Complete),
            resume_state: Some(CampaignResumeState::Complete),
            reuse_decision: None,
            artifact_path: None,
            artifact_sha256: None,
            report_xlsx_path: None,
            error: None,
            diagnostics: None,
            fixed_candidate_result: Some(CampaignFixedCandidateDatasetResult {
                gpu_kernel_used: true,
                candidates: vec![CampaignFixedCandidateObservation {
                    identity: plan.candidate_identities[0],
                    pnl: 100.0,
                    sharpe: 10.0,
                    max_drawdown: 0.01,
                    trades: 100,
                    hard_gate_passed: true,
                }],
            }),
        }];

        let aggregate = build_fixed_candidate_evaluation_summary(
            &req,
            ResearchCampaignExecutionMode::EvaluateCandidates,
            &items,
        )
        .expect("holdout-only fixed-candidate aggregate");
        assert_eq!(aggregate.training.requested_datasets, 0);
        assert_eq!(aggregate.training.evaluated_datasets, 0);
        assert_eq!(aggregate.candidates[0].evaluated_datasets, 0);
        assert_eq!(aggregate.selected_candidate_id, None);
    }

    #[test]
    fn vcet_fixed_candidate_summary_resume_contract_binds_manifest_and_aggregate() {
        let req = vcet_fixed_campaign_request(&[(6, 24)], 1);
        let manifest = build_campaign_manifest(
            &req,
            "Vcet",
            Some("1m"),
            &req.tags,
            None,
            ResearchCampaignExecutionMode::EvaluateCandidates,
        );
        let dataset = &req.datasets[0];
        let items = vec![ResearchCampaignItemSummary {
            item_id: Some(campaign_item_id(
                &manifest.manifest_hash,
                0,
                &dataset.csv_path,
                "1m",
            )),
            csv_path: dataset.csv_path.clone(),
            label: dataset.label.clone().unwrap_or_default(),
            timeframe: "1m".to_string(),
            is_holdout: false,
            status: "pending".to_string(),
            dataset_identity: None,
            evidence_state: Some(CampaignEvidenceState::None),
            resume_state: Some(CampaignResumeState::Resumable),
            reuse_decision: None,
            artifact_path: None,
            artifact_sha256: None,
            report_xlsx_path: None,
            error: None,
            diagnostics: None,
            fixed_candidate_result: None,
        }];
        let counts = ResearchCampaignCounts {
            total: 1,
            optimized: 0,
            validated: 0,
            skipped: 0,
            failed: 0,
            cancelled: false,
        };
        let mut summary = serde_json::to_value(build_campaign_summary(
            "2026-07-31T12:00:00Z",
            "running",
            "Vcet",
            Some("1m".to_string()),
            &req,
            ResearchCampaignExecutionMode::EvaluateCandidates,
            &req.tags,
            None,
            &counts,
            &items,
            None,
            None,
            &[],
            1,
        ))
        .expect("serialize fixed-candidate summary");
        validate_campaign_summary_artifact_value(&summary)
            .expect("untampered fixed-candidate summary is resumable");

        let mut cancelled_unsealed = summary.clone();
        cancelled_unsealed["state"] = Value::String("cancelled".to_string());
        cancelled_unsealed["counts"]["cancelled"] = Value::Bool(true);
        validate_campaign_summary_artifact_value(&cancelled_unsealed)
            .expect("cancelled Meta-ineligible checkpoints may remain unsealed for fast resume");

        let mut missing_terminal_artifact = summary.clone();
        missing_terminal_artifact["state"] = Value::String("completed".to_string());
        missing_terminal_artifact["items"][0]["status"] = Value::String("ok".to_string());
        missing_terminal_artifact["items"][0]["evidence_state"] =
            Value::String("complete".to_string());
        missing_terminal_artifact["items"][0]["resume_state"] =
            Value::String("complete".to_string());
        assert!(
            validate_campaign_summary_artifact_value(&missing_terminal_artifact)
                .expect_err("a successful terminal item must reference its run artifact")
                .contains("no run-artifact path")
        );

        let mut unsealed_terminal_artifact = missing_terminal_artifact;
        unsealed_terminal_artifact["items"][0]["artifact_path"] =
            Value::String("C:/runs/vcet-unsealed.json".to_string());
        assert!(
            validate_campaign_summary_artifact_value(&unsealed_terminal_artifact)
                .expect_err("a terminal VCET item must seal its exact raw artifact hash")
                .contains("sealed raw run-artifact SHA-256")
        );

        let mut tampered_cohort = summary.clone();
        tampered_cohort["fixed_candidate_evaluation"]["training"]["requested_datasets"] =
            Value::from(99_u64);
        assert!(validate_campaign_summary_artifact_value(&tampered_cohort)
            .expect_err("tampered training cohort must fail")
            .contains("cohort"));

        let mut tampered_selection = summary.clone();
        tampered_selection["fixed_candidate_evaluation"]["selected_candidate_id"] =
            Value::String("arbitrary-without-training-evidence".to_string());
        assert!(
            validate_campaign_summary_artifact_value(&tampered_selection)
                .expect_err("tampered training selection must fail")
                .contains("cohort")
        );

        let mut downgraded_admission_schema = summary.clone();
        downgraded_admission_schema["fixed_candidate_evaluation"]["schema_version"] =
            Value::from(2_u64);
        assert!(
            validate_campaign_summary_artifact_value(&downgraded_admission_schema)
                .expect_err("an admission-aware request must not downgrade to legacy selection")
                .contains("aggregate")
        );

        let mut tampered_retention = summary.clone();
        tampered_retention["retention"]["cohort_id"] = Value::String("other".to_string());
        assert!(
            validate_campaign_summary_artifact_value(&tampered_retention)
                .expect_err("tampered retention lineage must fail")
                .contains("retention cohort")
        );

        summary["fixed_candidate_evaluation"]["candidate_manifest_hash"] =
            Value::String("sha256:tampered".to_string());
        assert!(validate_campaign_summary_artifact_value(&summary)
            .expect_err("tampered fixed-candidate aggregate must fail")
            .contains("aggregate"));
    }

    #[test]
    fn vcet_final_campaign_summary_drives_exact_meta_train_holdout_expectation() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let runs_dir = super::super::run_artifacts::runs_dir(&handle).expect("runs dir");
        let mut req = vcet_fixed_campaign_request(&[(6, 24)], 2);
        let plan = build_vcet_fixed_candidate_plan(
            &req,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )
        .expect("valid plan")
        .expect("evaluate plan");
        req.holdout_pct = 50;
        req.holdout_tag = Some("holdout".to_string());
        let dataset_paths: Vec<String> = req
            .datasets
            .iter()
            .map(|dataset| dataset.csv_path.clone())
            .collect();
        let (_, holdout_paths) = split_holdout_paths(&dataset_paths, 50, &req.tags[0]);
        assert_eq!(holdout_paths.len(), 1);

        let manifest = build_campaign_manifest(
            &req,
            "Vcet",
            Some("1m"),
            &req.tags,
            req.holdout_tag.as_deref(),
            ResearchCampaignExecutionMode::EvaluateCandidates,
        );
        let mut items: Vec<ResearchCampaignItemSummary> = req
            .datasets
            .iter()
            .enumerate()
            .map(|(index, dataset)| {
                let artifact_path =
                    runs_dir.join(format!("vectorgrid_run_vcet_meta_lineage_{index}.json"));
                ResearchCampaignItemSummary {
                    item_id: Some(campaign_item_id(
                        &manifest.manifest_hash,
                        index,
                        &dataset.csv_path,
                        "1m",
                    )),
                    csv_path: dataset.csv_path.clone(),
                    label: dataset.label.clone().unwrap_or_default(),
                    timeframe: "1m".to_string(),
                    is_holdout: holdout_paths.contains(&dataset.csv_path),
                    status: "ok".to_string(),
                    dataset_identity: Some(ResearchCampaignDatasetIdentitySummary {
                        csv_path: dataset.csv_path.clone(),
                        label: dataset.label.clone().unwrap_or_default(),
                        timeframe: "1m".to_string(),
                        base_timeframe: Some("1m".to_string()),
                        derived_timeframe: Some("1m".to_string()),
                        source_size_bytes: dataset.source_size_bytes,
                        source_mtime_utc_ms: dataset.source_mtime_utc_ms,
                        source_change_marker: dataset.source_change_marker,
                        source_content_fingerprint: dataset.source_content_fingerprint.clone(),
                    }),
                    evidence_state: Some(CampaignEvidenceState::Complete),
                    resume_state: Some(CampaignResumeState::Complete),
                    reuse_decision: None,
                    artifact_path: Some(artifact_path.to_string_lossy().to_string()),
                    artifact_sha256: None,
                    report_xlsx_path: None,
                    error: None,
                    diagnostics: None,
                    fixed_candidate_result: Some(CampaignFixedCandidateDatasetResult {
                        gpu_kernel_used: true,
                        candidates: vec![CampaignFixedCandidateObservation {
                            identity: plan.candidate_identities[0],
                            pnl: 1.0 + index as f64,
                            sharpe: 0.5 + index as f64,
                            max_drawdown: 0.1,
                            trades: 20,
                            hard_gate_passed: true,
                        }],
                    }),
                }
            })
            .collect();
        for (index, item) in items.iter().enumerate() {
            let artifact_path = PathBuf::from(
                item.artifact_path
                    .as_deref()
                    .expect("lineage fixture artifact path"),
            );
            std::fs::write(
                artifact_path,
                serde_json::to_vec_pretty(&json!({
                    "kind": "vectorgrid_run_artifact",
                    "fixture_index": index,
                }))
                .expect("encode lineage fixture artifact"),
            )
            .expect("write lineage fixture artifact");
        }
        seal_vcet_campaign_artifact_hashes(&mut items, &handle)
            .expect("seal exact raw lineage fixture hashes");
        assert!(items.iter().all(|item| item
            .artifact_sha256
            .as_deref()
            .is_some_and(|sha256| sha256.len() == 64)));
        let counts = ResearchCampaignCounts {
            total: items.len(),
            optimized: 0,
            validated: items.len(),
            skipped: 0,
            failed: 0,
            cancelled: false,
        };
        let mut summary = build_campaign_summary(
            "2026-07-31T12:00:00Z",
            "completed",
            "Vcet",
            Some("1m".to_string()),
            &req,
            ResearchCampaignExecutionMode::EvaluateCandidates,
            &req.tags,
            req.holdout_tag.clone(),
            &counts,
            &items,
            None,
            None,
            &[],
            1,
        );
        let expected_paths: Vec<PathBuf> = items
            .iter()
            .filter_map(|item| item.artifact_path.as_deref().map(PathBuf::from))
            .collect();
        let (summary_path, _) = save_campaign_summary_checkpoint_blocking(
            &mut summary,
            &mut items,
            None,
            &handle,
            None,
        )
        .expect("save final campaign summary");
        let meta_req: MetaSelectionRequest = serde_json::from_value(json!({
            "strategy_kind": "Vcet",
            "timeframe": "1m",
            "required_tag": req.tags[0],
            "eval_holdout": true,
            "holdout_tag": "holdout"
        }))
        .expect("decode Meta request");

        let expectation =
            load_meta_campaign_cohort_expectation(Some(&summary_path), &meta_req, &handle)
                .expect("load campaign-bound Meta expectation")
                .expect("campaign expectation");
        assert_eq!(expectation.lineage.expected_train_datasets, 1);
        assert_eq!(expectation.lineage.expected_holdout_datasets, 1);
        assert_eq!(expectation.lineage.admitted_candidate_count, 1);
        assert!(expectation.lineage.selected_candidate_id.is_some());
        assert_eq!(expectation.referenced_artifact_paths, expected_paths);

        let downgraded_path = runs_dir.join("vectorgrid_campaign_meta_downgraded.json");
        let mut downgraded_value: Value = serde_json::from_slice(
            &std::fs::read(&summary_path).expect("read saved campaign summary"),
        )
        .expect("decode saved campaign summary");
        downgraded_value["version"] = Value::from(1_u64);
        std::fs::write(
            &downgraded_path,
            serde_json::to_vec_pretty(&downgraded_value).expect("encode downgraded summary"),
        )
        .expect("write downgraded summary fixture");
        assert!(load_meta_campaign_cohort_expectation(
            downgraded_path.to_str(),
            &meta_req,
            &handle,
        )
        .expect_err("campaign-bound Meta must reject a legacy-version downgrade")
        .contains("integrity version 2"));

        let mut rejected_items = items.clone();
        for item in rejected_items.iter_mut().filter(|item| !item.is_holdout) {
            let result = item
                .fixed_candidate_result
                .as_mut()
                .expect("training fixed-candidate result");
            result.candidates[0].pnl = -1.0;
            result.candidates[0].sharpe = -0.5;
            result.candidates[0].hard_gate_passed = false;
        }
        let mut rejected_summary = build_campaign_summary(
            "2026-07-31T12:01:00Z",
            "completed",
            "Vcet",
            Some("1m".to_string()),
            &req,
            ResearchCampaignExecutionMode::EvaluateCandidates,
            &req.tags,
            req.holdout_tag.clone(),
            &counts,
            &rejected_items,
            None,
            None,
            &[],
            1,
        );
        let (rejected_summary_path, _) = save_campaign_summary_checkpoint_blocking(
            &mut rejected_summary,
            &mut rejected_items,
            None,
            &handle,
            None,
        )
        .expect("save no-admission campaign summary");
        assert!(load_meta_campaign_cohort_expectation(
            Some(&rejected_summary_path),
            &meta_req,
            &handle,
        )
        .expect_err("Meta must not bypass a campaign with no admitted VCET candidate")
        .contains("no VCET candidate met"));
    }

    fn make_plain_doublema_request(backend: Backend) -> DoubleMaRequest {
        DoubleMaRequest {
            backend,
            data_id: "campaign-test".to_string(),
            fast_range: (5, 5, 1),
            slow_range: (20, 20, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
            ma_source: "hl2".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            hurdle_tau: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(8),
            include_all: Some(false),
            heatmap_bins: None,
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
        }
    }

    fn sample_params(fast_len: u16, slow_len: u16) -> ta_app_core::DoubleMaParamsResolved {
        ta_app_core::DoubleMaParamsResolved {
            fast_len,
            slow_len,
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
        }
    }

    fn sample_metrics(sharpe: f64, calmar: f64, omega: f64, max_dd: f64) -> Metrics {
        Metrics {
            pnl: sharpe * 10.0,
            sharpe,
            sortino: sharpe,
            calmar,
            omega,
            max_dd,
            ..Metrics::default()
        }
    }

    fn sample_backend_result(
        top: Vec<(ta_app_core::DoubleMaParamsResolved, Metrics)>,
    ) -> ta_app_core::BackendOptimizationResult {
        let (best_params, best_metrics) = top
            .first()
            .cloned()
            .unwrap_or_else(|| (sample_params(5, 20), Metrics::default()));
        ta_app_core::BackendOptimizationResult {
            best_params,
            best_metrics,
            best_score: None,
            mode_used: ta_app_core::OptimizationModeResolved::Grid,
            backend_used: ta_app_core::BackendUsed::Cpu,
            matrix_index: None,
            best_col: None,
            top_cols: None,
            num_combos: top.len(),
            num_candles: 1000,
            runtime_ms: 10,
            top,
            top_scores: None,
            all: None,
            all_scores: None,
            heatmap: None,
            vdps: None,
            hybrid_timing_ms: None,
            gpu_kernel_timing_ms: None,
            backend_route: None,
            backend_fallback_reason: None,
            route_diagnostics: None,
            indicator_route_plan: None,
            residency: None,
            followup_cache_key: None,
            shortlist_slice_diagnostics: None,
            score_observation_context: None,
            metric_surface_kind: None,
            shortlist_union_telemetry: None,
            search_semantics: None,
        }
    }

    #[test]
    fn optimization_trial_count_uses_reported_backend_and_composite_search_counts() {
        let mut backend = sample_backend_result(vec![(
            sample_params(5, 20),
            sample_metrics(1.0, 1.0, 1.0, 1.0),
        )]);
        backend.num_combos = 58_300;
        assert_eq!(
            optimization_trials_from_result(&CampaignOptimizationResult::Backend(backend)),
            Some(58_300)
        );

        let composite = ta_app_core::CompositeMoResult {
            data_id: "campaign-test".to_string(),
            pareto: Vec::new(),
            metric_surface_kind: None,
            search_semantics: None,
            total_evaluations: 4_096,
            runtime_ms: 0,
            telemetry: ta_app_core::composite_mo::CompositeMoTelemetry {
                prescan: None,
                search: ta_app_core::composite_mo::CompositeMoSearchTelemetry {
                    sampler: "test".to_string(),
                    requested_evaluations: 4_096,
                    actual_evaluations: 4_096,
                    unique_evaluations: 4_000,
                    duplicate_evaluations_avoided: 96,
                    feasible_points: 3_900,
                    rejected_decode_or_order: 50,
                    rejected_prescan_pair: 25,
                    rejected_other_constraints: 25,
                    exact_grid_used: false,
                    exact_grid_total_combinations: None,
                    post_search_trade22_points: 0,
                    post_search_trade22_runtime_ms: 0,
                },
            },
        };
        assert_eq!(
            optimization_trials_from_result(&CampaignOptimizationResult::CompositeMo(composite)),
            Some(4_096)
        );

        let artifact = CampaignOptimizationResult::CompositeMoArtifact(json!({
            "total_evaluations": 0,
            "telemetry": {
                "search": {
                    "actual_evaluations": 2_048
                }
            }
        }));
        assert_eq!(optimization_trials_from_result(&artifact), Some(2_048));
        assert_eq!(
            optimization_trials_from_result(&CampaignOptimizationResult::CompositeMoArtifact(
                json!({
                    "total_evaluations": 0
                })
            )),
            None
        );
    }

    #[test]
    fn campaign_validation_context_preserves_full_search_distribution() {
        let summary = ta_optimizer::SearchDistributionSummary {
            metric_surface_kind: "core17".to_string(),
            sharpe_metric_id: ta_optimizer::DOUBLE_MA_SHARPE_METRIC_ID.to_string(),
            attempted_count: 100,
            unique_count: 90,
            feasible_count: 80,
            finite_sharpe_count: 75,
            finite_sharpe_mean: Some(0.2),
            finite_sharpe_sample_std: Some(0.4),
        };
        let mut backend = sample_backend_result(vec![(
            sample_params(5, 20),
            sample_metrics(1.0, 1.0, 1.0, 1.0),
        )]);
        backend.search_semantics = Some(ta_app_core::OptimizationSearchSemanticsSummary {
            approx_mode: "exact_exhaustive".to_string(),
            workload_family: "test".to_string(),
            contract_summary: "test".to_string(),
            declared_domain: None,
            base_ranker: None,
            rerank_operator: None,
            heatmap_semantics: None,
            search_distribution: Some(summary.clone()),
            search_distribution_unavailable_reason: None,
        });
        let from_backend =
            search_distribution_from_result(&CampaignOptimizationResult::Backend(backend));
        let from_artifact = search_distribution_from_result(
            &CampaignOptimizationResult::CompositeMoArtifact(json!({
                "search_semantics": {
                    "search_distribution": summary
                }
            })),
        );

        assert_eq!(from_backend, from_artifact);
        assert_eq!(
            from_backend
                .as_ref()
                .map(|distribution| distribution.finite_sharpe_count),
            Some(75)
        );
    }

    #[test]
    fn doublema_vram_prefetch_config_accepts_exact_kernel_and_learned_auto() {
        let exact = make_plain_doublema_request(Backend::GpuKernel { device_id: 7 });
        assert_eq!(
            build_doublema_vram_prefetch_config(&exact, None).map(
                |CampaignDoubleMaVramPrefetchConfig {
                     device_id,
                     fast_ma_type,
                     slow_ma_type,
                     ma_source,
                 }| (device_id, fast_ma_type, slow_ma_type, ma_source)
            ),
            Some((7, "sma".to_string(), "ema".to_string(), "hl2".to_string()))
        );

        let auto = make_plain_doublema_request(Backend::Auto { device_id: 99 });
        assert_eq!(build_doublema_vram_prefetch_config(&auto, None), None);
        assert_eq!(
            build_doublema_vram_prefetch_config(&auto, Some(3))
                .map(|CampaignDoubleMaVramPrefetchConfig { device_id, .. }| device_id),
            Some(3)
        );
    }

    #[test]
    fn doublema_vram_prefetch_config_rejects_non_plain_shapes() {
        let mut multi_ma = make_plain_doublema_request(Backend::GpuKernel { device_id: 0 });
        multi_ma.fast_ma_types.push("alma".to_string());
        assert_eq!(build_doublema_vram_prefetch_config(&multi_ma, None), None);

        let cpu = make_plain_doublema_request(Backend::CpuOnly);
        assert_eq!(build_doublema_vram_prefetch_config(&cpu, Some(0)), None);
    }

    #[test]
    fn requested_gpu_device_id_tracks_gpu_capable_backends_only() {
        assert_eq!(requested_gpu_device_id(&Backend::CpuOnly), None);
        assert_eq!(
            requested_gpu_device_id(&Backend::Auto { device_id: 2 }),
            Some(2)
        );
        assert_eq!(
            requested_gpu_device_id(&Backend::GpuKernel { device_id: 4 }),
            Some(4)
        );
    }

    #[test]
    fn push_unique_warning_deduplicates_identical_entries() {
        let mut warnings = vec!["existing".to_string()];
        push_unique_warning(&mut warnings, "existing".to_string());
        push_unique_warning(&mut warnings, String::new());
        push_unique_warning(&mut warnings, "new".to_string());
        assert_eq!(warnings, vec!["existing".to_string(), "new".to_string()]);
    }

    #[test]
    fn record_prefetch_feedback_keeps_observational_outcomes_explicit() {
        let mut warnings = Vec::new();
        let mut disabled = false;

        let not_attempted = record_prefetch_feedback(&mut warnings, &mut disabled, None);
        assert_eq!(not_attempted.outcome, "not_attempted");
        assert_eq!(not_attempted.note, None);
        assert_eq!(not_attempted.wall_ms, 0.0);
        assert_eq!(not_attempted.policy, "heuristic_observational");
        assert!(!not_attempted.attempted);
        assert!(!not_attempted.success);
        assert!(!not_attempted.failure_risk_event);
        assert!(!disabled);
        assert!(warnings.is_empty());

        let staged = record_prefetch_feedback(
            &mut warnings,
            &mut disabled,
            Some(DoubleMaCampaignVramPrefetchFeedback::Staged {
                stage_wall_ms: 12.5,
            }),
        );
        assert_eq!(staged.outcome, "staged");
        assert_eq!(staged.note, None);
        assert_eq!(staged.wall_ms, 12.5);
        assert_eq!(staged.policy, "heuristic_observational");
        assert!(staged.attempted);
        assert!(staged.success);
        assert!(!staged.failure_risk_event);
        assert!(!disabled);
        assert!(warnings.is_empty());
    }

    #[test]
    fn record_prefetch_feedback_surfaces_warning_and_fatal_without_claiming_optimality() {
        let mut warnings = Vec::new();
        let mut disabled = false;

        let warning = record_prefetch_feedback(
            &mut warnings,
            &mut disabled,
            Some(DoubleMaCampaignVramPrefetchFeedback::Warning {
                note: "staging skipped after measured warning".to_string(),
                stage_wall_ms: 8.0,
            }),
        );
        assert_eq!(warning.outcome, "warning");
        assert_eq!(
            warning.note.as_deref(),
            Some("staging skipped after measured warning")
        );
        assert_eq!(warning.wall_ms, 8.0);
        assert_eq!(warning.policy, "heuristic_observational");
        assert!(warning.attempted);
        assert!(!warning.success);
        assert!(warning.failure_risk_event);
        assert!(!disabled);
        assert_eq!(
            warnings,
            vec!["staging skipped after measured warning".to_string()]
        );

        let fatal = record_prefetch_feedback(
            &mut warnings,
            &mut disabled,
            Some(DoubleMaCampaignVramPrefetchFeedback::Fatal {
                note: "staging disabled after measured failure".to_string(),
                stage_wall_ms: 9.5,
            }),
        );
        assert_eq!(fatal.outcome, "fatal");
        assert_eq!(
            fatal.note.as_deref(),
            Some("staging disabled after measured failure")
        );
        assert_eq!(fatal.wall_ms, 9.5);
        assert_eq!(fatal.policy, "heuristic_observational");
        assert!(fatal.attempted);
        assert!(!fatal.success);
        assert!(fatal.failure_risk_event);
        assert!(disabled);
        assert_eq!(
            warnings,
            vec![
                "staging skipped after measured warning".to_string(),
                "staging disabled after measured failure".to_string()
            ]
        );
    }

    #[test]
    fn build_item_diagnostics_labels_prefetch_economics_as_heuristic_observational() {
        let diagnostics = build_item_diagnostics(
            DatasetPrepareSource::SyncPrepare,
            20.0,
            &CampaignPrefetchDiagnostics {
                outcome: "staged",
                note: None,
                wall_ms: 5.0,
                policy: "heuristic_observational",
                attempted: true,
                success: true,
                failure_risk_event: false,
            },
            Some(10.0),
            Some(2.5),
            Some(1.0),
        );

        assert_eq!(
            diagnostics.vram_prefetch_policy.as_deref(),
            Some("heuristic_observational")
        );
        assert_eq!(diagnostics.vram_prefetch_attempted, Some(true));
        assert_eq!(diagnostics.vram_prefetch_success, Some(true));
        assert_eq!(diagnostics.vram_prefetch_failure_risk_event, Some(false));
        assert_eq!(
            diagnostics.vram_prefetch_stage_cost_vs_optimize_frac,
            Some(0.5)
        );
        assert_eq!(
            diagnostics.vram_prefetch_stage_cost_vs_prepare_wait_frac,
            Some(0.25)
        );
    }

    #[test]
    fn validate_only_always_loads_existing_artifacts_even_without_skip_existing() {
        assert!(should_load_existing_artifacts(
            false,
            ResearchCampaignExecutionMode::ValidateOnly
        ));
        assert!(should_load_existing_artifacts(
            true,
            ResearchCampaignExecutionMode::Full
        ));
        assert!(!should_load_existing_artifacts(
            false,
            ResearchCampaignExecutionMode::Full
        ));
    }

    #[test]
    fn existing_artifact_listing_failure_is_not_treated_as_empty_reuse_set() {
        let err =
            load_existing_campaign_artifacts(true, ResearchCampaignExecutionMode::Full, || {
                Err("catalog unavailable".to_string())
            })
            .expect_err("listing failures should fail closed");
        assert!(err.contains("failed to load existing run artifacts"));
        assert!(err.contains("catalog unavailable"));

        let mut called = false;
        let existing =
            load_existing_campaign_artifacts(false, ResearchCampaignExecutionMode::Full, || {
                called = true;
                Ok(Vec::new())
            })
            .expect("skip disabled should not load artifacts");
        assert!(existing.is_empty());
        assert!(!called);
    }

    #[test]
    fn normalize_campaign_datasets_trims_fields_and_rejects_blank_paths() {
        let err = normalize_campaign_datasets(vec![ResearchCampaignDatasetInput {
            csv_path: "   ".to_string(),
            label: None,
            timeframe: None,
            source_size_bytes: None,
            source_mtime_utc_ms: None,
            source_change_marker: None,
            source_content_fingerprint: None,
        }])
        .expect_err("blank csv_path should be rejected");
        assert!(err.contains("csv_path is empty"));

        let normalized = normalize_campaign_datasets(vec![ResearchCampaignDatasetInput {
            csv_path: "  C:/data/AAPL.csv  ".to_string(),
            label: Some("  AAPL  ".to_string()),
            timeframe: Some(" 15M ".to_string()),
            source_size_bytes: Some(1),
            source_mtime_utc_ms: Some(2),
            source_change_marker: Some(3),
            source_content_fingerprint: Some("  fp-1  ".to_string()),
        }])
        .expect("normalize datasets");
        assert_eq!(normalized[0].csv_path, "C:/data/AAPL.csv");
        assert_eq!(normalized[0].label.as_deref(), Some("AAPL"));
        assert_eq!(normalized[0].timeframe.as_deref(), Some("15m"));
        assert_eq!(
            normalized[0].source_content_fingerprint.as_deref(),
            Some("fp-1")
        );

        let duplicate_err = normalize_campaign_datasets(vec![
            ResearchCampaignDatasetInput {
                csv_path: "C:/data/./AAPL.csv".to_string(),
                label: Some("AAPL primary".to_string()),
                timeframe: Some("1m".to_string()),
                source_size_bytes: None,
                source_mtime_utc_ms: None,
                source_change_marker: None,
                source_content_fingerprint: None,
            },
            ResearchCampaignDatasetInput {
                csv_path: "c:\\DATA\\nested\\..\\aapl.CSV".to_string(),
                label: Some("AAPL alias".to_string()),
                timeframe: Some("1m".to_string()),
                source_size_bytes: None,
                source_mtime_utc_ms: None,
                source_change_marker: None,
                source_content_fingerprint: None,
            },
        ])
        .expect_err("alias-equivalent csv paths should be rejected");
        assert!(duplicate_err.contains("duplicate normalized csv_path"));

        let duplicate_fingerprint_err = normalize_campaign_datasets(vec![
            ResearchCampaignDatasetInput {
                csv_path: "C:/data/AAPL-copy-1.csv".to_string(),
                label: Some("AAPL copy 1".to_string()),
                timeframe: Some("1m".to_string()),
                source_size_bytes: None,
                source_mtime_utc_ms: None,
                source_change_marker: None,
                source_content_fingerprint: Some("FNV1A64-FULL-AABBCCDD".to_string()),
            },
            ResearchCampaignDatasetInput {
                csv_path: "C:/data/AAPL-copy-2.csv".to_string(),
                label: Some("AAPL copy 2".to_string()),
                timeframe: Some("1m".to_string()),
                source_size_bytes: None,
                source_mtime_utc_ms: None,
                source_change_marker: None,
                source_content_fingerprint: Some("fnv1a64-full-aabbccdd".to_string()),
            },
        ])
        .expect_err("same-content campaign inputs should be rejected before execution");
        assert!(duplicate_fingerprint_err.contains("same source_content_fingerprint"));

        let distinct_paths_without_fingerprints = normalize_campaign_datasets(vec![
            ResearchCampaignDatasetInput {
                csv_path: "C:/data/AAPL.csv".to_string(),
                label: None,
                timeframe: Some("1m".to_string()),
                source_size_bytes: None,
                source_mtime_utc_ms: None,
                source_change_marker: None,
                source_content_fingerprint: None,
            },
            ResearchCampaignDatasetInput {
                csv_path: "C:/data/MSFT.csv".to_string(),
                label: None,
                timeframe: Some("1m".to_string()),
                source_size_bytes: None,
                source_mtime_utc_ms: None,
                source_change_marker: None,
                source_content_fingerprint: None,
            },
        ])
        .expect("missing discovery fingerprints are verified after preparation");
        assert_eq!(distinct_paths_without_fingerprints.len(), 2);
    }

    #[test]
    fn prepared_campaign_sources_reject_same_content_at_runtime() {
        let first = prepared_dataset_for_matching();
        let mut second = prepared_dataset_for_matching();
        second.input.csv_path = "C:/data/AAPL-copy.csv".to_string();
        second.dataset_meta["csv_path"] = Value::String(second.input.csv_path.clone());

        let mut seen = HashMap::new();
        record_unique_prepared_campaign_source(&first, &mut seen)
            .expect("first source identity should be recorded");
        let error = record_unique_prepared_campaign_source(&second, &mut seen)
            .expect_err("freshly prepared same-content copies must be rejected");
        assert!(error.contains("same source content identity"));
        assert!(error.contains("duplicate source content cannot be counted twice"));
    }

    #[test]
    fn prepare_campaign_dataset_rejects_stale_discovery_fingerprint_and_persists_current_value() {
        let temp_dir = make_temp_dir("campaign_dataset_fingerprint");
        let cache_dir = temp_dir.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");
        let csv_path = temp_dir.join("AAPL.csv");
        std::fs::write(
            &csv_path,
            concat!(
                "timestamp,open,high,low,close,volume\n",
                "1700000000000,100,101,99,100.5,1000\n",
                "1700000060000,100.5,102,100,101.5,1100\n"
            ),
        )
        .expect("write discovered csv");
        let discovered_fingerprint = crate::data_cache::content_fingerprint_for_source(&csv_path)
            .expect("fingerprint discovered csv");

        std::fs::write(
            &csv_path,
            concat!(
                "timestamp,open,high,low,close,volume\n",
                "1700000000000,110,111,109,110.5,1001\n",
                "1700000060000,110.5,112,110,111.5,1101\n"
            ),
        )
        .expect("mutate csv after discovery");
        let current_fingerprint = crate::data_cache::content_fingerprint_for_source(&csv_path)
            .expect("fingerprint current csv");
        let current_identity =
            crate::data_cache::identity_for_source(&csv_path).expect("identity current csv");
        assert_ne!(discovered_fingerprint, current_fingerprint);

        let stale_error = prepare_campaign_dataset(
            ResearchCampaignDatasetInput {
                csv_path: csv_path.to_string_lossy().into_owned(),
                label: Some("AAPL".to_string()),
                timeframe: Some("1m".to_string()),
                source_size_bytes: None,
                source_mtime_utc_ms: None,
                source_change_marker: None,
                source_content_fingerprint: Some(discovered_fingerprint.clone()),
            },
            None,
            &cache_dir,
        )
        .err()
        .expect("stale discovery fingerprint must fail closed");
        assert!(stale_error.contains("source_content_fingerprint is stale"));
        assert!(stale_error.contains(&discovered_fingerprint));
        assert!(stale_error.contains(&current_fingerprint));

        let stale_size_error = prepare_campaign_dataset(
            ResearchCampaignDatasetInput {
                csv_path: csv_path.to_string_lossy().into_owned(),
                label: Some("AAPL".to_string()),
                timeframe: Some("1m".to_string()),
                source_size_bytes: Some(current_identity.size.saturating_add(1)),
                source_mtime_utc_ms: Some(current_identity.mtime_utc_ms),
                source_change_marker: Some(current_identity.change_marker),
                source_content_fingerprint: Some(current_fingerprint.clone()),
            },
            None,
            &cache_dir,
        )
        .err()
        .expect("stale discovery size must fail closed");
        assert!(stale_size_error.contains("source_size_bytes is stale"));

        let stale_mtime_error = prepare_campaign_dataset(
            ResearchCampaignDatasetInput {
                csv_path: csv_path.to_string_lossy().into_owned(),
                label: Some("AAPL".to_string()),
                timeframe: Some("1m".to_string()),
                source_size_bytes: Some(current_identity.size),
                source_mtime_utc_ms: Some(current_identity.mtime_utc_ms.saturating_add(1)),
                source_change_marker: Some(current_identity.change_marker),
                source_content_fingerprint: Some(current_fingerprint.clone()),
            },
            None,
            &cache_dir,
        )
        .err()
        .expect("stale discovery mtime must fail closed");
        assert!(stale_mtime_error.contains("source_mtime_utc_ms is stale"));

        let prepared = prepare_campaign_dataset(
            ResearchCampaignDatasetInput {
                csv_path: csv_path.to_string_lossy().into_owned(),
                label: Some("AAPL".to_string()),
                timeframe: Some("1m".to_string()),
                source_size_bytes: None,
                source_mtime_utc_ms: None,
                source_change_marker: None,
                source_content_fingerprint: None,
            },
            None,
            &cache_dir,
        )
        .expect("current source should prepare without a stale hint");
        assert_eq!(
            prepared
                .dataset_meta
                .get("source_content_fingerprint")
                .and_then(Value::as_str),
            Some(current_fingerprint.as_str())
        );
        assert_eq!(
            prepared.input.source_content_fingerprint.as_deref(),
            Some(current_fingerprint.as_str())
        );
        assert_eq!(
            prepared.input.source_size_bytes,
            Some(current_identity.size)
        );
        assert_eq!(
            prepared.input.source_mtime_utc_ms,
            Some(current_identity.mtime_utc_ms)
        );
        assert_eq!(
            prepared.input.source_change_marker,
            Some(current_identity.change_marker)
        );

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn campaign_manifest_refreshes_missing_source_hints_before_hashing() {
        let temp_dir = make_temp_dir("campaign_manifest_fresh_identity");
        let csv_path = temp_dir.join("AAPL.csv");
        std::fs::write(
            &csv_path,
            concat!(
                "timestamp,open,high,low,close,volume\n",
                "1700000000000,100,101,99,100.5,1000\n"
            ),
        )
        .expect("write initial csv");

        let make_request = || ResearchCampaignRequest {
            strategy_kind: "DoubleMa".to_string(),
            datasets: vec![ResearchCampaignDatasetInput {
                csv_path: csv_path.to_string_lossy().into_owned(),
                label: Some("AAPL".to_string()),
                timeframe: Some("1m".to_string()),
                source_size_bytes: None,
                source_mtime_utc_ms: None,
                source_change_marker: None,
                source_content_fingerprint: None,
            }],
            timeframe_override: Some("1m".to_string()),
            tags: vec!["fresh-identity".to_string()],
            holdout_pct: 0,
            holdout_tag: None,
            skip_existing: false,
            auto_meta: false,
            auto_stage5: false,
            execution_mode: ResearchCampaignExecutionMode::OptimizeOnly,
            optimization_template: json!({
                "strategy_kind": "DoubleMa",
                "data_id": "fresh-identity"
            }),
            validation_config: ResearchCampaignValidationConfig::default(),
            meta_request: None,
            stage5_request: None,
            preload_policy: ResearchCampaignPreloadPolicy::Disabled,
        };

        let mut first = make_request();
        refresh_campaign_dataset_source_identities(&mut first.datasets)
            .expect("refresh initial source identity");
        assert!(first.datasets[0].source_size_bytes.is_some());
        assert!(first.datasets[0].source_mtime_utc_ms.is_some());
        assert!(first.datasets[0].source_change_marker.is_some());
        let first_manifest = build_campaign_manifest(
            &first,
            "DoubleMa",
            Some("1m"),
            &first.tags,
            None,
            ResearchCampaignExecutionMode::OptimizeOnly,
        );
        assert!(first_manifest.strict_identity_required);

        std::fs::OpenOptions::new()
            .append(true)
            .open(&csv_path)
            .and_then(|mut file| {
                use std::io::Write as _;
                file.write_all(b"1700000060000,100.5,102,100,101.5,1100\n")
            })
            .expect("mutate csv after first manifest");
        let mut second = make_request();
        refresh_campaign_dataset_source_identities(&mut second.datasets)
            .expect("refresh changed source identity");
        let second_manifest = build_campaign_manifest(
            &second,
            "DoubleMa",
            Some("1m"),
            &second.tags,
            None,
            ResearchCampaignExecutionMode::OptimizeOnly,
        );
        assert_ne!(
            first_manifest.dataset_universe_hash, second_manifest.dataset_universe_hash,
            "a source change discovered without request hints must create a new cohort"
        );
        assert_ne!(first_manifest.manifest_hash, second_manifest.manifest_hash);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn campaign_manifest_preserves_fingerprinted_legacy_hash_but_binds_marker_without_fingerprint()
    {
        let mut fingerprinted = vcet_fixed_campaign_request(&[(6, 24)], 1);
        let with_marker = build_campaign_manifest(
            &fingerprinted,
            "Vcet",
            Some("1m"),
            &fingerprinted.tags,
            None,
            ResearchCampaignExecutionMode::EvaluateCandidates,
        );
        fingerprinted.datasets[0].source_change_marker = None;
        let legacy_without_marker = build_campaign_manifest(
            &fingerprinted,
            "Vcet",
            Some("1m"),
            &fingerprinted.tags,
            None,
            ResearchCampaignExecutionMode::EvaluateCandidates,
        );
        assert_eq!(
            with_marker.dataset_universe_hash,
            legacy_without_marker.dataset_universe_hash
        );
        assert_eq!(
            with_marker.manifest_hash,
            legacy_without_marker.manifest_hash
        );
        assert!(legacy_without_marker.strict_identity_required);

        fingerprinted.datasets[0].source_content_fingerprint = None;
        let marker_absent = build_campaign_manifest(
            &fingerprinted,
            "Vcet",
            Some("1m"),
            &fingerprinted.tags,
            None,
            ResearchCampaignExecutionMode::EvaluateCandidates,
        );
        assert!(
            marker_absent.strict_identity_required,
            "pre-marker size+mtime identity remains a valid legacy v2 contract"
        );

        let mut legacy_summary = resumable_summary_value();
        legacy_summary["request"]["datasets"][0]
            .as_object_mut()
            .expect("legacy request dataset")
            .remove("source_change_marker");
        legacy_summary["request"]["datasets"][0]
            .as_object_mut()
            .expect("legacy request dataset")
            .remove("source_content_fingerprint");
        let legacy_request: ResearchCampaignRequest =
            serde_json::from_value(legacy_summary["request"].clone())
                .expect("decode pre-marker size+mtime request");
        let legacy_manifest = build_campaign_manifest(
            &legacy_request,
            "DoubleMa",
            Some("15m"),
            &legacy_request.tags,
            None,
            ResearchCampaignExecutionMode::OptimizeOnly,
        );
        assert!(legacy_manifest.strict_identity_required);
        let legacy_manifest_hash = legacy_manifest.manifest_hash.clone();
        legacy_summary["manifest"] =
            serde_json::to_value(&legacy_manifest).expect("serialize legacy manifest");
        legacy_summary["retention"]["cohort_id"] = Value::String(legacy_manifest_hash.clone());
        legacy_summary["items"][0]["item_id"] = Value::String(campaign_item_id(
            &legacy_manifest_hash,
            0,
            "C:/data/AAPL.csv",
            "15m",
        ));
        validate_campaign_summary_artifact_value(&legacy_summary)
            .expect("pre-marker size+mtime-only v2 summary remains loadable for restart guidance");

        fingerprinted.datasets[0].source_change_marker = Some(42);
        let marker_bound = build_campaign_manifest(
            &fingerprinted,
            "Vcet",
            Some("1m"),
            &fingerprinted.tags,
            None,
            ResearchCampaignExecutionMode::EvaluateCandidates,
        );
        assert!(marker_bound.strict_identity_required);
        assert_ne!(
            marker_absent.dataset_universe_hash,
            marker_bound.dataset_universe_hash
        );
        assert_ne!(marker_absent.manifest_hash, marker_bound.manifest_hash);
    }

    #[test]
    fn request_summary_preserves_dataset_source_identity() {
        let req = ResearchCampaignRequest {
            strategy_kind: "DoubleMa".to_string(),
            datasets: vec![ResearchCampaignDatasetInput {
                csv_path: "C:/data/AAPL.csv".to_string(),
                label: Some("AAPL".to_string()),
                timeframe: Some("1m".to_string()),
                source_size_bytes: Some(123),
                source_mtime_utc_ms: Some(456),
                source_change_marker: Some(789),
                source_content_fingerprint: Some("sha256:aapl".to_string()),
            }],
            timeframe_override: Some("1m".to_string()),
            tags: vec!["campaign-main".to_string()],
            holdout_pct: 0,
            holdout_tag: None,
            skip_existing: false,
            auto_meta: false,
            auto_stage5: false,
            execution_mode: ResearchCampaignExecutionMode::Full,
            optimization_template: json!({"strategy_kind": "DoubleMa", "data_id": "data-1"}),
            validation_config: ResearchCampaignValidationConfig::default(),
            meta_request: None,
            stage5_request: None,
            preload_policy: ResearchCampaignPreloadPolicy::HostOnly,
        };

        let summary = build_request_summary_value(
            &req,
            &["campaign-main".to_string()],
            ResearchCampaignExecutionMode::Full,
        );

        assert_eq!(
            summary
                .pointer("/datasets/0/source_size_bytes")
                .and_then(Value::as_u64),
            Some(123)
        );
        assert_eq!(
            summary
                .pointer("/datasets/0/source_mtime_utc_ms")
                .and_then(Value::as_i64),
            Some(456)
        );
        assert_eq!(
            summary
                .pointer("/datasets/0/source_change_marker")
                .and_then(Value::as_i64),
            Some(789)
        );
        assert_eq!(
            summary
                .pointer("/datasets/0/source_content_fingerprint")
                .and_then(Value::as_str),
            Some("sha256:aapl")
        );
    }

    #[test]
    fn existing_optimization_match_uses_prepared_dataset_identity() {
        let prepared = prepared_dataset_for_matching();
        let existing = vec![run_artifact_summary("C:/runs/opt.json", false)];
        let mut warnings = Vec::new();

        let matched = existing_optimization_match(
            &existing,
            "DoubleMa",
            "cfg-1",
            "campaign-main",
            &prepared,
            &mut warnings,
        )
        .expect("should match using prepared dataset identity");

        assert_eq!(matched.summary.path, "C:/runs/opt.json");
        assert!(warnings.is_empty());
    }

    #[test]
    fn existing_validation_match_returns_the_validation_artifact_summary() {
        let prepared = prepared_dataset_for_matching();
        let existing = vec![
            run_artifact_summary("C:/runs/opt_only.json", false),
            run_artifact_summary("C:/runs/validated.json", true),
        ];
        let mut warnings = Vec::new();

        let matched = existing_validation_match(
            &existing,
            "DoubleMa",
            "cfg-1",
            "campaign-main",
            &prepared,
            &mut warnings,
        )
        .expect("expected validation match");

        assert_eq!(matched.summary.path, "C:/runs/validated.json");
        assert!(matched.summary.has_validation);
        assert!(warnings.is_empty());
    }

    #[test]
    fn artifact_reuse_fingerprint_and_index_reject_cohort_role_or_tag_changes() {
        let prepared = prepared_dataset_for_matching();
        let campaign_tags = vec!["campaign-main".to_string()];
        let training_tags = campaign_artifact_tags(&campaign_tags, "AAPL", false, Some("holdout"));
        let holdout_tags = campaign_artifact_tags(&campaign_tags, "AAPL", true, Some("holdout"));
        let changed_holdout_tags =
            campaign_artifact_tags(&campaign_tags, "AAPL", true, Some("oos"));
        let training_fingerprint = campaign_artifact_config_fingerprint(
            "cfg-base",
            &training_tags,
            false,
            Some("holdout"),
        );
        let holdout_fingerprint =
            campaign_artifact_config_fingerprint("cfg-base", &holdout_tags, true, Some("holdout"));
        let changed_holdout_fingerprint = campaign_artifact_config_fingerprint(
            "cfg-base",
            &changed_holdout_tags,
            true,
            Some("oos"),
        );
        assert_ne!(training_fingerprint, holdout_fingerprint);
        assert_ne!(holdout_fingerprint, changed_holdout_fingerprint);
        let mut first_campaign_tags = training_tags.clone();
        first_campaign_tags.push(super::campaign_manifest_cohort_tag("sha256:first"));
        let mut second_campaign_tags = training_tags.clone();
        second_campaign_tags.push(super::campaign_manifest_cohort_tag("sha256:second"));
        assert_ne!(
            campaign_artifact_config_fingerprint(
                "cfg-base",
                &first_campaign_tags,
                false,
                Some("holdout"),
            ),
            campaign_artifact_config_fingerprint(
                "cfg-base",
                &second_campaign_tags,
                false,
                Some("holdout"),
            ),
            "a new manifest cohort must rerun instead of borrowing an artifact owned by an older retained summary"
        );

        let holdout_artifact = build_run_artifact_value(
            "2026-07-31T12:00:00Z",
            &holdout_fingerprint,
            "campaign-manifest-1",
            true,
            Some("holdout"),
            &prepared.input.csv_path,
            &prepared.dataset_meta,
            &holdout_tags,
            &json!({"strategy_kind": "DoubleMa"}),
            &json!({}),
            None,
            None,
        );
        assert_eq!(
            holdout_artifact
                .pointer("/campaign/cohort_role")
                .and_then(Value::as_str),
            Some("holdout")
        );
        assert_eq!(
            holdout_artifact
                .pointer("/campaign/manifest_hash")
                .and_then(Value::as_str),
            Some("campaign-manifest-1")
        );
        assert_eq!(
            holdout_artifact
                .pointer("/campaign/cohort_tag")
                .and_then(Value::as_str),
            Some(super::campaign_manifest_cohort_tag("campaign-manifest-1").as_str())
        );

        let mut artifact = run_artifact_summary("C:/runs/train.json", true);
        artifact.config_fingerprint = training_fingerprint.clone();
        artifact.tags = training_tags.clone();
        let artifacts = vec![artifact];
        let index = CampaignArtifactReuseIndex::new(&artifacts);
        assert_eq!(
            index
                .exact_candidate_indices(
                    "DoubleMa",
                    &training_fingerprint,
                    &training_tags,
                    &prepared,
                )
                .len(),
            1
        );
        assert!(index
            .exact_candidate_indices("DoubleMa", &holdout_fingerprint, &holdout_tags, &prepared,)
            .is_empty());
        assert!(index
            .exact_candidate_indices(
                "DoubleMa",
                &changed_holdout_fingerprint,
                &changed_holdout_tags,
                &prepared,
            )
            .is_empty());
    }

    #[test]
    fn campaign_artifact_reuse_index_keeps_ten_thousand_exact_lookups_linear() {
        const DATASETS: usize = 10_000;
        let tags = vec!["campaign-scale".to_string()];
        let prepared: Vec<PreparedCampaignDataset> = (0..DATASETS)
            .map(|index| {
                prepared_dataset_for_resume(
                    &format!("S{index:05}"),
                    1_000 + index as u64,
                    1_720_000_000_000 + index as i64,
                    &format!("sha256:scale-{index}"),
                )
            })
            .collect();
        let artifacts: Vec<super::run_artifacts::RunArtifactSummary> = prepared
            .iter()
            .enumerate()
            .map(|(index, dataset)| {
                let mut artifact =
                    run_artifact_summary(&format!("C:/runs/scale-{index}.json"), true);
                artifact.config_fingerprint = "cfg-scale".to_string();
                artifact.csv_path = dataset.input.csv_path.clone();
                artifact.timeframe = dataset.timeframe.clone();
                artifact.source_size_bytes = dataset.input.source_size_bytes;
                artifact.source_mtime_utc_ms = dataset.input.source_mtime_utc_ms;
                artifact.source_content_fingerprint =
                    dataset.input.source_content_fingerprint.clone();
                artifact.tags = tags.clone();
                artifact
            })
            .collect();
        let index = CampaignArtifactReuseIndex::new(&artifacts);
        let inspected: usize = prepared
            .iter()
            .map(|dataset| {
                index
                    .exact_candidate_indices("DoubleMa", "cfg-scale", &tags, dataset)
                    .len()
            })
            .sum();
        assert_eq!(inspected, DATASETS);
        assert_eq!(index.coarse.len(), DATASETS);
    }

    #[test]
    fn campaign_scale_policies_bound_full_snapshots_and_retention_scans() {
        const DATASETS: usize = 10_000;
        assert_eq!(campaign_checkpoint_stride(DATASETS), 100);
        assert_eq!(planned_campaign_full_summary_builds(DATASETS), 102);
        assert!(planned_campaign_full_summary_builds(DATASETS) < DATASETS / 50);
        for mode in [
            ResearchCampaignExecutionMode::Full,
            ResearchCampaignExecutionMode::OptimizeOnly,
            ResearchCampaignExecutionMode::ValidateOnly,
            ResearchCampaignExecutionMode::EvaluateCandidates,
        ] {
            assert_eq!(
                planned_campaign_run_artifact_prunes("Vcet", mode, DATASETS),
                1,
                "every VCET batch mode should pay for one final retention pass"
            );
        }
        assert_eq!(
            planned_campaign_run_artifact_prunes(
                "DoubleMa",
                ResearchCampaignExecutionMode::Full,
                DATASETS,
            ),
            DATASETS,
            "non-VCET campaign retention behavior remains unchanged"
        );
    }

    #[test]
    fn holdout_tag_must_be_disjoint_from_every_base_tag_and_dataset_label() {
        let datasets = vec![ResearchCampaignDatasetInput {
            csv_path: "C:/data/AAPL.csv".to_string(),
            label: Some("AAPL".to_string()),
            timeframe: Some("1m".to_string()),
            ..ResearchCampaignDatasetInput::default()
        }];
        let base_tag_error = validate_holdout_tag_isolation(
            &["primary".to_string(), "HOLDOUT".to_string()],
            &datasets,
            20,
            Some("holdout"),
        )
        .expect_err("any base-tag collision must be rejected");
        assert!(base_tag_error.contains("base campaign tags"));

        let mut colliding_label = datasets;
        colliding_label[0].label = Some("holdout".to_string());
        let label_error = validate_holdout_tag_isolation(
            &["primary".to_string()],
            &colliding_label,
            20,
            Some("HOLDOUT"),
        )
        .expect_err("dataset-label collision must be rejected for legacy readers");
        assert!(label_error.contains("dataset label"));
    }

    #[test]
    fn artifact_match_classifier_fails_closed_without_exact_source_identity() {
        let prepared = prepared_dataset_for_matching();
        let exact = run_artifact_summary("C:/runs/exact.json", true);
        assert_eq!(
            classify_run_artifact_for_campaign(&exact, "DoubleMa", "cfg-1", &exact.tags, &prepared,),
            CampaignArtifactMatchDecision::Exact
        );

        let mut stale = exact.clone();
        stale.source_content_fingerprint = Some("sha256:other".to_string());
        assert_eq!(
            classify_run_artifact_for_campaign(&stale, "DoubleMa", "cfg-1", &exact.tags, &prepared,),
            CampaignArtifactMatchDecision::Stale
        );

        let mut legacy = exact.clone();
        legacy.source_content_fingerprint = None;
        legacy.source_size_bytes = None;
        legacy.source_mtime_utc_ms = None;
        assert_eq!(
            classify_run_artifact_for_campaign(
                &legacy,
                "DoubleMa",
                "cfg-1",
                &exact.tags,
                &prepared,
            ),
            CampaignArtifactMatchDecision::LegacyDegraded
        );

        let unsafe_prepared = PreparedCampaignDataset {
            input: ResearchCampaignDatasetInput {
                csv_path: "C:/data/AAPL.csv".to_string(),
                label: Some("AAPL".to_string()),
                timeframe: Some("15m".to_string()),
                source_size_bytes: None,
                source_mtime_utc_ms: None,
                source_change_marker: None,
                source_content_fingerprint: None,
            },
            candles: vector_ta::utilities::data_loader::Candles::new(
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            dataset_meta: json!({
                "csv_path": "C:/data/AAPL.csv",
                "timeframe": "15m"
            }),
            timeframe: "15m".to_string(),
            label: "AAPL".to_string(),
        };
        assert_eq!(
            classify_run_artifact_for_campaign(
                &exact,
                "DoubleMa",
                "cfg-1",
                &exact.tags,
                &unsafe_prepared,
            ),
            CampaignArtifactMatchDecision::Unsafe
        );
    }

    #[test]
    fn save_campaign_summary_checkpoint_records_timing_without_second_full_rewrite() {
        let app = build_mock_app();
        let summary_path = make_temp_dir("campaign_summary_checkpoint").join("summary.json");
        let mut items = vec![ResearchCampaignItemSummary {
            item_id: Some("item-1".to_string()),
            csv_path: "C:/data/AAPL.csv".to_string(),
            label: "AAPL".to_string(),
            timeframe: "15m".to_string(),
            is_holdout: false,
            status: "ok".to_string(),
            dataset_identity: None,
            evidence_state: Some(CampaignEvidenceState::Complete),
            resume_state: Some(CampaignResumeState::Complete),
            reuse_decision: None,
            artifact_path: Some("C:/runs/validated.json".to_string()),
            artifact_sha256: Some("c".repeat(64)),
            report_xlsx_path: None,
            error: None,
            diagnostics: Some(ResearchCampaignItemDiagnostics {
                prepare_source: Some("sync_prepare".to_string()),
                prepare_wait_ms: Some(1.0),
                vram_prefetch_outcome: Some("not_attempted".to_string()),
                vram_prefetch_note: None,
                vram_prefetch_wall_ms: Some(0.0),
                vram_prefetch_policy: Some("heuristic_observational".to_string()),
                vram_prefetch_attempted: Some(false),
                vram_prefetch_success: Some(false),
                vram_prefetch_failure_risk_event: Some(false),
                vram_prefetch_stage_cost_vs_optimize_frac: Some(0.0),
                vram_prefetch_stage_cost_vs_prepare_wait_frac: Some(0.0),
                optimize_wall_ms: Some(2.0),
                optimize_artifact_save_ms: Some(3.0),
                checkpoint_save_ms: None,
            }),
            fixed_candidate_result: None,
        }];
        let mut summary = ResearchCampaignSummary {
            version: 2,
            kind: "vectorgrid_campaign_summary".to_string(),
            ts: "2026-04-15T00:00:00Z".to_string(),
            state: "running".to_string(),
            retention: None,
            manifest: None,
            evidence_state: Some(CampaignEvidenceState::Complete),
            resume_state: Some(CampaignResumeState::Complete),
            strategy_kind: "DoubleMa".to_string(),
            timeframe: Some("15m".to_string()),
            tags: vec!["campaign-main".to_string()],
            holdout_tag: None,
            holdout_pct: 0,
            request: json!({}),
            counts: ResearchCampaignCounts {
                total: 1,
                optimized: 1,
                validated: 1,
                skipped: 0,
                failed: 0,
                cancelled: false,
            },
            items: items.clone(),
            fixed_candidate_evaluation: None,
            meta_json_path: None,
            stage5_json_path: None,
            warnings: Vec::new(),
            runtime_ms: 42,
        };

        let _ = save_campaign_summary_checkpoint_blocking(
            &mut summary,
            &mut items,
            Some(0),
            &app.handle().clone(),
            Some(summary_path.to_string_lossy().as_ref()),
        )
        .expect("save summary with checkpoint timing");

        let checkpoint = items[0]
            .diagnostics
            .as_ref()
            .and_then(|diagnostics| diagnostics.checkpoint_save_ms);
        assert!(checkpoint.is_some());
        assert!(checkpoint.unwrap_or_default() >= 0.0);

        let saved: Value = serde_json::from_slice(
            &std::fs::read(&summary_path).expect("summary json should exist on disk"),
        )
        .expect("summary json should parse");
        assert_eq!(
            saved
                .pointer("/items/0/diagnostics/checkpoint_save_ms")
                .and_then(Value::as_f64),
            None,
            "the timing observed after publication must wait for the next bounded/final snapshot instead of causing a second full rewrite"
        );
        assert_eq!(
            saved
                .pointer("/items/0/diagnostics/vram_prefetch_policy")
                .and_then(Value::as_str),
            Some("heuristic_observational")
        );
    }

    #[test]
    fn failed_campaign_summary_replacement_preserves_previous_valid_summary() {
        let temp_dir = make_temp_dir("campaign_summary_failed_replace");
        let summary_path = temp_dir.join("summary.json");
        let previous = br#"{"version":2,"state":"running","items":[{"status":"ok"}]}"#;
        let replacement = br#"{"version":2,"state":"complete","items":[]}"#;
        std::fs::write(&summary_path, previous).expect("write previous valid summary");

        let mut staged_path = None;
        let error = write_campaign_summary_bytes_with_publish(
            &summary_path,
            replacement,
            |staged, final_path, target_exists| {
                assert!(target_exists, "the existing summary must be replaced");
                assert_eq!(final_path, summary_path.as_path());
                assert_eq!(
                    std::fs::read(staged).expect("read complete staged summary"),
                    replacement
                );
                staged_path = Some(staged.to_path_buf());
                Err("injected atomic replacement failure".to_string())
            },
        )
        .expect_err("injected replacement failure must propagate");

        assert!(error.contains("injected atomic replacement failure"));
        let preserved = std::fs::read(&summary_path).expect("previous summary must survive");
        assert_eq!(preserved, previous);
        let parsed: Value =
            serde_json::from_slice(&preserved).expect("preserved summary must remain valid JSON");
        assert_eq!(
            parsed.pointer("/items/0/status").and_then(Value::as_str),
            Some("ok")
        );
        assert!(
            !staged_path
                .expect("publish attempt must observe the staged path")
                .exists(),
            "failed replacement must remove its staged temp file"
        );

        std::fs::remove_dir_all(temp_dir).expect("remove test temp dir");
    }

    #[cfg(windows)]
    #[test]
    fn locked_campaign_summary_replacement_preserves_existing_file_and_cleans_temp() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;

        let temp_dir = make_temp_dir("campaign_summary_locked_replace");
        let summary_path = temp_dir.join("summary.json");
        let previous = br#"{"version":2,"state":"running"}"#;
        std::fs::write(&summary_path, previous).expect("write previous campaign summary");
        let locked_summary = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&summary_path)
            .expect("open campaign summary without delete sharing");

        let error =
            write_campaign_summary_bytes_atomically(&summary_path, br#"{"state":"complete"}"#)
                .expect_err("locked target must reject atomic replacement");
        assert!(error.contains("failed to atomically replace campaign summary"));
        assert_eq!(
            std::fs::read(&summary_path).expect("previous summary must remain readable"),
            previous
        );
        let remaining_paths = std::fs::read_dir(&temp_dir)
            .expect("list summary directory")
            .map(|entry| entry.expect("read summary directory entry").path())
            .collect::<Vec<_>>();
        assert_eq!(
            remaining_paths,
            vec![summary_path.clone()],
            "failed OS-level replacement must remove its staged temp file"
        );

        drop(locked_summary);
        std::fs::remove_dir_all(temp_dir).expect("remove test temp dir");
    }

    #[test]
    fn campaign_summary_atomic_replace_leaves_no_temp_artifacts() {
        let temp_dir = make_temp_dir("campaign_summary_atomic_replace");
        let summary_path = temp_dir.join("summary.json");
        std::fs::write(&summary_path, br#"{"state":"running"}"#).expect("write initial summary");

        let replacement = br#"{"state":"complete"}"#;
        write_campaign_summary_bytes_atomically(&summary_path, replacement)
            .expect("atomically replace campaign summary");

        assert_eq!(
            std::fs::read(&summary_path).expect("read replacement summary"),
            replacement
        );
        let remaining_paths = std::fs::read_dir(&temp_dir)
            .expect("list summary directory")
            .map(|entry| entry.expect("read summary directory entry").path())
            .collect::<Vec<_>>();
        assert_eq!(
            remaining_paths,
            vec![summary_path],
            "successful replacement must consume the staged temp file"
        );

        std::fs::remove_dir_all(temp_dir).expect("remove test temp dir");
    }

    #[test]
    fn cancelled_checkpoint_resumes_from_fresh_state_once_and_second_resume_is_noop() {
        const STRATEGY_KIND: &str = "DoubleMa";
        const PRIMARY_TAG: &str = "campaign-resume";
        const CONFIG_FINGERPRINT: &str = "cfg-resume-v1";

        let temp_dir = make_temp_dir("campaign_cancel_restart_resume");
        let summary_path = temp_dir.join("campaign-summary.json");
        let artifact_catalog_path = temp_dir.join("artifact-catalog.json");
        let artifact_dir = temp_dir.join("runs");
        let prepared = vec![
            prepared_dataset_for_resume("AAPL", 2_048, 1_712_000_000_000, "sha256:aapl"),
            prepared_dataset_for_resume("MSFT", 4_096, 1_712_000_000_001, "sha256:msft"),
        ];
        let tags = vec![PRIMARY_TAG.to_string()];
        let req = ResearchCampaignRequest {
            strategy_kind: STRATEGY_KIND.to_string(),
            datasets: prepared
                .iter()
                .map(|dataset| dataset.input.clone())
                .collect(),
            timeframe_override: Some("15m".to_string()),
            tags: tags.clone(),
            holdout_pct: 0,
            holdout_tag: None,
            skip_existing: true,
            auto_meta: false,
            auto_stage5: false,
            execution_mode: ResearchCampaignExecutionMode::OptimizeOnly,
            optimization_template: json!({
                "strategy_kind": STRATEGY_KIND,
                "data_id": "campaign_resume_template",
            }),
            validation_config: ResearchCampaignValidationConfig::default(),
            meta_request: None,
            stage5_request: None,
            preload_policy: ResearchCampaignPreloadPolicy::Disabled,
        };
        let manifest = build_campaign_manifest(
            &req,
            STRATEGY_KIND,
            Some("15m"),
            &tags,
            None,
            ResearchCampaignExecutionMode::OptimizeOnly,
        );
        let manifest_hash = manifest.manifest_hash.clone();
        let expected_item_ids = pending_resume_items(&manifest_hash, &prepared)
            .into_iter()
            .map(|item| item.item_id.expect("campaign item id"))
            .collect::<Vec<_>>();
        assert_ne!(
            expected_item_ids[0], expected_item_ids[1],
            "different dataset identities must not share a campaign item id"
        );

        // Phase 1 models a bounded cancellation at the same item boundary used
        // by the production campaign loop. The durable run artifact is published
        // before the cancelled summary checkpoint.
        let cancel = AtomicBool::new(false);
        let mut executed_item_ids = Vec::new();
        let mut artifacts = Vec::new();
        let mut items = pending_resume_items(&manifest_hash, &prepared);
        let mut counts = ResearchCampaignCounts {
            total: prepared.len(),
            optimized: 0,
            validated: 0,
            skipped: 0,
            failed: 0,
            cancelled: false,
        };
        for (index, dataset) in prepared.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                counts.cancelled = true;
                break;
            }
            let mut warnings = Vec::new();
            assert!(
                existing_optimization_match(
                    &artifacts,
                    STRATEGY_KIND,
                    CONFIG_FINGERPRINT,
                    PRIMARY_TAG,
                    dataset,
                    &mut warnings,
                )
                .is_none(),
                "a missing identity must execute exactly once"
            );
            let item_id = items[index].item_id.clone().expect("phase-one item id");
            let artifact = resume_artifact_summary(&artifact_dir, &item_id, dataset);
            write_test_json_atomically(std::path::Path::new(&artifact.path), &artifact);
            complete_resume_item(&mut items[index], "optimized", artifact.path.clone(), None);
            artifacts.push(artifact);
            executed_item_ids.push(item_id);
            counts.optimized += 1;
            cancel.store(true, Ordering::Relaxed);
        }
        if cancel.load(Ordering::Relaxed) {
            counts.cancelled = true;
        }
        assert_eq!(counts.optimized, 1);
        assert!(counts.cancelled);

        let cancelled_summary = build_campaign_summary(
            "2026-07-24T18:00:00Z",
            "cancelled",
            STRATEGY_KIND,
            Some("15m".to_string()),
            &req,
            ResearchCampaignExecutionMode::OptimizeOnly,
            &tags,
            None,
            &counts,
            &items,
            None,
            None,
            &[],
            10,
        );
        write_test_json_atomically(&artifact_catalog_path, &artifacts);
        write_test_json_atomically(&summary_path, &cancelled_summary);

        let checkpoint: ResearchCampaignSummary = read_test_json(&summary_path);
        assert_eq!(checkpoint.version, 2);
        assert_eq!(checkpoint.kind, "vectorgrid_campaign_summary");
        assert_eq!(checkpoint.state, "cancelled");
        assert_eq!(checkpoint.counts.optimized, 1);
        assert!(checkpoint.counts.cancelled);
        assert_eq!(
            checkpoint
                .manifest
                .as_ref()
                .map(|value| value.manifest_hash.as_str()),
            Some(manifest_hash.as_str())
        );
        assert_eq!(
            checkpoint.evidence_state,
            Some(CampaignEvidenceState::Partial)
        );
        assert_eq!(
            checkpoint.resume_state,
            Some(CampaignResumeState::Resumable)
        );
        assert_eq!(checkpoint.items[0].status, "optimized");
        assert_eq!(checkpoint.items[1].status, "pending");
        assert_eq!(
            checkpoint.items[0].item_id.as_deref(),
            Some(expected_item_ids[0].as_str())
        );
        assert_eq!(
            checkpoint.items[1].item_id.as_deref(),
            Some(expected_item_ids[1].as_str())
        );

        // Phase 2 deliberately reconstructs every orchestration value from the
        // request and the durable artifact catalog. It does not reuse the prior
        // process's items, counters, cancel token, or in-memory artifact vector.
        let mut resumed_artifacts: Vec<super::run_artifacts::RunArtifactSummary> =
            read_test_json(&artifact_catalog_path);
        let mut resumed_items = pending_resume_items(&manifest_hash, &prepared);
        let mut resumed_counts = ResearchCampaignCounts {
            total: prepared.len(),
            optimized: 0,
            validated: 0,
            skipped: 0,
            failed: 0,
            cancelled: false,
        };
        for (index, dataset) in prepared.iter().enumerate() {
            let mut warnings = Vec::new();
            let existing_path = existing_optimization_match(
                &resumed_artifacts,
                STRATEGY_KIND,
                CONFIG_FINGERPRINT,
                PRIMARY_TAG,
                dataset,
                &mut warnings,
            )
            .map(|matched| matched.summary.path.clone());
            if let Some(existing_path) = existing_path {
                resumed_counts.skipped += 1;
                complete_resume_item(
                    &mut resumed_items[index],
                    "skipped",
                    existing_path,
                    Some("exact_optimization_artifact".to_string()),
                );
                continue;
            }

            let item_id = resumed_items[index]
                .item_id
                .clone()
                .expect("resumed item id");
            let artifact = resume_artifact_summary(&artifact_dir, &item_id, dataset);
            assert!(
                !std::path::Path::new(&artifact.path).exists(),
                "a missing identity must not collide with an existing artifact path"
            );
            write_test_json_atomically(std::path::Path::new(&artifact.path), &artifact);
            complete_resume_item(
                &mut resumed_items[index],
                "optimized",
                artifact.path.clone(),
                None,
            );
            resumed_artifacts.push(artifact);
            executed_item_ids.push(item_id);
            resumed_counts.optimized += 1;
        }
        assert_eq!(resumed_counts.optimized, 1);
        assert_eq!(resumed_counts.skipped, 1);
        assert_eq!(resumed_items[0].status, "skipped");
        assert_eq!(resumed_items[1].status, "optimized");

        let completed_summary = build_campaign_summary(
            "2026-07-24T18:00:01Z",
            "completed",
            STRATEGY_KIND,
            Some("15m".to_string()),
            &req,
            ResearchCampaignExecutionMode::OptimizeOnly,
            &tags,
            None,
            &resumed_counts,
            &resumed_items,
            None,
            None,
            &[],
            20,
        );
        write_test_json_atomically(&artifact_catalog_path, &resumed_artifacts);
        write_test_json_atomically(&summary_path, &completed_summary);
        let catalog_bytes_after_resume =
            std::fs::read(&artifact_catalog_path).expect("read completed artifact catalog");

        let completed_checkpoint: ResearchCampaignSummary = read_test_json(&summary_path);
        assert_eq!(completed_checkpoint.state, "completed");
        assert_eq!(
            completed_checkpoint.evidence_state,
            Some(CampaignEvidenceState::Complete)
        );
        assert_eq!(
            completed_checkpoint.resume_state,
            Some(CampaignResumeState::Complete)
        );
        assert_eq!(
            completed_checkpoint
                .items
                .iter()
                .map(|item| item.item_id.as_deref().expect("completed item id"))
                .collect::<Vec<_>>(),
            expected_item_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );

        let unique_artifact_paths = resumed_artifacts
            .iter()
            .map(|artifact| artifact.path.as_str())
            .collect::<std::collections::HashSet<_>>();
        let unique_source_identities = resumed_artifacts
            .iter()
            .map(|artifact| {
                artifact
                    .source_content_fingerprint
                    .as_deref()
                    .expect("strict artifact source identity")
            })
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(resumed_artifacts.len(), prepared.len());
        assert_eq!(unique_artifact_paths.len(), prepared.len());
        assert_eq!(unique_source_identities.len(), prepared.len());
        for expected_id in &expected_item_ids {
            assert_eq!(
                executed_item_ids
                    .iter()
                    .filter(|executed| *executed == expected_id)
                    .count(),
                1,
                "each stable campaign item identity must execute exactly once"
            );
        }

        // Phase 3 is another fresh reconstruction. Every strict identity must
        // now resolve to skip-existing, leaving the durable catalog byte-identical.
        let second_resume_artifacts: Vec<super::run_artifacts::RunArtifactSummary> =
            read_test_json(&artifact_catalog_path);
        let mut second_resume_items = pending_resume_items(&manifest_hash, &prepared);
        let mut second_resume_counts = ResearchCampaignCounts {
            total: prepared.len(),
            optimized: 0,
            validated: 0,
            skipped: 0,
            failed: 0,
            cancelled: false,
        };
        let execution_count_before_second_resume = executed_item_ids.len();
        for (index, dataset) in prepared.iter().enumerate() {
            let mut warnings = Vec::new();
            let matched = existing_optimization_match(
                &second_resume_artifacts,
                STRATEGY_KIND,
                CONFIG_FINGERPRINT,
                PRIMARY_TAG,
                dataset,
                &mut warnings,
            )
            .expect("second resume must find every strict artifact identity");
            second_resume_counts.skipped += 1;
            complete_resume_item(
                &mut second_resume_items[index],
                "skipped",
                matched.summary.path.clone(),
                Some("exact_optimization_artifact".to_string()),
            );
        }
        assert_eq!(second_resume_counts.optimized, 0);
        assert_eq!(second_resume_counts.skipped, prepared.len());
        assert_eq!(
            executed_item_ids.len(),
            execution_count_before_second_resume,
            "a second resume must not execute any completed identity"
        );
        assert_eq!(
            std::fs::read(&artifact_catalog_path).expect("reread no-op artifact catalog"),
            catalog_bytes_after_resume,
            "a no-op resume must not mutate the durable artifact catalog"
        );
        assert!(second_resume_items
            .iter()
            .all(|item| item.status == "skipped"
                && item.evidence_state == Some(CampaignEvidenceState::Complete)
                && item.resume_state == Some(CampaignResumeState::Complete)));

        let no_op_summary = build_campaign_summary(
            "2026-07-24T18:00:02Z",
            "completed",
            STRATEGY_KIND,
            Some("15m".to_string()),
            &req,
            ResearchCampaignExecutionMode::OptimizeOnly,
            &tags,
            None,
            &second_resume_counts,
            &second_resume_items,
            None,
            None,
            &[],
            5,
        );
        assert_eq!(
            no_op_summary.evidence_state,
            Some(CampaignEvidenceState::Complete)
        );
        assert_eq!(
            no_op_summary.resume_state,
            Some(CampaignResumeState::Complete)
        );

        std::fs::remove_dir_all(temp_dir).expect("remove resume lifecycle temp dir");
    }

    #[test]
    fn build_composite_candidate_value_preserves_strategy_specific_payloads() {
        let params = serde_json::Map::from_iter([
            ("fast_ma_params".to_string(), json!({ "alpha": 2.0 })),
            ("slow_ma_params".to_string(), json!({ "offset": 0.85 })),
        ]);
        let base_strategy = json!({ "fee_bps": 1.5 });
        let run_req = json!({
            "strategy": base_strategy,
            "halftrend": { "amplitude": 2 },
            "macz": { "fast_length": 12 },
            "ott": { "period": 9 },
            "signal_mode": "soft_vote",
            "asr": { "atr_period": 21 },
            "vcc": { "trend_period": 6 },
            "vemr": { "mfi_period": 14 },
            "rocb": { "roc_period": 9 },
            "vcet": { "vol_ema_period": 18 },
            "mkcs": { "breakout_lookback": 30 },
            "kqsr": { "prb_period": 24 },
            "carm": { "otto_period": 16 },
            "kvdf": { "fvg_lookback": 11 },
            "cdmc": { "decycler_period": 34 }
        });

        for (kind, expected_key, expected_value) in [
            ("Asr", "asr", json!({ "atr_period": 21 })),
            ("Vcc", "vcc", json!({ "trend_period": 6 })),
            ("Vemr", "vemr", json!({ "mfi_period": 14 })),
            ("Rocb", "rocb", json!({ "roc_period": 9 })),
            ("Vcet", "vcet", json!({ "vol_ema_period": 18 })),
            ("Mkcs", "mkcs", json!({ "breakout_lookback": 30 })),
            ("Kqsr", "kqsr", json!({ "prb_period": 24 })),
            ("Carm", "carm", json!({ "otto_period": 16 })),
            ("Kvdf", "kvdf", json!({ "fvg_lookback": 11 })),
            ("Cdmc", "cdmc", json!({ "decycler_period": 34 })),
        ] {
            let candidate = build_composite_candidate_value(
                kind,
                &run_req,
                &params,
                "ema".to_string(),
                "sma".to_string(),
                12,
                48,
            );
            assert_eq!(candidate.get(expected_key), Some(&expected_value));
            assert_eq!(candidate.get("strategy"), Some(&base_strategy));
            assert_eq!(
                candidate.get("fast_ma_params"),
                Some(&json!({ "alpha": 2.0 }))
            );
            assert_eq!(
                candidate.get("slow_ma_params"),
                Some(&json!({ "offset": 0.85 }))
            );
        }

        let composite_cpu_candidate = build_composite_candidate_value(
            "CompositeCpu",
            &run_req,
            &params,
            "ema".to_string(),
            "sma".to_string(),
            12,
            48,
        );
        assert_eq!(
            composite_cpu_candidate.get("halftrend"),
            Some(&json!({ "amplitude": 2 }))
        );
        assert_eq!(
            composite_cpu_candidate.get("macz"),
            Some(&json!({ "fast_length": 12 }))
        );
        assert_eq!(
            composite_cpu_candidate.get("ott"),
            Some(&json!({ "period": 9 }))
        );
        assert_eq!(
            composite_cpu_candidate.get("signal_mode"),
            Some(&Value::String("soft_vote".to_string()))
        );
    }

    #[test]
    fn campaign_scalar_union_admission_diversifies_shortlist() {
        let run_req = json!({
            "strategy_kind": "DoubleMa",
            "strategy": {},
            "objective": "Sharpe"
        });
        let optimization = CampaignOptimizationResult::Backend(sample_backend_result(vec![
            (sample_params(5, 20), sample_metrics(3.0, 1.0, 1.0, 30.0)),
            (sample_params(8, 30), sample_metrics(2.0, 4.0, 4.0, 10.0)),
            (sample_params(12, 48), sample_metrics(1.5, 3.0, 3.0, 8.0)),
        ]));
        let validation_cfg = ResearchCampaignValidationConfig {
            max_candidates: 2,
            admission_policy: ValidationAdmissionPolicy::ScalarUnion,
            ..ResearchCampaignValidationConfig::default()
        };

        let (candidates, provenance) = candidate_selection_from_optimization(
            "DoubleMa",
            CampaignOptimizationFamily::DoubleLike,
            &run_req,
            &optimization,
            &validation_cfg,
        )
        .expect("scalar-union candidates");

        assert_eq!(
            provenance.applied_policy,
            ValidationAdmissionPolicy::ScalarUnion
        );
        assert_eq!(provenance.candidate_cap_limit, 2);
        assert_eq!(provenance.trimmed_count, 1);
        assert!((provenance.trim_ratio - (1.0 / 3.0)).abs() <= 1.0e-12);
        assert!(provenance.cap_applied);
        assert_eq!(
            provenance.selection_bias_note.as_deref(),
            Some("candidate_cap_trim")
        );
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0].get("fast_len").and_then(Value::as_u64),
            Some(5)
        );
        assert_eq!(
            candidates[1].get("fast_len").and_then(Value::as_u64),
            Some(12)
        );
    }

    #[test]
    fn campaign_scalar_union_admission_counts_full_dedupe_with_duplicate_params() {
        let run_req = json!({
            "strategy_kind": "DoubleMa",
            "strategy": {},
            "objective": "Sharpe"
        });
        let optimization = CampaignOptimizationResult::Backend(sample_backend_result(vec![
            (sample_params(5, 20), sample_metrics(3.0, 1.0, 1.0, 30.0)),
            (sample_params(8, 30), sample_metrics(2.0, 4.0, 4.0, 10.0)),
            (sample_params(5, 20), sample_metrics(2.5, 1.5, 1.5, 31.0)),
            (sample_params(12, 48), sample_metrics(1.5, 3.0, 3.0, 8.0)),
        ]));
        let validation_cfg = ResearchCampaignValidationConfig {
            max_candidates: 2,
            admission_policy: ValidationAdmissionPolicy::ScalarUnion,
            ..ResearchCampaignValidationConfig::default()
        };

        let (candidates, provenance) = candidate_selection_from_optimization(
            "DoubleMa",
            CampaignOptimizationFamily::DoubleLike,
            &run_req,
            &optimization,
            &validation_cfg,
        )
        .expect("scalar-union candidates");

        assert_eq!(
            provenance.applied_policy,
            ValidationAdmissionPolicy::ScalarUnion
        );
        assert_eq!(provenance.pre_dedupe_count, 16);
        assert_eq!(provenance.post_dedupe_count, 3);
        assert_eq!(provenance.post_cap_count, 2);
        assert_eq!(provenance.trimmed_count, 1);
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0].get("fast_len").and_then(Value::as_u64),
            Some(5)
        );
        assert_eq!(
            candidates[1].get("fast_len").and_then(Value::as_u64),
            Some(12)
        );
    }

    #[test]
    fn campaign_composite_mo_auto_uses_pareto_frontier_ordering() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeMo",
                    "strategy": { "fee_bps": 1.5 }
                },
                "res": {
                    "data_id": "campaign_1",
                    "pareto": [
                        {
                            "params": {
                                "fast_ma_type": "alma",
                                "slow_ma_type": "alma",
                                "fast_len": 10,
                                "slow_len": 30,
                                "strategy": { "fee_bps": 1.5 }
                            },
                            "metrics": { "pnl": 10.0, "sharpe": 1.0, "max_dd": 6.0 },
                            "maximize_value": 5.0,
                            "minimize_value": 1.0,
                            "violation": 1.0
                        },
                        {
                            "params": {
                                "fast_ma_type": "alma",
                                "slow_ma_type": "alma",
                                "fast_len": 12,
                                "slow_len": 48,
                                "strategy": { "fee_bps": 1.5 }
                            },
                            "metrics": { "pnl": 9.0, "sharpe": 0.9, "max_dd": 4.0 },
                            "maximize_value": 4.0,
                            "minimize_value": 2.0,
                            "violation": 0.0
                        }
                    ],
                    "total_evaluations": 2,
                    "runtime_ms": 10,
                    "telemetry": {
                        "prescan": null,
                        "search": {
                            "sampler": "ExactGrid",
                            "requested_evaluations": 2,
                            "actual_evaluations": 2,
                            "unique_evaluations": 2,
                            "duplicate_evaluations_avoided": 0,
                            "feasible_points": 1,
                            "rejected_decode_or_order": 0,
                            "rejected_prescan_pair": 0,
                            "rejected_other_constraints": 0,
                            "exact_grid_used": true,
                            "exact_grid_total_combinations": 2
                        }
                    }
                }
            }
        });
        let run_req = artifact.pointer("/run/req").cloned().expect("run.req");
        let optimization = optimization_result_from_artifact_value(
            CampaignOptimizationFamily::CompositeMo,
            &artifact,
        )
        .expect("decode CompositeMo artifact");
        let validation_cfg = ResearchCampaignValidationConfig {
            max_candidates: 2,
            admission_policy: ValidationAdmissionPolicy::Auto,
            ..ResearchCampaignValidationConfig::default()
        };

        let (candidates, provenance) = candidate_selection_from_optimization(
            "CompositeMo",
            CampaignOptimizationFamily::CompositeMo,
            &run_req,
            &optimization,
            &validation_cfg,
        )
        .expect("pareto admission candidates");

        assert_eq!(
            provenance.applied_policy,
            ValidationAdmissionPolicy::ParetoFrontier
        );
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0].get("fast_len").and_then(Value::as_u64),
            Some(12)
        );
        assert_eq!(
            candidates[1].get("fast_len").and_then(Value::as_u64),
            Some(10)
        );
    }

    #[test]
    fn campaign_composite_mo_dense_frontier_uses_diversity_trim() {
        let pareto = (10_u64..=14_u64)
            .enumerate()
            .map(|(index, fast_len)| {
                json!({
                    "params": {
                        "fast_ma_type": "alma",
                        "slow_ma_type": "alma",
                        "fast_len": fast_len,
                        "slow_len": fast_len + 20,
                        "strategy": { "fee_bps": 1.5 }
                    },
                    "metrics": {
                        "pnl": 10.0 - index as f64,
                        "sharpe": 1.5 - (index as f64 * 0.1),
                        "max_dd": 0.08 + (index as f64 * 0.01)
                    },
                    "maximize_value": 5.0 - index as f64,
                    "minimize_value": 1.0 + index as f64,
                    "violation": 0.0
                })
            })
            .collect::<Vec<_>>();
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeMo",
                    "strategy": { "fee_bps": 1.5 }
                },
                "res": {
                    "data_id": "campaign_dense_frontier",
                    "pareto": pareto,
                    "total_evaluations": 5,
                    "runtime_ms": 10,
                    "telemetry": {
                        "prescan": null,
                        "search": {
                            "sampler": "ExactGrid",
                            "requested_evaluations": 5,
                            "actual_evaluations": 5,
                            "unique_evaluations": 5,
                            "duplicate_evaluations_avoided": 0,
                            "feasible_points": 5,
                            "rejected_decode_or_order": 0,
                            "rejected_prescan_pair": 0,
                            "rejected_other_constraints": 0,
                            "exact_grid_used": true,
                            "exact_grid_total_combinations": 5
                        }
                    }
                }
            }
        });
        let run_req = artifact.pointer("/run/req").cloned().expect("run.req");
        let optimization = optimization_result_from_artifact_value(
            CampaignOptimizationFamily::CompositeMo,
            &artifact,
        )
        .expect("decode CompositeMo artifact");
        let validation_cfg = ResearchCampaignValidationConfig {
            max_candidates: 3,
            admission_policy: ValidationAdmissionPolicy::Auto,
            ..ResearchCampaignValidationConfig::default()
        };

        let (candidates, provenance) = candidate_selection_from_optimization(
            "CompositeMo",
            CampaignOptimizationFamily::CompositeMo,
            &run_req,
            &optimization,
            &validation_cfg,
        )
        .expect("dense pareto admission candidates");

        assert_eq!(
            provenance.applied_policy,
            ValidationAdmissionPolicy::ParetoFrontier
        );
        assert_eq!(provenance.post_dedupe_count, 5);
        assert_eq!(provenance.post_cap_count, 3);
        assert_eq!(provenance.candidate_cap_limit, 3);
        assert_eq!(provenance.trimmed_count, 2);
        assert!((provenance.trim_ratio - 0.4).abs() <= 1.0e-12);
        assert!(provenance.cap_applied);
        assert_eq!(
            provenance.selection_bias_note.as_deref(),
            Some("heuristic_frontier_trim")
        );
        assert_eq!(provenance.note.as_deref(), Some("pareto_diversity_trim"));
        assert_eq!(
            provenance
                .heads
                .first()
                .and_then(|head| head.note.as_deref()),
            Some("pareto_diversity_trim")
        );
        assert_eq!(candidates.len(), 3);
        assert_eq!(
            candidates[0].get("fast_len").and_then(Value::as_u64),
            Some(10)
        );
        assert_eq!(
            candidates[1].get("fast_len").and_then(Value::as_u64),
            Some(12)
        );
        assert_eq!(
            candidates[2].get("fast_len").and_then(Value::as_u64),
            Some(14)
        );
    }

    #[test]
    fn dataset_preload_job_drop_waits_for_worker_completion() {
        let (ready_tx, ready_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (drop_done_tx, drop_done_rx) = mpsc::channel();
        let finished = Arc::new(AtomicBool::new(false));
        let finished_worker = Arc::clone(&finished);
        let job = DatasetPreloadJob::spawn_with_task(move || {
            let _ = ready_tx.send(());
            let _ = release_rx.recv();
            finished_worker.store(true, Ordering::Relaxed);
            Err("cancelled for test".to_string())
        });

        ready_rx.recv().expect("worker should start");

        let drop_thread = std::thread::spawn(move || {
            drop(job);
            let _ = drop_done_tx.send(());
        });

        assert!(
            drop_done_rx.try_recv().is_err(),
            "dropping the preload job should wait for the worker to finish"
        );
        assert!(
            !finished.load(Ordering::Relaxed),
            "worker should still be blocked before release"
        );

        release_tx.send(()).expect("should unblock worker");
        drop_thread.join().expect("drop thread should join cleanly");
        drop_done_rx
            .recv()
            .expect("drop completion signal should arrive");
        assert!(
            finished.load(Ordering::Relaxed),
            "worker should complete before the preload job is dropped"
        );
    }
}
