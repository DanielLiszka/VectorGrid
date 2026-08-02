use super::commands_vcet_temporal::{
    plan_vcet_temporal_validation_blocking, PlanVcetTemporalValidationRequestV1,
    PlanVcetTemporalValidationResponseV1,
};
use super::commands_vcet_temporal_execute::{
    candidate_params_from_manifest, evaluate_window, loaded_candles_snapshot_sha256,
    profile_loaded_dataset_coverage_for_planning, validate_loaded_dataset_shape,
    VcetTemporalLoadedDatasetBindingV1, MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST,
};
use super::commands_vcet_temporal_prefinal::{
    validate_vcet_temporal_final_shard_authorization,
    validate_vcet_temporal_pre_final_reduction_artifact,
    ReduceVcetTemporalPreFinalShardsResponseV1, VcetTemporalFinalShardAuthorizationV1,
    VcetTemporalGlobalWinnerLockV1,
};
use super::commands_vcet_temporal_universe::{
    recompute_vcet_dataset_coverage_sha256, recompute_vcet_temporal_interval_fingerprint_sha256,
    universe_entry_map, universe_shards_for_role, validate_sealed_vcet_temporal_universe,
    VcetTemporalDatasetCohortRoleV1, VcetTemporalPhasedExecutionSpecV1,
    VcetTemporalSealedDatasetUniverseV1, VcetTemporalUniverseDatasetV1,
    VcetTemporalUniverseShardV1,
};
use super::vcet_temporal_durable_ledger::{
    admit_durable_final_authorization, load_durable_final_completion_for_authorization,
    load_durable_pre_final_winner_lock, DurableFinalAdmission, DurableFinalClaim,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use ta_app_core::composite_validate::{CompositeParams, VcetFixedCandidateWindowBackendProvenance};
use ta_app_core::double_ma::Backend;
use ta_app_core::vcet_dataset_coverage::VcetDatasetCoverageEvidence;
use ta_app_core::vcet_temporal::{
    VcetTemporalEvaluationSliceV1, VcetTemporalPositionBoundaryPolicyV1,
};
use ta_strategies::double_ma::Metrics;
use tauri::Manager;
use vector_ta::utilities::data_loader::Candles;

pub const VCET_TEMPORAL_FINAL_SHARD_ARTIFACT_SCHEMA_VERSION: u16 = 1;
pub const VCET_TEMPORAL_FINAL_REDUCTION_ARTIFACT_SCHEMA_VERSION: u16 = 1;

const FINAL_SHARD_REQUEST_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_final_shard_request.v1";
const FINAL_SHARD_COMPLETED_STATE_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_final_shard_completed_state.v1";
const FINAL_SHARD_ARTIFACT_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_final_shard_artifact.v1";
const FINAL_SHARD_SET_ROOT_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_final_shard_set_root.v1";
const FINAL_REDUCTION_ARTIFACT_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_final_reduction_artifact.v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteVcetTemporalFinalTestShardRequestV1 {
    pub sealed_universe: VcetTemporalSealedDatasetUniverseV1,
    pub expected_universe_sha256: String,
    pub global_winner_reduction: ReduceVcetTemporalPreFinalShardsResponseV1,
    pub expected_global_winner_reduction_artifact_sha256: String,
    pub final_shard: VcetTemporalUniverseShardV1,
    pub planning_request: PlanVcetTemporalValidationRequestV1,
    pub planning_evidence: PlanVcetTemporalValidationResponseV1,
    pub expected_planning_evidence_sha256: String,
    pub loaded_datasets: Vec<VcetTemporalLoadedDatasetBindingV1>,
    pub expected_authorized_prior_state_sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadVcetTemporalFinalTestShardRequestV1 {
    pub authorization_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalFinalScoringSemanticsV1 {
    pub scored_interval_semantics: String,
    pub indicator_history_semantics: String,
    pub execution_start_semantics: String,
    pub scored_end_semantics: String,
    pub trade_support_basis: String,
    pub position_boundary_policy: VcetTemporalPositionBoundaryPolicyV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VcetTemporalFinalSupportStatusV1 {
    SupportPassed,
    SupportFailed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalFinalTestDatasetExecutionV1 {
    pub dataset_id: String,
    pub loaded_data_id: String,
    pub source_dataset_sha256: String,
    pub dataset_attestation_sha256: String,
    pub timeline_sha256: String,
    pub loaded_candles_snapshot_sha256: String,
    pub dataset_coverage: VcetDatasetCoverageEvidence,
    pub dataset_coverage_sha256: String,
    pub temporal_interval_fingerprint_sha256: String,
    pub final_test: VcetTemporalEvaluationSliceV1,
    pub global_candidate_ordinal: usize,
    pub candidate_id: String,
    pub params: CompositeParams,
    pub metrics: Metrics,
    pub backend: VcetFixedCandidateWindowBackendProvenance,
    pub observed_trades: u64,
    pub minimum_trades: u64,
    pub minimum_trade_support_passed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalFinalTestShardArtifactPayloadV1 {
    pub schema_version: u16,
    pub evidence_kind: String,
    pub universe_sha256: String,
    pub global_winner_reduction_artifact_sha256: String,
    pub global_pre_final_evidence_sha256: String,
    pub winner_lock: VcetTemporalGlobalWinnerLockV1,
    pub authorization_sha256: String,
    pub final_stage_nonce: String,
    pub final_shard: VcetTemporalUniverseShardV1,
    pub planning_evidence_sha256: String,
    pub execution_spec_sha256: String,
    pub execution_spec: VcetTemporalPhasedExecutionSpecV1,
    pub temporal_interval_fingerprint_sha256: String,
    pub scoring_semantics: VcetTemporalFinalScoringSemanticsV1,
    pub request_sha256: String,
    pub authorized_prior_state_sha256: String,
    pub completed_state_sha256: String,
    pub expected_candidate_window_trial_count: usize,
    pub evaluated_candidate_window_trial_count: usize,
    pub physical_shard_execution_count: usize,
    pub candidate_count: usize,
    pub datasets: Vec<VcetTemporalFinalTestDatasetExecutionV1>,
    pub support_status: VcetTemporalFinalSupportStatusV1,
    pub minimum_trade_support_passed: bool,
    pub selection_used_final_test: bool,
    pub winner_reranked_after_final_test: bool,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteVcetTemporalFinalTestShardResponseV1 {
    pub payload: VcetTemporalFinalTestShardArtifactPayloadV1,
    pub artifact_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReduceVcetTemporalFinalTestShardsRequestV1 {
    pub sealed_universe: VcetTemporalSealedDatasetUniverseV1,
    pub expected_universe_sha256: String,
    pub global_winner_reduction: ReduceVcetTemporalPreFinalShardsResponseV1,
    pub expected_global_winner_reduction_artifact_sha256: String,
    pub final_shard_artifacts: Vec<ExecuteVcetTemporalFinalTestShardResponseV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalFinalShardReductionEvidenceV1 {
    pub shard_ordinal: usize,
    pub shard_id: String,
    pub authorization_sha256: String,
    pub authorized_prior_state_sha256: String,
    pub completed_state_sha256: String,
    pub artifact_sha256: String,
    pub dataset_count: usize,
    pub candidate_window_trial_count: usize,
    pub minimum_trade_support_passed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalFinalReductionArtifactPayloadV1 {
    pub schema_version: u16,
    pub evidence_kind: String,
    pub universe_sha256: String,
    pub global_winner_reduction_artifact_sha256: String,
    pub global_pre_final_evidence_sha256: String,
    pub winner_lock: VcetTemporalGlobalWinnerLockV1,
    pub final_stage_nonce: String,
    pub execution_spec_sha256: String,
    pub execution_spec: VcetTemporalPhasedExecutionSpecV1,
    pub temporal_interval_fingerprint_sha256: String,
    pub scoring_semantics: VcetTemporalFinalScoringSemanticsV1,
    pub logical_final_scoring_stage_open_count: usize,
    pub physical_final_shard_count: usize,
    pub final_test_candidate_count: usize,
    pub expected_final_test_dataset_count: usize,
    pub evaluated_final_test_dataset_count: usize,
    pub final_test_candidate_window_trial_count: usize,
    pub shard_set_root_sha256: String,
    pub shards: Vec<VcetTemporalFinalShardReductionEvidenceV1>,
    pub datasets: Vec<VcetTemporalFinalTestDatasetExecutionV1>,
    pub support_status: VcetTemporalFinalSupportStatusV1,
    pub minimum_trade_support_passed: bool,
    pub selection_used_final_test: bool,
    pub winner_reranked_after_final_test: bool,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReduceVcetTemporalFinalTestShardsResponseV1 {
    pub payload: VcetTemporalFinalReductionArtifactPayloadV1,
    pub artifact_sha256: String,
}

#[derive(Serialize)]
struct HashEnvelope<'a, T> {
    hash_contract: &'static str,
    value: &'a T,
}

#[derive(Serialize)]
struct CompletedStateHashInput<'a> {
    hash_contract: &'static str,
    authorization_sha256: &'a str,
    request_sha256: &'a str,
    authorized_prior_state_sha256: &'a str,
    universe_sha256: &'a str,
    winner_lock_sha256: &'a str,
    final_shard: &'a VcetTemporalUniverseShardV1,
    datasets: &'a [VcetTemporalFinalTestDatasetExecutionV1],
    expected_candidate_window_trial_count: usize,
    evaluated_candidate_window_trial_count: usize,
}

#[derive(Serialize)]
struct FinalShardSetRootInput<'a> {
    hash_contract: &'static str,
    universe_sha256: &'a str,
    winner_lock_sha256: &'a str,
    final_stage_nonce: &'a str,
    shards: &'a [VcetTemporalFinalShardReductionEvidenceV1],
}

fn sha256_prefixed(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(71);
    out.push_str("sha256:");
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_canonical_final_shard_id(value: &str) -> bool {
    value
        .strip_prefix("vcet-temporal-shard-v1-")
        .is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

fn hash_serializable(value: &impl Serialize, label: &str) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_prefixed(&bytes))
        .map_err(|error| format!("failed to serialize {label} for SHA-256: {error}"))
}

fn hash_enveloped(
    hash_contract: &'static str,
    value: &impl Serialize,
    label: &str,
) -> Result<String, String> {
    hash_serializable(
        &HashEnvelope {
            hash_contract,
            value,
        },
        label,
    )
}

fn serialized_eq(left: &impl Serialize, right: &impl Serialize) -> Result<bool, String> {
    let left = serde_json::to_vec(left)
        .map_err(|error| format!("failed to serialize left canonical value: {error}"))?;
    let right = serde_json::to_vec(right)
        .map_err(|error| format!("failed to serialize right canonical value: {error}"))?;
    Ok(left == right)
}

fn final_scoring_semantics(
    policy: VcetTemporalPositionBoundaryPolicyV1,
) -> VcetTemporalFinalScoringSemanticsV1 {
    VcetTemporalFinalScoringSemanticsV1 {
        scored_interval_semantics: "half_open_[start_index,end_index_exclusive)".to_string(),
        indicator_history_semantics: "full_loaded_dataset_history_available_before_scored_start"
            .to_string(),
        execution_start_semantics: "reset_flat_at_scored_start".to_string(),
        scored_end_semantics: "discard_open_position_without_forced_liquidation".to_string(),
        trade_support_basis: "metrics.trades_transition_count_per_scored_window".to_string(),
        position_boundary_policy: policy,
    }
}

fn final_shard_limitations() -> Vec<String> {
    vec![
        "winner_and_execution_spec_are_consumed_only_from_the_verified_universe_and_global_winner_lock"
            .to_string(),
        "the_final_test_shard_evaluates_exactly_one_locked_candidate_and_never_ranks_or_reranks"
            .to_string(),
        "the_authorization_is_consumed_before_actual_final_candle_lookup_snapshot_verification_or_scoring"
            .to_string(),
        "gpu_metric_fields_listed_as_unavailable_in_backend_provenance_are_not_claimed_as_full_gpu_trade_distribution_evidence"
            .to_string(),
        "the_durable_local_final_path_uses_a_staged_synced_no_clobber_hard_link_claim_and_one_immutable_discriminated_terminal_so_cooperating_local_processes_replay_completed_records_identically_after_process_restart_and_interrupted_claims_remain_fail_closed_while_the_ledger_is_intact"
            .to_string(),
        "the_durable_local_final_path_requires_the_supplied_global_winner_reduction_to_match_the_durable_pre_final_winner_lock_receipt_before_final_access"
            .to_string(),
        "the_direct_blocking_headless_helper_retains_process_local_one_shot_semantics_so_the_artifact_alone_does_not_prove_desktop_durable_authorization"
            .to_string(),
        "durable_completed_responses_are_currently_stored_inline_with_a_256_mib_safety_limit_and_should_later_compact_to_validated_artifact_references"
            .to_string(),
        "the_local_ledger_attempts_best_effort_parent_namespace_sync_but_does_not_guarantee_survival_of_sudden_power_loss_filesystem_rollback_storage_loss_or_storage_corruption"
            .to_string(),
        "sha256_receipt_hashes_are_unkeyed_integrity_checks_not_signatures_or_macs_so_an_attacker_with_local_write_access_can_forge_or_replace_ledger_state"
            .to_string(),
        "the_local_ledger_does_not_provide_distributed_or_cross_machine_exactly_once_execution"
            .to_string(),
    ]
}

fn final_reduction_limitations() -> Vec<String> {
    vec![
        "the_global_final_reducer_validates_and_combines_already_scored_shards_and_never_executes_candidates"
            .to_string(),
        "support_failure_is_reported_as_final_evidence_and_does_not_unlock_selection_or_trigger_reranking"
            .to_string(),
        "physical_final_shards_share_one_sealed_nonce_and_are_reported_as_one_logical_final_scoring_stage_open"
            .to_string(),
        "the_durable_local_final_path_coordinates_cooperating_local_processes_replays_completed_records_identically_after_process_restart_and_leaves_interrupted_claims_fail_closed_while_the_ledger_is_intact_but_direct_headless_helpers_remain_process_local"
            .to_string(),
        "the_durable_local_reducer_enumerates_every_sealed_authorization_and_requires_its_durable_completed_receipt_while_the_direct_blocking_reducer_relies_on_process_local_completed_authority"
            .to_string(),
        "the_local_ledger_attempts_best_effort_parent_namespace_sync_but_does_not_guarantee_survival_of_sudden_power_loss_filesystem_rollback_storage_loss_or_storage_corruption"
            .to_string(),
        "sha256_receipt_hashes_are_unkeyed_integrity_checks_not_signatures_or_macs_so_an_attacker_with_local_write_access_can_forge_or_replace_ledger_state"
            .to_string(),
        "the_local_ledger_does_not_provide_distributed_or_cross_machine_exactly_once_execution"
            .to_string(),
    ]
}

#[derive(Clone)]
struct LoadedFinalDataset {
    binding: VcetTemporalLoadedDatasetBindingV1,
    candles: Arc<Candles>,
}

struct PreparedFinalDataset {
    binding: VcetTemporalLoadedDatasetBindingV1,
    universe: VcetTemporalUniverseDatasetV1,
    final_test: VcetTemporalEvaluationSliceV1,
    candles: Arc<Candles>,
}

struct PreparedFinalShard {
    request_sha256: String,
    authorization: VcetTemporalFinalShardAuthorizationV1,
    planning_evidence_sha256: String,
    candidate_params: CompositeParams,
    candidate_id: String,
    candidate_ordinal: usize,
    datasets: Vec<PreparedFinalDataset>,
}

#[derive(Clone)]
enum FinalAuthorizationLedgerEntry {
    Running {
        request_sha256: String,
    },
    Completed {
        request_sha256: String,
        response: ExecuteVcetTemporalFinalTestShardResponseV1,
    },
    Failed {
        request_sha256: String,
        error: String,
    },
}

static FINAL_AUTHORIZATION_LEDGER: OnceLock<
    Mutex<BTreeMap<String, FinalAuthorizationLedgerEntry>>,
> = OnceLock::new();

fn final_authorization_ledger() -> &'static Mutex<BTreeMap<String, FinalAuthorizationLedgerEntry>> {
    FINAL_AUTHORIZATION_LEDGER.get_or_init(|| Mutex::new(BTreeMap::new()))
}

#[derive(Debug)]
enum LedgerAdmission {
    Execute,
    Cached(ExecuteVcetTemporalFinalTestShardResponseV1),
}

fn consume_final_authorization(
    authorization_sha256: &str,
    request_sha256: &str,
) -> Result<LedgerAdmission, String> {
    let mut ledger = final_authorization_ledger()
        .lock()
        .map_err(|_| "VCET final-test authorization ledger lock is poisoned".to_string())?;
    match ledger.get(authorization_sha256) {
        None => {
            ledger.insert(
                authorization_sha256.to_string(),
                FinalAuthorizationLedgerEntry::Running {
                    request_sha256: request_sha256.to_string(),
                },
            );
            Ok(LedgerAdmission::Execute)
        }
        Some(FinalAuthorizationLedgerEntry::Running {
            request_sha256: prior,
        }) if prior == request_sha256 => Err(format!(
            "VCET final-test authorization {authorization_sha256} is already executing; concurrent retries are rejected"
        )),
        Some(FinalAuthorizationLedgerEntry::Completed {
            request_sha256: prior,
            response,
        }) if prior == request_sha256 => Ok(LedgerAdmission::Cached(response.clone())),
        Some(FinalAuthorizationLedgerEntry::Failed {
            request_sha256: prior,
            error,
        }) if prior == request_sha256 => Err(format!(
            "VCET final-test authorization {authorization_sha256} was consumed by an execution that failed and cannot be reopened: {error}"
        )),
        Some(_) => Err(format!(
            "VCET final-test authorization {authorization_sha256} was already consumed by a different request"
        )),
    }
}

#[allow(dead_code)]
fn lookup_existing_final_authorization(
    authorization_sha256: &str,
    request_sha256: &str,
) -> Result<Option<ExecuteVcetTemporalFinalTestShardResponseV1>, String> {
    let ledger = final_authorization_ledger()
        .lock()
        .map_err(|_| "VCET final-test authorization ledger lock is poisoned".to_string())?;
    match ledger.get(authorization_sha256) {
        None => Ok(None),
        Some(FinalAuthorizationLedgerEntry::Running {
            request_sha256: prior,
        }) if prior == request_sha256 => Err(format!(
            "VCET final-test authorization {authorization_sha256} is already executing; concurrent retries are rejected"
        )),
        Some(FinalAuthorizationLedgerEntry::Completed {
            request_sha256: prior,
            response,
        }) if prior == request_sha256 => Ok(Some(response.clone())),
        Some(FinalAuthorizationLedgerEntry::Failed {
            request_sha256: prior,
            error,
        }) if prior == request_sha256 => Err(format!(
            "VCET final-test authorization {authorization_sha256} was consumed by an execution that failed and cannot be reopened: {error}"
        )),
        Some(_) => Err(format!(
            "VCET final-test authorization {authorization_sha256} was already consumed by a different request"
        )),
    }
}

#[allow(dead_code)]
fn lookup_identical_final_retry(
    req: &ExecuteVcetTemporalFinalTestShardRequestV1,
) -> Result<Option<ExecuteVcetTemporalFinalTestShardResponseV1>, String> {
    let matches = req
        .global_winner_reduction
        .payload
        .final_shard_authorizations
        .iter()
        .filter(|authorization| authorization.shard.shard_id == req.final_shard.shard_id)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Ok(None);
    }
    let request_sha256 = hash_enveloped(
        FINAL_SHARD_REQUEST_HASH_CONTRACT,
        req,
        "VCET final-test shard request",
    )?;
    lookup_existing_final_authorization(&matches[0].authorization_sha256, &request_sha256)
}

fn complete_final_authorization(
    authorization_sha256: &str,
    request_sha256: &str,
    response: ExecuteVcetTemporalFinalTestShardResponseV1,
) -> Result<(), String> {
    let mut ledger = final_authorization_ledger()
        .lock()
        .map_err(|_| "VCET final-test authorization ledger lock is poisoned".to_string())?;
    match ledger.get(authorization_sha256) {
        Some(FinalAuthorizationLedgerEntry::Running {
            request_sha256: active,
        }) if active == request_sha256 => {
            ledger.insert(
                authorization_sha256.to_string(),
                FinalAuthorizationLedgerEntry::Completed {
                    request_sha256: request_sha256.to_string(),
                    response,
                },
            );
            Ok(())
        }
        _ => Err(format!(
            "VCET final-test authorization {authorization_sha256} ledger state changed before completion"
        )),
    }
}

fn hydrate_completed_final_authorization(
    authorization_sha256: &str,
    request_sha256: &str,
    response: ExecuteVcetTemporalFinalTestShardResponseV1,
) -> Result<(), String> {
    let mut ledger = final_authorization_ledger()
        .lock()
        .map_err(|_| "VCET final-test authorization ledger lock is poisoned".to_string())?;
    match ledger.get(authorization_sha256) {
        None => {
            ledger.insert(
                authorization_sha256.to_string(),
                FinalAuthorizationLedgerEntry::Completed {
                    request_sha256: request_sha256.to_string(),
                    response,
                },
            );
            Ok(())
        }
        Some(FinalAuthorizationLedgerEntry::Completed {
            request_sha256: completed_request,
            response: completed_response,
        }) if completed_request == request_sha256
            && serialized_eq(completed_response, &response)? =>
        {
            Ok(())
        }
        Some(_) => Err(format!(
            "durable VCET final-test replay for authorization {authorization_sha256} conflicts with existing process-local state"
        )),
    }
}

fn fail_final_authorization(
    authorization_sha256: &str,
    request_sha256: &str,
    error: &str,
) -> Result<(), String> {
    let mut ledger = final_authorization_ledger()
        .lock()
        .map_err(|_| "VCET final-test authorization ledger lock is poisoned".to_string())?;
    match ledger.get(authorization_sha256) {
        Some(FinalAuthorizationLedgerEntry::Running {
            request_sha256: active,
        }) if active == request_sha256 => {
            ledger.insert(
                authorization_sha256.to_string(),
                FinalAuthorizationLedgerEntry::Failed {
                    request_sha256: request_sha256.to_string(),
                    error: error.to_string(),
                },
            );
            Ok(())
        }
        _ => Err(format!(
            "VCET final-test authorization {authorization_sha256} ledger state changed before failure recording"
        )),
    }
}

fn validate_completed_final_authorization_artifact(
    authorization_sha256: &str,
    request_sha256: &str,
    artifact: &ExecuteVcetTemporalFinalTestShardResponseV1,
) -> Result<(), String> {
    let entry = final_authorization_ledger()
        .lock()
        .map_err(|_| "VCET final-test authorization ledger lock is poisoned".to_string())?
        .get(authorization_sha256)
        .cloned();
    match entry {
        Some(FinalAuthorizationLedgerEntry::Completed {
            request_sha256: completed_request,
            response,
        }) if completed_request == request_sha256 && serialized_eq(&response, artifact)? => Ok(()),
        Some(FinalAuthorizationLedgerEntry::Completed { .. }) => Err(format!(
            "VCET final-test artifact for authorization {authorization_sha256} does not exactly match the ledger-authoritative completed response"
        )),
        Some(FinalAuthorizationLedgerEntry::Running { .. }) => Err(format!(
            "VCET final-test authorization {authorization_sha256} is still running and cannot be reduced"
        )),
        Some(FinalAuthorizationLedgerEntry::Failed { error, .. }) => Err(format!(
            "VCET final-test authorization {authorization_sha256} failed and cannot be reduced: {error}"
        )),
        None => Err(format!(
            "VCET final-test authorization {authorization_sha256} has no process-local completed-ledger authority; cross-process reduction requires a durable transactional authority"
        )),
    }
}

fn validate_exact_final_shard_descriptor(
    universe: &VcetTemporalSealedDatasetUniverseV1,
    requested: &VcetTemporalUniverseShardV1,
) -> Result<VcetTemporalUniverseShardV1, String> {
    if requested.role != VcetTemporalDatasetCohortRoleV1::FinalTest
        || !requested.locked_winner_only
        || requested.candidate_start_index != 0
        || requested.candidate_end_index_exclusive != 0
    {
        return Err(
            "VCET final-test execution requires a canonical locked-winner-only final_test shard"
                .to_string(),
        );
    }
    if requested.dataset_ids.is_empty()
        || requested.dataset_ids.len() > MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST
    {
        return Err(format!(
            "VCET final-test shard must contain between 1 and {MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST} datasets"
        ));
    }
    if requested.expected_candidate_window_trial_count != requested.dataset_ids.len() {
        return Err(
            "VCET final-test shard expected trial count must equal its dataset count for one locked winner"
                .to_string(),
        );
    }
    let matches = universe_shards_for_role(
        &universe.payload,
        VcetTemporalDatasetCohortRoleV1::FinalTest,
    )
    .into_iter()
    .filter(|candidate| candidate.shard_id == requested.shard_id)
    .collect::<Vec<_>>();
    if matches.len() != 1 || !serialized_eq(matches[0], requested)? {
        return Err(format!(
            "VCET final-test shard '{}' is not the exact canonical descriptor sealed in the universe",
            requested.shard_id
        ));
    }
    Ok(requested.clone())
}

fn validate_backend_provenance(
    backend: &VcetFixedCandidateWindowBackendProvenance,
    requested: &Backend,
) -> Result<(), String> {
    if !backend.strict_backend || backend.fallback_used {
        return Err(
            "VCET final-test backend provenance must be strict and must not report fallback"
                .to_string(),
        );
    }
    match requested {
        Backend::CpuOnly => {
            if backend.requested_backend != "CpuOnly"
                || backend.realized_backend != "Cpu"
                || backend.backend_route != "CpuOnly"
                || backend.device_id.is_some()
                || backend.metric_contract != "full_cpu_metrics_v1"
                || !backend.unavailable_metric_fields.is_empty()
            {
                return Err(
                    "VCET final-test CPU provenance does not match the strict CpuOnly contract"
                        .to_string(),
                );
            }
        }
        Backend::GpuKernel { device_id } => {
            if backend.requested_backend != "GpuKernel"
                || backend.realized_backend != "Gpu"
                || backend.backend_route != "GpuKernel"
                || backend.device_id != Some(*device_id)
                || backend.metric_contract != "vcet_gpu_19_field_metrics_v1"
            {
                return Err(
                    "VCET final-test GPU provenance does not match the strict GpuKernel contract"
                        .to_string(),
                );
            }
            let expected_unavailable = [
                "closed_trades",
                "winning_trades",
                "losing_trades",
                "gross_profit",
                "gross_loss",
            ];
            if backend
                .unavailable_metric_fields
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != expected_unavailable
            {
                return Err(
                    "VCET final-test GPU provenance unavailable-metric contract is not canonical"
                        .to_string(),
                );
            }
        }
        _ => {
            return Err(
                "VCET final-test execution requires CpuOnly or strict GpuKernel".to_string(),
            )
        }
    }
    Ok(())
}

fn validate_finite_metrics(metrics: &Metrics, dataset_id: &str) -> Result<(), String> {
    for (field, value) in [
        ("pnl", metrics.pnl),
        ("sharpe", metrics.sharpe),
        ("sortino", metrics.sortino),
        ("downside_deviation", metrics.downside_deviation),
        ("downside_frequency", metrics.downside_frequency),
        ("downside_severity", metrics.downside_severity),
        ("calmar", metrics.calmar),
        ("omega", metrics.omega),
        ("kappa_tau", metrics.kappa_tau),
        ("volatility", metrics.volatility),
        ("ulcer_index", metrics.ulcer_index),
        ("martin_ratio", metrics.martin_ratio),
        ("max_dd", metrics.max_dd),
        ("exposure", metrics.exposure),
        ("net_exposure", metrics.net_exposure),
        ("long_mean_return", metrics.long_mean_return),
        ("short_mean_return", metrics.short_mean_return),
        ("gross_profit", metrics.gross_profit),
        ("gross_loss", metrics.gross_loss),
        ("underwater_fraction", metrics.underwater_fraction),
    ] {
        if !value.is_finite() {
            return Err(format!(
                "VCET final-test dataset '{dataset_id}' metric '{field}' is non-finite"
            ));
        }
    }
    Ok(())
}

fn find_final_authorization(
    reduction: &ReduceVcetTemporalPreFinalShardsResponseV1,
    descriptor: &VcetTemporalUniverseShardV1,
) -> Result<VcetTemporalFinalShardAuthorizationV1, String> {
    let matches = reduction
        .payload
        .final_shard_authorizations
        .iter()
        .filter(|authorization| authorization.shard.shard_id == descriptor.shard_id)
        .collect::<Vec<_>>();
    if matches.len() != 1 || !serialized_eq(&matches[0].shard, descriptor)? {
        return Err(format!(
            "VCET final-test shard '{}' does not have exactly one matching sealed authorization",
            descriptor.shard_id
        ));
    }
    Ok(matches[0].clone())
}

fn prevalidate_before_final_data_access(
    req: &ExecuteVcetTemporalFinalTestShardRequestV1,
) -> Result<(VcetTemporalFinalShardAuthorizationV1, String), String> {
    validate_sealed_vcet_temporal_universe(&req.sealed_universe, &req.expected_universe_sha256)?;
    validate_vcet_temporal_pre_final_reduction_artifact(
        &req.global_winner_reduction,
        &req.expected_global_winner_reduction_artifact_sha256,
    )?;
    if req.sealed_universe.universe_sha256 != req.global_winner_reduction.payload.universe_sha256 {
        return Err(
            "VCET final-test global winner reduction belongs to a different universe".to_string(),
        );
    }
    let descriptor = validate_exact_final_shard_descriptor(&req.sealed_universe, &req.final_shard)?;
    let authorization = find_final_authorization(&req.global_winner_reduction, &descriptor)?;
    validate_vcet_temporal_final_shard_authorization(
        &authorization,
        &req.sealed_universe,
        &req.global_winner_reduction.payload.winner_lock,
    )?;
    if authorization.expected_prior_state_sha256 != req.expected_authorized_prior_state_sha256 {
        return Err(format!(
            "VCET final-test prior-state SHA mismatch for shard '{}'",
            descriptor.shard_id
        ));
    }
    if !serialized_eq(
        &req.planning_request.sealed_candidate_manifest,
        &req.sealed_universe.payload.candidate_manifest,
    )? || req.planning_request.expected_candidate_manifest_sha256
        != req.sealed_universe.payload.candidate_manifest_sha256
        || !serialized_eq(
            &req.planning_request.plan,
            &req.sealed_universe.payload.plan,
        )?
        || req.planning_request.expected_plan_identity != req.sealed_universe.payload.plan_identity
    {
        return Err(
            "VCET final-test planning request does not exactly match the sealed universe manifest and plan"
                .to_string(),
        );
    }
    if req.planning_request.datasets.len() != descriptor.dataset_ids.len() {
        return Err(format!(
            "VCET final-test planning request has {} datasets for a {}-dataset descriptor",
            req.planning_request.datasets.len(),
            descriptor.dataset_ids.len()
        ));
    }
    let universe_entries = universe_entry_map(&req.sealed_universe.payload)?;
    let expected_ids = descriptor
        .dataset_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if expected_ids.len() != descriptor.dataset_ids.len() {
        return Err("VCET final-test descriptor contains duplicate dataset IDs".to_string());
    }
    let mut observed_timeline_ids = BTreeSet::new();
    for timeline in &req.planning_request.datasets {
        let universe_dataset = universe_entries.get(&timeline.dataset_id).ok_or_else(|| {
            format!(
                "VCET final-test dataset '{}' is absent from the sealed universe",
                timeline.dataset_id
            )
        })?;
        if timeline.dataset_id.trim() != timeline.dataset_id
            || !expected_ids.contains(&timeline.dataset_id)
            || !observed_timeline_ids.insert(timeline.dataset_id.clone())
            || universe_dataset.role != VcetTemporalDatasetCohortRoleV1::FinalTest
            || universe_dataset.source_dataset_sha256 != timeline.source_dataset_sha256
        {
            return Err(
                "VCET final-test planning timelines must exactly match the canonical final descriptor and source lineage"
                    .to_string(),
            );
        }
    }
    let canonical_planning = plan_vcet_temporal_validation_blocking(req.planning_request.clone())?;
    if canonical_planning != req.planning_evidence
        || req.expected_planning_evidence_sha256 != canonical_planning.evidence_sha256
    {
        return Err(
            "VCET final-test planning evidence is not the exact canonical result for the supplied descriptor timelines"
                .to_string(),
        );
    }
    for planned in &canonical_planning.datasets {
        let universe_dataset = universe_entries.get(&planned.dataset_id).ok_or_else(|| {
            format!(
                "VCET final-test planning evidence contains unknown dataset '{}'",
                planned.dataset_id
            )
        })?;
        let interval_sha256 = recompute_vcet_temporal_interval_fingerprint_sha256(&planned.layout)?;
        if planned.source_dataset_sha256 != universe_dataset.source_dataset_sha256
            || planned.timeline_sha256 != universe_dataset.timeline_sha256
            || planned.fold_count != universe_dataset.fold_count
            || interval_sha256 != universe_dataset.temporal_interval_fingerprint_sha256
            || interval_sha256
                != req
                    .sealed_universe
                    .payload
                    .temporal_interval_fingerprint_sha256
        {
            return Err(format!(
                "VCET final-test planning source, timeline, fold, or interval evidence differs for dataset '{}'",
                planned.dataset_id
            ));
        }
    }
    let winner = &req.global_winner_reduction.payload.winner_lock;
    let manifest_candidate = req
        .sealed_universe
        .payload
        .candidate_manifest
        .payload
        .candidates
        .get(winner.global_candidate_ordinal)
        .ok_or_else(|| {
            "VCET final-test winner ordinal is outside the sealed candidate manifest".to_string()
        })?;
    let candidate_params = candidate_params_from_manifest(
        &manifest_candidate.identity_inputs,
        &req.sealed_universe.payload.execution_spec.strategy,
    );
    if manifest_candidate.ordinal != winner.global_candidate_ordinal
        || manifest_candidate.candidate_id != winner.candidate_id
        || !serialized_eq(&candidate_params, &winner.params)?
    {
        return Err(
            "VCET final-test winner identity or parameters differ from the sealed universe"
                .to_string(),
        );
    }
    if req.loaded_datasets.len() != descriptor.dataset_ids.len() {
        return Err(
            "VCET final-test loaded bindings must cover exactly the descriptor datasets"
                .to_string(),
        );
    }
    let mut binding_ids = BTreeSet::new();
    let mut loaded_ids = BTreeSet::new();
    for binding in &req.loaded_datasets {
        if binding.dataset_id.trim() != binding.dataset_id
            || binding.loaded_data_id.trim() != binding.loaded_data_id
            || binding.loaded_data_id.is_empty()
            || !expected_ids.contains(&binding.dataset_id)
            || !binding_ids.insert(binding.dataset_id.clone())
            || !loaded_ids.insert(binding.loaded_data_id.clone())
        {
            return Err(
                "VCET final-test loaded bindings must be unique, canonical, and exact for the descriptor"
                    .to_string(),
            );
        }
    }
    let request_sha256 = hash_enveloped(
        FINAL_SHARD_REQUEST_HASH_CONTRACT,
        req,
        "VCET final-test shard request",
    )?;
    Ok((authorization, request_sha256))
}

fn prevalidate_final_shard_request(
    req: &ExecuteVcetTemporalFinalTestShardRequestV1,
    loaded: Vec<LoadedFinalDataset>,
) -> Result<PreparedFinalShard, String> {
    validate_sealed_vcet_temporal_universe(&req.sealed_universe, &req.expected_universe_sha256)?;
    validate_vcet_temporal_pre_final_reduction_artifact(
        &req.global_winner_reduction,
        &req.expected_global_winner_reduction_artifact_sha256,
    )?;
    if req.sealed_universe.universe_sha256 != req.global_winner_reduction.payload.universe_sha256 {
        return Err(
            "VCET final-test global winner reduction belongs to a different universe".to_string(),
        );
    }
    let descriptor = validate_exact_final_shard_descriptor(&req.sealed_universe, &req.final_shard)?;
    let authorization = find_final_authorization(&req.global_winner_reduction, &descriptor)?;
    validate_vcet_temporal_final_shard_authorization(
        &authorization,
        &req.sealed_universe,
        &req.global_winner_reduction.payload.winner_lock,
    )?;
    if authorization.expected_prior_state_sha256 != req.expected_authorized_prior_state_sha256 {
        return Err(format!(
            "VCET final-test prior-state SHA mismatch for shard '{}'",
            descriptor.shard_id
        ));
    }

    if !serialized_eq(
        &req.planning_request.sealed_candidate_manifest,
        &req.sealed_universe.payload.candidate_manifest,
    )? || req.planning_request.expected_candidate_manifest_sha256
        != req.sealed_universe.payload.candidate_manifest_sha256
        || !serialized_eq(
            &req.planning_request.plan,
            &req.sealed_universe.payload.plan,
        )?
        || req.planning_request.expected_plan_identity != req.sealed_universe.payload.plan_identity
    {
        return Err(
            "VCET final-test planning request does not exactly match the sealed universe manifest and plan"
                .to_string(),
        );
    }
    if req.planning_request.datasets.len() != descriptor.dataset_ids.len() {
        return Err(format!(
            "VCET final-test planning request has {} datasets for a {}-dataset descriptor",
            req.planning_request.datasets.len(),
            descriptor.dataset_ids.len()
        ));
    }
    let universe_entries = universe_entry_map(&req.sealed_universe.payload)?;
    let expected_ids = descriptor
        .dataset_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if expected_ids.len() != descriptor.dataset_ids.len() {
        return Err("VCET final-test descriptor contains duplicate dataset IDs".to_string());
    }
    let mut timeline_by_id = BTreeMap::new();
    for timeline in &req.planning_request.datasets {
        if timeline.dataset_id.trim() != timeline.dataset_id
            || !expected_ids.contains(&timeline.dataset_id)
            || timeline_by_id
                .insert(timeline.dataset_id.clone(), timeline)
                .is_some()
        {
            return Err(
                "VCET final-test planning request must contain each descriptor dataset exactly once with canonical IDs"
                    .to_string(),
            );
        }
        let universe_dataset = universe_entries.get(&timeline.dataset_id).ok_or_else(|| {
            format!(
                "VCET final-test dataset '{}' is absent from the sealed universe",
                timeline.dataset_id
            )
        })?;
        if universe_dataset.role != VcetTemporalDatasetCohortRoleV1::FinalTest
            || universe_dataset.source_dataset_sha256 != timeline.source_dataset_sha256
        {
            return Err(format!(
                "VCET final-test dataset '{}' role or source lineage differs from the sealed universe",
                timeline.dataset_id
            ));
        }
    }

    let canonical_planning = plan_vcet_temporal_validation_blocking(req.planning_request.clone())?;
    if canonical_planning != req.planning_evidence
        || req.expected_planning_evidence_sha256 != canonical_planning.evidence_sha256
    {
        return Err(
            "VCET final-test planning evidence is not the exact canonical result for the supplied descriptor timelines"
                .to_string(),
        );
    }
    if canonical_planning.datasets.len() != descriptor.dataset_ids.len() {
        return Err("VCET final-test canonical planning dataset count mismatch".to_string());
    }

    let winner = &req.global_winner_reduction.payload.winner_lock;
    let manifest_candidate = req
        .sealed_universe
        .payload
        .candidate_manifest
        .payload
        .candidates
        .get(winner.global_candidate_ordinal)
        .ok_or_else(|| {
            "VCET final-test winner ordinal is outside the sealed candidate manifest".to_string()
        })?;
    if manifest_candidate.ordinal != winner.global_candidate_ordinal
        || manifest_candidate.candidate_id != winner.candidate_id
    {
        return Err(
            "VCET final-test winner identity does not match the sealed candidate manifest"
                .to_string(),
        );
    }
    let candidate_params = candidate_params_from_manifest(
        &manifest_candidate.identity_inputs,
        &req.sealed_universe.payload.execution_spec.strategy,
    );
    if !serialized_eq(&candidate_params, &winner.params)? {
        return Err(
            "VCET final-test winner parameters are not the canonical parameters derived from the sealed universe"
                .to_string(),
        );
    }

    if req.loaded_datasets.len() != descriptor.dataset_ids.len()
        || loaded.len() != descriptor.dataset_ids.len()
    {
        return Err(
            "VCET final-test loaded bindings must cover exactly the descriptor datasets"
                .to_string(),
        );
    }
    let mut binding_by_dataset = BTreeMap::new();
    let mut loaded_id_set = BTreeSet::new();
    for binding in &req.loaded_datasets {
        if binding.dataset_id.trim() != binding.dataset_id
            || binding.loaded_data_id.trim() != binding.loaded_data_id
            || binding.loaded_data_id.is_empty()
            || !expected_ids.contains(&binding.dataset_id)
            || binding_by_dataset
                .insert(binding.dataset_id.clone(), binding.clone())
                .is_some()
            || !loaded_id_set.insert(binding.loaded_data_id.clone())
        {
            return Err(
                "VCET final-test loaded bindings must be unique, canonical, and exact for the descriptor"
                    .to_string(),
            );
        }
    }
    let mut loaded_by_dataset = BTreeMap::new();
    for item in loaded {
        if loaded_by_dataset
            .insert(item.binding.dataset_id.clone(), item)
            .is_some()
        {
            return Err("duplicate resolved VCET final-test dataset".to_string());
        }
    }

    let mut planned_by_id = BTreeMap::new();
    for planned in &canonical_planning.datasets {
        planned_by_id.insert(planned.dataset_id.clone(), planned);
    }
    let mut prepared_datasets = Vec::with_capacity(descriptor.dataset_ids.len());
    for dataset_id in &descriptor.dataset_ids {
        let binding = binding_by_dataset
            .get(dataset_id)
            .ok_or_else(|| format!("missing final-test binding for dataset '{dataset_id}'"))?;
        let resolved = loaded_by_dataset
            .remove(dataset_id)
            .ok_or_else(|| format!("missing resolved final-test candles for '{dataset_id}'"))?;
        if resolved.binding != *binding {
            return Err(format!(
                "resolved final-test binding differs for dataset '{dataset_id}'"
            ));
        }
        let universe_dataset = (**universe_entries
            .get(dataset_id)
            .ok_or_else(|| format!("missing universe entry for '{dataset_id}'"))?)
        .clone();
        let timeline = timeline_by_id
            .get(dataset_id)
            .ok_or_else(|| format!("missing final-test timeline for '{dataset_id}'"))?;
        let planned = planned_by_id
            .get(dataset_id)
            .ok_or_else(|| format!("missing final-test plan evidence for '{dataset_id}'"))?;
        if timeline.timestamps_utc_ms != resolved.candles.timestamp {
            return Err(format!(
                "VCET final-test actual timestamps differ from the sealed timeline for dataset '{dataset_id}'"
            ));
        }
        validate_loaded_dataset_shape(dataset_id, &resolved.candles)?;
        let snapshot_sha256 = loaded_candles_snapshot_sha256(&resolved.candles);
        if snapshot_sha256 != universe_dataset.loaded_candles_snapshot_sha256
            || planned.source_dataset_sha256 != universe_dataset.source_dataset_sha256
            || planned.timeline_sha256 != universe_dataset.timeline_sha256
            || planned.fold_count != universe_dataset.fold_count
        {
            return Err(format!(
                "VCET final-test source, timeline, fold, or actual snapshot evidence differs for dataset '{dataset_id}'"
            ));
        }
        let interval_sha256 = recompute_vcet_temporal_interval_fingerprint_sha256(&planned.layout)?;
        if interval_sha256 != universe_dataset.temporal_interval_fingerprint_sha256
            || interval_sha256
                != req
                    .sealed_universe
                    .payload
                    .temporal_interval_fingerprint_sha256
        {
            return Err(format!(
                "VCET final-test temporal interval differs for dataset '{dataset_id}'"
            ));
        }
        let coverage = profile_loaded_dataset_coverage_for_planning(
            &req.planning_request,
            dataset_id,
            &resolved.candles,
        )?;
        let coverage_sha256 = recompute_vcet_dataset_coverage_sha256(
            dataset_id,
            &snapshot_sha256,
            &planned.timeline_sha256,
            &coverage,
        )?;
        if coverage != universe_dataset.dataset_coverage
            || coverage_sha256 != universe_dataset.dataset_coverage_sha256
        {
            return Err(format!(
                "VCET final-test actual dataset coverage differs from the sealed universe for '{dataset_id}'"
            ));
        }
        prepared_datasets.push(PreparedFinalDataset {
            binding: binding.clone(),
            universe: universe_dataset,
            final_test: planned.layout.final_test.clone(),
            candles: resolved.candles,
        });
    }
    if !loaded_by_dataset.is_empty() {
        return Err("unrequested resolved final-test candles were supplied".to_string());
    }
    let request_sha256 = hash_enveloped(
        FINAL_SHARD_REQUEST_HASH_CONTRACT,
        req,
        "VCET final-test shard request",
    )?;
    Ok(PreparedFinalShard {
        request_sha256,
        authorization,
        planning_evidence_sha256: canonical_planning.evidence_sha256,
        candidate_params,
        candidate_id: winner.candidate_id.clone(),
        candidate_ordinal: winner.global_candidate_ordinal,
        datasets: prepared_datasets,
    })
}

fn recompute_completed_state_sha256(
    authorization_sha256: &str,
    request_sha256: &str,
    authorized_prior_state_sha256: &str,
    universe_sha256: &str,
    winner_lock_sha256: &str,
    final_shard: &VcetTemporalUniverseShardV1,
    datasets: &[VcetTemporalFinalTestDatasetExecutionV1],
    expected_candidate_window_trial_count: usize,
    evaluated_candidate_window_trial_count: usize,
) -> Result<String, String> {
    hash_serializable(
        &CompletedStateHashInput {
            hash_contract: FINAL_SHARD_COMPLETED_STATE_HASH_CONTRACT,
            authorization_sha256,
            request_sha256,
            authorized_prior_state_sha256,
            universe_sha256,
            winner_lock_sha256,
            final_shard,
            datasets,
            expected_candidate_window_trial_count,
            evaluated_candidate_window_trial_count,
        },
        "VCET final-test completed state",
    )
}

pub(super) fn recompute_vcet_temporal_final_shard_artifact_sha256(
    payload: &VcetTemporalFinalTestShardArtifactPayloadV1,
) -> Result<String, String> {
    hash_enveloped(
        FINAL_SHARD_ARTIFACT_HASH_CONTRACT,
        payload,
        "VCET final-test shard artifact",
    )
}

pub(super) fn validate_vcet_temporal_final_shard_artifact(
    response: &ExecuteVcetTemporalFinalTestShardResponseV1,
) -> Result<(), String> {
    let payload = &response.payload;
    if payload.schema_version != VCET_TEMPORAL_FINAL_SHARD_ARTIFACT_SCHEMA_VERSION
        || payload.evidence_kind != "vcet_temporal_final_test_shard_artifact"
    {
        return Err("unsupported or non-canonical VCET final-test shard artifact".to_string());
    }
    if payload.expected_candidate_window_trial_count
        != payload.final_shard.expected_candidate_window_trial_count
        || payload.expected_candidate_window_trial_count == 0
        || payload.evaluated_candidate_window_trial_count
            != payload.expected_candidate_window_trial_count
        || payload.datasets.len() != payload.expected_candidate_window_trial_count
        || payload.physical_shard_execution_count != 1
        || payload.candidate_count != 1
        || payload.selection_used_final_test
        || payload.winner_reranked_after_final_test
        || payload.execution_spec_sha256 != payload.winner_lock.execution_spec_sha256
        || payload.universe_sha256 != payload.winner_lock.universe_sha256
        || payload.global_pre_final_evidence_sha256
            != payload.winner_lock.global_pre_final_evidence_sha256
        || payload.final_stage_nonce.trim().is_empty()
        || !is_canonical_sha256(&payload.request_sha256)
        || !is_canonical_sha256(&payload.authorization_sha256)
        || !is_canonical_sha256(&payload.authorized_prior_state_sha256)
        || !is_canonical_sha256(&payload.completed_state_sha256)
    {
        return Err(
            "VCET final-test shard artifact counts or no-rerank flags are invalid".to_string(),
        );
    }
    if payload.final_shard.role != VcetTemporalDatasetCohortRoleV1::FinalTest
        || !payload.final_shard.locked_winner_only
        || payload.final_shard.candidate_start_index != 0
        || payload.final_shard.candidate_end_index_exclusive != 0
        || !is_canonical_final_shard_id(&payload.final_shard.shard_id)
        || payload.scoring_semantics
            != final_scoring_semantics(VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen)
    {
        return Err(
            "VCET final-test shard artifact descriptor or logical scoring semantics are invalid"
                .to_string(),
        );
    }
    let dataset_ids = payload
        .datasets
        .iter()
        .map(|dataset| dataset.dataset_id.as_str())
        .collect::<Vec<_>>();
    let descriptor_ids = payload
        .final_shard
        .dataset_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if dataset_ids != descriptor_ids
        || descriptor_ids.is_empty()
        || descriptor_ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(
            "VCET final-test shard artifact dataset order differs from its descriptor".to_string(),
        );
    }
    let mut loaded_ids = BTreeSet::new();
    let mut common_final_test: Option<&VcetTemporalEvaluationSliceV1> = None;
    let mut common_minimum_trades: Option<u64> = None;
    for dataset in &payload.datasets {
        validate_finite_metrics(&dataset.metrics, &dataset.dataset_id)?;
        if dataset.global_candidate_ordinal != payload.winner_lock.global_candidate_ordinal
            || dataset.candidate_id != payload.winner_lock.candidate_id
            || !serialized_eq(&dataset.params, &payload.winner_lock.params)?
            || dataset.temporal_interval_fingerprint_sha256
                != payload.temporal_interval_fingerprint_sha256
            || dataset.observed_trades != u64::from(dataset.metrics.trades)
            || dataset.minimum_trade_support_passed
                != (dataset.observed_trades >= dataset.minimum_trades)
            || dataset.final_test.scored_bars.is_empty()
            || dataset.dataset_id.trim() != dataset.dataset_id
            || dataset.dataset_id.is_empty()
            || dataset.loaded_data_id.trim() != dataset.loaded_data_id
            || dataset.loaded_data_id.is_empty()
            || !loaded_ids.insert(dataset.loaded_data_id.clone())
        {
            return Err(format!(
                "VCET final-test shard dataset '{}' has inconsistent winner, interval, support, or binding evidence",
                dataset.dataset_id
            ));
        }
        if let Some(expected_final_test) = common_final_test {
            if dataset.final_test != *expected_final_test {
                return Err(
                    "VCET final-test shard datasets do not share one exact final scored interval"
                        .to_string(),
                );
            }
        } else {
            common_final_test = Some(&dataset.final_test);
        }
        if let Some(expected_minimum) = common_minimum_trades {
            if dataset.minimum_trades != expected_minimum {
                return Err(
                    "VCET final-test shard datasets do not share one minimum-trade contract"
                        .to_string(),
                );
            }
        } else {
            common_minimum_trades = Some(dataset.minimum_trades);
        }
        let coverage_sha256 = recompute_vcet_dataset_coverage_sha256(
            &dataset.dataset_id,
            &dataset.loaded_candles_snapshot_sha256,
            &dataset.timeline_sha256,
            &dataset.dataset_coverage,
        )?;
        if coverage_sha256 != dataset.dataset_coverage_sha256 {
            return Err(format!(
                "VCET final-test shard dataset '{}' coverage SHA-256 mismatch",
                dataset.dataset_id
            ));
        }
        validate_backend_provenance(&dataset.backend, &payload.execution_spec.backend)?;
    }
    let completed = recompute_completed_state_sha256(
        &payload.authorization_sha256,
        &payload.request_sha256,
        &payload.authorized_prior_state_sha256,
        &payload.universe_sha256,
        &payload.winner_lock.winner_lock_sha256,
        &payload.final_shard,
        &payload.datasets,
        payload.expected_candidate_window_trial_count,
        payload.evaluated_candidate_window_trial_count,
    )?;
    if completed != payload.completed_state_sha256 {
        return Err("VCET final-test shard completed-state SHA-256 mismatch".to_string());
    }
    let support_passed = payload
        .datasets
        .iter()
        .all(|dataset| dataset.minimum_trade_support_passed);
    let expected_status = if support_passed {
        VcetTemporalFinalSupportStatusV1::SupportPassed
    } else {
        VcetTemporalFinalSupportStatusV1::SupportFailed
    };
    if payload.minimum_trade_support_passed != support_passed
        || payload.support_status != expected_status
        || payload.limitations != final_shard_limitations()
    {
        return Err("VCET final-test shard support evidence is inconsistent".to_string());
    }
    let actual = recompute_vcet_temporal_final_shard_artifact_sha256(payload)?;
    if response.artifact_sha256 != actual {
        return Err(format!(
            "VCET final-test shard artifact SHA-256 mismatch (declared {}, actual {actual})",
            response.artifact_sha256
        ));
    }
    Ok(())
}

fn execute_prevalidated_final_shard(
    req: &ExecuteVcetTemporalFinalTestShardRequestV1,
    prepared: &PreparedFinalShard,
) -> Result<ExecuteVcetTemporalFinalTestShardResponseV1, String> {
    let universe = &req.sealed_universe.payload;
    let winner = &req.global_winner_reduction.payload.winner_lock;
    let spec = &universe.execution_spec;
    let candidate_params = vec![prepared.candidate_params.clone()];
    let candidate_ids = vec![prepared.candidate_id.clone()];
    let minimum_trades = universe.plan.minimum_support.min_final_test_trades;
    let mut datasets = Vec::with_capacity(prepared.datasets.len());
    for prepared_dataset in &prepared.datasets {
        let window = evaluate_window(
            &prepared_dataset.universe.dataset_id,
            &prepared_dataset.candles,
            &spec.backend,
            &spec.ma_source,
            &candidate_params,
            &candidate_ids,
            prepared_dataset.final_test.scored_bars,
            universe.plan.position_boundary_policy,
        )?;
        if window.scored_bars != prepared_dataset.final_test.scored_bars
            || window.requested_candidate_count != 1
            || window.evaluated_candidate_count != 1
            || window.candidates.len() != 1
            || window.candidates[0].candidate_index != 0
            || window.candidates[0].candidate_id != prepared.candidate_id
        {
            return Err(format!(
                "VCET final-test evaluator violated locked-winner cardinality for dataset '{}'",
                prepared_dataset.universe.dataset_id
            ));
        }
        validate_backend_provenance(&window.backend, &spec.backend)?;
        let metrics = window.candidates[0].metrics.clone();
        validate_finite_metrics(&metrics, &prepared_dataset.universe.dataset_id)?;
        let observed_trades = u64::from(metrics.trades);
        let minimum_trade_support_passed = observed_trades >= minimum_trades;
        datasets.push(VcetTemporalFinalTestDatasetExecutionV1 {
            dataset_id: prepared_dataset.universe.dataset_id.clone(),
            loaded_data_id: prepared_dataset.binding.loaded_data_id.clone(),
            source_dataset_sha256: prepared_dataset.universe.source_dataset_sha256.clone(),
            dataset_attestation_sha256: prepared_dataset
                .universe
                .dataset_attestation_sha256
                .clone(),
            timeline_sha256: prepared_dataset.universe.timeline_sha256.clone(),
            loaded_candles_snapshot_sha256: prepared_dataset
                .universe
                .loaded_candles_snapshot_sha256
                .clone(),
            dataset_coverage: prepared_dataset.universe.dataset_coverage.clone(),
            dataset_coverage_sha256: prepared_dataset.universe.dataset_coverage_sha256.clone(),
            temporal_interval_fingerprint_sha256: prepared_dataset
                .universe
                .temporal_interval_fingerprint_sha256
                .clone(),
            final_test: prepared_dataset.final_test.clone(),
            global_candidate_ordinal: prepared.candidate_ordinal,
            candidate_id: prepared.candidate_id.clone(),
            params: prepared.candidate_params.clone(),
            metrics,
            backend: window.backend,
            observed_trades,
            minimum_trades,
            minimum_trade_support_passed,
        });
    }
    if datasets.len() != req.final_shard.expected_candidate_window_trial_count {
        return Err(format!(
            "VCET final-test evaluated {} candidate windows; descriptor requires {}",
            datasets.len(),
            req.final_shard.expected_candidate_window_trial_count
        ));
    }
    let completed_state_sha256 = recompute_completed_state_sha256(
        &prepared.authorization.authorization_sha256,
        &prepared.request_sha256,
        &prepared.authorization.expected_prior_state_sha256,
        &req.sealed_universe.universe_sha256,
        &winner.winner_lock_sha256,
        &req.final_shard,
        &datasets,
        req.final_shard.expected_candidate_window_trial_count,
        datasets.len(),
    )?;
    let minimum_trade_support_passed = datasets
        .iter()
        .all(|dataset| dataset.minimum_trade_support_passed);
    let support_status = if minimum_trade_support_passed {
        VcetTemporalFinalSupportStatusV1::SupportPassed
    } else {
        VcetTemporalFinalSupportStatusV1::SupportFailed
    };
    let payload = VcetTemporalFinalTestShardArtifactPayloadV1 {
        schema_version: VCET_TEMPORAL_FINAL_SHARD_ARTIFACT_SCHEMA_VERSION,
        evidence_kind: "vcet_temporal_final_test_shard_artifact".to_string(),
        universe_sha256: req.sealed_universe.universe_sha256.clone(),
        global_winner_reduction_artifact_sha256: req
            .global_winner_reduction
            .artifact_sha256
            .clone(),
        global_pre_final_evidence_sha256: req
            .global_winner_reduction
            .payload
            .global_pre_final_evidence_sha256
            .clone(),
        winner_lock: winner.clone(),
        authorization_sha256: prepared.authorization.authorization_sha256.clone(),
        final_stage_nonce: prepared.authorization.final_stage_nonce.clone(),
        final_shard: req.final_shard.clone(),
        planning_evidence_sha256: prepared.planning_evidence_sha256.clone(),
        execution_spec_sha256: winner.execution_spec_sha256.clone(),
        execution_spec: universe.execution_spec.clone(),
        temporal_interval_fingerprint_sha256: universe.temporal_interval_fingerprint_sha256.clone(),
        scoring_semantics: final_scoring_semantics(universe.plan.position_boundary_policy),
        request_sha256: prepared.request_sha256.clone(),
        authorized_prior_state_sha256: prepared.authorization.expected_prior_state_sha256.clone(),
        completed_state_sha256,
        expected_candidate_window_trial_count: req
            .final_shard
            .expected_candidate_window_trial_count,
        evaluated_candidate_window_trial_count: datasets.len(),
        physical_shard_execution_count: 1,
        candidate_count: 1,
        datasets,
        support_status,
        minimum_trade_support_passed,
        selection_used_final_test: false,
        winner_reranked_after_final_test: false,
        limitations: final_shard_limitations(),
    };
    let artifact_sha256 = recompute_vcet_temporal_final_shard_artifact_sha256(&payload)?;
    let response = ExecuteVcetTemporalFinalTestShardResponseV1 {
        payload,
        artifact_sha256,
    };
    validate_vcet_temporal_final_shard_artifact(&response)?;
    Ok(response)
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

#[allow(dead_code)]
pub(super) fn execute_vcet_temporal_final_test_shard_blocking_with_loaded_candles(
    req: ExecuteVcetTemporalFinalTestShardRequestV1,
    loaded: Vec<(VcetTemporalLoadedDatasetBindingV1, Arc<Candles>)>,
) -> Result<ExecuteVcetTemporalFinalTestShardResponseV1, String> {
    if let Some(response) = lookup_identical_final_retry(&req)? {
        return Ok(response);
    }
    let (authorization, request_sha256) = prevalidate_before_final_data_access(&req)?;
    let authorization_sha256 = authorization.authorization_sha256.clone();
    match consume_final_authorization(&authorization_sha256, &request_sha256)? {
        LedgerAdmission::Cached(response) => return Ok(response),
        LedgerAdmission::Execute => {}
    }
    execute_consumed_final_shard(req, loaded, authorization_sha256, request_sha256)
}

fn execute_consumed_final_shard(
    req: ExecuteVcetTemporalFinalTestShardRequestV1,
    loaded: Vec<(VcetTemporalLoadedDatasetBindingV1, Arc<Candles>)>,
    authorization_sha256: String,
    request_sha256: String,
) -> Result<ExecuteVcetTemporalFinalTestShardResponseV1, String> {
    let loaded = loaded
        .into_iter()
        .map(|(binding, candles)| LoadedFinalDataset { binding, candles })
        .collect::<Vec<_>>();
    let execution = catch_unwind(AssertUnwindSafe(|| {
        let prepared = prevalidate_final_shard_request(&req, loaded)?;
        if prepared.authorization.authorization_sha256 != authorization_sha256
            || prepared.request_sha256 != request_sha256
        {
            return Err(
                "VCET final-test request identity changed across the authorization boundary"
                    .to_string(),
            );
        }
        execute_prevalidated_final_shard(&req, &prepared)
    }));
    match execution {
        Ok(Ok(response)) => {
            complete_final_authorization(&authorization_sha256, &request_sha256, response.clone())?;
            Ok(response)
        }
        Ok(Err(error)) => {
            fail_final_authorization(&authorization_sha256, &request_sha256, &error)?;
            Err(error)
        }
        Err(payload) => {
            let error = format!(
                "VCET final-test shard execution panicked after consuming authorization: {}",
                panic_message(payload)
            );
            fail_final_authorization(&authorization_sha256, &request_sha256, &error)?;
            Err(error)
        }
    }
}

fn record_durable_final_failure(claim: &DurableFinalClaim, error: String) -> String {
    match claim.fail(&error) {
        Ok(()) => error,
        Err(ledger_error) => format!(
            "{error}; additionally failed to publish the durable VCET final-test failure state: {ledger_error}"
        ),
    }
}

fn append_process_failure_record(
    authorization_sha256: &str,
    request_sha256: &str,
    error: String,
) -> String {
    fail_final_authorization(authorization_sha256, request_sha256, &error)
        .err()
        .map(|ledger_error| {
            format!(
                "{error}; additionally failed to publish the process-local VCET final-test failure state: {ledger_error}"
            )
        })
        .unwrap_or(error)
}

fn validate_final_receipt_against_execute_authority(
    req: &ExecuteVcetTemporalFinalTestShardRequestV1,
    authorization: &VcetTemporalFinalShardAuthorizationV1,
    request_sha256: &str,
    response: &ExecuteVcetTemporalFinalTestShardResponseV1,
) -> Result<(), String> {
    validate_vcet_temporal_final_shard_artifact(response)?;
    let payload = &response.payload;
    let reduction = &req.global_winner_reduction.payload;
    if payload.request_sha256 != request_sha256
        || payload.authorization_sha256 != authorization.authorization_sha256
        || payload.authorized_prior_state_sha256 != authorization.expected_prior_state_sha256
        || !serialized_eq(&payload.final_shard, &req.final_shard)?
        || payload.universe_sha256 != req.sealed_universe.universe_sha256
        || payload.global_winner_reduction_artifact_sha256
            != req.expected_global_winner_reduction_artifact_sha256
        || payload.global_winner_reduction_artifact_sha256
            != req.global_winner_reduction.artifact_sha256
        || payload.global_pre_final_evidence_sha256 != reduction.global_pre_final_evidence_sha256
        || !serialized_eq(&payload.winner_lock, &reduction.winner_lock)?
        || payload.final_stage_nonce != authorization.final_stage_nonce
        || payload.final_stage_nonce != reduction.final_stage_nonce
        || payload.planning_evidence_sha256 != req.expected_planning_evidence_sha256
        || payload.execution_spec_sha256 != reduction.winner_lock.execution_spec_sha256
        || !serialized_eq(
            &payload.execution_spec,
            &req.sealed_universe.payload.execution_spec,
        )?
        || payload.temporal_interval_fingerprint_sha256
            != req
                .sealed_universe
                .payload
                .temporal_interval_fingerprint_sha256
    {
        return Err(format!(
            "VCET final-test durable receipt for shard '{}' differs from its exact request, winner, authorization, or universe authority",
            req.final_shard.shard_id
        ));
    }
    Ok(())
}

fn execute_acquired_durable_final_claim_with_loader<F>(
    req: ExecuteVcetTemporalFinalTestShardRequestV1,
    claim: DurableFinalClaim,
    authorization: VcetTemporalFinalShardAuthorizationV1,
    request_sha256: String,
    load_candles: F,
) -> Result<ExecuteVcetTemporalFinalTestShardResponseV1, String>
where
    F: FnOnce(
        &ExecuteVcetTemporalFinalTestShardRequestV1,
    ) -> Result<Vec<(VcetTemporalLoadedDatasetBindingV1, Arc<Candles>)>, String>,
{
    let authorization_sha256 = authorization.authorization_sha256.clone();
    let process_execution_admitted = std::cell::Cell::new(false);
    let stage = catch_unwind(AssertUnwindSafe(|| {
        match consume_final_authorization(&authorization_sha256, &request_sha256)? {
            LedgerAdmission::Cached(response) => {
                validate_final_receipt_against_execute_authority(
                    &req,
                    &authorization,
                    &request_sha256,
                    &response,
                )?;
                return Ok(response);
            }
            LedgerAdmission::Execute => process_execution_admitted.set(true),
        }

        let loaded = load_candles(&req).map_err(|error| {
            append_process_failure_record(&authorization_sha256, &request_sha256, error)
        })?;
        execute_consumed_final_shard(
            req,
            loaded,
            authorization_sha256.clone(),
            request_sha256.clone(),
        )
    }));

    match stage {
        Ok(Ok(response)) => {
            if let Err(error) = claim.complete(&response) {
                return Err(record_durable_final_failure(&claim, error));
            }
            Ok(response)
        }
        Ok(Err(error)) => Err(record_durable_final_failure(&claim, error)),
        Err(payload) => {
            let error = format!(
                "VCET final-test owned durable task panicked after claim admission: {}",
                panic_message(payload)
            );
            let process_error = if process_execution_admitted.get() {
                append_process_failure_record(&authorization_sha256, &request_sha256, error)
            } else {
                error
            };
            Err(record_durable_final_failure(&claim, process_error))
        }
    }
}

fn execute_vcet_temporal_final_test_shard_durable_blocking_with_loader<F>(
    req: ExecuteVcetTemporalFinalTestShardRequestV1,
    app_local_data_dir: &Path,
    load_candles: F,
) -> Result<ExecuteVcetTemporalFinalTestShardResponseV1, String>
where
    F: FnOnce(
        &ExecuteVcetTemporalFinalTestShardRequestV1,
    ) -> Result<Vec<(VcetTemporalLoadedDatasetBindingV1, Arc<Candles>)>, String>,
{
    let (authorization, request_sha256) = prevalidate_before_final_data_access(&req)?;
    let authorization_sha256 = authorization.authorization_sha256.clone();
    let durable_winner: ReduceVcetTemporalPreFinalShardsResponseV1 =
        load_durable_pre_final_winner_lock(app_local_data_dir, &req.expected_universe_sha256)?;
    validate_vcet_temporal_pre_final_reduction_artifact(
        &durable_winner,
        &req.expected_global_winner_reduction_artifact_sha256,
    )?;
    if !serialized_eq(&durable_winner, &req.global_winner_reduction)? {
        return Err(
            "VCET final-test request winner reduction differs from the durable pre-final winner-lock receipt"
                .to_string(),
        );
    }
    match admit_durable_final_authorization::<ExecuteVcetTemporalFinalTestShardResponseV1>(
        app_local_data_dir,
        &authorization_sha256,
        &request_sha256,
    )? {
        DurableFinalAdmission::Replay(response) => {
            validate_final_receipt_against_execute_authority(
                &req,
                &authorization,
                &request_sha256,
                &response,
            )?;
            hydrate_completed_final_authorization(
                &authorization_sha256,
                &request_sha256,
                response.clone(),
            )?;
            Ok(response)
        }
        DurableFinalAdmission::Acquired(claim) => execute_acquired_durable_final_claim_with_loader(
            req,
            claim,
            authorization,
            request_sha256,
            load_candles,
        ),
    }
}

#[allow(dead_code)]
pub(super) fn execute_vcet_temporal_final_test_shard_durable_blocking_with_loaded_candles(
    req: ExecuteVcetTemporalFinalTestShardRequestV1,
    loaded: Vec<(VcetTemporalLoadedDatasetBindingV1, Arc<Candles>)>,
    app_local_data_dir: &Path,
) -> Result<ExecuteVcetTemporalFinalTestShardResponseV1, String> {
    execute_vcet_temporal_final_test_shard_durable_blocking_with_loader(
        req,
        app_local_data_dir,
        move |_req| Ok(loaded),
    )
}

fn execute_vcet_temporal_final_test_shard_owned(
    req: ExecuteVcetTemporalFinalTestShardRequestV1,
    app: tauri::AppHandle,
) -> Result<ExecuteVcetTemporalFinalTestShardResponseV1, String> {
    let app_local_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    execute_vcet_temporal_final_test_shard_durable_blocking_with_loader(
        req,
        &app_local_data_dir,
        move |request| {
            // This closure is invoked only after the durable claim is acquired, from the same
            // owned blocking task that will publish its terminal record.
            let state = app.state::<AppState>();
            let mut loaded = Vec::with_capacity(request.loaded_datasets.len());
            for binding in &request.loaded_datasets {
                let candles = state
                    .get_candles_for_run(&binding.loaded_data_id)
                    .map_err(|error| {
                        format!(
                            "failed to resolve loaded candles '{}' for VCET final-test dataset '{}': {error}",
                            binding.loaded_data_id, binding.dataset_id
                        )
                    })?;
                loaded.push((binding.clone(), candles));
            }
            Ok(loaded)
        },
    )
}

#[tauri::command]
pub async fn execute_vcet_temporal_final_test_shard(
    req: ExecuteVcetTemporalFinalTestShardRequestV1,
    app: tauri::AppHandle,
) -> Result<ExecuteVcetTemporalFinalTestShardResponseV1, String> {
    let task_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        execute_vcet_temporal_final_test_shard_owned(req, task_app)
    })
    .await
    .map_err(|error| format!("VCET final-test owned task failed: {error}"))?
}

#[tauri::command]
pub async fn load_vcet_temporal_final_test_shard(
    req: LoadVcetTemporalFinalTestShardRequestV1,
    app: tauri::AppHandle,
) -> Result<ExecuteVcetTemporalFinalTestShardResponseV1, String> {
    let app_local_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let (request_sha256, response) = load_durable_final_completion_for_authorization::<
            ExecuteVcetTemporalFinalTestShardResponseV1,
        >(&app_local_data_dir, &req.authorization_sha256)?;
        validate_vcet_temporal_final_shard_artifact(&response)?;
        let requested_authorization = req
            .authorization_sha256
            .strip_prefix("sha256:")
            .unwrap_or(&req.authorization_sha256);
        let observed_authorization = response
            .payload
            .authorization_sha256
            .strip_prefix("sha256:")
            .unwrap_or(&response.payload.authorization_sha256);
        if !requested_authorization.eq_ignore_ascii_case(observed_authorization)
            || response.payload.request_sha256 != request_sha256
        {
            return Err(
                "VCET durable final-test receipt does not match its authorization claim identity"
                    .to_string(),
            );
        }
        Ok(response)
    })
    .await
    .map_err(|error| format!("VCET final-test recovery task failed: {error}"))?
}

fn validate_final_dataset_against_universe(
    dataset: &VcetTemporalFinalTestDatasetExecutionV1,
    expected: &VcetTemporalUniverseDatasetV1,
    winner: &VcetTemporalGlobalWinnerLockV1,
    execution_spec: &VcetTemporalPhasedExecutionSpecV1,
    interval_sha256: &str,
    minimum_trades: u64,
) -> Result<(), String> {
    if dataset.dataset_id != expected.dataset_id
        || dataset.source_dataset_sha256 != expected.source_dataset_sha256
        || dataset.dataset_attestation_sha256 != expected.dataset_attestation_sha256
        || dataset.timeline_sha256 != expected.timeline_sha256
        || dataset.loaded_candles_snapshot_sha256 != expected.loaded_candles_snapshot_sha256
        || dataset.dataset_coverage != expected.dataset_coverage
        || dataset.dataset_coverage_sha256 != expected.dataset_coverage_sha256
        || dataset.temporal_interval_fingerprint_sha256
            != expected.temporal_interval_fingerprint_sha256
        || dataset.temporal_interval_fingerprint_sha256 != interval_sha256
        || dataset.global_candidate_ordinal != winner.global_candidate_ordinal
        || dataset.candidate_id != winner.candidate_id
        || !serialized_eq(&dataset.params, &winner.params)?
    {
        return Err(format!(
            "VCET final-test dataset '{}' does not exactly match universe or locked-winner evidence",
            dataset.dataset_id
        ));
    }
    if dataset.loaded_data_id.trim() != dataset.loaded_data_id || dataset.loaded_data_id.is_empty()
    {
        return Err(format!(
            "VCET final-test dataset '{}' has a non-canonical loaded_data_id",
            dataset.dataset_id
        ));
    }
    if dataset.final_test.scored_bars.is_empty()
        || dataset.observed_trades != u64::from(dataset.metrics.trades)
        || dataset.minimum_trades != minimum_trades
        || dataset.minimum_trade_support_passed
            != (dataset.observed_trades >= dataset.minimum_trades)
    {
        return Err(format!(
            "VCET final-test dataset '{}' has inconsistent scored-window or trade-support evidence",
            dataset.dataset_id
        ));
    }
    validate_finite_metrics(&dataset.metrics, &dataset.dataset_id)?;
    validate_backend_provenance(&dataset.backend, &execution_spec.backend)
}

fn recompute_final_shard_set_root_sha256(
    universe_sha256: &str,
    winner_lock_sha256: &str,
    final_stage_nonce: &str,
    shards: &[VcetTemporalFinalShardReductionEvidenceV1],
) -> Result<String, String> {
    hash_serializable(
        &FinalShardSetRootInput {
            hash_contract: FINAL_SHARD_SET_ROOT_HASH_CONTRACT,
            universe_sha256,
            winner_lock_sha256,
            final_stage_nonce,
            shards,
        },
        "VCET final-test shard-set root",
    )
}

pub(super) fn recompute_vcet_temporal_final_reduction_artifact_sha256(
    payload: &VcetTemporalFinalReductionArtifactPayloadV1,
) -> Result<String, String> {
    hash_enveloped(
        FINAL_REDUCTION_ARTIFACT_HASH_CONTRACT,
        payload,
        "VCET final-test reduction artifact",
    )
}

pub(super) fn validate_vcet_temporal_final_reduction_artifact(
    response: &ReduceVcetTemporalFinalTestShardsResponseV1,
) -> Result<(), String> {
    let payload = &response.payload;
    if payload.schema_version != VCET_TEMPORAL_FINAL_REDUCTION_ARTIFACT_SCHEMA_VERSION
        || payload.evidence_kind != "vcet_temporal_final_test_reduction_artifact"
        || payload.logical_final_scoring_stage_open_count != 1
        || payload.final_test_candidate_count != 1
        || payload.physical_final_shard_count == 0
        || payload.expected_final_test_dataset_count == 0
        || payload.selection_used_final_test
        || payload.winner_reranked_after_final_test
        || payload.physical_final_shard_count != payload.shards.len()
        || payload.expected_final_test_dataset_count != payload.datasets.len()
        || payload.evaluated_final_test_dataset_count != payload.datasets.len()
        || payload.final_test_candidate_window_trial_count != payload.datasets.len()
        || payload.execution_spec_sha256 != payload.winner_lock.execution_spec_sha256
        || payload.universe_sha256 != payload.winner_lock.universe_sha256
        || payload.global_pre_final_evidence_sha256
            != payload.winner_lock.global_pre_final_evidence_sha256
        || payload.final_stage_nonce.trim().is_empty()
        || !is_canonical_sha256(&payload.shard_set_root_sha256)
    {
        return Err(
            "VCET final-test reduction artifact counts or no-rerank flags are invalid".to_string(),
        );
    }
    if payload.scoring_semantics
        != final_scoring_semantics(VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen)
        || payload.shards.windows(2).any(|pair| {
            pair[0].shard_ordinal >= pair[1].shard_ordinal || pair[0].shard_id == pair[1].shard_id
        })
        || payload
            .shards
            .iter()
            .enumerate()
            .any(|(expected_ordinal, shard)| {
                shard.shard_ordinal != expected_ordinal
                    || shard.dataset_count == 0
                    || shard.candidate_window_trial_count != shard.dataset_count
            })
        || payload
            .datasets
            .windows(2)
            .any(|pair| pair[0].dataset_id >= pair[1].dataset_id)
        || payload
            .shards
            .iter()
            .map(|shard| shard.dataset_count)
            .sum::<usize>()
            != payload.datasets.len()
        || payload
            .shards
            .iter()
            .map(|shard| shard.candidate_window_trial_count)
            .sum::<usize>()
            != payload.final_test_candidate_window_trial_count
        || payload
            .shards
            .iter()
            .all(|shard| shard.minimum_trade_support_passed)
            != payload
                .datasets
                .iter()
                .all(|dataset| dataset.minimum_trade_support_passed)
    {
        return Err(
            "VCET final-test reduction artifact ordering, semantics, or aggregate counts are invalid"
                .to_string(),
        );
    }
    let mut dataset_offset = 0usize;
    let mut shard_ids = BTreeSet::new();
    for shard in &payload.shards {
        if !is_canonical_final_shard_id(&shard.shard_id)
            || !shard_ids.insert(shard.shard_id.clone())
            || !is_canonical_sha256(&shard.authorization_sha256)
            || !is_canonical_sha256(&shard.authorized_prior_state_sha256)
            || !is_canonical_sha256(&shard.completed_state_sha256)
            || !is_canonical_sha256(&shard.artifact_sha256)
        {
            return Err(format!(
                "VCET final-test reduction shard '{}' has a duplicate/non-canonical ID or hash evidence",
                shard.shard_id
            ));
        }
        let dataset_end = dataset_offset
            .checked_add(shard.dataset_count)
            .ok_or_else(|| "VCET final-test reduction shard dataset offset overflow".to_string())?;
        let shard_datasets = payload
            .datasets
            .get(dataset_offset..dataset_end)
            .ok_or_else(|| {
                "VCET final-test reduction shard dataset partition is out of bounds".to_string()
            })?;
        if shard.minimum_trade_support_passed
            != shard_datasets
                .iter()
                .all(|dataset| dataset.minimum_trade_support_passed)
        {
            return Err(format!(
                "VCET final-test reduction shard '{}' support flag differs from its dataset partition",
                shard.shard_id
            ));
        }
        dataset_offset = dataset_end;
    }
    if dataset_offset != payload.datasets.len() {
        return Err(
            "VCET final-test reduction shard partitions do not exactly cover the dataset list"
                .to_string(),
        );
    }
    let mut common_final_test: Option<&VcetTemporalEvaluationSliceV1> = None;
    let mut common_minimum_trades: Option<u64> = None;
    let mut loaded_ids = BTreeSet::new();
    for dataset in &payload.datasets {
        validate_finite_metrics(&dataset.metrics, &dataset.dataset_id)?;
        if dataset.global_candidate_ordinal != payload.winner_lock.global_candidate_ordinal
            || dataset.candidate_id != payload.winner_lock.candidate_id
            || !serialized_eq(&dataset.params, &payload.winner_lock.params)?
            || dataset.temporal_interval_fingerprint_sha256
                != payload.temporal_interval_fingerprint_sha256
            || dataset.observed_trades != u64::from(dataset.metrics.trades)
            || dataset.minimum_trade_support_passed
                != (dataset.observed_trades >= dataset.minimum_trades)
            || dataset.final_test.scored_bars.is_empty()
            || dataset.loaded_data_id.trim() != dataset.loaded_data_id
            || dataset.loaded_data_id.is_empty()
            || dataset.dataset_id.trim() != dataset.dataset_id
            || dataset.dataset_id.is_empty()
            || !loaded_ids.insert(dataset.loaded_data_id.clone())
        {
            return Err(format!(
                "VCET final-test reduction dataset '{}' has inconsistent winner, interval, or support evidence",
                dataset.dataset_id
            ));
        }
        if let Some(expected_final_test) = common_final_test {
            if dataset.final_test != *expected_final_test {
                return Err(
                    "VCET final-test reduction datasets do not share one exact final scored interval"
                        .to_string(),
                );
            }
        } else {
            common_final_test = Some(&dataset.final_test);
        }
        if let Some(expected_minimum) = common_minimum_trades {
            if dataset.minimum_trades != expected_minimum {
                return Err(
                    "VCET final-test reduction datasets do not share one minimum-trade contract"
                        .to_string(),
                );
            }
        } else {
            common_minimum_trades = Some(dataset.minimum_trades);
        }
        let coverage_sha256 = recompute_vcet_dataset_coverage_sha256(
            &dataset.dataset_id,
            &dataset.loaded_candles_snapshot_sha256,
            &dataset.timeline_sha256,
            &dataset.dataset_coverage,
        )?;
        if coverage_sha256 != dataset.dataset_coverage_sha256 {
            return Err(format!(
                "VCET final-test reduction dataset '{}' coverage SHA-256 mismatch",
                dataset.dataset_id
            ));
        }
        validate_backend_provenance(&dataset.backend, &payload.execution_spec.backend)?;
    }
    let root = recompute_final_shard_set_root_sha256(
        &payload.universe_sha256,
        &payload.winner_lock.winner_lock_sha256,
        &payload.final_stage_nonce,
        &payload.shards,
    )?;
    if root != payload.shard_set_root_sha256 {
        return Err("VCET final-test shard-set root SHA-256 mismatch".to_string());
    }
    let support_passed = payload
        .datasets
        .iter()
        .all(|dataset| dataset.minimum_trade_support_passed);
    let expected_status = if support_passed {
        VcetTemporalFinalSupportStatusV1::SupportPassed
    } else {
        VcetTemporalFinalSupportStatusV1::SupportFailed
    };
    if payload.minimum_trade_support_passed != support_passed
        || payload.support_status != expected_status
        || payload.limitations != final_reduction_limitations()
    {
        return Err("VCET final-test reduction support evidence is inconsistent".to_string());
    }
    let actual = recompute_vcet_temporal_final_reduction_artifact_sha256(payload)?;
    if actual != response.artifact_sha256 {
        return Err(format!(
            "VCET final-test reduction artifact SHA-256 mismatch (declared {}, actual {actual})",
            response.artifact_sha256
        ));
    }
    Ok(())
}

pub fn reduce_vcet_temporal_final_test_shards_blocking(
    req: ReduceVcetTemporalFinalTestShardsRequestV1,
) -> Result<ReduceVcetTemporalFinalTestShardsResponseV1, String> {
    validate_sealed_vcet_temporal_universe(&req.sealed_universe, &req.expected_universe_sha256)?;
    validate_vcet_temporal_pre_final_reduction_artifact(
        &req.global_winner_reduction,
        &req.expected_global_winner_reduction_artifact_sha256,
    )?;
    let universe = &req.sealed_universe.payload;
    let reduction = &req.global_winner_reduction.payload;
    if req.sealed_universe.universe_sha256 != reduction.universe_sha256 {
        return Err(
            "VCET final reducer received a winner reduction from a different universe".to_string(),
        );
    }
    let mut expected_descriptors =
        universe_shards_for_role(universe, VcetTemporalDatasetCohortRoleV1::FinalTest);
    expected_descriptors.sort_by(|left, right| {
        left.shard_ordinal
            .cmp(&right.shard_ordinal)
            .then_with(|| left.shard_id.cmp(&right.shard_id))
    });
    if expected_descriptors.is_empty()
        || req.final_shard_artifacts.len() != expected_descriptors.len()
        || reduction.final_shard_authorizations.len() != expected_descriptors.len()
    {
        return Err(format!(
            "VCET final reducer requires exactly one artifact and authorization for each of {} final descriptors; received {} artifacts and {} authorizations",
            expected_descriptors.len(),
            req.final_shard_artifacts.len(),
            reduction.final_shard_authorizations.len()
        ));
    }
    let mut artifact_by_shard = BTreeMap::new();
    for artifact in &req.final_shard_artifacts {
        validate_vcet_temporal_final_shard_artifact(artifact)?;
        let shard_id = artifact.payload.final_shard.shard_id.clone();
        if artifact_by_shard
            .insert(shard_id.clone(), artifact)
            .is_some()
        {
            return Err(format!(
                "duplicate VCET final-test shard artifact for '{shard_id}'"
            ));
        }
    }

    let universe_entries = universe_entry_map(universe)?;
    let expected_final_dataset_ids = universe
        .datasets
        .iter()
        .filter(|dataset| dataset.role == VcetTemporalDatasetCohortRoleV1::FinalTest)
        .map(|dataset| dataset.dataset_id.clone())
        .collect::<BTreeSet<_>>();
    let minimum_trades = universe.plan.minimum_support.min_final_test_trades;
    let expected_semantics = final_scoring_semantics(universe.plan.position_boundary_policy);
    let mut observed_dataset_ids = BTreeSet::new();
    let mut shard_evidence = Vec::with_capacity(expected_descriptors.len());
    let mut datasets = Vec::with_capacity(expected_final_dataset_ids.len());
    let mut common_final_test: Option<VcetTemporalEvaluationSliceV1> = None;
    for descriptor in expected_descriptors {
        let artifact = artifact_by_shard
            .remove(&descriptor.shard_id)
            .ok_or_else(|| {
                format!(
                    "missing VCET final-test shard artifact for canonical descriptor '{}'",
                    descriptor.shard_id
                )
            })?;
        let payload = &artifact.payload;
        let authorization = find_final_authorization(&req.global_winner_reduction, descriptor)?;
        validate_vcet_temporal_final_shard_authorization(
            &authorization,
            &req.sealed_universe,
            &reduction.winner_lock,
        )?;
        validate_completed_final_authorization_artifact(
            &authorization.authorization_sha256,
            &payload.request_sha256,
            artifact,
        )?;
        if !serialized_eq(&payload.final_shard, descriptor)?
            || payload.universe_sha256 != req.sealed_universe.universe_sha256
            || payload.global_winner_reduction_artifact_sha256
                != req.global_winner_reduction.artifact_sha256
            || payload.global_pre_final_evidence_sha256
                != reduction.global_pre_final_evidence_sha256
            || !serialized_eq(&payload.winner_lock, &reduction.winner_lock)?
            || payload.authorization_sha256 != authorization.authorization_sha256
            || payload.authorized_prior_state_sha256 != authorization.expected_prior_state_sha256
            || payload.final_stage_nonce != reduction.final_stage_nonce
            || payload.final_stage_nonce != authorization.final_stage_nonce
            || payload.execution_spec_sha256 != reduction.winner_lock.execution_spec_sha256
            || !serialized_eq(&payload.execution_spec, &universe.execution_spec)?
            || payload.temporal_interval_fingerprint_sha256
                != universe.temporal_interval_fingerprint_sha256
            || payload.scoring_semantics != expected_semantics
            || payload.datasets.len() != descriptor.dataset_ids.len()
            || payload.expected_candidate_window_trial_count
                != descriptor.expected_candidate_window_trial_count
            || payload.evaluated_candidate_window_trial_count
                != descriptor.expected_candidate_window_trial_count
        {
            return Err(format!(
                "VCET final-test shard artifact '{}' differs from its universe, winner, authorization, interval, or execution spec",
                descriptor.shard_id
            ));
        }
        let artifact_dataset_ids = payload
            .datasets
            .iter()
            .map(|dataset| dataset.dataset_id.as_str())
            .collect::<Vec<_>>();
        let descriptor_dataset_ids = descriptor
            .dataset_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if artifact_dataset_ids != descriptor_dataset_ids {
            return Err(format!(
                "VCET final-test shard '{}' dataset order or coverage differs from its descriptor",
                descriptor.shard_id
            ));
        }
        for dataset in &payload.datasets {
            if !observed_dataset_ids.insert(dataset.dataset_id.clone()) {
                return Err(format!(
                    "VCET final-test dataset '{}' appears in more than one shard artifact",
                    dataset.dataset_id
                ));
            }
            let expected = universe_entries.get(&dataset.dataset_id).ok_or_else(|| {
                format!(
                    "VCET final-test artifact contains unknown dataset '{}'",
                    dataset.dataset_id
                )
            })?;
            if expected.role != VcetTemporalDatasetCohortRoleV1::FinalTest {
                return Err(format!(
                    "VCET final-test artifact contains selection dataset '{}'",
                    dataset.dataset_id
                ));
            }
            validate_final_dataset_against_universe(
                dataset,
                expected,
                &reduction.winner_lock,
                &universe.execution_spec,
                &universe.temporal_interval_fingerprint_sha256,
                minimum_trades,
            )?;
            if let Some(expected_final_test) = common_final_test.as_ref() {
                if dataset.final_test != *expected_final_test {
                    return Err(format!(
                        "VCET final-test dataset '{}' does not share the common exact final scored interval",
                        dataset.dataset_id
                    ));
                }
            } else {
                common_final_test = Some(dataset.final_test.clone());
            }
            datasets.push(dataset.clone());
        }
        shard_evidence.push(VcetTemporalFinalShardReductionEvidenceV1 {
            shard_ordinal: descriptor.shard_ordinal,
            shard_id: descriptor.shard_id.clone(),
            authorization_sha256: authorization.authorization_sha256.clone(),
            authorized_prior_state_sha256: authorization.expected_prior_state_sha256.clone(),
            completed_state_sha256: payload.completed_state_sha256.clone(),
            artifact_sha256: artifact.artifact_sha256.clone(),
            dataset_count: payload.datasets.len(),
            candidate_window_trial_count: payload.evaluated_candidate_window_trial_count,
            minimum_trade_support_passed: payload.minimum_trade_support_passed,
        });
    }
    if !artifact_by_shard.is_empty() || observed_dataset_ids != expected_final_dataset_ids {
        return Err(
            "VCET final-test shard set does not provide complete exact non-overlapping final cohort coverage"
                .to_string(),
        );
    }
    datasets.sort_by(|left, right| left.dataset_id.cmp(&right.dataset_id));
    let shard_set_root_sha256 = recompute_final_shard_set_root_sha256(
        &req.sealed_universe.universe_sha256,
        &reduction.winner_lock.winner_lock_sha256,
        &reduction.final_stage_nonce,
        &shard_evidence,
    )?;
    let minimum_trade_support_passed = datasets
        .iter()
        .all(|dataset| dataset.minimum_trade_support_passed);
    let support_status = if minimum_trade_support_passed {
        VcetTemporalFinalSupportStatusV1::SupportPassed
    } else {
        VcetTemporalFinalSupportStatusV1::SupportFailed
    };
    let payload = VcetTemporalFinalReductionArtifactPayloadV1 {
        schema_version: VCET_TEMPORAL_FINAL_REDUCTION_ARTIFACT_SCHEMA_VERSION,
        evidence_kind: "vcet_temporal_final_test_reduction_artifact".to_string(),
        universe_sha256: req.sealed_universe.universe_sha256.clone(),
        global_winner_reduction_artifact_sha256: req
            .global_winner_reduction
            .artifact_sha256
            .clone(),
        global_pre_final_evidence_sha256: reduction.global_pre_final_evidence_sha256.clone(),
        winner_lock: reduction.winner_lock.clone(),
        final_stage_nonce: reduction.final_stage_nonce.clone(),
        execution_spec_sha256: reduction.winner_lock.execution_spec_sha256.clone(),
        execution_spec: universe.execution_spec.clone(),
        temporal_interval_fingerprint_sha256: universe.temporal_interval_fingerprint_sha256.clone(),
        scoring_semantics: expected_semantics,
        logical_final_scoring_stage_open_count: 1,
        physical_final_shard_count: shard_evidence.len(),
        final_test_candidate_count: 1,
        expected_final_test_dataset_count: expected_final_dataset_ids.len(),
        evaluated_final_test_dataset_count: datasets.len(),
        final_test_candidate_window_trial_count: datasets.len(),
        shard_set_root_sha256,
        shards: shard_evidence,
        datasets,
        support_status,
        minimum_trade_support_passed,
        selection_used_final_test: false,
        winner_reranked_after_final_test: false,
        limitations: final_reduction_limitations(),
    };
    let artifact_sha256 = recompute_vcet_temporal_final_reduction_artifact_sha256(&payload)?;
    let response = ReduceVcetTemporalFinalTestShardsResponseV1 {
        payload,
        artifact_sha256,
    };
    validate_vcet_temporal_final_reduction_artifact(&response)?;
    Ok(response)
}

struct PreparedFinalReductionFanIn {
    ordered_authorizations: Vec<VcetTemporalFinalShardAuthorizationV1>,
    supplied_by_shard: BTreeMap<String, ExecuteVcetTemporalFinalTestShardResponseV1>,
}

#[derive(Debug)]
struct LoadedDurableFinalReceipt {
    authorization_sha256: String,
    request_sha256: String,
    response: ExecuteVcetTemporalFinalTestShardResponseV1,
}

fn validate_final_receipt_against_reduction_authority(
    req: &ReduceVcetTemporalFinalTestShardsRequestV1,
    authorization: &VcetTemporalFinalShardAuthorizationV1,
    artifact: &ExecuteVcetTemporalFinalTestShardResponseV1,
) -> Result<(), String> {
    validate_vcet_temporal_final_shard_artifact(artifact)?;
    let payload = &artifact.payload;
    let universe = &req.sealed_universe.payload;
    let reduction = &req.global_winner_reduction.payload;
    if !serialized_eq(&payload.final_shard, &authorization.shard)?
        || payload.universe_sha256 != req.sealed_universe.universe_sha256
        || payload.global_winner_reduction_artifact_sha256
            != req.expected_global_winner_reduction_artifact_sha256
        || payload.global_winner_reduction_artifact_sha256
            != req.global_winner_reduction.artifact_sha256
        || payload.global_pre_final_evidence_sha256 != reduction.global_pre_final_evidence_sha256
        || !serialized_eq(&payload.winner_lock, &reduction.winner_lock)?
        || payload.authorization_sha256 != authorization.authorization_sha256
        || payload.authorized_prior_state_sha256 != authorization.expected_prior_state_sha256
        || payload.final_stage_nonce != reduction.final_stage_nonce
        || payload.final_stage_nonce != authorization.final_stage_nonce
        || payload.execution_spec_sha256 != reduction.winner_lock.execution_spec_sha256
        || !serialized_eq(&payload.execution_spec, &universe.execution_spec)?
        || payload.temporal_interval_fingerprint_sha256
            != universe.temporal_interval_fingerprint_sha256
        || payload.scoring_semantics
            != final_scoring_semantics(universe.plan.position_boundary_policy)
        || payload.datasets.len() != authorization.shard.dataset_ids.len()
        || payload.expected_candidate_window_trial_count
            != authorization.shard.expected_candidate_window_trial_count
        || payload.evaluated_candidate_window_trial_count
            != authorization.shard.expected_candidate_window_trial_count
    {
        return Err(format!(
            "VCET final-test shard receipt '{}' differs from its sealed universe, winner, authorization, interval, or execution spec",
            authorization.shard.shard_id
        ));
    }
    let artifact_dataset_ids = payload
        .datasets
        .iter()
        .map(|dataset| dataset.dataset_id.as_str())
        .collect::<Vec<_>>();
    let descriptor_dataset_ids = authorization
        .shard
        .dataset_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if artifact_dataset_ids != descriptor_dataset_ids {
        return Err(format!(
            "VCET final-test shard receipt '{}' dataset order differs from its sealed descriptor",
            authorization.shard.shard_id
        ));
    }
    Ok(())
}

fn order_exact_final_authorizations_with<V>(
    expected_descriptors: &[VcetTemporalUniverseShardV1],
    authorizations: &[VcetTemporalFinalShardAuthorizationV1],
    mut validate_authorization: V,
) -> Result<Vec<VcetTemporalFinalShardAuthorizationV1>, String>
where
    V: FnMut(&VcetTemporalFinalShardAuthorizationV1) -> Result<(), String>,
{
    let mut expected_descriptors = expected_descriptors.to_vec();
    expected_descriptors.sort_by(|left, right| {
        left.shard_ordinal
            .cmp(&right.shard_ordinal)
            .then_with(|| left.shard_id.cmp(&right.shard_id))
    });
    if expected_descriptors.is_empty() || authorizations.len() != expected_descriptors.len() {
        return Err(format!(
            "VCET durable final reducer requires exactly one authorization for each of {} sealed final descriptors; received {} authorizations",
            expected_descriptors.len(),
            authorizations.len()
        ));
    }

    let mut expected_by_shard = BTreeMap::new();
    let mut descriptor_ordinals = BTreeSet::new();
    for descriptor in &expected_descriptors {
        if expected_by_shard
            .insert(descriptor.shard_id.as_str(), descriptor)
            .is_some()
            || !descriptor_ordinals.insert(descriptor.shard_ordinal)
        {
            return Err(
                "sealed VCET final descriptor cohort contains duplicate shard IDs or ordinals"
                    .to_string(),
            );
        }
    }

    let mut authorization_by_shard = BTreeMap::new();
    let mut authorization_hashes = BTreeSet::new();
    let mut prior_state_hashes = BTreeSet::new();
    for authorization in authorizations {
        validate_authorization(authorization)?;
        let descriptor = expected_by_shard
            .get(authorization.shard.shard_id.as_str())
            .ok_or_else(|| {
                format!(
                    "VCET final authorization names unknown sealed shard '{}'",
                    authorization.shard.shard_id
                )
            })?;
        if !serialized_eq(&authorization.shard, *descriptor)? {
            return Err(format!(
                "VCET final authorization shard '{}' is not the exact sealed descriptor",
                authorization.shard.shard_id
            ));
        }
        if authorization_by_shard
            .insert(authorization.shard.shard_id.clone(), authorization)
            .is_some()
            || !authorization_hashes.insert(authorization.authorization_sha256.clone())
            || !prior_state_hashes.insert(authorization.expected_prior_state_sha256.clone())
        {
            return Err(
                "VCET final authorization cohort contains duplicate shard, authorization, or prior-state identities"
                    .to_string(),
            );
        }
    }

    let mut ordered_authorizations = Vec::with_capacity(expected_descriptors.len());
    for descriptor in expected_descriptors {
        let authorization = authorization_by_shard
            .remove(&descriptor.shard_id)
            .ok_or_else(|| {
                format!(
                    "missing VCET final authorization for sealed shard '{}'",
                    descriptor.shard_id
                )
            })?;
        ordered_authorizations.push((*authorization).clone());
    }
    if !authorization_by_shard.is_empty() {
        return Err("VCET final authorization cohort contains unexpected shards".to_string());
    }
    Ok(ordered_authorizations)
}

fn prevalidate_final_reduction_before_receipt_io(
    req: &ReduceVcetTemporalFinalTestShardsRequestV1,
) -> Result<PreparedFinalReductionFanIn, String> {
    validate_sealed_vcet_temporal_universe(&req.sealed_universe, &req.expected_universe_sha256)?;
    validate_vcet_temporal_pre_final_reduction_artifact(
        &req.global_winner_reduction,
        &req.expected_global_winner_reduction_artifact_sha256,
    )?;
    let reduction = &req.global_winner_reduction.payload;
    if req.sealed_universe.universe_sha256 != reduction.universe_sha256 {
        return Err(
            "VCET final reducer received a winner reduction from a different universe".to_string(),
        );
    }

    let expected_descriptors = universe_shards_for_role(
        &req.sealed_universe.payload,
        VcetTemporalDatasetCohortRoleV1::FinalTest,
    )
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();
    let ordered_authorizations = order_exact_final_authorizations_with(
        &expected_descriptors,
        &reduction.final_shard_authorizations,
        |authorization| {
            validate_vcet_temporal_final_shard_authorization(
                authorization,
                &req.sealed_universe,
                &reduction.winner_lock,
            )
        },
    )?;

    if !req.final_shard_artifacts.is_empty()
        && req.final_shard_artifacts.len() != ordered_authorizations.len()
    {
        return Err(format!(
            "caller-supplied VCET final receipts must be empty or cover all {} sealed final shards; received {}",
            ordered_authorizations.len(),
            req.final_shard_artifacts.len()
        ));
    }
    let authorization_by_shard = ordered_authorizations
        .iter()
        .map(|authorization| (authorization.shard.shard_id.as_str(), authorization))
        .collect::<BTreeMap<_, _>>();
    let mut supplied_by_shard = BTreeMap::new();
    let mut supplied_authorizations = BTreeSet::new();
    let mut supplied_requests = BTreeSet::new();
    let mut supplied_completed_states = BTreeSet::new();
    for artifact in &req.final_shard_artifacts {
        validate_vcet_temporal_final_shard_artifact(artifact)?;
        let shard_id = artifact.payload.final_shard.shard_id.clone();
        let authorization = authorization_by_shard
            .get(shard_id.as_str())
            .ok_or_else(|| format!("caller supplied unknown VCET final shard '{shard_id}'"))?;
        validate_final_receipt_against_reduction_authority(req, authorization, artifact)?;
        if supplied_by_shard
            .insert(shard_id, artifact.clone())
            .is_some()
            || !supplied_authorizations.insert(artifact.payload.authorization_sha256.clone())
            || !supplied_requests.insert(artifact.payload.request_sha256.clone())
            || !supplied_completed_states.insert(artifact.payload.completed_state_sha256.clone())
        {
            return Err(
                "caller-supplied VCET final receipts contain duplicate shard, authorization, request, or completed-state identities"
                    .to_string(),
            );
        }
    }
    if !supplied_by_shard.is_empty() && supplied_by_shard.len() != ordered_authorizations.len() {
        return Err(
            "caller-supplied VCET final receipts do not exactly cover the sealed authorization cohort"
                .to_string(),
        );
    }
    Ok(PreparedFinalReductionFanIn {
        ordered_authorizations,
        supplied_by_shard,
    })
}

fn load_expected_final_receipts_with<F, V>(
    prepared: &PreparedFinalReductionFanIn,
    mut load: F,
    mut validate: V,
) -> Result<Vec<LoadedDurableFinalReceipt>, String>
where
    F: FnMut(&str) -> Result<(String, ExecuteVcetTemporalFinalTestShardResponseV1), String>,
    V: FnMut(
        &VcetTemporalFinalShardAuthorizationV1,
        &ExecuteVcetTemporalFinalTestShardResponseV1,
    ) -> Result<(), String>,
{
    let mut receipts = Vec::with_capacity(prepared.ordered_authorizations.len());
    for authorization in &prepared.ordered_authorizations {
        let (request_sha256, response) = load(&authorization.authorization_sha256)?;
        if response.payload.request_sha256 != request_sha256 {
            return Err(format!(
                "durable VCET final receipt for authorization {} declares request {} but its canonical claim declares {}",
                authorization.authorization_sha256,
                response.payload.request_sha256,
                request_sha256
            ));
        }
        validate(authorization, &response)?;
        if let Some(supplied) = prepared
            .supplied_by_shard
            .get(&authorization.shard.shard_id)
        {
            if !serialized_eq(supplied, &response)? {
                return Err(format!(
                    "caller-supplied VCET final shard '{}' differs from its durable completed receipt",
                    authorization.shard.shard_id
                ));
            }
        }
        receipts.push(LoadedDurableFinalReceipt {
            authorization_sha256: authorization.authorization_sha256.clone(),
            request_sha256,
            response,
        });
    }
    Ok(receipts)
}

#[allow(dead_code)]
pub(super) fn reduce_vcet_temporal_final_test_shards_durable_blocking(
    mut req: ReduceVcetTemporalFinalTestShardsRequestV1,
    app_local_data_dir: &Path,
) -> Result<ReduceVcetTemporalFinalTestShardsResponseV1, String> {
    // All caller-controlled cohort shape and artifact validation precedes durable receipt I/O.
    let prepared = prevalidate_final_reduction_before_receipt_io(&req)?;
    let durable_winner: ReduceVcetTemporalPreFinalShardsResponseV1 =
        load_durable_pre_final_winner_lock(app_local_data_dir, &req.expected_universe_sha256)?;
    validate_vcet_temporal_pre_final_reduction_artifact(
        &durable_winner,
        &req.expected_global_winner_reduction_artifact_sha256,
    )?;
    if !serialized_eq(&durable_winner, &req.global_winner_reduction)? {
        return Err(
            "VCET final reducer winner reduction differs from the durable pre-final winner-lock receipt"
                .to_string(),
        );
    }

    let loaded = load_expected_final_receipts_with(
        &prepared,
        |authorization_sha256| {
            load_durable_final_completion_for_authorization(
                app_local_data_dir,
                authorization_sha256,
            )
        },
        |authorization, response| {
            validate_final_receipt_against_reduction_authority(&req, authorization, response)
        },
    )?;
    for receipt in &loaded {
        hydrate_completed_final_authorization(
            &receipt.authorization_sha256,
            &receipt.request_sha256,
            receipt.response.clone(),
        )?;
    }
    req.final_shard_artifacts = loaded.into_iter().map(|receipt| receipt.response).collect();
    reduce_vcet_temporal_final_test_shards_blocking(req)
}

#[tauri::command]
pub async fn reduce_vcet_temporal_final_test_shards(
    req: ReduceVcetTemporalFinalTestShardsRequestV1,
    app: tauri::AppHandle,
) -> Result<ReduceVcetTemporalFinalTestShardsResponseV1, String> {
    let app_local_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        reduce_vcet_temporal_final_test_shards_durable_blocking(req, &app_local_data_dir)
    })
    .await
    .map_err(|error| format!("VCET final-test reduction task failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::super::commands_vcet_temporal::VcetTemporalDatasetTimelineInputV1;
    use super::super::commands_vcet_temporal_execute::VcetTemporalSelectionPolicyV1;
    use super::super::commands_vcet_temporal_prefinal::temporal_report_pre_final_fixture;
    use super::super::commands_vcet_temporal_universe::{
        VcetTemporalCrossUniverseAdmissionV1,
        VCET_TEMPORAL_CROSS_UNIVERSE_ADMISSION_SCHEMA_VERSION,
        VCET_TEMPORAL_SELECTION_REDUCTION_CONTRACT_V1,
    };
    use super::super::vcet_temporal_durable_ledger::admit_durable_pre_final_winner_lock;
    use super::*;
    use ta_app_core::vcet_dataset_coverage::{
        profile_vcet_dataset_coverage, VcetDatasetCoverageOptions,
    };
    use ta_app_core::vcet_search::VcetCandidateIdentityInputs;
    use ta_app_core::vcet_temporal::{VcetTemporalBarRangeV1, VcetTemporalUtcBoundsV1};
    use ta_strategies::double_ma::StrategyConfig;

    fn test_hash(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn test_shard_id(byte: char) -> String {
        format!("vcet-temporal-shard-v1-{}", byte.to_string().repeat(64))
    }

    fn test_params() -> CompositeParams {
        candidate_params_from_manifest(
            &VcetCandidateIdentityInputs {
                vidya_short_period: 8,
                vidya_long_period: 32,
                vidya_alpha_micros: 200_000,
                ce_period: 20,
                ce_mult_micros: 3_000_000,
                ce_use_close: true,
                require_vidya_slope: true,
                max_trade_bars: 4,
            },
            &StrategyConfig::default(),
        )
    }

    fn test_execution_spec() -> VcetTemporalPhasedExecutionSpecV1 {
        VcetTemporalPhasedExecutionSpecV1 {
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            strategy: StrategyConfig::default(),
            selection_policy: VcetTemporalSelectionPolicyV1::MeanValidationSharpe,
            selection_reduction_contract: VCET_TEMPORAL_SELECTION_REDUCTION_CONTRACT_V1.to_string(),
            cross_universe_admission: VcetTemporalCrossUniverseAdmissionV1 {
                schema_version: VCET_TEMPORAL_CROSS_UNIVERSE_ADMISSION_SCHEMA_VERSION,
                min_requested_dataset_coverage_share: 0.0,
                min_evaluated_dataset_share_requested: 0.0,
                min_profitable_dataset_share: 0.0,
                min_hard_support_dataset_share: 0.0,
                min_metric_support_dataset_share: 0.0,
                min_validation_pnl_p10: -1_000_000.0,
                min_validation_sharpe_p10: -1_000_000.0,
            },
        }
    }

    fn test_winner_lock(params: &CompositeParams) -> VcetTemporalGlobalWinnerLockV1 {
        VcetTemporalGlobalWinnerLockV1 {
            global_candidate_ordinal: 0,
            candidate_id: "vcet_candidate_test".to_string(),
            params: params.clone(),
            selection_policy: VcetTemporalSelectionPolicyV1::MeanValidationSharpe,
            selection_score: 1.0,
            runner_up_candidate_id: None,
            runner_up_selection_score: None,
            winner_to_runner_up_score_margin: None,
            candidate_id_tie_break_used: false,
            selection_reduction_contract: VCET_TEMPORAL_SELECTION_REDUCTION_CONTRACT_V1.to_string(),
            universe_sha256: test_hash('a'),
            candidate_manifest_sha256: test_hash('b'),
            plan_sha256: test_hash('c'),
            execution_spec_sha256: test_hash('d'),
            global_pre_final_evidence_sha256: test_hash('e'),
            winner_lock_sha256: test_hash('f'),
        }
    }

    fn test_final_shard_response() -> ExecuteVcetTemporalFinalTestShardResponseV1 {
        let params = test_params();
        let winner_lock = test_winner_lock(&params);
        let execution_spec = test_execution_spec();
        let timestamps = vec![0_i64, 60_000, 120_000];
        let coverage =
            profile_vcet_dataset_coverage(&timestamps, VcetDatasetCoverageOptions::default())
                .expect("coverage");
        let snapshot_sha256 = test_hash('1');
        let timeline_sha256 = test_hash('2');
        let coverage_sha256 = recompute_vcet_dataset_coverage_sha256(
            "FINAL-A",
            &snapshot_sha256,
            &timeline_sha256,
            &coverage,
        )
        .expect("coverage hash");
        let final_test = VcetTemporalEvaluationSliceV1 {
            warmup_bars: VcetTemporalBarRangeV1 {
                start_index: 0,
                end_index_exclusive: 1,
            },
            scored_bars: VcetTemporalBarRangeV1 {
                start_index: 1,
                end_index_exclusive: 3,
            },
            scored_utc: VcetTemporalUtcBoundsV1 {
                start_utc_ms_inclusive: 60_000,
                end_utc_ms_inclusive: 120_000,
            },
            scored_session: None,
        };
        let backend = VcetFixedCandidateWindowBackendProvenance {
            requested_backend: "CpuOnly".to_string(),
            realized_backend: "Cpu".to_string(),
            backend_route: "CpuOnly".to_string(),
            device_id: None,
            strict_backend: true,
            fallback_used: false,
            metric_contract: "full_cpu_metrics_v1".to_string(),
            unavailable_metric_fields: Vec::new(),
        };
        let descriptor = VcetTemporalUniverseShardV1 {
            role: VcetTemporalDatasetCohortRoleV1::FinalTest,
            shard_ordinal: 0,
            shard_id: test_shard_id('3'),
            dataset_ids: vec!["FINAL-A".to_string()],
            candidate_start_index: 0,
            candidate_end_index_exclusive: 0,
            locked_winner_only: true,
            expected_candidate_window_trial_count: 1,
        };
        let request_sha256 = test_hash('4');
        let authorization_sha256 = test_hash('5');
        let prior_sha256 = test_hash('6');
        let interval_sha256 = test_hash('7');
        let datasets = vec![VcetTemporalFinalTestDatasetExecutionV1 {
            dataset_id: "FINAL-A".to_string(),
            loaded_data_id: "loaded-final-a".to_string(),
            source_dataset_sha256: test_hash('8'),
            dataset_attestation_sha256: test_hash('9'),
            timeline_sha256,
            loaded_candles_snapshot_sha256: snapshot_sha256,
            dataset_coverage: coverage,
            dataset_coverage_sha256: coverage_sha256,
            temporal_interval_fingerprint_sha256: interval_sha256.clone(),
            final_test,
            global_candidate_ordinal: 0,
            candidate_id: winner_lock.candidate_id.clone(),
            params,
            metrics: Metrics::default(),
            backend,
            observed_trades: 0,
            minimum_trades: 0,
            minimum_trade_support_passed: true,
        }];
        let completed_state_sha256 = recompute_completed_state_sha256(
            &authorization_sha256,
            &request_sha256,
            &prior_sha256,
            &winner_lock.universe_sha256,
            &winner_lock.winner_lock_sha256,
            &descriptor,
            &datasets,
            1,
            1,
        )
        .expect("completed state");
        let payload = VcetTemporalFinalTestShardArtifactPayloadV1 {
            schema_version: VCET_TEMPORAL_FINAL_SHARD_ARTIFACT_SCHEMA_VERSION,
            evidence_kind: "vcet_temporal_final_test_shard_artifact".to_string(),
            universe_sha256: winner_lock.universe_sha256.clone(),
            global_winner_reduction_artifact_sha256: test_hash('a'),
            global_pre_final_evidence_sha256: winner_lock.global_pre_final_evidence_sha256.clone(),
            winner_lock,
            authorization_sha256,
            final_stage_nonce: "test-final-nonce".to_string(),
            final_shard: descriptor,
            planning_evidence_sha256: test_hash('b'),
            execution_spec_sha256: test_hash('d'),
            execution_spec,
            temporal_interval_fingerprint_sha256: interval_sha256,
            scoring_semantics: final_scoring_semantics(
                VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen,
            ),
            request_sha256,
            authorized_prior_state_sha256: prior_sha256,
            completed_state_sha256,
            expected_candidate_window_trial_count: 1,
            evaluated_candidate_window_trial_count: 1,
            physical_shard_execution_count: 1,
            candidate_count: 1,
            datasets,
            support_status: VcetTemporalFinalSupportStatusV1::SupportPassed,
            minimum_trade_support_passed: true,
            selection_used_final_test: false,
            winner_reranked_after_final_test: false,
            limitations: final_shard_limitations(),
        };
        let artifact_sha256 =
            recompute_vcet_temporal_final_shard_artifact_sha256(&payload).expect("artifact hash");
        ExecuteVcetTemporalFinalTestShardResponseV1 {
            payload,
            artifact_sha256,
        }
    }

    fn rehash_shard(response: &mut ExecuteVcetTemporalFinalTestShardResponseV1) {
        response.payload.completed_state_sha256 = recompute_completed_state_sha256(
            &response.payload.authorization_sha256,
            &response.payload.request_sha256,
            &response.payload.authorized_prior_state_sha256,
            &response.payload.universe_sha256,
            &response.payload.winner_lock.winner_lock_sha256,
            &response.payload.final_shard,
            &response.payload.datasets,
            response.payload.expected_candidate_window_trial_count,
            response.payload.evaluated_candidate_window_trial_count,
        )
        .expect("completed state");
        response.artifact_sha256 =
            recompute_vcet_temporal_final_shard_artifact_sha256(&response.payload)
                .expect("artifact hash");
    }

    fn test_final_reduction_response() -> ReduceVcetTemporalFinalTestShardsResponseV1 {
        let shard = test_final_shard_response();
        let shards = vec![VcetTemporalFinalShardReductionEvidenceV1 {
            shard_ordinal: 0,
            shard_id: shard.payload.final_shard.shard_id.clone(),
            authorization_sha256: shard.payload.authorization_sha256.clone(),
            authorized_prior_state_sha256: shard.payload.authorized_prior_state_sha256.clone(),
            completed_state_sha256: shard.payload.completed_state_sha256.clone(),
            artifact_sha256: shard.artifact_sha256.clone(),
            dataset_count: 1,
            candidate_window_trial_count: 1,
            minimum_trade_support_passed: true,
        }];
        let root = recompute_final_shard_set_root_sha256(
            &shard.payload.universe_sha256,
            &shard.payload.winner_lock.winner_lock_sha256,
            &shard.payload.final_stage_nonce,
            &shards,
        )
        .expect("root");
        let payload = VcetTemporalFinalReductionArtifactPayloadV1 {
            schema_version: VCET_TEMPORAL_FINAL_REDUCTION_ARTIFACT_SCHEMA_VERSION,
            evidence_kind: "vcet_temporal_final_test_reduction_artifact".to_string(),
            universe_sha256: shard.payload.universe_sha256.clone(),
            global_winner_reduction_artifact_sha256: shard
                .payload
                .global_winner_reduction_artifact_sha256
                .clone(),
            global_pre_final_evidence_sha256: shard
                .payload
                .global_pre_final_evidence_sha256
                .clone(),
            winner_lock: shard.payload.winner_lock.clone(),
            final_stage_nonce: shard.payload.final_stage_nonce.clone(),
            execution_spec_sha256: shard.payload.execution_spec_sha256.clone(),
            execution_spec: shard.payload.execution_spec.clone(),
            temporal_interval_fingerprint_sha256: shard
                .payload
                .temporal_interval_fingerprint_sha256
                .clone(),
            scoring_semantics: shard.payload.scoring_semantics.clone(),
            logical_final_scoring_stage_open_count: 1,
            physical_final_shard_count: 1,
            final_test_candidate_count: 1,
            expected_final_test_dataset_count: 1,
            evaluated_final_test_dataset_count: 1,
            final_test_candidate_window_trial_count: 1,
            shard_set_root_sha256: root,
            shards,
            datasets: shard.payload.datasets.clone(),
            support_status: VcetTemporalFinalSupportStatusV1::SupportPassed,
            minimum_trade_support_passed: true,
            selection_used_final_test: false,
            winner_reranked_after_final_test: false,
            limitations: final_reduction_limitations(),
        };
        let artifact_sha256 =
            recompute_vcet_temporal_final_reduction_artifact_sha256(&payload).expect("hash");
        ReduceVcetTemporalFinalTestShardsResponseV1 {
            payload,
            artifact_sha256,
        }
    }

    pub(crate) fn temporal_report_final_fixture(
        sealed_universe: &VcetTemporalSealedDatasetUniverseV1,
        pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
    ) -> ReduceVcetTemporalFinalTestShardsResponseV1 {
        let universe = &sealed_universe.payload;
        let final_dataset = universe
            .datasets
            .iter()
            .find(|dataset| dataset.role == VcetTemporalDatasetCohortRoleV1::FinalTest)
            .expect("temporal-report fixture final dataset");
        let authorization = pre_final
            .payload
            .final_shard_authorizations
            .iter()
            .find(|authorization| {
                authorization
                    .shard
                    .dataset_ids
                    .iter()
                    .any(|dataset_id| dataset_id == &final_dataset.dataset_id)
            })
            .expect("temporal-report fixture final authorization");
        let minimum_trades = universe.plan.minimum_support.min_final_test_trades;
        let mut metrics = Metrics {
            pnl: 0.12,
            sharpe: 1.1,
            sortino: 1.3,
            max_dd: 0.05,
            trades: u32::try_from(minimum_trades).expect("fixture trades fit u32"),
            exposure: 0.42,
            net_exposure: 0.10,
            ..Metrics::default()
        };
        if metrics.trades == 0 {
            metrics.trades = 1;
        }
        let observed_trades = u64::from(metrics.trades);
        let final_test = VcetTemporalEvaluationSliceV1 {
            warmup_bars: VcetTemporalBarRangeV1 {
                start_index: 921,
                end_index_exclusive: 971,
            },
            scored_bars: VcetTemporalBarRangeV1 {
                start_index: 971,
                end_index_exclusive: 1_001,
            },
            scored_utc: VcetTemporalUtcBoundsV1 {
                start_utc_ms_inclusive: 971_000,
                end_utc_ms_inclusive: 1_000_000,
            },
            scored_session: None,
        };
        let dataset = VcetTemporalFinalTestDatasetExecutionV1 {
            dataset_id: final_dataset.dataset_id.clone(),
            loaded_data_id: "loaded-final-dataset-report-fixture".to_string(),
            source_dataset_sha256: final_dataset.source_dataset_sha256.clone(),
            dataset_attestation_sha256: final_dataset.dataset_attestation_sha256.clone(),
            timeline_sha256: final_dataset.timeline_sha256.clone(),
            loaded_candles_snapshot_sha256: final_dataset.loaded_candles_snapshot_sha256.clone(),
            dataset_coverage: final_dataset.dataset_coverage.clone(),
            dataset_coverage_sha256: final_dataset.dataset_coverage_sha256.clone(),
            temporal_interval_fingerprint_sha256: final_dataset
                .temporal_interval_fingerprint_sha256
                .clone(),
            final_test,
            global_candidate_ordinal: pre_final.payload.winner_lock.global_candidate_ordinal,
            candidate_id: pre_final.payload.winner_lock.candidate_id.clone(),
            params: pre_final.payload.winner_lock.params.clone(),
            metrics,
            backend: VcetFixedCandidateWindowBackendProvenance {
                requested_backend: "CpuOnly".to_string(),
                realized_backend: "Cpu".to_string(),
                backend_route: "CpuOnly".to_string(),
                device_id: None,
                strict_backend: true,
                fallback_used: false,
                metric_contract: "full_cpu_metrics_v1".to_string(),
                unavailable_metric_fields: Vec::new(),
            },
            observed_trades,
            minimum_trades,
            minimum_trade_support_passed: observed_trades >= minimum_trades,
        };
        let request_sha256 = test_hash('4');
        let datasets = vec![dataset];
        let completed_state_sha256 = recompute_completed_state_sha256(
            &authorization.authorization_sha256,
            &request_sha256,
            &authorization.expected_prior_state_sha256,
            &sealed_universe.universe_sha256,
            &pre_final.payload.winner_lock.winner_lock_sha256,
            &authorization.shard,
            &datasets,
            authorization.shard.expected_candidate_window_trial_count,
            authorization.shard.expected_candidate_window_trial_count,
        )
        .expect("temporal-report fixture completed-state hash");
        let shard_payload = VcetTemporalFinalTestShardArtifactPayloadV1 {
            schema_version: VCET_TEMPORAL_FINAL_SHARD_ARTIFACT_SCHEMA_VERSION,
            evidence_kind: "vcet_temporal_final_test_shard_artifact".to_string(),
            universe_sha256: sealed_universe.universe_sha256.clone(),
            global_winner_reduction_artifact_sha256: pre_final.artifact_sha256.clone(),
            global_pre_final_evidence_sha256: pre_final
                .payload
                .global_pre_final_evidence_sha256
                .clone(),
            winner_lock: pre_final.payload.winner_lock.clone(),
            authorization_sha256: authorization.authorization_sha256.clone(),
            final_stage_nonce: pre_final.payload.final_stage_nonce.clone(),
            final_shard: authorization.shard.clone(),
            planning_evidence_sha256: test_hash('b'),
            execution_spec_sha256: pre_final.payload.execution_spec_sha256.clone(),
            execution_spec: universe.execution_spec.clone(),
            temporal_interval_fingerprint_sha256: universe
                .temporal_interval_fingerprint_sha256
                .clone(),
            scoring_semantics: final_scoring_semantics(universe.plan.position_boundary_policy),
            request_sha256,
            authorized_prior_state_sha256: authorization.expected_prior_state_sha256.clone(),
            completed_state_sha256,
            expected_candidate_window_trial_count: authorization
                .shard
                .expected_candidate_window_trial_count,
            evaluated_candidate_window_trial_count: authorization
                .shard
                .expected_candidate_window_trial_count,
            physical_shard_execution_count: 1,
            candidate_count: 1,
            datasets,
            support_status: VcetTemporalFinalSupportStatusV1::SupportPassed,
            minimum_trade_support_passed: true,
            selection_used_final_test: false,
            winner_reranked_after_final_test: false,
            limitations: final_shard_limitations(),
        };
        let shard = ExecuteVcetTemporalFinalTestShardResponseV1 {
            artifact_sha256: recompute_vcet_temporal_final_shard_artifact_sha256(&shard_payload)
                .expect("temporal-report fixture final shard hash"),
            payload: shard_payload,
        };
        validate_vcet_temporal_final_shard_artifact(&shard)
            .expect("validate temporal-report final shard fixture");

        let shard_evidence = vec![VcetTemporalFinalShardReductionEvidenceV1 {
            shard_ordinal: authorization.shard.shard_ordinal,
            shard_id: authorization.shard.shard_id.clone(),
            authorization_sha256: authorization.authorization_sha256.clone(),
            authorized_prior_state_sha256: authorization.expected_prior_state_sha256.clone(),
            completed_state_sha256: shard.payload.completed_state_sha256.clone(),
            artifact_sha256: shard.artifact_sha256.clone(),
            dataset_count: shard.payload.datasets.len(),
            candidate_window_trial_count: shard.payload.evaluated_candidate_window_trial_count,
            minimum_trade_support_passed: true,
        }];
        let shard_set_root_sha256 = recompute_final_shard_set_root_sha256(
            &sealed_universe.universe_sha256,
            &pre_final.payload.winner_lock.winner_lock_sha256,
            &pre_final.payload.final_stage_nonce,
            &shard_evidence,
        )
        .expect("temporal-report fixture final shard-set root");
        let payload = VcetTemporalFinalReductionArtifactPayloadV1 {
            schema_version: VCET_TEMPORAL_FINAL_REDUCTION_ARTIFACT_SCHEMA_VERSION,
            evidence_kind: "vcet_temporal_final_test_reduction_artifact".to_string(),
            universe_sha256: sealed_universe.universe_sha256.clone(),
            global_winner_reduction_artifact_sha256: pre_final.artifact_sha256.clone(),
            global_pre_final_evidence_sha256: pre_final
                .payload
                .global_pre_final_evidence_sha256
                .clone(),
            winner_lock: pre_final.payload.winner_lock.clone(),
            final_stage_nonce: pre_final.payload.final_stage_nonce.clone(),
            execution_spec_sha256: pre_final.payload.execution_spec_sha256.clone(),
            execution_spec: universe.execution_spec.clone(),
            temporal_interval_fingerprint_sha256: universe
                .temporal_interval_fingerprint_sha256
                .clone(),
            scoring_semantics: final_scoring_semantics(universe.plan.position_boundary_policy),
            logical_final_scoring_stage_open_count: 1,
            physical_final_shard_count: 1,
            final_test_candidate_count: 1,
            expected_final_test_dataset_count: shard.payload.datasets.len(),
            evaluated_final_test_dataset_count: shard.payload.datasets.len(),
            final_test_candidate_window_trial_count: shard.payload.datasets.len(),
            shard_set_root_sha256,
            shards: shard_evidence,
            datasets: shard.payload.datasets,
            support_status: VcetTemporalFinalSupportStatusV1::SupportPassed,
            minimum_trade_support_passed: true,
            selection_used_final_test: false,
            winner_reranked_after_final_test: false,
            limitations: final_reduction_limitations(),
        };
        let reduction = ReduceVcetTemporalFinalTestShardsResponseV1 {
            artifact_sha256: recompute_vcet_temporal_final_reduction_artifact_sha256(&payload)
                .expect("temporal-report fixture final reduction hash"),
            payload,
        };
        validate_vcet_temporal_final_reduction_artifact(&reduction)
            .expect("validate temporal-report final reduction fixture");
        reduction
    }

    fn rehash_reduction(response: &mut ReduceVcetTemporalFinalTestShardsResponseV1) {
        response.payload.shard_set_root_sha256 = recompute_final_shard_set_root_sha256(
            &response.payload.universe_sha256,
            &response.payload.winner_lock.winner_lock_sha256,
            &response.payload.final_stage_nonce,
            &response.payload.shards,
        )
        .expect("root");
        response.artifact_sha256 =
            recompute_vcet_temporal_final_reduction_artifact_sha256(&response.payload)
                .expect("hash");
    }

    #[test]
    fn one_shot_ledger_rejects_concurrency_different_retries_and_failure_reopen() {
        let authorization =
            "sha256:final-module-ledger-failure-test-000000000000000000000000000000";
        let request = "sha256:request-one";
        assert!(matches!(
            consume_final_authorization(authorization, request).expect("first admission"),
            LedgerAdmission::Execute
        ));
        assert!(consume_final_authorization(authorization, request)
            .unwrap_err()
            .contains("already executing"));
        assert!(
            consume_final_authorization(authorization, "sha256:request-two")
                .unwrap_err()
                .contains("different request")
        );
        fail_final_authorization(authorization, request, "synthetic failure")
            .expect("failure recorded");
        let error = consume_final_authorization(authorization, request).unwrap_err();
        assert!(error.contains("consumed"));
        assert!(error.contains("cannot be reopened"));
        assert!(error.contains("synthetic failure"));
    }

    #[test]
    fn logical_scoring_semantics_are_half_open_reset_flat_and_no_rerank() {
        let semantics =
            final_scoring_semantics(VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen);
        assert_eq!(
            semantics.scored_interval_semantics,
            "half_open_[start_index,end_index_exclusive)"
        );
        assert_eq!(
            semantics.execution_start_semantics,
            "reset_flat_at_scored_start"
        );
        assert_eq!(
            semantics.scored_end_semantics,
            "discard_open_position_without_forced_liquidation"
        );
        assert_eq!(
            semantics.trade_support_basis,
            "metrics.trades_transition_count_per_scored_window"
        );
    }

    #[test]
    fn completed_ledger_returns_identical_retry_and_rejects_rehashed_tamper() {
        let response = test_final_shard_response();
        validate_vcet_temporal_final_shard_artifact(&response).expect("valid shard artifact");
        let authorization = response.payload.authorization_sha256.clone();
        let request = response.payload.request_sha256.clone();
        assert!(matches!(
            consume_final_authorization(&authorization, &request).expect("consume"),
            LedgerAdmission::Execute
        ));
        complete_final_authorization(&authorization, &request, response.clone()).expect("complete");
        match consume_final_authorization(&authorization, &request).expect("cached retry") {
            LedgerAdmission::Cached(cached) => {
                assert_eq!(cached.artifact_sha256, response.artifact_sha256)
            }
            LedgerAdmission::Execute => panic!("completed authorization was reopened"),
        }
        assert!(consume_final_authorization(&authorization, &test_hash('0'))
            .unwrap_err()
            .contains("different request"));
        validate_completed_final_authorization_artifact(&authorization, &request, &response)
            .expect("ledger authority");

        let mut tampered = response;
        tampered.payload.datasets[0]
            .final_test
            .scored_bars
            .start_index = 2;
        rehash_shard(&mut tampered);
        validate_vcet_temporal_final_shard_artifact(&tampered)
            .expect("self-consistent but unauthorized tamper");
        assert!(validate_completed_final_authorization_artifact(
            &authorization,
            &request,
            &tampered
        )
        .unwrap_err()
        .contains("ledger-authoritative"));
    }

    #[test]
    fn standalone_shard_validator_rejects_empty_and_spec_winner_drift() {
        let valid = test_final_shard_response();
        validate_vcet_temporal_final_shard_artifact(&valid).expect("valid");

        let mut empty = valid.clone();
        empty.payload.final_shard.dataset_ids.clear();
        empty
            .payload
            .final_shard
            .expected_candidate_window_trial_count = 0;
        empty.payload.datasets.clear();
        empty.payload.expected_candidate_window_trial_count = 0;
        empty.payload.evaluated_candidate_window_trial_count = 0;
        rehash_shard(&mut empty);
        assert!(validate_vcet_temporal_final_shard_artifact(&empty).is_err());

        let mut spec_drift = valid;
        spec_drift.payload.execution_spec_sha256 = test_hash('0');
        rehash_shard(&mut spec_drift);
        assert!(validate_vcet_temporal_final_shard_artifact(&spec_drift).is_err());
    }

    #[test]
    fn standalone_reduction_validator_rejects_empty_trial_and_support_tamper() {
        let valid = test_final_reduction_response();
        validate_vcet_temporal_final_reduction_artifact(&valid).expect("valid reduction");

        let mut empty = valid.clone();
        empty.payload.shards.clear();
        empty.payload.datasets.clear();
        empty.payload.physical_final_shard_count = 0;
        empty.payload.expected_final_test_dataset_count = 0;
        empty.payload.evaluated_final_test_dataset_count = 0;
        empty.payload.final_test_candidate_window_trial_count = 0;
        rehash_reduction(&mut empty);
        assert!(validate_vcet_temporal_final_reduction_artifact(&empty).is_err());

        let mut trials = valid.clone();
        trials.payload.shards[0].candidate_window_trial_count = 2;
        rehash_reduction(&mut trials);
        assert!(validate_vcet_temporal_final_reduction_artifact(&trials).is_err());

        let mut support = valid;
        support.payload.shards[0].minimum_trade_support_passed = false;
        rehash_reduction(&mut support);
        assert!(validate_vcet_temporal_final_reduction_artifact(&support).is_err());
    }

    #[test]
    fn support_failed_final_evidence_is_terminal_and_never_reranks() {
        let mut failed = test_final_reduction_response();
        failed.payload.datasets[0].minimum_trades = 1;
        failed.payload.datasets[0].minimum_trade_support_passed = false;
        failed.payload.shards[0].minimum_trade_support_passed = false;
        failed.payload.support_status = VcetTemporalFinalSupportStatusV1::SupportFailed;
        failed.payload.minimum_trade_support_passed = false;
        rehash_reduction(&mut failed);
        validate_vcet_temporal_final_reduction_artifact(&failed)
            .expect("support failure remains valid final evidence");
        assert_eq!(failed.payload.logical_final_scoring_stage_open_count, 1);
        assert!(!failed.payload.selection_used_final_test);
        assert!(!failed.payload.winner_reranked_after_final_test);
    }

    fn authorization_for_final_response(
        response: &ExecuteVcetTemporalFinalTestShardResponseV1,
    ) -> VcetTemporalFinalShardAuthorizationV1 {
        VcetTemporalFinalShardAuthorizationV1 {
            shard: response.payload.final_shard.clone(),
            universe_sha256: response.payload.universe_sha256.clone(),
            winner_lock_sha256: response.payload.winner_lock.winner_lock_sha256.clone(),
            global_candidate_ordinal: response.payload.winner_lock.global_candidate_ordinal,
            candidate_id: response.payload.winner_lock.candidate_id.clone(),
            global_pre_final_evidence_sha256: response
                .payload
                .global_pre_final_evidence_sha256
                .clone(),
            final_stage_nonce: response.payload.final_stage_nonce.clone(),
            expected_prior_state_sha256: response.payload.authorized_prior_state_sha256.clone(),
            authorization_sha256: response.payload.authorization_sha256.clone(),
        }
    }

    #[test]
    fn authorization_cohort_is_exact_unique_and_canonically_ordered() {
        let response = test_final_shard_response();
        let first = authorization_for_final_response(&response);
        let mut second = first.clone();
        second.shard.shard_ordinal = 1;
        second.shard.shard_id = test_shard_id('4');
        second.shard.dataset_ids = vec!["FINAL-B".to_string()];
        second.expected_prior_state_sha256 = test_hash('7');
        second.authorization_sha256 = test_hash('8');
        let descriptors = vec![second.shard.clone(), first.shard.clone()];

        let ordered = order_exact_final_authorizations_with(
            &descriptors,
            &[second.clone(), first.clone()],
            |_authorization| Ok(()),
        )
        .expect("exact authorization cohort");
        assert_eq!(ordered[0].shard.shard_id, first.shard.shard_id);
        assert_eq!(ordered[1].shard.shard_id, second.shard.shard_id);

        let mut duplicate_authorization = second.clone();
        duplicate_authorization.authorization_sha256 = first.authorization_sha256.clone();
        assert!(order_exact_final_authorizations_with(
            &descriptors,
            &[first.clone(), duplicate_authorization],
            |_authorization| Ok(()),
        )
        .unwrap_err()
        .contains("duplicate"));

        let mut duplicate_prior = second.clone();
        duplicate_prior.expected_prior_state_sha256 = first.expected_prior_state_sha256.clone();
        assert!(order_exact_final_authorizations_with(
            &descriptors,
            &[first.clone(), duplicate_prior],
            |_authorization| Ok(()),
        )
        .unwrap_err()
        .contains("duplicate"));

        let mut duplicate_shard = second;
        duplicate_shard.shard = first.shard.clone();
        assert!(order_exact_final_authorizations_with(
            &descriptors,
            &[first, duplicate_shard],
            |_authorization| Ok(()),
        )
        .unwrap_err()
        .contains("duplicate"));
    }

    #[test]
    fn durable_fan_in_enumerates_authorizations_when_caller_receipts_are_empty() {
        let response = test_final_shard_response();
        let authorization = authorization_for_final_response(&response);
        let prepared = PreparedFinalReductionFanIn {
            ordered_authorizations: vec![authorization.clone()],
            supplied_by_shard: BTreeMap::new(),
        };
        let load_count = std::cell::Cell::new(0_usize);
        let loaded = load_expected_final_receipts_with(
            &prepared,
            |authorization_sha256| {
                load_count.set(load_count.get() + 1);
                assert_eq!(authorization_sha256, authorization.authorization_sha256);
                Ok((response.payload.request_sha256.clone(), response.clone()))
            },
            |expected, receipt| {
                assert_eq!(
                    expected.authorization_sha256,
                    receipt.payload.authorization_sha256
                );
                Ok(())
            },
        )
        .expect("authorization-driven durable fan-in");
        assert_eq!(load_count.get(), 1);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].response.artifact_sha256, response.artifact_sha256);
    }

    #[test]
    fn durable_fan_in_exact_compares_optional_caller_receipts() {
        let response = test_final_shard_response();
        let authorization = authorization_for_final_response(&response);
        let supplied_by_shard =
            BTreeMap::from([(authorization.shard.shard_id.clone(), response.clone())]);
        let prepared = PreparedFinalReductionFanIn {
            ordered_authorizations: vec![authorization.clone()],
            supplied_by_shard,
        };
        load_expected_final_receipts_with(
            &prepared,
            |_authorization_sha256| Ok((response.payload.request_sha256.clone(), response.clone())),
            |_expected, _receipt| Ok(()),
        )
        .expect("identical supplied receipt");

        let mut altered = response.clone();
        altered.payload.datasets[0].loaded_data_id = "different-loaded-final-a".to_string();
        rehash_shard(&mut altered);
        validate_vcet_temporal_final_shard_artifact(&altered)
            .expect("self-consistent altered receipt");
        let prepared = PreparedFinalReductionFanIn {
            ordered_authorizations: vec![authorization],
            supplied_by_shard: BTreeMap::from([(
                response.payload.final_shard.shard_id.clone(),
                altered,
            )]),
        };
        let error = load_expected_final_receipts_with(
            &prepared,
            |_authorization_sha256| Ok((response.payload.request_sha256.clone(), response.clone())),
            |_expected, _receipt| Ok(()),
        )
        .unwrap_err();
        assert!(error.contains("differs from its durable completed receipt"));
    }

    #[test]
    fn direct_durable_final_command_reports_live_claim_as_retryable_then_interrupted_after_drop() {
        let (sealed_universe, pre_final) = temporal_report_pre_final_fixture();
        let final_shard = universe_shards_for_role(
            &sealed_universe.payload,
            VcetTemporalDatasetCohortRoleV1::FinalTest,
        )
        .into_iter()
        .next()
        .expect("final shard")
        .clone();
        let final_dataset = sealed_universe
            .payload
            .datasets
            .iter()
            .find(|dataset| dataset.role == VcetTemporalDatasetCohortRoleV1::FinalTest)
            .expect("final dataset");
        let authorization = pre_final
            .payload
            .final_shard_authorizations
            .iter()
            .find(|authorization| authorization.shard.shard_id == final_shard.shard_id)
            .expect("final authorization");
        let planning_request = PlanVcetTemporalValidationRequestV1 {
            sealed_candidate_manifest: sealed_universe.payload.candidate_manifest.clone(),
            expected_candidate_manifest_sha256: sealed_universe
                .payload
                .candidate_manifest_sha256
                .clone(),
            plan: sealed_universe.payload.plan.clone(),
            expected_plan_identity: sealed_universe.payload.plan_identity.clone(),
            datasets: vec![VcetTemporalDatasetTimelineInputV1 {
                dataset_id: final_dataset.dataset_id.clone(),
                source_dataset_sha256: final_dataset.source_dataset_sha256.clone(),
                timestamps_utc_ms: (0_i64..=1_000).map(|index| index * 1_000).collect(),
                session_calendar_id: None,
                session_open_utc_ms: None,
            }],
        };
        let planning_evidence = plan_vcet_temporal_validation_blocking(planning_request.clone())
            .expect("canonical final planning evidence");
        let request = ExecuteVcetTemporalFinalTestShardRequestV1 {
            sealed_universe: sealed_universe.clone(),
            expected_universe_sha256: sealed_universe.universe_sha256.clone(),
            global_winner_reduction: pre_final.clone(),
            expected_global_winner_reduction_artifact_sha256: pre_final.artifact_sha256.clone(),
            final_shard,
            planning_request,
            expected_planning_evidence_sha256: planning_evidence.evidence_sha256.clone(),
            planning_evidence,
            loaded_datasets: vec![VcetTemporalLoadedDatasetBindingV1 {
                dataset_id: final_dataset.dataset_id.clone(),
                loaded_data_id: "loaded-final-dataset-live-claim".to_string(),
            }],
            expected_authorized_prior_state_sha256: authorization
                .expected_prior_state_sha256
                .clone(),
        };
        let sequence = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vectorgrid_vcet_final_live_claim_{}_{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create durable final test directory");

        let winner_claim = match admit_durable_pre_final_winner_lock::<
            ReduceVcetTemporalPreFinalShardsResponseV1,
        >(&root, &sealed_universe.universe_sha256, &test_hash('0'))
        .expect("admit durable winner lock")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("fresh winner claim cannot replay"),
        };
        winner_claim
            .complete(&pre_final)
            .expect("publish durable winner lock");

        let (validated_authorization, request_sha256) =
            prevalidate_before_final_data_access(&request).expect("prevalidate final request");
        let claim =
            match admit_durable_final_authorization::<ExecuteVcetTemporalFinalTestShardResponseV1>(
                &root,
                &validated_authorization.authorization_sha256,
                &request_sha256,
            )
            .expect("seed live direct final-command claim")
            {
                DurableFinalAdmission::Acquired(claim) => claim,
                DurableFinalAdmission::Replay(_) => panic!("fresh final claim cannot replay"),
            };

        let live_error = execute_vcet_temporal_final_test_shard_durable_blocking_with_loader(
            request.clone(),
            &root,
            |_| Err("final data loader unexpectedly ran".to_string()),
        )
        .expect_err("same-process live final command must be retryable");
        assert!(
            live_error.contains("temporarily unavailable"),
            "{live_error}"
        );
        assert!(
            live_error.contains("same-process execution still active"),
            "{live_error}"
        );
        assert!(!live_error.contains("interrupted"), "{live_error}");

        drop(claim);
        let interrupted_error =
            execute_vcet_temporal_final_test_shard_durable_blocking_with_loader(
                request,
                &root,
                |_| Err("final data loader unexpectedly ran".to_string()),
            )
            .expect_err("dropped unfinished final command must remain fail-closed");
        assert!(
            interrupted_error.contains("uncompleted durable claim"),
            "{interrupted_error}"
        );
        assert!(
            interrupted_error.contains("still active or crashed"),
            "{interrupted_error}"
        );
    }

    #[test]
    fn limitations_distinguish_process_restart_power_loss_and_adversarial_writes() {
        for limitations in [final_shard_limitations(), final_reduction_limitations()] {
            assert!(limitations
                .iter()
                .any(|item| item.contains("process_restart")));
            assert!(limitations.iter().any(|item| item.contains("power_loss")));
            assert!(limitations
                .iter()
                .any(|item| item.contains("signatures_or_macs")));
            assert!(limitations
                .iter()
                .any(|item| item.contains("cross_machine")));
        }
    }
}

#[cfg(test)]
pub(crate) use tests::temporal_report_final_fixture;
