//! Bounded, backend-owned VCET temporal campaign orchestration.
//!
//! This product surface intentionally accepts only an app-owned terminal
//! `evaluate_candidates` campaign summary. It promotes the already-admitted
//! candidates into the exact temporal identity contract, prepares at most one
//! loaded dataset at a time, persists immutable stage artifacts, and keeps the
//! final-test gate behind an explicit winner-lock confirmation.

use super::commands_campaign::{
    build_vcet_fixed_candidate_artifact_contract, build_vcet_temporal_campaign_candidate_source,
    prepare_campaign_dataset, validate_campaign_summary_artifact_value,
    validate_vcet_fixed_candidate_artifact_contract,
    validate_vcet_fixed_candidate_artifact_request_contract,
    validate_vcet_fixed_candidate_training_evidence, CampaignEvidenceState, CampaignResumeState,
    ResearchCampaignDatasetInput, ResearchCampaignSummary,
};
use super::commands_vcet_temporal::{
    plan_vcet_temporal_validation_blocking, PlanVcetTemporalValidationRequestV1,
    PlanVcetTemporalValidationResponseV1, VcetTemporalDatasetTimelineInputV1,
};
use super::commands_vcet_temporal_execute::{
    loaded_candles_snapshot_sha256, profile_loaded_dataset_coverage_for_planning,
    sha256_serializable, validate_loaded_dataset_shape, VcetTemporalLoadedDatasetBindingV1,
    VcetTemporalSelectionPolicyV1,
};
use super::commands_vcet_temporal_final::{
    execute_vcet_temporal_final_test_shard_durable_blocking_with_loaded_candles,
    reduce_vcet_temporal_final_test_shards_durable_blocking,
    validate_vcet_temporal_final_reduction_artifact, validate_vcet_temporal_final_shard_artifact,
    ExecuteVcetTemporalFinalTestShardRequestV1, ExecuteVcetTemporalFinalTestShardResponseV1,
    ReduceVcetTemporalFinalTestShardsRequestV1, ReduceVcetTemporalFinalTestShardsResponseV1,
};
use super::commands_vcet_temporal_prefinal::{
    execute_vcet_temporal_pre_final_shard_blocking_with_loaded_candles,
    reduce_vcet_temporal_pre_final_shards_durable_blocking,
    validate_vcet_temporal_pre_final_reduction_against_universe,
    validate_vcet_temporal_pre_final_reduction_artifact,
    validate_vcet_temporal_pre_final_shard_artifact, ExecuteVcetTemporalPreFinalShardRequestV1,
    ExecuteVcetTemporalPreFinalShardResponseV1, ReduceVcetTemporalPreFinalShardsRequestV1,
    ReduceVcetTemporalPreFinalShardsResponseV1, MAX_VCET_TEMPORAL_INLINE_PRE_FINAL_SHARD_FAN_IN,
};
use super::commands_vcet_temporal_universe::{
    recompute_vcet_dataset_attestation_sha256, recompute_vcet_dataset_coverage_sha256,
    recompute_vcet_temporal_interval_fingerprint_sha256,
    seal_vcet_temporal_dataset_universe_blocking, universe_shards_for_role,
    validate_sealed_vcet_temporal_universe, SealVcetTemporalDatasetUniverseRequestV1,
    VcetTemporalCrossUniverseAdmissionV1, VcetTemporalDatasetCohortRoleV1,
    VcetTemporalPhasedExecutionSpecV1, VcetTemporalSealedDatasetUniverseV1,
    VcetTemporalShardPolicyV1, VcetTemporalUniverseDatasetV1,
    VCET_TEMPORAL_CROSS_UNIVERSE_ADMISSION_SCHEMA_VERSION,
    VCET_TEMPORAL_SELECTION_REDUCTION_CONTRACT_V1,
};
use super::vcet_temporal_durable_ledger::{
    probe_durable_final_completion_for_authorization, probe_durable_pre_final_winner_lock,
    DurableCompletionProbe, DurableProbeBlockedReason, DurableProbeUnavailable,
};
use super::{commands_reports, run_artifacts};
use crate::data_cache;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use ta_app_core::vcet_temporal::{
    vcet_temporal_plan_identity, VcetTemporalPositionBoundaryPolicyV1, VcetTemporalValidationPlanV1,
};
use tauri::Manager;
use vector_ta::utilities::data_loader::Candles;

pub const VCET_TEMPORAL_PRODUCT_CAMPAIGN_SCHEMA_VERSION: u16 = 1;
pub const MAX_VCET_TEMPORAL_PRODUCT_DATASETS: usize = 64;
pub const MAX_VCET_TEMPORAL_PRODUCT_TIMELINE_VALUES_PER_DATASET: usize = 2_000_000;
pub const MAX_VCET_TEMPORAL_PRODUCT_TOTAL_TIMELINE_VALUES: usize = 10_000_000;
pub const MAX_VCET_TEMPORAL_PRODUCT_EVIDENCE_CELLS: usize = 524_288;
pub const MAX_VCET_TEMPORAL_PRODUCT_CANDIDATE_WINDOW_TRIALS: usize = 20_000_000;
pub const MAX_VCET_TEMPORAL_PRODUCT_PRE_FINAL_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_VCET_TEMPORAL_PRODUCT_SOURCE_TRAINING_ARTIFACT_BYTES: u64 = 256 * 1024 * 1024;

const PREPARATION_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_product_preparation.v1";
const CAMPAIGN_IDENTITY_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_product_campaign_identity.v1";
const SOURCE_DATASET_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_product_source_dataset.v1";
const REPORT_RECEIPT_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_product_report_receipt.v1";
const CAMPAIGN_DIR_NAME: &str = "vcet_temporal_campaigns";
const PREPARATION_FILE_NAME: &str = "preparation.json";
const PRE_FINAL_REDUCTION_FILE_NAME: &str = "pre_final_reduction.json";
const FINAL_REDUCTION_FILE_NAME: &str = "final_reduction.json";
const SELECTION_REPORT_RECEIPT_FILE_NAME: &str = "selection_report.json";
const FINAL_REPORT_RECEIPT_FILE_NAME: &str = "final_report.json";
const MAX_PRODUCT_ARTIFACT_READ_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareVcetTemporalCampaignRequestV1 {
    pub source_campaign_summary_path: String,
    pub plan: VcetTemporalValidationPlanV1,
    #[serde(default)]
    pub report_label: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunVcetTemporalCampaignSelectionRequestV1 {
    pub preparation_artifact_path: String,
    pub expected_preparation_artifact_sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenVcetTemporalCampaignFinalTestRequestV1 {
    pub preparation_artifact_path: String,
    pub expected_preparation_artifact_sha256: String,
    pub expected_winner_lock_sha256: String,
    pub confirm_final_test_access: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetVcetTemporalCampaignStatusRequestV1 {
    pub preparation_artifact_path: String,
    pub expected_preparation_artifact_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VcetTemporalCampaignStateV1 {
    Prepared,
    SelectionComplete,
    FinalTestComplete,
}

#[derive(Clone, Debug, Serialize)]
pub struct VcetTemporalCampaignStatusV1 {
    pub schema_version: u16,
    pub campaign_id: String,
    pub state: VcetTemporalCampaignStateV1,
    pub preparation_artifact_path: String,
    pub preparation_artifact_sha256: String,
    pub source_campaign_summary_path: String,
    pub source_campaign_summary_sha256: String,
    pub universe_sha256: String,
    pub candidate_count: usize,
    pub dataset_count: usize,
    pub selection_dataset_count: usize,
    pub final_test_dataset_count: usize,
    pub selection_shard_count: usize,
    pub completed_selection_shard_count: usize,
    pub final_test_shard_count: usize,
    pub completed_final_test_shard_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner_candidate_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner_lock_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner_runner_up_margin: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_final_reduction_artifact_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_final_reduction_artifact_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_reduction_artifact_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_reduction_artifact_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_report_xlsx_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_report_xlsx_path: Option<String>,
    pub execution_blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_shard_ordinal: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_reason_code: Option<String>,
    pub next_required_action: String,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VcetTemporalCampaignDatasetSpecV1 {
    dataset_id: String,
    role: VcetTemporalDatasetCohortRoleV1,
    input: ResearchCampaignDatasetInput,
    source_run_artifact_path: String,
    source_run_artifact_sha256: String,
    source_dataset_sha256: String,
    loaded_candles_snapshot_sha256: String,
    timeline_sha256: String,
    dataset_attestation_sha256: String,
    dataset_coverage_sha256: String,
    observed_bar_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VcetTemporalCampaignPreparationPayloadV1 {
    schema_version: u16,
    evidence_kind: String,
    campaign_id: String,
    source_campaign_summary_path: String,
    source_campaign_summary_sha256: String,
    source_campaign_manifest_sha256: String,
    source_candidate_manifest_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    report_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timeframe_override: Option<String>,
    datasets: Vec<VcetTemporalCampaignDatasetSpecV1>,
    sealed_universe: VcetTemporalSealedDatasetUniverseV1,
    total_timeline_value_count: usize,
    limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VcetTemporalCampaignPreparationArtifactV1 {
    payload: VcetTemporalCampaignPreparationPayloadV1,
    artifact_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VcetTemporalCampaignReportReceiptPayloadV1 {
    schema_version: u16,
    campaign_id: String,
    stage: String,
    report_path: String,
    report_bytes: usize,
    report_sha256: String,
    source_artifact_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VcetTemporalCampaignReportReceiptV1 {
    payload: VcetTemporalCampaignReportReceiptPayloadV1,
    artifact_sha256: String,
}

#[derive(Serialize)]
struct HashEnvelope<'a, T> {
    hash_contract: &'static str,
    value: &'a T,
}

#[derive(Serialize)]
struct SourceDatasetHashInput<'a> {
    hash_contract: &'static str,
    dataset_id: &'a str,
    input: &'a ResearchCampaignDatasetInput,
    effective_timeframe: &'a str,
    source_run_artifact_sha256: &'a str,
}

struct LoadedProductDataset {
    planning_request: PlanVcetTemporalValidationRequestV1,
    planning_evidence: PlanVcetTemporalValidationResponseV1,
    binding: VcetTemporalLoadedDatasetBindingV1,
    candles: Arc<Candles>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VcetTemporalCampaignExecutionBlock {
    stage: &'static str,
    shard_ordinal: Option<usize>,
    reason_code: &'static str,
}

const DURABLE_STAGE_NOT_RECOVERABLE_REASON: &str = "durable_stage_failed_interrupted_or_invalid";
const REPORT_RECEIPT_NOT_RECOVERABLE_REASON: &str = "report_receipt_invalid_or_foreign";
const REPORT_XLSX_NOT_RECOVERABLE_REASON: &str = "report_xlsx_not_recoverable";
const BLOCKED_CAMPAIGN_ACTION: &str = "start_new_campaign";

static ACTIVE_UNIVERSE_AUTHORIZATIONS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

#[derive(Debug)]
struct ActiveCampaignGuard {
    authorization_identity: String,
}

impl Drop for ActiveCampaignGuard {
    fn drop(&mut self) {
        if let Ok(mut active) = ACTIVE_UNIVERSE_AUTHORIZATIONS
            .get_or_init(|| Mutex::new(HashSet::new()))
            .lock()
        {
            active.remove(&self.authorization_identity);
        }
    }
}

/// Process-local serialization for the durable universe authorization.
///
/// The desktop app registers `tauri-plugin-single-instance`, so product
/// commands execute in one application process. The durable ledger remains the
/// fail-closed authority after crashes; this guard prevents a live same-process
/// claim from being misclassified as interrupted by status or another campaign
/// alias with the same universe.
fn acquire_campaign_guard(authorization_identity: &str) -> Result<ActiveCampaignGuard, String> {
    let mut active = ACTIVE_UNIVERSE_AUTHORIZATIONS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .map_err(|_| "VCET temporal campaign operation registry is poisoned".to_string())?;
    if !active.insert(authorization_identity.to_string()) {
        return Err("VCET temporal universe already has an active operation".to_string());
    }
    Ok(ActiveCampaignGuard {
        authorization_identity: authorization_identity.to_string(),
    })
}

fn acquire_campaign_status_guard(
    authorization_identity: &str,
) -> Result<ActiveCampaignGuard, String> {
    acquire_campaign_guard(authorization_identity).map_err(|_| {
        "VCET temporal campaign operation is in progress; retry status refresh".to_string()
    })
}

fn product_limitations() -> Vec<String> {
    vec![
        format!(
            "product_v1_is_bounded_to_{MAX_VCET_TEMPORAL_PRODUCT_DATASETS}_datasets_and_larger_universes_require_separately_sealed_campaigns"
        ),
        format!(
            "product_v1_is_bounded_to_{MAX_VCET_TEMPORAL_PRODUCT_TOTAL_TIMELINE_VALUES}_aggregate_timeline_values"
        ),
        format!(
            "product_v1_is_bounded_to_{MAX_VCET_TEMPORAL_PRODUCT_TIMELINE_VALUES_PER_DATASET}_timeline_values_per_dataset"
        ),
        format!(
            "product_v1_is_bounded_to_{MAX_VCET_TEMPORAL_PRODUCT_CANDIDATE_WINDOW_TRIALS}_aggregate_candidate_window_trials"
        ),
        format!(
            "product_v1_is_bounded_to_{MAX_VCET_TEMPORAL_PRODUCT_SOURCE_TRAINING_ARTIFACT_BYTES}_aggregate_serialized_source_training_artifact_bytes"
        ),
        "one_dataset_is_loaded_per_execution_shard_to_bound_host_and_device_residency"
            .to_string(),
        "position_boundary_policy_is_fixed_to_reset_flat_discard_open_for_strict_cpu_gpu_parity"
            .to_string(),
        "all_source_campaign_items_and_raw_artifacts_must_be_complete_present_and_hash_valid_before_universe_sealing"
            .to_string(),
        "selection_resumes_only_from_immutable_hash_validated_shard_artifacts".to_string(),
        "final_test_access_requires_the_exact_persisted_winner_lock_and_an_explicit_confirmation"
            .to_string(),
        "source_evaluate_candidates_holdout_artifacts_already_exist_and_temporal_isolation_prevents_automated_selection_use_but_does_not_prove_no_prior_human_or_upstream_observation"
            .to_string(),
        "csv_calendar_normalization_gap_repair_and_corporate_action_adjustment_remain_upstream_data_responsibilities"
            .to_string(),
    ]
}

fn normalize_sha256(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("sha256:").unwrap_or(trimmed);
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{field} must contain exactly 64 hexadecimal SHA-256 digits"
        ));
    }
    Ok(format!("sha256:{}", hex.to_ascii_lowercase()))
}

fn preparation_sha256(
    payload: &VcetTemporalCampaignPreparationPayloadV1,
) -> Result<String, String> {
    sha256_serializable(
        &HashEnvelope {
            hash_contract: PREPARATION_HASH_CONTRACT,
            value: payload,
        },
        "VCET temporal product preparation",
    )
}

fn campaign_identity_sha256(
    payload: &VcetTemporalCampaignPreparationPayloadV1,
) -> Result<String, String> {
    let mut identity = payload.clone();
    identity.campaign_id.clear();
    sha256_serializable(
        &HashEnvelope {
            hash_contract: CAMPAIGN_IDENTITY_HASH_CONTRACT,
            value: &identity,
        },
        "VCET temporal product campaign identity",
    )
}

fn report_receipt_sha256(
    payload: &VcetTemporalCampaignReportReceiptPayloadV1,
) -> Result<String, String> {
    sha256_serializable(
        &HashEnvelope {
            hash_contract: REPORT_RECEIPT_HASH_CONTRACT,
            value: payload,
        },
        "VCET temporal product report receipt",
    )
}

fn source_dataset_sha256(
    dataset_id: &str,
    input: &ResearchCampaignDatasetInput,
    effective_timeframe: &str,
    source_run_artifact_sha256: &str,
) -> Result<String, String> {
    sha256_serializable(
        &SourceDatasetHashInput {
            hash_contract: SOURCE_DATASET_HASH_CONTRACT,
            dataset_id,
            input,
            effective_timeframe,
            source_run_artifact_sha256,
        },
        "VCET temporal source dataset lineage",
    )
}

#[cfg(windows)]
fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArtifactAccessFailureV1 {
    Invalid,
    Unavailable,
}

fn artifact_access_error(stage: &str, failure: ArtifactAccessFailureV1) -> String {
    match failure {
        ArtifactAccessFailureV1::Invalid => format!(
            "VCET temporal {stage} artifact is invalid or exceeds its bounded safety contract"
        ),
        ArtifactAccessFailureV1::Unavailable => format!(
            "VCET temporal {stage} artifact is temporarily unavailable; retry the operation"
        ),
    }
}

#[cfg(windows)]
fn open_artifact_read_handle(path: &Path) -> std::io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    fs::OpenOptions::new()
        .read(true)
        // Authenticate a stable read-only snapshot: concurrent readers are
        // allowed, while writers/deleters must wait until the handle closes.
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(not(windows))]
fn open_artifact_read_handle(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new().read(true).open(path)
}

fn open_bounded_regular_artifact(
    path: &Path,
    max_bytes: u64,
) -> Result<Option<(fs::File, u64)>, ArtifactAccessFailureV1> {
    #[cfg(not(windows))]
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(ArtifactAccessFailureV1::Invalid)
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ArtifactAccessFailureV1::Unavailable),
    }

    let file = match open_artifact_read_handle(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ArtifactAccessFailureV1::Unavailable),
    };
    let metadata = file
        .metadata()
        .map_err(|_| ArtifactAccessFailureV1::Unavailable)?;
    if metadata.file_type().is_symlink()
        || metadata_is_reparse_point(&metadata)
        || !metadata.is_file()
        || metadata.len() > max_bytes
    {
        return Err(ArtifactAccessFailureV1::Invalid);
    }
    Ok(Some((file, metadata.len())))
}

fn read_bounded_artifact_bytes(
    path: &Path,
    max_bytes: u64,
) -> Result<Option<Vec<u8>>, ArtifactAccessFailureV1> {
    let Some((file, observed_len)) = open_bounded_regular_artifact(path, max_bytes)? else {
        return Ok(None);
    };
    read_bounded_open_artifact(file, observed_len, max_bytes).map(Some)
}

fn read_bounded_open_artifact(
    file: fs::File,
    observed_len: u64,
    max_bytes: u64,
) -> Result<Vec<u8>, ArtifactAccessFailureV1> {
    let mut bytes = Vec::with_capacity(observed_len.min(max_bytes) as usize);
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| ArtifactAccessFailureV1::Unavailable)?;
    if bytes.len() as u64 > max_bytes {
        return Err(ArtifactAccessFailureV1::Invalid);
    }
    Ok(bytes)
}

fn read_json_file_classified<T: DeserializeOwned>(
    path: &Path,
    max_bytes: u64,
) -> Result<Option<T>, ArtifactAccessFailureV1> {
    let Some(bytes) = read_bounded_artifact_bytes(path, max_bytes)? else {
        return Ok(None);
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| ArtifactAccessFailureV1::Invalid)
}

fn ensure_regular_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink()
                || metadata_is_reparse_point(&metadata)
                || !metadata.is_dir()
            {
                return Err(format!(
                    "VCET temporal campaign directory is not an ordinary non-reparse directory: {}",
                    path.display()
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|error| {
                format!(
                    "create VCET temporal campaign directory {}: {error}",
                    path.display()
                )
            })?;
            let metadata = fs::symlink_metadata(path).map_err(|error| {
                format!(
                    "inspect created VCET temporal campaign directory {}: {error}",
                    path.display()
                )
            })?;
            if metadata.file_type().is_symlink()
                || metadata_is_reparse_point(&metadata)
                || !metadata.is_dir()
            {
                return Err(format!(
                    "created VCET temporal campaign path is not an ordinary non-reparse directory: {}",
                    path.display()
                ));
            }
        }
        Err(error) => {
            return Err(format!(
                "inspect VCET temporal campaign directory {}: {error}",
                path.display()
            ))
        }
    }
    Ok(())
}

fn ensure_regular_directory_classified(path: &Path) -> Result<(), ArtifactAccessFailureV1> {
    let inspect = |metadata: fs::Metadata| {
        if metadata.file_type().is_symlink()
            || metadata_is_reparse_point(&metadata)
            || !metadata.is_dir()
        {
            Err(ArtifactAccessFailureV1::Invalid)
        } else {
            Ok(())
        }
    };
    match fs::symlink_metadata(path) {
        Ok(metadata) => inspect(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::create_dir(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(ArtifactAccessFailureV1::Unavailable),
            }
            fs::symlink_metadata(path)
                .map_err(|_| ArtifactAccessFailureV1::Unavailable)
                .and_then(inspect)
        }
        Err(_) => Err(ArtifactAccessFailureV1::Unavailable),
    }
}

fn campaign_store_root(app_local_data_dir: &Path) -> Result<PathBuf, String> {
    ensure_regular_directory(app_local_data_dir)?;
    let root = app_local_data_dir.join(CAMPAIGN_DIR_NAME);
    ensure_regular_directory(&root)?;
    Ok(root)
}

fn campaign_dir(app_local_data_dir: &Path, campaign_id: &str) -> Result<PathBuf, String> {
    if campaign_id.is_empty()
        || campaign_id.len() > 96
        || !campaign_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("invalid VCET temporal product campaign_id".to_string());
    }
    let root = campaign_store_root(app_local_data_dir)?;
    let dir = root.join(campaign_id);
    ensure_regular_directory(&dir)?;
    Ok(dir)
}

#[cfg(test)]
fn file_sha256(path: &Path, max_bytes: u64) -> Result<String, String> {
    let (file, _) = open_bounded_regular_artifact(path, max_bytes)
        .map_err(|failure| artifact_access_error("file", failure))?
        .ok_or_else(|| "VCET temporal file artifact is missing".to_string())?;
    hash_bounded_open_artifact(file, max_bytes).map(|(sha256, _bytes)| sha256)
}

fn hash_bounded_open_artifact(file: fs::File, max_bytes: u64) -> Result<(String, u64), String> {
    hash_bounded_open_artifact_classified(file, max_bytes)
        .map_err(|failure| artifact_access_error("file", failure))
}

fn hash_bounded_open_artifact_classified(
    file: fs::File,
    max_bytes: u64,
) -> Result<(String, u64), ArtifactAccessFailureV1> {
    let mut file = file.take(max_bytes.saturating_add(1));
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| ArtifactAccessFailureV1::Unavailable)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or(ArtifactAccessFailureV1::Invalid)?;
        if total > max_bytes {
            return Err(ArtifactAccessFailureV1::Invalid);
        }
        hasher.update(&buffer[..count]);
    }
    Ok((format!("sha256:{:x}", hasher.finalize()), total))
}

