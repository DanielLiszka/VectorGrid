use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ta_app_core::vcet_search::{VcetCandidateIdentityInputs, MAX_VCET_SEARCH_BUDGET};
use ta_app_core::vcet_temporal::{
    plan_vcet_temporal_folds, validate_vcet_temporal_cross_dataset_alignment,
    vcet_temporal_plan_identity, VcetTemporalLayoutV1, VcetTemporalPlanIdentityV1,
    VcetTemporalSessionSeriesRef, VcetTemporalValidationPlanV1,
};

pub const VCET_TEMPORAL_PLANNING_EVIDENCE_SCHEMA_VERSION: u16 = 1;
pub const VCET_TEMPORAL_CANDIDATE_MANIFEST_SCHEMA_VERSION: u16 = 1;
pub const MAX_VCET_TEMPORAL_DATASETS_PER_REQUEST: usize = 10_000;
pub const MAX_VCET_TEMPORAL_TIMELINE_VALUES_PER_REQUEST: usize = 10_000_000;

const CANDIDATE_MANIFEST_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_candidate_manifest.v1";
const PLANNING_EVIDENCE_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_planning_evidence.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalCandidateEntryV1 {
    pub ordinal: usize,
    pub candidate_id: String,
    pub identity_inputs: VcetCandidateIdentityInputs,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalCandidateManifestPayloadV1 {
    pub schema_version: u16,
    pub strategy_kind: String,
    pub source_campaign_manifest_hash: String,
    pub source_candidate_manifest_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_search_id: Option<String>,
    pub candidates: Vec<VcetTemporalCandidateEntryV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalSealedCandidateManifestV1 {
    pub payload: VcetTemporalCandidateManifestPayloadV1,
    pub manifest_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalDatasetTimelineInputV1 {
    pub dataset_id: String,
    pub source_dataset_sha256: String,
    pub timestamps_utc_ms: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_calendar_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_open_utc_ms: Option<Vec<i64>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanVcetTemporalValidationRequestV1 {
    pub sealed_candidate_manifest: VcetTemporalSealedCandidateManifestV1,
    pub expected_candidate_manifest_sha256: String,
    pub plan: VcetTemporalValidationPlanV1,
    pub expected_plan_identity: VcetTemporalPlanIdentityV1,
    pub datasets: Vec<VcetTemporalDatasetTimelineInputV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VcetTemporalPlanningStatusV1 {
    PlannedNotExecuted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VcetTemporalCoverageStatusV1 {
    Covered,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VcetTemporalAlignmentStatusV1 {
    ExactAcrossDatasets,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalCandidateManifestEvidenceV1 {
    pub manifest_sha256: String,
    pub source_campaign_manifest_hash: String,
    pub source_candidate_manifest_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_search_id: Option<String>,
    pub candidate_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalDatasetPlanningEvidenceV1 {
    pub dataset_id: String,
    pub source_dataset_sha256: String,
    pub timeline_sha256: String,
    pub coverage_status: VcetTemporalCoverageStatusV1,
    pub observed_bar_count: usize,
    pub selected_bar_count: usize,
    pub fold_count: usize,
    pub layout: VcetTemporalLayoutV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanVcetTemporalValidationResponseV1 {
    pub schema_version: u16,
    pub evidence_kind: String,
    pub planning_status: VcetTemporalPlanningStatusV1,
    pub evaluation_executed: bool,
    pub evaluation_result_count: usize,
    pub candidate_manifest: VcetTemporalCandidateManifestEvidenceV1,
    pub plan_identity: VcetTemporalPlanIdentityV1,
    pub dataset_count: usize,
    pub alignment_status: VcetTemporalAlignmentStatusV1,
    pub alignment_reference_dataset_id: String,
    pub datasets: Vec<VcetTemporalDatasetPlanningEvidenceV1>,
    pub limitations: Vec<String>,
    pub next_required_stage: String,
    pub evidence_sha256: String,
}

#[derive(Serialize)]
struct CandidateManifestHashInput<'a> {
    hash_contract: &'static str,
    payload: &'a VcetTemporalCandidateManifestPayloadV1,
}

#[derive(Serialize)]
struct PlanningEvidenceHashInput<'a> {
    hash_contract: &'static str,
    schema_version: u16,
    evidence_kind: &'a str,
    planning_status: VcetTemporalPlanningStatusV1,
    evaluation_executed: bool,
    evaluation_result_count: usize,
    candidate_manifest: &'a VcetTemporalCandidateManifestEvidenceV1,
    plan_identity: &'a VcetTemporalPlanIdentityV1,
    dataset_count: usize,
    alignment_status: VcetTemporalAlignmentStatusV1,
    alignment_reference_dataset_id: &'a str,
    datasets: &'a [VcetTemporalDatasetPlanningEvidenceV1],
    limitations: &'a [String],
    next_required_stage: &'a str,
}

fn sha256_prefixed(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(7 + digest.len() * 2);
    out.push_str("sha256:");
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn sha256_serializable(value: &impl Serialize, label: &str) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("failed to serialize {label} for SHA-256: {error}"))?;
    Ok(sha256_prefixed(&bytes))
}

fn normalize_sha256(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("sha256:").unwrap_or(trimmed);
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{field} must be a SHA-256 value containing exactly 64 hexadecimal digits"
        ));
    }
    Ok(format!("sha256:{}", hex.to_ascii_lowercase()))
}

fn validate_candidate_identity(identity: &VcetCandidateIdentityInputs) -> Result<(), String> {
    if identity.vidya_short_period < 2 {
        return Err("VCET temporal candidate vidya_short_period must be >= 2".to_string());
    }
    if identity.vidya_long_period <= identity.vidya_short_period {
        return Err(
            "VCET temporal candidate vidya_long_period must be > vidya_short_period".to_string(),
        );
    }
    if !(0..=1_000_000).contains(&identity.vidya_alpha_micros) {
        return Err(
            "VCET temporal candidate vidya_alpha_micros must stay within [0, 1000000]".to_string(),
        );
    }
    if identity.ce_period == 0 {
        return Err("VCET temporal candidate ce_period must be > 0".to_string());
    }
    if identity.ce_mult_micros <= 0 {
        return Err("VCET temporal candidate ce_mult_micros must be > 0".to_string());
    }
    if identity.max_trade_bars == 0 {
        return Err("VCET temporal candidate max_trade_bars must be > 0".to_string());
    }
    Ok(())
}

fn candidate_manifest_sha256(
    payload: &VcetTemporalCandidateManifestPayloadV1,
) -> Result<String, String> {
    sha256_serializable(
        &CandidateManifestHashInput {
            hash_contract: CANDIDATE_MANIFEST_HASH_CONTRACT,
            payload,
        },
        "VCET temporal candidate manifest",
    )
}

/// Build a canonical candidate manifest for persistence before invoking the planning command.
/// Existing campaign and candidate hashes remain lineage inputs; this helper does not replace or
/// mutate those campaign artifacts.
pub fn seal_vcet_temporal_candidate_manifest(
    source_campaign_manifest_hash: impl Into<String>,
    source_candidate_manifest_hash: impl Into<String>,
    source_search_id: Option<String>,
    identities: Vec<VcetCandidateIdentityInputs>,
) -> Result<VcetTemporalSealedCandidateManifestV1, String> {
    if identities.is_empty() {
        return Err("VCET temporal candidate manifest must not be empty".to_string());
    }
    if identities.len() > MAX_VCET_SEARCH_BUDGET {
        return Err(format!(
            "VCET temporal candidate manifest exceeds the {MAX_VCET_SEARCH_BUDGET} candidate limit"
        ));
    }
    let mut identities = identities;
    identities.sort();
    if identities.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(
            "VCET temporal candidate manifest contains duplicate effective candidates".to_string(),
        );
    }
    for identity in &identities {
        validate_candidate_identity(identity)?;
    }

    let source_campaign_manifest_hash = normalize_sha256(
        &source_campaign_manifest_hash.into(),
        "source_campaign_manifest_hash",
    )?;
    let source_candidate_manifest_hash = normalize_sha256(
        &source_candidate_manifest_hash.into(),
        "source_candidate_manifest_hash",
    )?;
    let source_search_id = source_search_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(search_id) = source_search_id.as_deref() {
        if !search_id.starts_with("vcet_search_v1_") {
            return Err(
                "source_search_id must use the vcet_search_v1 identity contract".to_string(),
            );
        }
    }

    let candidates = identities
        .into_iter()
        .enumerate()
        .map(|(ordinal, identity_inputs)| VcetTemporalCandidateEntryV1 {
            ordinal,
            candidate_id: identity_inputs.candidate_id(),
            identity_inputs,
        })
        .collect();
    let payload = VcetTemporalCandidateManifestPayloadV1 {
        schema_version: VCET_TEMPORAL_CANDIDATE_MANIFEST_SCHEMA_VERSION,
        strategy_kind: "vcet".to_string(),
        source_campaign_manifest_hash,
        source_candidate_manifest_hash,
        source_search_id,
        candidates,
    };
    let manifest_sha256 = candidate_manifest_sha256(&payload)?;
    Ok(VcetTemporalSealedCandidateManifestV1 {
        payload,
        manifest_sha256,
    })
}

pub(super) fn validate_sealed_candidate_manifest(
    manifest: &VcetTemporalSealedCandidateManifestV1,
    expected_manifest_sha256: &str,
) -> Result<(), String> {
    let payload = &manifest.payload;
    if payload.schema_version != VCET_TEMPORAL_CANDIDATE_MANIFEST_SCHEMA_VERSION {
        return Err(format!(
            "unsupported VCET temporal candidate manifest schema version {}; expected {}",
            payload.schema_version, VCET_TEMPORAL_CANDIDATE_MANIFEST_SCHEMA_VERSION
        ));
    }
    if payload.strategy_kind != "vcet" {
        return Err("VCET temporal candidate manifest strategy_kind must be 'vcet'".to_string());
    }
    if payload.candidates.is_empty() || payload.candidates.len() > MAX_VCET_SEARCH_BUDGET {
        return Err(format!(
            "VCET temporal candidate manifest must contain between 1 and {MAX_VCET_SEARCH_BUDGET} candidates"
        ));
    }
    for (expected_ordinal, candidate) in payload.candidates.iter().enumerate() {
        validate_candidate_identity(&candidate.identity_inputs)?;
        if candidate.ordinal != expected_ordinal {
            return Err(format!(
                "VCET temporal candidate ordinal {} does not match canonical ordinal {expected_ordinal}",
                candidate.ordinal
            ));
        }
        let expected_candidate_id = candidate.identity_inputs.candidate_id();
        if candidate.candidate_id != expected_candidate_id {
            return Err(format!(
                "VCET temporal candidate {} identity mismatch: expected {expected_candidate_id}",
                candidate.ordinal
            ));
        }
        if expected_ordinal > 0
            && payload.candidates[expected_ordinal - 1].identity_inputs >= candidate.identity_inputs
        {
            return Err(
                "VCET temporal candidates must be unique and ordered by canonical identity"
                    .to_string(),
            );
        }
    }
    let normalized_campaign_hash = normalize_sha256(
        &payload.source_campaign_manifest_hash,
        "source_campaign_manifest_hash",
    )?;
    let normalized_candidate_hash = normalize_sha256(
        &payload.source_candidate_manifest_hash,
        "source_candidate_manifest_hash",
    )?;
    if normalized_campaign_hash != payload.source_campaign_manifest_hash
        || normalized_candidate_hash != payload.source_candidate_manifest_hash
    {
        return Err(
            "VCET temporal source hashes must use canonical lowercase sha256: form".to_string(),
        );
    }
    if let Some(search_id) = payload.source_search_id.as_deref() {
        if !search_id.starts_with("vcet_search_v1_") || search_id.trim() != search_id {
            return Err(
                "source_search_id must use the canonical vcet_search_v1 contract".to_string(),
            );
        }
    }

    let declared = normalize_sha256(&manifest.manifest_sha256, "manifest_sha256")?;
    let expected = normalize_sha256(
        expected_manifest_sha256,
        "expected_candidate_manifest_sha256",
    )?;
    let actual = candidate_manifest_sha256(payload)?;
    if declared != manifest.manifest_sha256 {
        return Err(
            "VCET temporal manifest_sha256 must use canonical lowercase sha256: form".to_string(),
        );
    }
    if declared != actual || expected != actual {
        return Err(format!(
            "VCET temporal candidate manifest SHA-256 mismatch (declared {declared}, expected {expected}, actual {actual})"
        ));
    }
    Ok(())
}

fn timeline_sha256(
    timestamps_utc_ms: &[i64],
    session_calendar_id: Option<&str>,
    session_open_utc_ms: Option<&[i64]>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"vectorgrid.vcet.temporal_timeline.v1\0");
    hasher.update((timestamps_utc_ms.len() as u64).to_le_bytes());
    for timestamp in timestamps_utc_ms {
        hasher.update(timestamp.to_le_bytes());
    }
    match (session_calendar_id, session_open_utc_ms) {
        (Some(calendar_id), Some(sessions)) => {
            hasher.update([1]);
            hasher.update((calendar_id.len() as u64).to_le_bytes());
            hasher.update(calendar_id.as_bytes());
            hasher.update((sessions.len() as u64).to_le_bytes());
            for session in sessions {
                hasher.update(session.to_le_bytes());
            }
        }
        _ => hasher.update([0]),
    }
    let digest = hasher.finalize();
    let mut value = String::with_capacity(7 + digest.len() * 2);
    value.push_str("sha256:");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in digest {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 0x0f) as usize] as char);
    }
    value
}

