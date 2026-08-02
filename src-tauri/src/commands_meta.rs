use super::{
    build_meta_report_xlsx_bytes,
    commands_campaign::{
        build_vcet_fixed_candidate_artifact_contract, validate_campaign_summary_artifact_value,
        validate_vcet_fixed_candidate_artifact_contract,
        validate_vcet_fixed_candidate_training_evidence,
        CampaignVcetFixedCandidateArtifactContract, ResearchCampaignCrossUniverseAdmissionPolicy,
        ResearchCampaignExecutionMode, ResearchCampaignFixedCandidateAggregate,
        ResearchCampaignRequest, ResearchCampaignSummary,
    },
    metric_surface::{summarize_artifact_metric_surfaces, ArtifactMetricSurfaceSummary},
    run_artifacts, sanitize_filename_component, TauriProgressSink,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use ta_app_core::{
    EvidenceGateRole, EvidenceMetricClass, EvidencePolicyMetric, EvidencePolicySummary,
    MetaSelectionRequest, MetaSelectionResult, MlConfidenceCompatibilityKey,
    MlConfidenceEvidenceTier, MlConfidenceGroupFeatures, MlConfidenceLabelConfig,
    MlConfidenceModel, MlConfidenceModelSummary, MlConfidenceScope, MlConfidenceTrainConfig,
    MlConfidenceTrainMetrics,
};
use tauri::Manager;

static META_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
const META_VCET_CANDIDATE_EVIDENCE_ROW_CAP: usize =
    ta_app_core::vcet_search::MAX_VCET_SEARCH_BUDGET;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MlConfidenceModelState {
    Missing,
    Healthy,
    Warning,
    Blocked,
    Legacy,
}

#[derive(Debug, Clone, Serialize)]
pub struct MlConfidenceModelStatus {
    pub exists: bool,
    pub model_path: String,
    pub bytes: usize,
    pub available_meta_artifacts: usize,
    pub available_labeled_rows: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest_meta_ts_available: Option<String>,
    pub stale: bool,
    pub retrain_recommended: bool,
    pub state: MlConfidenceModelState,
    pub legacy_model: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_healthy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrain_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_summary: Option<MlConfidenceModelSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_scan_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrainMlConfidenceResponse {
    pub model_path: String,
    pub model_bytes: usize,
    pub model_summary: MlConfidenceModelSummary,
    pub metrics: MlConfidenceTrainMetrics,
    pub status: MlConfidenceModelStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyMlConfidenceResponse {
    pub model_path: String,
    pub confidences: Vec<Option<f64>>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaCohortArtifactRef {
    pub role: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csv_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_content_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaCampaignCohortLineage {
    pub campaign_summary_path: String,
    pub campaign_summary_sha256: String,
    pub manifest_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_manifest_hash: Option<String>,
    pub admitted_candidate_count: usize,
    pub admitted_candidates: Vec<MetaVcetAdmittedCandidate>,
    pub cross_universe_admission: ResearchCampaignCrossUniverseAdmissionPolicy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_candidate_id: Option<String>,
    pub expected_train_datasets: usize,
    pub expected_holdout_datasets: usize,
    pub matched_train_artifacts: usize,
    pub matched_holdout_artifacts: usize,
    pub excluded_out_of_scope_artifacts: usize,
    /// Bounded, report-ready projection of the sealed campaign candidate
    /// evidence. This keeps Meta artifacts and XLSX generation independent of
    /// the potentially much larger campaign-summary payload while prioritizing
    /// selected/admitted rows and their effective VCET controls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcet_candidate_evidence: Option<MetaVcetCandidateEvidenceSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaVcetAdmittedCandidate {
    pub candidate_id: String,
    pub fast_len: u16,
    pub slow_len: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaVcetSearchProvenance {
    pub schema_version: u32,
    pub sequence: String,
    pub search_id: String,
    pub seed: u64,
    pub requested_budget: usize,
    pub effective_budget: usize,
    pub valid_search_space_size: String,
    pub float_identity_scale: i64,
    pub halton_attempts: usize,
    pub invalid_period_pairs_discarded: usize,
    pub duplicate_candidates_discarded: usize,
    pub lexicographic_fallback_candidates: usize,
    pub exhaustive_candidates: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ta_app_core::vcet_search::VcetSearchConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaVcetEffectiveControls {
    pub evidence_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vidya_short_period: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vidya_long_period: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vidya_alpha: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ce_period: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ce_mult: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ce_use_close: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_vidya_slope: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_trade_bars: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaVcetCandidateEvidenceRow {
    pub manifest_ordinal: usize,
    pub candidate_id: String,
    pub selected: bool,
    pub admitted: bool,
    pub controls: MetaVcetEffectiveControls,
    pub training: ResearchCampaignFixedCandidateAggregate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holdout: Option<ResearchCampaignFixedCandidateAggregate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_universe: Option<ResearchCampaignFixedCandidateAggregate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaVcetCandidateEvidenceSummary {
    pub schema_version: u32,
    pub candidate_manifest_hash: String,
    pub selection_cohort: String,
    pub selection_status: String,
    pub require_gpu_kernel: bool,
    pub source_candidate_count: usize,
    pub included_candidate_count: usize,
    pub omitted_candidate_count: usize,
    pub row_cap: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<MetaVcetSearchProvenance>,
    pub candidates: Vec<MetaVcetCandidateEvidenceRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaCohortDisjointnessSummary {
    pub train_artifacts: usize,
    pub holdout_artifacts: usize,
    pub overlapping_dataset_keys: usize,
    pub missing_train_dataset_keys: usize,
    pub missing_holdout_dataset_keys: usize,
    pub disjoint: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaCohortManifest {
    pub train_artifacts: Vec<MetaCohortArtifactRef>,
    pub holdout_artifacts: Vec<MetaCohortArtifactRef>,
    pub disjointness: MetaCohortDisjointnessSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holdout_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign_lineage: Option<MetaCampaignCohortLineage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

fn unavailable_vcet_controls(evidence_state: &str) -> MetaVcetEffectiveControls {
    MetaVcetEffectiveControls {
        evidence_state: evidence_state.to_string(),
        vidya_short_period: None,
        vidya_long_period: None,
        vidya_alpha: None,
        ce_period: None,
        ce_mult: None,
        ce_use_close: None,
        require_vidya_slope: None,
        max_trade_bars: None,
    }
}

fn controls_from_search_candidate(
    candidate: &ta_app_core::vcet_search::VcetSearchCandidate,
) -> MetaVcetEffectiveControls {
    let effective = &candidate.effective;
    MetaVcetEffectiveControls {
        evidence_state: "sealed_search_manifest".to_string(),
        vidya_short_period: Some(effective.vidya_short_period),
        vidya_long_period: Some(effective.vidya_long_period),
        vidya_alpha: Some(effective.vidya_alpha),
        ce_period: Some(effective.ce_period),
        ce_mult: Some(effective.ce_mult),
        ce_use_close: Some(effective.ce_use_close),
        require_vidya_slope: Some(effective.require_vidya_slope),
        max_trade_bars: Some(effective.max_trade_bars),
    }
}

fn controls_from_campaign_template(
    vcet: Option<&ta_app_core::VcetStrategyParams>,
    fast_len: u16,
    slow_len: u16,
) -> MetaVcetEffectiveControls {
    let Some(vcet) = vcet else {
        return unavailable_vcet_controls("unavailable_campaign_template");
    };
    let Ok(periods) = ta_app_core::vcet::resolve_vcet_periods_checked(vcet, fast_len, slow_len)
    else {
        return unavailable_vcet_controls("invalid_campaign_template_mapping");
    };
    MetaVcetEffectiveControls {
        evidence_state: "resolved_campaign_template".to_string(),
        vidya_short_period: Some(periods.vidya_short),
        vidya_long_period: Some(periods.vidya_long),
        vidya_alpha: Some(vcet.vidya_alpha),
        ce_period: Some(periods.ce),
        ce_mult: Some(vcet.ce_mult),
        ce_use_close: Some(vcet.ce_use_close),
        require_vidya_slope: Some(vcet.require_vidya_slope),
        max_trade_bars: Some(vcet.max_trade_bars),
    }
}

fn build_meta_vcet_candidate_evidence(
    summary: &ResearchCampaignSummary,
) -> Option<MetaVcetCandidateEvidenceSummary> {
    let evaluation = summary.fixed_candidate_evaluation.as_ref()?;
    let request = serde_json::from_value::<ResearchCampaignRequest>(summary.request.clone()).ok();
    let search_config = request
        .as_ref()
        .and_then(|request| request.validation_config.vcet_search.clone());
    let template_vcet = request.as_ref().and_then(|request| {
        request
            .optimization_template
            .get("vcet")
            .filter(|value| !value.is_null())
            .and_then(|value| {
                serde_json::from_value::<ta_app_core::VcetStrategyParams>(value.clone()).ok()
            })
    });

    let search_by_evaluation_id: HashMap<&str, &ta_app_core::vcet_search::VcetSearchCandidate> =
        evaluation
            .search_manifest
            .as_ref()
            .into_iter()
            .flat_map(|manifest| manifest.candidates.iter())
            .filter_map(|candidate| {
                candidate
                    .evaluation_candidate_id
                    .as_deref()
                    .map(|candidate_id| (candidate_id, candidate))
            })
            .collect();
    let holdout_by_id: HashMap<&str, &ResearchCampaignFixedCandidateAggregate> = evaluation
        .holdout
        .candidates
        .iter()
        .map(|candidate| (candidate.candidate_id.as_str(), candidate))
        .collect();
    let all_universe_by_id: HashMap<&str, &ResearchCampaignFixedCandidateAggregate> = evaluation
        .all_universe
        .candidates
        .iter()
        .map(|candidate| (candidate.candidate_id.as_str(), candidate))
        .collect();
    let admitted_ids: HashSet<&str> = evaluation
        .admitted_candidate_ids
        .iter()
        .map(String::as_str)
        .collect();
    let selected_id = evaluation.selected_candidate_id.as_deref();

    let mut ordered: Vec<(u8, usize, &ResearchCampaignFixedCandidateAggregate)> = evaluation
        .training
        .candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let priority = if selected_id == Some(candidate.candidate_id.as_str()) {
                0
            } else if admitted_ids.contains(candidate.candidate_id.as_str()) {
                1
            } else {
                2
            };
            (priority, index, candidate)
        })
        .collect();
    ordered.sort_by_key(|(priority, index, _)| (*priority, *index));
    ordered.truncate(META_VCET_CANDIDATE_EVIDENCE_ROW_CAP);

    let candidates = ordered
        .into_iter()
        .map(|(_, fallback_ordinal, candidate)| {
            let search_candidate = search_by_evaluation_id
                .get(candidate.candidate_id.as_str())
                .copied();
            let controls = match evaluation.search_manifest.as_ref() {
                Some(_) => search_candidate
                    .map(controls_from_search_candidate)
                    .unwrap_or_else(|| {
                        unavailable_vcet_controls("unavailable_search_manifest_mapping")
                    }),
                None => controls_from_campaign_template(
                    template_vcet.as_ref(),
                    candidate.fast_len,
                    candidate.slow_len,
                ),
            };
            MetaVcetCandidateEvidenceRow {
                manifest_ordinal: search_candidate
                    .map(|candidate| candidate.ordinal)
                    .unwrap_or(fallback_ordinal),
                candidate_id: candidate.candidate_id.clone(),
                selected: selected_id == Some(candidate.candidate_id.as_str()),
                admitted: admitted_ids.contains(candidate.candidate_id.as_str()),
                controls,
                training: candidate.clone(),
                holdout: holdout_by_id
                    .get(candidate.candidate_id.as_str())
                    .map(|candidate| (*candidate).clone()),
                all_universe: all_universe_by_id
                    .get(candidate.candidate_id.as_str())
                    .map(|candidate| (*candidate).clone()),
            }
        })
        .collect::<Vec<_>>();
    let source_candidate_count = evaluation.training.candidates.len();
    let included_candidate_count = candidates.len();

    let search = evaluation.search_manifest.as_ref().map(|manifest| {
        let stats = &manifest.generation_stats;
        MetaVcetSearchProvenance {
            schema_version: manifest.schema_version,
            sequence: match manifest.sequence {
                ta_app_core::vcet_search::VcetSearchSequence::Halton => "halton",
                ta_app_core::vcet_search::VcetSearchSequence::HaltonCranleyPatterson => {
                    "halton_cranley_patterson"
                }
                ta_app_core::vcet_search::VcetSearchSequence::ExhaustiveLexicographic => {
                    "exhaustive_lexicographic"
                }
            }
            .to_string(),
            search_id: manifest.search_id.clone(),
            seed: manifest.seed,
            requested_budget: manifest.requested_budget,
            effective_budget: manifest.effective_budget,
            valid_search_space_size: manifest.valid_search_space_size.clone(),
            float_identity_scale: manifest.float_identity_scale,
            halton_attempts: stats.halton_attempts,
            invalid_period_pairs_discarded: stats.invalid_period_pairs_discarded,
            duplicate_candidates_discarded: stats.duplicate_candidates_discarded,
            lexicographic_fallback_candidates: stats.lexicographic_fallback_candidates,
            exhaustive_candidates: stats.exhaustive_candidates,
            config: search_config,
        }
    });

    Some(MetaVcetCandidateEvidenceSummary {
        schema_version: evaluation.schema_version,
        candidate_manifest_hash: evaluation.candidate_manifest_hash.clone(),
        selection_cohort: evaluation.selection_cohort.clone(),
        selection_status: evaluation.selection_status.clone(),
        require_gpu_kernel: evaluation.require_gpu_kernel,
        source_candidate_count,
        included_candidate_count,
        omitted_candidate_count: source_candidate_count.saturating_sub(included_candidate_count),
        row_cap: META_VCET_CANDIDATE_EVIDENCE_ROW_CAP,
        search,
        candidates,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetaCampaignCohortRole {
    Train,
    Holdout,
}

#[derive(Debug, Clone)]
struct ExpectedCampaignDatasetIdentity {
    role: MetaCampaignCohortRole,
    source_content_fingerprint: Option<String>,
    path_timeframe_key: String,
    artifact_path_key: Option<String>,
    artifact_sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct MetaCampaignCohortExpectation {
    pub(super) lineage: MetaCampaignCohortLineage,
    fingerprints: HashMap<String, ExpectedCampaignDatasetIdentity>,
    paths: HashMap<String, ExpectedCampaignDatasetIdentity>,
    artifact_paths: HashMap<String, ExpectedCampaignDatasetIdentity>,
    pub(super) referenced_artifact_paths: Vec<PathBuf>,
    candidate_artifact_contract: Option<CampaignVcetFixedCandidateArtifactContract>,
    admitted_vcet_periods: Option<HashSet<(u16, u16)>>,
    campaign_summary: Option<ResearchCampaignSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MlConfidencePolicyEvaluationRequest {
    pub strategy_kind: String,
    #[serde(default)]
    pub timeframe: Option<String>,
    #[serde(default = "default_stage4_top_k")]
    pub top_k: usize,
    #[serde(default = "default_stage4_min_ml_confidence")]
    pub min_ml_confidence: f64,
    #[serde(default = "default_stage4_meta_weight")]
    pub meta_weight: f64,
    #[serde(default = "default_stage4_ml_weight")]
    pub ml_weight: f64,
    #[serde(default = "default_stage4_uncertainty_weight")]
    pub uncertainty_weight: f64,
    #[serde(default = "default_stage4_true")]
    pub require_healthy_model: bool,
    #[serde(default = "default_stage4_true")]
    pub fallback_to_meta_if_empty: bool,
    #[serde(default)]
    pub label: Option<MlConfidenceLabelConfig>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct MlConfidencePolicyArmMetrics {
    pub evaluated_artifacts: usize,
    pub selected_total: usize,
    pub positives_total: usize,
    pub precision_at_k: f64,
    pub selection_rate: f64,
    pub base_rate: f64,
    pub lift_vs_base_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MlConfidencePolicyEvaluationResult {
    pub strategy_kind: String,
    pub timeframe: String,
    pub artifacts_considered: usize,
    pub artifacts_evaluated: usize,
    pub artifacts_skipped_no_history: usize,
    pub artifacts_skipped_training_failure: usize,
    pub artifacts_skipped_unhealthy_model: usize,
    pub artifacts_skipped_no_labels: usize,
    pub total_label_rows: usize,
    pub positive_label_rows: usize,
    pub base_rate: f64,
    pub top_k: usize,
    pub min_ml_confidence: f64,
    pub meta_weight: f64,
    pub ml_weight: f64,
    pub uncertainty_weight: f64,
    pub meta_only: MlConfidencePolicyArmMetrics,
    pub ml_only: MlConfidencePolicyArmMetrics,
    pub stage5_blended: MlConfidencePolicyArmMetrics,
    pub best_policy: String,
    pub stage5_vs_meta_precision_delta: f64,
    pub stage5_vs_ml_precision_delta: f64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunMlConfidencePolicyEvaluationResponse {
    pub ts: String,
    pub eval_json_path: String,
    pub eval_json_bytes: usize,
    pub result: MlConfidencePolicyEvaluationResult,
}

fn default_stage4_top_k() -> usize {
    10
}

fn default_stage4_min_ml_confidence() -> f64 {
    0.55
}

fn default_stage4_meta_weight() -> f64 {
    0.60
}

fn default_stage4_ml_weight() -> f64 {
    0.35
}

fn default_stage4_uncertainty_weight() -> f64 {
    0.05
}

fn default_stage4_accept_confidence() -> f64 {
    0.70
}

fn default_stage4_review_confidence() -> f64 {
    0.55
}

fn default_stage4_max_accept_uncertainty() -> f64 {
    0.45
}

fn default_stage4_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stage5TrainingHistoryPolicy {
    SourceExcluded,
    ChronologicalBeforeSource,
}

fn default_stage5_training_history_policy() -> Stage5TrainingHistoryPolicy {
    Stage5TrainingHistoryPolicy::SourceExcluded
}

#[derive(Debug, Clone, Serialize)]
pub struct Stage5TrainingArtifactRef {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stage5ExcludedArtifactRef {
    #[serde(flatten)]
    pub artifact: Stage5TrainingArtifactRef,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stage5TrainingManifest {
    pub policy: Stage5TrainingHistoryPolicy,
    pub source_artifact: Stage5TrainingArtifactRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub training_artifacts: Vec<Stage5TrainingArtifactRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_artifacts: Vec<Stage5ExcludedArtifactRef>,
    pub training_artifact_count: usize,
    pub excluded_source_count: usize,
    pub excluded_future_or_equal_count: usize,
    pub excluded_missing_ts_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetaStage4Request {
    pub strategy_kind: String,
    #[serde(default)]
    pub timeframe: Option<String>,
    #[serde(default)]
    pub source_meta_path: Option<String>,
    #[serde(default = "default_stage4_top_k")]
    pub top_k: usize,

    #[serde(default = "default_stage4_min_ml_confidence")]
    pub min_ml_confidence: f64,
    #[serde(default = "default_stage4_meta_weight")]
    pub meta_weight: f64,
    #[serde(default = "default_stage4_ml_weight")]
    pub ml_weight: f64,
    #[serde(default = "default_stage4_uncertainty_weight")]
    pub uncertainty_weight: f64,
    #[serde(default = "default_stage4_accept_confidence")]
    pub accept_confidence: f64,
    #[serde(default = "default_stage4_review_confidence")]
    pub review_confidence: f64,
    #[serde(default = "default_stage4_max_accept_uncertainty")]
    pub max_accept_uncertainty: f64,
    #[serde(default = "default_stage4_true")]
    pub auto_train_model: bool,
    #[serde(default = "default_stage4_true")]
    pub auto_retrain_stale: bool,
    #[serde(default = "default_stage4_true")]
    pub require_healthy_model: bool,
    #[serde(default = "default_stage4_true")]
    pub fallback_to_meta_if_empty: bool,
    #[serde(default = "default_stage5_training_history_policy")]
    pub training_history_policy: Stage5TrainingHistoryPolicy,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaStage4Result {
    pub strategy_kind: String,
    pub timeframe: String,
    pub source_meta_json_path: String,
    pub model_path: String,
    pub model_state: MlConfidenceModelState,
    pub model_evidence_tier: MlConfidenceEvidenceTier,
    pub model_healthy: bool,
    pub model_trained_now: bool,
    pub groups_considered: usize,
    pub groups_scored: usize,
    pub groups_selected: usize,
    pub min_ml_confidence: f64,
    pub top_k: usize,
    pub meta_weight: f64,
    pub ml_weight: f64,
    pub uncertainty_weight: f64,
    pub accept_confidence: f64,
    pub review_confidence: f64,
    pub max_accept_uncertainty: f64,
    pub decision_counts: Stage4DecisionCounts,
    pub selected_groups: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    pub training_manifest: Stage5TrainingManifest,
    pub training_artifact_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_artifact_sha256: Option<String>,
    pub warning_count: usize,
    pub evidence_policy_summary: EvidencePolicySummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stage4DecisionCounts {
    pub accept: usize,
    pub review: usize,
    pub reject: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunMetaStage4Response {
    pub ts: String,
    pub stage4_json_path: String,
    pub stage4_json_bytes: usize,
    pub stage5_json_path: String,
    pub stage5_json_bytes: usize,
    pub result: MetaStage4Result,
}

pub type MetaStage5Request = MetaStage4Request;
pub type MetaStage5Result = MetaStage4Result;
pub type RunMetaStage5Response = RunMetaStage4Response;

fn normalize_ml_scope(mut scope: MlConfidenceScope) -> Result<MlConfidenceScope, String> {
    scope.strategy_kind = scope.strategy_kind.trim().to_string();
    scope.timeframe = normalize_timeframe_label(&scope.timeframe);
    if scope.strategy_kind.is_empty() {
        return Err("strategy_kind is required".to_string());
    }
    if scope.timeframe.is_empty() {
        return Err("timeframe is required (e.g. '1m')".to_string());
    }
    Ok(scope)
}

fn ml_confidence_models_base_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<std::path::PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    Ok(base.join("models").join("confidence"))
}

fn ml_confidence_models_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<std::path::PathBuf, String> {
    let dir = ml_confidence_models_base_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn ml_confidence_model_path_in_dir(scope: &MlConfidenceScope, dir: PathBuf) -> std::path::PathBuf {
    let sk = sanitize_filename_component(&scope.strategy_kind);
    let tf = sanitize_filename_component(&scope.timeframe);
    dir.join(format!("ml_confidence_{sk}_{tf}.json"))
}

fn ml_confidence_model_path(
    scope: &MlConfidenceScope,
    app: &tauri::AppHandle<impl tauri::Runtime>,
) -> Result<std::path::PathBuf, String> {
    Ok(ml_confidence_model_path_in_dir(
        scope,
        ml_confidence_models_dir(app)?,
    ))
}

fn ml_confidence_model_path_read_only(
    scope: &MlConfidenceScope,
    app: &tauri::AppHandle<impl tauri::Runtime>,
) -> Result<std::path::PathBuf, String> {
    Ok(ml_confidence_model_path_in_dir(
        scope,
        ml_confidence_models_base_dir(app)?,
    ))
}

fn meta_temp_path(path: &Path) -> Result<PathBuf, String> {
    let parent = path.parent().ok_or_else(|| {
        format!(
            "artifact path has no parent directory: {}",
            path.to_string_lossy()
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("artifact.json");
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let seq = META_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".{file_name}.{}.{}.{}.tmp",
        std::process::id(),
        nanos,
        seq
    )))
}

fn existing_regular_file_metadata(path: &Path) -> Result<Option<std::fs::Metadata>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(format!(
                    "artifact path must be a regular file: {}",
                    path.to_string_lossy()
                ));
            }
            Ok(Some(metadata))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "failed to inspect artifact path {}: {error}",
            path.to_string_lossy()
        )),
    }
}

#[cfg(windows)]
fn replace_existing_file_with_temp(path: &Path, temp_path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    fn wide_null(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    let replaced = wide_null(path);
    let replacement = wide_null(temp_path);
    let ok = unsafe {
        ReplaceFileW(
            replaced.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_existing_file_with_temp(path: &Path, temp_path: &Path) -> Result<(), String> {
    std::fs::rename(temp_path, path).map_err(|e| e.to_string())
}

fn write_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = meta_temp_path(path)?;
    let result = (|| -> Result<(), String> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        if existing_regular_file_metadata(path)?.is_some() {
            replace_existing_file_with_temp(path, &tmp)
        } else {
            std::fs::rename(&tmp, path).map_err(|e| e.to_string())
        }
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn classify_ml_confidence_model_state(
    exists: bool,
    model_summary: Option<&MlConfidenceModelSummary>,
    error: Option<&str>,
    stale: bool,
    retrain_recommended: bool,
) -> (MlConfidenceModelState, bool, Option<bool>) {
    if !exists {
        return (MlConfidenceModelState::Missing, false, None);
    }
    if error.is_some() {
        return (MlConfidenceModelState::Blocked, false, None);
    }

    let Some(model) = model_summary else {
        return (MlConfidenceModelState::Blocked, false, None);
    };

    let model_healthy = model.meta.as_ref().map(|m| m.healthy);
    let legacy_model = model.meta.is_none();
    let state = if legacy_model {
        MlConfidenceModelState::Legacy
    } else if matches!(model_healthy, Some(false)) {
        MlConfidenceModelState::Blocked
    } else if stale || retrain_recommended {
        MlConfidenceModelState::Warning
    } else {
        MlConfidenceModelState::Healthy
    };
    (state, legacy_model, model_healthy)
}

fn read_json_value(path: &Path) -> Option<Value> {
    let file = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).ok()
}

fn load_meta_artifacts_with_paths_blocking(
    app: &tauri::AppHandle,
) -> Result<Vec<(PathBuf, Value)>, String> {
    run_artifacts::load_meta_artifacts_with_paths_blocking(app)
}

fn load_meta_artifacts_blocking(app: &tauri::AppHandle) -> Result<Vec<Value>, String> {
    run_artifacts::load_meta_artifacts_blocking(app)
}

fn strategy_supports_holdout(strategy_kind: &str) -> bool {
    matches!(
        strategy_kind,
        "CompositeCpu"
            | "CompositeMo"
            | "DoubleMa"
            | "Pit"
            | "Set"
            | "Mts"
            | "Orc"
            | "Marrt"
            | "Vfct"
            | "Rocb"
            | "Asr"
            | "Vcc"
            | "Vemr"
            | "Vcet"
            | "CompositeTrend"
            | "RegimeHybrid"
            | "BreakoutMomentum"
    )
}

fn strategy_uses_double_ma_validation(strategy_kind: &str) -> bool {
    matches!(
        strategy_kind,
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

fn sha256_file_hex_opt(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Some(format!("{:x}", hasher.finalize()))
}

fn value_string_at(value: &Value, pointers: &[&str]) -> Option<String> {
    pointers.iter().find_map(|pointer| {
        value
            .pointer(pointer)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    })
}

fn artifact_tags_vec(artifact: &Value) -> Vec<String> {
    artifact
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|tags| {
            tags.iter()
                .filter_map(|tag| tag.as_str())
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn cohort_artifact_ref(
    path: &Path,
    artifact: &Value,
    role: &str,
    precomputed_sha256: Option<&str>,
) -> MetaCohortArtifactRef {
    MetaCohortArtifactRef {
        role: role.to_string(),
        path: path.to_string_lossy().to_string(),
        artifact_sha256: precomputed_sha256
            .map(str::to_string)
            .or_else(|| sha256_file_hex_opt(path)),
        ts: value_string_at(artifact, &["/ts"]),
        dataset_label: value_string_at(artifact, &["/dataset_meta/label", "/data/label"]),
        csv_path: artifact_csv_path(artifact),
        source_content_fingerprint: value_string_at(
            artifact,
            &[
                "/dataset_meta/source_content_fingerprint",
                "/data/source_content_fingerprint",
                "/run/req/source_content_fingerprint",
            ],
        ),
        timeframe: value_string_at(artifact, &["/dataset_meta/timeframe", "/run/req/timeframe"])
            .map(|tf| normalize_timeframe_label(&tf)),
        tags: artifact_tags_vec(artifact),
    }
}

fn cohort_dataset_key(artifact: &Value) -> Option<String> {
    if let Some(fp) = value_string_at(
        artifact,
        &[
            "/dataset_meta/source_content_fingerprint",
            "/data/source_content_fingerprint",
            "/run/req/source_content_fingerprint",
        ],
    ) {
        return Some(format!("fingerprint:{fp}"));
    }
    let csv_path = artifact_csv_path(artifact)?;
    let timeframe = value_string_at(artifact, &["/dataset_meta/timeframe", "/run/req/timeframe"])
        .map(|tf| normalize_timeframe_label(&tf))
        .unwrap_or_default();
    Some(cohort_path_timeframe_key(&csv_path, &timeframe))
}

fn cohort_path_timeframe_key(csv_path: &str, timeframe: &str) -> String {
    let mut path_key = String::with_capacity(csv_path.len());
    let mut needs_separator = false;
    for part in csv_path.trim().split(['/', '\\']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if needs_separator {
            path_key.push('/');
        }
        for ch in part.chars() {
            path_key.push(ch.to_ascii_lowercase());
        }
        needs_separator = true;
    }
    format!(
        "path:{}|tf:{}",
        path_key,
        normalize_timeframe_label(timeframe)
    )
}

fn cohort_artifact_path_key(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .to_string_lossy()
        .trim()
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn campaign_manifest_cohort_tag(manifest_hash: &str) -> String {
    format!(
        "campaign-cohort:{}",
        manifest_hash.trim().trim_start_matches("sha256:")
    )
}

fn insert_expected_campaign_identity(
    expectation: &mut MetaCampaignCohortExpectation,
    path_key: String,
    fingerprint: Option<String>,
    artifact_path: Option<&str>,
    artifact_sha256: Option<&str>,
    role: MetaCampaignCohortRole,
) -> Result<(), String> {
    if expectation.paths.contains_key(&path_key) {
        return Err(format!(
            "campaign-bound Meta cohort contains a duplicate dataset path/timeframe identity: {path_key}"
        ));
    }
    let artifact_path_key = artifact_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(cohort_artifact_path_key);
    let identity = ExpectedCampaignDatasetIdentity {
        role,
        source_content_fingerprint: fingerprint.clone(),
        path_timeframe_key: path_key.clone(),
        artifact_path_key: artifact_path_key.clone(),
        artifact_sha256: artifact_sha256.map(str::to_string),
    };
    if let Some(fingerprint) = fingerprint.as_deref() {
        if expectation.fingerprints.contains_key(fingerprint) {
            return Err(
                "campaign-bound Meta cohort contains duplicate dataset content fingerprints; each requested stock must have a unique dataset identity"
                    .to_string(),
            );
        }
        expectation
            .fingerprints
            .insert(fingerprint.to_string(), identity.clone());
    }
    if let Some(artifact_path_key) = artifact_path_key {
        if expectation.artifact_paths.contains_key(&artifact_path_key) {
            return Err(format!(
                "campaign-bound Meta cohort contains a duplicate run-artifact path: {artifact_path_key}"
            ));
        }
        expectation
            .artifact_paths
            .insert(artifact_path_key, identity.clone());
    }
    expectation.paths.insert(path_key, identity);
    Ok(())
}

fn validate_campaign_artifact_raw_sha256(
    expectation: &MetaCampaignCohortExpectation,
    artifact_path: &Path,
    actual_sha256: &str,
) -> Result<(), String> {
    let path_key = cohort_artifact_path_key(artifact_path);
    let expected = expectation.artifact_paths.get(&path_key).ok_or_else(|| {
        format!(
            "campaign-bound Meta has no sealed artifact identity for '{}'",
            artifact_path.to_string_lossy()
        )
    })?;
    let expected_sha256 = expected
        .artifact_sha256
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "campaign-bound Meta is missing the sealed raw SHA-256 for '{}'",
                artifact_path.to_string_lossy()
            )
        })?;
    if !actual_sha256.eq_ignore_ascii_case(expected_sha256) {
        return Err(format!(
            "campaign-bound Meta rejected '{}' because its raw SHA-256 changed after the terminal campaign summary was sealed",
            artifact_path.to_string_lossy()
        ));
    }
    Ok(())
}

pub(super) fn load_meta_campaign_cohort_expectation<R: tauri::Runtime>(
    campaign_summary_path: Option<&str>,
    req: &MetaSelectionRequest,
    app: &tauri::AppHandle<R>,
) -> Result<Option<MetaCampaignCohortExpectation>, String> {
    let Some(path) = campaign_summary_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return Ok(None);
    };
    let (value, campaign_summary_sha256) =
        run_artifacts::load_artifact_value_checked_with_sha256_blocking(
            path.to_string(),
            app.clone(),
        )?;
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if !matches!(
        kind,
        "vectorgrid_campaign_summary" | "vectorbt_campaign_summary"
    ) {
        return Err(
            "campaign-bound Meta requires a research campaign summary artifact".to_string(),
        );
    }
    if value.get("version").and_then(Value::as_u64) != Some(2) {
        return Err(
            "campaign-bound Meta requires campaign summary integrity version 2; rerun the campaign with this build"
                .to_string(),
        );
    }
    validate_campaign_summary_artifact_value(&value).map_err(|error| {
        format!("campaign-bound Meta rejected the supplied campaign summary: {error}")
    })?;
    let summary: ResearchCampaignSummary = serde_json::from_value(value)
        .map_err(|error| format!("invalid campaign summary for Meta lineage: {error}"))?;
    if summary.counts.cancelled
        || !matches!(
            summary.state.trim(),
            "completed" | "completed_with_failures" | "completed_with_warnings"
        )
    {
        return Err(
            "campaign-bound Meta requires a terminal, non-cancelled campaign summary".to_string(),
        );
    }
    let manifest = summary
        .manifest
        .as_ref()
        .ok_or_else(|| "campaign-bound Meta requires a v2 campaign manifest".to_string())?;
    if manifest.execution_mode != ResearchCampaignExecutionMode::EvaluateCandidates {
        return Err(
            "campaign-bound Meta currently requires a VCET evaluate_candidates campaign summary"
                .to_string(),
        );
    }
    if req.strategy_kind.trim() != "Vcet" || summary.strategy_kind.trim() != "Vcet" {
        return Err(
            "campaign-bound Meta lineage currently supports strategy_kind='Vcet' only".to_string(),
        );
    }
    let candidate_artifact_contract = build_vcet_fixed_candidate_artifact_contract(&summary)
        .map_err(|error| {
            format!("campaign-bound Meta rejected the VCET candidate contract: {error}")
        })?;
    let fixed_candidate_evaluation =
        summary.fixed_candidate_evaluation.as_ref().ok_or_else(|| {
            "campaign-bound Meta requires a VCET fixed-candidate evaluation summary".to_string()
        })?;
    if fixed_candidate_evaluation.schema_version != 4 {
        return Err(
            "campaign-bound Meta requires sealed VCET fixed-candidate admission schema v4; rerun the campaign with this build"
                .to_string(),
        );
    }
    if fixed_candidate_evaluation.admitted_candidate_ids.is_empty()
        || fixed_candidate_evaluation.selected_candidate_id.is_none()
    {
        return Err(
            "campaign-bound Meta stopped because no VCET candidate met the configured cross-universe admission policy"
                .to_string(),
        );
    }
    let admitted_candidate_ids: HashSet<&str> = fixed_candidate_evaluation
        .admitted_candidate_ids
        .iter()
        .map(String::as_str)
        .collect();
    let admitted_vcet_periods: HashSet<(u16, u16)> = fixed_candidate_evaluation
        .training
        .candidates
        .iter()
        .filter(|row| admitted_candidate_ids.contains(row.candidate_id.as_str()))
        .map(|row| (row.fast_len, row.slow_len))
        .collect();
    let admitted_candidates: Vec<MetaVcetAdmittedCandidate> = fixed_candidate_evaluation
        .training
        .candidates
        .iter()
        .filter(|row| admitted_candidate_ids.contains(row.candidate_id.as_str()))
        .map(|row| MetaVcetAdmittedCandidate {
            candidate_id: row.candidate_id.clone(),
            fast_len: row.fast_len,
            slow_len: row.slow_len,
        })
        .collect();
    if admitted_vcet_periods.len() != fixed_candidate_evaluation.admitted_candidate_count
        || !fixed_candidate_evaluation
            .selected_candidate_id
            .as_deref()
            .is_some_and(|candidate_id| admitted_candidate_ids.contains(candidate_id))
    {
        return Err(
            "campaign-bound Meta rejected inconsistent VCET cross-universe admission lineage"
                .to_string(),
        );
    }
    let requested_timeframe = req
        .timeframe
        .as_deref()
        .map(normalize_timeframe_label)
        .filter(|value| !value.is_empty());
    let campaign_timeframe = summary
        .timeframe
        .as_deref()
        .map(normalize_timeframe_label)
        .filter(|value| !value.is_empty());
    if requested_timeframe.is_some()
        && campaign_timeframe.is_some()
        && requested_timeframe != campaign_timeframe
    {
        return Err(
            "campaign-bound Meta timeframe does not match the campaign summary".to_string(),
        );
    }
    if let Some(required_tag) = req
        .required_tag
        .as_deref()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        let cohort_tag = campaign_manifest_cohort_tag(&manifest.manifest_hash);
        if !summary
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(required_tag))
            && !cohort_tag.eq_ignore_ascii_case(required_tag)
        {
            return Err(format!(
                "campaign-bound Meta required_tag '{required_tag}' does not identify the supplied campaign summary"
            ));
        }
    }
    if req.eval_holdout {
        let requested_holdout_tag = req
            .holdout_tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .ok_or_else(|| {
                "campaign-bound VCET holdout evaluation requires holdout_tag".to_string()
            })?;
        if summary.holdout_pct == 0
            || summary
                .holdout_tag
                .as_deref()
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .is_none_or(|tag| !tag.eq_ignore_ascii_case(requested_holdout_tag))
        {
            return Err(
                "campaign-bound Meta holdout scope does not match the campaign summary".to_string(),
            );
        }
    }

    let expected_train_datasets = summary.items.iter().filter(|item| !item.is_holdout).count();
    let expected_holdout_datasets = summary.items.iter().filter(|item| item.is_holdout).count();
    let mut expectation = MetaCampaignCohortExpectation {
        lineage: MetaCampaignCohortLineage {
            campaign_summary_path: path.to_string(),
            campaign_summary_sha256,
            manifest_hash: manifest.manifest_hash.clone(),
            candidate_manifest_hash: manifest.candidate_manifest_hash.clone(),
            admitted_candidate_count: fixed_candidate_evaluation.admitted_candidate_count,
            admitted_candidates,
            cross_universe_admission: fixed_candidate_evaluation.cross_universe_admission,
            selected_candidate_id: fixed_candidate_evaluation.selected_candidate_id.clone(),
            expected_train_datasets,
            expected_holdout_datasets,
            matched_train_artifacts: 0,
            matched_holdout_artifacts: 0,
            excluded_out_of_scope_artifacts: 0,
            vcet_candidate_evidence: build_meta_vcet_candidate_evidence(&summary),
        },
        fingerprints: HashMap::with_capacity(summary.items.len()),
        paths: HashMap::with_capacity(summary.items.len()),
        artifact_paths: HashMap::with_capacity(summary.items.len()),
        referenced_artifact_paths: Vec::with_capacity(summary.items.len()),
        candidate_artifact_contract: Some(candidate_artifact_contract),
        admitted_vcet_periods: Some(admitted_vcet_periods),
        campaign_summary: Some(summary.clone()),
    };
    for item in &summary.items {
        let role = if item.is_holdout {
            MetaCampaignCohortRole::Holdout
        } else {
            MetaCampaignCohortRole::Train
        };
        let fingerprint = item
            .dataset_identity
            .as_ref()
            .and_then(|identity| identity.source_content_fingerprint.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        insert_expected_campaign_identity(
            &mut expectation,
            cohort_path_timeframe_key(&item.csv_path, &item.timeframe),
            fingerprint,
            item.artifact_path.as_deref(),
            item.artifact_sha256.as_deref(),
            role,
        )?;
        if let Some(path) = item
            .artifact_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            expectation
                .referenced_artifact_paths
                .push(PathBuf::from(path));
        }
    }
    Ok(Some(expectation))
}

fn campaign_cohort_role_for_artifact(
    expectation: &MetaCampaignCohortExpectation,
    artifact_path: &Path,
    artifact: &Value,
) -> Option<MetaCampaignCohortRole> {
    let expected_manifest_hash = expectation.lineage.manifest_hash.trim();
    let mut has_campaign_lineage = false;
    for pointer in ["/campaign/manifest_hash", "/campaign/cohort_id"] {
        if let Some(actual) = artifact
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            has_campaign_lineage = true;
            if actual != expected_manifest_hash {
                return None;
            }
        }
    }
    let artifact_path_key = cohort_artifact_path_key(artifact_path);
    let exact_path_identity = expectation.artifact_paths.get(&artifact_path_key);
    if !has_campaign_lineage && exact_path_identity.is_none() {
        return None;
    }

    if let Some(expected_candidate_manifest_hash) = expectation
        .lineage
        .candidate_manifest_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let actual_candidate_manifest_hash = artifact
            .pointer("/validation/req/candidate_manifest_hash")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());
        match actual_candidate_manifest_hash {
            Some(actual) if actual == expected_candidate_manifest_hash => {}
            _ => return None,
        }
    }
    if let Some(contract) = expectation.candidate_artifact_contract.as_ref() {
        if validate_vcet_fixed_candidate_artifact_contract(artifact, contract).is_err() {
            return None;
        }
    }

    let fingerprint = value_string_at(
        artifact,
        &[
            "/dataset_meta/source_content_fingerprint",
            "/data/source_content_fingerprint",
            "/run/req/source_content_fingerprint",
        ],
    );
    let fingerprint_identity = fingerprint
        .as_deref()
        .and_then(|value| expectation.fingerprints.get(value));
    if fingerprint.is_some() && fingerprint_identity.is_none() {
        return None;
    }
    let csv_path = artifact_csv_path(artifact)?;
    let timeframe = value_string_at(artifact, &["/dataset_meta/timeframe", "/run/req/timeframe"])
        .map(|value| normalize_timeframe_label(&value))
        .unwrap_or_default();
    let path_key = cohort_path_timeframe_key(&csv_path, &timeframe);
    let expected = expectation.paths.get(&path_key)?;
    match (
        expected.source_content_fingerprint.as_deref(),
        fingerprint.as_deref(),
    ) {
        (Some(expected_fingerprint), Some(actual_fingerprint))
            if expected_fingerprint != actual_fingerprint =>
        {
            None
        }
        (Some(_), None) => None,
        _ => {
            for identity in [fingerprint_identity, exact_path_identity]
                .into_iter()
                .flatten()
            {
                if identity.role != expected.role
                    || identity.path_timeframe_key != expected.path_timeframe_key
                    || identity.source_content_fingerprint != expected.source_content_fingerprint
                    || identity.artifact_path_key != expected.artifact_path_key
                    || identity.artifact_sha256 != expected.artifact_sha256
                {
                    return None;
                }
            }
            if let Some(actual_role) = artifact
                .pointer("/campaign/cohort_role")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                let role_matches = match expected.role {
                    MetaCampaignCohortRole::Train => {
                        actual_role.eq_ignore_ascii_case("train")
                            || actual_role.eq_ignore_ascii_case("training")
                    }
                    MetaCampaignCohortRole::Holdout => actual_role.eq_ignore_ascii_case("holdout"),
                };
                if !role_matches {
                    return None;
                }
            }
            Some(expected.role)
        }
    }
}

fn retain_admitted_vcet_candidate_rows(
    artifact: &mut Value,
    admitted_periods: &HashSet<(u16, u16)>,
) -> Result<(), String> {
    let rows = artifact
        .pointer_mut("/validation/res/candidates")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            "campaign-bound VCET artifact is missing validation.res.candidates".to_string()
        })?;
    rows.retain(|row| {
        let fast_len = row
            .pointer("/params/fast_len")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok());
        let slow_len = row
            .pointer("/params/slow_len")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok());
        fast_len
            .zip(slow_len)
            .is_some_and(|periods| admitted_periods.contains(&periods))
    });
    if rows.len() != admitted_periods.len() {
        return Err(format!(
            "campaign-bound VCET artifact retained {} admitted candidates, expected {}",
            rows.len(),
            admitted_periods.len()
        ));
    }
    Ok(())
}

fn requested_dataset_denominator(observed: usize, expected: Option<usize>) -> usize {
    expected.unwrap_or(observed).max(observed)
}

#[cfg(test)]
fn load_campaign_referenced_artifact<R: tauri::Runtime>(
    path: &Path,
    app: &tauri::AppHandle<R>,
) -> Result<Value, String> {
    run_artifacts::load_artifact_value_checked_blocking(
        path.to_string_lossy().to_string(),
        app.clone(),
    )
    .map_err(|error| {
        format!(
            "campaign-bound Meta could not load referenced run artifact '{}': {error}",
            path.to_string_lossy()
        )
    })
}

fn build_meta_cohort_manifest(
    train_artifacts: &[(PathBuf, Value)],
    holdout_artifacts: &[(PathBuf, Value)],
    holdout_tag: Option<&str>,
    campaign_lineage: Option<MetaCampaignCohortLineage>,
    artifact_sha256_by_path: Option<&HashMap<String, String>>,
) -> MetaCohortManifest {
    let train_refs: Vec<MetaCohortArtifactRef> = train_artifacts
        .iter()
        .map(|(path, artifact)| {
            let sha256 = artifact_sha256_by_path
                .and_then(|digests| digests.get(&cohort_artifact_path_key(path)));
            cohort_artifact_ref(path, artifact, "train", sha256.map(String::as_str))
        })
        .collect();
    let holdout_refs: Vec<MetaCohortArtifactRef> = holdout_artifacts
        .iter()
        .map(|(path, artifact)| {
            let sha256 = artifact_sha256_by_path
                .and_then(|digests| digests.get(&cohort_artifact_path_key(path)));
            cohort_artifact_ref(path, artifact, "holdout", sha256.map(String::as_str))
        })
        .collect();

    let mut train_keys: HashSet<String> = HashSet::new();
    let mut holdout_keys: HashSet<String> = HashSet::new();
    let mut missing_train_dataset_keys = 0usize;
    let mut missing_holdout_dataset_keys = 0usize;

    for (_, artifact) in train_artifacts {
        if let Some(key) = cohort_dataset_key(artifact) {
            train_keys.insert(key);
        } else {
            missing_train_dataset_keys = missing_train_dataset_keys.saturating_add(1);
        }
    }
    for (_, artifact) in holdout_artifacts {
        if let Some(key) = cohort_dataset_key(artifact) {
            holdout_keys.insert(key);
        } else {
            missing_holdout_dataset_keys = missing_holdout_dataset_keys.saturating_add(1);
        }
    }

    let overlapping_dataset_keys = train_keys.intersection(&holdout_keys).count();
    let disjoint = overlapping_dataset_keys == 0
        && missing_train_dataset_keys == 0
        && missing_holdout_dataset_keys == 0;
    let mut warnings = Vec::new();
    if overlapping_dataset_keys > 0 {
        warnings.push(format!(
            "Meta cohort warning: train and holdout cohorts share {overlapping_dataset_keys} dataset key(s)."
        ));
    }
    let missing_keys = missing_train_dataset_keys.saturating_add(missing_holdout_dataset_keys);
    if missing_keys > 0 {
        warnings.push(format!(
            "Meta cohort warning: {missing_keys} artifact(s) were missing dataset fingerprints/paths for disjointness checks."
        ));
    }

    MetaCohortManifest {
        train_artifacts: train_refs,
        holdout_artifacts: holdout_refs,
        disjointness: MetaCohortDisjointnessSummary {
            train_artifacts: train_artifacts.len(),
            holdout_artifacts: holdout_artifacts.len(),
            overlapping_dataset_keys,
            missing_train_dataset_keys,
            missing_holdout_dataset_keys,
            disjoint,
        },
        holdout_tag: holdout_tag.map(str::to_string),
        campaign_lineage,
        warnings,
    }
}

fn require_meta_holdout_disjointness(
    eval_holdout: bool,
    holdout_artifact_count: usize,
    cohort_manifest: &MetaCohortManifest,
) -> Result<(), String> {
    if !eval_holdout || holdout_artifact_count == 0 {
        return Ok(());
    }

    let disjointness = &cohort_manifest.disjointness;
    if disjointness.overlapping_dataset_keys > 0 {
        return Err(format!(
            "Meta holdout is not sealed: train and holdout cohorts share {} dataset key(s). Move every overlapping dataset into exactly one cohort before evaluating holdout evidence.",
            disjointness.overlapping_dataset_keys
        ));
    }

    let missing_keys = disjointness
        .missing_train_dataset_keys
        .saturating_add(disjointness.missing_holdout_dataset_keys);
    if missing_keys > 0 {
        return Err(format!(
            "Meta holdout disjointness cannot be proven: {missing_keys} artifact(s) are missing both a dataset content fingerprint and a dataset path. Rebuild or repair those artifacts before evaluating holdout evidence."
        ));
    }

    Ok(())
}

fn require_requested_vcet_holdout_artifacts(
    strategy_kind: &str,
    eval_holdout: bool,
    requested_holdout_tag: Option<&str>,
    matched_holdout_artifacts: usize,
) -> Result<(), String> {
    let requested_holdout_tag = requested_holdout_tag
        .map(str::trim)
        .filter(|tag| !tag.is_empty());
    if strategy_kind.trim() != "Vcet"
        || !eval_holdout
        || requested_holdout_tag.is_none()
        || matched_holdout_artifacts > 0
    {
        return Ok(());
    }

    let tag = requested_holdout_tag.unwrap_or_default();
    Err(format!(
        "VCET holdout evaluation requested tag '{tag}', but zero matching holdout validation artifacts remained after the strategy, tag, and timeframe filters. Verify that the campaign saved VCET validation artifacts with this holdout tag and timeframe, select the matching scope, or disable holdout evaluation."
    ))
}

fn require_usable_vcet_holdout_results(
    strategy_kind: &str,
    holdout_tag: &str,
    datasets_requested: usize,
    datasets_evaluated: usize,
) -> Result<(), String> {
    if strategy_kind.trim() != "Vcet" || datasets_evaluated > 0 {
        return Ok(());
    }
    Err(format!(
        "VCET holdout evaluation for tag '{holdout_tag}' produced zero usable dataset results out of {datasets_requested} requested. No Meta XLSX or JSON was published; repair the referenced CSV/validation failures before promotion."
    ))
}

fn stage5_training_artifact_ref(path: &Path, meta: &Value) -> Stage5TrainingArtifactRef {
    let (strategy_kind, timeframe) = extract_meta_scope(meta)
        .map(|(strategy_kind, timeframe)| (Some(strategy_kind), Some(timeframe)))
        .unwrap_or((None, None));
    Stage5TrainingArtifactRef {
        path: path.to_string_lossy().to_string(),
        artifact_sha256: sha256_file_hex_opt(path),
        ts: value_string_at(meta, &["/ts"]),
        strategy_kind,
        timeframe,
    }
}

fn path_matches_source(
    path: &Path,
    source_meta_path: &Path,
    source_canon: Option<&PathBuf>,
) -> bool {
    if let Some(src) = source_canon {
        std::fs::canonicalize(path)
            .map(|p| p.as_path() == src.as_path())
            .unwrap_or(path == source_meta_path)
    } else {
        path == source_meta_path
    }
}

fn parse_rfc3339_utc_ts(raw: Option<&str>) -> Option<chrono::DateTime<chrono::Utc>> {
    let s = raw?.trim();
    if s.is_empty() {
        return None;
    }
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

fn stage4_training_metas_for_policy(
    metas_with_paths: Vec<(PathBuf, Value)>,
    source_meta_path: &Path,
    source_meta: &Value,
    policy: Stage5TrainingHistoryPolicy,
) -> (Vec<Value>, Stage5TrainingManifest) {
    let source_canon = std::fs::canonicalize(source_meta_path).ok();
    let mut out: Vec<Value> = Vec::with_capacity(metas_with_paths.len());
    let mut training_artifacts: Vec<Stage5TrainingArtifactRef> = Vec::new();
    let mut excluded_artifacts: Vec<Stage5ExcludedArtifactRef> = Vec::new();
    let mut excluded_source_count = 0usize;
    let mut excluded_future_or_equal_count = 0usize;
    let mut excluded_missing_ts_count = 0usize;
    let source_artifact = stage5_training_artifact_ref(source_meta_path, source_meta);
    let source_ts = parse_rfc3339_utc_ts(source_artifact.ts.as_deref());

    for (path, meta) in metas_with_paths {
        let artifact_ref = stage5_training_artifact_ref(&path, &meta);
        let matches_source = path_matches_source(&path, source_meta_path, source_canon.as_ref());
        let source_hash_match = artifact_ref
            .artifact_sha256
            .as_ref()
            .zip(source_artifact.artifact_sha256.as_ref())
            .map(|(artifact_hash, source_hash)| artifact_hash == source_hash)
            .unwrap_or(false);
        let source_content_match = &meta == source_meta;
        if matches_source || source_hash_match || source_content_match {
            excluded_source_count = excluded_source_count.saturating_add(1);
            excluded_artifacts.push(Stage5ExcludedArtifactRef {
                artifact: artifact_ref,
                reason: if matches_source {
                    "source_artifact".to_string()
                } else {
                    "source_artifact_duplicate".to_string()
                },
            });
            continue;
        }

        if policy == Stage5TrainingHistoryPolicy::ChronologicalBeforeSource {
            let Some(source_dt) = source_ts else {
                excluded_missing_ts_count = excluded_missing_ts_count.saturating_add(1);
                excluded_artifacts.push(Stage5ExcludedArtifactRef {
                    artifact: artifact_ref,
                    reason: "source_ts_missing_for_chronological_policy".to_string(),
                });
                continue;
            };
            let Some(artifact_dt) = parse_rfc3339_utc_ts(artifact_ref.ts.as_deref()) else {
                excluded_missing_ts_count = excluded_missing_ts_count.saturating_add(1);
                excluded_artifacts.push(Stage5ExcludedArtifactRef {
                    artifact: artifact_ref,
                    reason: "missing_ts_for_chronological_policy".to_string(),
                });
                continue;
            };
            if artifact_dt >= source_dt {
                excluded_future_or_equal_count = excluded_future_or_equal_count.saturating_add(1);
                excluded_artifacts.push(Stage5ExcludedArtifactRef {
                    artifact: artifact_ref,
                    reason: "future_or_equal_ts_for_chronological_policy".to_string(),
                });
                continue;
            }
        }

        training_artifacts.push(artifact_ref);
        out.push(meta);
    }

    let manifest = Stage5TrainingManifest {
        policy,
        source_artifact,
        training_artifact_count: training_artifacts.len(),
        training_artifacts,
        excluded_artifacts,
        excluded_source_count,
        excluded_future_or_equal_count,
        excluded_missing_ts_count,
    };
    (out, manifest)
}

fn push_unique_stage5_warning(warnings: &mut Vec<String>, warning: String) {
    if !warnings.iter().any(|existing| existing == &warning) {
        warnings.push(warning);
    }
}

fn append_stage5_training_manifest_warnings(
    manifest: &Stage5TrainingManifest,
    warnings: &mut Vec<String>,
) {
    if manifest.excluded_source_count > 0 {
        push_unique_stage5_warning(
            warnings,
            format!(
                "Auto-train excluded {} source meta artifact(s) to avoid train/score leakage.",
                manifest.excluded_source_count
            ),
        );
    } else {
        push_unique_stage5_warning(
            warnings,
            "Auto-train did not find the active source meta artifact in saved history; duplicate hash/content checks found no source match.".to_string(),
        );
    }

    if manifest.training_artifact_count == 0 {
        push_unique_stage5_warning(
            warnings,
            "Auto-train has no eligible historical meta artifacts after leakage filters; Stage 5 will fall back to an existing model if available.".to_string(),
        );
    } else if manifest.training_artifact_count < 2 {
        push_unique_stage5_warning(
            warnings,
            format!(
                "Auto-train history is thin: only {} eligible meta artifact(s) after leakage filters.",
                manifest.training_artifact_count
            ),
        );
    }

    if manifest.excluded_future_or_equal_count > 0 {
        push_unique_stage5_warning(
            warnings,
            format!(
                "Auto-train chronological policy excluded {} source-or-future meta artifact(s).",
                manifest.excluded_future_or_equal_count
            ),
        );
    }
    if manifest.excluded_missing_ts_count > 0 {
        push_unique_stage5_warning(
            warnings,
            format!(
                "Auto-train chronological policy excluded {} meta artifact(s) with missing/invalid timestamps.",
                manifest.excluded_missing_ts_count
            ),
        );
    }
}

fn try_load_ml_confidence_model(path: &Path) -> Option<MlConfidenceModel> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<MlConfidenceModel>(&raw).ok()
}

fn try_load_ml_confidence_model_summary(path: &Path) -> Result<MlConfidenceModelSummary, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| e.to_string())
}

fn build_ml_confidence_status(
    exists: bool,
    model_path: String,
    bytes: usize,
    model_summary: Option<MlConfidenceModelSummary>,
    error: Option<String>,
    meta_scan_error: Option<String>,
    meta_summary: &MlMetaScopeSummary,
) -> MlConfidenceModelStatus {
    let available_meta_artifacts = meta_summary.available_meta_artifacts;
    let newest_meta_ts_available = meta_summary.newest_meta_ts_available.clone();
    let available_labeled_rows = if meta_summary.has_holdout_dataset_rows {
        meta_summary.available_labeled_rows_dataset
    } else {
        meta_summary.available_labeled_rows_holdout
    };

    let mut stale = false;
    let mut retrain_recommended = false;
    let mut retrain_reason: Option<String> = None;

    if model_summary.is_none() {
        if !exists {
            retrain_recommended = true;
            retrain_reason = Some("No model saved for this scope.".to_string());
        } else if error.is_some() {
            retrain_recommended = true;
            retrain_reason =
                Some("Model exists but failed to load; retrain recommended.".to_string());
        }
    } else if let Some(summary) = model_summary.as_ref() {
        let meta = summary.meta.as_ref();
        if meta.is_none() {
            stale = available_meta_artifacts > 0;
            if stale && available_labeled_rows >= 25 {
                retrain_recommended = true;
                retrain_reason = Some(
                    "Legacy model (missing training metadata); retrain recommended.".to_string(),
                );
            } else if stale {
                retrain_reason = Some(
                    "Legacy model (missing training metadata); retrain when more labels exist."
                        .to_string(),
                );
            }
        } else if let Some(mm) = meta {
            match (
                mm.compatibility_key.as_ref(),
                meta_summary.active_compatibility_key.as_ref(),
            ) {
                (Some(model_key), Some(active_key)) if model_key != active_key => {
                    stale = true;
                    retrain_recommended = true;
                    retrain_reason = Some(format!(
                        "Model compatibility changed: saved model expects target={} score={} grouping={} shortlist={}, but current compatible history is target={} score={} grouping={} shortlist={}.",
                        model_key.target_family,
                        model_key.train_score_semantics,
                        model_key.grouping_mode,
                        model_key.shortlist_policy,
                        active_key.target_family,
                        active_key.train_score_semantics,
                        active_key.grouping_mode,
                        active_key.shortlist_policy
                    ));
                }
                (None, Some(active_key)) => {
                    stale = true;
                    retrain_recommended = true;
                    retrain_reason = Some(format!(
                        "Model is missing compatibility metadata; current compatible history contract is target={} score={} grouping={} shortlist={}.",
                        active_key.target_family,
                        active_key.train_score_semantics,
                        active_key.grouping_mode,
                        active_key.shortlist_policy
                    ));
                }
                _ => {}
            }
            if let (Some(used), Some(avail)) = (
                mm.newest_meta_ts_used.as_ref(),
                newest_meta_ts_available.as_ref(),
            ) {
                if avail.as_str() > used.as_str() {
                    stale = true;
                }
            } else if newest_meta_ts_available.is_some() {
                stale = true;
            }
            if let Some(used_count) = mm.meta_artifacts_used {
                if available_meta_artifacts > used_count {
                    stale = true;
                }
            } else if available_meta_artifacts > 0 {
                stale = true;
            }

            let trained_rows = mm.trained_rows;
            let new_rows = available_labeled_rows.saturating_sub(trained_rows);
            let threshold = std::cmp::max(25, trained_rows / 10);

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            let cooldown_ok = now_ms.saturating_sub(mm.trained_at_ms) > 60_000;

            if retrain_reason.is_some() {
            } else if stale && new_rows >= threshold && cooldown_ok {
                retrain_recommended = true;
                retrain_reason = Some(format!(
                    "Model stale: new labeled rows {new_rows} (threshold {threshold})."
                ));
            } else if stale && new_rows > 0 {
                retrain_reason = Some(format!(
                    "Model stale: new labeled rows {new_rows} (<{threshold})."
                ));
            } else if stale && !cooldown_ok {
                retrain_reason = Some("Model stale but recently trained (cooldown).".to_string());
            }
        }
    }

    let (state, legacy_model, model_healthy) = classify_ml_confidence_model_state(
        exists,
        model_summary.as_ref(),
        error.as_deref(),
        stale,
        retrain_recommended,
    );
    if retrain_reason.is_none() && meta_summary.skipped_incompatible_artifacts > 0 {
        retrain_reason = Some(format!(
            "Skipped {} historical artifact(s) with incompatible ML-confidence history contracts.",
            meta_summary.skipped_incompatible_artifacts
        ));
    }

    MlConfidenceModelStatus {
        exists,
        model_path,
        bytes,
        available_meta_artifacts,
        available_labeled_rows,
        newest_meta_ts_available,
        stale,
        retrain_recommended,
        state,
        legacy_model,
        model_healthy,
        retrain_reason,
        model_summary,
        meta_scan_error,
        error,
    }
}

fn should_block_stage4_for_unhealthy_model(
    require_healthy_model: bool,
    model_health: Option<bool>,
) -> bool {
    require_healthy_model && matches!(model_health, Some(false))
}

fn stage4_autotrain_failure_fallback(
    train_err: String,
    model_path: &Path,
    existing_model_loaded: bool,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    if existing_model_loaded {
        warnings.push(format!(
            "Auto-train failed ({train_err}); falling back to existing model at '{}'.",
            model_path.to_string_lossy()
        ));
        return Ok(());
    }
    Err(format!(
        "Auto-train failed and no usable existing model is available: {train_err}"
    ))
}

fn ensure_path_within_dir(path: &Path, dir: &Path) -> Result<(), String> {
    let dir = std::fs::canonicalize(dir).map_err(|e| e.to_string())?;
    let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    if !path.starts_with(&dir) {
        return Err("path is outside app runs dir".to_string());
    }
    Ok(())
}

fn suggest_stage5_artifact_path(ts: &str, app: &tauri::AppHandle) -> Result<PathBuf, String> {
    run_artifacts::suggest_stage5_artifact_path(ts, app)
}

#[cfg(test)]
fn is_known_non_run_artifact_filename(name: &str) -> bool {
    run_artifacts::is_known_non_run_artifact_file_name(name)
}

fn suggest_ml_policy_eval_artifact_path(
    ts: &str,
    app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    run_artifacts::suggest_ml_policy_eval_artifact_path(ts, app)
}

fn is_meta_selection_artifact(v: &Value) -> bool {
    v.get("kind")
        .and_then(|x| x.as_str())
        .map(run_artifacts::is_meta_selection_kind)
        .unwrap_or(false)
}

fn group_holdout_label(group: &Value, cfg: &MlConfidenceLabelConfig) -> Option<f64> {
    if group
        .pointer("/holdout_coverage/evaluated")
        .and_then(|v| v.as_bool())
        == Some(false)
    {
        return None;
    }
    let holdout = group.get("holdout")?;
    let datasets = holdout
        .get("datasets")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    if datasets < cfg.min_holdout_datasets {
        return None;
    }

    let pass_rate = holdout.get("pass_rate").and_then(|v| v.as_f64())?;
    let score_p10 = holdout.get("score_p10").and_then(|v| v.as_f64())?;
    let max_dd_p90 = holdout
        .get("max_dd_p90")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let mut ok = pass_rate >= cfg.min_holdout_pass_rate && score_p10 >= cfg.min_holdout_score_p10;
    if let Some(dd_max) = cfg.max_holdout_max_dd_p90 {
        ok = ok && max_dd_p90 <= dd_max;
    }
    Some(if ok { 1.0 } else { 0.0 })
}

#[derive(Debug, Clone)]
struct PolicyEvalCandidate {
    label: f64,
    meta_score: f64,
    ml_confidence: f64,
    ml_logit: f64,
    ml_uncertainty: f64,
}

fn update_policy_arm_metrics(metrics: &mut MlConfidencePolicyArmMetrics, labels: &[f64]) {
    if labels.is_empty() {
        return;
    }
    metrics.evaluated_artifacts = metrics.evaluated_artifacts.saturating_add(1);
    metrics.selected_total = metrics.selected_total.saturating_add(labels.len());
    metrics.positives_total = metrics
        .positives_total
        .saturating_add(labels.iter().filter(|&&label| label > 0.5).count());
    metrics.precision_at_k = if metrics.selected_total == 0 {
        0.0
    } else {
        metrics.positives_total as f64 / metrics.selected_total as f64
    };
}

fn finalize_policy_arm_metrics(
    metrics: &mut MlConfidencePolicyArmMetrics,
    total_label_rows: usize,
    positive_label_rows: usize,
) {
    metrics.selection_rate = if total_label_rows == 0 {
        0.0
    } else {
        metrics.selected_total as f64 / total_label_rows as f64
    };
    metrics.base_rate = if total_label_rows == 0 {
        0.0
    } else {
        positive_label_rows as f64 / total_label_rows as f64
    };
    metrics.lift_vs_base_rate = if metrics.base_rate > 0.0 {
        metrics.precision_at_k / metrics.base_rate
    } else {
        0.0
    };
}

fn normalize_ml_threshold(raw: f64) -> f64 {
    if !raw.is_finite() {
        return 0.0;
    }
    if raw > 1.0 && raw <= 100.0 {
        return (raw / 100.0).clamp(0.0, 1.0);
    }
    raw.clamp(0.0, 1.0)
}

fn normalize_non_negative(raw: f64, default_v: f64) -> f64 {
    if raw.is_finite() && raw >= 0.0 {
        raw
    } else {
        default_v
    }
}

fn stable_logit(p: f64) -> f64 {
    let pp = p.clamp(1e-9, 1.0 - 1e-9);
    (pp / (1.0 - pp)).ln()
}

fn ml_uncertainty_from_prob(p: f64) -> f64 {
    (4.0 * p * (1.0 - p)).clamp(0.0, 1.0)
}

fn mean_std(xs: &[f64]) -> (f64, f64) {
    if xs.is_empty() {
        return (0.0, 1.0);
    }
    let n = xs.len() as f64;
    let mean = xs.iter().copied().sum::<f64>() / n;
    let var = xs
        .iter()
        .copied()
        .map(|x| {
            let d = x - mean;
            d * d
        })
        .sum::<f64>()
        / n;
    let std = var.sqrt();
    if std.is_finite() && std > 1e-12 {
        (mean, std)
    } else {
        (mean, 1.0)
    }
}

fn zscore(x: f64, mean: f64, std: f64) -> f64 {
    (x - mean) / std
}

fn decision_bucket_label(
    req: &MetaStage4Request,
    p: Option<f64>,
    unc: Option<f64>,
) -> &'static str {
    let Some(pp) = p else {
        return "reject";
    };
    let uu = unc.unwrap_or(1.0);
    if pp >= req.accept_confidence && uu <= req.max_accept_uncertainty {
        "accept"
    } else if pp >= req.review_confidence {
        "review"
    } else {
        "reject"
    }
}

fn stage5_evidence_policy_summary() -> EvidencePolicySummary {
    EvidencePolicySummary {
        version: 1,
        workflow: "meta_stage5_ml_confidence".to_string(),
        summary_role: "interpretation_metadata_only".to_string(),
        metrics: vec![
            EvidencePolicyMetric {
                metric: "ml_confidence".to_string(),
                metric_class: EvidenceMetricClass::Approximate,
                gate_role: EvidenceGateRole::RunRecommendation,
                sample_unit: "meta_group_features".to_string(),
                interpretation:
                    "Model-estimated holdout success probability for a meta-selection group."
                        .to_string(),
                caveats: vec![
                    "This is not deployability proof and does not replace holdout review."
                        .to_string(),
                ],
            },
            EvidencePolicyMetric {
                metric: "ml_uncertainty".to_string(),
                metric_class: EvidenceMetricClass::Approximate,
                gate_role: EvidenceGateRole::RunRecommendation,
                sample_unit: "predicted_probability".to_string(),
                interpretation:
                    "Probability-derived uncertainty used by Stage 5 blended ranking."
                        .to_string(),
                caveats: vec![
                    "Uncertainty is derived from model probability and is not an independent calibration guarantee."
                        .to_string(),
                ],
            },
            EvidencePolicyMetric {
                metric: "final_rank_score".to_string(),
                metric_class: EvidenceMetricClass::RankingOnly,
                gate_role: EvidenceGateRole::RunRecommendation,
                sample_unit: "selected_candidate_set".to_string(),
                interpretation:
                    "Weighted rank score combining meta evidence, ML confidence, and uncertainty."
                        .to_string(),
                caveats: vec![
                    "Final rank score orders candidates but is not a hard validation proof.".to_string(),
                ],
            },
            EvidencePolicyMetric {
                metric: "decision_bucket".to_string(),
                metric_class: EvidenceMetricClass::MetadataOnly,
                gate_role: EvidenceGateRole::Advisory,
                sample_unit: "selected_group".to_string(),
                interpretation:
                    "Threshold bucket derived from confidence and uncertainty settings.".to_string(),
                caveats: vec![
                    "Decision buckets are review labels, not execution approval or deployability proof."
                        .to_string(),
                ],
            },
            EvidencePolicyMetric {
                metric: "training_history_policy".to_string(),
                metric_class: EvidenceMetricClass::MetadataOnly,
                gate_role: EvidenceGateRole::Provenance,
                sample_unit: "meta_artifact_history".to_string(),
                interpretation:
                    "Training-history policy records how source/future artifacts are excluded."
                        .to_string(),
                caveats: vec![
                    "The policy documents leakage controls without changing saved artifact replay semantics."
                        .to_string(),
                ],
            },
        ],
        caveats: vec![
            "Stage 5 labels are additive metadata and do not change ranking, thresholds, replay, or saved artifact compatibility.".to_string(),
            "ML confidence is a model-estimated holdout success probability, not deployability proof.".to_string(),
        ],
    }
}

fn annotate_stage5_group_aliases(
    obj: &mut serde_json::Map<String, Value>,
    rank: Option<usize>,
    fallback: bool,
    final_rank_score: Option<f64>,
) {
    obj.insert("stage4_fallback".to_string(), Value::Bool(fallback));
    obj.insert("stage5_fallback".to_string(), Value::Bool(fallback));
    if let Some(rank) = rank {
        let rank = serde_json::Number::from(rank as u64);
        obj.insert("stage4_rank".to_string(), Value::Number(rank.clone()));
        obj.insert("stage5_rank".to_string(), Value::Number(rank));
    }
    if let Some(final_rank_score) = final_rank_score {
        obj.insert(
            "final_rank_score".to_string(),
            serde_json::json!(final_rank_score),
        );
    }
}

fn extract_meta_scope(meta: &Value) -> Option<(String, String)> {
    let strat = meta
        .pointer("/req/strategy_kind")
        .and_then(|v| v.as_str())
        .or_else(|| meta.pointer("/res/strategy_kind").and_then(|v| v.as_str()))
        .unwrap_or("")
        .trim()
        .to_string();
    let tf_raw = meta
        .pointer("/req/timeframe")
        .and_then(|v| v.as_str())
        .or_else(|| meta.pointer("/res/timeframe").and_then(|v| v.as_str()))
        .unwrap_or("")
        .trim()
        .to_string();
    let tf = normalize_timeframe_label(&tf_raw);
    if strat.is_empty() || tf.is_empty() {
        return None;
    }
    Some((strat, tf))
}

fn meta_candidate_group_features(
    group: &ta_app_core::MetaCandidateAggregate,
) -> MlConfidenceGroupFeatures {
    MlConfidenceGroupFeatures {
        samples: group.samples as u64,
        pass_rate: group.pass_rate,
        robust_score_p10: group.robust_score_p10,
        robust_score_median: group.robust_score_median,
        max_dd_p90: group.max_dd_p90,
        cross_score_median: group.cross_score_median,
        meta_score: group.meta_score,
    }
}

#[derive(Debug, Clone, Default)]
struct MlMetaScopeSummary {
    available_meta_artifacts: usize,
    newest_meta_ts_available: Option<String>,
    available_labeled_rows_holdout: usize,
    available_labeled_rows_dataset: usize,
    has_holdout_dataset_rows: bool,
    active_compatibility_key: Option<MlConfidenceCompatibilityKey>,
    skipped_incompatible_artifacts: usize,
}

fn summarize_meta_artifacts_for_scope(
    metas: &[Value],
    scope: &MlConfidenceScope,
    cfg: &MlConfidenceLabelConfig,
) -> MlMetaScopeSummary {
    let prepared = ta_app_core::prepare_ml_confidence_rows_from_meta_artifacts(scope, metas, cfg);
    let count = prepared.artifacts.len();
    let newest = prepared
        .artifacts
        .iter()
        .filter_map(|artifact| artifact.ts.clone())
        .max();
    let labeled_rows_holdout = prepared
        .artifacts
        .iter()
        .map(|artifact| artifact.holdout_gate_rows.len())
        .sum();
    let labeled_rows_dataset = prepared
        .artifacts
        .iter()
        .map(|artifact| artifact.holdout_dataset_rows.len())
        .sum();

    MlMetaScopeSummary {
        available_meta_artifacts: count,
        newest_meta_ts_available: newest,
        available_labeled_rows_holdout: labeled_rows_holdout,
        available_labeled_rows_dataset: labeled_rows_dataset,
        has_holdout_dataset_rows: prepared.has_holdout_dataset_rows,
        active_compatibility_key: prepared.compatibility_key,
        skipped_incompatible_artifacts: prepared.skipped_incompatible_artifacts,
    }
}

fn ml_confidence_model_status_blocking(
    scope: MlConfidenceScope,
    app: &tauri::AppHandle,
) -> Result<MlConfidenceModelStatus, String> {
    let scope = normalize_ml_scope(scope)?;
    let path = ml_confidence_model_path_read_only(&scope, app)?;
    let model_path = path.to_string_lossy().to_string();
    let exists = path.exists();

    let mut bytes = 0usize;
    let mut model_summary: Option<MlConfidenceModelSummary> = None;
    let mut error: Option<String> = None;
    let mut label_cfg = MlConfidenceLabelConfig::default();

    if exists {
        bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len() as usize;
        match try_load_ml_confidence_model_summary(&path) {
            Ok(summary) => {
                label_cfg = summary.label.clone();
                model_summary = Some(summary);
            }
            Err(e) => {
                error = Some(e);
            }
        }
    }

    let (meta_summary, meta_scan_error) = match load_meta_artifacts_blocking(app) {
        Ok(metas) => (
            summarize_meta_artifacts_for_scope(&metas, &scope, &label_cfg),
            None,
        ),
        Err(e) => (MlMetaScopeSummary::default(), Some(e)),
    };

    Ok(build_ml_confidence_status(
        exists,
        model_path,
        bytes,
        model_summary,
        error,
        meta_scan_error,
        &meta_summary,
    ))
}

#[tauri::command]
pub async fn ml_confidence_model_status(
    scope: MlConfidenceScope,
    app: tauri::AppHandle,
) -> Result<MlConfidenceModelStatus, String> {
    tauri::async_runtime::spawn_blocking(move || ml_confidence_model_status_blocking(scope, &app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn train_ml_confidence_model(
    scope: MlConfidenceScope,
    cfg: Option<MlConfidenceTrainConfig>,
    app: tauri::AppHandle,
) -> Result<TrainMlConfidenceResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let cfg = cfg.unwrap_or_default();
        let metas = load_meta_artifacts_blocking(&app)?;
        let (model, metrics) =
            ta_app_core::train_ml_confidence_model_from_meta_artifacts(scope.clone(), &metas, cfg)?;

        let path = ml_confidence_model_path(&model.scope, &app)?;
        let bytes = serde_json::to_vec_pretty(&model).map_err(|e| e.to_string())?;
        write_bytes_atomically(&path, &bytes)?;
        let model_summary = MlConfidenceModelSummary::from(&model);
        let meta_summary = summarize_meta_artifacts_for_scope(&metas, &scope, &model.label);
        let status = build_ml_confidence_status(
            true,
            path.to_string_lossy().to_string(),
            bytes.len(),
            Some(model_summary.clone()),
            None,
            None,
            &meta_summary,
        );

        Ok(TrainMlConfidenceResponse {
            model_path: path.to_string_lossy().to_string(),
            model_bytes: bytes.len(),
            model_summary,
            metrics,
            status,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_ml_confidence_model(
    scope: MlConfidenceScope,
    groups: Vec<MlConfidenceGroupFeatures>,
    app: tauri::AppHandle,
) -> Result<ApplyMlConfidenceResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let path = ml_confidence_model_path_read_only(&scope, &app)?;
        if !path.exists() {
            return Err("ML confidence model not found. Train it first.".to_string());
        }
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let model: MlConfidenceModel = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

        let mut warnings: Vec<String> = Vec::new();
        if normalize_timeframe_label(&model.scope.timeframe) != scope.timeframe
            || model.scope.strategy_kind != scope.strategy_kind
        {
            warnings.push("Model scope does not match request scope. Results may be unreliable.".to_string());
        }
        if let Some(meta) = model.meta.as_ref() {
            if !meta.healthy {
                warnings.push("Model health is poor (AUC/Brier gates failed). Treat ML confidence as untrustworthy.".to_string());
            }
        }

        let confidences: Vec<Option<f64>> = groups
            .iter()
            .map(|g| ta_app_core::predict_ml_confidence_for_features(&model, g))
            .collect();

        Ok(ApplyMlConfidenceResponse {
            model_path: path.to_string_lossy().to_string(),
            confidences,
            warnings,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_ml_confidence_model(
    scope: MlConfidenceScope,
    app: tauri::AppHandle,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let path = ml_confidence_model_path_read_only(&scope, &app)?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn normalize_meta_stage4_request(mut req: MetaStage4Request) -> Result<MetaStage4Request, String> {
    req.strategy_kind = req.strategy_kind.trim().to_string();
    if req.strategy_kind.is_empty() {
        return Err("strategy_kind is required".to_string());
    }

    req.timeframe = req
        .timeframe
        .as_deref()
        .map(normalize_timeframe_label)
        .filter(|s| !s.is_empty());

    req.source_meta_path = req
        .source_meta_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    req.top_k = req.top_k.max(1);
    req.min_ml_confidence = normalize_ml_threshold(req.min_ml_confidence);
    req.accept_confidence = normalize_ml_threshold(req.accept_confidence);
    req.review_confidence = normalize_ml_threshold(req.review_confidence);
    req.max_accept_uncertainty = normalize_ml_threshold(req.max_accept_uncertainty);
    if req.review_confidence > req.accept_confidence {
        req.review_confidence = req.accept_confidence;
    }

    req.meta_weight = normalize_non_negative(req.meta_weight, default_stage4_meta_weight());
    req.ml_weight = normalize_non_negative(req.ml_weight, default_stage4_ml_weight());
    req.uncertainty_weight =
        normalize_non_negative(req.uncertainty_weight, default_stage4_uncertainty_weight());
    let sum_w = req.meta_weight + req.ml_weight + req.uncertainty_weight;
    if !(sum_w.is_finite() && sum_w > 0.0) {
        req.meta_weight = default_stage4_meta_weight();
        req.ml_weight = default_stage4_ml_weight();
        req.uncertainty_weight = default_stage4_uncertainty_weight();
    } else {
        req.meta_weight /= sum_w;
        req.ml_weight /= sum_w;
        req.uncertainty_weight /= sum_w;
    }

    if !req.auto_train_model {
        req.auto_retrain_stale = false;
    }
    Ok(req)
}

fn resolve_meta_stage4_source(
    req: &MetaStage4Request,
    app: &tauri::AppHandle,
) -> Result<(PathBuf, Value, String), String> {
    let runs_dir = run_artifacts::runs_dir(app)?;
    let strategy_kind = req.strategy_kind.trim();
    let req_tf = req.timeframe.as_deref().unwrap_or("");

    if let Some(path_raw) = req.source_meta_path.as_deref() {
        let path = PathBuf::from(path_raw);
        ensure_path_within_dir(&path, &runs_dir)?;
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("");
        if !run_artifacts::is_meta_selection_kind(kind) {
            return Err(format!(
                "source_meta_path must point to a meta-selection artifact kind ('vectorgrid_meta_selection' or legacy 'vectorbt_meta_selection') (got '{kind}')."
            ));
        }
        let Some((sk, tf)) = extract_meta_scope(&v) else {
            return Err(
                "source meta artifact is missing scope metadata (strategy/timeframe)".to_string(),
            );
        };
        if sk != strategy_kind {
            return Err(format!(
                "source meta strategy mismatch: expected '{strategy_kind}', got '{sk}'."
            ));
        }
        if !req_tf.is_empty() && tf != req_tf {
            return Err(format!(
                "source meta timeframe mismatch: expected '{req_tf}', got '{tf}'."
            ));
        }
        return Ok((path, v, tf));
    }

    let metas = load_meta_artifacts_with_paths_blocking(app)?;
    let mut chosen: Option<(String, PathBuf, Value, String)> = None;
    for (path, v) in metas {
        let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("");
        if !run_artifacts::is_meta_selection_kind(kind) {
            continue;
        }
        let Some((sk, tf)) = extract_meta_scope(&v) else {
            continue;
        };
        if sk != strategy_kind {
            continue;
        }
        if !req_tf.is_empty() && tf != req_tf {
            continue;
        }
        let ts = v
            .get("ts")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let take = chosen
            .as_ref()
            .map(|(best_ts, _, _, _)| ts.as_str() > best_ts.as_str())
            .unwrap_or(true);
        if take {
            chosen = Some((ts, path, v, tf));
        }
    }

    if let Some((_ts, path, v, tf)) = chosen {
        return Ok((path, v, tf));
    }
    if req_tf.is_empty() {
        Err(format!(
            "No saved meta artifact found for strategy_kind='{strategy_kind}'. Run meta analysis first."
        ))
    } else {
        Err(format!(
            "No saved meta artifact found for strategy_kind='{strategy_kind}' timeframe='{req_tf}'."
        ))
    }
}

fn group_rank_score(group: &Value) -> f64 {
    group
        .get("final_score")
        .and_then(|v| v.as_f64())
        .or_else(|| group.get("meta_score").and_then(|v| v.as_f64()))
        .unwrap_or(f64::NEG_INFINITY)
}

fn group_policy_eval_meta_score(group: &Value) -> f64 {
    group
        .get("meta_score")
        .and_then(|value| value.as_f64())
        .unwrap_or(f64::NEG_INFINITY)
}

fn normalize_ml_policy_evaluation_request(
    mut req: MlConfidencePolicyEvaluationRequest,
) -> Result<MlConfidencePolicyEvaluationRequest, String> {
    req.strategy_kind = req.strategy_kind.trim().to_string();
    if req.strategy_kind.is_empty() {
        return Err("strategy_kind is required".to_string());
    }
    let timeframe = req
        .timeframe
        .as_deref()
        .map(normalize_timeframe_label)
        .unwrap_or_default();
    if timeframe.is_empty() {
        return Err("timeframe is required (e.g. '1m')".to_string());
    }
    req.timeframe = Some(timeframe);
    req.top_k = req.top_k.max(1);
    req.min_ml_confidence = normalize_ml_threshold(req.min_ml_confidence);
    req.meta_weight = normalize_non_negative(req.meta_weight, default_stage4_meta_weight());
    req.ml_weight = normalize_non_negative(req.ml_weight, default_stage4_ml_weight());
    req.uncertainty_weight =
        normalize_non_negative(req.uncertainty_weight, default_stage4_uncertainty_weight());
    if req.meta_weight == 0.0 && req.ml_weight == 0.0 && req.uncertainty_weight == 0.0 {
        req.meta_weight = default_stage4_meta_weight();
        req.ml_weight = default_stage4_ml_weight();
        req.uncertainty_weight = default_stage4_uncertainty_weight();
    }
    Ok(req)
}

#[derive(Clone, Copy)]
enum PolicyEvalTieBreak {
    PreserveInputOrder,
    MetaThenInputOrder,
}

fn select_top_labels_by_metric<F>(
    candidates: &[PolicyEvalCandidate],
    top_k: usize,
    tie_break: PolicyEvalTieBreak,
    mut score_of: F,
) -> Vec<f64>
where
    F: FnMut(&PolicyEvalCandidate) -> f64,
{
    let mut ranked: Vec<(usize, &PolicyEvalCandidate)> = candidates.iter().enumerate().collect();
    ranked.sort_by(|(lhs_idx, a), (rhs_idx, b)| {
        let lhs = score_of(a);
        let rhs = score_of(b);
        let tie_break = match tie_break {
            PolicyEvalTieBreak::PreserveInputOrder => lhs_idx.cmp(rhs_idx),
            PolicyEvalTieBreak::MetaThenInputOrder => b
                .meta_score
                .total_cmp(&a.meta_score)
                .then_with(|| lhs_idx.cmp(rhs_idx)),
        };
        rhs.total_cmp(&lhs).then_with(|| tie_break)
    });
    ranked
        .into_iter()
        .take(top_k)
        .map(|(_, candidate)| candidate.label)
        .collect()
}

fn evaluate_policy_candidates(
    req: &MlConfidencePolicyEvaluationRequest,
    candidates: &[PolicyEvalCandidate],
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let meta_labels = select_top_labels_by_metric(
        candidates,
        req.top_k,
        PolicyEvalTieBreak::PreserveInputOrder,
        |candidate| candidate.meta_score,
    );
    let ml_labels = select_top_labels_by_metric(
        candidates,
        req.top_k,
        PolicyEvalTieBreak::MetaThenInputOrder,
        |candidate| candidate.ml_confidence,
    );

    let candidate_indices: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter_map(|(idx, candidate)| {
            (candidate.ml_confidence >= req.min_ml_confidence).then_some(idx)
        })
        .collect();

    let stage5_labels = if !candidate_indices.is_empty() {
        let meta_vals: Vec<f64> = candidate_indices
            .iter()
            .map(|&idx| candidates[idx].meta_score)
            .collect();
        let ml_vals: Vec<f64> = candidate_indices
            .iter()
            .map(|&idx| candidates[idx].ml_logit)
            .collect();
        let unc_vals: Vec<f64> = candidate_indices
            .iter()
            .map(|&idx| candidates[idx].ml_uncertainty)
            .collect();

        let (meta_mean, meta_std) = mean_std(&meta_vals);
        let (ml_mean, ml_std) = mean_std(&ml_vals);
        let (unc_mean, unc_std) = mean_std(&unc_vals);

        let mut ranked: Vec<(usize, f64)> = candidate_indices
            .into_iter()
            .map(|idx| {
                let candidate = &candidates[idx];
                let score = req.meta_weight * zscore(candidate.meta_score, meta_mean, meta_std)
                    + req.ml_weight * zscore(candidate.ml_logit, ml_mean, ml_std)
                    - req.uncertainty_weight * zscore(candidate.ml_uncertainty, unc_mean, unc_std);
                (idx, score)
            })
            .collect();
        ranked.sort_by(|(lhs_idx, lhs_score), (rhs_idx, rhs_score)| {
            rhs_score
                .total_cmp(lhs_score)
                .then_with(|| {
                    candidates[*rhs_idx]
                        .ml_confidence
                        .total_cmp(&candidates[*lhs_idx].ml_confidence)
                })
                .then_with(|| {
                    candidates[*rhs_idx]
                        .meta_score
                        .total_cmp(&candidates[*lhs_idx].meta_score)
                })
        });
        ranked
            .into_iter()
            .take(req.top_k)
            .map(|(idx, _)| candidates[idx].label)
            .collect()
    } else if req.fallback_to_meta_if_empty {
        meta_labels.clone()
    } else {
        Vec::new()
    };

    (meta_labels, ml_labels, stage5_labels)
}

fn evaluate_ml_confidence_policy_from_scoped_metas(
    req: &MlConfidencePolicyEvaluationRequest,
    scope: &MlConfidenceScope,
    label_cfg: &MlConfidenceLabelConfig,
    scoped: &[Value],
) -> MlConfidencePolicyEvaluationResult {
    let prepared_rows =
        ta_app_core::prepare_ml_confidence_rows_from_meta_artifacts(scope, scoped, label_cfg);

    let artifacts_considered = scoped.len();
    let mut artifacts_evaluated = 0usize;
    let mut artifacts_skipped_no_history = 0usize;
    let mut artifacts_skipped_training_failure = 0usize;
    let mut artifacts_skipped_unhealthy_model = 0usize;
    let mut artifacts_skipped_no_labels = 0usize;
    let mut total_label_rows = 0usize;
    let mut positive_label_rows = 0usize;
    let mut warnings: Vec<String> = Vec::new();

    let mut meta_only = MlConfidencePolicyArmMetrics::default();
    let mut ml_only = MlConfidencePolicyArmMetrics::default();
    let mut stage5_blended = MlConfidencePolicyArmMetrics::default();

    for idx in 0..scoped.len() {
        if idx == 0 {
            artifacts_skipped_no_history = artifacts_skipped_no_history.saturating_add(1);
            continue;
        }

        let mut train_cfg = MlConfidenceTrainConfig::default();
        train_cfg.label = label_cfg.clone();

        let (model, metrics) = match ta_app_core::train_ml_confidence_model_from_prepared_rows(
            scope.clone(),
            &prepared_rows,
            train_cfg,
            Some(idx),
        ) {
            Ok(v) => v,
            Err(err) => {
                artifacts_skipped_training_failure =
                    artifacts_skipped_training_failure.saturating_add(1);
                warnings.push(format!(
                    "Skipped artifact {} due to training failure: {err}",
                    scoped[idx]
                        .get("ts")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                ));
                continue;
            }
        };

        if req.require_healthy_model && !metrics.healthy {
            artifacts_skipped_unhealthy_model = artifacts_skipped_unhealthy_model.saturating_add(1);
            continue;
        }

        let groups = scoped[idx]
            .pointer("/res/groups")
            .and_then(|v| v.as_array())
            .map(|groups| groups.as_slice())
            .unwrap_or(&[]);
        let mut candidates: Vec<PolicyEvalCandidate> = Vec::new();
        for group in groups {
            let Some(label) = group_holdout_label(group, label_cfg) else {
                continue;
            };
            let Some(features) = ta_app_core::extract_ml_confidence_group_features(group) else {
                continue;
            };
            let Some(probability) =
                ta_app_core::predict_ml_confidence_for_features(&model, &features)
                    .filter(|p| p.is_finite())
            else {
                continue;
            };
            let raw_meta_score = group_policy_eval_meta_score(group);
            let meta_score = if raw_meta_score.is_finite() {
                raw_meta_score
            } else {
                0.0
            };
            candidates.push(PolicyEvalCandidate {
                label,
                meta_score,
                ml_confidence: probability,
                ml_logit: stable_logit(probability),
                ml_uncertainty: ml_uncertainty_from_prob(probability),
            });
        }

        if candidates.is_empty() {
            artifacts_skipped_no_labels = artifacts_skipped_no_labels.saturating_add(1);
            continue;
        }

        total_label_rows = total_label_rows.saturating_add(candidates.len());
        positive_label_rows = positive_label_rows.saturating_add(
            candidates
                .iter()
                .filter(|candidate| candidate.label > 0.5)
                .count(),
        );
        let (meta_labels, ml_labels, stage5_labels) = evaluate_policy_candidates(req, &candidates);
        update_policy_arm_metrics(&mut meta_only, &meta_labels);
        update_policy_arm_metrics(&mut ml_only, &ml_labels);
        update_policy_arm_metrics(&mut stage5_blended, &stage5_labels);
        artifacts_evaluated = artifacts_evaluated.saturating_add(1);
    }

    if artifacts_considered == 0 {
        warnings.push("No meta-selection artifacts found for this scope.".to_string());
    } else if artifacts_evaluated == 0 {
        warnings.push(
            "No scoped artifacts produced a comparable policy-evaluation result.".to_string(),
        );
    }
    let base_rate = if total_label_rows == 0 {
        0.0
    } else {
        positive_label_rows as f64 / total_label_rows as f64
    };
    finalize_policy_arm_metrics(&mut meta_only, total_label_rows, positive_label_rows);
    finalize_policy_arm_metrics(&mut ml_only, total_label_rows, positive_label_rows);
    finalize_policy_arm_metrics(&mut stage5_blended, total_label_rows, positive_label_rows);

    let meta_precision = meta_only.precision_at_k;
    let ml_precision = ml_only.precision_at_k;
    let stage5_precision = stage5_blended.precision_at_k;
    let best_policy = if stage5_precision >= meta_precision && stage5_precision >= ml_precision {
        "stage5_blended"
    } else if meta_precision >= ml_precision {
        "meta_only"
    } else {
        "ml_only"
    }
    .to_string();

    MlConfidencePolicyEvaluationResult {
        strategy_kind: scope.strategy_kind.clone(),
        timeframe: scope.timeframe.clone(),
        artifacts_considered,
        artifacts_evaluated,
        artifacts_skipped_no_history,
        artifacts_skipped_training_failure,
        artifacts_skipped_unhealthy_model,
        artifacts_skipped_no_labels,
        total_label_rows,
        positive_label_rows,
        base_rate,
        top_k: req.top_k,
        min_ml_confidence: req.min_ml_confidence,
        meta_weight: req.meta_weight,
        ml_weight: req.ml_weight,
        uncertainty_weight: req.uncertainty_weight,
        meta_only,
        ml_only,
        stage5_blended,
        best_policy,
        stage5_vs_meta_precision_delta: stage5_precision - meta_precision,
        stage5_vs_ml_precision_delta: stage5_precision - ml_precision,
        warnings,
    }
}

fn run_ml_confidence_policy_evaluation_blocking(
    req: MlConfidencePolicyEvaluationRequest,
    app: tauri::AppHandle,
) -> Result<RunMlConfidencePolicyEvaluationResponse, String> {
    let req = normalize_ml_policy_evaluation_request(req)?;
    let scope = normalize_ml_scope(MlConfidenceScope {
        strategy_kind: req.strategy_kind.clone(),
        timeframe: req.timeframe.clone().unwrap_or_default(),
    })?;
    let label_cfg = req.label.clone().unwrap_or_default();

    let metas = load_meta_artifacts_blocking(&app)?;
    let mut scoped: Vec<Value> = metas
        .into_iter()
        .filter(|meta| {
            is_meta_selection_artifact(meta)
                && extract_meta_scope(meta)
                    .map(|(strategy_kind, timeframe)| {
                        strategy_kind == scope.strategy_kind && timeframe == scope.timeframe
                    })
                    .unwrap_or(false)
        })
        .collect();
    scoped.sort_by(|lhs, rhs| {
        let lhs_ts = lhs.get("ts").and_then(|v| v.as_str()).unwrap_or("");
        let rhs_ts = rhs.get("ts").and_then(|v| v.as_str()).unwrap_or("");
        lhs_ts.cmp(rhs_ts)
    });
    let result = evaluate_ml_confidence_policy_from_scoped_metas(&req, &scope, &label_cfg, &scoped);

    let ts = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::json!({
        "kind": "vectorgrid_ml_confidence_policy_evaluation",
        "ts": ts,
        "req": req,
        "res": result,
    });
    let out_bytes = serde_json::to_vec_pretty(&payload).map_err(|e| e.to_string())?;
    let _artifact_library_lock = run_artifacts::acquire_artifact_library_process_lock(&app)?;
    let out_path = suggest_ml_policy_eval_artifact_path(&ts, &app)?;
    run_artifacts::invalidate_artifact_catalog_under_process_lock(&app)?;
    run_artifacts::publish_new_artifact_file_atomically(&out_path, &out_bytes)?;
    run_artifacts::prune_old_ml_policy_eval_artifacts_under_process_lock(
        &app,
        run_artifacts::artifact_retention_policy_for_app(&app).ml_policy_eval_keep_newest,
    );

    Ok(RunMlConfidencePolicyEvaluationResponse {
        ts,
        eval_json_path: out_path.to_string_lossy().to_string(),
        eval_json_bytes: out_bytes.len(),
        result,
    })
}

fn run_meta_stage5_ml_confidence_blocking(
    req: MetaStage5Request,
    app: tauri::AppHandle,
) -> Result<RunMetaStage5Response, String> {
    let req = normalize_meta_stage4_request(req)?;
    let (source_meta_path, source_meta, meta_tf) = resolve_meta_stage4_source(&req, &app)?;
    let timeframe = req.timeframe.clone().unwrap_or(meta_tf);
    let scope = normalize_ml_scope(MlConfidenceScope {
        strategy_kind: req.strategy_kind.clone(),
        timeframe,
    })?;

    let mut warnings: Vec<String> = Vec::new();
    let mut model_trained_now = false;
    let model_path = ml_confidence_model_path(&scope, &app)?;
    let mut model: Option<MlConfidenceModel> = None;
    let mut training_manifest = Stage5TrainingManifest {
        policy: req.training_history_policy,
        source_artifact: stage5_training_artifact_ref(&source_meta_path, &source_meta),
        training_artifacts: Vec::new(),
        excluded_artifacts: Vec::new(),
        training_artifact_count: 0,
        excluded_source_count: 0,
        excluded_future_or_equal_count: 0,
        excluded_missing_ts_count: 0,
    };

    let mut status = ml_confidence_model_status_blocking(scope.clone(), &app)?;
    let should_train = req.auto_train_model
        && (!status.exists
            || status.error.is_some()
            || (req.auto_retrain_stale && status.retrain_recommended));
    if should_train {
        let metas_with_paths = load_meta_artifacts_with_paths_blocking(&app)?;
        let all_metas: Vec<Value> = metas_with_paths
            .iter()
            .map(|(_, meta)| meta.clone())
            .collect();
        let (metas, manifest) = stage4_training_metas_for_policy(
            metas_with_paths,
            &source_meta_path,
            &source_meta,
            req.training_history_policy,
        );
        training_manifest = manifest;
        append_stage5_training_manifest_warnings(&training_manifest, &mut warnings);
        let cfg = MlConfidenceTrainConfig::default();
        match ta_app_core::train_ml_confidence_model_from_meta_artifacts(scope.clone(), &metas, cfg)
        {
            Ok((trained_model, _metrics)) => {
                let bytes = serde_json::to_vec_pretty(&trained_model).map_err(|e| e.to_string())?;
                write_bytes_atomically(&model_path, &bytes)?;
                let meta_summary =
                    summarize_meta_artifacts_for_scope(&all_metas, &scope, &trained_model.label);
                model = Some(trained_model);
                model_trained_now = true;
                status = build_ml_confidence_status(
                    true,
                    model_path.to_string_lossy().to_string(),
                    bytes.len(),
                    model.as_ref().map(MlConfidenceModelSummary::from),
                    None,
                    None,
                    &meta_summary,
                );
            }
            Err(e) => {
                if model.is_none() {
                    model = try_load_ml_confidence_model(&model_path);
                }
                stage4_autotrain_failure_fallback(e, &model_path, model.is_some(), &mut warnings)?;
            }
        }
    }

    let model: MlConfidenceModel = if let Some(m) = model {
        m
    } else {
        if !model_path.exists() {
            return Err(
                "ML confidence model not found for this scope. Train one first (or enable auto-train)."
                    .to_string(),
            );
        }
        let model_raw = std::fs::read_to_string(&model_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&model_raw).map_err(|e| e.to_string())?
    };
    let model_health = model.meta.as_ref().map(|m| m.healthy);
    let model_healthy = model_health.unwrap_or(false);
    let model_state = status.state.clone();
    if should_block_stage4_for_unhealthy_model(req.require_healthy_model, model_health) {
        return Err(
            "ML model is unhealthy for this scope. Retrain with more labeled rows or disable strict healthy-model requirement."
                .to_string(),
        );
    }
    match model_health {
        Some(true) => {}
        Some(false) => {
            warnings.push(
                "Using an unhealthy ML model (AUC/Brier gates failed). Treat Stage 5 ranking as experimental."
                    .to_string(),
            );
        }
        None => {
            warnings.push(
                "Using a legacy ML model without health metadata; strict healthy-model gate is skipped for compatibility."
                    .to_string(),
            );
        }
    }
    if status.stale {
        warnings.push("ML model is stale relative to available meta artifacts.".to_string());
    }

    let groups = source_meta
        .pointer("/res/groups")
        .and_then(|v| v.as_array())
        .map(|xs| xs.as_slice())
        .unwrap_or(&[]);
    if groups.is_empty() {
        return Err("Source meta artifact has no groups to rank.".to_string());
    }

    #[derive(Clone)]
    struct Stage4Entry {
        ml: Option<f64>,
        ml_logit: Option<f64>,
        ml_uncertainty: Option<f64>,
        meta_score: f64,
        final_rank_score: Option<f64>,
        group: Value,
    }

    let mut entries: Vec<Stage4Entry> = Vec::with_capacity(groups.len());
    let mut groups_scored = 0usize;
    let mut groups_bad_meta_score = 0usize;
    for g in groups {
        let ml = ta_app_core::extract_ml_confidence_group_features(g)
            .and_then(|features| ta_app_core::predict_ml_confidence_for_features(&model, &features))
            .filter(|p| p.is_finite());
        if ml.is_some() {
            groups_scored = groups_scored.saturating_add(1);
        }
        let ml_logit = ml.map(stable_logit);
        let ml_uncertainty = ml.map(ml_uncertainty_from_prob);
        let raw_meta_score = group_rank_score(g);
        let meta_score = if raw_meta_score.is_finite() {
            raw_meta_score
        } else {
            groups_bad_meta_score = groups_bad_meta_score.saturating_add(1);
            0.0
        };
        let mut g2 = g.clone();
        if let Some(obj) = g2.as_object_mut() {
            if let Some(p) = ml {
                obj.insert("ml_confidence".to_string(), serde_json::json!(p));
            } else {
                obj.insert("ml_confidence".to_string(), Value::Null);
            }
            if let Some(u) = ml_uncertainty {
                obj.insert("ml_uncertainty".to_string(), serde_json::json!(u));
            } else {
                obj.insert("ml_uncertainty".to_string(), Value::Null);
            }
            obj.insert(
                "ml_confidence_semantics".to_string(),
                serde_json::json!("model_estimated_holdout_success_probability"),
            );
            obj.insert(
                "stage5_decision_semantics".to_string(),
                serde_json::json!("review_label_not_deployability_proof"),
            );
            obj.insert("final_rank_score".to_string(), Value::Null);
        }
        entries.push(Stage4Entry {
            ml,
            ml_logit,
            ml_uncertainty,
            meta_score,
            final_rank_score: None,
            group: g2,
        });
    }

    if groups_bad_meta_score > 0 {
        warnings.push(format!(
            "{} group(s) had non-finite meta score; fallback value 0.0 was used.",
            groups_bad_meta_score
        ));
    }

    let candidate_indices: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.ml.filter(|p| *p >= req.min_ml_confidence).map(|_| i))
        .collect();

    if !candidate_indices.is_empty() {
        let meta_vals: Vec<f64> = candidate_indices
            .iter()
            .map(|&i| entries[i].meta_score)
            .collect();
        let ml_vals: Vec<f64> = candidate_indices
            .iter()
            .filter_map(|&i| entries[i].ml_logit)
            .collect();
        let unc_vals: Vec<f64> = candidate_indices
            .iter()
            .filter_map(|&i| entries[i].ml_uncertainty)
            .collect();

        let (meta_mean, meta_std) = mean_std(&meta_vals);
        let (ml_mean, ml_std) = mean_std(&ml_vals);
        let (unc_mean, unc_std) = mean_std(&unc_vals);

        for &i in &candidate_indices {
            let z_meta = zscore(entries[i].meta_score, meta_mean, meta_std);
            let z_ml = zscore(entries[i].ml_logit.unwrap_or(0.0), ml_mean, ml_std);
            let z_unc = zscore(entries[i].ml_uncertainty.unwrap_or(0.0), unc_mean, unc_std);
            let final_rank =
                req.meta_weight * z_meta + req.ml_weight * z_ml - req.uncertainty_weight * z_unc;
            entries[i].final_rank_score = Some(final_rank);
            if let Some(obj) = entries[i].group.as_object_mut() {
                obj.insert(
                    "final_rank_score".to_string(),
                    serde_json::json!(final_rank),
                );
                annotate_stage5_group_aliases(obj, None, false, None);
            }
        }
    }

    let mut selected_groups: Vec<Value> = Vec::new();
    let mut fallback_reason: Option<String> = None;
    if !candidate_indices.is_empty() {
        let mut ranked = candidate_indices;
        ranked.sort_by(|&a, &b| {
            let af = entries[a].final_rank_score.unwrap_or(f64::NEG_INFINITY);
            let bf = entries[b].final_rank_score.unwrap_or(f64::NEG_INFINITY);
            let ap = entries[a].ml.unwrap_or(-1.0);
            let bp = entries[b].ml.unwrap_or(-1.0);
            bf.total_cmp(&af)
                .then_with(|| bp.total_cmp(&ap))
                .then_with(|| entries[b].meta_score.total_cmp(&entries[a].meta_score))
        });
        selected_groups = ranked
            .into_iter()
            .take(req.top_k)
            .map(|i| entries[i].group.clone())
            .collect();
    } else if req.fallback_to_meta_if_empty {
        let mut fallback_indices: Vec<usize> = (0..entries.len()).collect();
        fallback_indices.sort_by(|&a, &b| entries[b].meta_score.total_cmp(&entries[a].meta_score));
        selected_groups = fallback_indices
            .into_iter()
            .take(req.top_k)
            .map(|i| {
                let mut g = entries[i].group.clone();
                if let Some(obj) = g.as_object_mut() {
                    let final_rank_score = (!obj
                        .get("final_rank_score")
                        .map(|v| v.is_number())
                        .unwrap_or(false))
                    .then_some(entries[i].meta_score);
                    annotate_stage5_group_aliases(obj, None, true, final_rank_score);
                }
                g
            })
            .collect();
        let reason = format!(
            "No groups met min_ml_confidence={:.3}. Fell back to meta-score ranking.",
            req.min_ml_confidence
        );
        fallback_reason = Some(reason.clone());
        warnings.push(reason);
    }

    let mut decision_counts = Stage4DecisionCounts {
        accept: 0,
        review: 0,
        reject: 0,
    };
    for (i, g) in selected_groups.iter_mut().enumerate() {
        if let Some(obj) = g.as_object_mut() {
            let final_rank_score = (!obj
                .get("final_rank_score")
                .map(|v| v.is_number())
                .unwrap_or(false))
            .then(|| {
                obj.get("final_score")
                    .and_then(|v| v.as_f64())
                    .or_else(|| obj.get("meta_score").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0)
            });
            annotate_stage5_group_aliases(obj, Some(i + 1), false, final_rank_score);
            let ml = obj.get("ml_confidence").and_then(|v| v.as_f64());
            let unc = obj.get("ml_uncertainty").and_then(|v| v.as_f64());
            let bucket = decision_bucket_label(&req, ml, unc);
            obj.insert("decision_bucket".to_string(), serde_json::json!(bucket));
            obj.insert(
                "decision_bucket_semantics".to_string(),
                serde_json::json!("threshold_review_bucket_not_deployability_proof"),
            );
            match bucket {
                "accept" => decision_counts.accept = decision_counts.accept.saturating_add(1),
                "review" => decision_counts.review = decision_counts.review.saturating_add(1),
                _ => decision_counts.reject = decision_counts.reject.saturating_add(1),
            }
        }
    }

    if groups_scored < groups.len() {
        warnings.push(format!(
            "ML model produced confidences for {}/{} groups.",
            groups_scored,
            groups.len()
        ));
    }

    let model_evidence_tier = model
        .meta
        .as_ref()
        .map(|meta| meta.evidence_tier)
        .unwrap_or_default();
    let training_artifact_count = training_manifest.training_artifact_count.max(
        model
            .meta
            .as_ref()
            .and_then(|meta| meta.meta_artifacts_used)
            .unwrap_or(0),
    );
    let source_artifact_sha256 = training_manifest.source_artifact.artifact_sha256.clone();
    let warning_count = warnings.len();
    let ts = chrono::Utc::now().to_rfc3339();
    let result = MetaStage4Result {
        strategy_kind: scope.strategy_kind.clone(),
        timeframe: scope.timeframe.clone(),
        source_meta_json_path: source_meta_path.to_string_lossy().to_string(),
        model_path: model_path.to_string_lossy().to_string(),
        model_state,
        model_evidence_tier,
        model_healthy,
        model_trained_now,
        groups_considered: groups.len(),
        groups_scored,
        groups_selected: selected_groups.len(),
        min_ml_confidence: req.min_ml_confidence,
        top_k: req.top_k,
        meta_weight: req.meta_weight,
        ml_weight: req.ml_weight,
        uncertainty_weight: req.uncertainty_weight,
        accept_confidence: req.accept_confidence,
        review_confidence: req.review_confidence,
        max_accept_uncertainty: req.max_accept_uncertainty,
        decision_counts,
        selected_groups,
        fallback_reason,
        training_manifest,
        training_artifact_count,
        source_artifact_sha256,
        warning_count,
        evidence_policy_summary: stage5_evidence_policy_summary(),
        warnings,
    };

    let out_obj = serde_json::json!({
        "version": 1,
        "kind": "vectorgrid_meta_stage5_ml_confidence",
        "ts": ts,
        "tags": [],
        "req": req,
        "res": &result,
    });
    let out_bytes = serde_json::to_vec_pretty(&out_obj).map_err(|e| e.to_string())?;
    let _artifact_library_lock = run_artifacts::acquire_artifact_library_process_lock(&app)?;
    let out_path = suggest_stage5_artifact_path(&ts, &app)?;
    run_artifacts::invalidate_artifact_catalog_under_process_lock(&app)?;
    run_artifacts::publish_new_artifact_file_atomically(&out_path, &out_bytes)?;
    run_artifacts::prune_old_stage5_artifacts_under_process_lock(
        &app,
        run_artifacts::artifact_retention_policy_for_app(&app).stage5_keep_newest,
    );

    Ok(RunMetaStage4Response {
        ts,
        stage4_json_path: out_path.to_string_lossy().to_string(),
        stage4_json_bytes: out_bytes.len(),
        stage5_json_path: out_path.to_string_lossy().to_string(),
        stage5_json_bytes: out_bytes.len(),
        result,
    })
}

pub(super) fn run_meta_stage4_experimental_blocking(
    req: MetaStage4Request,
    app: tauri::AppHandle,
) -> Result<RunMetaStage4Response, String> {
    run_meta_stage5_ml_confidence_blocking(req, app)
}

#[tauri::command]
pub async fn run_meta_stage4_experimental(
    req: MetaStage4Request,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunMetaStage4Response, String> {
    let _run_guard = state.try_begin_run()?;
    tauri::async_runtime::spawn_blocking(move || run_meta_stage4_experimental_blocking(req, app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_meta_stage5_ml_confidence(
    req: MetaStage5Request,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunMetaStage5Response, String> {
    let _run_guard = state.try_begin_run()?;
    tauri::async_runtime::spawn_blocking(move || run_meta_stage5_ml_confidence_blocking(req, app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn evaluate_ml_confidence_policy(
    req: MlConfidencePolicyEvaluationRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunMlConfidencePolicyEvaluationResponse, String> {
    let _run_guard = state.try_begin_run()?;
    tauri::async_runtime::spawn_blocking(move || {
        run_ml_confidence_policy_evaluation_blocking(req, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize)]
pub struct RunMetaSelectionResponse {
    pub ts: String,
    pub report_xlsx_path: String,
    pub report_xlsx_bytes: usize,
    pub meta_json_path: String,
    pub meta_json_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cohort_manifest: Option<MetaCohortManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holdout_execution: Option<MetaHoldoutExecutionEvidence>,
    pub result: MetaSelectionResult,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MetaHoldoutGroupExecutionEvidence {
    pub group_key: String,
    pub datasets_requested: usize,
    pub datasets_evaluated: usize,
    pub datasets_failed: usize,
    pub passes: usize,
    pub profitable: usize,
    pub evaluation_rate: f64,
    pub pass_rate: f64,
    pub profitable_rate: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MetaHoldoutExecutionEvidence {
    pub requested_backend: String,
    pub require_gpu: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_device_id: Option<u32>,
    pub datasets_requested: usize,
    pub datasets_evaluated: usize,
    pub datasets_failed: usize,
    pub gpu_datasets: usize,
    pub cpu_datasets: usize,
    pub hybrid_datasets: usize,
    pub fallback_datasets: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub configured_auxiliary_stages: Vec<String>,
    pub pass_rate_denominator: String,
    pub profitable_rate_denominator: String,
    pub route_validation_batch_counts: BTreeMap<String, usize>,
    pub group_metrics: Vec<MetaHoldoutGroupExecutionEvidence>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
struct MetaOptimizationMetricSurfaceSummary {
    pub train: ArtifactMetricSurfaceSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holdout: Option<ArtifactMetricSurfaceSummary>,
    pub mixed_train_surfaces: bool,
    pub mixed_holdout_surfaces: bool,
}

#[tauri::command]
pub async fn run_meta_selection(
    req: MetaSelectionRequest,
    campaign_summary_path: Option<String>,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunMetaSelectionResponse, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_meta_selection_blocking(req, campaign_summary_path, app2, cancel2)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn reports_dir(app: &tauri::AppHandle<impl tauri::Runtime>) -> Result<std::path::PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("reports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn suggest_meta_report_path(
    ts: &str,
    app: &tauri::AppHandle<impl tauri::Runtime>,
    ext: &'static str,
) -> Result<std::path::PathBuf, String> {
    let dir = reports_dir(app)?;
    let safe_ts = sanitize_filename_component(ts);

    let base_name = format!("vectorgrid_meta_{safe_ts}");
    let mut path = dir.join(format!("{base_name}.{ext}"));
    if !path.exists() {
        return Ok(path);
    }
    for i in 2..=999u32 {
        let p = dir.join(format!("{base_name}_{i}.{ext}"));
        if !p.exists() {
            path = p;
            break;
        }
    }
    if path.exists() {
        return Err(format!(
            "failed to allocate a unique meta report path for timestamp '{safe_ts}'"
        ));
    }
    Ok(path)
}

fn into_owned_artifact_values(artifacts: Vec<(PathBuf, Value)>) -> Vec<Value> {
    artifacts
        .into_iter()
        .map(|(_, artifact)| artifact)
        .collect()
}

pub(super) fn run_meta_selection_blocking(
    mut req: MetaSelectionRequest,
    campaign_summary_path: Option<String>,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<RunMetaSelectionResponse, String> {
    let mut campaign_expectation =
        load_meta_campaign_cohort_expectation(campaign_summary_path.as_deref(), &req, &app)?;

    let supports_holdout = strategy_supports_holdout(req.strategy_kind.as_str());
    let holdout_tag = if req.eval_holdout && supports_holdout {
        req.holdout_tag
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    } else {
        None
    };

    let mut paths: Vec<PathBuf> = if let Some(expectation) = campaign_expectation.as_ref() {
        expectation.referenced_artifact_paths.clone()
    } else {
        let dir = run_artifacts::runs_dir(&app)?;
        let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
        let mut discovered = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if run_artifacts::is_known_non_run_artifact_file_name(name) {
                continue;
            }
            discovered.push(path);
        }
        discovered
    };
    paths.sort_unstable();

    let sink = TauriProgressSink { app: app.clone() };
    let mut train_artifacts_with_paths: Vec<(PathBuf, Value)> = Vec::with_capacity(paths.len());
    let mut holdout_artifacts_with_paths: Vec<(PathBuf, Value)> = Vec::new();
    let mut campaign_artifact_sha256_by_path: HashMap<String, String> =
        HashMap::with_capacity(paths.len());
    let mut unreadable_artifact_count = 0usize;
    let mut campaign_loaded_artifacts = if campaign_expectation.is_some() {
        Some(
            run_artifacts::load_artifact_values_checked_bulk_with_sha256_and_cancel_blocking(
                &paths,
                &app,
                Some(cancel.as_ref()),
            )
                .map_err(|error| {
                    format!(
                        "campaign-bound Meta could not load its referenced run-artifact cohort: {error}"
                    )
                })?
                .into_iter(),
        )
    } else {
        None
    };
    for (i, p) in paths.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let v = if let Some(loaded) = campaign_loaded_artifacts.as_mut() {
            let (loaded_path, value, sha256) = loaded.next().ok_or_else(|| {
                "campaign-bound Meta bulk artifact load returned too few entries".to_string()
            })?;
            if loaded_path != *p {
                return Err(
                    "campaign-bound Meta bulk artifact load changed referenced path ordering"
                        .to_string(),
                );
            }
            if let Some(expectation) = campaign_expectation.as_ref() {
                validate_campaign_artifact_raw_sha256(expectation, &loaded_path, &sha256)?;
            }
            campaign_artifact_sha256_by_path.insert(cohort_artifact_path_key(&loaded_path), sha256);
            value
        } else {
            let Some(value) = read_json_value(p) else {
                unreadable_artifact_count = unreadable_artifact_count.saturating_add(1);
                ta_app_core::progress::emit_double_ma_progress(
                    Some(&sink),
                    i + 1,
                    paths.len(),
                    "meta_read",
                );
                continue;
            };
            value
        };
        if !artifact_matches_meta_req_relaxed(&v, &req) {
            if campaign_expectation.is_some() {
                return Err(format!(
                    "campaign-bound Meta referenced artifact '{}' does not match the supplied strategy/tag scope",
                    p.to_string_lossy()
                ));
            }
            ta_app_core::progress::emit_double_ma_progress(
                Some(&sink),
                i + 1,
                paths.len(),
                "meta_read",
            );
            continue;
        }
        if let Some(expectation) = campaign_expectation.as_mut() {
            let role = campaign_cohort_role_for_artifact(expectation, p, &v);
            match role {
                Some(MetaCampaignCohortRole::Train) => {
                    expectation.lineage.matched_train_artifacts = expectation
                        .lineage
                        .matched_train_artifacts
                        .saturating_add(1);
                    train_artifacts_with_paths.push((p.clone(), v));
                }
                Some(MetaCampaignCohortRole::Holdout) => {
                    expectation.lineage.matched_holdout_artifacts = expectation
                        .lineage
                        .matched_holdout_artifacts
                        .saturating_add(1);
                    holdout_artifacts_with_paths.push((p.clone(), v));
                }
                None => {
                    expectation.lineage.excluded_out_of_scope_artifacts = expectation
                        .lineage
                        .excluded_out_of_scope_artifacts
                        .saturating_add(1);
                    return Err(format!(
                        "campaign-bound Meta rejected referenced artifact '{}' because its cohort, role, dataset path, or fingerprint lineage does not match the supplied summary",
                        p.to_string_lossy()
                    ));
                }
            }
        } else if holdout_tag
            .as_deref()
            .map(|tag| artifact_has_tag(&v, tag))
            .unwrap_or(false)
        {
            holdout_artifacts_with_paths.push((p.clone(), v));
        } else {
            train_artifacts_with_paths.push((p.clone(), v));
        }
        ta_app_core::progress::emit_double_ma_progress(
            Some(&sink),
            i + 1,
            paths.len(),
            "meta_read",
        );
    }

    let mut auto_tf_warning: Option<String> = None;
    let tf_set = req
        .timeframe
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .is_some();
    if !tf_set {
        let mut uniq: HashSet<String> = HashSet::new();
        let mut tfs: Vec<String> = Vec::new();
        let mut missing = 0usize;

        for (_, a) in &train_artifacts_with_paths {
            let raw = a
                .pointer("/dataset_meta/timeframe")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let tf = normalize_timeframe_label(raw);
            if tf.is_empty() {
                missing += 1;
                continue;
            }
            if uniq.insert(tf.clone()) {
                tfs.push(tf);
            }
        }

        if tfs.len() == 1 {
            let tf = tfs[0].clone();
            req.timeframe = Some(tf.clone());
            if missing > 0 {
                auto_tf_warning = Some(format!(
                    "Detected timeframe='{tf}'. Excluded {missing} artifact(s) missing dataset_meta.timeframe."
                ));
            }
        } else if tfs.len() > 1 {
            tfs.sort();
            let list = tfs.join(", ");
            let miss = if missing > 0 {
                format!(" + {missing} missing timeframe")
            } else {
                String::new()
            };
            return Err(format!(
                "Meta selection requires a timeframe filter (found multiple timeframes: {list}{miss})."
            ));
        } else if missing > 0 {
            return Err(format!(
                "Meta selection requires a timeframe filter ({missing} candidate artifact(s) are missing dataset_meta.timeframe)."
            ));
        }
    }

    if req
        .timeframe
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        let train_before = train_artifacts_with_paths.len();
        let holdout_before = holdout_artifacts_with_paths.len();
        train_artifacts_with_paths
            .retain(|(_, artifact)| artifact_matches_meta_req(artifact, &req));
        holdout_artifacts_with_paths
            .retain(|(_, artifact)| artifact_matches_meta_req(artifact, &req));
        if campaign_expectation.is_some()
            && (train_before != train_artifacts_with_paths.len()
                || holdout_before != holdout_artifacts_with_paths.len())
        {
            return Err(
                "campaign-bound Meta artifact timeframe/tag evidence drifted from the supplied campaign summary"
                    .to_string(),
            );
        }
    }

    if let Some(expectation) = campaign_expectation.as_ref() {
        let summary = expectation.campaign_summary.as_ref().ok_or_else(|| {
            "campaign-bound Meta is missing its validated campaign summary".to_string()
        })?;
        validate_vcet_fixed_candidate_training_evidence(
            summary,
            train_artifacts_with_paths
                .iter()
                .map(|(_, artifact)| artifact),
        )
        .map_err(|error| format!("campaign-bound Meta evidence audit failed: {error}"))?;
    }

    require_requested_vcet_holdout_artifacts(
        &req.strategy_kind,
        req.eval_holdout,
        holdout_tag.as_deref(),
        holdout_artifacts_with_paths.len(),
    )?;

    let cohort_manifest = build_meta_cohort_manifest(
        &train_artifacts_with_paths,
        &holdout_artifacts_with_paths,
        holdout_tag.as_deref(),
        campaign_expectation
            .as_ref()
            .map(|expectation| expectation.lineage.clone()),
        campaign_expectation
            .as_ref()
            .map(|_| &campaign_artifact_sha256_by_path),
    );
    require_meta_holdout_disjointness(
        req.eval_holdout,
        holdout_artifacts_with_paths.len(),
        &cohort_manifest,
    )?;
    if let Some(admitted_periods) = campaign_expectation
        .as_ref()
        .and_then(|expectation| expectation.admitted_vcet_periods.as_ref())
    {
        for (path, artifact) in train_artifacts_with_paths
            .iter_mut()
            .chain(holdout_artifacts_with_paths.iter_mut())
        {
            retain_admitted_vcet_candidate_rows(artifact, admitted_periods).map_err(|error| {
                format!(
                    "campaign-bound Meta rejected referenced artifact '{}': {error}",
                    path.to_string_lossy()
                )
            })?;
        }
    }
    let train_artifacts = into_owned_artifact_values(train_artifacts_with_paths);
    let holdout_artifacts = into_owned_artifact_values(holdout_artifacts_with_paths);

    let expected_train_datasets = campaign_expectation
        .as_ref()
        .map(|expectation| expectation.lineage.expected_train_datasets);
    let expected_holdout_datasets = campaign_expectation
        .as_ref()
        .map(|expectation| expectation.lineage.expected_holdout_datasets);
    let mut result = ta_app_core::meta_select_from_artifacts_with_dataset_universe(
        &req,
        &train_artifacts,
        expected_train_datasets,
        Some(&sink),
    );
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }
    for warning in build_metric_surface_meta_warnings(&train_artifacts, &holdout_artifacts) {
        result.warnings.push(warning);
    }
    if unreadable_artifact_count > 0 {
        result.warnings.push(format!(
            "Skipped {unreadable_artifact_count} unreadable run artifact(s) while scanning meta-selection inputs."
        ));
    }
    if let Some(w) = auto_tf_warning {
        result.warnings.push(w);
    }
    for warning in &cohort_manifest.warnings {
        result.warnings.push(warning.clone());
    }

    if req.eval_holdout && !supports_holdout {
        result.warnings.push(format!(
            "Holdout evaluation is not supported for strategy_kind='{}'. Skipping.",
            req.strategy_kind
        ));
    }

    let mut holdout_execution = None;
    if let Some(tag) = holdout_tag.as_deref() {
        let mut selected_groups: Option<Vec<usize>> = None;
        let top_n = req.holdout_eval_top_n.unwrap_or(0);
        let explore_n = req.holdout_eval_explore_n;
        if (top_n > 0 || explore_n > 0) && !result.groups.is_empty() {
            let total = result.groups.len();
            let mut rank_by = req
                .holdout_eval_rank_by
                .as_deref()
                .unwrap_or("auto")
                .trim()
                .to_ascii_lowercase();
            if rank_by.is_empty() {
                rank_by = "auto".to_string();
            }

            let mut ml_model: Option<MlConfidenceModel> = None;
            if rank_by != "meta" {
                if let Some(tf) = req
                    .timeframe
                    .as_deref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                {
                    let scope = MlConfidenceScope {
                        strategy_kind: req.strategy_kind.clone(),
                        timeframe: normalize_timeframe_label(tf),
                    };
                    if let Ok(path) = ml_confidence_model_path_read_only(&scope, &app) {
                        if path.exists() {
                            if let Ok(raw) = std::fs::read_to_string(&path) {
                                if let Ok(model) = serde_json::from_str::<MlConfidenceModel>(&raw) {
                                    let healthy =
                                        model.meta.as_ref().map(|m| m.healthy).unwrap_or(false);
                                    if healthy {
                                        ml_model = Some(model);
                                    } else {
                                        result.warnings.push(
                                            "Holdout selection: ML model exists but is unhealthy/missing health metadata; falling back to meta score.".to_string(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if rank_by != "ml" && rank_by != "meta" && rank_by != "auto" {
                result.warnings.push(format!(
                    "Holdout selection: unknown holdout_eval_rank_by='{rank_by}', using auto."
                ));
                rank_by = "auto".to_string();
            }

            let using_ml = ml_model.is_some() && (rank_by == "ml" || rank_by == "auto");
            if rank_by == "ml" && !using_ml {
                result.warnings.push(
                    "Holdout selection: requested ML ranking but no healthy model is available; falling back to meta score.".to_string(),
                );
            }

            let mut selected: Vec<usize> = Vec::new();
            if using_ml {
                let model = ml_model.as_ref().unwrap();
                let mut scored: Vec<(usize, f64)> = Vec::with_capacity(total);
                for (i, g) in result.groups.iter().enumerate() {
                    let features = meta_candidate_group_features(g);
                    let p = ta_app_core::predict_ml_confidence_for_features(model, &features)
                        .unwrap_or(-1.0);
                    scored.push((i, p));
                }
                scored.sort_by(|a, b| b.1.total_cmp(&a.1));

                let exploit_n = top_n.min(total);
                for (i, _p) in scored.iter().take(exploit_n) {
                    selected.push(*i);
                }

                if explore_n > 0 {
                    let selected_set: HashSet<usize> = selected.iter().copied().collect();
                    let mut rem: Vec<(usize, f64)> = scored
                        .iter()
                        .filter(|(i, _)| !selected_set.contains(i))
                        .map(|(i, p)| (*i, (p - 0.5).abs()))
                        .collect();
                    rem.sort_by(|a, b| a.1.total_cmp(&b.1));
                    for (i, _) in rem
                        .into_iter()
                        .take(explore_n.min(total.saturating_sub(selected.len())))
                    {
                        selected.push(i);
                    }
                }
            } else {
                if explore_n > 0 {
                    result.warnings.push(
                        "Holdout selection: explore_n is ignored unless ML ranking is available."
                            .to_string(),
                    );
                }
                let mut scored: Vec<(usize, f64)> = result
                    .groups
                    .iter()
                    .enumerate()
                    .map(|(i, g)| (i, g.meta_score))
                    .collect();
                scored.sort_by(|a, b| b.1.total_cmp(&a.1));
                let exploit_n = top_n.min(total);
                for (i, _s) in scored.iter().take(exploit_n) {
                    selected.push(*i);
                }
            }

            selected.sort();
            selected.dedup();
            if !selected.is_empty() && selected.len() < total {
                result.warnings.push(format!(
                    "Holdout evaluation: evaluating {}/{} groups (top_n={}, explore_n={}, rank_by={}).",
                    selected.len(),
                    total,
                    top_n,
                    explore_n,
                    req.holdout_eval_rank_by.as_deref().unwrap_or("auto")
                ));
                selected_groups = Some(selected);
            }
        }

        holdout_execution = apply_holdout_evaluation(
            &mut result,
            &req,
            &holdout_artifacts,
            expected_holdout_datasets,
            &app,
            cancel.as_ref(),
            &sink,
            tag,
            selected_groups.as_deref(),
        )?;
    }

    let ts = chrono::Utc::now().to_rfc3339();

    let metric_surface_summary =
        build_metric_surface_meta_summary(&train_artifacts, &holdout_artifacts);

    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 0, 1, "meta_export");
    let xlsx_bytes = build_meta_report_xlsx_bytes(
        &ts,
        &result,
        holdout_execution.as_ref(),
        Some(&cohort_manifest),
    )?;
    let _artifact_library_lock = run_artifacts::acquire_artifact_library_process_lock(&app)?;
    let report_path = suggest_meta_report_path(&ts, &app, "xlsx")?;
    run_artifacts::publish_new_artifact_file_atomically(&report_path, &xlsx_bytes)?;
    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 1, 1, "meta_export");

    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 0, 1, "meta_save");
    let meta_obj = serde_json::json!({
        "version": 1,
        "kind": "vectorgrid_meta_selection",
        "ts": ts,
        "report_xlsx_path": report_path.to_string_lossy().to_string(),
        "tags": [],
        "optimization_metric_surface_summary": metric_surface_summary,
        "cohort_manifest": &cohort_manifest,
        "holdout_execution": &holdout_execution,
        "req": req,
        "res": result,
    });
    let meta_bytes = serde_json::to_vec_pretty(&meta_obj).map_err(|e| e.to_string())?;
    let meta_path = run_artifacts::suggest_meta_artifact_path(&ts, &app)?;
    run_artifacts::invalidate_artifact_catalog_under_process_lock(&app)?;
    run_artifacts::publish_new_artifact_file_atomically(&meta_path, &meta_bytes)?;

    run_artifacts::prune_old_meta_artifacts_under_process_lock(
        &app,
        run_artifacts::artifact_retention_policy_for_app(&app).meta_keep_newest,
    );
    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 1, 1, "meta_save");

    Ok(RunMetaSelectionResponse {
        ts,
        report_xlsx_path: report_path.to_string_lossy().to_string(),
        report_xlsx_bytes: xlsx_bytes.len(),
        meta_json_path: meta_path.to_string_lossy().to_string(),
        meta_json_bytes: meta_bytes.len(),
        cohort_manifest: Some(cohort_manifest),
        holdout_execution,
        result,
    })
}

fn build_metric_surface_meta_warnings(
    train_artifacts: &[Value],
    holdout_artifacts: &[Value],
) -> Vec<String> {
    let mut warnings: Vec<String> = Vec::new();
    let train_summary = summarize_artifact_metric_surfaces(train_artifacts);
    if train_summary.total() == 0 {
        return warnings;
    }

    if train_summary.is_mixed() {
        warnings.push(format!(
            "Meta selection is mixing optimization metric surfaces across training artifacts ({}). Prefer a uniform Core17/Trade22 corpus for compare/meta/ML consistency.",
            train_summary.describe_counts()
        ));
    }
    if train_summary.missing_metadata > 0 {
        warnings.push(format!(
            "Meta selection inferred optimization metric surfaces for {} training artifact(s) with missing explicit metadata or stale schema metadata.",
            train_summary.missing_metadata
        ));
    }

    let holdout_summary = summarize_artifact_metric_surfaces(holdout_artifacts);
    if holdout_summary.total() == 0 {
        return warnings;
    }

    if holdout_summary.is_mixed() {
        warnings.push(format!(
            "Holdout artifact set mixes optimization metric surfaces ({}). Holdout diagnostics may be harder to compare cleanly across artifacts.",
            holdout_summary.describe_counts()
        ));
    }
    if holdout_summary.missing_metadata > 0 {
        warnings.push(format!(
            "Meta selection inferred optimization metric surfaces for {} holdout artifact(s) with missing explicit metadata or stale schema metadata.",
            holdout_summary.missing_metadata
        ));
    }
    if train_summary.describe_counts() != holdout_summary.describe_counts() {
        warnings.push(format!(
            "Training vs holdout metric-surface mix differs (train: {}; holdout: {}).",
            train_summary.describe_counts(),
            holdout_summary.describe_counts()
        ));
    }

    warnings
}

fn build_metric_surface_meta_summary(
    train_artifacts: &[Value],
    holdout_artifacts: &[Value],
) -> Option<MetaOptimizationMetricSurfaceSummary> {
    let train = summarize_artifact_metric_surfaces(train_artifacts);
    let holdout = summarize_artifact_metric_surfaces(holdout_artifacts);
    if train.total() == 0 && holdout.total() == 0 {
        return None;
    }
    Some(MetaOptimizationMetricSurfaceSummary {
        mixed_train_surfaces: train.is_mixed(),
        mixed_holdout_surfaces: holdout.is_mixed(),
        train,
        holdout: (holdout.total() > 0).then_some(holdout),
    })
}

fn artifact_has_tag(artifact: &Value, tag: &str) -> bool {
    let tag = tag.trim();
    if tag.is_empty() {
        return false;
    }
    let tags = artifact
        .get("tags")
        .and_then(|v| v.as_array())
        .map_or(&[][..], |xs| xs.as_slice());
    tags.iter()
        .filter_map(|v| v.as_str())
        .any(|t| t.trim().eq_ignore_ascii_case(tag))
}

fn normalize_timeframe_label(raw: &str) -> String {
    let s0 = raw.trim().to_ascii_lowercase();
    if s0.is_empty() {
        return String::new();
    }

    if let Some(sec_str) = s0.strip_suffix('s') {
        if let Ok(sec) = sec_str.parse::<i64>() {
            let mapped = match sec {
                60 => Some("1m"),
                120 => Some("2m"),
                180 => Some("3m"),
                300 => Some("5m"),
                900 => Some("15m"),
                1800 => Some("30m"),
                3600 => Some("1h"),
                7200 => Some("2h"),
                14_400 => Some("4h"),
                86_400 => Some("1d"),
                _ => None,
            };
            if let Some(lbl) = mapped {
                return lbl.to_string();
            }
        }
    }
    s0
}

fn artifact_matches_meta_req(artifact: &Value, req: &MetaSelectionRequest) -> bool {
    let kind = artifact.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    if !run_artifacts::is_run_artifact_kind(kind) {
        return false;
    }

    let strat = artifact
        .pointer("/run/req/strategy_kind")
        .and_then(|v| v.as_str())
        .or_else(|| {
            artifact
                .pointer("/run/req/strategyKind")
                .and_then(|v| v.as_str())
        })
        .unwrap_or("");
    if strat != req.strategy_kind {
        return false;
    }

    if let Some(tf) = req
        .timeframe
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let tf2 = artifact
            .pointer("/dataset_meta/timeframe")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if normalize_timeframe_label(tf2) != normalize_timeframe_label(tf) {
            return false;
        }
    }

    if let Some(tag) = req
        .required_tag
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        if !artifact_has_tag(artifact, tag) {
            return false;
        }
    }

    artifact.pointer("/validation/res/candidates").is_some()
}

fn artifact_matches_meta_req_relaxed(artifact: &Value, req: &MetaSelectionRequest) -> bool {
    let kind = artifact.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    if !run_artifacts::is_run_artifact_kind(kind) {
        return false;
    }

    let strat = artifact
        .pointer("/run/req/strategy_kind")
        .and_then(|v| v.as_str())
        .or_else(|| {
            artifact
                .pointer("/run/req/strategyKind")
                .and_then(|v| v.as_str())
        })
        .unwrap_or("");
    if strat != req.strategy_kind {
        return false;
    }

    if let Some(tag) = req
        .required_tag
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        if !artifact_has_tag(artifact, tag) {
            return false;
        }
    }

    artifact.pointer("/validation/res/candidates").is_some()
}

fn artifact_csv_path(artifact: &Value) -> Option<String> {
    let p = artifact
        .pointer("/data/csv_path")
        .and_then(|v| v.as_str())
        .or_else(|| {
            artifact
                .pointer("/dataset_meta/csv_path")
                .and_then(|v| v.as_str())
        })?;
    let s = p.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn artifact_dataset_label(artifact: &Value, csv_path: &str) -> String {
    let lbl = artifact
        .pointer("/dataset_meta/label")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(l) = lbl {
        return l;
    }
    Path::new(csv_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("holdout")
        .to_string()
}

fn share_safe_holdout_dataset_key(identity: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in identity.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("source-fnv1a64-{hash:016x}")
}

fn unique_holdout_dataset_paths(
    holdout: &[&Value],
) -> Vec<(String, String, String, Option<String>)> {
    let mut unique: Vec<(String, String, String, Option<String>)> = Vec::new();
    let mut index_by_identity: HashMap<String, usize> = HashMap::new();
    for artifact in holdout {
        let csv_path = artifact_csv_path(artifact);
        let Some(identity) = cohort_dataset_key(artifact).or_else(|| csv_path.clone()) else {
            continue;
        };
        if let Some(&existing_idx) = index_by_identity.get(&identity) {
            if unique[existing_idx].0.is_empty() {
                if let Some(replacement_path) = csv_path {
                    unique[existing_idx].0 = replacement_path.clone();
                    unique[existing_idx].1 = artifact_dataset_label(artifact, &replacement_path);
                }
            }
            if unique[existing_idx].3.is_none() {
                unique[existing_idx].3 = value_string_at(
                    artifact,
                    &[
                        "/dataset_meta/source_content_fingerprint",
                        "/data/source_content_fingerprint",
                        "/run/req/source_content_fingerprint",
                    ],
                );
            }
            continue;
        }
        let csv_path = csv_path.unwrap_or_default();
        index_by_identity.insert(identity.clone(), unique.len());
        unique.push((
            csv_path.clone(),
            artifact_dataset_label(artifact, &csv_path),
            share_safe_holdout_dataset_key(&identity),
            value_string_at(
                artifact,
                &[
                    "/dataset_meta/source_content_fingerprint",
                    "/data/source_content_fingerprint",
                    "/run/req/source_content_fingerprint",
                ],
            ),
        ));
    }
    unique
}

fn require_unchanged_holdout_source_fingerprint(
    expected: Option<&str>,
    actual: &str,
    label: &str,
) -> Result<(), String> {
    if expected
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some_and(|expected| expected != actual)
    {
        return Err(format!(
            "Meta holdout source fingerprint changed after campaign capture for dataset '{label}'; rerun the campaign before holdout validation"
        ));
    }
    Ok(())
}

#[derive(Default, Clone)]
struct HoldoutAcc {
    scores: Vec<f64>,
    sharpes: Vec<f64>,
    max_dds: Vec<f64>,
    pnls: Vec<f64>,
    passes: usize,
    profitable: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RepresentativeParamKey(Arc<str>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct HoldoutConfigKey(Arc<str>);

#[derive(Debug, Clone, Copy, PartialEq)]
struct HoldoutDatasetStats {
    score_median: f64,
    score_p10: f64,
    sharpe_median: f64,
    max_dd_worst: f64,
    pnl_median: f64,
    profitable: bool,
    pass: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HoldoutMetricSample {
    score: f64,
    sharpe: f64,
    max_dd: f64,
    pnl: f64,
}

#[derive(Clone, Debug)]
struct ResolvedMetaHoldoutExecution {
    backend: ta_app_core::Backend,
    requested_backend: &'static str,
    require_gpu: bool,
    gpu_device_id: Option<u32>,
    configured_auxiliary_stages: Vec<String>,
}

#[derive(Default)]
struct HoldoutRouteEvidenceAcc {
    gpu_datasets: usize,
    cpu_datasets: usize,
    hybrid_datasets: usize,
    fallback_datasets: usize,
    route_validation_batch_counts: BTreeMap<String, usize>,
}

#[derive(Default)]
struct HoldoutDatasetRouteState {
    saw_gpu: bool,
    saw_cpu: bool,
    saw_fallback: bool,
}

impl HoldoutDatasetRouteState {
    fn observe(
        &mut self,
        backend_used: &ta_app_core::BackendUsed,
        backend_route: Option<&str>,
        backend_fallback_reason: Option<&str>,
        routes: &mut BTreeMap<String, usize>,
    ) {
        match backend_used {
            ta_app_core::BackendUsed::Gpu => self.saw_gpu = true,
            ta_app_core::BackendUsed::Cpu => self.saw_cpu = true,
        }
        let route = backend_route.unwrap_or("unknown");
        if route.contains("->CpuOnly") || backend_fallback_reason.is_some() {
            self.saw_fallback = true;
        }
        *routes.entry(route.to_string()).or_default() += 1;
    }
}

impl HoldoutRouteEvidenceAcc {
    fn record_evaluated_dataset(
        &mut self,
        dataset: HoldoutDatasetRouteState,
        route_counts: BTreeMap<String, usize>,
    ) {
        if dataset.saw_gpu && dataset.saw_cpu {
            self.hybrid_datasets = self.hybrid_datasets.saturating_add(1);
        } else if dataset.saw_gpu {
            self.gpu_datasets = self.gpu_datasets.saturating_add(1);
        } else if dataset.saw_cpu {
            self.cpu_datasets = self.cpu_datasets.saturating_add(1);
        }
        if dataset.saw_fallback {
            self.fallback_datasets = self.fallback_datasets.saturating_add(1);
        }
        for (route, count) in route_counts {
            *self.route_validation_batch_counts.entry(route).or_default() += count;
        }
    }
}

fn resolve_meta_holdout_execution(
    req: &MetaSelectionRequest,
    validation_engine: ta_app_core::ValidationEngine,
) -> Result<ResolvedMetaHoldoutExecution, String> {
    let (use_gpu, require_gpu, gpu_device_id) = req
        .holdout_validation
        .as_ref()
        .map(|cfg| (cfg.use_gpu, cfg.require_gpu, cfg.gpu_device_id))
        .unwrap_or((false, false, 0));
    let gpu_requested = use_gpu || require_gpu;
    if gpu_requested && req.strategy_kind != "Vcet" {
        return Err(format!(
            "Meta holdout GPU execution currently supports strategy_kind='Vcet' only (received '{}').",
            req.strategy_kind
        ));
    }
    if gpu_requested && validation_engine != ta_app_core::ValidationEngine::Vectorized {
        return Err(
            "Meta VCET GPU holdout execution requires validation_engine='vectorized'.".to_string(),
        );
    }
    let mut configured_auxiliary_stages = Vec::new();
    if matches!(req.holdout_mode, ta_app_core::MetaHoldoutMode::Heavy) {
        if let Some(cfg) = req.holdout_validation.as_ref() {
            if cfg.walk_forward.is_some() {
                configured_auxiliary_stages.push("walk_forward".to_string());
            }
            if cfg.perturb.is_some() {
                configured_auxiliary_stages.push("perturb".to_string());
            }
            if cfg.trade_bootstrap.is_some() {
                configured_auxiliary_stages.push("trade_bootstrap".to_string());
            }
            if cfg.monte_carlo.is_some() {
                configured_auxiliary_stages.push("monte_carlo".to_string());
            }
        }
    }
    if require_gpu && !configured_auxiliary_stages.is_empty() {
        return Err(format!(
            "Strict Meta VCET GPU holdout cannot prove end-to-end GPU residency for configured stage(s): {}. Remove those stages or set require_gpu=false to permit hybrid execution.",
            configured_auxiliary_stages.join(", ")
        ));
    }

    Ok(if gpu_requested {
        ResolvedMetaHoldoutExecution {
            backend: ta_app_core::Backend::GpuKernel {
                device_id: gpu_device_id,
            },
            requested_backend: "GpuKernel",
            require_gpu,
            gpu_device_id: Some(gpu_device_id),
            configured_auxiliary_stages,
        }
    } else {
        ResolvedMetaHoldoutExecution {
            backend: ta_app_core::Backend::CpuOnly,
            requested_backend: "CpuOnly",
            require_gpu: false,
            gpu_device_id: None,
            configured_auxiliary_stages,
        }
    })
}

fn require_strict_meta_vcet_gpu_result(
    require_gpu: bool,
    requested_backend: &str,
    backend_used: &ta_app_core::BackendUsed,
    backend_route: Option<&str>,
    backend_fallback_reason: Option<&str>,
    validation_residency_tier: &str,
) -> Result<(), String> {
    if !require_gpu {
        return Ok(());
    }
    let valid = requested_backend == "GpuKernel"
        && matches!(backend_used, ta_app_core::BackendUsed::Gpu)
        && backend_route == Some("GpuKernel")
        && backend_fallback_reason.is_none()
        && validation_residency_tier == "gpu_kernel";
    if valid {
        return Ok(());
    }

    Err(format!(
        "Strict Meta VCET GPU holdout execution failed: requested_backend={requested_backend}, backend_used={backend_used:?}, backend_route={}, validation_residency_tier={validation_residency_tier}, fallback_reason={}",
        backend_route.unwrap_or("none"),
        backend_fallback_reason.unwrap_or("none")
    ))
}

fn describe_holdout_coverage_selection_policy(
    req: &MetaSelectionRequest,
    active_groups: usize,
    total_groups: usize,
) -> String {
    if active_groups >= total_groups {
        return "all_groups".to_string();
    }
    format!(
        "partial(top_n={}, explore_n={}, rank_by={})",
        req.holdout_eval_top_n.unwrap_or(0),
        req.holdout_eval_explore_n,
        req.holdout_eval_rank_by.as_deref().unwrap_or("auto")
    )
}

fn set_meta_holdout_coverage_for_zero_usable_datasets(
    result: &mut MetaSelectionResult,
    req: &MetaSelectionRequest,
    active_indices: &[usize],
    skipped: usize,
) {
    let total_groups = result.groups.len();
    if total_groups == 0 {
        return;
    }
    let active: HashSet<usize> = active_indices.iter().copied().collect();
    for (idx, group) in result.groups.iter_mut().enumerate() {
        let missing_reason = if active.contains(&idx) {
            "no_usable_holdout_datasets"
        } else {
            "not_selected_for_partial_holdout"
        };
        group.holdout_coverage = Some(ta_app_core::MetaGroupHoldoutCoverage {
            evaluated: false,
            dataset_rows: 0,
            missing_reason: Some(missing_reason.to_string()),
        });
    }
    result.holdout_coverage = Some(ta_app_core::MetaHoldoutCoverageSummary {
        groups_total: total_groups,
        groups_evaluated: 0,
        groups_unevaluated: total_groups,
        datasets_total: 0,
        datasets_skipped: skipped,
        coverage_mode: "none".to_string(),
        selection_policy: describe_holdout_coverage_selection_policy(
            req,
            active_indices.len(),
            total_groups,
        ),
    });
}

#[derive(Default)]
struct HoldoutSummaryScratch {
    scores: Vec<f64>,
    sharpes: Vec<f64>,
    max_dds: Vec<f64>,
    pnls: Vec<f64>,
}

impl HoldoutSummaryScratch {
    fn summarize_rows(
        &mut self,
        strategy_kind: &str,
        rows: &[HoldoutMetricSample],
    ) -> Option<HoldoutDatasetStats> {
        if rows.is_empty() {
            return None;
        }

        self.scores.clear();
        self.sharpes.clear();
        self.max_dds.clear();
        self.pnls.clear();
        self.scores.reserve(rows.len());
        self.sharpes.reserve(rows.len());
        self.max_dds.reserve(rows.len());
        self.pnls.reserve(rows.len());

        for row in rows {
            self.scores.push(row.score);
            self.sharpes.push(row.sharpe);
            self.max_dds.push(row.max_dd);
            self.pnls.push(row.pnl);
        }

        summarize_holdout_metric_columns(
            strategy_kind,
            self.scores.as_mut_slice(),
            self.sharpes.as_mut_slice(),
            self.max_dds.as_slice(),
            self.pnls.as_mut_slice(),
        )
    }
}

#[derive(Clone)]
struct PreparedDoubleMaHoldoutBatch {
    request: ta_app_core::DoubleMaValidationRequest,
    candidate_targets: Vec<Vec<usize>>,
    candidate_idx_by_key: HashMap<RepresentativeParamKey, usize>,
}

#[derive(Clone)]
struct PreparedCompositeHoldoutBatch {
    request: ta_app_core::CompositeValidationRequest,
    candidate_targets: Vec<Vec<usize>>,
    candidate_idx_by_key: HashMap<RepresentativeParamKey, usize>,
}

#[derive(Clone)]
enum PreparedHoldoutBatch {
    DoubleMa(PreparedDoubleMaHoldoutBatch),
    Composite(PreparedCompositeHoldoutBatch),
}

fn prepared_candidate_targets_for_params<'a, T: Serialize>(
    params: &T,
    candidate_idx_by_key: &HashMap<RepresentativeParamKey, usize>,
    candidate_targets: &'a [Vec<usize>],
) -> Result<(RepresentativeParamKey, &'a [usize]), String> {
    let key = canonical_typed_param_key(params)?;
    let candidate_idx = candidate_idx_by_key.get(&key).copied().ok_or_else(|| {
        "holdout validation returned candidate parameters absent from the prepared request"
            .to_string()
    })?;
    let targets = candidate_targets.get(candidate_idx).ok_or_else(|| {
        "holdout validation candidate target index drifted from the prepared request".to_string()
    })?;
    Ok((key, targets.as_slice()))
}

fn quantile_index(len: usize, q: f64) -> usize {
    if len <= 1 {
        return 0;
    }
    let qq = q.clamp(0.0, 1.0);
    (qq * (len.saturating_sub(1) as f64)).round() as usize
}

fn quantile_in_place(values: &mut [f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    if values.len() == 1 {
        return values[0];
    }
    let idx = quantile_index(values.len(), q).min(values.len() - 1);
    let (_, nth, _) = values.select_nth_unstable_by(idx, |a, b| a.total_cmp(b));
    *nth
}

fn max_total_cmp(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .max_by(|a, b| a.total_cmp(b))
        .unwrap_or(0.0)
}

fn summarize_holdout_metric_columns(
    strategy_kind: &str,
    scores: &mut [f64],
    sharpes: &mut [f64],
    max_dds: &[f64],
    pnls: &mut [f64],
) -> Option<HoldoutDatasetStats> {
    if scores.is_empty() || sharpes.is_empty() || max_dds.is_empty() || pnls.is_empty() {
        return None;
    }

    let score_median = quantile_in_place(scores, 0.50);
    let score_p10 = quantile_in_place(scores, 0.10);
    let sharpe_median = quantile_in_place(sharpes, 0.50);
    let max_dd_worst = max_total_cmp(max_dds);
    let pnl_median = quantile_in_place(pnls, 0.50);
    let profitable = pnl_median > 0.0;
    let pass = score_p10 > 0.0 && sharpe_median > 0.0 && (strategy_kind != "Vcet" || profitable);

    Some(HoldoutDatasetStats {
        score_median,
        score_p10,
        sharpe_median,
        max_dd_worst,
        pnl_median,
        profitable,
        pass,
    })
}

fn append_canonical_json_key(buf: &mut String, value: &Value) -> Result<(), String> {
    match value {
        Value::Null => buf.push_str("null"),
        Value::Bool(v) => {
            if *v {
                buf.push_str("true");
            } else {
                buf.push_str("false");
            }
        }
        Value::Number(v) => buf.push_str(&v.to_string()),
        Value::String(v) => buf.push_str(&serde_json::to_string(v).map_err(|e| e.to_string())?),
        Value::Array(values) => {
            buf.push('[');
            for (idx, item) in values.iter().enumerate() {
                if idx > 0 {
                    buf.push(',');
                }
                append_canonical_json_key(buf, item)?;
            }
            buf.push(']');
        }
        Value::Object(map) => {
            buf.push('{');
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_unstable_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
            for (idx, (key, item)) in entries.into_iter().enumerate() {
                if idx > 0 {
                    buf.push(',');
                }
                buf.push_str(&serde_json::to_string(key).map_err(|e| e.to_string())?);
                buf.push(':');
                append_canonical_json_key(buf, item)?;
            }
            buf.push('}');
        }
    }
    Ok(())
}

fn canonical_json_key(value: &Value) -> Result<String, String> {
    let mut out = String::new();
    append_canonical_json_key(&mut out, value)?;
    Ok(out)
}

fn canonical_typed_param_key<T: Serialize>(params: &T) -> Result<RepresentativeParamKey, String> {
    Ok(RepresentativeParamKey(Arc::from(canonical_json_key(
        &serde_json::to_value(params).map_err(|e| e.to_string())?,
    )?)))
}

fn holdout_cfg_key(
    ma_source: &str,
    fast_ma_params: &Option<HashMap<String, f64>>,
    slow_ma_params: &Option<HashMap<String, f64>>,
) -> Result<HoldoutConfigKey, String> {
    Ok(HoldoutConfigKey(Arc::from(canonical_json_key(
        &serde_json::json!({
            "ma_source": ma_source,
            "fast_ma_params": fast_ma_params,
            "slow_ma_params": slow_ma_params,
        }),
    )?)))
}

fn composite_holdout_effective_cfg_key(
    params: &ta_app_core::CompositeParams,
) -> Result<HoldoutConfigKey, String> {
    Ok(HoldoutConfigKey(Arc::from(canonical_json_key(
        &serde_json::json!({
            "strategy": &params.strategy,
            "vcet": &params.vcet,
        }),
    )?)))
}

fn is_vcet_only_holdout_candidate(params: &ta_app_core::CompositeParams) -> bool {
    params.vcet.is_some()
        && params.asr.is_none()
        && params.vcc.is_none()
        && params.vemr.is_none()
        && params.rocb.is_none()
        && params.mkcs.is_none()
        && params.kqsr.is_none()
        && params.carm.is_none()
        && params.kvdf.is_none()
        && params.cdmc.is_none()
}

fn decode_double_ma_validation_params(
    value: &Value,
) -> Result<ta_app_core::DoubleMaValidationParams, String> {
    match serde_json::from_value::<ta_app_core::DoubleMaValidationParams>(value.clone()) {
        Ok(v) => Ok(v),
        Err(_) => {
            let object = value
                .as_object()
                .ok_or_else(|| "double-ma representative is not an object".to_string())?;
            let as_u16 = |key: &str, default: u16| -> u16 {
                object
                    .get(key)
                    .and_then(|v| v.as_u64())
                    .map(|v| v.min(u16::MAX as u64) as u16)
                    .unwrap_or(default)
            };
            Ok(ta_app_core::DoubleMaValidationParams {
                fast_ma_type: object
                    .get("fast_ma_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("sma")
                    .to_string(),
                slow_ma_type: object
                    .get("slow_ma_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("sma")
                    .to_string(),
                fast_len: as_u16("fast_len", 10),
                slow_len: as_u16("slow_len", 30),
                fast_ma_params: object
                    .get("fast_ma_params")
                    .and_then(|v| serde_json::from_value(v.clone()).ok()),
                slow_ma_params: object
                    .get("slow_ma_params")
                    .and_then(|v| serde_json::from_value(v.clone()).ok()),
                strategy: object
                    .get("strategy")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default(),
                halftrend_filter: object
                    .get("halftrend_filter")
                    .or_else(|| object.get("halftrend"))
                    .and_then(|v| serde_json::from_value(v.clone()).ok()),
            })
        }
    }
}

fn build_prepared_holdout_batches(
    result: &MetaSelectionResult,
    req: &MetaSelectionRequest,
    active_indices: &[usize],
    dense_index_by_global: &HashMap<usize, usize>,
    holdout_backend: ta_app_core::Backend,
    hv_seed: u64,
    hv_wf: Option<ta_app_core::WalkForwardConfigOptional>,
    hv_perturb: Option<ta_app_core::PerturbConfig>,
    hv_mc: Option<ta_app_core::TradeBootstrapConfig>,
    hv_monte_carlo: Option<ta_app_core::MonteCarloValidationConfig>,
    hv_engine: ta_app_core::ValidationEngine,
    hv_iterative: ta_app_core::IterativeSimConfig,
) -> Result<Vec<PreparedHoldoutBatch>, String> {
    let mut groups_by_cfg: HashMap<HoldoutConfigKey, Vec<usize>> =
        HashMap::with_capacity(active_indices.len());
    for global_idx in active_indices.iter().copied() {
        let group = &result.groups[global_idx];
        let cfg_key = holdout_cfg_key(
            &group.ma_source,
            &group.fast_ma_params,
            &group.slow_ma_params,
        )?;
        groups_by_cfg.entry(cfg_key).or_default().push(global_idx);
    }
    if groups_by_cfg.is_empty() {
        return Ok(Vec::new());
    }

    let mut batches = Vec::with_capacity(groups_by_cfg.len());
    let uses_double_ma = strategy_uses_double_ma_validation(req.strategy_kind.as_str());
    for idxs in groups_by_cfg.into_values() {
        let Some(first) = idxs.first() else {
            continue;
        };
        let g0 = &result.groups[*first];
        let expected_candidates = idxs
            .iter()
            .map(|&global_idx| result.groups[global_idx].representatives.len().max(1))
            .sum();

        if uses_double_ma {
            let mut candidates: Vec<ta_app_core::DoubleMaValidationParams> =
                Vec::with_capacity(expected_candidates);
            let mut candidate_targets: Vec<Vec<usize>> = Vec::with_capacity(expected_candidates);
            let mut candidate_idx_by_key: HashMap<RepresentativeParamKey, usize> =
                HashMap::with_capacity(expected_candidates);
            for &global_idx in &idxs {
                let group = &result.groups[global_idx];
                let dense_idx = dense_index_by_global
                    .get(&global_idx)
                    .copied()
                    .ok_or_else(|| "missing dense holdout index for group".to_string())?;
                let reps = if !group.representatives.is_empty() {
                    group.representatives.as_slice()
                } else {
                    std::slice::from_ref(&group.params)
                };
                for rep in reps {
                    let params = decode_double_ma_validation_params(rep)?;
                    let key = canonical_typed_param_key(&params)?;
                    if let Some(&candidate_idx) = candidate_idx_by_key.get(&key) {
                        candidate_targets[candidate_idx].push(dense_idx);
                    } else {
                        let candidate_idx = candidates.len();
                        candidate_idx_by_key.insert(key, candidate_idx);
                        candidates.push(params);
                        candidate_targets.push(vec![dense_idx]);
                    }
                }
            }

            if candidates.is_empty() {
                continue;
            }
            let max_candidates = candidates.len();
            batches.push(PreparedHoldoutBatch::DoubleMa(
                PreparedDoubleMaHoldoutBatch {
                    request: ta_app_core::DoubleMaValidationRequest {
                        data_id: String::new(),
                        backend: holdout_backend.clone(),
                        ma_source: g0.ma_source.clone(),
                        dynamic_strategy_ir: None,
                        fast_ma_params: g0.fast_ma_params.clone(),
                        slow_ma_params: g0.slow_ma_params.clone(),
                        candidates,
                        walk_forward: hv_wf.clone(),
                        reopt_walk_forward: None,
                        spa: None,
                        perturb: hv_perturb.clone(),
                        trade_bootstrap: hv_mc.clone(),
                        monte_carlo: hv_monte_carlo.clone(),
                        cross_ticker: None,
                        cscv_pbo: None,
                        seed: hv_seed,
                        max_candidates,
                        validation_engine: hv_engine,
                        iterative: hv_iterative.clone(),
                        min_psr: None,
                        min_dsr: None,
                        min_cscv_oos_rank_frac: None,
                        min_cscv_oos_metric: None,
                        max_pbo: None,
                        validation_context: None,
                    },
                    candidate_targets,
                    candidate_idx_by_key,
                },
            ));
        } else {
            type CompositeBatchParts = (
                Vec<ta_app_core::CompositeParams>,
                Vec<Vec<usize>>,
                HashMap<RepresentativeParamKey, usize>,
            );
            let mut batches_by_effective_cfg: HashMap<HoldoutConfigKey, CompositeBatchParts> =
                HashMap::with_capacity(expected_candidates);
            let gpu_requested = matches!(
                &holdout_backend,
                ta_app_core::Backend::GpuKernel { .. }
                    | ta_app_core::Backend::GpuHybridKernel { .. }
                    | ta_app_core::Backend::GpuOnly { .. }
                    | ta_app_core::Backend::Auto { .. }
            );
            for &global_idx in &idxs {
                let group = &result.groups[global_idx];
                let dense_idx = dense_index_by_global
                    .get(&global_idx)
                    .copied()
                    .ok_or_else(|| "missing dense holdout index for group".to_string())?;
                let reps = if !group.representatives.is_empty() {
                    group.representatives.as_slice()
                } else {
                    std::slice::from_ref(&group.params)
                };
                for rep in reps {
                    let params: ta_app_core::CompositeParams =
                        serde_json::from_value(rep.clone()).map_err(|e| e.to_string())?;
                    if req.strategy_kind == "Vcet" && params.vcet.is_none() {
                        return Err(
                            "Meta VCET holdout representative is missing VCET parameters."
                                .to_string(),
                        );
                    }
                    if gpu_requested && !is_vcet_only_holdout_candidate(&params) {
                        return Err(
                            "Meta VCET GPU holdout execution requires every representative to be VCET-only."
                                .to_string(),
                        );
                    }
                    let cfg_key = if req.strategy_kind == "Vcet" && gpu_requested {
                        composite_holdout_effective_cfg_key(&params)?
                    } else {
                        HoldoutConfigKey(Arc::from("shared_composite_batch"))
                    };
                    let (candidates, candidate_targets, candidate_idx_by_key) =
                        batches_by_effective_cfg
                            .entry(cfg_key)
                            .or_insert_with(|| (Vec::new(), Vec::new(), HashMap::new()));
                    let key = canonical_typed_param_key(&params)?;
                    if let Some(&candidate_idx) = candidate_idx_by_key.get(&key) {
                        candidate_targets[candidate_idx].push(dense_idx);
                    } else {
                        let candidate_idx = candidates.len();
                        candidate_idx_by_key.insert(key, candidate_idx);
                        candidates.push(params);
                        candidate_targets.push(vec![dense_idx]);
                    }
                }
            }

            for (candidates, candidate_targets, candidate_idx_by_key) in
                batches_by_effective_cfg.into_values()
            {
                if candidates.is_empty() {
                    continue;
                }
                let max_candidates = candidates.len();
                batches.push(PreparedHoldoutBatch::Composite(
                    PreparedCompositeHoldoutBatch {
                        request: ta_app_core::CompositeValidationRequest {
                            data_id: String::new(),
                            backend: holdout_backend.clone(),
                            ma_source: g0.ma_source.clone(),
                            fast_ma_params: g0.fast_ma_params.clone(),
                            slow_ma_params: g0.slow_ma_params.clone(),
                            candidates,
                            walk_forward: hv_wf.clone(),
                            reopt_walk_forward: None,
                            spa: None,
                            perturb: hv_perturb.clone(),
                            trade_bootstrap: hv_mc.clone(),
                            monte_carlo: hv_monte_carlo.clone(),
                            cross_ticker: None,
                            cscv_pbo: None,
                            seed: hv_seed,
                            max_candidates,
                            validation_engine: hv_engine,
                            iterative: hv_iterative.clone(),
                            min_psr: None,
                            min_dsr: None,
                            min_cscv_oos_rank_frac: None,
                            min_cscv_oos_metric: None,
                            max_pbo: None,
                            validation_context: None,
                        },
                        candidate_targets,
                        candidate_idx_by_key,
                    },
                ));
            }
        }
    }

    Ok(batches)
}

fn apply_holdout_accumulators_to_result(
    result: &mut MetaSelectionResult,
    req: &MetaSelectionRequest,
    active_indices: &[usize],
    accs: &mut [HoldoutAcc],
    holdout_rows: &mut Vec<Vec<ta_app_core::HoldoutDatasetRow>>,
    datasets_requested: usize,
    datasets_evaluated: usize,
    skipped: usize,
    holdout_tag: &str,
) {
    result.holdout = Some(ta_app_core::HoldoutSummary {
        datasets: datasets_evaluated,
        skipped,
    });

    for (dense_idx, global_idx) in active_indices.iter().copied().enumerate() {
        let g = &mut result.groups[global_idx];
        let acc = &mut accs[dense_idx];
        if acc.scores.is_empty() {
            continue;
        }

        let group_datasets_evaluated = acc.scores.len();
        let pass_rate = if datasets_requested == 0 {
            0.0
        } else {
            (acc.passes as f64) / (datasets_requested as f64)
        };
        let score_median = quantile_in_place(&mut acc.scores, 0.50);
        let score_p10 = quantile_in_place(&mut acc.scores, 0.10);
        let sharpe_median = quantile_in_place(&mut acc.sharpes, 0.50);
        let max_dd_p90 = quantile_in_place(&mut acc.max_dds, 0.90);
        let pnl_median = quantile_in_place(&mut acc.pnls, 0.50);

        g.holdout = Some(ta_app_core::HoldoutAggregate {
            datasets: group_datasets_evaluated,
            pass_rate,
            score_median,
            score_p10,
            sharpe_median,
            max_dd_p90,
            pnl_median,
        });
        ta_app_core::refresh_meta_group_evidence_flags(g);

        if req.save_holdout_rows && !holdout_rows.is_empty() {
            g.holdout_rows = std::mem::take(&mut holdout_rows[dense_idx]);
        }
    }

    let total_groups = result.groups.len();
    let dense_by_global: HashMap<usize, usize> = active_indices
        .iter()
        .copied()
        .enumerate()
        .map(|(dense_idx, global_idx)| (global_idx, dense_idx))
        .collect();
    let mut groups_evaluated = 0usize;
    for (global_idx, g) in result.groups.iter_mut().enumerate() {
        if let Some(dense_idx) = dense_by_global.get(&global_idx).copied() {
            let evaluated = g.holdout.is_some();
            if evaluated {
                groups_evaluated = groups_evaluated.saturating_add(1);
            }
            let dataset_rows = if !g.holdout_rows.is_empty() {
                g.holdout_rows.len()
            } else if evaluated {
                accs.get(dense_idx).map(|acc| acc.scores.len()).unwrap_or(0)
            } else {
                0
            };
            g.holdout_coverage = Some(ta_app_core::MetaGroupHoldoutCoverage {
                evaluated,
                dataset_rows,
                missing_reason: (!evaluated).then(|| "no_usable_holdout_rows".to_string()),
            });
        } else {
            g.holdout_coverage = Some(ta_app_core::MetaGroupHoldoutCoverage {
                evaluated: false,
                dataset_rows: 0,
                missing_reason: Some("not_selected_for_partial_holdout".to_string()),
            });
        }
    }
    let groups_unevaluated = total_groups.saturating_sub(groups_evaluated);
    let coverage_mode = if groups_evaluated == 0 {
        "none"
    } else if groups_evaluated == total_groups {
        "complete"
    } else {
        "partial"
    };
    result.holdout_coverage = Some(ta_app_core::MetaHoldoutCoverageSummary {
        groups_total: total_groups,
        groups_evaluated,
        groups_unevaluated,
        datasets_total: datasets_evaluated,
        datasets_skipped: skipped,
        coverage_mode: coverage_mode.to_string(),
        selection_policy: describe_holdout_coverage_selection_policy(
            req,
            active_indices.len(),
            total_groups,
        ),
    });

    for g in &mut result.groups {
        let base = g.meta_score;
        let final_score = if let Some(h) = g.holdout.as_ref() {
            base + req.weights.w_holdout_pass_rate * h.pass_rate
                + req.weights.w_holdout_p10 * h.score_p10
                - req.weights.w_holdout_dd_p90 * h.max_dd_p90
        } else {
            base
        };
        g.final_score = Some(final_score);
    }

    result.groups.sort_by(|a, b| {
        let a_eval = a.holdout.is_some();
        let b_eval = b.holdout.is_some();
        if a_eval != b_eval {
            return b_eval.cmp(&a_eval);
        }
        let ascore = a.final_score.unwrap_or(a.meta_score);
        let bscore = b.final_score.unwrap_or(b.meta_score);
        bscore
            .total_cmp(&ascore)
            .then_with(|| b.meta_score.total_cmp(&a.meta_score))
    });

    if skipped > 0 {
        result.warnings.push(format!(
            "Holdout evaluation: failed {skipped} of {datasets_requested} requested unique dataset(s) (missing/invalid CSV path, parse error, or validation failure) for tag='{holdout_tag}'."
        ));
    }
    result.warnings.push(
        "Holdout pass rates use requested unique holdout datasets as the denominator; failed datasets count as non-passes. Holdout quantiles use evaluated datasets only."
            .to_string(),
    );
    result.warnings.push(
        "Final group ordering uses holdout-adjusted score when holdout is available.".to_string(),
    );
    let groups_with_evidence_flags = result
        .groups
        .iter()
        .filter(|group| !group.evidence_flags.is_empty())
        .count();
    if groups_with_evidence_flags > 0 {
        result.warnings.push(format!(
            "Holdout/meta evidence warning: {groups_with_evidence_flags} group(s) still show thin or noisy evidence after holdout evaluation."
        ));
    }
}

fn build_holdout_group_execution_evidence(
    result: &MetaSelectionResult,
    active_indices: &[usize],
    accs: &[HoldoutAcc],
    datasets_requested: usize,
) -> Vec<MetaHoldoutGroupExecutionEvidence> {
    let denominator = datasets_requested as f64;
    active_indices
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(dense_idx, global_idx)| {
            let group = result.groups.get(global_idx)?;
            let acc = accs.get(dense_idx)?;
            let datasets_evaluated = acc.scores.len();
            Some(MetaHoldoutGroupExecutionEvidence {
                group_key: group.key.clone(),
                datasets_requested,
                datasets_evaluated,
                datasets_failed: datasets_requested.saturating_sub(datasets_evaluated),
                passes: acc.passes,
                profitable: acc.profitable,
                evaluation_rate: if datasets_requested == 0 {
                    0.0
                } else {
                    datasets_evaluated as f64 / denominator
                },
                pass_rate: if datasets_requested == 0 {
                    0.0
                } else {
                    acc.passes as f64 / denominator
                },
                profitable_rate: if datasets_requested == 0 {
                    0.0
                } else {
                    acc.profitable as f64 / denominator
                },
            })
        })
        .collect()
}

fn apply_holdout_evaluation(
    result: &mut MetaSelectionResult,
    req: &MetaSelectionRequest,
    holdout_artifacts: &[Value],
    expected_dataset_universe: Option<usize>,
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
    sink: &TauriProgressSink,
    holdout_tag: &str,
    selected_groups: Option<&[usize]>,
) -> Result<Option<MetaHoldoutExecutionEvidence>, String> {
    if result.groups.is_empty() {
        return Ok(None);
    }

    let active_indices: Vec<usize> = if let Some(indices) = selected_groups {
        indices
            .iter()
            .copied()
            .filter(|idx| *idx < result.groups.len())
            .collect()
    } else {
        (0..result.groups.len()).collect()
    };
    if active_indices.is_empty() {
        return Ok(None);
    }
    let mut dense_index_by_global: HashMap<usize, usize> =
        HashMap::with_capacity(active_indices.len());
    for (dense_idx, global_idx) in active_indices.iter().copied().enumerate() {
        dense_index_by_global.insert(global_idx, dense_idx);
    }

    let heavy = matches!(req.holdout_mode, ta_app_core::MetaHoldoutMode::Heavy);
    let (hv_seed, hv_wf, hv_perturb, hv_mc, hv_monte_carlo, hv_engine, hv_iterative) = if heavy {
        if let Some(cfg) = req.holdout_validation.clone() {
            (
                cfg.seed,
                cfg.walk_forward,
                cfg.perturb,
                cfg.trade_bootstrap,
                cfg.monte_carlo,
                cfg.validation_engine,
                cfg.iterative,
            )
        } else {
            (
                1u64,
                None,
                None,
                None,
                None,
                ta_app_core::ValidationEngine::Vectorized,
                ta_app_core::IterativeSimConfig::default(),
            )
        }
    } else {
        (
            1u64,
            None,
            None,
            None,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
        )
    };
    let holdout_execution = resolve_meta_holdout_execution(req, hv_engine)?;

    let holdout: Vec<&Value> = holdout_artifacts
        .iter()
        .filter(|a| artifact_matches_meta_req(a, req))
        .collect();
    if holdout.is_empty() {
        return Ok(None);
    }

    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let cache_dir = crate::data_cache::app_data_cache_dir(&base);

    let uniq_paths = unique_holdout_dataset_paths(&holdout);
    if uniq_paths.is_empty() {
        return Ok(None);
    }

    let prepared_batches = build_prepared_holdout_batches(
        result,
        req,
        &active_indices,
        &dense_index_by_global,
        holdout_execution.backend.clone(),
        hv_seed,
        hv_wf.clone(),
        hv_perturb.clone(),
        hv_mc.clone(),
        hv_monte_carlo.clone(),
        hv_engine,
        hv_iterative.clone(),
    )?;
    if prepared_batches.is_empty() {
        return Ok(None);
    }

    let observed_total = uniq_paths.len();
    let total = requested_dataset_denominator(observed_total, expected_dataset_universe);
    let missing_expected_datasets = total.saturating_sub(observed_total);
    let mut accs: Vec<HoldoutAcc> = vec![HoldoutAcc::default(); active_indices.len()];
    for acc in &mut accs {
        acc.scores.reserve(total);
        acc.sharpes.reserve(total);
        acc.max_dds.reserve(total);
        acc.pnls.reserve(total);
    }
    let mut holdout_rows: Vec<Vec<ta_app_core::HoldoutDatasetRow>> = if req.save_holdout_rows {
        vec![Vec::new(); active_indices.len()]
    } else {
        Vec::new()
    };
    if req.save_holdout_rows {
        for rows in &mut holdout_rows {
            rows.reserve(total);
        }
    }
    let mut local: Vec<Vec<HoldoutMetricSample>> = vec![Vec::new(); active_indices.len()];
    let mut summary_scratch = HoldoutSummaryScratch::default();
    let mut skipped = missing_expected_datasets;
    let mut route_evidence = HoldoutRouteEvidenceAcc::default();

    for (i, (csv_path, label, dataset_key, expected_source_fingerprint)) in
        uniq_paths.into_iter().enumerate()
    {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        if csv_path.trim().is_empty() {
            skipped = skipped.saturating_add(1);
            ta_app_core::progress::emit_double_ma_progress(
                Some(sink),
                i + 1,
                total,
                "meta_holdout",
            );
            continue;
        }

        let candles = match crate::data_cache::load_or_build(Path::new(&csv_path), &cache_dir) {
            Ok(outcome) => {
                require_unchanged_holdout_source_fingerprint(
                    expected_source_fingerprint.as_deref(),
                    &outcome.source_content_fingerprint,
                    &label,
                )?;
                outcome.candles
            }
            Err(_) => {
                skipped = skipped.saturating_add(1);
                ta_app_core::progress::emit_double_ma_progress(
                    Some(sink),
                    i + 1,
                    total,
                    "meta_holdout",
                );
                continue;
            }
        };

        for rows in &mut local {
            rows.clear();
        }
        let mut ok = true;
        let mut dataset_route_state = HoldoutDatasetRouteState::default();
        if holdout_execution.requested_backend == "GpuKernel"
            && !holdout_execution.configured_auxiliary_stages.is_empty()
        {
            dataset_route_state.saw_cpu = true;
        }
        let mut dataset_route_counts: BTreeMap<String, usize> = BTreeMap::new();

        for batch in &prepared_batches {
            match batch {
                PreparedHoldoutBatch::DoubleMa(prepared) => {
                    let mut val_req = prepared.request.clone();
                    val_req.data_id = label.clone();
                    let val_res = match ta_app_core::run_double_ma_validation_blocking_with_candles_with_aux_mask(
                        val_req,
                        &candles,
                        cancel,
                        None,
                        ta_app_core::ValidationAuxWorkMask::preserved_finalists(),
                    ) {
                        Ok(r) => r,
                        Err(_) => {
                            ok = false;
                            break;
                        }
                    };

                    require_strict_meta_vcet_gpu_result(
                        holdout_execution.require_gpu,
                        &val_res.requested_backend,
                        &val_res.backend_used,
                        val_res.backend_route.as_deref(),
                        val_res.backend_fallback_reason.as_deref(),
                        &val_res.validation_residency_tier,
                    )?;
                    dataset_route_state.observe(
                        &val_res.backend_used,
                        val_res.backend_route.as_deref(),
                        val_res.backend_fallback_reason.as_deref(),
                        &mut dataset_route_counts,
                    );

                    if val_res.candidates.len() != prepared.candidate_targets.len() {
                        return Err(format!(
                            "double-ma holdout batch result length drifted (expected {}, got {})",
                            prepared.candidate_targets.len(),
                            val_res.candidates.len()
                        ));
                    }
                    let mut seen_result_keys: HashSet<RepresentativeParamKey> =
                        HashSet::with_capacity(val_res.candidates.len());
                    for cand in val_res.candidates {
                        let (candidate_key, targets) = prepared_candidate_targets_for_params(
                            &cand.params,
                            &prepared.candidate_idx_by_key,
                            &prepared.candidate_targets,
                        )?;
                        if !seen_result_keys.insert(candidate_key) {
                            return Err(
                                "double-ma holdout validation returned duplicate candidate parameters"
                                    .to_string(),
                            );
                        }
                        let base_metrics =
                            cand.iterative_in_sample.as_ref().unwrap_or(&cand.in_sample);
                        let sample = if heavy {
                            if hv_engine == ta_app_core::ValidationEngine::Iterative {
                                let sharpe = base_metrics.sharpe;
                                let max_dd = base_metrics.max_dd;
                                let pnl = base_metrics.pnl;
                                HoldoutMetricSample {
                                    score: sharpe - 2.0 * max_dd,
                                    sharpe,
                                    max_dd,
                                    pnl,
                                }
                            } else {
                                let score = cand.robust_score;
                                if let Some(w) = cand.walk_forward.as_ref() {
                                    HoldoutMetricSample {
                                        score,
                                        sharpe: w.sharpe_median,
                                        max_dd: w.max_dd_worst,
                                        pnl: w.pnl_median,
                                    }
                                } else {
                                    HoldoutMetricSample {
                                        score,
                                        sharpe: base_metrics.sharpe,
                                        max_dd: base_metrics.max_dd,
                                        pnl: base_metrics.pnl,
                                    }
                                }
                            }
                        } else {
                            let sharpe = base_metrics.sharpe;
                            let max_dd = base_metrics.max_dd;
                            let pnl = base_metrics.pnl;
                            HoldoutMetricSample {
                                score: sharpe - 2.0 * max_dd,
                                sharpe,
                                max_dd,
                                pnl,
                            }
                        };

                        for &dense_idx in targets {
                            local[dense_idx].push(sample);
                        }
                    }
                }
                PreparedHoldoutBatch::Composite(prepared) => {
                    let mut val_req = prepared.request.clone();
                    val_req.data_id = label.clone();
                    let val_res = match ta_app_core::run_composite_validation_blocking_with_candles_with_aux_mask(
                        val_req,
                        &candles,
                        cancel,
                        None,
                        ta_app_core::ValidationAuxWorkMask::preserved_finalists(),
                    ) {
                        Ok(r) => r,
                        Err(_) => {
                            ok = false;
                            break;
                        }
                    };

                    require_strict_meta_vcet_gpu_result(
                        holdout_execution.require_gpu,
                        &val_res.requested_backend,
                        &val_res.backend_used,
                        val_res.backend_route.as_deref(),
                        val_res.backend_fallback_reason.as_deref(),
                        &val_res.validation_residency_tier,
                    )?;
                    dataset_route_state.observe(
                        &val_res.backend_used,
                        val_res.backend_route.as_deref(),
                        val_res.backend_fallback_reason.as_deref(),
                        &mut dataset_route_counts,
                    );

                    if val_res.candidates.len() != prepared.candidate_targets.len() {
                        return Err(format!(
                            "composite holdout batch result length drifted (expected {}, got {})",
                            prepared.candidate_targets.len(),
                            val_res.candidates.len()
                        ));
                    }
                    let mut seen_result_keys: HashSet<RepresentativeParamKey> =
                        HashSet::with_capacity(val_res.candidates.len());
                    for cand in val_res.candidates {
                        let (candidate_key, targets) = prepared_candidate_targets_for_params(
                            &cand.params,
                            &prepared.candidate_idx_by_key,
                            &prepared.candidate_targets,
                        )?;
                        if !seen_result_keys.insert(candidate_key) {
                            return Err(
                                "composite holdout validation returned duplicate candidate parameters"
                                    .to_string(),
                            );
                        }
                        let base_metrics =
                            cand.iterative_in_sample.as_ref().unwrap_or(&cand.in_sample);
                        let sample = if heavy {
                            if hv_engine == ta_app_core::ValidationEngine::Iterative {
                                let sharpe = base_metrics.sharpe;
                                let max_dd = base_metrics.max_dd;
                                let pnl = base_metrics.pnl;
                                HoldoutMetricSample {
                                    score: sharpe - 2.0 * max_dd,
                                    sharpe,
                                    max_dd,
                                    pnl,
                                }
                            } else {
                                let score = cand.robust_score;
                                if let Some(w) = cand.walk_forward.as_ref() {
                                    HoldoutMetricSample {
                                        score,
                                        sharpe: w.sharpe_median,
                                        max_dd: w.max_dd_worst,
                                        pnl: w.pnl_median,
                                    }
                                } else {
                                    HoldoutMetricSample {
                                        score,
                                        sharpe: base_metrics.sharpe,
                                        max_dd: base_metrics.max_dd,
                                        pnl: base_metrics.pnl,
                                    }
                                }
                            }
                        } else {
                            let sharpe = base_metrics.sharpe;
                            let max_dd = base_metrics.max_dd;
                            let pnl = base_metrics.pnl;
                            HoldoutMetricSample {
                                score: sharpe - 2.0 * max_dd,
                                sharpe,
                                max_dd,
                                pnl,
                            }
                        };

                        for &dense_idx in targets {
                            local[dense_idx].push(sample);
                        }
                    }
                }
            }
        }

        if !ok {
            skipped = skipped.saturating_add(1);
            ta_app_core::progress::emit_double_ma_progress(
                Some(sink),
                i + 1,
                total,
                "meta_holdout",
            );
            continue;
        }

        route_evidence.record_evaluated_dataset(dataset_route_state, dataset_route_counts);

        for (dense_idx, rows) in local.iter().enumerate() {
            if rows.is_empty() {
                continue;
            }

            let Some(stats) = summary_scratch.summarize_rows(&req.strategy_kind, rows) else {
                continue;
            };

            accs[dense_idx].scores.push(stats.score_median);
            accs[dense_idx].sharpes.push(stats.sharpe_median);
            accs[dense_idx].max_dds.push(stats.max_dd_worst);
            accs[dense_idx].pnls.push(stats.pnl_median);
            if stats.pass {
                accs[dense_idx].passes = accs[dense_idx].passes.saturating_add(1);
            }
            if stats.profitable {
                accs[dense_idx].profitable = accs[dense_idx].profitable.saturating_add(1);
            }

            if req.save_holdout_rows && !holdout_rows.is_empty() {
                holdout_rows[dense_idx].push(ta_app_core::HoldoutDatasetRow {
                    dataset: label.clone(),
                    dataset_key: dataset_key.clone(),
                    score_median: stats.score_median,
                    score_p10: stats.score_p10,
                    sharpe_median: stats.sharpe_median,
                    max_dd_worst: stats.max_dd_worst,
                    pnl_median: stats.pnl_median,
                    pass: stats.pass,
                });
            }
        }

        ta_app_core::progress::emit_double_ma_progress(Some(sink), i + 1, total, "meta_holdout");
    }

    ta_app_core::progress::emit_double_ma_progress(Some(sink), total, total, "meta_holdout");

    let datasets_evaluated =
        observed_total.saturating_sub(skipped.saturating_sub(missing_expected_datasets));
    let group_metrics =
        build_holdout_group_execution_evidence(result, &active_indices, &accs, total);
    let execution_evidence = MetaHoldoutExecutionEvidence {
        requested_backend: holdout_execution.requested_backend.to_string(),
        require_gpu: holdout_execution.require_gpu,
        gpu_device_id: holdout_execution.gpu_device_id,
        datasets_requested: total,
        datasets_evaluated,
        datasets_failed: skipped,
        gpu_datasets: route_evidence.gpu_datasets,
        cpu_datasets: route_evidence.cpu_datasets,
        hybrid_datasets: route_evidence.hybrid_datasets,
        fallback_datasets: route_evidence.fallback_datasets,
        configured_auxiliary_stages: holdout_execution.configured_auxiliary_stages,
        pass_rate_denominator: "requested_unique_holdout_datasets".to_string(),
        profitable_rate_denominator: "requested_unique_holdout_datasets".to_string(),
        route_validation_batch_counts: route_evidence.route_validation_batch_counts,
        group_metrics,
    };
    require_usable_vcet_holdout_results(
        &req.strategy_kind,
        holdout_tag,
        total,
        datasets_evaluated,
    )?;
    if datasets_evaluated == 0 {
        result.holdout = Some(ta_app_core::HoldoutSummary {
            datasets: 0,
            skipped,
        });
        set_meta_holdout_coverage_for_zero_usable_datasets(result, req, &active_indices, skipped);
        result.warnings.push(format!(
            "Holdout evaluation produced zero usable dataset results for tag='{holdout_tag}'. Review dataset paths and validation failures before trusting holdout-adjusted ranking."
        ));
        return Ok(Some(execution_evidence));
    }

    apply_holdout_accumulators_to_result(
        result,
        req,
        &active_indices,
        &mut accs,
        &mut holdout_rows,
        total,
        datasets_evaluated,
        skipped,
        holdout_tag,
    );
    if execution_evidence.requested_backend == "GpuKernel"
        && execution_evidence.fallback_datasets > 0
    {
        result.warnings.push(format!(
            "Meta VCET holdout requested GpuKernel, but {} evaluated dataset(s) used a CPU fallback route. Inspect holdout_execution route evidence before promotion.",
            execution_evidence.fallback_datasets
        ));
    }

    Ok(Some(execution_evidence))
}

#[cfg(test)]
mod tests {
    use super::{
        append_stage5_training_manifest_warnings, apply_holdout_accumulators_to_result,
        artifact_matches_meta_req, artifact_matches_meta_req_relaxed,
        build_holdout_group_execution_evidence, build_meta_cohort_manifest,
        build_meta_vcet_candidate_evidence, build_metric_surface_meta_summary,
        build_metric_surface_meta_warnings, build_ml_confidence_status,
        build_prepared_holdout_batches, canonical_typed_param_key, controls_from_campaign_template,
        decode_double_ma_validation_params, evaluate_ml_confidence_policy_from_scoped_metas,
        evaluate_policy_candidates, finalize_policy_arm_metrics, group_holdout_label,
        group_policy_eval_meta_score, group_rank_score, into_owned_artifact_values,
        is_known_non_run_artifact_filename, ml_confidence_model_path_read_only,
        ml_uncertainty_from_prob, normalize_ml_policy_evaluation_request, normalize_ml_scope,
        prepared_candidate_targets_for_params, require_meta_holdout_disjointness,
        require_requested_vcet_holdout_artifacts, require_strict_meta_vcet_gpu_result,
        require_unchanged_holdout_source_fingerprint, resolve_meta_holdout_execution,
        retain_admitted_vcet_candidate_rows, should_block_stage4_for_unhealthy_model, stable_logit,
        stage4_autotrain_failure_fallback, stage4_training_metas_for_policy,
        stage5_evidence_policy_summary, strategy_supports_holdout,
        strategy_uses_double_ma_validation, suggest_meta_report_path,
        summarize_holdout_metric_columns, unique_holdout_dataset_paths, update_policy_arm_metrics,
        write_bytes_atomically, HoldoutAcc, HoldoutDatasetRouteState, HoldoutMetricSample,
        HoldoutRouteEvidenceAcc, HoldoutSummaryScratch, MlConfidenceModelState,
        MlConfidencePolicyArmMetrics, MlConfidencePolicyEvaluationRequest,
        MlConfidencePolicyEvaluationResult, MlMetaScopeSummary, PreparedHoldoutBatch,
        Stage5TrainingHistoryPolicy,
    };
    use crate::commands::metric_surface::build_optimization_metric_surface_artifact_value;
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::path::PathBuf;
    use ta_app_core::ml_confidence::MlConfidenceModelMeta;
    use ta_app_core::{
        meta_group_key_for_request, HoldoutSummary, MetaCandidateAggregate, MetaClusterConfig,
        MetaGroupingMode, MetaHoldoutMode, MetaHoldoutValidationConfig, MetaScoreWeights,
        MetaSelectionRequest, MetaSelectionResult, MlConfidenceCompatibilityKey,
        MlConfidenceEvidenceTier, MlConfidenceLabelConfig, MlConfidenceModelKind,
        MlConfidenceModelSummary, MlConfidenceScope, MlConfidenceTarget, MlConfidenceTrainConfig,
    };

    fn build_mock_app() -> tauri::App<tauri::test::MockRuntime> {
        let unique = format!(
            "commands_meta_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().product_name = Some(unique.clone());
        context.config_mut().identifier = format!("com.test.{unique}");
        tauri::test::mock_builder()
            .manage(crate::state::AppState::default())
            .build(context)
            .expect("failed to build tauri mock app")
    }

    #[test]
    fn holdout_dataset_keys_are_stable_share_safe_path_fingerprints() {
        let first = super::share_safe_holdout_dataset_key("path:c:/private/one/aapl.csv|tf:1h");
        let first_again =
            super::share_safe_holdout_dataset_key("path:c:/private/one/aapl.csv|tf:1h");
        let second = super::share_safe_holdout_dataset_key("path:d:/secret/two/aapl.csv|tf:1h");

        assert_eq!(first, first_again);
        assert_ne!(first, second);
        assert!(first.starts_with("source-fnv1a64-"));
        assert!(!first.contains("private"));
        assert!(!second.contains("secret"));
    }

    #[test]
    fn artifact_value_projection_moves_large_fixture_payloads_without_cloning() {
        const ARTIFACTS: usize = 10_000;
        let artifacts = (0..ARTIFACTS)
            .map(|index| {
                (
                    PathBuf::from(format!("run-{index}.json")),
                    json!({
                        "index": index,
                        "payload": format!("{index:05}-{}", "x".repeat(256)),
                    }),
                )
            })
            .collect::<Vec<_>>();
        let payload_addresses = artifacts
            .iter()
            .map(|(_, artifact)| {
                artifact["payload"]
                    .as_str()
                    .expect("fixture payload")
                    .as_ptr() as usize
            })
            .collect::<Vec<_>>();

        let projected = into_owned_artifact_values(artifacts);

        assert_eq!(projected.len(), ARTIFACTS);
        for (index, (artifact, original_address)) in projected
            .iter()
            .zip(payload_addresses.into_iter())
            .enumerate()
        {
            assert_eq!(artifact["index"].as_u64(), Some(index as u64));
            assert_eq!(
                artifact["payload"]
                    .as_str()
                    .expect("projected payload")
                    .as_ptr() as usize,
                original_address
            );
        }
    }

    #[test]
    fn meta_holdout_gpu_config_defaults_preserve_cpu_compatibility() {
        let cfg: MetaHoldoutValidationConfig =
            serde_json::from_value(json!({})).expect("default holdout config");

        assert!(!cfg.use_gpu);
        assert!(!cfg.require_gpu);
        assert_eq!(cfg.gpu_device_id, 0);
        assert_eq!(
            cfg.validation_engine,
            ta_app_core::ValidationEngine::Vectorized
        );
    }

    #[test]
    fn meta_holdout_gpu_policy_is_vcet_only_and_vectorized() {
        let mut req = test_meta_req();
        req.strategy_kind = "Vcet".to_string();
        req.holdout_validation = Some(
            serde_json::from_value(json!({
                "require_gpu": true,
                "gpu_device_id": 3,
                "validation_engine": "vectorized"
            }))
            .expect("GPU holdout config"),
        );

        let resolved =
            resolve_meta_holdout_execution(&req, ta_app_core::ValidationEngine::Vectorized)
                .expect("VCET GPU execution");
        assert!(resolved.require_gpu);
        assert_eq!(resolved.gpu_device_id, Some(3));
        assert!(matches!(
            resolved.backend,
            ta_app_core::Backend::GpuKernel { device_id: 3 }
        ));

        req.strategy_kind = "CompositeMo".to_string();
        let non_vcet =
            resolve_meta_holdout_execution(&req, ta_app_core::ValidationEngine::Vectorized)
                .expect_err("non-VCET GPU execution must fail");
        assert!(non_vcet.contains("supports strategy_kind='Vcet' only"));

        req.strategy_kind = "Vcet".to_string();
        let iterative =
            resolve_meta_holdout_execution(&req, ta_app_core::ValidationEngine::Iterative)
                .expect_err("iterative GPU execution must fail");
        assert!(iterative.contains("requires validation_engine='vectorized'"));

        req.holdout_mode = MetaHoldoutMode::Heavy;
        req.holdout_validation
            .as_mut()
            .expect("holdout config")
            .walk_forward = Some(
            serde_json::from_value(json!({
                "window_bars": 20_000,
                "step_bars": 20_000
            }))
            .expect("walk-forward config"),
        );
        let unproven_stage =
            resolve_meta_holdout_execution(&req, ta_app_core::ValidationEngine::Vectorized)
                .expect_err("strict GPU must reject unproven auxiliary stages");
        assert!(unproven_stage.contains("walk_forward"), "{unproven_stage}");
        assert!(
            unproven_stage.contains("require_gpu=false"),
            "{unproven_stage}"
        );

        let cfg = req.holdout_validation.as_mut().expect("holdout config");
        cfg.require_gpu = false;
        cfg.use_gpu = true;
        let hybrid =
            resolve_meta_holdout_execution(&req, ta_app_core::ValidationEngine::Vectorized)
                .expect("non-strict GPU may use auxiliary CPU stages");
        assert_eq!(
            hybrid.configured_auxiliary_stages,
            vec!["walk_forward".to_string()]
        );
    }

    #[test]
    fn strict_meta_vcet_gpu_route_rejects_cpu_fallback() {
        require_strict_meta_vcet_gpu_result(
            true,
            "GpuKernel",
            &ta_app_core::BackendUsed::Gpu,
            Some("GpuKernel"),
            None,
            "gpu_kernel",
        )
        .expect("fully GPU-resident route");

        let error = require_strict_meta_vcet_gpu_result(
            true,
            "GpuKernel",
            &ta_app_core::BackendUsed::Cpu,
            Some("GpuKernel->CpuOnly"),
            Some("CUDA feature unavailable"),
            "cpu_fallback",
        )
        .expect_err("strict route must reject fallback");
        assert!(error.contains("backend_used=Cpu"), "{error}");
        assert!(error.contains("GpuKernel->CpuOnly"), "{error}");
        assert!(error.contains("CUDA feature unavailable"), "{error}");
    }

    #[test]
    fn mixed_holdout_routes_are_counted_once_as_hybrid_with_fallback_orthogonal() {
        let mut state = HoldoutDatasetRouteState::default();
        let mut route_counts = std::collections::BTreeMap::new();
        state.observe(
            &ta_app_core::BackendUsed::Gpu,
            Some("GpuKernel"),
            None,
            &mut route_counts,
        );
        state.observe(
            &ta_app_core::BackendUsed::Cpu,
            Some("GpuKernel->CpuOnly"),
            Some("fallback"),
            &mut route_counts,
        );
        let mut evidence = HoldoutRouteEvidenceAcc::default();

        evidence.record_evaluated_dataset(state, route_counts);

        let mut hybrid_state = HoldoutDatasetRouteState {
            saw_cpu: true,
            ..HoldoutDatasetRouteState::default()
        };
        let mut hybrid_routes = std::collections::BTreeMap::new();
        hybrid_state.observe(
            &ta_app_core::BackendUsed::Gpu,
            Some("GpuKernel"),
            None,
            &mut hybrid_routes,
        );
        evidence.record_evaluated_dataset(hybrid_state, hybrid_routes);

        assert_eq!(evidence.gpu_datasets, 0);
        assert_eq!(evidence.cpu_datasets, 0);
        assert_eq!(evidence.hybrid_datasets, 2);
        assert_eq!(evidence.fallback_datasets, 1);
        assert_eq!(evidence.route_validation_batch_counts["GpuKernel"], 2);
        assert_eq!(
            evidence.route_validation_batch_counts["GpuKernel->CpuOnly"],
            1
        );
    }

    #[test]
    fn one_dataset_multi_config_counts_validation_batches_not_datasets() {
        let mut state = HoldoutDatasetRouteState::default();
        let mut route_counts = std::collections::BTreeMap::new();
        state.observe(
            &ta_app_core::BackendUsed::Gpu,
            Some("GpuKernel"),
            None,
            &mut route_counts,
        );
        state.observe(
            &ta_app_core::BackendUsed::Gpu,
            Some("GpuKernel"),
            None,
            &mut route_counts,
        );
        state.observe(
            &ta_app_core::BackendUsed::Cpu,
            Some("CpuOnly"),
            None,
            &mut route_counts,
        );
        let mut evidence = HoldoutRouteEvidenceAcc::default();

        evidence.record_evaluated_dataset(state, route_counts);

        assert_eq!(
            evidence.gpu_datasets + evidence.cpu_datasets + evidence.hybrid_datasets,
            1
        );
        assert_eq!(evidence.hybrid_datasets, 1);
        assert_eq!(evidence.route_validation_batch_counts["GpuKernel"], 2);
        assert_eq!(evidence.route_validation_batch_counts["CpuOnly"], 1);
    }

    #[test]
    fn holdout_dataset_universe_dedupes_by_content_identity_not_path() {
        let first = json!({
            "data": {
                "csv_path": "c:/private/aapl-copy-one.csv",
                "source_content_fingerprint": "same-bars"
            },
            "dataset_meta": { "label": "AAPL one", "timeframe": "1m" }
        });
        let duplicate = json!({
            "data": {
                "csv_path": "d:/other/aapl-copy-two.csv",
                "source_content_fingerprint": "same-bars"
            },
            "dataset_meta": { "label": "AAPL two", "timeframe": "1m" }
        });
        let distinct = json!({
            "data": {
                "csv_path": "c:/private/msft.csv",
                "source_content_fingerprint": "different-bars"
            },
            "dataset_meta": { "label": "MSFT", "timeframe": "1m" }
        });
        let missing_path = json!({
            "data": { "source_content_fingerprint": "missing-path-bars" },
            "dataset_meta": { "label": "Unavailable", "timeframe": "1m" }
        });

        let unique = unique_holdout_dataset_paths(&[&first, &duplicate, &distinct, &missing_path]);

        assert_eq!(unique.len(), 3);
        assert_eq!(unique[0].0, "c:/private/aapl-copy-one.csv");
        assert_eq!(unique[1].0, "c:/private/msft.csv");
        assert!(unique[2].0.is_empty());
        assert_eq!(unique[2].1, "Unavailable");
        assert_ne!(unique[0].2, unique[1].2);
        assert!(!unique[0].2.contains("private"));
        assert_eq!(unique[0].3.as_deref(), Some("same-bars"));
        assert_eq!(unique[1].3.as_deref(), Some("different-bars"));
    }

    #[test]
    fn campaign_holdout_rejects_source_content_changed_after_capture() {
        require_unchanged_holdout_source_fingerprint(
            Some("sha256:captured"),
            "sha256:captured",
            "AAPL",
        )
        .expect("unchanged holdout source remains valid");
        require_unchanged_holdout_source_fingerprint(None, "sha256:current", "legacy")
            .expect("legacy unbound holdout has no captured fingerprint contract");
        let error = require_unchanged_holdout_source_fingerprint(
            Some("sha256:captured"),
            "sha256:mutated",
            "AAPL",
        )
        .expect_err("mutated holdout source must fail before backtest");
        assert!(error.contains("changed after campaign capture"));
        assert!(error.contains("AAPL"));
    }

    fn test_meta_req() -> MetaSelectionRequest {
        MetaSelectionRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: Some("bench".to_string()),
            holdout_tag: None,
            eval_holdout: true,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 25,
            min_samples: 5,
            strict_shortlist: true,
            allow_shortlist_fallback: false,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 1,
        }
    }

    fn make_meta_candidate(
        req: &MetaSelectionRequest,
        fast_len: u16,
        slow_len: u16,
        meta_score: f64,
    ) -> MetaCandidateAggregate {
        let params = json!({
            "fast_ma_type": "sma",
            "slow_ma_type": "sma",
            "fast_len": fast_len,
            "slow_len": slow_len,
        });
        let key = meta_group_key_for_request(req, &params, "close", &None, &None);
        MetaCandidateAggregate {
            params: params.clone(),
            key,
            ma_source: "close".to_string(),
            feature_context: None,
            run_context: None,
            fast_ma_params: None,
            slow_ma_params: None,
            representatives: vec![params],
            samples: 8,
            effective_samples_lower_bound: 1,
            datasets_covered: 1,
            pass_rate: 1.0,
            robust_score_median: meta_score + 0.1,
            robust_score_p10: meta_score,
            tail_gap: 0.05,
            max_dd_p90: 0.20,
            cross_score_median: Some(0.05),
            psr_median: 0.0,
            dsr_median: 0.0,
            meta_score,
            final_score: None,
            holdout: None,
            holdout_coverage: None,
            evidence_flags: Vec::new(),
            train_rows: Vec::new(),
            holdout_rows: Vec::new(),
        }
    }

    fn make_policy_eval_meta_artifact(strategy_kind: &str, ts: &str, groups: Vec<Value>) -> Value {
        json!({
            "kind": "vectorgrid_meta_selection",
            "ts": ts,
            "req": {
                "strategy_kind": strategy_kind,
                "timeframe": "1m"
            },
            "res": {
                "strategy_kind": strategy_kind,
                "timeframe": "1m",
                "groups": groups
            }
        })
    }

    fn make_policy_eval_group(strong: bool, gid: usize) -> Value {
        let mut rows = Vec::new();
        for ds in 0..6usize {
            let pass = if strong { ds != 0 } else { ds == 0 };
            rows.push(json!({
                "dataset": format!("DS{gid:03}_{ds:02}"),
                "score_median": if pass { 0.20 } else { -0.10 },
                "score_p10": if pass { 0.08 } else { -0.15 },
                "sharpe_median": if pass { 0.95 } else { -0.45 },
                "max_dd_worst": if pass { 0.28 } else { 0.60 },
                "pnl_median": if pass { 1.0 } else { -0.9 },
                "pass": pass
            }));
        }
        json!({
            "samples": 42,
            "pass_rate": if strong { 0.82 } else { 0.26 },
            "robust_score_p10": if strong { 0.16 } else { -0.10 },
            "robust_score_median": if strong { 0.22 } else { -0.03 },
            "max_dd_p90": if strong { 0.24 } else { 0.58 },
            "cross_score_median": if strong { 0.06 } else { -0.05 },
            "meta_score": if strong { 0.26 } else { -0.18 },
            "holdout": {
                "datasets": 6,
                "pass_rate": if strong { 0.8333333333 } else { 0.1666666667 },
                "score_p10": if strong { 0.08 } else { -0.15 },
                "score_median": if strong { 0.17 } else { -0.05 },
                "sharpe_median": if strong { 1.0 } else { -0.4 },
                "max_dd_p90": if strong { 0.26 } else { 0.56 },
                "pnl_median": if strong { 1.0 } else { -0.8 }
            },
            "holdout_rows": rows
        })
    }

    fn run_policy_eval_reference_from_raw(
        req: MlConfidencePolicyEvaluationRequest,
        scoped: &[Value],
    ) -> MlConfidencePolicyEvaluationResult {
        let req = normalize_ml_policy_evaluation_request(req).expect("normalize policy req");
        let scope = normalize_ml_scope(MlConfidenceScope {
            strategy_kind: req.strategy_kind.clone(),
            timeframe: req.timeframe.clone().unwrap_or_default(),
        })
        .expect("normalize scope");
        let label_cfg = req.label.clone().unwrap_or_default();

        let artifacts_considered = scoped.len();
        let mut artifacts_evaluated = 0usize;
        let mut artifacts_skipped_no_history = 0usize;
        let mut artifacts_skipped_training_failure = 0usize;
        let mut artifacts_skipped_unhealthy_model = 0usize;
        let mut artifacts_skipped_no_labels = 0usize;
        let mut total_label_rows = 0usize;
        let mut positive_label_rows = 0usize;
        let mut warnings: Vec<String> = Vec::new();

        let mut meta_only = MlConfidencePolicyArmMetrics::default();
        let mut ml_only = MlConfidencePolicyArmMetrics::default();
        let mut stage5_blended = MlConfidencePolicyArmMetrics::default();

        for idx in 0..scoped.len() {
            if idx == 0 {
                artifacts_skipped_no_history = artifacts_skipped_no_history.saturating_add(1);
                continue;
            }

            let mut train_cfg = MlConfidenceTrainConfig::default();
            train_cfg.label = label_cfg.clone();
            let (model, metrics) = match ta_app_core::train_ml_confidence_model_from_meta_artifacts(
                scope.clone(),
                &scoped[..idx],
                train_cfg,
            ) {
                Ok(v) => v,
                Err(_) => {
                    artifacts_skipped_training_failure =
                        artifacts_skipped_training_failure.saturating_add(1);
                    continue;
                }
            };
            if req.require_healthy_model && !metrics.healthy {
                artifacts_skipped_unhealthy_model =
                    artifacts_skipped_unhealthy_model.saturating_add(1);
                continue;
            }

            let groups = scoped[idx]
                .pointer("/res/groups")
                .and_then(|v| v.as_array())
                .map(|groups| groups.as_slice())
                .unwrap_or(&[]);
            let mut candidates: Vec<super::PolicyEvalCandidate> = Vec::new();
            for group in groups {
                let Some(label) = group_holdout_label(group, &label_cfg) else {
                    continue;
                };
                let Some(features) = ta_app_core::extract_ml_confidence_group_features(group)
                else {
                    continue;
                };
                let Some(probability) =
                    ta_app_core::predict_ml_confidence_for_features(&model, &features)
                        .filter(|p| p.is_finite())
                else {
                    continue;
                };
                let raw_meta_score = group_policy_eval_meta_score(group);
                let meta_score = if raw_meta_score.is_finite() {
                    raw_meta_score
                } else {
                    0.0
                };
                candidates.push(super::PolicyEvalCandidate {
                    label,
                    meta_score,
                    ml_confidence: probability,
                    ml_logit: stable_logit(probability),
                    ml_uncertainty: ml_uncertainty_from_prob(probability),
                });
            }

            if candidates.is_empty() {
                artifacts_skipped_no_labels = artifacts_skipped_no_labels.saturating_add(1);
                continue;
            }

            total_label_rows = total_label_rows.saturating_add(candidates.len());
            positive_label_rows = positive_label_rows.saturating_add(
                candidates
                    .iter()
                    .filter(|candidate| candidate.label > 0.5)
                    .count(),
            );
            let (meta_labels, ml_labels, stage5_labels) =
                evaluate_policy_candidates(&req, &candidates);
            update_policy_arm_metrics(&mut meta_only, &meta_labels);
            update_policy_arm_metrics(&mut ml_only, &ml_labels);
            update_policy_arm_metrics(&mut stage5_blended, &stage5_labels);
            artifacts_evaluated = artifacts_evaluated.saturating_add(1);
        }

        if artifacts_considered == 0 {
            warnings.push("No meta-selection artifacts found for this scope.".to_string());
        } else if artifacts_evaluated == 0 {
            warnings.push(
                "No scoped artifacts produced a comparable policy-evaluation result.".to_string(),
            );
        }

        let base_rate = if total_label_rows == 0 {
            0.0
        } else {
            positive_label_rows as f64 / total_label_rows as f64
        };
        finalize_policy_arm_metrics(&mut meta_only, total_label_rows, positive_label_rows);
        finalize_policy_arm_metrics(&mut ml_only, total_label_rows, positive_label_rows);
        finalize_policy_arm_metrics(&mut stage5_blended, total_label_rows, positive_label_rows);

        let meta_precision = meta_only.precision_at_k;
        let ml_precision = ml_only.precision_at_k;
        let stage5_precision = stage5_blended.precision_at_k;
        let best_policy = if stage5_precision >= meta_precision && stage5_precision >= ml_precision
        {
            "stage5_blended"
        } else if meta_precision >= ml_precision {
            "meta_only"
        } else {
            "ml_only"
        }
        .to_string();

        MlConfidencePolicyEvaluationResult {
            strategy_kind: scope.strategy_kind,
            timeframe: scope.timeframe,
            artifacts_considered,
            artifacts_evaluated,
            artifacts_skipped_no_history,
            artifacts_skipped_training_failure,
            artifacts_skipped_unhealthy_model,
            artifacts_skipped_no_labels,
            total_label_rows,
            positive_label_rows,
            base_rate,
            top_k: req.top_k,
            min_ml_confidence: req.min_ml_confidence,
            meta_weight: req.meta_weight,
            ml_weight: req.ml_weight,
            uncertainty_weight: req.uncertainty_weight,
            meta_only,
            ml_only,
            stage5_blended,
            best_policy,
            stage5_vs_meta_precision_delta: stage5_precision - meta_precision,
            stage5_vs_ml_precision_delta: stage5_precision - ml_precision,
            warnings,
        }
    }

    #[test]
    fn stage4_training_set_excludes_source_meta_artifact() {
        let source = PathBuf::from("runs/vectorgrid_meta_source.json");
        let other = PathBuf::from("runs/vectorgrid_meta_other.json");
        let metas = vec![
            (
                source.clone(),
                json!({"kind":"vectorbt_meta_selection","ts":"2026-01-01T00:00:00Z"}),
            ),
            (
                other,
                json!({"kind":"vectorbt_meta_selection","ts":"2026-01-01T00:01:00Z"}),
            ),
        ];

        let source_meta = json!({"kind":"vectorbt_meta_selection","ts":"2026-01-01T00:00:00Z"});
        let (filtered, manifest) = stage4_training_metas_for_policy(
            metas,
            &source,
            &source_meta,
            Stage5TrainingHistoryPolicy::SourceExcluded,
        );
        assert_eq!(manifest.excluded_source_count, 1);
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].get("ts").and_then(|v| v.as_str()),
            Some("2026-01-01T00:01:00Z")
        );
    }

    #[test]
    fn stage5_evidence_policy_labels_confidence_as_holdout_probability() {
        let summary = stage5_evidence_policy_summary();
        assert_eq!(summary.workflow, "meta_stage5_ml_confidence");
        let confidence = summary
            .metrics
            .iter()
            .find(|metric| metric.metric == "ml_confidence")
            .expect("ml confidence metric");
        assert_eq!(
            confidence.interpretation,
            "Model-estimated holdout success probability for a meta-selection group."
        );
        assert!(summary
            .caveats
            .iter()
            .any(|caveat| caveat.contains("not deployability proof")));
    }

    #[test]
    fn stage4_training_set_excludes_duplicate_source_content() {
        let source = PathBuf::from("runs/vectorgrid_meta_source.json");
        let duplicate = PathBuf::from("runs/copy_of_source.json");
        let other = PathBuf::from("runs/vectorgrid_meta_other.json");
        let source_meta = json!({
            "kind": "vectorbt_meta_selection",
            "ts": "2026-01-01T00:00:00Z",
            "res": { "strategy_kind": "CompositeMo", "timeframe": "1m" }
        });
        let metas = vec![
            (source.clone(), source_meta.clone()),
            (duplicate, source_meta.clone()),
            (
                other,
                json!({
                    "kind": "vectorbt_meta_selection",
                    "ts": "2026-01-01T00:01:00Z"
                }),
            ),
        ];

        let (filtered, manifest) = stage4_training_metas_for_policy(
            metas,
            &source,
            &source_meta,
            Stage5TrainingHistoryPolicy::SourceExcluded,
        );

        assert_eq!(manifest.excluded_source_count, 2);
        assert_eq!(filtered.len(), 1);
        assert!(manifest
            .excluded_artifacts
            .iter()
            .any(|entry| entry.reason == "source_artifact_duplicate"));
    }

    #[test]
    fn stage5_training_manifest_warnings_flag_thin_history_and_missing_source_match() {
        let source = PathBuf::from("runs/vectorgrid_meta_source.json");
        let source_meta = json!({
            "kind": "vectorbt_meta_selection",
            "ts": "2026-01-02T00:00:00Z"
        });
        let metas = vec![(
            PathBuf::from("runs/vectorgrid_meta_other.json"),
            json!({"kind":"vectorbt_meta_selection","ts":"2026-01-01T00:00:00Z"}),
        )];

        let (_filtered, manifest) = stage4_training_metas_for_policy(
            metas,
            &source,
            &source_meta,
            Stage5TrainingHistoryPolicy::SourceExcluded,
        );
        let mut warnings = Vec::new();
        append_stage5_training_manifest_warnings(&manifest, &mut warnings);

        assert_eq!(manifest.excluded_source_count, 0);
        assert_eq!(manifest.training_artifact_count, 1);
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("did not find the active source meta artifact")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("history is thin")));
    }

    #[test]
    fn stage5_training_manifest_warnings_flag_source_exclusion_and_chronological_filters() {
        let source = PathBuf::from("runs/vectorgrid_meta_source.json");
        let source_meta = json!({
            "kind": "vectorbt_meta_selection",
            "ts": "2026-01-02T00:00:00Z"
        });
        let metas = vec![
            (source.clone(), source_meta.clone()),
            (
                PathBuf::from("runs/past.json"),
                json!({"kind":"vectorbt_meta_selection","ts":"2026-01-01T00:00:00Z"}),
            ),
            (
                PathBuf::from("runs/future.json"),
                json!({"kind":"vectorbt_meta_selection","ts":"2026-01-03T00:00:00Z"}),
            ),
            (
                PathBuf::from("runs/missing_ts.json"),
                json!({"kind":"vectorbt_meta_selection"}),
            ),
        ];

        let (_filtered, manifest) = stage4_training_metas_for_policy(
            metas,
            &source,
            &source_meta,
            Stage5TrainingHistoryPolicy::ChronologicalBeforeSource,
        );
        let mut warnings = Vec::new();
        append_stage5_training_manifest_warnings(&manifest, &mut warnings);

        assert!(warnings
            .iter()
            .any(|warning| warning.contains("excluded 1 source meta artifact")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("excluded 1 source-or-future")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("missing/invalid timestamps")));
    }

    #[test]
    fn stage4_chronological_training_policy_excludes_future_equal_and_missing_ts() {
        let source = PathBuf::from("runs/vectorgrid_meta_source.json");
        let source_meta = json!({
            "kind": "vectorbt_meta_selection",
            "ts": "2026-01-02T00:00:00Z"
        });
        let metas = vec![
            (
                PathBuf::from("runs/past.json"),
                json!({"kind":"vectorbt_meta_selection","ts":"2026-01-01T00:00:00Z"}),
            ),
            (
                PathBuf::from("runs/equal.json"),
                json!({"kind":"vectorbt_meta_selection","ts":"2026-01-02T00:00:00Z","id":"equal"}),
            ),
            (
                PathBuf::from("runs/future.json"),
                json!({"kind":"vectorbt_meta_selection","ts":"2026-01-03T00:00:00Z"}),
            ),
            (
                PathBuf::from("runs/missing_ts.json"),
                json!({"kind":"vectorbt_meta_selection"}),
            ),
        ];

        let (filtered, manifest) = stage4_training_metas_for_policy(
            metas,
            &source,
            &source_meta,
            Stage5TrainingHistoryPolicy::ChronologicalBeforeSource,
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].get("ts").and_then(|v| v.as_str()),
            Some("2026-01-01T00:00:00Z")
        );
        assert_eq!(manifest.excluded_future_or_equal_count, 2);
        assert_eq!(manifest.excluded_missing_ts_count, 1);
    }

    #[test]
    fn meta_cohort_manifest_normalizes_paths_for_disjointness_checks() {
        let train = vec![(
            PathBuf::from("runs/train.json"),
            json!({
                "kind": "vectorgrid_report",
                "data": { "csv_path": "C:\\Data\\SPY.csv" },
                "dataset_meta": { "timeframe": "60s" }
            }),
        )];
        let holdout = vec![(
            PathBuf::from("runs/holdout.json"),
            json!({
                "kind": "vectorgrid_report",
                "data": { "csv_path": "c:/data/spy.csv" },
                "dataset_meta": { "timeframe": "1m" }
            }),
        )];

        let manifest = build_meta_cohort_manifest(&train, &holdout, Some("holdout"), None, None);

        assert_eq!(manifest.disjointness.overlapping_dataset_keys, 1);
        assert!(!manifest.disjointness.disjoint);
        assert!(manifest
            .warnings
            .iter()
            .any(|warning| warning.contains("share 1 dataset key")));
    }

    #[test]
    fn campaign_bound_meta_retains_only_cross_universe_admitted_vcet_candidates() {
        let artifact = json!({
            "validation": {
                "res": {
                    "candidates": [
                        { "params": { "fast_len": 6, "slow_len": 24 }, "in_sample": { "pnl": 1.0 } },
                        { "params": { "fast_len": 12, "slow_len": 48 }, "in_sample": { "pnl": 100.0 } },
                        { "params": { "fast_len": 20, "slow_len": 80 }, "in_sample": { "pnl": 2.0 } }
                    ]
                }
            }
        });
        let admitted: HashSet<(u16, u16)> = [(6, 24), (20, 80)].into_iter().collect();

        for role in ["training", "holdout"] {
            let mut scoped = artifact.clone();
            scoped["campaign"] = json!({ "cohort_role": role });
            retain_admitted_vcet_candidate_rows(&mut scoped, &admitted)
                .expect("both campaign cohorts retain the exact admitted candidate set");
            let periods: Vec<(u64, u64)> = scoped
                .pointer("/validation/res/candidates")
                .and_then(Value::as_array)
                .expect("candidate rows")
                .iter()
                .map(|row| {
                    (
                        row.pointer("/params/fast_len")
                            .and_then(Value::as_u64)
                            .expect("fast_len"),
                        row.pointer("/params/slow_len")
                            .and_then(Value::as_u64)
                            .expect("slow_len"),
                    )
                })
                .collect();
            assert_eq!(periods, vec![(6, 24), (20, 80)]);
        }

        let missing: HashSet<(u16, u16)> = [(6, 24), (99, 100)].into_iter().collect();
        let mut incomplete = artifact;
        assert!(
            retain_admitted_vcet_candidate_rows(&mut incomplete, &missing)
                .expect_err("every admitted candidate must exist in each validated artifact")
                .contains("expected 2")
        );
    }

    #[test]
    fn campaign_bound_meta_projects_qmc_controls_by_evaluation_candidate_identity() {
        let mut search_config = ta_app_core::vcet_search::VcetSearchConfig::default();
        search_config.budget = 1;
        search_config.seed = 99;
        let aggregate = json!({
            "candidate_id": "vcet_candidate_joined",
            "fast_len": 8,
            "slow_len": 34,
            "requested_datasets": 10,
            "evaluated_datasets": 9,
            "missing_or_failed_datasets": 1,
            "profitable_datasets": 7,
            "hard_gate_pass_datasets": 6,
            "profitable_share_requested": 0.7,
            "hard_gate_pass_share_requested": 0.6,
            "pnl_p10": 0.02,
            "pnl_median": 0.08,
            "sharpe_p10": 0.31,
            "sharpe_median": 0.72,
            "max_drawdown_p90": 0.14,
            "trades_median": 23.0
        });
        let search_manifest = json!({
            "schema_version": 1,
            "sequence": "halton_cranley_patterson",
            "search_id": "vcet_search_test",
            "seed": 99,
            "requested_budget": 1,
            "effective_budget": 1,
            "valid_search_space_size": "500000",
            "float_identity_scale": 1000000,
            "generation_stats": {
                "halton_attempts": 4,
                "invalid_period_pairs_discarded": 1,
                "duplicate_candidates_discarded": 0,
                "lexicographic_fallback_candidates": 0,
                "exhaustive_candidates": 0
            },
            "candidates": [{
                "ordinal": 7,
                "candidate_id": "vcet_v1_generator_identity",
                "evaluation_candidate_id": "vcet_candidate_joined",
                "identity_inputs": {
                    "vidya_short_period": 8,
                    "vidya_long_period": 34,
                    "vidya_alpha_micros": 275000,
                    "ce_period": 26,
                    "ce_mult_micros": 3250000,
                    "ce_use_close": false,
                    "require_vidya_slope": true,
                    "max_trade_bars": 150
                },
                "effective": {
                    "vidya_short_period": 8,
                    "vidya_long_period": 34,
                    "vidya_alpha": 0.275,
                    "ce_period": 26,
                    "ce_mult": 3.25,
                    "ce_use_close": false,
                    "require_vidya_slope": true,
                    "max_trade_bars": 150
                }
            }]
        });
        let request = json!({
            "strategy_kind": "Vcet",
            "datasets": [],
            "execution_mode": "evaluate_candidates",
            "optimization_template": {
                "vcet": {
                    "vidya_short_period": 6,
                    "vidya_long_period": 24,
                    "vidya_alpha": 0.2,
                    "ce_period": 22,
                    "ce_mult": 3.0,
                    "ce_use_close": true,
                    "require_vidya_slope": true,
                    "max_trade_bars": 120
                }
            },
            "validation_config": {
                "max_candidates": 1,
                "vcet_search": serde_json::to_value(&search_config).expect("search config")
            }
        });
        let summary: crate::commands::commands_campaign::ResearchCampaignSummary =
            serde_json::from_value(json!({
                "version": 2,
                "kind": "vectorgrid_campaign_summary",
                "ts": "2026-08-02T00:00:00Z",
                "state": "completed",
                "strategy_kind": "Vcet",
                "timeframe": "1m",
                "tags": [],
                "holdout_pct": 20,
                "request": request,
                "counts": {
                    "total": 10,
                    "optimized": 0,
                    "validated": 10,
                    "skipped": 0,
                    "failed": 0,
                    "cancelled": false
                },
                "items": [],
                "fixed_candidate_evaluation": {
                    "schema_version": 4,
                    "candidate_manifest_hash": "sha256:candidate-manifest",
                    "search_manifest": search_manifest,
                    "require_gpu_kernel": true,
                    "requested_datasets": 10,
                    "evaluated_datasets": 9,
                    "gpu_kernel_datasets": 9,
                    "missing_or_failed_datasets": 1,
                    "candidates": [aggregate.clone()],
                    "selection_cohort": "training",
                    "selected_candidate_id": "vcet_candidate_joined",
                    "admitted_candidate_count": 1,
                    "admitted_candidate_ids": ["vcet_candidate_joined"],
                    "selection_status": "selected",
                    "all_universe": {
                        "requested_datasets": 10,
                        "evaluated_datasets": 9,
                        "gpu_kernel_datasets": 9,
                        "missing_or_failed_datasets": 1,
                        "candidates": [aggregate.clone()]
                    },
                    "training": {
                        "requested_datasets": 8,
                        "evaluated_datasets": 8,
                        "gpu_kernel_datasets": 8,
                        "missing_or_failed_datasets": 0,
                        "candidates": [aggregate.clone()]
                    },
                    "holdout": {
                        "requested_datasets": 2,
                        "evaluated_datasets": 1,
                        "gpu_kernel_datasets": 1,
                        "missing_or_failed_datasets": 1,
                        "candidates": [aggregate]
                    }
                },
                "runtime_ms": 12
            }))
            .expect("campaign summary");

        let evidence = build_meta_vcet_candidate_evidence(&summary).expect("VCET evidence");
        assert_eq!(evidence.source_candidate_count, 1);
        assert_eq!(evidence.included_candidate_count, 1);
        assert_eq!(evidence.omitted_candidate_count, 0);
        let search = evidence.search.as_ref().expect("search provenance");
        assert_eq!(search.sequence, "halton_cranley_patterson");
        assert_eq!(search.exhaustive_candidates, 0);
        assert_eq!(search.config.as_ref().map(|config| config.seed), Some(99));
        let row = &evidence.candidates[0];
        assert_eq!(row.manifest_ordinal, 7);
        assert!(row.selected);
        assert!(row.admitted);
        assert_eq!(row.controls.evidence_state, "sealed_search_manifest");
        assert_eq!(row.controls.vidya_short_period, Some(8));
        assert_eq!(row.controls.vidya_long_period, Some(34));
        assert_eq!(row.controls.vidya_alpha, Some(0.275));
        assert_eq!(row.controls.ce_period, Some(26));
        assert_eq!(row.controls.ce_mult, Some(3.25));
        assert_eq!(row.controls.ce_use_close, Some(false));
        assert_eq!(row.controls.require_vidya_slope, Some(true));
        assert_eq!(row.controls.max_trade_bars, Some(150));
        assert!(row.holdout.is_some());
        assert!(row.all_universe.is_some());
    }

    #[test]
    fn campaign_bound_meta_resolves_legacy_fixed_pair_controls_from_template() {
        let mut vcet = ta_app_core::VcetStrategyParams::default();
        vcet.vidya_alpha = 0.4;
        vcet.ce_mult = 2.75;
        vcet.ce_use_close = false;
        vcet.require_vidya_slope = false;
        vcet.max_trade_bars = 180;

        let controls = controls_from_campaign_template(Some(&vcet), 8, 34);

        assert_eq!(controls.evidence_state, "resolved_campaign_template");
        assert_eq!(controls.vidya_short_period, Some(8));
        assert_eq!(controls.vidya_long_period, Some(34));
        assert_eq!(controls.ce_period, Some(34));
        assert_eq!(controls.vidya_alpha, Some(0.4));
        assert_eq!(controls.ce_mult, Some(2.75));
        assert_eq!(controls.ce_use_close, Some(false));
        assert_eq!(controls.require_vidya_slope, Some(false));
        assert_eq!(controls.max_trade_bars, Some(180));
    }

    fn test_campaign_cohort_expectation(
        artifact_path: &std::path::Path,
    ) -> super::MetaCampaignCohortExpectation {
        let mut expectation = super::MetaCampaignCohortExpectation {
            lineage: super::MetaCampaignCohortLineage {
                campaign_summary_path: "C:/runs/campaign.json".to_string(),
                campaign_summary_sha256: "campaign-summary-sha256".to_string(),
                manifest_hash: "sha256:cohort-current".to_string(),
                candidate_manifest_hash: Some("sha256:candidates".to_string()),
                admitted_candidate_count: 1,
                admitted_candidates: vec![super::MetaVcetAdmittedCandidate {
                    candidate_id: "vcet_candidate_test".to_string(),
                    fast_len: 6,
                    slow_len: 24,
                }],
                cross_universe_admission: Default::default(),
                selected_candidate_id: Some("vcet_candidate_test".to_string()),
                expected_train_datasets: 2,
                expected_holdout_datasets: 1,
                matched_train_artifacts: 0,
                matched_holdout_artifacts: 0,
                excluded_out_of_scope_artifacts: 0,
                vcet_candidate_evidence: None,
            },
            fingerprints: HashMap::new(),
            paths: HashMap::new(),
            artifact_paths: HashMap::new(),
            referenced_artifact_paths: Vec::new(),
            candidate_artifact_contract: None,
            admitted_vcet_periods: None,
            campaign_summary: None,
        };
        let artifact_path_string = artifact_path.to_string_lossy().to_string();
        let artifact_sha256 = "d".repeat(64);
        super::insert_expected_campaign_identity(
            &mut expectation,
            super::cohort_path_timeframe_key("C:/data/AAPL.csv", "1m"),
            Some("bars-aapl".to_string()),
            Some(&artifact_path_string),
            Some(&artifact_sha256),
            super::MetaCampaignCohortRole::Train,
        )
        .expect("expected campaign identity");
        expectation
            .referenced_artifact_paths
            .push(artifact_path.to_path_buf());
        expectation
    }

    #[test]
    fn campaign_cohort_artifacts_require_consistent_lineage_and_dataset_identity() {
        let artifact_path = PathBuf::from("C:/runs/vectorgrid_run_current.json");
        let expectation = test_campaign_cohort_expectation(&artifact_path);
        let artifact = json!({
            "kind": "vectorgrid_run_artifact",
            "data": { "csv_path": "c:\\data\\aapl.csv" },
            "dataset_meta": {
                "timeframe": "60s",
                "source_content_fingerprint": "bars-aapl"
            },
            "validation": {
                "req": { "candidate_manifest_hash": "sha256:candidates" }
            },
            "campaign": {
                "manifest_hash": "sha256:cohort-current",
                "cohort_id": "sha256:cohort-current",
                "cohort_role": "training"
            }
        });

        assert_eq!(
            super::campaign_cohort_role_for_artifact(
                &expectation,
                artifact_path.as_path(),
                &artifact,
            ),
            Some(super::MetaCampaignCohortRole::Train)
        );

        for (label, pointer, replacement) in [
            (
                "cohort",
                "/campaign/cohort_id",
                json!("sha256:cohort-older"),
            ),
            ("role", "/campaign/cohort_role", json!("holdout")),
            ("path", "/data/csv_path", json!("C:/data/MSFT.csv")),
            (
                "fingerprint",
                "/dataset_meta/source_content_fingerprint",
                json!("bars-msft"),
            ),
            (
                "candidate manifest",
                "/validation/req/candidate_manifest_hash",
                json!("sha256:older-candidates"),
            ),
        ] {
            let mut tampered = artifact.clone();
            *tampered.pointer_mut(pointer).expect("fixture pointer") = replacement;
            assert_eq!(
                super::campaign_cohort_role_for_artifact(
                    &expectation,
                    artifact_path.as_path(),
                    &tampered,
                ),
                None,
                "{label} contradiction must reject the artifact"
            );
        }

        let mut missing_candidate_manifest = artifact.clone();
        missing_candidate_manifest
            .pointer_mut("/validation/req")
            .and_then(Value::as_object_mut)
            .expect("validation request")
            .remove("candidate_manifest_hash");
        assert_eq!(
            super::campaign_cohort_role_for_artifact(
                &expectation,
                artifact_path.as_path(),
                &missing_candidate_manifest,
            ),
            None,
            "current campaign artifacts must carry the expected candidate manifest hash"
        );

        let mut legacy_exact_path = artifact.clone();
        legacy_exact_path
            .as_object_mut()
            .expect("artifact object")
            .remove("campaign");
        assert_eq!(
            super::campaign_cohort_role_for_artifact(
                &expectation,
                artifact_path.as_path(),
                &legacy_exact_path,
            ),
            Some(super::MetaCampaignCohortRole::Train),
            "the exact summary-referenced artifact path is a valid lineage fallback"
        );
        assert_eq!(
            super::campaign_cohort_role_for_artifact(
                &expectation,
                PathBuf::from("C:/runs/vectorgrid_run_other.json").as_path(),
                &legacy_exact_path,
            ),
            None,
            "an unmarked artifact from another path must not enter the bound cohort"
        );
    }

    #[test]
    fn campaign_artifact_raw_hash_rejects_psr_dsr_verdict_and_cscv_tampering() {
        let artifact_path = PathBuf::from("C:/runs/vectorgrid_run_sealed.json");
        let mut expectation = test_campaign_cohort_expectation(&artifact_path);
        let artifact = json!({
            "validation": {
                "res": {
                    "run_verdict": { "passed": true },
                    "cscv_pbo": 0.12,
                    "candidates": [{
                        "psr": 0.71,
                        "dsr": 0.64
                    }]
                }
            }
        });
        let raw_sha256 = |value: &Value| {
            let bytes = serde_json::to_vec_pretty(value).expect("serialize raw artifact fixture");
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            format!("{:x}", hasher.finalize())
        };
        let baseline_sha256 = raw_sha256(&artifact);
        expectation
            .artifact_paths
            .get_mut(&super::cohort_artifact_path_key(&artifact_path))
            .expect("expected path identity")
            .artifact_sha256 = Some(baseline_sha256.to_ascii_uppercase());
        super::validate_campaign_artifact_raw_sha256(
            &expectation,
            &artifact_path,
            &baseline_sha256,
        )
        .expect("an exact raw hash should pass case-insensitively");

        for (label, pointer, replacement) in [
            ("PSR", "/validation/res/candidates/0/psr", json!(0.99)),
            ("DSR", "/validation/res/candidates/0/dsr", json!(0.98)),
            (
                "run verdict",
                "/validation/res/run_verdict/passed",
                json!(false),
            ),
            ("CSCV PBO", "/validation/res/cscv_pbo", json!(0.91)),
        ] {
            let mut tampered = artifact.clone();
            *tampered.pointer_mut(pointer).expect("tamper pointer") = replacement;
            let tampered_sha256 = raw_sha256(&tampered);
            assert_ne!(
                baseline_sha256, tampered_sha256,
                "{label} must alter raw SHA-256"
            );
            assert!(
                super::validate_campaign_artifact_raw_sha256(
                    &expectation,
                    &artifact_path,
                    &tampered_sha256,
                )
                .expect_err("tampered raw evidence must fail before Meta ranking")
                .contains("changed after the terminal campaign summary was sealed"),
                "{label} tampering must fail the sealed artifact contract"
            );
        }
    }

    #[test]
    fn artifact_tag_matching_is_case_insensitive() {
        let artifact = json!({ "tags": ["Campaign-Main", "HOLDOUT"] });
        assert!(super::artifact_has_tag(&artifact, "campaign-main"));
        assert!(super::artifact_has_tag(&artifact, "holdout"));
        assert!(!super::artifact_has_tag(&artifact, "different"));
    }

    #[test]
    fn missing_campaign_summary_fails_before_meta_artifact_discovery() {
        let app = build_mock_app();
        let runs = super::run_artifacts::runs_dir(app.handle()).expect("runs dir");
        let missing = runs.join("vectorgrid_campaign_missing.json");

        let error = super::load_meta_campaign_cohort_expectation(
            Some(missing.to_string_lossy().as_ref()),
            &test_meta_req(),
            app.handle(),
        )
        .expect_err("missing campaign summary must fail closed");

        assert!(error.contains("does not exist"), "{error}");
        assert!(!super::reports_dir(app.handle())
            .expect("reports dir")
            .join("vectorgrid_meta.xlsx")
            .exists());
    }

    #[test]
    fn missing_summary_referenced_run_artifact_fails_closed() {
        let app = build_mock_app();
        let runs = super::run_artifacts::runs_dir(app.handle()).expect("runs dir");
        let missing = runs.join("vectorgrid_run_missing.json");

        let error = super::load_campaign_referenced_artifact(&missing, app.handle())
            .expect_err("missing referenced artifact must fail closed");

        assert!(error.contains("could not load referenced run artifact"));
        assert!(error.contains("does not exist"), "{error}");
    }

    #[test]
    fn requested_dataset_denominator_preserves_missing_campaign_members() {
        assert_eq!(super::requested_dataset_denominator(15, Some(100)), 100);
        assert_eq!(super::requested_dataset_denominator(120, Some(100)), 120);
        assert_eq!(super::requested_dataset_denominator(15, None), 15);
    }

    #[test]
    fn meta_holdout_evaluation_rejects_overlapping_dataset_keys() {
        let train = vec![(
            PathBuf::from("runs/train.json"),
            json!({
                "data": { "source_content_fingerprint": "same-content" },
                "dataset_meta": { "timeframe": "1m" }
            }),
        )];
        let holdout = vec![(
            PathBuf::from("runs/holdout.json"),
            json!({
                "data": { "source_content_fingerprint": "same-content" },
                "dataset_meta": { "timeframe": "1m" }
            }),
        )];
        let manifest = build_meta_cohort_manifest(&train, &holdout, Some("holdout"), None, None);

        let error = require_meta_holdout_disjointness(true, holdout.len(), &manifest)
            .expect_err("overlapping holdout datasets must fail closed");

        assert!(error.contains("not sealed"), "{error}");
        assert!(error.contains("share 1 dataset key"), "{error}");
    }

    #[test]
    fn meta_holdout_evaluation_rejects_unverifiable_dataset_identity() {
        let train = vec![(
            PathBuf::from("runs/train.json"),
            json!({"kind": "vectorgrid_report"}),
        )];
        let holdout = vec![(
            PathBuf::from("runs/holdout.json"),
            json!({
                "data": { "source_content_fingerprint": "holdout-content" }
            }),
        )];
        let manifest = build_meta_cohort_manifest(&train, &holdout, Some("holdout"), None, None);

        let error = require_meta_holdout_disjointness(true, holdout.len(), &manifest)
            .expect_err("unverifiable holdout disjointness must fail closed");

        assert!(error.contains("cannot be proven"), "{error}");
        assert!(error.contains("1 artifact"), "{error}");
    }

    #[test]
    fn meta_holdout_disjointness_gate_is_inactive_without_holdout_evaluation() {
        let train = vec![(
            PathBuf::from("runs/train.json"),
            json!({"kind": "vectorgrid_report"}),
        )];
        let holdout = vec![(
            PathBuf::from("runs/holdout.json"),
            json!({"kind": "vectorgrid_report"}),
        )];
        let manifest = build_meta_cohort_manifest(&train, &holdout, Some("holdout"), None, None);

        require_meta_holdout_disjointness(false, holdout.len(), &manifest)
            .expect("ordinary meta selection should retain legacy compatibility");
        require_meta_holdout_disjointness(true, 0, &manifest)
            .expect("an empty holdout cohort should remain a handled no-evidence case");
    }

    #[test]
    fn vcet_requested_holdout_tag_fails_closed_when_no_artifacts_match() {
        let error =
            require_requested_vcet_holdout_artifacts("Vcet", true, Some("campaign-holdout"), 0)
                .expect_err("VCET must not save a no-evidence holdout report");
        assert!(error.contains("campaign-holdout"), "{error}");
        assert!(
            error.contains("zero matching holdout validation artifacts"),
            "{error}"
        );
        assert!(
            error.contains("strategy, tag, and timeframe filters"),
            "{error}"
        );

        require_requested_vcet_holdout_artifacts("CompositeMo", true, Some("campaign-holdout"), 0)
            .expect("non-VCET behavior remains compatible");
        require_requested_vcet_holdout_artifacts("Vcet", true, Some("campaign-holdout"), 1)
            .expect("matched VCET holdout evidence is accepted");
    }

    #[test]
    fn group_holdout_label_skips_groups_not_evaluated_by_partial_holdout() {
        let label_cfg = MlConfidenceLabelConfig::default();
        let group = json!({
            "holdout_coverage": {
                "evaluated": false,
                "missing_reason": "not_selected_for_partial_holdout"
            },
            "holdout": {
                "datasets": 999,
                "pass_rate": 1.0,
                "score_p10": 1.0,
                "max_dd_p90": 0.0
            }
        });

        assert_eq!(group_holdout_label(&group, &label_cfg), None);
    }

    #[test]
    fn stage4_health_gate_blocks_only_explicit_unhealthy_models() {
        assert!(should_block_stage4_for_unhealthy_model(true, Some(false)));
        assert!(!should_block_stage4_for_unhealthy_model(true, Some(true)));
        assert!(!should_block_stage4_for_unhealthy_model(true, None));
        assert!(!should_block_stage4_for_unhealthy_model(false, Some(false)));
    }

    #[test]
    fn stage4_autotrain_failure_falls_back_to_existing_model() {
        let mut warnings: Vec<String> = Vec::new();
        let model_path = PathBuf::from("models/confidence/ml_confidence_test_1m.json");
        let out = stage4_autotrain_failure_fallback(
            "simulated training error".to_string(),
            &model_path,
            true,
            &mut warnings,
        );

        assert!(out.is_ok());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("falling back to existing model"));
    }

    #[test]
    fn stage4_autotrain_failure_errors_without_existing_model() {
        let mut warnings: Vec<String> = Vec::new();
        let model_path = PathBuf::from("models/confidence/ml_confidence_test_1m.json");
        let err = stage4_autotrain_failure_fallback(
            "simulated training error".to_string(),
            &model_path,
            false,
            &mut warnings,
        )
        .unwrap_err();

        assert!(warnings.is_empty());
        assert!(err.contains("no usable existing model"));
    }

    #[test]
    fn holdout_support_includes_asr_strategy() {
        assert!(strategy_supports_holdout("Asr"));
        assert!(strategy_supports_holdout("Vcc"));
        assert!(strategy_supports_holdout("Vemr"));
        assert!(strategy_supports_holdout("Vcet"));
        assert!(strategy_supports_holdout("DoubleMa"));
        assert!(strategy_supports_holdout("Pit"));
        assert!(strategy_supports_holdout("Set"));
        assert!(strategy_supports_holdout("Mts"));
        assert!(strategy_supports_holdout("Orc"));
        assert!(strategy_supports_holdout("Marrt"));
        assert!(strategy_supports_holdout("Vfct"));
        assert!(strategy_supports_holdout("Rocb"));
        assert!(!strategy_supports_holdout("Unknown"));
    }

    #[test]
    fn asr_routes_to_composite_validation_path() {
        assert!(!strategy_uses_double_ma_validation("Asr"));
        assert!(strategy_uses_double_ma_validation("DoubleMa"));
        assert!(strategy_uses_double_ma_validation("Pit"));
        assert!(strategy_uses_double_ma_validation("Set"));
        assert!(strategy_uses_double_ma_validation("Mts"));
        assert!(strategy_uses_double_ma_validation("Orc"));
        assert!(strategy_uses_double_ma_validation("Marrt"));
        assert!(strategy_uses_double_ma_validation("Vfct"));
        assert!(!strategy_uses_double_ma_validation("Rocb"));
    }

    #[test]
    fn known_non_run_artifact_filename_filters_meta_and_eval_outputs() {
        assert!(is_known_non_run_artifact_filename(
            "vectorgrid_meta_stage5_20260401.json"
        ));
        assert!(is_known_non_run_artifact_filename(
            "vectorgrid_ml_confidence_eval_20260401.json"
        ));
        assert!(!is_known_non_run_artifact_filename(
            "vectorgrid_run_artifact_20260401.json"
        ));
    }

    #[test]
    fn relaxed_meta_request_match_defers_timeframe_filter_until_after_inference() {
        let req = test_meta_req();
        let artifact = json!({
            "kind": "vectorgrid_run_artifact",
            "tags": ["bench"],
            "dataset_meta": { "timeframe": "5m" },
            "run": { "req": { "strategy_kind": "CompositeMo" } },
            "validation": { "res": { "candidates": [{}] } }
        });

        assert!(artifact_matches_meta_req_relaxed(&artifact, &req));
        assert!(!artifact_matches_meta_req(&artifact, &req));
    }

    #[test]
    fn metric_surface_meta_warnings_flag_mixed_train_artifacts_and_missing_metadata() {
        let train_legacy = json!({
            "run": {
                "req": { "strategy_kind": "CompositeMo" },
                "res": {
                    "best_metrics": {},
                    "top": []
                }
            }
        });
        let mut train_core17 = json!({
            "run": {
                "req": { "strategy_kind": "CompositeTrend" },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "route_diagnostics": ["gpu_kernel.metrics_count=17"],
                    "best_metrics": {
                        "pnl": 1.0,
                        "sharpe": 0.5,
                        "sortino": 0.6,
                        "downside_deviation": 0.2,
                        "downside_frequency": 0.1,
                        "downside_severity": 0.05,
                        "calmar": 0.9,
                        "omega": 1.2,
                        "kappa_tau": 0.4,
                        "volatility": 0.3,
                        "ulcer_index": 0.11,
                        "martin_ratio": 0.7,
                        "max_dd": 0.2,
                        "trades": 10,
                        "exposure": 0.5,
                        "net_exposure": 0.1,
                        "long_mean_return": 0.02,
                        "short_mean_return": -0.01,
                        "closed_trades": 8,
                        "winning_trades": 5,
                        "losing_trades": 3,
                        "gross_profit": 2.0,
                        "gross_loss": 1.0,
                        "underwater_fraction": 0.15,
                        "max_underwater_duration_bars": 12
                    },
                    "top": []
                }
            }
        });
        let train_core17_surface = build_optimization_metric_surface_artifact_value(&train_core17)
            .expect("metric surface");
        if let Some(root) = train_core17.as_object_mut() {
            root.insert(
                "optimization_metric_surface".to_string(),
                train_core17_surface,
            );
        }

        let warnings = build_metric_surface_meta_warnings(&[train_legacy, train_core17], &[]);
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("mixing optimization metric surfaces"));
        assert!(warnings[0].contains("legacy=1, core17=1, trade22=0"));
        assert!(warnings[1].contains("missing explicit metadata"));
        assert!(warnings[1].contains("1 training artifact"));
    }

    #[test]
    fn metric_surface_meta_warnings_flag_holdout_mix_difference() {
        let mut train = json!({
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "include_trade_metrics": true
                },
                "res": {
                    "metric_surface_trade_tail_exact": true,
                    "best_metrics": {
                        "pnl": 1.0,
                        "sharpe": 0.5,
                        "sortino": 0.6,
                        "downside_deviation": 0.2,
                        "downside_frequency": 0.1,
                        "downside_severity": 0.05,
                        "calmar": 0.9,
                        "omega": 1.2,
                        "kappa_tau": 0.4,
                        "volatility": 0.3,
                        "ulcer_index": 0.11,
                        "martin_ratio": 0.7,
                        "max_dd": 0.2,
                        "trades": 10,
                        "exposure": 0.5,
                        "net_exposure": 0.1,
                        "long_mean_return": 0.02,
                        "short_mean_return": -0.01,
                        "closed_trades": 8,
                        "winning_trades": 5,
                        "losing_trades": 3,
                        "gross_profit": 2.0,
                        "gross_loss": 1.0,
                        "underwater_fraction": 0.15,
                        "max_underwater_duration_bars": 12
                    },
                    "top": []
                }
            }
        });
        let mut holdout = json!({
            "run": {
                "req": { "strategy_kind": "CompositeMo" },
                "res": {
                    "best_metrics": {},
                    "top": []
                }
            }
        });
        let train_surface =
            build_optimization_metric_surface_artifact_value(&train).expect("train surface");
        if let Some(root) = train.as_object_mut() {
            root.insert("optimization_metric_surface".to_string(), train_surface);
        }
        let holdout_surface =
            build_optimization_metric_surface_artifact_value(&holdout).expect("holdout surface");
        if let Some(root) = holdout.as_object_mut() {
            root.insert("optimization_metric_surface".to_string(), holdout_surface);
        }

        let warnings = build_metric_surface_meta_warnings(&[train], &[holdout]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Training vs holdout metric-surface mix differs"));
        assert!(warnings[0].contains("train: legacy=0, core17=0, trade22=1"));
        assert!(warnings[0].contains("holdout: legacy=1, core17=0, trade22=0"));
    }

    #[test]
    fn metric_surface_meta_summary_persists_train_and_holdout_counts() {
        let train = json!({
            "optimization_metric_surface": {
                "schema_version": 3,
                "surface": "core17"
            },
            "run": {
                "req": { "strategy_kind": "CompositeTrend" },
                "res": {
                    "backend_route": "GpuKernel",
                    "best_metrics": {},
                    "top": []
                }
            }
        });
        let holdout = json!({
            "optimization_metric_surface": {
                "schema_version": 3,
                "surface": "trade22",
                "trade_tail_exact": true
            },
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "include_trade_metrics": true
                },
                "res": {
                    "best_metrics": {},
                    "top": []
                }
            }
        });

        let summary =
            build_metric_surface_meta_summary(&[train], &[holdout]).expect("surface summary");
        assert_eq!(summary.train.core17, 1);
        assert_eq!(summary.train.trade22, 0);
        assert!(!summary.mixed_train_surfaces);
        assert_eq!(summary.holdout.as_ref().map(|value| value.trade22), Some(1));
        assert!(!summary.mixed_holdout_surfaces);
    }

    #[test]
    fn build_ml_confidence_status_prefers_dataset_rows_and_keeps_scan_error() {
        let summary = MlConfidenceModelSummary {
            version: 1,
            scope: MlConfidenceScope {
                strategy_kind: "CompositeMo".to_string(),
                timeframe: "1m".to_string(),
            },
            model_kind: MlConfidenceModelKind::Logistic,
            label: MlConfidenceLabelConfig::default(),
            meta: Some(MlConfidenceModelMeta {
                trained_at_ms: 1,
                trained_rows: 40,
                trained_positives: 20,
                trained_negatives: 20,
                target: MlConfidenceTarget::HoldoutDatasetPass,
                model_kind: MlConfidenceModelKind::Logistic,
                compatibility_key: None,
                meta_artifacts_used: Some(2),
                newest_meta_ts_used: Some("2026-03-01T00:00:00Z".to_string()),
                cv_auc: Some(0.71),
                cv_brier: Some(0.21),
                cv_brier_calibrated: Some(0.19),
                evidence_tier: MlConfidenceEvidenceTier::CrossValidation,
                baseline_brier: 0.25,
                healthy: true,
                warnings: Vec::new(),
            }),
        };
        let meta_summary = MlMetaScopeSummary {
            available_meta_artifacts: 3,
            newest_meta_ts_available: Some("2026-03-02T00:00:00Z".to_string()),
            available_labeled_rows_holdout: 9,
            available_labeled_rows_dataset: 14,
            has_holdout_dataset_rows: true,
            active_compatibility_key: None,
            skipped_incompatible_artifacts: 0,
        };

        let status = build_ml_confidence_status(
            true,
            "models/confidence/ml_confidence_test_1m.json".to_string(),
            1024,
            Some(summary),
            None,
            Some("meta read failed".to_string()),
            &meta_summary,
        );

        assert_eq!(status.available_labeled_rows, 14);
        assert_eq!(status.meta_scan_error.as_deref(), Some("meta read failed"));
        assert!(status.stale);
        assert!(!status.retrain_recommended);
        assert!(status.model_summary.is_some());
        assert_eq!(status.model_healthy, Some(true));
        assert_eq!(status.state, MlConfidenceModelState::Warning);
    }

    #[test]
    fn build_ml_confidence_status_marks_compatibility_mismatch_as_stale() {
        let summary = MlConfidenceModelSummary {
            version: 1,
            scope: MlConfidenceScope {
                strategy_kind: "CompositeMo".to_string(),
                timeframe: "1m".to_string(),
            },
            model_kind: MlConfidenceModelKind::Logistic,
            label: MlConfidenceLabelConfig::default(),
            meta: Some(MlConfidenceModelMeta {
                trained_at_ms: 1,
                trained_rows: 40,
                trained_positives: 20,
                trained_negatives: 20,
                target: MlConfidenceTarget::HoldoutDatasetPass,
                model_kind: MlConfidenceModelKind::Logistic,
                compatibility_key: Some(MlConfidenceCompatibilityKey {
                    target_family: "holdout_gate_only".to_string(),
                    train_score_semantics: "meta_score_v1_full".to_string(),
                    grouping_mode: "exact".to_string(),
                    shortlist_policy: "strict_no_fallback".to_string(),
                }),
                meta_artifacts_used: Some(2),
                newest_meta_ts_used: Some("2026-03-01T00:00:00Z".to_string()),
                cv_auc: Some(0.71),
                cv_brier: Some(0.21),
                cv_brier_calibrated: Some(0.19),
                evidence_tier: MlConfidenceEvidenceTier::CrossValidation,
                baseline_brier: 0.25,
                healthy: true,
                warnings: Vec::new(),
            }),
        };
        let meta_summary = MlMetaScopeSummary {
            available_meta_artifacts: 3,
            newest_meta_ts_available: Some("2026-03-02T00:00:00Z".to_string()),
            available_labeled_rows_holdout: 9,
            available_labeled_rows_dataset: 14,
            has_holdout_dataset_rows: true,
            active_compatibility_key: Some(MlConfidenceCompatibilityKey {
                target_family: "holdout_dataset_pass_available".to_string(),
                train_score_semantics: "meta_score_v1_full".to_string(),
                grouping_mode: "clustered".to_string(),
                shortlist_policy: "legacy_unknown".to_string(),
            }),
            skipped_incompatible_artifacts: 1,
        };

        let status = build_ml_confidence_status(
            true,
            "models/confidence/ml_confidence_test_1m.json".to_string(),
            1024,
            Some(summary),
            None,
            None,
            &meta_summary,
        );

        assert_eq!(status.state, MlConfidenceModelState::Warning);
        assert!(status.stale);
        assert!(status.retrain_recommended);
        assert!(status
            .retrain_reason
            .as_deref()
            .unwrap_or("")
            .contains("Model compatibility changed"));
    }

    #[test]
    fn policy_eval_meta_score_ignores_holdout_adjusted_final_score() {
        let group = json!({
            "meta_score": 0.35,
            "final_score": 99.0
        });

        assert_eq!(group_policy_eval_meta_score(&group), 0.35);
        assert_eq!(group_rank_score(&group), 99.0);
    }

    #[test]
    fn meta_only_policy_ranking_does_not_use_ml_confidence_as_tie_break() {
        let req = MlConfidencePolicyEvaluationRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            top_k: 2,
            min_ml_confidence: 0.0,
            meta_weight: 1.0,
            ml_weight: 0.0,
            uncertainty_weight: 0.0,
            require_healthy_model: false,
            fallback_to_meta_if_empty: true,
            label: None,
        };
        let candidates = vec![
            super::PolicyEvalCandidate {
                label: 1.0,
                meta_score: 0.5,
                ml_confidence: 0.1,
                ml_logit: stable_logit(0.1),
                ml_uncertainty: ml_uncertainty_from_prob(0.1),
            },
            super::PolicyEvalCandidate {
                label: 2.0,
                meta_score: 0.5,
                ml_confidence: 0.9,
                ml_logit: stable_logit(0.9),
                ml_uncertainty: ml_uncertainty_from_prob(0.9),
            },
        ];

        let (meta_labels, ml_labels, _) = evaluate_policy_candidates(&req, &candidates);

        assert_eq!(meta_labels, vec![1.0, 2.0]);
        assert_eq!(ml_labels, vec![2.0, 1.0]);
    }

    #[test]
    fn read_only_model_path_does_not_create_parent_directory() {
        let app = build_mock_app();
        let scope = MlConfidenceScope {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: "1m".to_string(),
        };
        let model_path =
            ml_confidence_model_path_read_only(&scope, app.handle()).expect("model path");
        let parent = model_path.parent().expect("parent").to_path_buf();
        let _ = fs::remove_dir_all(&parent);

        let model_path =
            ml_confidence_model_path_read_only(&scope, app.handle()).expect("model path");

        assert_eq!(model_path.parent(), Some(parent.as_path()));
        assert!(!parent.exists());
    }

    #[test]
    fn atomic_model_write_replaces_existing_file_contents() {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "commands_meta_atomic_write_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));

        fs::write(&path, b"{\"old\":true}").expect("seed file");
        write_bytes_atomically(&path, b"{\"new\":true}").expect("atomic write");

        assert_eq!(
            fs::read_to_string(&path).expect("read file"),
            "{\"new\":true}"
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn suggest_meta_report_path_errors_after_suffix_space_is_exhausted() {
        let app = build_mock_app();
        let reports = super::reports_dir(app.handle()).expect("reports dir");
        let ts = "2026-04-16T00:00:00Z";
        let safe_ts = super::sanitize_filename_component(ts);
        let base_name = format!("vectorgrid_meta_{safe_ts}");
        fs::write(reports.join(format!("{base_name}.xlsx")), b"base").expect("base report");
        for i in 2..=999u32 {
            fs::write(reports.join(format!("{base_name}_{i}.xlsx")), b"suffix")
                .expect("suffix report");
        }

        let err = suggest_meta_report_path(ts, app.handle(), "xlsx").unwrap_err();
        assert!(err.contains("failed to allocate a unique meta report path"));
    }

    #[test]
    fn dense_selected_group_holdout_evaluation_matches_full_outputs_for_selected_groups() {
        let req = MetaSelectionRequest {
            strategy_kind: "DoubleMa".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: Some("holdout".to_string()),
            holdout_tag: Some("holdout".to_string()),
            eval_holdout: true,
            save_holdout_rows: true,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 25,
            min_samples: 1,
            strict_shortlist: false,
            allow_shortlist_fallback: true,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 1,
        };
        let base_groups = vec![
            make_meta_candidate(&req, 5, 20, 0.22),
            make_meta_candidate(&req, 8, 32, 0.18),
            make_meta_candidate(&req, 13, 55, 0.12),
        ];
        let selected_keys = vec![base_groups[0].key.clone(), base_groups[2].key.clone()];
        let mut full_result = MetaSelectionResult {
            runtime_ms: 0,
            strategy_kind: "DoubleMa".to_string(),
            timeframe: Some("1m".to_string()),
            artifacts_considered: 2,
            artifacts_used: 2,
            candidates_considered: 3,
            groups: base_groups.clone(),
            warnings: Vec::new(),
            holdout: Some(HoldoutSummary {
                datasets: 0,
                skipped: 0,
            }),
            holdout_coverage: None,
        };
        let mut subset_result = MetaSelectionResult {
            groups: base_groups,
            ..full_result.clone()
        };
        let mut full_accs = vec![
            HoldoutAcc {
                scores: vec![1.10, 1.20],
                sharpes: vec![1.00, 1.10],
                max_dds: vec![0.10, 0.12],
                pnls: vec![2.0, 2.2],
                passes: 2,
                profitable: 2,
            },
            HoldoutAcc {
                scores: vec![0.70, 0.75],
                sharpes: vec![0.65, 0.70],
                max_dds: vec![0.20, 0.24],
                pnls: vec![1.0, 1.1],
                passes: 2,
                profitable: 2,
            },
            HoldoutAcc {
                scores: vec![0.95, 1.05],
                sharpes: vec![0.90, 1.00],
                max_dds: vec![0.14, 0.16],
                pnls: vec![1.7, 1.9],
                passes: 2,
                profitable: 2,
            },
        ];
        let mut subset_accs = vec![full_accs[0].clone(), full_accs[2].clone()];
        let mut full_rows = vec![
            vec![ta_app_core::HoldoutDatasetRow {
                dataset: "A".to_string(),
                dataset_key: "source-a".to_string(),
                score_median: 1.15,
                score_p10: 1.10,
                sharpe_median: 1.05,
                max_dd_worst: 0.12,
                pnl_median: 2.1,
                pass: true,
            }],
            vec![ta_app_core::HoldoutDatasetRow {
                dataset: "A".to_string(),
                dataset_key: "source-a".to_string(),
                score_median: 0.72,
                score_p10: 0.70,
                sharpe_median: 0.68,
                max_dd_worst: 0.24,
                pnl_median: 1.05,
                pass: true,
            }],
            vec![ta_app_core::HoldoutDatasetRow {
                dataset: "A".to_string(),
                dataset_key: "source-a".to_string(),
                score_median: 1.00,
                score_p10: 0.95,
                sharpe_median: 0.95,
                max_dd_worst: 0.16,
                pnl_median: 1.8,
                pass: true,
            }],
        ];
        let mut subset_rows = vec![full_rows[0].clone(), full_rows[2].clone()];

        apply_holdout_accumulators_to_result(
            &mut full_result,
            &req,
            &[0, 1, 2],
            &mut full_accs,
            &mut full_rows,
            2,
            2,
            0,
            "holdout",
        );
        apply_holdout_accumulators_to_result(
            &mut subset_result,
            &req,
            &[0, 2],
            &mut subset_accs,
            &mut subset_rows,
            2,
            2,
            0,
            "holdout",
        );

        assert_eq!(
            serde_json::to_value(&full_result.holdout).unwrap(),
            serde_json::to_value(&subset_result.holdout).unwrap()
        );
        for key in selected_keys {
            let full_group = full_result
                .groups
                .iter()
                .find(|group| group.key == key)
                .expect("full selected group");
            let subset_group = subset_result
                .groups
                .iter()
                .find(|group| group.key == key)
                .expect("subset selected group");
            assert_eq!(
                serde_json::to_value(full_group).unwrap(),
                serde_json::to_value(subset_group).unwrap()
            );
        }
        let unselected = subset_result
            .groups
            .iter()
            .find(|group| group.key == make_meta_candidate(&req, 8, 32, 0.18).key)
            .expect("unselected group");
        assert!(unselected.holdout.is_none());
        assert!(unselected.holdout_rows.is_empty());
    }

    #[test]
    fn legacy_double_ma_representative_decode_preserves_fallback_fields() {
        let decoded = decode_double_ma_validation_params(&json!({
            "fast_ma_type": "ema",
            "slow_ma_type": "hma",
            "fast_len": 7,
            "slow_len": 29,
            "fast_ma_params": { "alpha": 0.5 },
            "slow_ma_params": { "phase": 0.25 },
            "strategy": "legacy_shape_that_forces_fallback",
            "halftrend": {
                "amplitude": 3,
                "channel_deviation": 2.0,
                "atr_period": 55
            }
        }))
        .expect("legacy decode");

        assert_eq!(decoded.fast_ma_type, "ema");
        assert_eq!(decoded.slow_ma_type, "hma");
        assert_eq!(decoded.fast_len, 7);
        assert_eq!(decoded.slow_len, 29);
        assert_eq!(
            decoded
                .fast_ma_params
                .as_ref()
                .and_then(|map| map.get("alpha"))
                .copied(),
            Some(0.5)
        );
        assert_eq!(
            decoded
                .slow_ma_params
                .as_ref()
                .and_then(|map| map.get("phase"))
                .copied(),
            Some(0.25)
        );
        assert!(decoded.halftrend_filter.is_some());
    }

    #[test]
    fn prepared_candidate_targets_follow_params_after_result_reordering() {
        let first = decode_double_ma_validation_params(&json!({
            "fast_ma_type": "ema",
            "slow_ma_type": "ema",
            "fast_len": 5,
            "slow_len": 20
        }))
        .expect("first params");
        let second = decode_double_ma_validation_params(&json!({
            "fast_ma_type": "ema",
            "slow_ma_type": "ema",
            "fast_len": 13,
            "slow_len": 55
        }))
        .expect("second params");
        let candidate_idx_by_key = HashMap::from([
            (canonical_typed_param_key(&first).unwrap(), 0usize),
            (canonical_typed_param_key(&second).unwrap(), 1usize),
        ]);
        let candidate_targets = vec![vec![10usize], vec![20usize]];

        let (_, second_targets) = prepared_candidate_targets_for_params(
            &second,
            &candidate_idx_by_key,
            &candidate_targets,
        )
        .expect("second result targets");
        let (_, first_targets) = prepared_candidate_targets_for_params(
            &first,
            &candidate_idx_by_key,
            &candidate_targets,
        )
        .expect("first result targets");

        assert_eq!(second_targets, &[20]);
        assert_eq!(first_targets, &[10]);
    }

    #[test]
    fn prepared_holdout_batches_dedupe_duplicate_representatives_across_groups() {
        let req = MetaSelectionRequest {
            strategy_kind: "DoubleMa".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: Some("holdout".to_string()),
            holdout_tag: Some("holdout".to_string()),
            eval_holdout: true,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 25,
            min_samples: 1,
            strict_shortlist: false,
            allow_shortlist_fallback: true,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 2,
        };
        let shared_rep = json!({
            "fast_ma_type": "sma",
            "slow_ma_type": "sma",
            "fast_len": 5,
            "slow_len": 20,
        });
        let mut group_a = make_meta_candidate(&req, 5, 20, 0.20);
        group_a.representatives = vec![shared_rep.clone()];
        let mut group_b = make_meta_candidate(&req, 8, 32, 0.18);
        let own_rep = group_b.params.clone();
        group_b.representatives = vec![shared_rep.clone(), group_b.params.clone()];
        let result = MetaSelectionResult {
            runtime_ms: 0,
            strategy_kind: "DoubleMa".to_string(),
            timeframe: Some("1m".to_string()),
            artifacts_considered: 1,
            artifacts_used: 1,
            candidates_considered: 2,
            groups: vec![group_a, group_b],
            warnings: Vec::new(),
            holdout: None,
            holdout_coverage: None,
        };
        let active = vec![0usize, 1usize];
        let dense = HashMap::from([(0usize, 0usize), (1usize, 1usize)]);

        let batches = build_prepared_holdout_batches(
            &result,
            &req,
            &active,
            &dense,
            ta_app_core::Backend::CpuOnly,
            1,
            None,
            None,
            None,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
        )
        .expect("prepared batches");

        assert_eq!(batches.len(), 1);
        let PreparedHoldoutBatch::DoubleMa(batch) = &batches[0] else {
            panic!("expected double-ma holdout batch");
        };
        assert_eq!(batch.request.candidates.len(), 2);

        let shared_key =
            canonical_typed_param_key(&batch.request.candidates[0]).expect("shared key");
        let own_key = canonical_typed_param_key(&batch.request.candidates[1]).expect("own key");

        let decoded_shared = decode_double_ma_validation_params(&shared_rep).expect("decoded rep");
        assert_eq!(
            shared_key,
            canonical_typed_param_key(&decoded_shared).unwrap()
        );
        let decoded_own = decode_double_ma_validation_params(&own_rep).expect("decoded own rep");
        assert_eq!(own_key, canonical_typed_param_key(&decoded_own).unwrap());
        assert_eq!(batch.candidate_targets, vec![vec![0, 1], vec![1]]);
    }

    #[test]
    fn prepared_vcet_gpu_holdout_batches_split_effective_execution_configs() {
        let mut req = test_meta_req();
        req.strategy_kind = "Vcet".to_string();

        let candidate = |ce_mult: f64, allow_flip: bool| {
            json!({
                "fast_ma_type": "ema",
                "slow_ma_type": "ema",
                "fast_len": 8,
                "slow_len": 34,
                "halftrend": {
                    "amplitude": 2,
                    "channel_deviation": 2.0,
                    "atr_period": 100
                },
                "vcet": { "ce_mult": ce_mult },
                "strategy": {
                    "long_only": true,
                    "allow_flip": allow_flip,
                    "trade_on_next_bar": true,
                    "commission": 0.0,
                    "eps_rel": 0.0
                }
            })
        };
        let first = candidate(3.0, false);
        let second_vcet = candidate(4.0, false);
        let third_strategy = candidate(3.0, true);
        let mut group = make_meta_candidate(&req, 8, 34, 0.25);
        group.params = first.clone();
        group.representatives = vec![first, second_vcet, third_strategy];
        let result = MetaSelectionResult {
            runtime_ms: 0,
            strategy_kind: "Vcet".to_string(),
            timeframe: Some("1m".to_string()),
            artifacts_considered: 1,
            artifacts_used: 1,
            candidates_considered: 3,
            groups: vec![group],
            warnings: Vec::new(),
            holdout: None,
            holdout_coverage: None,
        };
        let active = vec![0usize];
        let dense = HashMap::from([(0usize, 0usize)]);

        let batches = build_prepared_holdout_batches(
            &result,
            &req,
            &active,
            &dense,
            ta_app_core::Backend::GpuKernel { device_id: 7 },
            1,
            None,
            None,
            None,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
        )
        .expect("prepared VCET GPU batches");

        assert_eq!(batches.len(), 3);
        for batch in batches {
            let PreparedHoldoutBatch::Composite(batch) = batch else {
                panic!("expected composite VCET batch");
            };
            assert!(matches!(
                batch.request.backend,
                ta_app_core::Backend::GpuKernel { device_id: 7 }
            ));
            assert_eq!(batch.request.candidates.len(), 1);
            assert_eq!(batch.candidate_targets, vec![vec![0]]);
        }
    }

    #[test]
    fn holdout_rates_use_requested_unique_dataset_denominator() {
        let mut req = test_meta_req();
        req.save_holdout_rows = false;
        let mut result = MetaSelectionResult {
            runtime_ms: 0,
            strategy_kind: req.strategy_kind.clone(),
            timeframe: req.timeframe.clone(),
            artifacts_considered: 1,
            artifacts_used: 1,
            candidates_considered: 1,
            groups: vec![make_meta_candidate(&req, 8, 32, 0.2)],
            warnings: Vec::new(),
            holdout: None,
            holdout_coverage: None,
        };
        let mut accs = vec![HoldoutAcc {
            scores: vec![0.2, 0.3],
            sharpes: vec![0.8, 0.9],
            max_dds: vec![0.2, 0.3],
            pnls: vec![1.0, 1.5],
            passes: 1,
            profitable: 2,
        }];

        let evidence = build_holdout_group_execution_evidence(&result, &[0], &accs, 3);
        assert_eq!(evidence[0].datasets_requested, 3);
        assert_eq!(evidence[0].datasets_evaluated, 2);
        assert_eq!(evidence[0].datasets_failed, 1);
        assert_eq!(evidence[0].pass_rate, 1.0 / 3.0);
        assert_eq!(evidence[0].profitable_rate, 2.0 / 3.0);
        assert_eq!(evidence[0].evaluation_rate, 2.0 / 3.0);

        apply_holdout_accumulators_to_result(
            &mut result,
            &req,
            &[0],
            &mut accs,
            &mut Vec::new(),
            3,
            2,
            1,
            "holdout",
        );
        let aggregate = result.groups[0]
            .holdout
            .as_ref()
            .expect("holdout aggregate");
        assert_eq!(aggregate.datasets, 2);
        assert_eq!(aggregate.pass_rate, 1.0 / 3.0);
        assert_eq!(result.holdout.as_ref().unwrap().datasets, 2);
        assert_eq!(result.holdout.as_ref().unwrap().skipped, 1);
    }

    #[test]
    fn holdout_summary_scratch_reuses_buffers_without_row_leakage() {
        let mut scratch = HoldoutSummaryScratch::default();
        let first = vec![
            HoldoutMetricSample {
                score: 1.0,
                sharpe: 0.8,
                max_dd: 0.20,
                pnl: 2.0,
            },
            HoldoutMetricSample {
                score: 1.5,
                sharpe: 1.1,
                max_dd: 0.10,
                pnl: 2.4,
            },
        ];
        let second = vec![HoldoutMetricSample {
            score: 0.4,
            sharpe: 0.2,
            max_dd: 0.35,
            pnl: 0.7,
        }];

        let first_stats = scratch
            .summarize_rows("CompositeMo", &first)
            .expect("first stats");
        let second_stats = scratch
            .summarize_rows("CompositeMo", &second)
            .expect("second stats");

        let mut second_scores = vec![0.4];
        let mut second_sharpes = vec![0.2];
        let second_max_dds = vec![0.35];
        let mut second_pnls = vec![0.7];
        let reference = summarize_holdout_metric_columns(
            "CompositeMo",
            second_scores.as_mut_slice(),
            second_sharpes.as_mut_slice(),
            second_max_dds.as_slice(),
            second_pnls.as_mut_slice(),
        )
        .expect("reference stats");

        assert!(first_stats.score_median > second_stats.score_median);
        assert_eq!(second_stats, reference);
    }

    #[test]
    fn non_vcet_negative_pnl_preserves_historic_holdout_pass_and_final_score() {
        let mut req = test_meta_req();
        req.strategy_kind = "CompositeMo".to_string();
        let mut scores = vec![0.5, 0.6];
        let mut sharpes = vec![1.0, 1.1];
        let max_dds = vec![0.1, 0.2];
        let mut pnls = vec![-2.0, -1.0];

        let stats = summarize_holdout_metric_columns(
            &req.strategy_kind,
            &mut scores,
            &mut sharpes,
            &max_dds,
            &mut pnls,
        )
        .expect("holdout stats");

        assert!(!stats.profitable);
        assert!(stats.pass);

        let base_score = 0.2;
        let mut result = MetaSelectionResult {
            runtime_ms: 0,
            strategy_kind: req.strategy_kind.clone(),
            timeframe: req.timeframe.clone(),
            artifacts_considered: 1,
            artifacts_used: 1,
            candidates_considered: 1,
            groups: vec![make_meta_candidate(&req, 8, 32, base_score)],
            warnings: Vec::new(),
            holdout: None,
            holdout_coverage: None,
        };
        let mut accs = vec![HoldoutAcc {
            scores: vec![stats.score_median],
            sharpes: vec![stats.sharpe_median],
            max_dds: vec![stats.max_dd_worst],
            pnls: vec![stats.pnl_median],
            passes: usize::from(stats.pass),
            profitable: usize::from(stats.profitable),
        }];
        let mut rows = Vec::new();
        apply_holdout_accumulators_to_result(
            &mut result,
            &req,
            &[0],
            &mut accs,
            &mut rows,
            1,
            1,
            0,
            "holdout",
        );

        let group = &result.groups[0];
        let holdout = group.holdout.as_ref().expect("holdout aggregate");
        assert_eq!(holdout.pass_rate, 1.0);
        let expected_final_score = base_score
            + req.weights.w_holdout_pass_rate
            + req.weights.w_holdout_p10 * stats.score_median
            - req.weights.w_holdout_dd_p90 * stats.max_dd_worst;
        assert_eq!(group.final_score, Some(expected_final_score));
    }

    #[test]
    fn vcet_holdout_pass_requires_positive_median_pnl() {
        let summarize = |pnls: &mut [f64]| {
            let mut scores = vec![0.5, 0.6];
            let mut sharpes = vec![1.0, 1.1];
            let max_dds = vec![0.1, 0.2];
            summarize_holdout_metric_columns("Vcet", &mut scores, &mut sharpes, &max_dds, pnls)
                .expect("VCET holdout stats")
        };

        let losing = summarize(&mut [-2.0, -1.0]);
        assert!(!losing.profitable);
        assert!(!losing.pass);

        let profitable = summarize(&mut [1.0, 2.0]);
        assert!(profitable.profitable);
        assert!(profitable.pass);
    }

    #[test]
    fn prepared_row_policy_eval_matches_raw_prefix_reference() {
        let metas = vec![
            make_policy_eval_meta_artifact(
                "CompositeMo",
                "2026-03-01T00:00:00Z",
                (0..30usize)
                    .map(|idx| make_policy_eval_group(idx % 2 == 0, idx))
                    .collect(),
            ),
            make_policy_eval_meta_artifact(
                "CompositeMo",
                "2026-03-02T00:00:00Z",
                (0..30usize)
                    .map(|idx| make_policy_eval_group((idx + 1) % 2 == 0, idx))
                    .collect(),
            ),
            make_policy_eval_meta_artifact(
                "CompositeMo",
                "2026-03-03T00:00:00Z",
                (0..30usize)
                    .map(|idx| make_policy_eval_group(idx % 3 != 0, idx))
                    .collect(),
            ),
        ];
        let req = MlConfidencePolicyEvaluationRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            top_k: 8,
            min_ml_confidence: 0.55,
            meta_weight: 0.6,
            ml_weight: 0.35,
            uncertainty_weight: 0.05,
            require_healthy_model: true,
            fallback_to_meta_if_empty: true,
            label: Some(MlConfidenceLabelConfig {
                min_holdout_pass_rate: 0.60,
                min_holdout_score_p10: 0.0,
                max_holdout_max_dd_p90: Some(0.60),
                min_holdout_datasets: 5,
            }),
        };
        let normalized_req =
            normalize_ml_policy_evaluation_request(req.clone()).expect("normalize policy req");
        let scope = normalize_ml_scope(MlConfidenceScope {
            strategy_kind: normalized_req.strategy_kind.clone(),
            timeframe: normalized_req.timeframe.clone().unwrap_or_default(),
        })
        .expect("normalize scope");
        let label_cfg = normalized_req.label.clone().unwrap_or_default();

        let result = evaluate_ml_confidence_policy_from_scoped_metas(
            &normalized_req,
            &scope,
            &label_cfg,
            &metas,
        );
        let expected = run_policy_eval_reference_from_raw(req, &metas);

        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
    }

    #[test]
    fn vcet_zero_usable_holdout_results_fail_before_report_publication() {
        let error = super::require_usable_vcet_holdout_results("Vcet", "campaign-holdout", 25, 0)
            .expect_err("VCET zero-evidence holdout must fail closed");
        assert!(error.contains("zero usable dataset results"), "{error}");
        assert!(
            error.contains("No Meta XLSX or JSON was published"),
            "{error}"
        );

        super::require_usable_vcet_holdout_results("Vcet", "campaign-holdout", 25, 1)
            .expect("one usable dataset permits a denominator-aware report");
        super::require_usable_vcet_holdout_results("CompositeMo", "holdout", 25, 0)
            .expect("legacy non-VCET behavior remains unchanged");
    }
}