#[cfg(test)]
fn read_json_file<T: DeserializeOwned>(path: &Path, max_bytes: u64) -> Result<T, String> {
    read_json_file_classified(path, max_bytes)
        .map_err(|failure| artifact_access_error("JSON", failure))?
        .ok_or_else(|| "VCET temporal JSON artifact is missing".to_string())
}

fn read_optional_json_file<T: DeserializeOwned>(
    path: &Path,
    stage: &str,
) -> Result<Option<T>, String> {
    read_json_file_classified(path, MAX_PRODUCT_ARTIFACT_READ_BYTES).map_err(
        |failure| match failure {
            ArtifactAccessFailureV1::Invalid => {
                format!("VCET temporal {stage} projection is invalid and cannot be resumed")
            }
            ArtifactAccessFailureV1::Unavailable => format!(
                "VCET temporal {stage} projection is temporarily unavailable; retry the operation"
            ),
        },
    )
}

fn publish_new_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("encode VCET temporal artifact {}: {error}", path.display()))?;
    run_artifacts::publish_new_artifact_file_atomically(path, &bytes)
}

#[cfg(windows)]
fn final_path_for_open_artifact(
    file: &fs::File,
    _original_path: &Path,
) -> Result<PathBuf, ArtifactAccessFailureV1> {
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFinalPathNameByHandleW, FILE_NAME_NORMALIZED, VOLUME_NAME_DOS,
    };

    let handle = file.as_raw_handle() as HANDLE;
    let mut buffer = vec![0u16; 512];
    loop {
        let length = unsafe {
            GetFinalPathNameByHandleW(
                handle,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
            )
        };
        if length == 0 {
            return Err(ArtifactAccessFailureV1::Unavailable);
        }
        if (length as usize) < buffer.len() {
            buffer.truncate(length as usize);
            return Ok(PathBuf::from(std::ffi::OsString::from_wide(&buffer)));
        }
        let capacity = (length as usize)
            .checked_add(1)
            .filter(|capacity| *capacity <= 32_768)
            .ok_or(ArtifactAccessFailureV1::Invalid)?;
        buffer.resize(capacity, 0);
    }
}

#[cfg(not(windows))]
fn final_path_for_open_artifact(
    _file: &fs::File,
    original_path: &Path,
) -> Result<PathBuf, ArtifactAccessFailureV1> {
    fs::canonicalize(original_path).map_err(|_| ArtifactAccessFailureV1::Unavailable)
}

fn open_canonical_preparation(
    raw: &str,
    app_local_data_dir: &Path,
) -> Result<(PathBuf, Vec<u8>), String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("preparation_artifact_path must not be empty".to_string());
    }
    let root = campaign_store_root(app_local_data_dir)?;
    let canonical_root = fs::canonicalize(&root).map_err(|error| {
        format!(
            "canonicalize VCET temporal campaign root {}: {error}",
            root.display()
        )
    })?;
    let path = PathBuf::from(raw);
    let (file, observed_len) =
        open_bounded_regular_artifact(&path, MAX_PRODUCT_ARTIFACT_READ_BYTES)
            .map_err(|failure| artifact_access_error("preparation", failure))?
            .ok_or_else(|| "VCET temporal preparation artifact is missing".to_string())?;
    let canonical = final_path_for_open_artifact(&file, &path)
        .map_err(|failure| artifact_access_error("preparation", failure))?;
    if !canonical.starts_with(&canonical_root)
        || canonical.file_name().and_then(|value| value.to_str()) != Some(PREPARATION_FILE_NAME)
        || canonical.parent().and_then(Path::parent) != Some(canonical_root.as_path())
    {
        return Err(
            "preparation_artifact_path must identify an app-owned VCET temporal campaign preparation"
                .to_string(),
        );
    }
    let bytes = read_bounded_open_artifact(file, observed_len, MAX_PRODUCT_ARTIFACT_READ_BYTES)
        .map_err(|failure| artifact_access_error("preparation", failure))?;
    Ok((canonical, bytes))
}

#[cfg(test)]
fn canonical_preparation_path(raw: &str, app_local_data_dir: &Path) -> Result<PathBuf, String> {
    let (canonical, _bytes) = open_canonical_preparation(raw, app_local_data_dir)?;
    // Internal lineage checks must use the canonical target rather than a
    // caller-supplied alias. Public status rendering strips Windows verbatim
    // prefixes separately.
    Ok(canonical)
}

fn public_path_display(path: &Path) -> String {
    let value = path.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = value.strip_prefix(r"\\?\") {
            return rest.to_string();
        }
    }
    value.into_owned()
}

fn validate_preparation_artifact(
    artifact: &VcetTemporalCampaignPreparationArtifactV1,
    expected_sha256: &str,
) -> Result<(), String> {
    let expected = normalize_sha256(expected_sha256, "expected_preparation_artifact_sha256")?;
    if artifact.payload.schema_version != VCET_TEMPORAL_PRODUCT_CAMPAIGN_SCHEMA_VERSION
        || artifact.payload.evidence_kind != "vcet_temporal_product_preparation"
        || artifact.payload.datasets.is_empty()
        || artifact.payload.datasets.len() > MAX_VCET_TEMPORAL_PRODUCT_DATASETS
        || artifact.payload.limitations != product_limitations()
    {
        return Err("invalid VCET temporal product preparation structure".to_string());
    }
    let actual = preparation_sha256(&artifact.payload)?;
    if normalize_sha256(&artifact.artifact_sha256, "preparation artifact_sha256")? != actual
        || expected != actual
        || artifact.artifact_sha256 != actual
    {
        return Err(format!(
            "VCET temporal preparation SHA-256 mismatch (expected {expected}, actual {actual})"
        ));
    }
    let campaign_identity = campaign_identity_sha256(&artifact.payload)?;
    let expected_campaign_id = format!(
        "vcet-temporal-product-v1-{}",
        campaign_identity
            .strip_prefix("sha256:")
            .expect("internal SHA-256 is prefixed")
            .chars()
            .take(24)
            .collect::<String>()
    );
    if artifact.payload.campaign_id != expected_campaign_id {
        return Err("invalid VCET temporal product campaign identity".to_string());
    }
    let source_summary_sha256 = normalize_sha256(
        &artifact.payload.source_campaign_summary_sha256,
        "source_campaign_summary_sha256",
    )?;
    let source_campaign_manifest_sha256 = normalize_sha256(
        &artifact.payload.source_campaign_manifest_sha256,
        "source_campaign_manifest_sha256",
    )?;
    let source_candidate_manifest_sha256 = normalize_sha256(
        &artifact.payload.source_candidate_manifest_sha256,
        "source_candidate_manifest_sha256",
    )?;
    if source_summary_sha256 != artifact.payload.source_campaign_summary_sha256
        || source_campaign_manifest_sha256 != artifact.payload.source_campaign_manifest_sha256
        || source_candidate_manifest_sha256 != artifact.payload.source_candidate_manifest_sha256
    {
        return Err("VCET temporal preparation source hashes are not canonical".to_string());
    }
    validate_sealed_vcet_temporal_universe(
        &artifact.payload.sealed_universe,
        &artifact.payload.sealed_universe.universe_sha256,
    )?;
    let sealed_payload = &artifact.payload.sealed_universe.payload;
    if source_campaign_manifest_sha256
        != sealed_payload
            .candidate_manifest
            .payload
            .source_campaign_manifest_hash
        || source_candidate_manifest_sha256
            != sealed_payload
                .candidate_manifest
                .payload
                .source_candidate_manifest_hash
    {
        return Err(
            "VCET temporal preparation source lineage differs from its sealed candidate manifest"
                .to_string(),
        );
    }
    let candidate_count = sealed_payload.candidate_manifest.payload.candidates.len();
    let evidence_cells = candidate_count
        .checked_mul(sealed_payload.dataset_count)
        .ok_or_else(|| "VCET temporal preparation evidence-cell count overflow".to_string())?;
    let aggregate_trials = sealed_payload
        .selection_dataset_count
        .checked_mul(candidate_count)
        .and_then(|value| value.checked_mul(sealed_payload.fold_count_per_dataset))
        .and_then(|value| value.checked_mul(2))
        .and_then(|value| value.checked_add(sealed_payload.final_test_dataset_count))
        .ok_or_else(|| {
            "VCET temporal preparation aggregate candidate-window trial overflow".to_string()
        })?;
    if evidence_cells > MAX_VCET_TEMPORAL_PRODUCT_EVIDENCE_CELLS
        || aggregate_trials > MAX_VCET_TEMPORAL_PRODUCT_CANDIDATE_WINDOW_TRIALS
        || sealed_payload.shard_policy.max_datasets_per_shard != 1
        || sealed_payload.plan.position_boundary_policy
            != VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen
        || sealed_payload
            .shards
            .iter()
            .any(|shard| shard.dataset_ids.len() != 1)
    {
        return Err(
            "VCET temporal preparation exceeds its product cardinality or residency bounds"
                .to_string(),
        );
    }
    if artifact.payload.datasets.len() != artifact.payload.sealed_universe.payload.dataset_count
        || artifact.payload.total_timeline_value_count
            != artifact
                .payload
                .datasets
                .iter()
                .try_fold(0usize, |sum, dataset| {
                    sum.checked_add(dataset.observed_bar_count)
                })
                .ok_or_else(|| "VCET temporal preparation bar count overflow".to_string())?
        || artifact.payload.total_timeline_value_count
            > MAX_VCET_TEMPORAL_PRODUCT_TOTAL_TIMELINE_VALUES
    {
        return Err("VCET temporal preparation dataset counts are inconsistent".to_string());
    }
    let universe = artifact
        .payload
        .sealed_universe
        .payload
        .datasets
        .iter()
        .map(|dataset| (dataset.dataset_id.as_str(), dataset))
        .collect::<BTreeMap<_, _>>();
    let mut seen = HashSet::new();
    for dataset in &artifact.payload.datasets {
        if dataset.observed_bar_count > MAX_VCET_TEMPORAL_PRODUCT_TIMELINE_VALUES_PER_DATASET {
            return Err(format!(
                "VCET temporal preparation dataset '{}' exceeds the per-dataset timeline limit",
                dataset.dataset_id
            ));
        }
        if !seen.insert(dataset.dataset_id.as_str()) {
            return Err("VCET temporal preparation contains duplicate dataset IDs".to_string());
        }
        let sealed = universe.get(dataset.dataset_id.as_str()).ok_or_else(|| {
            format!(
                "VCET temporal preparation dataset '{}' is absent from the sealed universe",
                dataset.dataset_id
            )
        })?;
        if sealed.role != dataset.role
            || sealed.source_dataset_sha256 != dataset.source_dataset_sha256
            || sealed.loaded_candles_snapshot_sha256 != dataset.loaded_candles_snapshot_sha256
            || sealed.timeline_sha256 != dataset.timeline_sha256
            || sealed.dataset_attestation_sha256 != dataset.dataset_attestation_sha256
            || sealed.dataset_coverage_sha256 != dataset.dataset_coverage_sha256
        {
            return Err(format!(
                "VCET temporal preparation dataset '{}' differs from its sealed universe entry",
                dataset.dataset_id
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
fn load_preparation(
    path: &Path,
    expected_sha256: &str,
) -> Result<VcetTemporalCampaignPreparationArtifactV1, String> {
    let artifact: VcetTemporalCampaignPreparationArtifactV1 =
        read_json_file(path, MAX_PRODUCT_ARTIFACT_READ_BYTES)?;
    validate_loaded_preparation(&artifact, path, expected_sha256)?;
    Ok(artifact)
}

fn validate_loaded_preparation(
    artifact: &VcetTemporalCampaignPreparationArtifactV1,
    path: &Path,
    expected_sha256: &str,
) -> Result<(), String> {
    validate_preparation_artifact(artifact, expected_sha256)?;
    if path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        != Some(artifact.payload.campaign_id.as_str())
    {
        return Err(
            "VCET temporal preparation campaign_id does not match its app-owned directory"
                .to_string(),
        );
    }
    Ok(())
}

fn load_requested_preparation(
    raw_path: &str,
    expected_sha256: &str,
    app_local_data_dir: &Path,
) -> Result<(PathBuf, VcetTemporalCampaignPreparationArtifactV1), String> {
    let (path, bytes) = open_canonical_preparation(raw_path, app_local_data_dir)?;
    let artifact: VcetTemporalCampaignPreparationArtifactV1 = serde_json::from_slice(&bytes)
        .map_err(|_| "VCET temporal preparation artifact is invalid".to_string())?;
    validate_loaded_preparation(&artifact, &path, expected_sha256)?;
    Ok((path, artifact))
}

fn preparation_artifact_path(dir: &Path) -> PathBuf {
    dir.join(PREPARATION_FILE_NAME)
}

fn pre_final_shard_path(dir: &Path, ordinal: usize) -> PathBuf {
    dir.join(format!("pre_final_shard_{ordinal:04}.json"))
}

fn final_shard_path(dir: &Path, ordinal: usize) -> PathBuf {
    dir.join(format!("final_shard_{ordinal:04}.json"))
}

fn pre_final_reduction_path(dir: &Path) -> PathBuf {
    dir.join(PRE_FINAL_REDUCTION_FILE_NAME)
}

fn final_reduction_path(dir: &Path) -> PathBuf {
    dir.join(FINAL_REDUCTION_FILE_NAME)
}

fn final_stage_nonce(
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
) -> Result<String, String> {
    let universe_hex = normalize_sha256(
        &preparation.payload.sealed_universe.universe_sha256,
        "universe_sha256",
    )?;
    Ok(format!(
        "vcet-temporal-final:{}",
        universe_hex
            .strip_prefix("sha256:")
            .expect("normalized SHA-256 is prefixed")
    ))
}

fn canonical_report_label(value: Option<String>) -> Result<Option<String>, String> {
    let value = value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if value
        .as_ref()
        .is_some_and(|value| value.len() > 128 || value.chars().any(|ch| ch.is_control()))
    {
        return Err(
            "VCET temporal report_label must be at most 128 bytes and contain no control characters"
                .to_string(),
        );
    }
    Ok(value)
}

fn validate_product_position_boundary_policy(
    policy: VcetTemporalPositionBoundaryPolicyV1,
) -> Result<(), String> {
    if policy != VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen {
        return Err(
            "VCET temporal product currently requires position_boundary_policy=reset_flat_discard_open"
                .to_string(),
        );
    }
    Ok(())
}

fn build_single_dataset_planning_request(
    manifest: &super::commands_vcet_temporal::VcetTemporalSealedCandidateManifestV1,
    plan: &VcetTemporalValidationPlanV1,
    dataset_id: &str,
    source_dataset_sha256: &str,
    timestamps_utc_ms: Vec<i64>,
) -> Result<PlanVcetTemporalValidationRequestV1, String> {
    Ok(PlanVcetTemporalValidationRequestV1 {
        sealed_candidate_manifest: manifest.clone(),
        expected_candidate_manifest_sha256: manifest.manifest_sha256.clone(),
        plan: plan.clone(),
        expected_plan_identity: vcet_temporal_plan_identity(plan)
            .map_err(|error| error.to_string())?,
        datasets: vec![VcetTemporalDatasetTimelineInputV1 {
            dataset_id: dataset_id.to_string(),
            source_dataset_sha256: source_dataset_sha256.to_string(),
            timestamps_utc_ms,
            session_calendar_id: None,
            session_open_utc_ms: None,
        }],
    })
}

fn source_artifact_hash_matches(expected: &str, actual: &str) -> Result<bool, String> {
    Ok(normalize_sha256(expected, "source run artifact SHA-256")?
        == normalize_sha256(actual, "loaded source run artifact SHA-256")?)
}

fn source_item_is_complete(item: &super::commands_campaign::ResearchCampaignItemSummary) -> bool {
    !item.status.trim().eq_ignore_ascii_case("failed")
        && (item.status.trim().eq_ignore_ascii_case("ok")
            || item.evidence_state == Some(CampaignEvidenceState::Complete)
            || item.resume_state == Some(CampaignResumeState::Complete))
}

fn validate_product_source_campaign_summary_value(value: &serde_json::Value) -> Result<(), String> {
    if value.get("kind").and_then(serde_json::Value::as_str) != Some("vectorgrid_campaign_summary")
    {
        return Err(
            "VCET temporal product preparation requires a native vectorgrid_campaign_summary"
                .to_string(),
        );
    }
    validate_campaign_summary_artifact_value(value)
}

fn normalized_source_path(raw: &str) -> String {
    raw.trim().replace('/', "\\").to_ascii_lowercase()
}

fn required_artifact_text<'a>(
    artifact: &'a serde_json::Value,
    pointer: &str,
    label: &str,
) -> Result<&'a str, String> {
    artifact
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("source VCET artifact is missing {label}"))
}

fn validate_source_artifact_dataset_lineage(
    artifact: &serde_json::Value,
    summary: &ResearchCampaignSummary,
    item: &super::commands_campaign::ResearchCampaignItemSummary,
    input: &ResearchCampaignDatasetInput,
) -> Result<(), String> {
    if artifact.get("kind").and_then(serde_json::Value::as_str) != Some("vectorgrid_run_artifact")
        || artifact.get("version").and_then(serde_json::Value::as_u64) != Some(1)
    {
        return Err(format!(
            "source VCET artifact for '{}' is not a native run artifact v1",
            item.label
        ));
    }
    let manifest = summary
        .manifest
        .as_ref()
        .ok_or_else(|| "source campaign summary is missing its manifest".to_string())?;
    let artifact_manifest = required_artifact_text(
        artifact,
        "/campaign/manifest_hash",
        "campaign.manifest_hash",
    )?;
    if normalize_sha256(artifact_manifest, "source artifact campaign.manifest_hash")?
        != normalize_sha256(&manifest.manifest_hash, "source campaign manifest_hash")?
    {
        return Err(format!(
            "source VCET artifact for '{}' belongs to a different campaign manifest",
            item.label
        ));
    }
    let retention_cohort =
        required_artifact_text(artifact, "/retention/cohort_id", "retention.cohort_id")?;
    if normalize_sha256(retention_cohort, "source artifact retention.cohort_id")?
        != normalize_sha256(&manifest.manifest_hash, "source campaign manifest_hash")?
    {
        return Err(format!(
            "source VCET artifact for '{}' belongs to a different retained campaign cohort",
            item.label
        ));
    }
    let expected_role = if item.is_holdout {
        "holdout"
    } else {
        "training"
    };
    if required_artifact_text(artifact, "/campaign/cohort_role", "campaign.cohort_role")?
        != expected_role
    {
        return Err(format!(
            "source VCET artifact for '{}' has the wrong campaign cohort role",
            item.label
        ));
    }

    if required_artifact_text(
        artifact,
        "/validation/req/evaluation_mode",
        "validation.req.evaluation_mode",
    )? != "evaluate_candidates"
        || required_artifact_text(
            artifact,
            "/run/res/evaluation_mode",
            "run.res.evaluation_mode",
        )? != "evaluate_candidates"
        || required_artifact_text(
            artifact,
            "/validation/req/validation_context/strategy_kind",
            "validation.req.validation_context.strategy_kind",
        )? != "Vcet"
    {
        return Err(format!(
            "source VCET artifact for '{}' does not carry the evaluate-candidates execution context",
            item.label
        ));
    }
    let run_data_id = required_artifact_text(artifact, "/run/req/data_id", "run.req.data_id")?;
    if run_data_id
        != required_artifact_text(
            artifact,
            "/validation/req/data_id",
            "validation.req.data_id",
        )?
    {
        return Err(format!(
            "source VCET artifact for '{}' has inconsistent run/validation data IDs",
            item.label
        ));
    }

    let expected_path = normalized_source_path(&input.csv_path);
    if expected_path.is_empty()
        || expected_path != normalized_source_path(&item.csv_path)
        || expected_path
            != normalized_source_path(required_artifact_text(
                artifact,
                "/data/csv_path",
                "data.csv_path",
            )?)
        || expected_path
            != normalized_source_path(required_artifact_text(
                artifact,
                "/validation/req/validation_context/csv_path",
                "validation.req.validation_context.csv_path",
            )?)
        || expected_path
            != normalized_source_path(required_artifact_text(
                artifact,
                "/dataset_meta/csv_path",
                "dataset_meta.csv_path",
            )?)
    {
        return Err(format!(
            "source VCET artifact for '{}' does not match the requested dataset path",
            item.label
        ));
    }

    let identity = item.dataset_identity.as_ref().ok_or_else(|| {
        format!(
            "source campaign item '{}' is missing its completed dataset identity",
            item.label
        )
    })?;
    if expected_path != normalized_source_path(&identity.csv_path) {
        return Err(format!(
            "source campaign item '{}' has inconsistent dataset identity path",
            item.label
        ));
    }
    let context_timeframe = required_artifact_text(
        artifact,
        "/validation/req/validation_context/timeframe",
        "validation.req.validation_context.timeframe",
    )?;
    let context_label = required_artifact_text(
        artifact,
        "/validation/req/validation_context/label",
        "validation.req.validation_context.label",
    )?;
    if !context_timeframe.eq_ignore_ascii_case(item.timeframe.trim())
        || context_label != item.label.trim()
    {
        return Err(format!(
            "source VCET artifact for '{}' has foreign validation dataset context",
            item.label
        ));
    }
    let artifact_size = artifact
        .pointer("/dataset_meta/source_size_bytes")
        .and_then(serde_json::Value::as_u64);
    let artifact_mtime = artifact
        .pointer("/dataset_meta/source_mtime_utc_ms")
        .and_then(serde_json::Value::as_i64);
    let artifact_change_marker = artifact
        .pointer("/dataset_meta/source_change_marker")
        .and_then(serde_json::Value::as_i64);
    let artifact_fingerprint = artifact
        .pointer("/dataset_meta/source_content_fingerprint")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let identity_fingerprint = identity
        .source_content_fingerprint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if identity.source_size_bytes.is_none()
        || identity.source_mtime_utc_ms.is_none()
        || identity.source_change_marker.is_none()
        || identity_fingerprint.is_none()
        || artifact_size != identity.source_size_bytes
        || artifact_mtime != identity.source_mtime_utc_ms
        || artifact_change_marker != identity.source_change_marker
        || artifact_fingerprint != identity_fingerprint
        || input
            .source_size_bytes
            .is_some_and(|value| Some(value) != identity.source_size_bytes)
        || input
            .source_mtime_utc_ms
            .is_some_and(|value| Some(value) != identity.source_mtime_utc_ms)
        || input
            .source_change_marker
            .is_some_and(|value| Some(value) != identity.source_change_marker)
        || input
            .source_content_fingerprint
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| Some(value) != identity_fingerprint)
    {
        return Err(format!(
            "source VCET artifact for '{}' does not match its exact dataset source identity",
            item.label
        ));
    }
    Ok(())
}

fn verify_source_campaign_artifacts<R: tauri::Runtime>(
    summary: &ResearchCampaignSummary,
    source_datasets: &[ResearchCampaignDatasetInput],
    app: &tauri::AppHandle<R>,
) -> Result<Vec<(String, String, String)>, String> {
    if source_datasets.len() != summary.items.len() {
        return Err(
            "source campaign request/item dataset counts differ during artifact verification"
                .to_string(),
        );
    }
    let contract = build_vcet_fixed_candidate_artifact_contract(summary)?;
    let mut verified = Vec::with_capacity(summary.items.len());
    let mut training_artifacts = Vec::new();
    let mut training_artifact_bytes = 0u64;
    for (item, input) in summary.items.iter().zip(source_datasets) {
        if !source_item_is_complete(item) {
            return Err(format!(
                "VCET temporal preparation rejected incomplete source campaign item '{}' ({})",
                item.label, item.status
            ));
        }
        let item_id = item
            .item_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!(
                    "VCET temporal preparation requires a stable item_id for '{}'",
                    item.label
                )
            })?
            .to_string();
        let path = item
            .artifact_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!(
                    "VCET temporal preparation requires a run artifact for '{}'",
                    item.label
                )
            })?
            .to_string();
        let expected_sha256 = item
            .artifact_sha256
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!(
                    "VCET temporal preparation requires a sealed run-artifact SHA-256 for '{}'",
                    item.label
                )
            })?;
        let (artifact, observed_sha256, artifact_bytes, _authenticated_artifact_path) =
            run_artifacts::load_artifact_value_checked_bounded_with_sha256_blocking(
                path.clone(),
                MAX_VCET_TEMPORAL_PRODUCT_SOURCE_TRAINING_ARTIFACT_BYTES,
                app.clone(),
            )?;
        if !source_artifact_hash_matches(expected_sha256, &observed_sha256)? {
            return Err(format!(
                "VCET temporal preparation rejected source run-artifact SHA-256 drift for '{}'",
                item.label
            ));
        }
        validate_vcet_fixed_candidate_artifact_contract(&artifact, &contract).map_err(|error| {
            format!(
                "VCET temporal preparation rejected source run artifact for '{}': {error}",
                item.label
            )
        })?;
        validate_vcet_fixed_candidate_artifact_request_contract(&artifact, &contract).map_err(
            |error| {
                format!(
                    "VCET temporal preparation rejected source request envelope for '{}': {error}",
                    item.label
                )
            },
        )?;
        validate_source_artifact_dataset_lineage(&artifact, summary, item, input)?;
        if !item.is_holdout {
            training_artifact_bytes = training_artifact_bytes
                .checked_add(artifact_bytes)
                .ok_or_else(|| "VCET training artifact byte count overflow".to_string())?;
            if training_artifact_bytes > MAX_VCET_TEMPORAL_PRODUCT_SOURCE_TRAINING_ARTIFACT_BYTES {
                return Err(format!(
                    "exact VCET training artifacts require {training_artifact_bytes} raw parsed bytes, exceeding the {MAX_VCET_TEMPORAL_PRODUCT_SOURCE_TRAINING_ARTIFACT_BYTES} byte product limit"
                ));
            }
            training_artifacts.push(artifact);
        }
        verified.push((
            item_id,
            path,
            normalize_sha256(&observed_sha256, "source run artifact SHA-256")?,
        ));
    }
    validate_vcet_fixed_candidate_training_evidence(summary, training_artifacts.iter())?;
    Ok(verified)
}