pub fn plan_vcet_temporal_validation_blocking(
    mut req: PlanVcetTemporalValidationRequestV1,
) -> Result<PlanVcetTemporalValidationResponseV1, String> {
    validate_sealed_candidate_manifest(
        &req.sealed_candidate_manifest,
        &req.expected_candidate_manifest_sha256,
    )?;
    let plan_identity =
        vcet_temporal_plan_identity(&req.plan).map_err(|error| error.to_string())?;
    if plan_identity != req.expected_plan_identity {
        return Err(format!(
            "VCET temporal plan identity mismatch (expected {} / {}, actual {} / {})",
            req.expected_plan_identity.plan_id,
            req.expected_plan_identity.plan_sha256,
            plan_identity.plan_id,
            plan_identity.plan_sha256
        ));
    }
    if req.datasets.is_empty() {
        return Err("VCET temporal planning requires at least one dataset".to_string());
    }
    if req.datasets.len() > MAX_VCET_TEMPORAL_DATASETS_PER_REQUEST {
        return Err(format!(
            "VCET temporal planning accepts at most {MAX_VCET_TEMPORAL_DATASETS_PER_REQUEST} datasets per request"
        ));
    }

    for dataset in &mut req.datasets {
        dataset.dataset_id = dataset.dataset_id.trim().to_string();
        if dataset.dataset_id.is_empty() {
            return Err("VCET temporal dataset_id must not be empty".to_string());
        }
        dataset.source_dataset_sha256 =
            normalize_sha256(&dataset.source_dataset_sha256, "source_dataset_sha256")?;
        if let Some(calendar_id) = dataset.session_calendar_id.as_mut() {
            *calendar_id = calendar_id.trim().to_string();
            if calendar_id.is_empty() {
                return Err(format!(
                    "dataset '{}' session_calendar_id must not be empty",
                    dataset.dataset_id
                ));
            }
        }
        if dataset.session_calendar_id.is_some() != dataset.session_open_utc_ms.is_some() {
            return Err(format!(
                "dataset '{}' must provide session_calendar_id and session_open_utc_ms together",
                dataset.dataset_id
            ));
        }
    }
    req.datasets
        .sort_by(|left, right| left.dataset_id.cmp(&right.dataset_id));
    if req
        .datasets
        .windows(2)
        .any(|pair| pair[0].dataset_id == pair[1].dataset_id)
    {
        return Err("VCET temporal planning dataset_id values must be unique".to_string());
    }
    let total_timeline_values = req.datasets.iter().try_fold(0usize, |total, dataset| {
        let dataset_values = dataset
            .timestamps_utc_ms
            .len()
            .checked_add(dataset.session_open_utc_ms.as_ref().map_or(0, Vec::len))
            .ok_or_else(|| "VCET temporal dataset timeline count overflow".to_string())?;
        total
            .checked_add(dataset_values)
            .ok_or_else(|| "VCET temporal total timeline count overflow".to_string())
    })?;
    if total_timeline_values > MAX_VCET_TEMPORAL_TIMELINE_VALUES_PER_REQUEST {
        return Err(format!(
            "VCET temporal planning accepts at most {MAX_VCET_TEMPORAL_TIMELINE_VALUES_PER_REQUEST} total timestamp/session values per request; received {total_timeline_values}"
        ));
    }

    let mut dataset_headers = Vec::with_capacity(req.datasets.len());
    let mut layouts = Vec::with_capacity(req.datasets.len());
    for dataset in req.datasets {
        let session = match (
            dataset.session_calendar_id.as_deref(),
            dataset.session_open_utc_ms.as_deref(),
        ) {
            (Some(calendar_id), Some(session_open_utc_ms)) => Some(VcetTemporalSessionSeriesRef {
                calendar_id,
                session_open_utc_ms,
            }),
            (None, None) => None,
            _ => unreachable!("session fields were validated together"),
        };
        let timeline_sha256 = timeline_sha256(
            &dataset.timestamps_utc_ms,
            dataset.session_calendar_id.as_deref(),
            dataset.session_open_utc_ms.as_deref(),
        );
        let layout = plan_vcet_temporal_folds(
            &req.plan,
            dataset.dataset_id.clone(),
            &dataset.timestamps_utc_ms,
            session,
        )
        .map_err(|error| {
            format!(
                "dataset '{}' failed VCET temporal planning [{}]: {}",
                dataset.dataset_id,
                serde_json::to_value(error.code)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_string))
                    .unwrap_or_else(|| "unknown".to_string()),
                error
            )
        })?;
        dataset_headers.push((
            dataset.dataset_id,
            dataset.source_dataset_sha256,
            timeline_sha256,
        ));
        layouts.push(layout);
    }

    validate_vcet_temporal_cross_dataset_alignment(&req.plan, &layouts)
        .map_err(|error| format!("VCET temporal cross-dataset alignment failed: {error}"))?;
    let dataset_evidence = dataset_headers
        .into_iter()
        .zip(layouts)
        .map(
            |((dataset_id, source_dataset_sha256, timeline_sha256), layout)| {
                VcetTemporalDatasetPlanningEvidenceV1 {
                    dataset_id,
                    source_dataset_sha256,
                    timeline_sha256,
                    coverage_status: VcetTemporalCoverageStatusV1::Covered,
                    observed_bar_count: layout.dataset.observation.bar_count,
                    selected_bar_count: layout.dataset.selected_bars.len(),
                    fold_count: layout.folds.len(),
                    layout,
                }
            },
        )
        .collect::<Vec<_>>();

    let candidate_payload = &req.sealed_candidate_manifest.payload;
    let candidate_manifest = VcetTemporalCandidateManifestEvidenceV1 {
        manifest_sha256: req.sealed_candidate_manifest.manifest_sha256,
        source_campaign_manifest_hash: candidate_payload.source_campaign_manifest_hash.clone(),
        source_candidate_manifest_hash: candidate_payload.source_candidate_manifest_hash.clone(),
        source_search_id: candidate_payload.source_search_id.clone(),
        candidate_count: candidate_payload.candidates.len(),
    };
    let limitations = vec![
        "planning_only_no_candidate_backtests_metrics_or_promotion_decisions".to_string(),
        "in_memory_timeline_request_requires_bounded_batches_for_provider_scale_universes"
            .to_string(),
        "source_hashes_are_lineage_assertions_not_signatures_or_artifact_store_lookups".to_string(),
    ];
    let next_required_stage = "execute the locked candidate manifest over every scored interval, record trade support and backend provenance, lock one winner before final-test access, then persist a separately hashed evaluation artifact".to_string();
    let alignment_reference_dataset_id = dataset_evidence[0].dataset_id.clone();
    let evidence_kind = "vcet_temporal_validation_plan_evidence".to_string();
    let planning_status = VcetTemporalPlanningStatusV1::PlannedNotExecuted;
    let alignment_status = VcetTemporalAlignmentStatusV1::ExactAcrossDatasets;
    let hash_input = PlanningEvidenceHashInput {
        hash_contract: PLANNING_EVIDENCE_HASH_CONTRACT,
        schema_version: VCET_TEMPORAL_PLANNING_EVIDENCE_SCHEMA_VERSION,
        evidence_kind: &evidence_kind,
        planning_status,
        evaluation_executed: false,
        evaluation_result_count: 0,
        candidate_manifest: &candidate_manifest,
        plan_identity: &plan_identity,
        dataset_count: dataset_evidence.len(),
        alignment_status,
        alignment_reference_dataset_id: &alignment_reference_dataset_id,
        datasets: &dataset_evidence,
        limitations: &limitations,
        next_required_stage: &next_required_stage,
    };
    let evidence_sha256 = sha256_serializable(&hash_input, "VCET temporal planning evidence")?;

    Ok(PlanVcetTemporalValidationResponseV1 {
        schema_version: VCET_TEMPORAL_PLANNING_EVIDENCE_SCHEMA_VERSION,
        evidence_kind,
        planning_status,
        evaluation_executed: false,
        evaluation_result_count: 0,
        candidate_manifest,
        plan_identity,
        dataset_count: dataset_evidence.len(),
        alignment_status,
        alignment_reference_dataset_id,
        datasets: dataset_evidence,
        limitations,
        next_required_stage,
        evidence_sha256,
    })
}