fn prepare_vcet_temporal_campaign_blocking<R: tauri::Runtime>(
    req: PrepareVcetTemporalCampaignRequestV1,
    app: tauri::AppHandle<R>,
    app_local_data_dir: &Path,
) -> Result<VcetTemporalCampaignStatusV1, String> {
    let source_path_input = req.source_campaign_summary_path.trim();
    if source_path_input.is_empty() {
        return Err("source_campaign_summary_path must not be empty".to_string());
    }
    let source_path_for_load = source_path_input.to_string();
    let (summary_value, summary_file_sha256, _summary_file_bytes, authenticated_source_path) =
        run_artifacts::load_artifact_value_checked_bounded_with_sha256_blocking(
            source_path_for_load,
            MAX_PRODUCT_ARTIFACT_READ_BYTES,
            app.clone(),
        )?;
    // Bind lineage to the final canonical identity resolved from the same open
    // handle whose bytes were parsed and hashed. Never reopen or recanonicalize
    // the caller-supplied pathname after that authenticated read.
    let source_path = authenticated_source_path.to_string_lossy().to_string();
    validate_product_source_campaign_summary_value(&summary_value)?;
    let summary: ResearchCampaignSummary = serde_json::from_value(summary_value)
        .map_err(|error| format!("decode source campaign summary: {error}"))?;
    if summary.version != 2 {
        return Err("VCET temporal product preparation requires campaign summary v2".to_string());
    }
    if summary.items.is_empty() || summary.items.len() > MAX_VCET_TEMPORAL_PRODUCT_DATASETS {
        return Err(format!(
            "VCET temporal product preparation requires between 1 and {MAX_VCET_TEMPORAL_PRODUCT_DATASETS} source datasets"
        ));
    }
    if summary.items.iter().all(|item| item.is_holdout)
        || summary.items.iter().all(|item| !item.is_holdout)
    {
        return Err(
            "VCET temporal product preparation requires non-empty selection and final-test cohorts"
                .to_string(),
        );
    }

    let source = build_vcet_temporal_campaign_candidate_source(&summary)?;
    if source.request.datasets.len() != summary.items.len() {
        return Err(
            "source campaign request/item dataset counts differ after integrity validation"
                .to_string(),
        );
    }
    let candidate_count = source.sealed_candidate_manifest.payload.candidates.len();
    let evidence_cells = candidate_count
        .checked_mul(summary.items.len())
        .ok_or_else(|| "VCET temporal product evidence-cell count overflow".to_string())?;
    if evidence_cells > MAX_VCET_TEMPORAL_PRODUCT_EVIDENCE_CELLS {
        return Err(format!(
            "VCET temporal product requires {evidence_cells} dataset/candidate evidence cells, exceeding the {MAX_VCET_TEMPORAL_PRODUCT_EVIDENCE_CELLS} limit"
        ));
    }
    validate_product_position_boundary_policy(req.plan.position_boundary_policy)?;
    let plan_identity = vcet_temporal_plan_identity(&req.plan)
        .map_err(|error| format!("invalid VCET temporal validation plan: {error}"))?;
    let report_label = canonical_report_label(req.report_label)?;
    let verified_artifacts =
        verify_source_campaign_artifacts(&summary, &source.request.datasets, &app)?;
    let app_cache_dir = data_cache::app_data_cache_dir(app_local_data_dir);

    let mut dataset_specs = Vec::with_capacity(summary.items.len());
    let mut universe_datasets = Vec::with_capacity(summary.items.len());
    let mut total_timeline_value_count = 0usize;
    for (index, ((input, item), item_artifact)) in source
        .request
        .datasets
        .iter()
        .cloned()
        .zip(summary.items.iter())
        .zip(verified_artifacts.iter())
        .enumerate()
    {
        let (item_id, source_run_artifact_path, source_run_artifact_sha256) = item_artifact;
        let prepared = prepare_campaign_dataset(
            input,
            source.request.timeframe_override.as_deref(),
            &app_cache_dir,
        )
        .map_err(|error| {
            format!(
                "prepare VCET temporal dataset {} ('{}'): {error}",
                index + 1,
                item.label
            )
        })?;
        validate_loaded_dataset_shape(item_id, &prepared.candles)?;
        let observed_bar_count = prepared.candles.timestamp.len();
        if observed_bar_count > MAX_VCET_TEMPORAL_PRODUCT_TIMELINE_VALUES_PER_DATASET {
            return Err(format!(
                "VCET temporal dataset '{}' contains {observed_bar_count} timeline values, exceeding the {MAX_VCET_TEMPORAL_PRODUCT_TIMELINE_VALUES_PER_DATASET} per-dataset product limit",
                item.label
            ));
        }
        total_timeline_value_count = total_timeline_value_count
            .checked_add(observed_bar_count)
            .ok_or_else(|| "VCET temporal aggregate timeline count overflow".to_string())?;
        if total_timeline_value_count > MAX_VCET_TEMPORAL_PRODUCT_TOTAL_TIMELINE_VALUES {
            return Err(format!(
                "VCET temporal product aggregate timeline count {total_timeline_value_count} exceeds the {MAX_VCET_TEMPORAL_PRODUCT_TOTAL_TIMELINE_VALUES} limit"
            ));
        }
        let source_dataset_sha256 = source_dataset_sha256(
            item_id,
            &prepared.input,
            &prepared.timeframe,
            source_run_artifact_sha256,
        )?;
        let planning_request = build_single_dataset_planning_request(
            &source.sealed_candidate_manifest,
            &req.plan,
            item_id,
            &source_dataset_sha256,
            prepared.candles.timestamp.clone(),
        )?;
        let planning_evidence = plan_vcet_temporal_validation_blocking(planning_request)?;
        let planned = planning_evidence
            .datasets
            .first()
            .ok_or_else(|| "single-dataset planning returned no dataset evidence".to_string())?;
        let loaded_candles_snapshot_sha256 = loaded_candles_snapshot_sha256(&prepared.candles);
        let role = if item.is_holdout {
            VcetTemporalDatasetCohortRoleV1::FinalTest
        } else {
            VcetTemporalDatasetCohortRoleV1::Selection
        };
        let dataset_attestation_sha256 = recompute_vcet_dataset_attestation_sha256(
            item_id,
            &source_dataset_sha256,
            &loaded_candles_snapshot_sha256,
            role,
        )?;
        let coverage_request = build_single_dataset_planning_request(
            &source.sealed_candidate_manifest,
            &req.plan,
            item_id,
            &source_dataset_sha256,
            prepared.candles.timestamp.clone(),
        )?;
        let dataset_coverage = profile_loaded_dataset_coverage_for_planning(
            &coverage_request,
            item_id,
            &prepared.candles,
        )?;
        let dataset_coverage_sha256 = recompute_vcet_dataset_coverage_sha256(
            item_id,
            &loaded_candles_snapshot_sha256,
            &planned.timeline_sha256,
            &dataset_coverage,
        )?;
        let temporal_interval_fingerprint_sha256 =
            recompute_vcet_temporal_interval_fingerprint_sha256(&planned.layout)?;
        universe_datasets.push(VcetTemporalUniverseDatasetV1 {
            dataset_id: item_id.clone(),
            source_dataset_sha256: source_dataset_sha256.clone(),
            loaded_candles_snapshot_sha256: loaded_candles_snapshot_sha256.clone(),
            dataset_attestation_sha256: dataset_attestation_sha256.clone(),
            timeline_sha256: planned.timeline_sha256.clone(),
            temporal_interval_fingerprint_sha256,
            dataset_coverage,
            dataset_coverage_sha256: dataset_coverage_sha256.clone(),
            fold_count: planned.fold_count,
            role,
        });
        dataset_specs.push(VcetTemporalCampaignDatasetSpecV1 {
            dataset_id: item_id.clone(),
            role,
            input: prepared.input,
            source_run_artifact_path: source_run_artifact_path.clone(),
            source_run_artifact_sha256: source_run_artifact_sha256.clone(),
            source_dataset_sha256,
            loaded_candles_snapshot_sha256,
            timeline_sha256: planned.timeline_sha256.clone(),
            dataset_attestation_sha256,
            dataset_coverage_sha256,
            observed_bar_count,
        });
    }

    let campaign_admission = source.cross_universe_admission;
    let execution_spec = VcetTemporalPhasedExecutionSpecV1 {
        backend: source.optimization_request.backend,
        ma_source: source.optimization_request.ma_source,
        strategy: source.optimization_request.strategy,
        selection_policy: VcetTemporalSelectionPolicyV1::MeanValidationSharpe,
        selection_reduction_contract: VCET_TEMPORAL_SELECTION_REDUCTION_CONTRACT_V1.to_string(),
        cross_universe_admission: VcetTemporalCrossUniverseAdmissionV1 {
            schema_version: VCET_TEMPORAL_CROSS_UNIVERSE_ADMISSION_SCHEMA_VERSION,
            min_requested_dataset_coverage_share: 1.0,
            min_evaluated_dataset_share_requested: 1.0,
            min_profitable_dataset_share: campaign_admission.min_profitable_share_requested,
            min_hard_support_dataset_share: campaign_admission.min_hard_gate_pass_share_requested,
            min_metric_support_dataset_share: campaign_admission.min_hard_gate_pass_share_requested,
            min_validation_pnl_p10: campaign_admission.min_pnl_p10,
            min_validation_sharpe_p10: campaign_admission.min_sharpe_p10,
        },
    };
    let expected_candidate_manifest_sha256 =
        source.sealed_candidate_manifest.manifest_sha256.clone();
    let sealed_universe =
        seal_vcet_temporal_dataset_universe_blocking(SealVcetTemporalDatasetUniverseRequestV1 {
            sealed_candidate_manifest: source.sealed_candidate_manifest,
            expected_candidate_manifest_sha256,
            plan: req.plan,
            expected_plan_identity: plan_identity,
            execution_spec,
            shard_policy: VcetTemporalShardPolicyV1 {
                max_datasets_per_shard: 1,
            },
            datasets: universe_datasets,
        })?
        .sealed_universe;
    let fold_count = sealed_universe.payload.fold_count_per_dataset;
    let aggregate_trials = sealed_universe
        .payload
        .selection_dataset_count
        .checked_mul(candidate_count)
        .and_then(|value| value.checked_mul(fold_count))
        .and_then(|value| value.checked_mul(2))
        .and_then(|value| value.checked_add(sealed_universe.payload.final_test_dataset_count))
        .ok_or_else(|| "VCET temporal aggregate candidate-window trial overflow".to_string())?;
    if aggregate_trials > MAX_VCET_TEMPORAL_PRODUCT_CANDIDATE_WINDOW_TRIALS {
        return Err(format!(
            "VCET temporal product requires {aggregate_trials} aggregate candidate-window trials, exceeding the {MAX_VCET_TEMPORAL_PRODUCT_CANDIDATE_WINDOW_TRIALS} limit"
        ));
    }

    let source_campaign_manifest_sha256 = sealed_universe
        .payload
        .candidate_manifest
        .payload
        .source_campaign_manifest_hash
        .clone();
    let source_candidate_manifest_sha256 = sealed_universe
        .payload
        .candidate_manifest
        .payload
        .source_candidate_manifest_hash
        .clone();
    let mut payload = VcetTemporalCampaignPreparationPayloadV1 {
        schema_version: VCET_TEMPORAL_PRODUCT_CAMPAIGN_SCHEMA_VERSION,
        evidence_kind: "vcet_temporal_product_preparation".to_string(),
        campaign_id: String::new(),
        source_campaign_summary_path: source_path,
        source_campaign_summary_sha256: normalize_sha256(
            &summary_file_sha256,
            "source campaign summary SHA-256",
        )?,
        source_campaign_manifest_sha256,
        source_candidate_manifest_sha256,
        report_label,
        timeframe_override: source.request.timeframe_override,
        datasets: dataset_specs,
        sealed_universe,
        total_timeline_value_count,
        limitations: product_limitations(),
    };
    let identity = campaign_identity_sha256(&payload)?;
    payload.campaign_id = format!(
        "vcet-temporal-product-v1-{}",
        identity
            .strip_prefix("sha256:")
            .expect("internal SHA-256 is prefixed")
            .chars()
            .take(24)
            .collect::<String>()
    );
    let artifact_sha256 = preparation_sha256(&payload)?;
    let artifact = VcetTemporalCampaignPreparationArtifactV1 {
        payload,
        artifact_sha256: artifact_sha256.clone(),
    };
    validate_preparation_artifact(&artifact, &artifact_sha256)?;
    let dir = campaign_dir(app_local_data_dir, &artifact.payload.campaign_id)?;
    let path = preparation_artifact_path(&dir);
    if let Some(existing) =
        read_optional_json_file::<VcetTemporalCampaignPreparationArtifactV1>(&path, "preparation")?
    {
        validate_loaded_preparation(&existing, &path, &artifact_sha256)?;
        let existing_bytes = serde_json::to_vec(&existing)
            .map_err(|error| format!("encode existing VCET preparation: {error}"))?;
        let replay_bytes = serde_json::to_vec(&artifact)
            .map_err(|error| format!("encode replayed VCET preparation: {error}"))?;
        if existing_bytes != replay_bytes {
            return Err(
                "existing VCET temporal preparation differs from deterministic replay".to_string(),
            );
        }
    } else {
        publish_new_json(&path, &artifact)?;
    }
    campaign_status_from_preparation(&artifact, &path, app_local_data_dir)
}

fn dataset_spec<'a>(
    preparation: &'a VcetTemporalCampaignPreparationArtifactV1,
    dataset_id: &str,
) -> Result<&'a VcetTemporalCampaignDatasetSpecV1, String> {
    preparation
        .payload
        .datasets
        .iter()
        .find(|dataset| dataset.dataset_id == dataset_id)
        .ok_or_else(|| format!("VCET temporal campaign has no prepared dataset '{dataset_id}'"))
}

fn load_product_dataset(
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    spec: &VcetTemporalCampaignDatasetSpecV1,
    app_local_data_dir: &Path,
) -> Result<LoadedProductDataset, String> {
    let prepared = prepare_campaign_dataset(
        spec.input.clone(),
        preparation.payload.timeframe_override.as_deref(),
        &data_cache::app_data_cache_dir(app_local_data_dir),
    )?;
    validate_loaded_dataset_shape(&spec.dataset_id, &prepared.candles)?;
    if prepared.candles.timestamp.len() != spec.observed_bar_count {
        return Err(format!(
            "VCET temporal dataset '{}' observed bar count changed after sealing",
            spec.dataset_id
        ));
    }
    let source_sha256 = source_dataset_sha256(
        &spec.dataset_id,
        &prepared.input,
        &prepared.timeframe,
        &spec.source_run_artifact_sha256,
    )?;
    if source_sha256 != spec.source_dataset_sha256 {
        return Err(format!(
            "VCET temporal dataset '{}' source lineage changed after sealing",
            spec.dataset_id
        ));
    }
    let snapshot_sha256 = loaded_candles_snapshot_sha256(&prepared.candles);
    if snapshot_sha256 != spec.loaded_candles_snapshot_sha256 {
        return Err(format!(
            "VCET temporal dataset '{}' loaded OHLCV snapshot changed after sealing",
            spec.dataset_id
        ));
    }

    let manifest = &preparation
        .payload
        .sealed_universe
        .payload
        .candidate_manifest;
    let plan = &preparation.payload.sealed_universe.payload.plan;
    let planning_for_evidence = build_single_dataset_planning_request(
        manifest,
        plan,
        &spec.dataset_id,
        &spec.source_dataset_sha256,
        prepared.candles.timestamp.clone(),
    )?;
    let planning_evidence = plan_vcet_temporal_validation_blocking(planning_for_evidence)?;
    let planned = planning_evidence
        .datasets
        .first()
        .ok_or_else(|| "single-dataset planning returned no evidence".to_string())?;
    if planned.timeline_sha256 != spec.timeline_sha256
        || planning_evidence.evidence_sha256.trim().is_empty()
    {
        return Err(format!(
            "VCET temporal dataset '{}' planning evidence changed after sealing",
            spec.dataset_id
        ));
    }
    let planning_request = build_single_dataset_planning_request(
        manifest,
        plan,
        &spec.dataset_id,
        &spec.source_dataset_sha256,
        prepared.candles.timestamp.clone(),
    )?;
    let binding = VcetTemporalLoadedDatasetBindingV1 {
        dataset_id: spec.dataset_id.clone(),
        loaded_data_id: format!("{}:loaded", spec.dataset_id),
    };
    Ok(LoadedProductDataset {
        planning_request,
        planning_evidence,
        binding,
        candles: Arc::new(prepared.candles),
    })
}

#[cfg(test)]
fn load_pre_final_shard_artifact(
    path: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    expected_shard: &super::commands_vcet_temporal_universe::VcetTemporalUniverseShardV1,
) -> Result<ExecuteVcetTemporalPreFinalShardResponseV1, String> {
    let artifact: ExecuteVcetTemporalPreFinalShardResponseV1 =
        read_json_file(path, MAX_PRODUCT_ARTIFACT_READ_BYTES)?;
    validate_pre_final_shard_for_campaign(&artifact, preparation, expected_shard)?;
    Ok(artifact)
}

fn validate_pre_final_shard_for_campaign(
    artifact: &ExecuteVcetTemporalPreFinalShardResponseV1,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    expected_shard: &super::commands_vcet_temporal_universe::VcetTemporalUniverseShardV1,
) -> Result<(), String> {
    validate_vcet_temporal_pre_final_shard_artifact(&artifact)?;
    if artifact.payload.universe_sha256 != preparation.payload.sealed_universe.universe_sha256
        || artifact.payload.selection_shard != *expected_shard
    {
        return Err(format!(
            "persisted VCET pre-final shard {} does not match the prepared campaign",
            expected_shard.shard_id
        ));
    }
    Ok(())
}

#[cfg(test)]
fn load_pre_final_reduction_artifact(
    path: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
) -> Result<ReduceVcetTemporalPreFinalShardsResponseV1, String> {
    let artifact: ReduceVcetTemporalPreFinalShardsResponseV1 =
        read_json_file(path, MAX_PRODUCT_ARTIFACT_READ_BYTES)?;
    validate_pre_final_reduction_for_campaign(&artifact, preparation)?;
    Ok(artifact)
}

fn validate_pre_final_reduction_for_campaign(
    artifact: &ReduceVcetTemporalPreFinalShardsResponseV1,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
) -> Result<(), String> {
    validate_vcet_temporal_pre_final_reduction_artifact(artifact, &artifact.artifact_sha256)?;
    validate_vcet_temporal_pre_final_reduction_against_universe(
        artifact,
        &preparation.payload.sealed_universe,
    )?;
    if artifact.payload.final_stage_nonce != final_stage_nonce(preparation)? {
        return Err(
            "persisted VCET pre-final reduction has a foreign final-stage nonce".to_string(),
        );
    }
    Ok(())
}

fn retryable_durable_probe_error(stage: &str, _error: DurableProbeUnavailable) -> String {
    format!("VCET temporal {stage} durable state is temporarily unavailable; retry the operation")
}

fn probe_pre_final_reduction_from_durable_terminal(
    app_local_data_dir: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
) -> Result<DurableCompletionProbe<ReduceVcetTemporalPreFinalShardsResponseV1>, String> {
    let probe = probe_durable_pre_final_winner_lock::<ReduceVcetTemporalPreFinalShardsResponseV1>(
        app_local_data_dir,
        &preparation.payload.sealed_universe.universe_sha256,
    )
    .map_err(|error| retryable_durable_probe_error("pre-final winner-lock", error))?;
    match probe {
        DurableCompletionProbe::Completed {
            request_sha256,
            response,
        } => {
            if validate_pre_final_reduction_for_campaign(&response, preparation).is_err() {
                Ok(DurableCompletionProbe::Blocked(
                    DurableProbeBlockedReason::Invalid,
                ))
            } else {
                Ok(DurableCompletionProbe::Completed {
                    request_sha256,
                    response,
                })
            }
        }
        other => Ok(other),
    }
}

#[cfg(test)]
fn load_final_shard_artifact(
    path: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    expected_shard: &super::commands_vcet_temporal_universe::VcetTemporalUniverseShardV1,
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
) -> Result<ExecuteVcetTemporalFinalTestShardResponseV1, String> {
    let artifact: ExecuteVcetTemporalFinalTestShardResponseV1 =
        read_json_file(path, MAX_PRODUCT_ARTIFACT_READ_BYTES)?;
    validate_final_shard_for_campaign(&artifact, preparation, expected_shard, pre_final)?;
    Ok(artifact)
}

fn validate_final_shard_for_campaign(
    artifact: &ExecuteVcetTemporalFinalTestShardResponseV1,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    expected_shard: &super::commands_vcet_temporal_universe::VcetTemporalUniverseShardV1,
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
) -> Result<(), String> {
    validate_vcet_temporal_final_shard_artifact(artifact)?;
    let authorization = pre_final
        .payload
        .final_shard_authorizations
        .iter()
        .find(|authorization| authorization.shard.shard_id == expected_shard.shard_id)
        .ok_or_else(|| {
            format!(
                "persisted VCET final-test shard {} has no current winner authorization",
                expected_shard.shard_id
            )
        })?;
    if artifact.payload.universe_sha256 != preparation.payload.sealed_universe.universe_sha256
        || artifact.payload.final_shard != *expected_shard
        || authorization.shard != *expected_shard
        || artifact.payload.winner_lock.winner_lock_sha256
            != pre_final.payload.winner_lock.winner_lock_sha256
        || artifact.payload.global_winner_reduction_artifact_sha256 != pre_final.artifact_sha256
        || artifact.payload.global_pre_final_evidence_sha256
            != pre_final.payload.global_pre_final_evidence_sha256
        || artifact.payload.final_stage_nonce != pre_final.payload.final_stage_nonce
        || artifact.payload.authorization_sha256 != authorization.authorization_sha256
        || artifact.payload.authorized_prior_state_sha256
            != authorization.expected_prior_state_sha256
    {
        return Err(format!(
            "persisted VCET final-test shard {} does not match the prepared winner authority",
            expected_shard.shard_id
        ));
    }
    Ok(())
}

fn probe_final_shard_from_durable_terminal(
    app_local_data_dir: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    expected_shard: &super::commands_vcet_temporal_universe::VcetTemporalUniverseShardV1,
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
    authorization_sha256: &str,
) -> Result<DurableCompletionProbe<ExecuteVcetTemporalFinalTestShardResponseV1>, String> {
    let probe = probe_durable_final_completion_for_authorization::<
        ExecuteVcetTemporalFinalTestShardResponseV1,
    >(app_local_data_dir, authorization_sha256)
    .map_err(|error| retryable_durable_probe_error("final-test shard", error))?;
    match probe {
        DurableCompletionProbe::Completed {
            request_sha256,
            response,
        } => {
            if response.payload.request_sha256 != request_sha256
                || validate_final_shard_for_campaign(
                    &response,
                    preparation,
                    expected_shard,
                    pre_final,
                )
                .is_err()
            {
                Ok(DurableCompletionProbe::Blocked(
                    DurableProbeBlockedReason::Invalid,
                ))
            } else {
                Ok(DurableCompletionProbe::Completed {
                    request_sha256,
                    response,
                })
            }
        }
        other => Ok(other),
    }
}

#[cfg(test)]
fn recover_all_before_any_new_execution<T, R, E>(
    item_count: usize,
    mut recover: R,
    mut execute: E,
) -> Result<Vec<T>, String>
where
    R: FnMut(usize) -> Result<Option<T>, String>,
    E: FnMut(usize) -> Result<T, String>,
{
    let mut recovered = Vec::with_capacity(item_count);
    let mut first_recovery_error = None;
    for index in 0..item_count {
        match recover(index) {
            Ok(value) => recovered.push(value),
            Err(error) => {
                if first_recovery_error.is_none() {
                    first_recovery_error = Some(error);
                }
                recovered.push(None);
            }
        }
    }
    if let Some(error) = first_recovery_error {
        return Err(error);
    }
    for (index, artifact) in recovered.iter_mut().enumerate() {
        if artifact.is_none() {
            *artifact = Some(execute(index)?);
        }
    }
    recovered
        .into_iter()
        .enumerate()
        .map(|(index, artifact)| {
            artifact.ok_or_else(|| {
                format!("VCET temporal recovery left final shard index {index} unresolved")
            })
        })
        .collect()
}

#[cfg(test)]
fn load_final_reduction_artifact(
    path: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
) -> Result<ReduceVcetTemporalFinalTestShardsResponseV1, String> {
    let artifact: ReduceVcetTemporalFinalTestShardsResponseV1 =
        read_json_file(path, MAX_PRODUCT_ARTIFACT_READ_BYTES)?;
    validate_final_reduction_for_campaign(&artifact, preparation, pre_final)?;
    Ok(artifact)
}

fn validate_final_reduction_for_campaign(
    artifact: &ReduceVcetTemporalFinalTestShardsResponseV1,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
) -> Result<(), String> {
    validate_vcet_temporal_final_reduction_artifact(&artifact)?;
    if artifact.payload.universe_sha256 != preparation.payload.sealed_universe.universe_sha256
        || artifact.payload.winner_lock.winner_lock_sha256
            != pre_final.payload.winner_lock.winner_lock_sha256
        || artifact.payload.global_winner_reduction_artifact_sha256 != pre_final.artifact_sha256
        || artifact.payload.global_pre_final_evidence_sha256
            != pre_final.payload.global_pre_final_evidence_sha256
        || artifact.payload.final_stage_nonce != pre_final.payload.final_stage_nonce
    {
        return Err(
            "persisted VCET final-test reduction belongs to a different winner authority"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_complete_pre_final_shard_files(
    dir: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    expected_shards: &[super::commands_vcet_temporal_universe::VcetTemporalUniverseShardV1],
    reduction: &ReduceVcetTemporalPreFinalShardsResponseV1,
) -> Result<(), String> {
    if reduction.payload.ordered_shards.len() != expected_shards.len() {
        return Err(
            "VCET pre-final reduction does not cover every immutable selection shard".to_string(),
        );
    }
    for shard in expected_shards {
        let path = pre_final_shard_path(dir, shard.shard_ordinal);
        let artifact = read_optional_json_file::<ExecuteVcetTemporalPreFinalShardResponseV1>(
            &path,
            "selection shard",
        )?
        .ok_or_else(|| {
            format!(
                "VCET pre-final reduction exists without immutable selection shard '{}'",
                shard.shard_id
            )
        })?;
        validate_pre_final_shard_for_campaign(&artifact, preparation, shard)?;
        let evidence = reduction
            .payload
            .ordered_shards
            .iter()
            .find(|evidence| evidence.shard_id == shard.shard_id)
            .ok_or_else(|| {
                format!(
                    "VCET pre-final reduction omits immutable selection shard '{}'",
                    shard.shard_id
                )
            })?;
        if evidence.artifact_sha256 != artifact.artifact_sha256 {
            return Err(format!(
                "VCET pre-final reduction has foreign lineage for selection shard '{}'",
                shard.shard_id
            ));
        }
    }
    Ok(())
}

fn validate_complete_final_shard_files(
    dir: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    expected_shards: &[super::commands_vcet_temporal_universe::VcetTemporalUniverseShardV1],
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
    reduction: &ReduceVcetTemporalFinalTestShardsResponseV1,
) -> Result<(), String> {
    if reduction.payload.shards.len() != expected_shards.len() {
        return Err("VCET final reduction does not cover every immutable final shard".to_string());
    }
    for shard in expected_shards {
        let path = final_shard_path(dir, shard.shard_ordinal);
        let artifact = read_optional_json_file::<ExecuteVcetTemporalFinalTestShardResponseV1>(
            &path,
            "final-test shard",
        )?
        .ok_or_else(|| {
            format!(
                "VCET final reduction exists without immutable final shard '{}'",
                shard.shard_id
            )
        })?;
        validate_final_shard_for_campaign(&artifact, preparation, shard, pre_final)?;
        let evidence = reduction
            .payload
            .shards
            .iter()
            .find(|evidence| evidence.shard_id == shard.shard_id)
            .ok_or_else(|| {
                format!(
                    "VCET final reduction omits immutable final shard '{}'",
                    shard.shard_id
                )
            })?;
        if evidence.artifact_sha256 != artifact.artifact_sha256
            || evidence.authorization_sha256 != artifact.payload.authorization_sha256
            || evidence.authorized_prior_state_sha256
                != artifact.payload.authorized_prior_state_sha256
        {
            return Err(format!(
                "VCET final reduction has foreign lineage for final shard '{}'",
                shard.shard_id
            ));
        }
    }
    Ok(())
}

fn publish_or_verify_json<T>(path: &Path, value: &T, label: &str) -> Result<(), String>
where
    T: Serialize + DeserializeOwned,
{
    if let Some(existing) = read_optional_json_file::<T>(path, label)? {
        let existing = serde_json::to_vec(&existing)
            .map_err(|error| format!("encode existing {label}: {error}"))?;
        let proposed = serde_json::to_vec(value)
            .map_err(|error| format!("encode proposed {label}: {error}"))?;
        if existing != proposed {
            return Err(format!(
                "existing immutable {label} differs from deterministic replay: {}",
                path.display()
            ));
        }
        Ok(())
    } else {
        publish_new_json(path, value)
    }
}

fn preflight_app_owned_reports_dir(app_local_data_dir: &Path) -> Result<PathBuf, String> {
    ensure_regular_directory(app_local_data_dir)?;
    let reports_dir = app_local_data_dir.join("reports");
    // This must run before the shared report writer: create_dir_all would
    // otherwise follow an attacker-planted reports symlink/junction.
    ensure_regular_directory(&reports_dir)?;
    let canonical_base = fs::canonicalize(app_local_data_dir).map_err(|error| {
        format!(
            "canonicalize app local data directory {}: {error}",
            app_local_data_dir.display()
        )
    })?;
    let canonical_reports = fs::canonicalize(&reports_dir).map_err(|error| {
        format!(
            "canonicalize app-owned reports directory {}: {error}",
            reports_dir.display()
        )
    })?;
    if canonical_reports.parent() != Some(canonical_base.as_path()) {
        return Err("app-owned reports directory escaped app local data".to_string());
    }
    Ok(reports_dir)
}

fn preflight_app_owned_reports_dir_classified(
    app_local_data_dir: &Path,
) -> Result<PathBuf, ArtifactAccessFailureV1> {
    ensure_regular_directory_classified(app_local_data_dir)?;
    let reports_dir = app_local_data_dir.join("reports");
    ensure_regular_directory_classified(&reports_dir)?;
    let canonical_base =
        fs::canonicalize(app_local_data_dir).map_err(|_| ArtifactAccessFailureV1::Unavailable)?;
    let canonical_reports =
        fs::canonicalize(&reports_dir).map_err(|_| ArtifactAccessFailureV1::Unavailable)?;
    if canonical_reports.parent() != Some(canonical_base.as_path()) {
        return Err(ArtifactAccessFailureV1::Invalid);
    }
    Ok(reports_dir)
}

fn validate_app_owned_report_target(
    raw: &str,
    app_local_data_dir: &Path,
    stage: &str,
) -> Result<PathBuf, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("VCET temporal report path must not be empty".to_string());
    }
    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        return Err("VCET temporal report path must be absolute".to_string());
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("xlsx"))
    {
        return Err("VCET temporal report receipt must identify an XLSX file".to_string());
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "VCET temporal report filename is not valid UTF-8".to_string())?;
    if !commands_reports::vcet_temporal_report_filename_matches_stage(file_name, stage) {
        return Err(format!(
            "VCET temporal {stage} report filename is outside the generated report contract"
        ));
    }
    let reports_dir = preflight_app_owned_reports_dir(app_local_data_dir)?;
    let canonical_reports_dir = fs::canonicalize(&reports_dir).map_err(|error| {
        format!(
            "canonicalize app-owned reports directory {}: {error}",
            reports_dir.display()
        )
    })?;
    let parent = path
        .parent()
        .ok_or_else(|| "VCET temporal report receipt path has no parent directory".to_string())?;
    let canonical_parent = fs::canonicalize(parent).map_err(|error| {
        format!(
            "canonicalize VCET temporal report parent {}: {error}",
            parent.display()
        )
    })?;
    if canonical_parent != canonical_reports_dir {
        return Err(
            "VCET temporal report receipt must identify a direct file in the app-owned reports directory"
                .to_string(),
        );
    }
    Ok(path)
}

fn validate_app_owned_report_target_classified(
    raw: &str,
    app_local_data_dir: &Path,
    stage: &str,
) -> Result<PathBuf, ArtifactAccessFailureV1> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(ArtifactAccessFailureV1::Invalid);
    }
    let path = PathBuf::from(raw);
    if !path.is_absolute()
        || path
            .extension()
            .and_then(|value| value.to_str())
            .is_none_or(|value| !value.eq_ignore_ascii_case("xlsx"))
    {
        return Err(ArtifactAccessFailureV1::Invalid);
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ArtifactAccessFailureV1::Invalid)?;
    if !commands_reports::vcet_temporal_report_filename_matches_stage(file_name, stage) {
        return Err(ArtifactAccessFailureV1::Invalid);
    }
    let reports_dir = preflight_app_owned_reports_dir_classified(app_local_data_dir)?;
    let canonical_reports_dir =
        fs::canonicalize(&reports_dir).map_err(|_| ArtifactAccessFailureV1::Unavailable)?;
    let parent = path.parent().ok_or(ArtifactAccessFailureV1::Invalid)?;
    let canonical_parent = fs::canonicalize(parent).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ArtifactAccessFailureV1::Invalid
        } else {
            ArtifactAccessFailureV1::Unavailable
        }
    })?;
    if canonical_parent != canonical_reports_dir {
        return Err(ArtifactAccessFailureV1::Invalid);
    }
    Ok(path)
}

#[cfg(test)]
fn validate_app_owned_report_path(
    raw: &str,
    app_local_data_dir: &Path,
    stage: &str,
) -> Result<(PathBuf, u64), String> {
    let Some((canonical_report, file, _observed_bytes)) =
        open_app_owned_report_path(raw, app_local_data_dir, stage)?
    else {
        return Err("VCET temporal report artifact is missing".to_string());
    };
    let (_sha256, actual_bytes) =
        hash_bounded_open_artifact(file, MAX_PRODUCT_ARTIFACT_READ_BYTES)?;
    Ok((canonical_report, actual_bytes))
}

fn open_app_owned_report_path(
    raw: &str,
    app_local_data_dir: &Path,
    stage: &str,
) -> Result<Option<(PathBuf, fs::File, u64)>, String> {
    let path = validate_app_owned_report_target(raw, app_local_data_dir, stage)?;
    let Some((file, observed_bytes)) =
        open_bounded_regular_artifact(&path, MAX_PRODUCT_ARTIFACT_READ_BYTES)
            .map_err(|failure| artifact_access_error("report", failure))?
    else {
        return Ok(None);
    };
    let canonical_report = final_path_for_open_artifact(&file, &path)
        .map_err(|failure| artifact_access_error("report", failure))?;
    let canonical_reports = fs::canonicalize(preflight_app_owned_reports_dir(app_local_data_dir)?)
        .map_err(|_| "VCET temporal report directory is temporarily unavailable".to_string())?;
    if canonical_report.parent() != Some(canonical_reports.as_path())
        || canonical_report
            .file_name()
            .and_then(|value| value.to_str())
            .is_none_or(|file_name| {
                !commands_reports::vcet_temporal_report_filename_matches_stage(file_name, stage)
            })
    {
        return Err(
            "VCET temporal report handle resolved outside the app-owned report contract"
                .to_string(),
        );
    }
    Ok(Some((canonical_report, file, observed_bytes)))
}

fn persist_report_receipt(
    dir: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    stage: &str,
    report_path: String,
    report_bytes: usize,
    source_artifact_sha256: String,
    app_local_data_dir: &Path,
) -> Result<(), String> {
    let Some((_canonical_report_path, report_file, _metadata_bytes)) =
        open_app_owned_report_path(&report_path, app_local_data_dir, stage)?
    else {
        return Err("VCET temporal generated report artifact is missing".to_string());
    };
    let (report_sha256, observed_report_bytes) =
        hash_bounded_open_artifact(report_file, MAX_PRODUCT_ARTIFACT_READ_BYTES)?;
    if usize::try_from(observed_report_bytes).ok() != Some(report_bytes) {
        return Err(format!(
            "VCET temporal {stage} XLSX byte count differs from the generated report receipt"
        ));
    }
    let payload = VcetTemporalCampaignReportReceiptPayloadV1 {
        schema_version: VCET_TEMPORAL_PRODUCT_CAMPAIGN_SCHEMA_VERSION,
        campaign_id: preparation.payload.campaign_id.clone(),
        stage: stage.to_string(),
        report_sha256,
        // Preserve the writer-returned absolute spelling for public/open-file
        // APIs; canonicalization is used only for containment and hashing.
        report_path: report_path.trim().to_string(),
        report_bytes,
        source_artifact_sha256,
    };
    let receipt = VcetTemporalCampaignReportReceiptV1 {
        artifact_sha256: report_receipt_sha256(&payload)?,
        payload,
    };
    let file_name = match stage {
        "selection" => SELECTION_REPORT_RECEIPT_FILE_NAME,
        "final" => FINAL_REPORT_RECEIPT_FILE_NAME,
        _ => return Err("invalid VCET temporal report stage".to_string()),
    };
    publish_or_verify_json(&dir.join(file_name), &receipt, "VCET report receipt")
}

fn load_report_receipt_record(
    path: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    stage: &str,
    app_local_data_dir: &Path,
) -> Result<Option<VcetTemporalCampaignReportReceiptV1>, String> {
    let Some(receipt) = read_optional_json_file::<VcetTemporalCampaignReportReceiptV1>(
        path,
        &format!("{stage} report receipt"),
    )?
    else {
        return Ok(None);
    };
    if receipt.payload.schema_version != VCET_TEMPORAL_PRODUCT_CAMPAIGN_SCHEMA_VERSION
        || receipt.payload.campaign_id != preparation.payload.campaign_id
        || receipt.payload.stage != stage
        || receipt.artifact_sha256 != report_receipt_sha256(&receipt.payload)?
    {
        return Err(format!("invalid VCET temporal {stage} report receipt"));
    }
    validate_app_owned_report_target(&receipt.payload.report_path, app_local_data_dir, stage)?;
    Ok(Some(receipt))
}

fn load_report_receipt(
    path: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    stage: &str,
    app_local_data_dir: &Path,
) -> Result<Option<VcetTemporalCampaignReportReceiptV1>, String> {
    let Some(receipt) = load_report_receipt_record(path, preparation, stage, app_local_data_dir)?
    else {
        return Ok(None);
    };
    let Some((_canonical_report_path, report_file, _metadata_bytes)) =
        open_app_owned_report_path(&receipt.payload.report_path, app_local_data_dir, stage)?
    else {
        return Ok(None);
    };
    let (observed_report_sha256, observed_report_bytes) =
        hash_bounded_open_artifact(report_file, MAX_PRODUCT_ARTIFACT_READ_BYTES)?;
    if usize::try_from(observed_report_bytes).ok() != Some(receipt.payload.report_bytes) {
        return Err(format!(
            "VCET temporal {stage} XLSX byte count no longer matches its immutable report receipt"
        ));
    }
    if observed_report_sha256 != receipt.payload.report_sha256 {
        return Err(format!(
            "VCET temporal {stage} XLSX no longer matches its immutable report receipt"
        ));
    }
    Ok(Some(receipt))
}

enum StatusReportProjectionV1 {
    Missing,
    Valid(VcetTemporalCampaignReportReceiptV1),
    Blocked(&'static str),
    Unavailable,
}

enum StatusReportFileOpenV1 {
    Missing,
    Opened(fs::File),
    ContentMismatch,
}

fn report_status_stage(stage: &str) -> Result<&'static str, String> {
    match stage {
        "selection" => Ok("selection_report"),
        "final" => Ok("final_report"),
        _ => Err("invalid VCET temporal report stage".to_string()),
    }
}

fn open_status_report_file(
    raw: &str,
    app_local_data_dir: &Path,
    stage: &str,
) -> Result<StatusReportFileOpenV1, ArtifactAccessFailureV1> {
    let path = validate_app_owned_report_target_classified(raw, app_local_data_dir, stage)?;
    match fs::symlink_metadata(&path) {
        Ok(metadata)
            if metadata.file_type().is_symlink()
                || metadata_is_reparse_point(&metadata)
                || !metadata.is_file() =>
        {
            return Err(ArtifactAccessFailureV1::Invalid)
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StatusReportFileOpenV1::Missing)
        }
        Err(_) => return Err(ArtifactAccessFailureV1::Unavailable),
    }
    let file = match open_artifact_read_handle(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StatusReportFileOpenV1::Missing)
        }
        Err(_) => return Err(ArtifactAccessFailureV1::Unavailable),
    };
    let metadata = file
        .metadata()
        .map_err(|_| ArtifactAccessFailureV1::Unavailable)?;
    if metadata.file_type().is_symlink()
        || metadata_is_reparse_point(&metadata)
        || !metadata.is_file()
    {
        return Err(ArtifactAccessFailureV1::Invalid);
    }
    let canonical_report = final_path_for_open_artifact(&file, &path)?;
    let reports_dir = preflight_app_owned_reports_dir_classified(app_local_data_dir)?;
    let canonical_reports =
        fs::canonicalize(reports_dir).map_err(|_| ArtifactAccessFailureV1::Unavailable)?;
    if canonical_report.parent() != Some(canonical_reports.as_path())
        || canonical_report
            .file_name()
            .and_then(|value| value.to_str())
            .is_none_or(|file_name| {
                !commands_reports::vcet_temporal_report_filename_matches_stage(file_name, stage)
            })
    {
        return Err(ArtifactAccessFailureV1::Invalid);
    }
    if metadata.len() > MAX_PRODUCT_ARTIFACT_READ_BYTES {
        return Ok(StatusReportFileOpenV1::ContentMismatch);
    }
    Ok(StatusReportFileOpenV1::Opened(file))
}