#[tauri::command]
pub async fn plan_vcet_temporal_validation(
    req: PlanVcetTemporalValidationRequestV1,
) -> Result<PlanVcetTemporalValidationResponseV1, String> {
    tauri::async_runtime::spawn_blocking(move || plan_vcet_temporal_validation_blocking(req))
        .await
        .map_err(|error| format!("VCET temporal planning task failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use ta_app_core::vcet_temporal::{
        VcetTemporalFoldModeV1, VcetTemporalMinimumSupportV1, VcetTemporalPositionBoundaryPolicyV1,
        VcetTemporalRequestedBoundsV1, VcetTemporalUtcBoundsV1,
        VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION,
    };

    fn lineage_hash(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn identity(short: u16, long: u16) -> VcetCandidateIdentityInputs {
        VcetCandidateIdentityInputs {
            vidya_short_period: short,
            vidya_long_period: long,
            vidya_alpha_micros: 200_000,
            ce_period: 20,
            ce_mult_micros: 3_000_000,
            ce_use_close: true,
            require_vidya_slope: true,
            max_trade_bars: 120,
        }
    }

    fn plan(end: i64) -> VcetTemporalValidationPlanV1 {
        VcetTemporalValidationPlanV1 {
            schema_version: VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION,
            requested_bounds: VcetTemporalRequestedBoundsV1 {
                utc: VcetTemporalUtcBoundsV1 {
                    start_utc_ms_inclusive: 0,
                    end_utc_ms_inclusive: end,
                },
                session: None,
            },
            fold_mode: VcetTemporalFoldModeV1::Anchored,
            train_bars: 8,
            validation_bars: 4,
            final_test_bars: 5,
            step_bars: 5,
            purge_bars: 1,
            embargo_bars: 1,
            warmup_bars: 2,
            position_boundary_policy: VcetTemporalPositionBoundaryPolicyV1::ResetFlatForceLiquidate,
            minimum_support: VcetTemporalMinimumSupportV1 {
                min_folds: 2,
                min_train_bars: 8,
                min_validation_bars: 4,
                min_final_test_bars: 5,
                min_train_trades_per_fold: 2,
                min_validation_trades_per_fold: 1,
                min_final_test_trades: 1,
            },
        }
    }

    fn timestamps(count: usize) -> Vec<i64> {
        (0..count).map(|index| index as i64 * 60_000).collect()
    }

    fn request() -> PlanVcetTemporalValidationRequestV1 {
        let timestamps = timestamps(45);
        let plan = plan(*timestamps.last().unwrap());
        let manifest = seal_vcet_temporal_candidate_manifest(
            lineage_hash('a'),
            lineage_hash('b'),
            Some(format!("vcet_search_v1_{}", "c".repeat(64))),
            vec![identity(8, 32), identity(4, 24)],
        )
        .expect("sealed manifest");
        PlanVcetTemporalValidationRequestV1 {
            expected_candidate_manifest_sha256: manifest.manifest_sha256.clone(),
            sealed_candidate_manifest: manifest,
            expected_plan_identity: vcet_temporal_plan_identity(&plan).expect("plan identity"),
            plan,
            datasets: vec![
                VcetTemporalDatasetTimelineInputV1 {
                    dataset_id: "MSFT".to_string(),
                    source_dataset_sha256: lineage_hash('d'),
                    timestamps_utc_ms: timestamps.clone(),
                    session_calendar_id: None,
                    session_open_utc_ms: None,
                },
                VcetTemporalDatasetTimelineInputV1 {
                    dataset_id: "AAPL".to_string(),
                    source_dataset_sha256: lineage_hash('e'),
                    timestamps_utc_ms: timestamps,
                    session_calendar_id: None,
                    session_open_utc_ms: None,
                },
            ],
        }
    }

    #[test]
    fn planning_is_deterministic_sorted_aligned_and_explicitly_not_executed() {
        let req = request();
        let first = plan_vcet_temporal_validation_blocking(req.clone()).expect("first plan");
        let mut reordered = req;
        reordered.datasets.reverse();
        let second = plan_vcet_temporal_validation_blocking(reordered).expect("second plan");

        assert_eq!(first, second);
        assert_eq!(
            first.planning_status,
            VcetTemporalPlanningStatusV1::PlannedNotExecuted
        );
        assert!(!first.evaluation_executed);
        assert_eq!(first.evaluation_result_count, 0);
        assert_eq!(
            first.alignment_status,
            VcetTemporalAlignmentStatusV1::ExactAcrossDatasets
        );
        assert_eq!(first.alignment_reference_dataset_id, "AAPL");
        assert_eq!(first.candidate_manifest.candidate_count, 2);
        assert_eq!(first.evidence_sha256.len(), 71);
        assert_eq!(first.datasets[0].dataset_id, "AAPL");
        assert!(first.datasets.iter().all(|dataset| {
            dataset.coverage_status == VcetTemporalCoverageStatusV1::Covered
                && dataset.fold_count >= 2
        }));

        let json = serde_json::to_value(&first).expect("response json");
        assert_eq!(json["evaluation_executed"], false);
        assert_eq!(json["evaluation_result_count"], 0);
        assert!(json.get("metrics").is_none());
        assert!(json.get("candidate_results").is_none());
    }

    #[test]
    fn altered_manifest_content_is_rejected_against_both_seals() {
        let mut req = request();
        req.sealed_candidate_manifest.payload.candidates[0]
            .identity_inputs
            .ce_period += 1;
        let error = plan_vcet_temporal_validation_blocking(req).unwrap_err();
        assert!(error.contains("identity mismatch") || error.contains("SHA-256 mismatch"));
    }

    #[test]
    fn independently_resealed_noncanonical_candidate_id_is_rejected() {
        let mut req = request();
        req.sealed_candidate_manifest.payload.candidates[0].candidate_id = "wrong".to_string();
        req.sealed_candidate_manifest.manifest_sha256 =
            candidate_manifest_sha256(&req.sealed_candidate_manifest.payload)
                .expect("resealed hash");
        req.expected_candidate_manifest_sha256 =
            req.sealed_candidate_manifest.manifest_sha256.clone();
        let error = plan_vcet_temporal_validation_blocking(req).unwrap_err();
        assert!(error.contains("identity mismatch"));
    }

    #[test]
    fn timeline_gap_that_shifts_folds_fails_cross_dataset_alignment() {
        let mut req = request();
        req.datasets[0].timestamps_utc_ms.remove(12);
        let error = plan_vcet_temporal_validation_blocking(req).unwrap_err();
        assert!(error.contains("cross-dataset alignment failed"));
    }

    #[test]
    fn plan_identity_and_session_shape_are_fail_closed() {
        let mut wrong_plan = request();
        wrong_plan.expected_plan_identity.plan_sha256 = lineage_hash('f');
        assert!(plan_vcet_temporal_validation_blocking(wrong_plan)
            .unwrap_err()
            .contains("plan identity mismatch"));

        let mut missing_session = request();
        missing_session.datasets[0].session_calendar_id = Some("XNYS".to_string());
        assert!(plan_vcet_temporal_validation_blocking(missing_session)
            .unwrap_err()
            .contains("must provide session_calendar_id and session_open_utc_ms together"));
    }

    #[test]
    fn manifest_builder_normalizes_order_and_rejects_duplicates() {
        let manifest = seal_vcet_temporal_candidate_manifest(
            lineage_hash('a'),
            lineage_hash('b'),
            None,
            vec![identity(8, 32), identity(4, 24)],
        )
        .expect("manifest");
        assert!(
            manifest.payload.candidates[0].identity_inputs
                < manifest.payload.candidates[1].identity_inputs
        );
        assert_eq!(
            manifest.payload.candidates[0].candidate_id,
            manifest.payload.candidates[0]
                .identity_inputs
                .candidate_id()
        );

        let error = seal_vcet_temporal_candidate_manifest(
            lineage_hash('a'),
            lineage_hash('b'),
            None,
            vec![identity(4, 24), identity(4, 24)],
        )
        .unwrap_err();
        assert!(error.contains("duplicate"));
    }

    #[test]
    fn declared_candidate_manifest_expectation_cannot_be_substituted() {
        let mut req = request();
        req.expected_candidate_manifest_sha256 = lineage_hash('0');
        assert!(plan_vcet_temporal_validation_blocking(req)
            .unwrap_err()
            .contains("SHA-256 mismatch"));
    }
}