fn verify_status_report_file(
    receipt: &VcetTemporalCampaignReportReceiptV1,
    app_local_data_dir: &Path,
    stage: &str,
) -> Result<bool, StatusReportProjectionV1> {
    let opened =
        match open_status_report_file(&receipt.payload.report_path, app_local_data_dir, stage) {
            Ok(opened) => opened,
            Err(ArtifactAccessFailureV1::Unavailable) => {
                return Err(StatusReportProjectionV1::Unavailable)
            }
            Err(ArtifactAccessFailureV1::Invalid) => {
                return Err(StatusReportProjectionV1::Blocked(
                    REPORT_XLSX_NOT_RECOVERABLE_REASON,
                ))
            }
        };
    let file = match opened {
        StatusReportFileOpenV1::Missing | StatusReportFileOpenV1::ContentMismatch => {
            return Ok(false)
        }
        StatusReportFileOpenV1::Opened(file) => file,
    };
    let (sha256, bytes) =
        match hash_bounded_open_artifact_classified(file, MAX_PRODUCT_ARTIFACT_READ_BYTES) {
            Ok(result) => result,
            Err(ArtifactAccessFailureV1::Unavailable) => {
                return Err(StatusReportProjectionV1::Unavailable)
            }
            Err(ArtifactAccessFailureV1::Invalid) => return Ok(false),
        };
    Ok(
        usize::try_from(bytes).ok() == Some(receipt.payload.report_bytes)
            && sha256 == receipt.payload.report_sha256,
    )
}

fn classify_and_repair_status_report(
    receipt_path: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    stage: &str,
    pre_final: Option<&ReduceVcetTemporalPreFinalShardsResponseV1>,
    final_test: Option<&ReduceVcetTemporalFinalTestShardsResponseV1>,
    expected_source_artifact_sha256: Option<&str>,
    allow_missing_source_lineage_deferral: bool,
    app_local_data_dir: &Path,
) -> StatusReportProjectionV1 {
    let receipt = match read_json_file_classified::<VcetTemporalCampaignReportReceiptV1>(
        receipt_path,
        MAX_PRODUCT_ARTIFACT_READ_BYTES,
    ) {
        Ok(None) => return StatusReportProjectionV1::Missing,
        Ok(Some(receipt)) => receipt,
        Err(ArtifactAccessFailureV1::Unavailable) => return StatusReportProjectionV1::Unavailable,
        Err(ArtifactAccessFailureV1::Invalid) => {
            return StatusReportProjectionV1::Blocked(REPORT_RECEIPT_NOT_RECOVERABLE_REASON)
        }
    };
    let receipt_hash_is_valid = report_receipt_sha256(&receipt.payload)
        .ok()
        .is_some_and(|sha256| sha256 == receipt.artifact_sha256);
    if receipt.payload.schema_version != VCET_TEMPORAL_PRODUCT_CAMPAIGN_SCHEMA_VERSION
        || receipt.payload.campaign_id != preparation.payload.campaign_id
        || receipt.payload.stage != stage
        || !receipt_hash_is_valid
    {
        return StatusReportProjectionV1::Blocked(REPORT_RECEIPT_NOT_RECOVERABLE_REASON);
    }
    match validate_app_owned_report_target_classified(
        &receipt.payload.report_path,
        app_local_data_dir,
        stage,
    ) {
        Ok(_) => {}
        Err(ArtifactAccessFailureV1::Invalid) => {
            return StatusReportProjectionV1::Blocked(REPORT_RECEIPT_NOT_RECOVERABLE_REASON)
        }
        Err(ArtifactAccessFailureV1::Unavailable) => return StatusReportProjectionV1::Unavailable,
    }
    let Some(expected_source_artifact_sha256) = expected_source_artifact_sha256 else {
        // A final report receipt can legitimately outlive the mutable final
        // reduction projection. Its source lineage becomes verifiable again
        // after the command deterministically rebuilds that reduction from the
        // completed durable shard receipts. Keep malformed/foreign receipt
        // structure blocked above, but do not let an otherwise valid receipt
        // prevent that recovery pass from running.
        return if stage == "final" && allow_missing_source_lineage_deferral {
            StatusReportProjectionV1::Missing
        } else {
            StatusReportProjectionV1::Blocked(REPORT_RECEIPT_NOT_RECOVERABLE_REASON)
        };
    };
    if receipt.payload.source_artifact_sha256 != expected_source_artifact_sha256 {
        return StatusReportProjectionV1::Blocked(REPORT_RECEIPT_NOT_RECOVERABLE_REASON);
    }

    match verify_status_report_file(&receipt, app_local_data_dir, stage) {
        Ok(true) => return StatusReportProjectionV1::Valid(receipt),
        Err(classification) => return classification,
        Ok(false) => {}
    }

    let Some(pre_final) = pre_final else {
        return StatusReportProjectionV1::Blocked(REPORT_XLSX_NOT_RECOVERABLE_REASON);
    };
    let regenerated = match commands_reports::build_vcet_temporal_report_xlsx_bytes(
        &preparation.payload.sealed_universe,
        pre_final,
        final_test,
    ) {
        Ok(bytes) => bytes,
        Err(_) => return StatusReportProjectionV1::Blocked(REPORT_XLSX_NOT_RECOVERABLE_REASON),
    };
    let regenerated_sha256 = format!("sha256:{:x}", Sha256::digest(&regenerated));
    if regenerated.len() != receipt.payload.report_bytes
        || regenerated_sha256 != receipt.payload.report_sha256
    {
        return StatusReportProjectionV1::Blocked(REPORT_XLSX_NOT_RECOVERABLE_REASON);
    }
    let report_path = match validate_app_owned_report_target_classified(
        &receipt.payload.report_path,
        app_local_data_dir,
        stage,
    ) {
        Ok(path) => path,
        Err(ArtifactAccessFailureV1::Invalid) => {
            return StatusReportProjectionV1::Blocked(REPORT_XLSX_NOT_RECOVERABLE_REASON)
        }
        Err(ArtifactAccessFailureV1::Unavailable) => return StatusReportProjectionV1::Unavailable,
    };
    if run_artifacts::publish_artifact_file_atomically(&report_path, &regenerated).is_err() {
        return match verify_status_report_file(&receipt, app_local_data_dir, stage) {
            Ok(true) => StatusReportProjectionV1::Valid(receipt),
            Ok(false) => StatusReportProjectionV1::Unavailable,
            Err(classification) => classification,
        };
    }
    match verify_status_report_file(&receipt, app_local_data_dir, stage) {
        Ok(true) => StatusReportProjectionV1::Valid(receipt),
        Ok(false) => StatusReportProjectionV1::Blocked(REPORT_XLSX_NOT_RECOVERABLE_REASON),
        Err(classification) => classification,
    }
}

fn report_receipt_file_name(stage: &str) -> Result<&'static str, String> {
    match stage {
        "selection" => Ok(SELECTION_REPORT_RECEIPT_FILE_NAME),
        "final" => Ok(FINAL_REPORT_RECEIPT_FILE_NAME),
        _ => Err("invalid VCET temporal report stage".to_string()),
    }
}

fn ensure_vcet_temporal_report(
    dir: &Path,
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    stage: &str,
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
    final_test: Option<&ReduceVcetTemporalFinalTestShardsResponseV1>,
    source_artifact_sha256: &str,
    app_local_data_dir: &Path,
) -> Result<(), String> {
    let receipt_path = dir.join(report_receipt_file_name(stage)?);
    let Some(receipt) =
        load_report_receipt_record(&receipt_path, preparation, stage, app_local_data_dir)?
    else {
        preflight_app_owned_reports_dir(app_local_data_dir)?;
        let report = commands_reports::save_vcet_temporal_report_xlsx_for_base(
            &preparation.payload.sealed_universe,
            pre_final,
            final_test,
            preparation.payload.report_label.as_deref(),
            app_local_data_dir,
        )?;
        return persist_report_receipt(
            dir,
            preparation,
            stage,
            report.path,
            report.bytes,
            source_artifact_sha256.to_string(),
            app_local_data_dir,
        );
    };
    if receipt.payload.source_artifact_sha256 != source_artifact_sha256 {
        return Err(format!(
            "VCET temporal {stage} report receipt has foreign source-artifact lineage"
        ));
    }
    if load_report_receipt(&receipt_path, preparation, stage, app_local_data_dir)?.is_some() {
        return Ok(());
    }

    let regenerated = commands_reports::build_vcet_temporal_report_xlsx_bytes(
        &preparation.payload.sealed_universe,
        pre_final,
        final_test,
    )?;
    let regenerated_sha256 = format!("sha256:{:x}", Sha256::digest(&regenerated));
    if regenerated.len() != receipt.payload.report_bytes
        || regenerated_sha256 != receipt.payload.report_sha256
    {
        return Err(format!(
            "VCET temporal {stage} XLSX deterministic regeneration differs from its immutable report receipt"
        ));
    }
    let report_path =
        validate_app_owned_report_target(&receipt.payload.report_path, app_local_data_dir, stage)?;
    if let Err(publish_error) =
        run_artifacts::publish_new_artifact_file_atomically(&report_path, &regenerated)
    {
        if load_report_receipt(&receipt_path, preparation, stage, app_local_data_dir)?.is_some() {
            return Ok(());
        }
        return Err(format!(
            "restore VCET temporal {stage} XLSX from immutable receipt: {publish_error}"
        ));
    }
    load_report_receipt(&receipt_path, preparation, stage, app_local_data_dir)?
        .ok_or_else(|| format!("VCET temporal {stage} XLSX regeneration did not persist"))?;
    Ok(())
}

fn run_vcet_temporal_campaign_selection_blocking(
    req: RunVcetTemporalCampaignSelectionRequestV1,
    app_local_data_dir: &Path,
) -> Result<VcetTemporalCampaignStatusV1, String> {
    let (preparation_path, preparation) = load_requested_preparation(
        &req.preparation_artifact_path,
        &req.expected_preparation_artifact_sha256,
        app_local_data_dir,
    )?;
    let _guard = acquire_campaign_guard(&preparation.payload.sealed_universe.universe_sha256)?;
    let dir = preparation_path
        .parent()
        .ok_or_else(|| "VCET temporal preparation has no campaign directory".to_string())?;
    let selection_shards = universe_shards_for_role(
        &preparation.payload.sealed_universe.payload,
        VcetTemporalDatasetCohortRoleV1::Selection,
    )
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();
    if selection_shards.is_empty()
        || selection_shards.len() > MAX_VCET_TEMPORAL_INLINE_PRE_FINAL_SHARD_FAN_IN
    {
        return Err("VCET temporal selection shard fan-in is outside product bounds".to_string());
    }

    let reduction_path = pre_final_reduction_path(dir);
    // Probe the single-assignment winner-lock ledger before consulting any
    // mutable source CSV/cache. A poisoned authorization is a terminal campaign
    // status, while a temporary read outage remains a retryable command error.
    let durable_reduction =
        match probe_pre_final_reduction_from_durable_terminal(app_local_data_dir, &preparation)? {
            DurableCompletionProbe::Missing => None,
            DurableCompletionProbe::Blocked(_) => {
                return campaign_status_from_preparation(
                    &preparation,
                    &preparation_path,
                    app_local_data_dir,
                )
            }
            DurableCompletionProbe::Completed { response, .. } => Some(response),
        };
    let persisted_reduction = read_optional_json_file::<ReduceVcetTemporalPreFinalShardsResponseV1>(
        &reduction_path,
        "pre-final reduction",
    )?;
    if let Some(persisted) = persisted_reduction.as_ref() {
        validate_pre_final_reduction_for_campaign(persisted, &preparation)?;
    }
    if let (Some(durable), Some(persisted)) =
        (durable_reduction.as_ref(), persisted_reduction.as_ref())
    {
        if durable.artifact_sha256 != persisted.artifact_sha256 {
            return campaign_status_from_preparation(
                &preparation,
                &preparation_path,
                app_local_data_dir,
            );
        }
    } else if persisted_reduction.is_some() {
        return campaign_status_from_preparation(
            &preparation,
            &preparation_path,
            app_local_data_dir,
        );
    }
    let known_reduction = durable_reduction.or(persisted_reduction);
    if let Some(reduction) = known_reduction.as_ref() {
        if publish_recovered_projection(&reduction_path, reduction, "pre-final reduction")?
            == RecoveredProjectionPublishV1::Conflict
        {
            return campaign_status_from_preparation(
                &preparation,
                &preparation_path,
                app_local_data_dir,
            );
        }
    }

    let mut shard_artifacts = Vec::with_capacity(selection_shards.len());
    let mut serialized_bytes = 0u64;
    let mut loaded_dataset: Option<(String, LoadedProductDataset)> = None;
    for shard in &selection_shards {
        if shard.dataset_ids.len() != 1 {
            return Err(format!(
                "product VCET selection shard '{}' is not single-dataset",
                shard.shard_id
            ));
        }
        let path = pre_final_shard_path(dir, shard.shard_ordinal);
        let persisted = read_optional_json_file::<ExecuteVcetTemporalPreFinalShardResponseV1>(
            &path,
            "selection shard",
        )?;
        let projection_exists = persisted.is_some();
        let artifact = if let Some(artifact) = persisted {
            validate_pre_final_shard_for_campaign(&artifact, &preparation, shard)?;
            artifact
        } else {
            let dataset_id = &shard.dataset_ids[0];
            if loaded_dataset
                .as_ref()
                .is_none_or(|(loaded_id, _)| loaded_id != dataset_id)
            {
                // Release the previous Candles/plan buffers before loading
                // the next dataset; assignment alone would evaluate the
                // new load while the old option was still resident.
                drop(loaded_dataset.take());
                let spec = dataset_spec(&preparation, dataset_id)?;
                loaded_dataset = Some((
                    dataset_id.clone(),
                    load_product_dataset(&preparation, spec, app_local_data_dir)?,
                ));
            }
            let loaded = &loaded_dataset
                .as_ref()
                .expect("loaded dataset was established")
                .1;
            let request = ExecuteVcetTemporalPreFinalShardRequestV1 {
                sealed_universe: preparation.payload.sealed_universe.clone(),
                expected_universe_sha256: preparation
                    .payload
                    .sealed_universe
                    .universe_sha256
                    .clone(),
                selection_shard: shard.clone(),
                planning_request: loaded.planning_request.clone(),
                planning_evidence: loaded.planning_evidence.clone(),
                expected_planning_evidence_sha256: loaded.planning_evidence.evidence_sha256.clone(),
                loaded_datasets: vec![loaded.binding.clone()],
            };
            let artifact = execute_vcet_temporal_pre_final_shard_blocking_with_loaded_candles(
                request,
                vec![(loaded.binding.clone(), Arc::clone(&loaded.candles))],
            )?;
            validate_vcet_temporal_pre_final_shard_artifact(&artifact)?;
            artifact
        };
        if let Some(reduction) = known_reduction.as_ref() {
            let expected = reduction
                .payload
                .ordered_shards
                .iter()
                .find(|evidence| evidence.shard_id == shard.shard_id)
                .ok_or_else(|| {
                    format!(
                        "durable VCET pre-final reduction omits selection shard '{}'",
                        shard.shard_id
                    )
                })?;
            if expected.artifact_sha256 != artifact.artifact_sha256 {
                return Err(format!(
                    "reconstructed VCET selection shard '{}' differs from the immutable winner-lock lineage",
                    shard.shard_id
                ));
            }
        }
        if !projection_exists {
            publish_new_json(&path, &artifact)?;
        }
        let artifact_bytes = u64::try_from(
            serde_json::to_vec_pretty(&artifact)
                .map_err(|error| format!("encode VCET pre-final shard artifact: {error}"))?
                .len(),
        )
        .map_err(|_| "VCET pre-final serialized byte count overflow".to_string())?;
        serialized_bytes = serialized_bytes
            .checked_add(artifact_bytes)
            .ok_or_else(|| "VCET pre-final serialized byte count overflow".to_string())?;
        if serialized_bytes > MAX_VCET_TEMPORAL_PRODUCT_PRE_FINAL_BYTES {
            return Err(format!(
                "VCET pre-final shard artifacts require {serialized_bytes} serialized bytes, exceeding the {MAX_VCET_TEMPORAL_PRODUCT_PRE_FINAL_BYTES} byte product limit"
            ));
        }
        shard_artifacts.push(artifact);
    }
    drop(loaded_dataset);
    let reduction = if let Some(reduction) = known_reduction {
        reduction
    } else {
        let reduction = reduce_vcet_temporal_pre_final_shards_durable_blocking(
            ReduceVcetTemporalPreFinalShardsRequestV1 {
                sealed_universe: preparation.payload.sealed_universe.clone(),
                expected_universe_sha256: preparation
                    .payload
                    .sealed_universe
                    .universe_sha256
                    .clone(),
                final_stage_nonce: final_stage_nonce(&preparation)?,
                pre_final_shard_artifacts: shard_artifacts,
            },
            app_local_data_dir,
        )?;
        validate_vcet_temporal_pre_final_reduction_artifact(
            &reduction,
            &reduction.artifact_sha256,
        )?;
        reduction
    };
    if publish_recovered_projection(&reduction_path, &reduction, "pre-final reduction")?
        == RecoveredProjectionPublishV1::Conflict
    {
        return campaign_status_from_preparation(
            &preparation,
            &preparation_path,
            app_local_data_dir,
        );
    }
    validate_complete_pre_final_shard_files(dir, &preparation, &selection_shards, &reduction)?;

    ensure_vcet_temporal_report(
        dir,
        &preparation,
        "selection",
        &reduction,
        None,
        &reduction.artifact_sha256,
        app_local_data_dir,
    )?;
    campaign_status_from_preparation(&preparation, &preparation_path, app_local_data_dir)
}

fn open_vcet_temporal_campaign_final_test_blocking(
    req: OpenVcetTemporalCampaignFinalTestRequestV1,
    app_local_data_dir: &Path,
) -> Result<VcetTemporalCampaignStatusV1, String> {
    if !req.confirm_final_test_access {
        return Err(
            "opening VCET final-test scoring requires confirm_final_test_access=true".to_string(),
        );
    }
    let (preparation_path, preparation) = load_requested_preparation(
        &req.preparation_artifact_path,
        &req.expected_preparation_artifact_sha256,
        app_local_data_dir,
    )?;
    let _guard = acquire_campaign_guard(&preparation.payload.sealed_universe.universe_sha256)?;
    let dir = preparation_path
        .parent()
        .ok_or_else(|| "VCET temporal preparation has no campaign directory".to_string())?;
    let initial_status =
        campaign_status_from_preparation(&preparation, &preparation_path, app_local_data_dir)?;
    if initial_status.execution_blocked {
        return Ok(initial_status);
    }
    if initial_status.completed_selection_shard_count != initial_status.selection_shard_count {
        return Ok(initial_status);
    }
    let pre_final_path = pre_final_reduction_path(dir);
    let Some(pre_final) = read_optional_json_file::<ReduceVcetTemporalPreFinalShardsResponseV1>(
        &pre_final_path,
        "pre-final reduction",
    )?
    else {
        return Err(
            "VCET final-test scoring cannot open before global pre-final selection completes"
                .to_string(),
        );
    };
    validate_pre_final_reduction_for_campaign(&pre_final, &preparation)?;
    let durable_pre_final =
        match probe_pre_final_reduction_from_durable_terminal(app_local_data_dir, &preparation)? {
            DurableCompletionProbe::Completed { response, .. } => response,
            DurableCompletionProbe::Missing | DurableCompletionProbe::Blocked(_) => {
                return campaign_status_from_preparation(
                    &preparation,
                    &preparation_path,
                    app_local_data_dir,
                )
            }
        };
    if durable_pre_final.artifact_sha256 != pre_final.artifact_sha256 {
        return Err(
            "VCET final-test scoring rejected campaign and durable winner-lock lineage drift"
                .to_string(),
        );
    }
    let selection_receipt = load_report_receipt(
        &dir.join(SELECTION_REPORT_RECEIPT_FILE_NAME),
        &preparation,
        "selection",
        app_local_data_dir,
    )?
    .ok_or_else(|| {
        "VCET final-test scoring cannot open before the immutable selection report is complete"
            .to_string()
    })?;
    if selection_receipt.payload.source_artifact_sha256 != pre_final.artifact_sha256 {
        return Err(
            "VCET final-test scoring rejected a selection report with foreign reduction lineage"
                .to_string(),
        );
    }
    let expected_winner = normalize_sha256(
        &req.expected_winner_lock_sha256,
        "expected_winner_lock_sha256",
    )?;
    if expected_winner != pre_final.payload.winner_lock.winner_lock_sha256 {
        return Err(format!(
            "VCET final-test winner-lock confirmation mismatch (expected {}, persisted {})",
            expected_winner, pre_final.payload.winner_lock.winner_lock_sha256
        ));
    }
    let final_shards = universe_shards_for_role(
        &preparation.payload.sealed_universe.payload,
        VcetTemporalDatasetCohortRoleV1::FinalTest,
    )
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();
    if final_shards.is_empty() {
        return Err("VCET temporal campaign has no final-test shards".to_string());
    }

    let final_reduction_file = final_reduction_path(dir);
    let persisted_final_reduction = read_optional_json_file::<
        ReduceVcetTemporalFinalTestShardsResponseV1,
    >(&final_reduction_file, "final reduction")?;
    if let Some(reduction) = persisted_final_reduction.as_ref() {
        validate_final_reduction_for_campaign(reduction, &preparation, &pre_final)?;
    }
    let final_reduction_was_published = persisted_final_reduction.is_some();
    let authorizations = pre_final
        .payload
        .final_shard_authorizations
        .iter()
        .map(|authorization| (authorization.shard.shard_id.as_str(), authorization))
        .collect::<BTreeMap<_, _>>();
    let mut durable_probes = Vec::with_capacity(final_shards.len());
    let mut observed_blocked_terminal = false;
    let mut first_probe_unavailable = None;
    for shard in &final_shards {
        if shard.dataset_ids.len() != 1 {
            return Err(format!(
                "product VCET final-test shard '{}' is not single-dataset",
                shard.shard_id
            ));
        }
        let authorization = authorizations.get(shard.shard_id.as_str()).ok_or_else(|| {
            format!(
                "global winner reduction did not authorize final shard '{}'",
                shard.shard_id
            )
        })?;
        let probe = match probe_final_shard_from_durable_terminal(
            app_local_data_dir,
            &preparation,
            shard,
            &pre_final,
            &authorization.authorization_sha256,
        ) {
            Ok(probe) => {
                observed_blocked_terminal |= matches!(probe, DurableCompletionProbe::Blocked(_));
                Some(probe)
            }
            Err(error) => {
                if first_probe_unavailable.is_none() {
                    first_probe_unavailable = Some(error);
                }
                None
            }
        };
        durable_probes.push(probe);
    }
    let mut artifacts = Vec::with_capacity(final_shards.len());
    for (shard, probe) in final_shards.iter().zip(durable_probes) {
        let Some(probe) = probe else {
            artifacts.push(None);
            continue;
        };
        let path = final_shard_path(dir, shard.shard_ordinal);
        let persisted = read_optional_json_file::<ExecuteVcetTemporalFinalTestShardResponseV1>(
            &path,
            "final-test shard",
        )?;
        if let Some(artifact) = persisted.as_ref() {
            validate_final_shard_for_campaign(artifact, &preparation, shard, &pre_final)?;
        }
        match (persisted, probe) {
            (Some(persisted), DurableCompletionProbe::Completed { response, .. }) => {
                if persisted.artifact_sha256 != response.artifact_sha256 {
                    observed_blocked_terminal = true;
                    artifacts.push(None);
                } else {
                    artifacts.push(Some(persisted));
                }
            }
            (Some(_), DurableCompletionProbe::Missing) => {
                observed_blocked_terminal = true;
                artifacts.push(None);
            }
            (None, DurableCompletionProbe::Completed { response, .. }) => {
                if publish_recovered_projection(&path, &response, "final-test shard")?
                    == RecoveredProjectionPublishV1::Conflict
                {
                    observed_blocked_terminal = true;
                    artifacts.push(None);
                } else {
                    artifacts.push(Some(response));
                }
            }
            (None, DurableCompletionProbe::Missing) => artifacts.push(None),
            (_, DurableCompletionProbe::Blocked(_)) => artifacts.push(None),
        }
    }
    if let Some(error) = first_probe_unavailable {
        // Every other completed terminal has already been reconciled above;
        // temporary unavailability never becomes a permanent block.
        return Err(error);
    }
    if observed_blocked_terminal {
        // Status performs another complete read-only pass and returns stable
        // blocked evidence without exposing terminal error text or paths.
        return campaign_status_from_preparation(
            &preparation,
            &preparation_path,
            app_local_data_dir,
        );
    }
    if final_reduction_was_published && artifacts.iter().any(Option::is_none) {
        return campaign_status_from_preparation(
            &preparation,
            &preparation_path,
            app_local_data_dir,
        );
    }
    for (index, artifact) in artifacts.iter_mut().enumerate() {
        if artifact.is_some() {
            continue;
        }
        let shard = &final_shards[index];
        let authorization = authorizations.get(shard.shard_id.as_str()).ok_or_else(|| {
            format!(
                "global winner reduction did not authorize final shard '{}'",
                shard.shard_id
            )
        })?;
        let spec = dataset_spec(&preparation, &shard.dataset_ids[0])?;
        let loaded = load_product_dataset(&preparation, spec, app_local_data_dir)?;
        let request = ExecuteVcetTemporalFinalTestShardRequestV1 {
            sealed_universe: preparation.payload.sealed_universe.clone(),
            expected_universe_sha256: preparation.payload.sealed_universe.universe_sha256.clone(),
            global_winner_reduction: pre_final.clone(),
            expected_global_winner_reduction_artifact_sha256: pre_final.artifact_sha256.clone(),
            final_shard: shard.clone(),
            planning_request: loaded.planning_request.clone(),
            planning_evidence: loaded.planning_evidence.clone(),
            expected_planning_evidence_sha256: loaded.planning_evidence.evidence_sha256.clone(),
            loaded_datasets: vec![loaded.binding.clone()],
            expected_authorized_prior_state_sha256: authorization
                .expected_prior_state_sha256
                .clone(),
        };
        let executed = execute_vcet_temporal_final_test_shard_durable_blocking_with_loaded_candles(
            request,
            vec![(loaded.binding, loaded.candles)],
            app_local_data_dir,
        )?;
        validate_vcet_temporal_final_shard_artifact(&executed)?;
        publish_new_json(&final_shard_path(dir, shard.shard_ordinal), &executed)?;
        *artifact = Some(executed);
    }
    let artifacts = artifacts
        .into_iter()
        .enumerate()
        .map(|(index, artifact)| {
            artifact.ok_or_else(|| {
                format!("VCET temporal recovery left final shard index {index} unresolved")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let final_reduction = if let Some(reduction) = persisted_final_reduction {
        reduction
    } else {
        let reduction = reduce_vcet_temporal_final_test_shards_durable_blocking(
            ReduceVcetTemporalFinalTestShardsRequestV1 {
                sealed_universe: preparation.payload.sealed_universe.clone(),
                expected_universe_sha256: preparation
                    .payload
                    .sealed_universe
                    .universe_sha256
                    .clone(),
                global_winner_reduction: pre_final.clone(),
                expected_global_winner_reduction_artifact_sha256: pre_final.artifact_sha256.clone(),
                final_shard_artifacts: artifacts,
            },
            app_local_data_dir,
        )?;
        validate_vcet_temporal_final_reduction_artifact(&reduction)?;
        publish_new_json(&final_reduction_file, &reduction)?;
        reduction
    };
    validate_complete_final_shard_files(
        dir,
        &preparation,
        &final_shards,
        &pre_final,
        &final_reduction,
    )?;

    // Once the exact durable reduction projection exists, status owns repair
    // of any retained report receipt/XLSX pair. This both converts foreign or
    // non-reproducible evidence into the stable additive block contract and
    // replaces byte-corrupt ordinary XLSX files only when deterministic bytes
    // exactly match the immutable receipt.
    let reconciled_status =
        campaign_status_from_preparation(&preparation, &preparation_path, app_local_data_dir)?;
    if reconciled_status.execution_blocked || reconciled_status.final_report_xlsx_path.is_some() {
        return Ok(reconciled_status);
    }

    ensure_vcet_temporal_report(
        dir,
        &preparation,
        "final",
        &pre_final,
        Some(&final_reduction),
        &final_reduction.artifact_sha256,
        app_local_data_dir,
    )?;
    campaign_status_from_preparation(&preparation, &preparation_path, app_local_data_dir)
}

fn product_campaign_state(
    has_pre_final_reduction: bool,
    has_selection_report: bool,
    has_final_reduction: bool,
    has_final_report: bool,
) -> VcetTemporalCampaignStateV1 {
    if has_pre_final_reduction && has_final_reduction && has_final_report {
        VcetTemporalCampaignStateV1::FinalTestComplete
    } else if has_pre_final_reduction && (has_selection_report || has_final_reduction) {
        VcetTemporalCampaignStateV1::SelectionComplete
    } else {
        VcetTemporalCampaignStateV1::Prepared
    }
}

fn product_campaign_state_with_recovery_evidence(
    artifact_state: VcetTemporalCampaignStateV1,
    selection_projections_complete: bool,
    has_selection_report: bool,
) -> VcetTemporalCampaignStateV1 {
    if !selection_projections_complete && !has_selection_report {
        VcetTemporalCampaignStateV1::Prepared
    } else {
        // Persisted reductions/reports remain visible as the campaign stage
        // they actually reached. Durable poison is represented orthogonally by
        // execution_blocked + blocked_* evidence, never by erasing late-stage
        // artifact state.
        artifact_state
    }
}

fn product_next_required_action(
    state: VcetTemporalCampaignStateV1,
    execution_blocked: bool,
    has_pre_final_reduction: bool,
    has_selection_report: bool,
    has_final_reduction: bool,
    has_final_report: bool,
    completed_selection_shard_count: usize,
    selection_shard_count: usize,
    completed_final_test_shard_count: usize,
) -> &'static str {
    if execution_blocked {
        BLOCKED_CAMPAIGN_ACTION
    } else if completed_selection_shard_count != selection_shard_count {
        if completed_selection_shard_count == 0 && !has_pre_final_reduction {
            "run_selection"
        } else {
            "resume_selection"
        }
    } else if has_pre_final_reduction && !has_selection_report {
        "resume_selection_report"
    } else if has_final_reduction && !has_final_report {
        "confirm_and_resume_final_report"
    } else {
        match state {
            VcetTemporalCampaignStateV1::Prepared => {
                if completed_selection_shard_count == 0 {
                    "run_selection"
                } else {
                    "resume_selection"
                }
            }
            VcetTemporalCampaignStateV1::SelectionComplete => {
                if completed_final_test_shard_count == 0 {
                    "confirm_and_open_final_test"
                } else {
                    "confirm_and_resume_final_test"
                }
            }
            VcetTemporalCampaignStateV1::FinalTestComplete => "none",
        }
    }
}

fn durable_execution_block(
    stage: &'static str,
    shard_ordinal: Option<usize>,
) -> VcetTemporalCampaignExecutionBlock {
    VcetTemporalCampaignExecutionBlock {
        stage,
        shard_ordinal,
        reason_code: DURABLE_STAGE_NOT_RECOVERABLE_REASON,
    }
}

fn report_execution_block(
    stage: &'static str,
    reason_code: &'static str,
) -> VcetTemporalCampaignExecutionBlock {
    VcetTemporalCampaignExecutionBlock {
        stage,
        shard_ordinal: None,
        reason_code,
    }
}

fn record_first_execution_block(
    current: &mut Option<VcetTemporalCampaignExecutionBlock>,
    stage: &'static str,
    shard_ordinal: Option<usize>,
) {
    if current.is_none() {
        *current = Some(durable_execution_block(stage, shard_ordinal));
    }
}

fn record_first_report_block(
    current: &mut Option<VcetTemporalCampaignExecutionBlock>,
    stage: &'static str,
    reason_code: &'static str,
) {
    if current.is_none() {
        *current = Some(report_execution_block(stage, reason_code));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecoveredProjectionPublishV1 {
    PublishedOrVerified,
    Conflict,
}

fn verify_recovered_projection<T>(
    path: &Path,
    value: &T,
) -> Result<RecoveredProjectionPublishV1, String>
where
    T: Serialize + DeserializeOwned,
{
    let existing: T = match read_json_file_classified(path, MAX_PRODUCT_ARTIFACT_READ_BYTES) {
        Ok(Some(value)) => value,
        Ok(None) | Err(ArtifactAccessFailureV1::Invalid) => {
            return Ok(RecoveredProjectionPublishV1::Conflict)
        }
        Err(ArtifactAccessFailureV1::Unavailable) => return Err(
            "VCET temporal recovered projection is temporarily unavailable; retry the operation"
                .to_string(),
        ),
    };
    let existing = match serde_json::to_vec(&existing) {
        Ok(value) => value,
        Err(_) => return Ok(RecoveredProjectionPublishV1::Conflict),
    };
    let proposed = match serde_json::to_vec(value) {
        Ok(value) => value,
        Err(_) => return Ok(RecoveredProjectionPublishV1::Conflict),
    };
    Ok(if existing == proposed {
        RecoveredProjectionPublishV1::PublishedOrVerified
    } else {
        RecoveredProjectionPublishV1::Conflict
    })
}

fn publish_recovered_projection<T>(
    path: &Path,
    value: &T,
    stage: &str,
) -> Result<RecoveredProjectionPublishV1, String>
where
    T: Serialize + DeserializeOwned,
{
    match fs::symlink_metadata(path) {
        Ok(_) => verify_recovered_projection(path, value),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match publish_new_json(path, value) {
                Ok(()) => Ok(RecoveredProjectionPublishV1::PublishedOrVerified),
                Err(_) => match fs::symlink_metadata(path) {
                    Ok(_) => verify_recovered_projection(path, value),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(format!(
                        "VCET temporal {stage} projection is temporarily unavailable; retry the operation"
                    )),
                    Err(_) => Err(format!(
                        "VCET temporal {stage} projection is temporarily unavailable; retry the operation"
                    )),
                },
            }
        }
        Err(_) => Err(format!(
            "VCET temporal {stage} projection is temporarily unavailable; retry the operation"
        )),
    }
}

struct DurableProjectionReconciliation<T> {
    /// Best validated stage authority available for read-only recovery/status.
    /// A durable completion response takes precedence over a local projection.
    artifact: Option<T>,
    /// Whether the campaign projection itself is present and agrees with the
    /// durable terminal (or was just reconstructed from it).
    projection_complete: bool,
    blocked: bool,
    unavailable: bool,
}

enum CampaignProjectionStateV1<T> {
    Missing,
    Valid(T),
    Invalid,
    Unavailable,
}

fn classify_json_projection<T, F>(path: &Path, validate: F) -> CampaignProjectionStateV1<T>
where
    T: DeserializeOwned,
    F: FnOnce(&T) -> Result<(), String>,
{
    match read_json_file_classified(path, MAX_PRODUCT_ARTIFACT_READ_BYTES) {
        Ok(None) => CampaignProjectionStateV1::Missing,
        Ok(Some(value)) => {
            if validate(&value).is_ok() {
                CampaignProjectionStateV1::Valid(value)
            } else {
                CampaignProjectionStateV1::Invalid
            }
        }
        Err(ArtifactAccessFailureV1::Invalid) => CampaignProjectionStateV1::Invalid,
        Err(ArtifactAccessFailureV1::Unavailable) => CampaignProjectionStateV1::Unavailable,
    }
}

fn reconcile_durable_projection<T, F>(
    path: &Path,
    projection: CampaignProjectionStateV1<T>,
    probe: DurableCompletionProbe<T>,
    stage: &str,
    identity: F,
) -> Result<DurableProjectionReconciliation<T>, String>
where
    T: Serialize + DeserializeOwned,
    F: Fn(&T) -> String,
{
    match probe {
        DurableCompletionProbe::Missing => Ok(match projection {
            CampaignProjectionStateV1::Missing => DurableProjectionReconciliation {
                artifact: None,
                projection_complete: false,
                blocked: false,
                unavailable: false,
            },
            CampaignProjectionStateV1::Valid(value) => DurableProjectionReconciliation {
                // Retain the validated projection only so status can discover
                // and probe later-stage durable authorizations. It is blocked
                // because no durable terminal authorizes it.
                artifact: Some(value),
                projection_complete: false,
                blocked: true,
                unavailable: false,
            },
            CampaignProjectionStateV1::Invalid => DurableProjectionReconciliation {
                artifact: None,
                projection_complete: false,
                blocked: true,
                unavailable: false,
            },
            CampaignProjectionStateV1::Unavailable => DurableProjectionReconciliation {
                artifact: None,
                projection_complete: false,
                blocked: false,
                unavailable: true,
            },
        }),
        DurableCompletionProbe::Blocked(_) => Ok(match projection {
            CampaignProjectionStateV1::Valid(value) => DurableProjectionReconciliation {
                artifact: Some(value),
                projection_complete: false,
                blocked: true,
                unavailable: false,
            },
            CampaignProjectionStateV1::Unavailable => DurableProjectionReconciliation {
                artifact: None,
                projection_complete: false,
                blocked: true,
                // The durable terminal is already a conclusive permanent
                // block, so a projection read outage cannot make that state
                // ambiguous or retryable.
                unavailable: false,
            },
            CampaignProjectionStateV1::Missing | CampaignProjectionStateV1::Invalid => {
                DurableProjectionReconciliation {
                    artifact: None,
                    projection_complete: false,
                    blocked: true,
                    unavailable: false,
                }
            }
        }),
        DurableCompletionProbe::Completed { response, .. } => {
            match projection {
                CampaignProjectionStateV1::Valid(existing) => {
                    let matches = identity(&existing) == identity(&response);
                    Ok(DurableProjectionReconciliation {
                        // Durable completion is the authority even when a
                        // foreign-but-well-formed campaign projection exists.
                        artifact: Some(response),
                        projection_complete: matches,
                        blocked: !matches,
                        unavailable: false,
                    })
                }
                CampaignProjectionStateV1::Invalid => Ok(DurableProjectionReconciliation {
                    artifact: Some(response),
                    projection_complete: false,
                    blocked: true,
                    unavailable: false,
                }),
                CampaignProjectionStateV1::Unavailable => Ok(DurableProjectionReconciliation {
                    artifact: Some(response),
                    projection_complete: false,
                    blocked: false,
                    unavailable: true,
                }),
                CampaignProjectionStateV1::Missing => {
                    match publish_recovered_projection(path, &response, stage) {
                        Ok(RecoveredProjectionPublishV1::PublishedOrVerified) => {
                            Ok(DurableProjectionReconciliation {
                                artifact: Some(response),
                                projection_complete: true,
                                blocked: false,
                                unavailable: false,
                            })
                        }
                        Ok(RecoveredProjectionPublishV1::Conflict) => {
                            Ok(DurableProjectionReconciliation {
                                artifact: Some(response),
                                projection_complete: false,
                                blocked: true,
                                unavailable: false,
                            })
                        }
                        Err(_) => Ok(DurableProjectionReconciliation {
                            artifact: Some(response),
                            projection_complete: false,
                            blocked: false,
                            unavailable: true,
                        }),
                    }
                }
            }
        }
    }
}

fn campaign_status_from_preparation(
    preparation: &VcetTemporalCampaignPreparationArtifactV1,
    preparation_path: &Path,
    app_local_data_dir: &Path,
) -> Result<VcetTemporalCampaignStatusV1, String> {
    validate_preparation_artifact(preparation, &preparation.artifact_sha256)?;
    let expected_dir = campaign_dir(app_local_data_dir, &preparation.payload.campaign_id)?;
    let canonical_expected = fs::canonicalize(&expected_dir).map_err(|error| {
        format!(
            "canonicalize VCET temporal campaign directory {}: {error}",
            expected_dir.display()
        )
    })?;
    let canonical_preparation = fs::canonicalize(preparation_path).map_err(|error| {
        format!(
            "canonicalize VCET temporal preparation {}: {error}",
            preparation_path.display()
        )
    })?;
    if canonical_preparation.parent() != Some(canonical_expected.as_path()) {
        return Err("VCET temporal preparation path/campaign identity mismatch".to_string());
    }
    let dir = canonical_expected;
    let selection_shards = universe_shards_for_role(
        &preparation.payload.sealed_universe.payload,
        VcetTemporalDatasetCohortRoleV1::Selection,
    )
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();
    let final_shards = universe_shards_for_role(
        &preparation.payload.sealed_universe.payload,
        VcetTemporalDatasetCohortRoleV1::FinalTest,
    )
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();
    let pre_final_path = pre_final_reduction_path(&dir);
    let pre_final_projection = classify_json_projection(&pre_final_path, |artifact| {
        validate_pre_final_reduction_for_campaign(artifact, preparation)
    });
    let selection_projection_states = selection_shards
        .iter()
        .map(|shard| {
            classify_json_projection(
                &pre_final_shard_path(&dir, shard.shard_ordinal),
                |artifact| validate_pre_final_shard_for_campaign(artifact, preparation, shard),
            )
        })
        .collect::<Vec<_>>();

    let mut execution_block = None;
    let mut durable_unavailable_error = None;
    let pre_final_probe =
        match probe_pre_final_reduction_from_durable_terminal(app_local_data_dir, preparation) {
            Ok(probe) => Some(probe),
            Err(error) => {
                durable_unavailable_error = Some(error);
                None
            }
        };
    let mut authoritative_durable_block = matches!(
        pre_final_probe.as_ref(),
        Some(DurableCompletionProbe::Blocked(_))
    );

    // Probe every discoverable final-shard terminal before interpreting any
    // malformed/foreign campaign projection as a stable block. A valid local
    // pre-final projection may be used only to discover authorization keys
    // when its durable terminal is missing/blocked; the reconciliation below
    // still fails that projection closed.
    let probe_winner_authority = match pre_final_probe.as_ref() {
        Some(DurableCompletionProbe::Completed { response, .. }) => Some(response),
        _ => match &pre_final_projection {
            CampaignProjectionStateV1::Valid(artifact) => Some(artifact),
            CampaignProjectionStateV1::Missing
            | CampaignProjectionStateV1::Invalid
            | CampaignProjectionStateV1::Unavailable => None,
        },
    };
    let mut final_projection_states = Vec::with_capacity(final_shards.len());
    let mut final_probes = Vec::with_capacity(final_shards.len());
    let mut final_probe_unavailable = Vec::with_capacity(final_shards.len());
    let mut all_final_durable_terminals_completed = !final_shards.is_empty();
    for shard in &final_shards {
        let path = final_shard_path(&dir, shard.shard_ordinal);
        final_projection_states.push(classify_json_projection(&path, |artifact| {
            let winner_authority = probe_winner_authority
                .ok_or_else(|| "VCET temporal final shard has no winner authority".to_string())?;
            validate_final_shard_for_campaign(artifact, preparation, shard, winner_authority)
        }));

        let Some(winner_authority) = probe_winner_authority else {
            all_final_durable_terminals_completed = false;
            final_probes.push(None);
            final_probe_unavailable.push(false);
            continue;
        };
        let Some(authorization) = winner_authority
            .payload
            .final_shard_authorizations
            .iter()
            .find(|authorization| authorization.shard.shard_id == shard.shard_id)
        else {
            all_final_durable_terminals_completed = false;
            record_first_execution_block(
                &mut execution_block,
                "final_test_shard",
                Some(shard.shard_ordinal),
            );
            final_probes.push(None);
            final_probe_unavailable.push(false);
            continue;
        };
        match probe_final_shard_from_durable_terminal(
            app_local_data_dir,
            preparation,
            shard,
            winner_authority,
            &authorization.authorization_sha256,
        ) {
            Ok(probe) => {
                all_final_durable_terminals_completed &=
                    matches!(&probe, DurableCompletionProbe::Completed { .. });
                authoritative_durable_block |= matches!(&probe, DurableCompletionProbe::Blocked(_));
                final_probes.push(Some(probe));
                final_probe_unavailable.push(false);
            }
            Err(error) => {
                all_final_durable_terminals_completed = false;
                if durable_unavailable_error.is_none() {
                    durable_unavailable_error = Some(error);
                }
                final_probes.push(None);
                final_probe_unavailable.push(true);
            }
        }
    }

    let pre_final_reconciliation = if let Some(probe) = pre_final_probe {
        Some(reconcile_durable_projection(
            &pre_final_path,
            pre_final_projection,
            probe,
            "pre-final reduction",
            |artifact| artifact.artifact_sha256.clone(),
        )?)
    } else {
        None
    };
    if let Some(reconciliation) = pre_final_reconciliation.as_ref() {
        if reconciliation.blocked {
            record_first_execution_block(&mut execution_block, "pre_final_reduction", None);
        }
        if reconciliation.unavailable && durable_unavailable_error.is_none() {
            durable_unavailable_error = Some(
                "VCET temporal pre-final reduction projection is temporarily unavailable; retry the operation"
                    .to_string(),
            );
        }
    }
    let pre_final_projection_complete = pre_final_reconciliation
        .as_ref()
        .is_some_and(|reconciliation| reconciliation.projection_complete);
    let pre_final = pre_final_reconciliation.and_then(|reconciliation| reconciliation.artifact);

    let mut completed_selection_shard_count = 0usize;
    for (shard, projection) in selection_shards
        .iter()
        .zip(selection_projection_states.into_iter())
    {
        match projection {
            CampaignProjectionStateV1::Missing => {}
            CampaignProjectionStateV1::Unavailable => {
                if durable_unavailable_error.is_none() {
                    durable_unavailable_error = Some(
                        "VCET temporal selection shard projection is temporarily unavailable; retry the operation"
                            .to_string(),
                    );
                }
            }
            CampaignProjectionStateV1::Invalid => {
                record_first_execution_block(&mut execution_block, "pre_final_reduction", None);
            }
            CampaignProjectionStateV1::Valid(artifact) => {
                completed_selection_shard_count += 1;
                if let Some(reduction) = pre_final.as_ref() {
                    let lineage_matches = reduction
                        .payload
                        .ordered_shards
                        .iter()
                        .find(|evidence| evidence.shard_id == shard.shard_id)
                        .is_some_and(|evidence| {
                            evidence.artifact_sha256 == artifact.artifact_sha256
                        });
                    if !lineage_matches {
                        record_first_execution_block(
                            &mut execution_block,
                            "pre_final_reduction",
                            None,
                        );
                    }
                }
            }
        }
    }

    let mut completed_final_test_shard_count = 0usize;
    let mut final_authorities = Vec::with_capacity(final_shards.len());
    let mut final_projection_complete = Vec::with_capacity(final_shards.len());
    for ((((shard, path), projection), probe), probe_unavailable) in final_shards
        .iter()
        .map(|shard| (shard, final_shard_path(&dir, shard.shard_ordinal)))
        .zip(final_projection_states.into_iter())
        .zip(final_probes.into_iter())
        .zip(final_probe_unavailable.into_iter())
    {
        if probe_unavailable {
            final_authorities.push(None);
            final_projection_complete.push(false);
            continue;
        }
        let Some(probe) = probe else {
            match projection {
                CampaignProjectionStateV1::Missing => {}
                CampaignProjectionStateV1::Unavailable => {
                    if durable_unavailable_error.is_none() {
                        durable_unavailable_error = Some(
                            "VCET temporal final-test shard projection is temporarily unavailable; retry the operation"
                                .to_string(),
                        );
                    }
                }
                CampaignProjectionStateV1::Valid(_) | CampaignProjectionStateV1::Invalid => {
                    record_first_execution_block(
                        &mut execution_block,
                        if pre_final.is_some() {
                            "final_test_shard"
                        } else {
                            "pre_final_reduction"
                        },
                        pre_final.as_ref().map(|_| shard.shard_ordinal),
                    );
                }
            }
            final_authorities.push(None);
            final_projection_complete.push(false);
            continue;
        };
        let reconciliation = reconcile_durable_projection(
            &path,
            projection,
            probe,
            "final-test shard",
            |artifact| artifact.artifact_sha256.clone(),
        )?;
        if reconciliation.blocked {
            record_first_execution_block(
                &mut execution_block,
                "final_test_shard",
                Some(shard.shard_ordinal),
            );
        }
        if reconciliation.unavailable && durable_unavailable_error.is_none() {
            durable_unavailable_error = Some(
                "VCET temporal final-test shard projection is temporarily unavailable; retry the operation"
                    .to_string(),
            );
        }
        if reconciliation.projection_complete {
            completed_final_test_shard_count += 1;
        }
        final_projection_complete.push(reconciliation.projection_complete);
        final_authorities.push(reconciliation.artifact);
    }

    let final_path = final_reduction_path(&dir);
    let final_reduction_projection = classify_json_projection(&final_path, |artifact| {
        let winner_authority = pre_final
            .as_ref()
            .ok_or_else(|| "VCET temporal final reduction has no winner authority".to_string())?;
        validate_final_reduction_for_campaign(artifact, preparation, winner_authority)
    });
    let final_reduction = match final_reduction_projection {
        CampaignProjectionStateV1::Missing => None,
        CampaignProjectionStateV1::Unavailable => {
            if durable_unavailable_error.is_none() {
                durable_unavailable_error = Some(
                    "VCET temporal final reduction projection is temporarily unavailable; retry the operation"
                        .to_string(),
                );
            }
            None
        }
        CampaignProjectionStateV1::Invalid => {
            record_first_execution_block(
                &mut execution_block,
                if pre_final.is_some() {
                    "final_test_shard"
                } else {
                    "pre_final_reduction"
                },
                pre_final
                    .as_ref()
                    .and_then(|_| final_shards.first().map(|shard| shard.shard_ordinal)),
            );
            None
        }
        CampaignProjectionStateV1::Valid(reduction) => {
            for (index, shard) in final_shards.iter().enumerate() {
                let evidence = reduction
                    .payload
                    .shards
                    .iter()
                    .find(|evidence| evidence.shard_id == shard.shard_id);
                let artifact = final_authorities.get(index).and_then(Option::as_ref);
                let lineage_matches = evidence.zip(artifact).is_some_and(|(evidence, artifact)| {
                    evidence.artifact_sha256 == artifact.artifact_sha256
                        && evidence.authorization_sha256 == artifact.payload.authorization_sha256
                        && evidence.authorized_prior_state_sha256
                            == artifact.payload.authorized_prior_state_sha256
                });
                if !final_projection_complete
                    .get(index)
                    .copied()
                    .unwrap_or(false)
                    || !lineage_matches
                {
                    record_first_execution_block(
                        &mut execution_block,
                        "final_test_shard",
                        Some(shard.shard_ordinal),
                    );
                }
            }
            if reduction.payload.shards.len() != final_shards.len() {
                record_first_execution_block(
                    &mut execution_block,
                    "final_test_shard",
                    final_shards.first().map(|shard| shard.shard_ordinal),
                );
            }
            Some(reduction)
        }
    };

    if let Some(error) = durable_unavailable_error.filter(|_| !authoritative_durable_block) {
        return Err(error);
    }

    let selection_report = match classify_and_repair_status_report(
        &dir.join(SELECTION_REPORT_RECEIPT_FILE_NAME),
        preparation,
        "selection",
        pre_final.as_ref(),
        None,
        pre_final
            .as_ref()
            .map(|reduction| reduction.artifact_sha256.as_str()),
        false,
        app_local_data_dir,
    ) {
        StatusReportProjectionV1::Missing => None,
        StatusReportProjectionV1::Valid(receipt) => Some(receipt),
        StatusReportProjectionV1::Blocked(reason_code) => {
            record_first_report_block(
                &mut execution_block,
                report_status_stage("selection")?,
                reason_code,
            );
            None
        }
        StatusReportProjectionV1::Unavailable if authoritative_durable_block => None,
        StatusReportProjectionV1::Unavailable => {
            return Err(
                "VCET temporal selection report projection is temporarily unavailable; retry the operation"
                    .to_string(),
            )
        }
    };
    let final_report = match classify_and_repair_status_report(
        &dir.join(FINAL_REPORT_RECEIPT_FILE_NAME),
        preparation,
        "final",
        pre_final.as_ref(),
        final_reduction.as_ref(),
        final_reduction
            .as_ref()
            .map(|reduction| reduction.artifact_sha256.as_str()),
        all_final_durable_terminals_completed,
        app_local_data_dir,
    ) {
        StatusReportProjectionV1::Missing => None,
        StatusReportProjectionV1::Valid(receipt) => Some(receipt),
        StatusReportProjectionV1::Blocked(reason_code) => {
            record_first_report_block(
                &mut execution_block,
                report_status_stage("final")?,
                reason_code,
            );
            None
        }
        StatusReportProjectionV1::Unavailable if authoritative_durable_block => None,
        StatusReportProjectionV1::Unavailable => return Err(
            "VCET temporal final report projection is temporarily unavailable; retry the operation"
                .to_string(),
        ),
    };

    let artifact_state = product_campaign_state(
        pre_final.is_some(),
        selection_report.is_some(),
        final_reduction.is_some(),
        final_report.is_some(),
    );
    let state = product_campaign_state_with_recovery_evidence(
        artifact_state,
        completed_selection_shard_count == selection_shards.len(),
        selection_report.is_some(),
    );
    let next_required_action = product_next_required_action(
        state,
        execution_block.is_some(),
        pre_final.is_some(),
        selection_report.is_some(),
        final_reduction.is_some(),
        final_report.is_some(),
        completed_selection_shard_count,
        selection_shards.len(),
        completed_final_test_shard_count,
    )
    .to_string();
    let public_pre_final = pre_final_projection_complete
        .then_some(())
        .and(pre_final.as_ref());

    Ok(VcetTemporalCampaignStatusV1 {
        schema_version: VCET_TEMPORAL_PRODUCT_CAMPAIGN_SCHEMA_VERSION,
        campaign_id: preparation.payload.campaign_id.clone(),
        state,
        preparation_artifact_path: public_path_display(preparation_path),
        preparation_artifact_sha256: preparation.artifact_sha256.clone(),
        source_campaign_summary_path: preparation.payload.source_campaign_summary_path.clone(),
        source_campaign_summary_sha256: preparation.payload.source_campaign_summary_sha256.clone(),
        universe_sha256: preparation.payload.sealed_universe.universe_sha256.clone(),
        candidate_count: preparation
            .payload
            .sealed_universe
            .payload
            .candidate_manifest
            .payload
            .candidates
            .len(),
        dataset_count: preparation.payload.sealed_universe.payload.dataset_count,
        selection_dataset_count: preparation
            .payload
            .sealed_universe
            .payload
            .selection_dataset_count,
        final_test_dataset_count: preparation
            .payload
            .sealed_universe
            .payload
            .final_test_dataset_count,
        selection_shard_count: selection_shards.len(),
        completed_selection_shard_count,
        final_test_shard_count: final_shards.len(),
        completed_final_test_shard_count,
        winner_candidate_id: public_pre_final
            .as_ref()
            .map(|artifact| artifact.payload.winner_lock.candidate_id.clone()),
        winner_lock_sha256: public_pre_final
            .as_ref()
            .map(|artifact| artifact.payload.winner_lock.winner_lock_sha256.clone()),
        winner_runner_up_margin: public_pre_final.and_then(|artifact| {
            artifact
                .payload
                .winner_lock
                .winner_to_runner_up_score_margin
        }),
        pre_final_reduction_artifact_path: pre_final_projection_complete
            .then(|| public_path_display(&pre_final_path)),
        pre_final_reduction_artifact_sha256: public_pre_final
            .as_ref()
            .map(|artifact| artifact.artifact_sha256.clone()),
        final_reduction_artifact_path: final_reduction
            .as_ref()
            .map(|_| public_path_display(&final_path)),
        final_reduction_artifact_sha256: final_reduction
            .as_ref()
            .map(|artifact| artifact.artifact_sha256.clone()),
        selection_report_xlsx_path: selection_report.map(|receipt| receipt.payload.report_path),
        final_report_xlsx_path: final_report.map(|receipt| receipt.payload.report_path),
        execution_blocked: execution_block.is_some(),
        blocked_stage: execution_block
            .as_ref()
            .map(|blocked| blocked.stage.to_string()),
        blocked_shard_ordinal: execution_block
            .as_ref()
            .and_then(|blocked| blocked.shard_ordinal),
        blocked_reason_code: execution_block
            .as_ref()
            .map(|blocked| blocked.reason_code.to_string()),
        next_required_action,
        limitations: preparation.payload.limitations.clone(),
    })
}

fn get_vcet_temporal_campaign_status_blocking(
    req: GetVcetTemporalCampaignStatusRequestV1,
    app_local_data_dir: &Path,
) -> Result<VcetTemporalCampaignStatusV1, String> {
    let (path, preparation) = load_requested_preparation(
        &req.preparation_artifact_path,
        &req.expected_preparation_artifact_sha256,
        app_local_data_dir,
    )?;
    let _guard =
        acquire_campaign_status_guard(&preparation.payload.sealed_universe.universe_sha256)?;
    campaign_status_from_preparation(&preparation, &path, app_local_data_dir)
}

#[tauri::command]
pub async fn prepare_vcet_temporal_campaign(
    req: PrepareVcetTemporalCampaignRequestV1,
    app: tauri::AppHandle,
) -> Result<VcetTemporalCampaignStatusV1, String> {
    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        prepare_vcet_temporal_campaign_blocking(req, app, &base)
    })
    .await
    .map_err(|error| format!("VCET temporal campaign preparation task failed: {error}"))?
}

#[tauri::command]
pub async fn run_vcet_temporal_campaign_selection(
    req: RunVcetTemporalCampaignSelectionRequestV1,
    app: tauri::AppHandle,
) -> Result<VcetTemporalCampaignStatusV1, String> {
    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        run_vcet_temporal_campaign_selection_blocking(req, &base)
    })
    .await
    .map_err(|error| format!("VCET temporal campaign selection task failed: {error}"))?
}

#[tauri::command]
pub async fn open_vcet_temporal_campaign_final_test(
    req: OpenVcetTemporalCampaignFinalTestRequestV1,
    app: tauri::AppHandle,
) -> Result<VcetTemporalCampaignStatusV1, String> {
    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        open_vcet_temporal_campaign_final_test_blocking(req, &base)
    })
    .await
    .map_err(|error| format!("VCET temporal campaign final-test task failed: {error}"))?
}

#[tauri::command]
pub async fn get_vcet_temporal_campaign_status(
    req: GetVcetTemporalCampaignStatusRequestV1,
    app: tauri::AppHandle,
) -> Result<VcetTemporalCampaignStatusV1, String> {
    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        get_vcet_temporal_campaign_status_blocking(req, &base)
    })
    .await
    .map_err(|error| format!("VCET temporal campaign status task failed: {error}"))?
}

#[cfg(test)]
#[path = "commands_vcet_temporal_campaign_lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct ProjectionFixture {
        artifact_sha256: String,
        value: usize,
    }

    fn test_temp_dir(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "vectorgrid-vcet-temporal-campaign-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test temp directory");
        path
    }

    #[test]
    fn public_requests_reject_unknown_fields_and_final_requires_explicit_boolean() {
        let status =
            serde_json::from_value::<GetVcetTemporalCampaignStatusRequestV1>(serde_json::json!({
                "preparation_artifact_path": "preparation.json",
                "expected_preparation_artifact_sha256": "a".repeat(64),
                "unexpected": true,
            }));
        assert!(status.is_err());

        let final_request = serde_json::from_value::<OpenVcetTemporalCampaignFinalTestRequestV1>(
            serde_json::json!({
                "preparation_artifact_path": "preparation.json",
                "expected_preparation_artifact_sha256": "a".repeat(64),
                "expected_winner_lock_sha256": "b".repeat(64),
            }),
        );
        assert!(final_request.is_err());
        let explicit_false = serde_json::from_value::<OpenVcetTemporalCampaignFinalTestRequestV1>(
            serde_json::json!({
                "preparation_artifact_path": "preparation.json",
                "expected_preparation_artifact_sha256": "a".repeat(64),
                "expected_winner_lock_sha256": "b".repeat(64),
                "confirm_final_test_access": false,
            }),
        )
        .expect("explicit false is structurally present");
        assert!(!explicit_false.confirm_final_test_access);
    }

    #[test]
    fn product_source_summary_rejects_foreign_and_legacy_kinds_before_shape_decode() {
        for kind in ["vectorgrid_run_artifact", "vectorbt_campaign_summary", ""] {
            let error = validate_product_source_campaign_summary_value(&serde_json::json!({
                "kind": kind,
                "version": 2,
            }))
            .expect_err("foreign kind must not bypass product campaign validation");
            assert!(error.contains("vectorgrid_campaign_summary"));
        }
    }

    #[test]
    fn report_labels_are_bounded_and_canonical() {
        assert_eq!(
            canonical_report_label(Some("  research run  ".to_string())).unwrap(),
            Some("research run".to_string())
        );
        assert_eq!(
            canonical_report_label(Some("  ".to_string())).unwrap(),
            None
        );
        assert!(canonical_report_label(Some("x".repeat(129))).is_err());
        assert!(canonical_report_label(Some("line\nbreak".to_string())).is_err());
    }

    #[test]
    fn product_boundary_policy_rejects_unimplemented_semantics_at_preparation() {
        assert!(validate_product_position_boundary_policy(
            VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen
        )
        .is_ok());
        assert!(validate_product_position_boundary_policy(
            VcetTemporalPositionBoundaryPolicyV1::ResetFlatForceLiquidate
        )
        .is_err());
        assert!(validate_product_position_boundary_policy(
            VcetTemporalPositionBoundaryPolicyV1::ResetFlatRequireClosed
        )
        .is_err());
    }

    #[test]
    fn source_dataset_lineage_is_deterministic_and_binds_artifact_and_identity() {
        let input = ResearchCampaignDatasetInput {
            csv_path: "C:\\data\\AAPL.csv".to_string(),
            label: Some("AAPL".to_string()),
            timeframe: Some("1m".to_string()),
            source_size_bytes: Some(42),
            source_mtime_utc_ms: Some(7),
            source_change_marker: Some(9),
            source_content_fingerprint: Some("fnv1a64:abc".to_string()),
        };
        let first = source_dataset_sha256("dataset-a", &input, "1m", &"a".repeat(64))
            .expect("source lineage");
        let second = source_dataset_sha256("dataset-a", &input, "1m", &"a".repeat(64))
            .expect("source lineage replay");
        assert_eq!(first, second);
        assert_ne!(
            first,
            source_dataset_sha256("dataset-a", &input, "1m", &"b".repeat(64))
                .expect("changed artifact lineage")
        );
        let mut changed = input;
        changed.source_change_marker = Some(10);
        assert_ne!(
            second,
            source_dataset_sha256("dataset-a", &changed, "1m", &"a".repeat(64))
                .expect("changed source identity")
        );
    }

    #[test]
    fn product_limits_are_explicit_in_sealed_limitations() {
        let limitations = product_limitations();
        assert!(limitations
            .iter()
            .any(|value| value.contains(&MAX_VCET_TEMPORAL_PRODUCT_DATASETS.to_string())));
        assert!(limitations
            .iter()
            .any(|value| value
                .contains(&MAX_VCET_TEMPORAL_PRODUCT_TOTAL_TIMELINE_VALUES.to_string())));
        assert!(limitations.iter().any(|value| value
            .contains(&MAX_VCET_TEMPORAL_PRODUCT_TIMELINE_VALUES_PER_DATASET.to_string())));
        assert!(limitations
            .iter()
            .any(|value| value
                .contains(&MAX_VCET_TEMPORAL_PRODUCT_CANDIDATE_WINDOW_TRIALS.to_string())));
        assert!(limitations.iter().any(|value| value
            .contains(&MAX_VCET_TEMPORAL_PRODUCT_SOURCE_TRAINING_ARTIFACT_BYTES.to_string())));
    }

    #[test]
    fn product_state_and_resume_actions_do_not_overstate_missing_reports() {
        assert_eq!(
            product_campaign_state(true, false, false, false),
            VcetTemporalCampaignStateV1::Prepared
        );
        assert_eq!(
            product_next_required_action(
                VcetTemporalCampaignStateV1::Prepared,
                false,
                true,
                false,
                false,
                false,
                3,
                3,
                0,
            ),
            "resume_selection_report"
        );
        assert_eq!(
            product_campaign_state(true, true, true, false),
            VcetTemporalCampaignStateV1::SelectionComplete
        );
        assert_eq!(
            product_next_required_action(
                VcetTemporalCampaignStateV1::SelectionComplete,
                false,
                true,
                true,
                true,
                false,
                3,
                3,
                2,
            ),
            "confirm_and_resume_final_report"
        );
        assert_eq!(
            product_next_required_action(
                VcetTemporalCampaignStateV1::SelectionComplete,
                true,
                true,
                true,
                false,
                false,
                3,
                3,
                1,
            ),
            BLOCKED_CAMPAIGN_ACTION
        );
        assert_eq!(
            product_campaign_state(true, true, true, true),
            VcetTemporalCampaignStateV1::FinalTestComplete
        );
        assert_eq!(
            product_campaign_state(true, false, true, true),
            VcetTemporalCampaignStateV1::FinalTestComplete,
            "late final evidence remains visible alongside a selection-report block"
        );
        assert_eq!(
            product_campaign_state(true, false, true, false),
            VcetTemporalCampaignStateV1::SelectionComplete,
            "a final reduction proves the campaign crossed the selection boundary"
        );
        assert_eq!(
            product_next_required_action(
                VcetTemporalCampaignStateV1::Prepared,
                false,
                true,
                true,
                false,
                false,
                2,
                3,
                0,
            ),
            "resume_selection",
            "a durable winner/report cannot skip a missing selection projection"
        );
        assert_eq!(
            product_next_required_action(
                VcetTemporalCampaignStateV1::Prepared,
                false,
                true,
                false,
                false,
                false,
                0,
                3,
                0,
            ),
            "resume_selection",
            "a durable pre-final authority makes zero local shard projections resumable"
        );
        assert_eq!(
            product_next_required_action(
                VcetTemporalCampaignStateV1::Prepared,
                false,
                false,
                false,
                false,
                false,
                0,
                3,
                0,
            ),
            "run_selection",
            "a new campaign with no durable pre-final authority starts selection"
        );
        assert_eq!(
            product_campaign_state_with_recovery_evidence(
                VcetTemporalCampaignStateV1::FinalTestComplete,
                true,
                true,
            ),
            VcetTemporalCampaignStateV1::FinalTestComplete,
            "late final artifacts retain their actual stage alongside blocked evidence"
        );
        assert_eq!(
            product_campaign_state_with_recovery_evidence(
                VcetTemporalCampaignStateV1::SelectionComplete,
                false,
                true,
            ),
            VcetTemporalCampaignStateV1::SelectionComplete
        );
        assert_eq!(
            product_campaign_state_with_recovery_evidence(
                VcetTemporalCampaignStateV1::SelectionComplete,
                false,
                false,
            ),
            VcetTemporalCampaignStateV1::Prepared
        );
    }

    #[test]
    fn temporal_report_writer_output_is_accepted_by_campaign_receipt_contract() {
        let base = test_temp_dir("report-writer-receipt-contract");
        for stage in ["selection", "final"] {
            let saved = commands_reports::persist_vcet_temporal_report_bytes_for_test(
                stage,
                Some("Campaign / receipt"),
                &format!("sha256:{}", "a".repeat(64)),
                &base,
                b"xlsx-test",
            )
            .expect("persist report through the production naming writer");

            let file_name = saved
                .file_name()
                .and_then(|value| value.to_str())
                .expect("UTF-8 report filename");
            assert!(
                commands_reports::vcet_temporal_report_filename_matches_stage(file_name, stage)
            );
            let (canonical, bytes) = validate_app_owned_report_path(
                saved.to_str().expect("UTF-8 report path"),
                &base,
                stage,
            )
            .expect("writer output must satisfy campaign receipt validation");
            assert_eq!(
                canonical,
                fs::canonicalize(&saved).expect("canonical report")
            );
            assert_eq!(bytes, 9);
        }

        fs::remove_dir_all(base).expect("remove writer-receipt fixture");
    }

    #[test]
    fn report_paths_must_be_direct_app_owned_xlsx_files() {
        let base = test_temp_dir("report-paths");
        let reports = commands_reports::reports_dir_for_base(&base).expect("reports directory");
        let valid = reports.join("vectorgrid_vcet_temporal_selection_valid.xlsx");
        fs::write(&valid, b"xlsx-test").expect("write valid report fixture");
        let (canonical, bytes) = validate_app_owned_report_path(
            valid.to_str().expect("UTF-8 test report path"),
            &base,
            "selection",
        )
        .expect("direct app-owned report");
        assert_eq!(
            canonical,
            fs::canonicalize(&valid).expect("canonical report")
        );
        assert_eq!(bytes, 9);

        let nested_dir = reports.join("nested");
        fs::create_dir_all(&nested_dir).expect("create nested reports fixture");
        let nested = nested_dir.join("vectorgrid_vcet_temporal_selection_nested.xlsx");
        fs::write(&nested, b"nested").expect("write nested report fixture");
        assert!(validate_app_owned_report_path(
            nested.to_str().expect("UTF-8 nested path"),
            &base,
            "selection"
        )
        .is_err());

        let sibling_dir = base.join("reports_evil");
        fs::create_dir_all(&sibling_dir).expect("create sibling reports fixture");
        let sibling = sibling_dir.join("vectorgrid_vcet_temporal_selection_outside.xlsx");
        fs::write(&sibling, b"outside").expect("write sibling report fixture");
        assert!(validate_app_owned_report_path(
            sibling.to_str().expect("UTF-8 sibling path"),
            &base,
            "selection"
        )
        .is_err());

        let wrong_extension = reports.join("vectorgrid_vcet_temporal_selection_wrong.json");
        fs::write(&wrong_extension, b"wrong").expect("write wrong-extension fixture");
        assert!(validate_app_owned_report_path(
            wrong_extension
                .to_str()
                .expect("UTF-8 wrong-extension path"),
            &base,
            "selection"
        )
        .is_err());
        let wrong_stage = reports.join("vectorgrid_vcet_temporal_final_wrong-stage.xlsx");
        fs::write(&wrong_stage, b"wrong-stage").expect("write wrong-stage fixture");
        assert!(validate_app_owned_report_path(
            wrong_stage.to_str().expect("UTF-8 wrong-stage path"),
            &base,
            "selection"
        )
        .is_err());
        assert!(validate_app_owned_report_path(
            "vectorgrid_vcet_temporal_selection_relative.xlsx",
            &base,
            "selection"
        )
        .is_err());
        let ads = reports.join("vectorgrid_vcet_temporal_selection_valid.xlsx:payload.xlsx");
        assert!(validate_app_owned_report_path(
            ads.to_str().expect("UTF-8 ADS path"),
            &base,
            "selection"
        )
        .is_err());
        fs::remove_dir_all(&base).expect("remove report path fixtures");
    }

    #[cfg(windows)]
    #[test]
    fn report_directory_symlink_is_rejected_before_writer_access() {
        let base = test_temp_dir("report-dir-symlink");
        let outside = test_temp_dir("report-dir-symlink-target");
        let reports = base.join("reports");
        if std::os::windows::fs::symlink_dir(&outside, &reports).is_ok() {
            assert!(preflight_app_owned_reports_dir(&base).is_err());
            assert_eq!(
                fs::read_dir(&outside)
                    .expect("read untouched symlink target")
                    .count(),
                0
            );
            fs::remove_dir(&reports).expect("remove reports symlink");
        }
        fs::remove_dir_all(&base).expect("remove report symlink base");
        fs::remove_dir_all(&outside).expect("remove report symlink target");
    }

    #[cfg(windows)]
    #[test]
    fn campaign_store_reparse_directory_is_rejected_before_artifact_access() {
        let base = test_temp_dir("campaign-store-reparse");
        let outside = test_temp_dir("campaign-store-reparse-target");
        let store = base.join(CAMPAIGN_DIR_NAME);
        if std::os::windows::fs::symlink_dir(&outside, &store).is_ok() {
            let metadata = fs::symlink_metadata(&store).expect("inspect campaign reparse fixture");
            assert!(metadata_is_reparse_point(&metadata));
            let error = campaign_store_root(&base)
                .expect_err("campaign store reparse point must be rejected");
            assert!(error.contains("non-reparse"));
            assert_eq!(
                fs::read_dir(&outside)
                    .expect("read untouched campaign reparse target")
                    .count(),
                0
            );
            fs::remove_dir(&store).expect("remove campaign store reparse fixture");
        }
        fs::remove_dir_all(&base).expect("remove campaign store reparse base");
        fs::remove_dir_all(&outside).expect("remove campaign store reparse target");
    }

    #[test]
    fn bounded_artifact_reads_and_hashes_reject_max_plus_one_without_path_leakage() {
        let base = test_temp_dir("bounded-artifact-io");
        let exact = base.join("exact.json");
        fs::write(&exact, b"12345678").expect("write exact-size artifact");
        assert_eq!(
            read_bounded_artifact_bytes(&exact, 8).expect("read exact-size artifact"),
            Some(b"12345678".to_vec())
        );
        assert!(file_sha256(&exact, 8)
            .expect("hash exact-size artifact")
            .starts_with("sha256:"));

        let oversized = base.join("oversized-secret.json");
        fs::write(&oversized, b"123456789").expect("write max-plus-one artifact");
        assert_eq!(
            read_bounded_artifact_bytes(&oversized, 8),
            Err(ArtifactAccessFailureV1::Invalid)
        );
        let error = file_sha256(&oversized, 8).expect_err("max-plus-one hash must fail closed");
        assert!(error.contains("bounded safety contract"));
        assert!(!error.contains("oversized-secret"));
        fs::remove_dir_all(base).expect("remove bounded artifact fixture");
    }

    #[cfg(windows)]
    #[test]
    fn final_component_file_reparse_point_is_rejected_by_bounded_reader() {
        let base = test_temp_dir("bounded-artifact-reparse");
        let target = base.join("target.json");
        let link = base.join("projection.json");
        fs::write(&target, br#"{"value":1}"#).expect("write reparse target");
        if std::os::windows::fs::symlink_file(&target, &link).is_ok() {
            assert_eq!(
                read_bounded_artifact_bytes(&link, MAX_PRODUCT_ARTIFACT_READ_BYTES),
                Err(ArtifactAccessFailureV1::Invalid)
            );
            fs::remove_file(&link).expect("remove file reparse fixture");
        }
        fs::remove_dir_all(base).expect("remove bounded reparse fixture");
    }

    #[cfg(windows)]
    #[test]
    fn bounded_campaign_reader_excludes_writers_and_classifies_writer_contention_as_retryable() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        };

        let base = test_temp_dir("bounded-artifact-sharing");
        let path = base.join("projection.json");
        fs::write(&path, br#"{"value":1}"#).expect("write sharing fixture");

        let held_writer = fs::OpenOptions::new()
            .write(true)
            .share_mode(0)
            .open(&path)
            .expect("hold exclusive writer");
        assert_eq!(
            read_bounded_artifact_bytes(&path, MAX_PRODUCT_ARTIFACT_READ_BYTES),
            Err(ArtifactAccessFailureV1::Unavailable),
            "a live writer is retryable, never permanent corruption"
        );
        drop(held_writer);

        let (reader, _) = open_bounded_regular_artifact(&path, MAX_PRODUCT_ARTIFACT_READ_BYTES)
            .expect("open bounded reader")
            .expect("sharing fixture exists");
        let writer_error = fs::OpenOptions::new()
            .write(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .open(&path)
            .expect_err("reader must exclude concurrent mutation");
        assert_ne!(writer_error.kind(), std::io::ErrorKind::NotFound);
        drop(reader);
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("writer proceeds after authenticated reader closes");
        fs::remove_dir_all(base).expect("remove sharing fixture");
    }

    #[test]
    fn preparation_alias_is_canonicalized_for_internal_lineage_and_clean_for_display() {
        let base = test_temp_dir("preparation-canonical-alias");
        let root = campaign_store_root(&base).expect("create campaign root");
        let campaign = root.join("vcet-temporal-product-v1-alias");
        fs::create_dir_all(campaign.join("alias-segment")).expect("create alias campaign segment");
        let preparation = campaign.join(PREPARATION_FILE_NAME);
        fs::write(&preparation, b"{}").expect("write preparation alias fixture");
        let alias = campaign
            .join("alias-segment")
            .join("..")
            .join(PREPARATION_FILE_NAME);
        let canonical =
            canonical_preparation_path(alias.to_str().expect("UTF-8 preparation alias"), &base)
                .expect("canonicalize accepted preparation alias");
        assert_eq!(
            canonical,
            fs::canonicalize(&preparation).expect("canonical preparation fixture")
        );
        let display = public_path_display(&canonical);
        assert!(!display.starts_with(r"\\?\"));
        assert!(display.ends_with(PREPARATION_FILE_NAME));
        fs::remove_dir_all(base).expect("remove canonical preparation fixture");
    }

    #[test]
    fn campaign_operation_guard_rejects_concurrent_mutation_and_recovers_on_drop() {
        let shared_universe_sha256 = format!("sha256:{}", "9".repeat(64));
        let distinct_universe_sha256 = format!("sha256:{}", "8".repeat(64));
        let first = acquire_campaign_guard(&shared_universe_sha256).expect("first universe guard");
        assert!(acquire_campaign_guard(&shared_universe_sha256)
            .unwrap_err()
            .contains("already has an active operation"));
        let status_error = acquire_campaign_status_guard(&shared_universe_sha256)
            .expect_err("a campaign alias must not inspect a live same-universe mutation");
        assert_eq!(
            status_error,
            "VCET temporal campaign operation is in progress; retry status refresh"
        );
        assert!(!status_error.contains(&shared_universe_sha256));
        let distinct = acquire_campaign_guard(&distinct_universe_sha256)
            .expect("a different durable universe can proceed concurrently");
        drop(distinct);
        drop(first);
        acquire_campaign_guard(&shared_universe_sha256).expect("guard after drop");
    }

    #[test]
    fn durable_terminal_recovery_precedes_mutable_source_access_after_restart() {
        let base = test_temp_dir("durable-recovery-before-source");
        let reconciled = base.join("recovered-shard.json");
        let events = std::cell::RefCell::new(Vec::new());
        let error = recover_all_before_any_new_execution(
            2,
            |index| {
                events.borrow_mut().push(format!("recover-{index}"));
                if index == 1 {
                    fs::write(&reconciled, b"durable-terminal")
                        .expect("publish recovered campaign projection");
                    Ok(Some("durable-terminal-1".to_string()))
                } else {
                    Ok(None)
                }
            },
            |index| {
                events.borrow_mut().push(format!("source-{index}"));
                Err("source disappeared after durable completion".to_string())
            },
        )
        .expect_err("missing earlier source should fail only after all recovery probes");
        assert!(error.contains("source disappeared"));
        assert_eq!(
            events.into_inner(),
            vec![
                "recover-0".to_string(),
                "recover-1".to_string(),
                "source-0".to_string(),
            ]
        );
        assert_eq!(
            fs::read(&reconciled).expect("read recovered campaign projection"),
            b"durable-terminal"
        );
        fs::remove_dir_all(base).expect("remove recovery-order fixture");

        let poisoned_source_accessed = std::cell::Cell::new(false);
        let error = recover_all_before_any_new_execution::<String, _, _>(
            1,
            |_index| Err("durable stage is interrupted".to_string()),
            |_index| {
                poisoned_source_accessed.set(true);
                Ok("must-not-run".to_string())
            },
        )
        .expect_err("a poisoned durable stage must fail closed before source access");
        assert!(error.contains("interrupted"));
        assert!(!poisoned_source_accessed.get());

        let new_source_accessed = std::cell::Cell::new(false);
        let executed = recover_all_before_any_new_execution(
            1,
            |_index| Ok(None),
            |_index| {
                new_source_accessed.set(true);
                Ok("new-source-validated-execution".to_string())
            },
        )
        .expect("missing durable state should permit a new source-validated execution");
        assert_eq!(executed, vec!["new-source-validated-execution".to_string()]);
        assert!(new_source_accessed.get());
    }

    #[test]
    fn blocked_probe_does_not_short_circuit_later_completion_reconciliation() {
        let base = test_temp_dir("blocked-before-later-completion");
        let later_projection = base.join("final_shard_1.json");
        let source_accessed = std::cell::Cell::new(false);
        let events = std::cell::RefCell::new(Vec::new());
        let error = recover_all_before_any_new_execution::<String, _, _>(
            2,
            |index| {
                events.borrow_mut().push(format!("recover-{index}"));
                if index == 0 {
                    Err("durable stage is poisoned".to_string())
                } else {
                    fs::write(&later_projection, b"completed-terminal")
                        .expect("publish later recovered projection");
                    Ok(Some("completed-terminal".to_string()))
                }
            },
            |_index| {
                source_accessed.set(true);
                Ok("must-not-execute".to_string())
            },
        )
        .expect_err("any poisoned probe blocks new execution");
        assert!(error.contains("poisoned"));
        assert_eq!(
            events.into_inner(),
            vec!["recover-0".to_string(), "recover-1".to_string()]
        );
        assert!(!source_accessed.get());
        assert_eq!(
            fs::read(&later_projection).expect("read later recovered projection"),
            b"completed-terminal"
        );
        fs::remove_dir_all(base).expect("remove recovery fixture");
    }

    #[test]
    fn durable_completion_republishes_a_missing_projection_idempotently() {
        let base = test_temp_dir("missing-projection-recovery");
        let path = base.join("final_shard_0.json");
        let artifact = ProjectionFixture {
            artifact_sha256: format!("sha256:{}", "a".repeat(64)),
            value: 7,
        };
        let first = reconcile_durable_projection(
            &path,
            CampaignProjectionStateV1::Missing,
            DurableCompletionProbe::Completed {
                request_sha256: format!("sha256:{}", "b".repeat(64)),
                response: artifact.clone(),
            },
            "final-test shard",
            |value| value.artifact_sha256.clone(),
        )
        .expect("recover missing projection");
        assert!(!first.blocked);
        assert!(first.projection_complete);
        assert_eq!(first.artifact, Some(artifact.clone()));
        let persisted: ProjectionFixture =
            read_json_file(&path, MAX_PRODUCT_ARTIFACT_READ_BYTES).expect("read projection");
        assert_eq!(persisted, artifact);

        let second = reconcile_durable_projection(
            &path,
            CampaignProjectionStateV1::Valid(persisted),
            DurableCompletionProbe::Completed {
                request_sha256: format!("sha256:{}", "b".repeat(64)),
                response: artifact.clone(),
            },
            "final-test shard",
            |value| value.artifact_sha256.clone(),
        )
        .expect("idempotent projection reconciliation");
        assert!(!second.blocked);
        assert!(second.projection_complete);
        assert_eq!(second.artifact, Some(artifact));

        let durable = ProjectionFixture {
            artifact_sha256: format!("sha256:{}", "7".repeat(64)),
            value: 21,
        };
        let invalid = reconcile_durable_projection(
            &base.join("invalid-projection.json"),
            CampaignProjectionStateV1::Invalid,
            DurableCompletionProbe::Completed {
                request_sha256: format!("sha256:{}", "6".repeat(64)),
                response: durable.clone(),
            },
            "final-test shard",
            |value| value.artifact_sha256.clone(),
        )
        .expect("classify invalid projection after probing durable completion");
        assert!(invalid.blocked);
        assert!(!invalid.projection_complete);
        assert_eq!(
            invalid.artifact,
            Some(durable),
            "validated durable authority remains available to probe later stages"
        );

        let same_path = base.join("final_shard_same.json");
        publish_new_json(
            &same_path,
            &ProjectionFixture {
                artifact_sha256: format!("sha256:{}", "e".repeat(64)),
                value: 13,
            },
        )
        .expect("publish exact race fixture");
        assert_eq!(
            publish_recovered_projection(
                &same_path,
                &ProjectionFixture {
                    artifact_sha256: format!("sha256:{}", "e".repeat(64)),
                    value: 13,
                },
                "final-test shard",
            )
            .expect("verify exact concurrent publication"),
            RecoveredProjectionPublishV1::PublishedOrVerified
        );
        assert_eq!(
            publish_recovered_projection(
                &same_path,
                &ProjectionFixture {
                    artifact_sha256: format!("sha256:{}", "f".repeat(64)),
                    value: 14,
                },
                "final-test shard",
            )
            .expect("mismatch is a classified integrity conflict"),
            RecoveredProjectionPublishV1::Conflict
        );
        fs::remove_dir_all(base).expect("remove projection fixture");
    }

    #[test]
    fn final_reduction_completeness_is_checked_after_recovered_projection_counting() {
        let base = test_temp_dir("final-reduction-present-recovery");
        let final_reduction = base.join(FINAL_REDUCTION_FILE_NAME);
        fs::write(&final_reduction, b"sealed-final-reduction")
            .expect("write final reduction sentinel");
        let path = base.join("final_shard_0.json");
        let artifact = ProjectionFixture {
            artifact_sha256: format!("sha256:{}", "c".repeat(64)),
            value: 11,
        };
        let reconciliation = reconcile_durable_projection(
            &path,
            CampaignProjectionStateV1::Missing,
            DurableCompletionProbe::Completed {
                request_sha256: format!("sha256:{}", "d".repeat(64)),
                response: artifact,
            },
            "final-test shard",
            |value| value.artifact_sha256.clone(),
        )
        .expect("recover projection before reduction completeness");
        let mut completed_count = 0usize;
        if reconciliation.projection_complete {
            completed_count += 1;
        }
        assert_eq!(completed_count, 1, "recovery must increment status count");
        assert!(final_reduction.exists());
        assert!(path.exists());
        fs::remove_dir_all(base).expect("remove final reduction recovery fixture");
    }

    #[test]
    fn durable_execution_blocks_are_safe_and_never_advertise_resume() {
        let blocked = VcetTemporalCampaignExecutionBlock {
            stage: "final_test_shard",
            shard_ordinal: Some(7),
            reason_code: DURABLE_STAGE_NOT_RECOVERABLE_REASON,
        };
        assert_eq!(blocked.stage, "final_test_shard");
        assert_eq!(blocked.shard_ordinal, Some(7));
        assert_eq!(
            blocked.reason_code,
            "durable_stage_failed_interrupted_or_invalid"
        );
        assert!(!blocked.reason_code.contains('\\'));
        assert!(!blocked.reason_code.contains('/'));
        assert_eq!(
            product_next_required_action(
                VcetTemporalCampaignStateV1::SelectionComplete,
                true,
                true,
                true,
                false,
                false,
                2,
                2,
                0,
            ),
            "start_new_campaign"
        );

        for (stage, reason) in [
            ("selection_report", REPORT_RECEIPT_NOT_RECOVERABLE_REASON),
            ("final_report", REPORT_XLSX_NOT_RECOVERABLE_REASON),
        ] {
            let report_block = report_execution_block(stage, reason);
            assert_eq!(report_block.stage, stage);
            assert_eq!(report_block.reason_code, reason);
            assert_eq!(report_block.shard_ordinal, None);
            assert!(!report_block.reason_code.contains('\\'));
            assert!(!report_block.reason_code.contains('/'));
        }
    }

    #[test]
    fn blocked_status_serialization_is_additive_compact_and_path_safe() {
        let mut status = VcetTemporalCampaignStatusV1 {
            schema_version: VCET_TEMPORAL_PRODUCT_CAMPAIGN_SCHEMA_VERSION,
            campaign_id: "vcet-temporal-product-v1-test".to_string(),
            state: VcetTemporalCampaignStateV1::SelectionComplete,
            preparation_artifact_path: "preparation.json".to_string(),
            preparation_artifact_sha256: format!("sha256:{}", "a".repeat(64)),
            source_campaign_summary_path: "summary.json".to_string(),
            source_campaign_summary_sha256: format!("sha256:{}", "b".repeat(64)),
            universe_sha256: format!("sha256:{}", "c".repeat(64)),
            candidate_count: 1,
            dataset_count: 2,
            selection_dataset_count: 1,
            final_test_dataset_count: 1,
            selection_shard_count: 1,
            completed_selection_shard_count: 1,
            final_test_shard_count: 1,
            completed_final_test_shard_count: 0,
            winner_candidate_id: Some("winner".to_string()),
            winner_lock_sha256: Some(format!("sha256:{}", "d".repeat(64))),
            winner_runner_up_margin: None,
            pre_final_reduction_artifact_path: Some("pre_final_reduction.json".to_string()),
            pre_final_reduction_artifact_sha256: Some(format!("sha256:{}", "e".repeat(64))),
            final_reduction_artifact_path: None,
            final_reduction_artifact_sha256: None,
            selection_report_xlsx_path: Some("selection.xlsx".to_string()),
            final_report_xlsx_path: None,
            execution_blocked: true,
            blocked_stage: Some("final_test_shard".to_string()),
            blocked_shard_ordinal: Some(3),
            blocked_reason_code: Some(DURABLE_STAGE_NOT_RECOVERABLE_REASON.to_string()),
            next_required_action: BLOCKED_CAMPAIGN_ACTION.to_string(),
            limitations: Vec::new(),
        };
        let blocked = serde_json::to_value(&status).expect("serialize blocked status");
        assert_eq!(blocked["execution_blocked"], serde_json::json!(true));
        assert_eq!(blocked["blocked_stage"], "final_test_shard");
        assert_eq!(blocked["blocked_shard_ordinal"], 3);
        assert_eq!(
            blocked["blocked_reason_code"],
            "durable_stage_failed_interrupted_or_invalid"
        );
        assert_eq!(blocked["next_required_action"], "start_new_campaign");

        status.execution_blocked = false;
        status.blocked_stage = None;
        status.blocked_shard_ordinal = None;
        status.blocked_reason_code = None;
        status.next_required_action = "confirm_and_open_final_test".to_string();
        let unblocked = serde_json::to_value(status).expect("serialize unblocked status");
        assert_eq!(unblocked["execution_blocked"], serde_json::json!(false));
        assert!(unblocked.get("blocked_stage").is_none());
        assert!(unblocked.get("blocked_shard_ordinal").is_none());
        assert!(unblocked.get("blocked_reason_code").is_none());
    }

    #[test]
    fn sha256_normalization_accepts_raw_or_prefixed_and_rejects_malformed() {
        let raw = "a".repeat(64);
        assert_eq!(
            normalize_sha256(&raw, "hash").unwrap(),
            format!("sha256:{raw}")
        );
        assert_eq!(
            normalize_sha256(&format!("sha256:{}", "A".repeat(64)), "hash").unwrap(),
            format!("sha256:{}", "a".repeat(64))
        );
        assert!(normalize_sha256("abc", "hash").is_err());
    }
}
