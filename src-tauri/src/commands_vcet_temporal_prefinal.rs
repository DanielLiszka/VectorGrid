//! VCET temporal pre-final shard execution and deterministic global reduction.

use super::commands_vcet_temporal::{
    plan_vcet_temporal_validation_blocking, PlanVcetTemporalValidationRequestV1,
    PlanVcetTemporalValidationResponseV1,
};
use super::commands_vcet_temporal_execute::{
    candidate_params_from_manifest, evaluate_window, loaded_candles_snapshot_sha256,
    profile_loaded_dataset_coverage_for_planning, sha256_serializable,
    validate_loaded_dataset_shape, VcetTemporalLoadedDatasetBindingV1,
    VcetTemporalWindowExecutionV1,
};
use super::commands_vcet_temporal_universe::{
    recompute_vcet_dataset_coverage_sha256, recompute_vcet_temporal_interval_fingerprint_sha256,
    universe_entry_map, universe_shards_for_role, validate_sealed_vcet_temporal_universe,
    VcetTemporalCrossUniverseAdmissionV1, VcetTemporalDatasetCohortRoleV1,
    VcetTemporalPhasedExecutionSpecV1, VcetTemporalSealedDatasetUniverseV1,
    VcetTemporalUniverseShardV1,
};
use super::vcet_temporal_durable_ledger::{
    admit_durable_pre_final_winner_lock, load_durable_pre_final_winner_lock, DurableFinalAdmission,
    DurableFinalClaim,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use ta_app_core::composite_validate::{CompositeParams, VcetFixedCandidateWindowBackendProvenance};
use ta_app_core::double_ma::Backend;
use ta_app_core::vcet_dataset_coverage::VcetDatasetCoverageEvidence;
use tauri::Manager;
use vector_ta::utilities::data_loader::Candles;

pub const VCET_TEMPORAL_PRE_FINAL_SHARD_ARTIFACT_SCHEMA_VERSION: u16 = 1;
pub const VCET_TEMPORAL_PRE_FINAL_REDUCTION_ARTIFACT_SCHEMA_VERSION: u16 = 1;
pub const MAX_VCET_TEMPORAL_INLINE_PRE_FINAL_SHARD_FAN_IN: usize = 4_096;
const MAX_SUPPORT_FAILURE_EXAMPLES_PER_DATASET_CANDIDATE: usize = 16;

const PRE_FINAL_SHARD_ARTIFACT_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_pre_final_shard_artifact.v1";
const PRE_FINAL_GLOBAL_EVIDENCE_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_pre_final_global_evidence.v1";
const PRE_FINAL_REDUCTION_ARTIFACT_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_pre_final_reduction_artifact.v1";
const PRE_FINAL_REDUCTION_REQUEST_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_pre_final_reduction_request.v1";
const GLOBAL_WINNER_LOCK_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_global_winner_lock.v1";
const FINAL_SHARD_PRIOR_STATE_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_final_shard_authorized_prior_state.v1";
const FINAL_SHARD_AUTHORIZATION_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_final_shard_authorization.v1";
const EXECUTION_SPEC_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_execution_spec.v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteVcetTemporalPreFinalShardRequestV1 {
    pub sealed_universe: VcetTemporalSealedDatasetUniverseV1,
    pub expected_universe_sha256: String,
    pub selection_shard: VcetTemporalUniverseShardV1,
    pub planning_request: PlanVcetTemporalValidationRequestV1,
    pub planning_evidence: PlanVcetTemporalValidationResponseV1,
    pub expected_planning_evidence_sha256: String,
    pub loaded_datasets: Vec<VcetTemporalLoadedDatasetBindingV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalPreFinalSupportFailureV1 {
    pub dataset_id: String,
    pub fold_index: usize,
    pub stage: String,
    pub observed_trades: u64,
    pub minimum_trades: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalPreFinalDatasetCandidateEvidenceV1 {
    pub global_candidate_ordinal: usize,
    pub candidate_id: String,
    pub validation_fold_count: usize,
    pub finite_validation_observation_count: usize,
    pub validation_pnl_sum: f64,
    pub validation_sharpe_sum: f64,
    pub mean_validation_pnl: Option<f64>,
    pub mean_validation_sharpe: Option<f64>,
    pub total_train_trades: u64,
    pub total_validation_trades: u64,
    pub support_failure_count: usize,
    pub support_failure_examples: Vec<VcetTemporalPreFinalSupportFailureV1>,
    pub non_finite_metric_count: usize,
    pub hard_support_passed: bool,
    pub metric_support_passed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalPreFinalDatasetEvidenceV1 {
    pub dataset_id: String,
    pub loaded_data_id: String,
    pub source_dataset_sha256: String,
    pub dataset_attestation_sha256: String,
    pub timeline_sha256: String,
    pub loaded_candles_snapshot_sha256: String,
    pub dataset_coverage: VcetDatasetCoverageEvidence,
    pub dataset_coverage_sha256: String,
    pub temporal_interval_fingerprint_sha256: String,
    pub fold_count: usize,
    pub candidates: Vec<VcetTemporalPreFinalDatasetCandidateEvidenceV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalPreFinalShardCandidateAggregateV1 {
    pub global_candidate_ordinal: usize,
    pub candidate_id: String,
    pub params: CompositeParams,
    pub validation_sharpe_sum: f64,
    pub validation_observation_count: usize,
    pub total_train_trades: u64,
    pub total_validation_trades: u64,
    pub support_failure_count: usize,
    pub non_finite_metric_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalPreFinalShardArtifactPayloadV1 {
    pub schema_version: u16,
    pub evidence_kind: String,
    pub universe_sha256: String,
    pub candidate_manifest_sha256: String,
    pub plan_sha256: String,
    pub execution_spec_sha256: String,
    pub execution_spec: VcetTemporalPhasedExecutionSpecV1,
    pub temporal_interval_fingerprint_sha256: String,
    pub selection_shard: VcetTemporalUniverseShardV1,
    pub planning_evidence_sha256: String,
    pub dataset_count: usize,
    pub candidate_count: usize,
    pub fold_count_per_dataset: usize,
    pub expected_candidate_window_trial_count: usize,
    pub evaluated_candidate_window_trial_count: usize,
    pub final_test_scoring_access_count: usize,
    pub candidates: Vec<VcetTemporalPreFinalShardCandidateAggregateV1>,
    pub datasets: Vec<VcetTemporalPreFinalDatasetEvidenceV1>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteVcetTemporalPreFinalShardResponseV1 {
    pub payload: VcetTemporalPreFinalShardArtifactPayloadV1,
    pub artifact_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReduceVcetTemporalPreFinalShardsRequestV1 {
    pub sealed_universe: VcetTemporalSealedDatasetUniverseV1,
    pub expected_universe_sha256: String,
    pub final_stage_nonce: String,
    pub pre_final_shard_artifacts: Vec<ExecuteVcetTemporalPreFinalShardResponseV1>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadVcetTemporalPreFinalWinnerLockRequestV1 {
    pub universe_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalGlobalCandidateEvidenceV1 {
    pub global_candidate_ordinal: usize,
    pub candidate_id: String,
    pub params: CompositeParams,
    pub requested_dataset_count: usize,
    pub covered_dataset_count: usize,
    pub evaluated_dataset_count: usize,
    pub profitable_dataset_count: usize,
    pub hard_support_dataset_count: usize,
    pub metric_support_dataset_count: usize,
    pub requested_dataset_coverage_share: f64,
    pub evaluated_dataset_share_requested: f64,
    pub profitable_dataset_share_requested: f64,
    pub hard_support_dataset_share_requested: f64,
    pub metric_support_dataset_share_requested: f64,
    pub validation_pnl_p10: Option<f64>,
    pub validation_sharpe_p10: Option<f64>,
    pub validation_sharpe_sum: f64,
    pub mean_validation_sharpe: Option<f64>,
    pub validation_observation_count: usize,
    pub total_train_trades: u64,
    pub total_validation_trades: u64,
    pub support_failure_count: usize,
    pub non_finite_metric_count: usize,
    pub admission_passed: bool,
    pub selected_before_final_test: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalGlobalWinnerLockV1 {
    pub global_candidate_ordinal: usize,
    pub candidate_id: String,
    pub params: CompositeParams,
    pub selection_policy: super::commands_vcet_temporal_execute::VcetTemporalSelectionPolicyV1,
    pub selection_score: f64,
    pub runner_up_candidate_id: Option<String>,
    pub runner_up_selection_score: Option<f64>,
    pub winner_to_runner_up_score_margin: Option<f64>,
    pub candidate_id_tie_break_used: bool,
    pub selection_reduction_contract: String,
    pub universe_sha256: String,
    pub candidate_manifest_sha256: String,
    pub plan_sha256: String,
    pub execution_spec_sha256: String,
    pub global_pre_final_evidence_sha256: String,
    pub winner_lock_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalFinalShardAuthorizationV1 {
    pub shard: VcetTemporalUniverseShardV1,
    pub universe_sha256: String,
    pub winner_lock_sha256: String,
    pub global_candidate_ordinal: usize,
    pub candidate_id: String,
    pub global_pre_final_evidence_sha256: String,
    pub final_stage_nonce: String,
    pub expected_prior_state_sha256: String,
    pub authorization_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalPreFinalShardReductionEvidenceV1 {
    pub shard_ordinal: usize,
    pub shard_id: String,
    pub artifact_sha256: String,
    pub dataset_count: usize,
    pub candidate_count: usize,
    pub candidate_window_trial_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalPreFinalReductionArtifactPayloadV1 {
    pub schema_version: u16,
    pub evidence_kind: String,
    pub universe_sha256: String,
    pub candidate_manifest_sha256: String,
    pub plan_sha256: String,
    pub execution_spec_sha256: String,
    pub execution_spec: VcetTemporalPhasedExecutionSpecV1,
    pub temporal_interval_fingerprint_sha256: String,
    pub selection_reduction_contract: String,
    pub global_pre_final_evidence_sha256: String,
    pub requested_selection_dataset_count: usize,
    pub covered_selection_dataset_count: usize,
    pub expected_selection_shard_count: usize,
    pub reduced_selection_shard_count: usize,
    pub pre_final_candidate_window_trial_count: usize,
    pub final_test_scoring_access_count: usize,
    pub ordered_shards: Vec<VcetTemporalPreFinalShardReductionEvidenceV1>,
    pub candidates: Vec<VcetTemporalGlobalCandidateEvidenceV1>,
    pub winner_lock: VcetTemporalGlobalWinnerLockV1,
    pub final_stage_nonce: String,
    pub final_shard_authorizations: Vec<VcetTemporalFinalShardAuthorizationV1>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReduceVcetTemporalPreFinalShardsResponseV1 {
    pub payload: VcetTemporalPreFinalReductionArtifactPayloadV1,
    pub artifact_sha256: String,
}

#[derive(Default)]
struct DatasetCandidateAccumulator {
    pnl_sum: f64,
    sharpe_sum: f64,
    finite_count: usize,
    validation_fold_count: usize,
    total_train_trades: u64,
    total_validation_trades: u64,
    support_failure_count: usize,
    support_failure_examples: Vec<VcetTemporalPreFinalSupportFailureV1>,
    non_finite_metric_count: usize,
}

struct LoadedPreFinalDataset {
    dataset_id: String,
    loaded_data_id: String,
    candles: Arc<Candles>,
}

#[derive(Serialize)]
struct HashEnvelope<'a, T> {
    hash_contract: &'static str,
    value: &'a T,
}

#[derive(Serialize)]
struct WinnerLockHashInput<'a> {
    hash_contract: &'static str,
    global_candidate_ordinal: usize,
    candidate_id: &'a str,
    params: &'a CompositeParams,
    selection_score_bits: u64,
    runner_up_candidate_id: &'a Option<String>,
    runner_up_selection_score_bits: Option<u64>,
    winner_to_runner_up_score_margin_bits: Option<u64>,
    candidate_id_tie_break_used: bool,
    selection_reduction_contract: &'a str,
    universe_sha256: &'a str,
    candidate_manifest_sha256: &'a str,
    plan_sha256: &'a str,
    execution_spec_sha256: &'a str,
    global_pre_final_evidence_sha256: &'a str,
}

#[derive(Serialize)]
struct FinalPriorStateHashInput<'a> {
    hash_contract: &'static str,
    universe_sha256: &'a str,
    winner_lock_sha256: &'a str,
    global_pre_final_evidence_sha256: &'a str,
    final_stage_nonce: &'a str,
    shard: &'a VcetTemporalUniverseShardV1,
    state: &'static str,
}

fn hash_enveloped<T: Serialize>(
    contract: &'static str,
    value: &T,
    label: &str,
) -> Result<String, String> {
    sha256_serializable(
        &HashEnvelope {
            hash_contract: contract,
            value,
        },
        label,
    )
}

fn serialized_eq(left: &impl Serialize, right: &impl Serialize) -> Result<bool, String> {
    let left = serde_json::to_vec(left)
        .map_err(|error| format!("failed to serialize canonical left value: {error}"))?;
    let right = serde_json::to_vec(right)
        .map_err(|error| format!("failed to serialize canonical right value: {error}"))?;
    Ok(left == right)
}

fn execution_spec_sha256(spec: &VcetTemporalPhasedExecutionSpecV1) -> Result<String, String> {
    hash_enveloped(
        EXECUTION_SPEC_HASH_CONTRACT,
        spec,
        "VCET phased execution spec",
    )
}

fn pre_final_shard_limitations() -> Vec<String> {
    vec![
        "pre_final_shard_scores_train_and_validation_only_and_never_selects_a_local_winner"
            .to_string(),
        "final_test_scoring_access_count_is_zero_but_loaded_snapshot_attestation_reads_the_sealed_full_candle_object"
            .to_string(),
        "gpu_trade_distribution_fields_listed_in_backend_provenance_are_unavailable_and_are_not_used_for_selection_or_support"
            .to_string(),
    ]
}

fn pre_final_reduction_limitations() -> Vec<String> {
    vec![
        "selection_uses_the_sealed_backend_and_canonical_shard_ordinal_reduction_order_without_claiming_cpu_gpu_winner_equivalence_for_near_ties"
            .to_string(),
        "missing_or_non_finite_dataset_metrics_remain_in_the_requested_denominator_and_lower_tail_mass"
            .to_string(),
        "the_desktop_reducer_uses_a_staged_synced_no_clobber_hard_link_universe_claim_and_one_immutable_discriminated_terminal_to_coordinate_cooperating_local_processes_and_remain_fail_closed_after_process_crash_while_the_ledger_is_intact_while_the_direct_blocking_helper_remains_process_local"
            .to_string(),
        "the_local_ledger_attempts_best_effort_parent_namespace_sync_but_does_not_guarantee_survival_of_sudden_power_loss_filesystem_rollback_storage_loss_or_storage_corruption"
            .to_string(),
        "sha256_receipt_hashes_are_unkeyed_integrity_checks_not_signatures_or_macs_so_adversarial_local_modification_is_outside_the_ledger_authority"
            .to_string(),
        format!(
            "inline_v1_reduction_is_bounded_to_{MAX_VCET_TEMPORAL_INLINE_PRE_FINAL_SHARD_FAN_IN}_shards_and_requires_app_owned_persisted_hierarchical_or_streaming_reduction_beyond_that_fan_in"
        ),
    ]
}

fn validate_strict_backend_provenance(
    backend: &VcetFixedCandidateWindowBackendProvenance,
    expected: &Backend,
) -> Result<(), String> {
    if !backend.strict_backend || backend.fallback_used {
        return Err("VCET pre-final shard backend was not strict or used fallback".to_string());
    }
    match expected {
        Backend::CpuOnly => {
            if backend.requested_backend != "CpuOnly"
                || backend.realized_backend != "Cpu"
                || backend.backend_route != "CpuOnly"
                || backend.device_id.is_some()
                || backend.metric_contract != "full_cpu_metrics_v1"
                || !backend.unavailable_metric_fields.is_empty()
            {
                return Err(
                    "VCET pre-final CPU provenance does not match the strict CpuOnly contract"
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
                    "VCET pre-final GPU provenance does not match the strict GpuKernel contract"
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
                    "VCET pre-final GPU unavailable-metric contract is not canonical".to_string(),
                );
            }
        }
        _ => {
            return Err("VCET pre-final execution requires CpuOnly or strict GpuKernel".to_string())
        }
    }
    Ok(())
}

fn exact_selection_descriptor(
    universe: &VcetTemporalSealedDatasetUniverseV1,
    supplied: &VcetTemporalUniverseShardV1,
) -> Result<VcetTemporalUniverseShardV1, String> {
    let matches = universe_shards_for_role(
        &universe.payload,
        VcetTemporalDatasetCohortRoleV1::Selection,
    )
    .into_iter()
    .filter(|shard| shard.shard_id == supplied.shard_id)
    .collect::<Vec<_>>();
    if matches.len() != 1 || !serialized_eq(matches[0], supplied)? {
        return Err(format!(
            "VCET pre-final shard '{}' is not an exact canonical selection descriptor",
            supplied.shard_id
        ));
    }
    let descriptor = matches[0].clone();
    if descriptor.locked_winner_only
        || descriptor.candidate_start_index >= descriptor.candidate_end_index_exclusive
        || descriptor.dataset_ids.is_empty()
    {
        return Err("VCET pre-final selection descriptor is empty or winner-only".to_string());
    }
    Ok(descriptor)
}

fn resolve_pre_final_candles(
    bindings: &[VcetTemporalLoadedDatasetBindingV1],
    state: &AppState,
) -> Result<Vec<LoadedPreFinalDataset>, String> {
    let mut seen_dataset_ids = BTreeSet::new();
    let mut seen_loaded_ids = BTreeSet::new();
    let mut loaded = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let dataset_id = binding.dataset_id.trim();
        let loaded_data_id = binding.loaded_data_id.trim();
        if dataset_id.is_empty()
            || loaded_data_id.is_empty()
            || dataset_id != binding.dataset_id
            || loaded_data_id != binding.loaded_data_id
        {
            return Err(
                "VCET pre-final loaded bindings require canonical non-empty IDs".to_string(),
            );
        }
        if !seen_dataset_ids.insert(dataset_id.to_string()) {
            return Err(format!(
                "duplicate VCET pre-final dataset binding '{dataset_id}'"
            ));
        }
        if !seen_loaded_ids.insert(loaded_data_id.to_string()) {
            return Err(format!(
                "VCET pre-final loaded_data_id '{loaded_data_id}' is reused"
            ));
        }
        let candles = state.get_candles_for_run(loaded_data_id).map_err(|error| {
            format!(
                "failed to resolve loaded candles '{loaded_data_id}' for VCET dataset '{dataset_id}': {error}"
            )
        })?;
        loaded.push(LoadedPreFinalDataset {
            dataset_id: dataset_id.to_string(),
            loaded_data_id: loaded_data_id.to_string(),
            candles,
        });
    }
    Ok(loaded)
}

fn push_support_failure(
    accumulator: &mut DatasetCandidateAccumulator,
    failure: VcetTemporalPreFinalSupportFailureV1,
) {
    accumulator.support_failure_count = accumulator.support_failure_count.saturating_add(1);
    if accumulator.support_failure_examples.len()
        < MAX_SUPPORT_FAILURE_EXAMPLES_PER_DATASET_CANDIDATE
    {
        accumulator.support_failure_examples.push(failure);
    }
}

fn update_dataset_candidate_accumulators(
    accumulators: &mut [DatasetCandidateAccumulator],
    dataset_id: &str,
    fold_index: usize,
    train: &VcetTemporalWindowExecutionV1,
    validation: &VcetTemporalWindowExecutionV1,
    minimum_train_trades: u64,
    minimum_validation_trades: u64,
) -> Result<(), String> {
    if train.candidates.len() != accumulators.len()
        || validation.candidates.len() != accumulators.len()
    {
        return Err("VCET pre-final fold candidate count mismatch".to_string());
    }
    for local_index in 0..accumulators.len() {
        let train_candidate = &train.candidates[local_index];
        let validation_candidate = &validation.candidates[local_index];
        if train_candidate.candidate_index != local_index
            || validation_candidate.candidate_index != local_index
            || train_candidate.candidate_id != validation_candidate.candidate_id
        {
            return Err(format!(
                "VCET pre-final local candidate ordering mismatch at {local_index}"
            ));
        }
        let accumulator = &mut accumulators[local_index];
        accumulator.validation_fold_count = accumulator.validation_fold_count.saturating_add(1);
        let train_trades = u64::from(train_candidate.metrics.trades);
        let validation_trades = u64::from(validation_candidate.metrics.trades);
        accumulator.total_train_trades =
            accumulator
                .total_train_trades
                .checked_add(train_trades)
                .ok_or_else(|| "VCET pre-final train trade count overflow".to_string())?;
        accumulator.total_validation_trades = accumulator
            .total_validation_trades
            .checked_add(validation_trades)
            .ok_or_else(|| "VCET pre-final validation trade count overflow".to_string())?;
        if train_trades < minimum_train_trades {
            push_support_failure(
                accumulator,
                VcetTemporalPreFinalSupportFailureV1 {
                    dataset_id: dataset_id.to_string(),
                    fold_index,
                    stage: "train".to_string(),
                    observed_trades: train_trades,
                    minimum_trades: minimum_train_trades,
                },
            );
        }
        if validation_trades < minimum_validation_trades {
            push_support_failure(
                accumulator,
                VcetTemporalPreFinalSupportFailureV1 {
                    dataset_id: dataset_id.to_string(),
                    fold_index,
                    stage: "validation".to_string(),
                    observed_trades: validation_trades,
                    minimum_trades: minimum_validation_trades,
                },
            );
        }
        if validation_candidate.metrics.pnl.is_finite()
            && validation_candidate.metrics.sharpe.is_finite()
        {
            accumulator.pnl_sum += validation_candidate.metrics.pnl;
            accumulator.sharpe_sum += validation_candidate.metrics.sharpe;
            accumulator.finite_count = accumulator.finite_count.saturating_add(1);
        } else {
            accumulator.non_finite_metric_count =
                accumulator.non_finite_metric_count.saturating_add(1);
        }
    }
    Ok(())
}

/// Derive redundant mean evidence from the exact f64 representation that is
/// stored alongside it. Keeping this as one non-inlined integer-bit seam
/// prevents producer and validator codegen contexts from disagreeing by an ULP
/// while preserving exact, fail-closed artifact validation.
#[inline(never)]
fn canonical_mean_from_sum_bits(sum_bits: u64, count: usize) -> Option<f64> {
    (count > 0).then(|| f64::from_bits(sum_bits) / count as f64)
}

fn dataset_candidate_evidence(
    global_candidate_ordinal: usize,
    candidate_id: String,
    accumulator: DatasetCandidateAccumulator,
) -> VcetTemporalPreFinalDatasetCandidateEvidenceV1 {
    let pnl_sum_bits = accumulator.pnl_sum.to_bits();
    let sharpe_sum_bits = accumulator.sharpe_sum.to_bits();
    let mean_validation_pnl = canonical_mean_from_sum_bits(pnl_sum_bits, accumulator.finite_count);
    let mean_validation_sharpe =
        canonical_mean_from_sum_bits(sharpe_sum_bits, accumulator.finite_count);
    let hard_support_passed = accumulator.support_failure_count == 0;
    let metric_support_passed = accumulator.non_finite_metric_count == 0
        && accumulator.finite_count == accumulator.validation_fold_count
        && accumulator.validation_fold_count > 0;
    VcetTemporalPreFinalDatasetCandidateEvidenceV1 {
        global_candidate_ordinal,
        candidate_id,
        validation_fold_count: accumulator.validation_fold_count,
        finite_validation_observation_count: accumulator.finite_count,
        validation_pnl_sum: f64::from_bits(pnl_sum_bits),
        validation_sharpe_sum: f64::from_bits(sharpe_sum_bits),
        mean_validation_pnl,
        mean_validation_sharpe,
        total_train_trades: accumulator.total_train_trades,
        total_validation_trades: accumulator.total_validation_trades,
        support_failure_count: accumulator.support_failure_count,
        support_failure_examples: accumulator.support_failure_examples,
        non_finite_metric_count: accumulator.non_finite_metric_count,
        hard_support_passed,
        metric_support_passed,
    }
}

fn execute_vcet_temporal_pre_final_shard_blocking_with_resolved_candles(
    req: ExecuteVcetTemporalPreFinalShardRequestV1,
    loaded: Vec<LoadedPreFinalDataset>,
) -> Result<ExecuteVcetTemporalPreFinalShardResponseV1, String> {
    validate_sealed_vcet_temporal_universe(&req.sealed_universe, &req.expected_universe_sha256)?;
    let descriptor = exact_selection_descriptor(&req.sealed_universe, &req.selection_shard)?;
    let universe = &req.sealed_universe.payload;
    if !serialized_eq(
        &req.planning_request.sealed_candidate_manifest,
        &universe.candidate_manifest,
    )? || req.planning_request.expected_candidate_manifest_sha256
        != universe.candidate_manifest_sha256
        || !serialized_eq(&req.planning_request.plan, &universe.plan)?
        || req.planning_request.expected_plan_identity != universe.plan_identity
    {
        return Err(
            "VCET pre-final planning request does not match the sealed universe manifest and plan"
                .to_string(),
        );
    }
    if req.planning_request.datasets.len() != descriptor.dataset_ids.len()
        || req.loaded_datasets.len() != descriptor.dataset_ids.len()
        || loaded.len() != descriptor.dataset_ids.len()
    {
        return Err(
            "VCET pre-final timelines and loaded bindings must exactly cover the descriptor"
                .to_string(),
        );
    }
    let expected_ids = descriptor
        .dataset_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if expected_ids.len() != descriptor.dataset_ids.len() {
        return Err("VCET pre-final descriptor contains duplicate dataset IDs".to_string());
    }
    let universe_entries = universe_entry_map(universe)?;
    let mut timeline_by_id = BTreeMap::new();
    for timeline in &req.planning_request.datasets {
        if timeline.dataset_id.trim() != timeline.dataset_id
            || !expected_ids.contains(&timeline.dataset_id)
            || timeline_by_id
                .insert(timeline.dataset_id.clone(), timeline)
                .is_some()
        {
            return Err(
                "VCET pre-final planning request must contain each descriptor dataset exactly once"
                    .to_string(),
            );
        }
        let entry = universe_entries.get(&timeline.dataset_id).ok_or_else(|| {
            format!(
                "VCET pre-final dataset '{}' is absent from the universe",
                timeline.dataset_id
            )
        })?;
        if entry.role != VcetTemporalDatasetCohortRoleV1::Selection
            || entry.source_dataset_sha256 != timeline.source_dataset_sha256
        {
            return Err(format!(
                "VCET pre-final dataset '{}' role or source lineage differs from the universe",
                timeline.dataset_id
            ));
        }
    }
    let canonical_planning = plan_vcet_temporal_validation_blocking(req.planning_request.clone())?;
    if canonical_planning != req.planning_evidence
        || canonical_planning.evidence_sha256 != req.expected_planning_evidence_sha256
    {
        return Err(
            "VCET pre-final planning evidence is not the exact canonical result for the descriptor timelines"
                .to_string(),
        );
    }
    if canonical_planning.datasets.len() != descriptor.dataset_ids.len()
        || canonical_planning
            .datasets
            .iter()
            .any(|dataset| dataset.fold_count != universe.fold_count_per_dataset)
    {
        return Err("VCET pre-final planning fold count differs from the universe".to_string());
    }

    let candidate_entries = &universe.candidate_manifest.payload.candidates
        [descriptor.candidate_start_index..descriptor.candidate_end_index_exclusive];
    let candidate_ids = candidate_entries
        .iter()
        .map(|candidate| candidate.candidate_id.clone())
        .collect::<Vec<_>>();
    let candidate_params = candidate_entries
        .iter()
        .map(|candidate| {
            candidate_params_from_manifest(
                &candidate.identity_inputs,
                &universe.execution_spec.strategy,
            )
        })
        .collect::<Vec<_>>();
    let expected_trials = descriptor
        .dataset_ids
        .len()
        .checked_mul(candidate_params.len())
        .and_then(|value| value.checked_mul(universe.fold_count_per_dataset))
        .and_then(|value| value.checked_mul(2))
        .ok_or_else(|| "VCET pre-final trial count overflow".to_string())?;
    if expected_trials != descriptor.expected_candidate_window_trial_count {
        return Err("VCET pre-final descriptor trial count is inconsistent".to_string());
    }

    let mut binding_by_dataset = BTreeMap::new();
    let mut loaded_ids = BTreeSet::new();
    for binding in &req.loaded_datasets {
        if binding.dataset_id.trim() != binding.dataset_id
            || binding.loaded_data_id.trim() != binding.loaded_data_id
            || binding.dataset_id.is_empty()
            || binding.loaded_data_id.is_empty()
            || !expected_ids.contains(&binding.dataset_id)
            || binding_by_dataset
                .insert(binding.dataset_id.clone(), binding.clone())
                .is_some()
            || !loaded_ids.insert(binding.loaded_data_id.clone())
        {
            return Err(
                "VCET pre-final loaded bindings must be canonical, unique, and exact".to_string(),
            );
        }
    }
    let mut loaded_by_dataset = BTreeMap::new();
    for item in loaded {
        if loaded_by_dataset
            .insert(item.dataset_id.clone(), item)
            .is_some()
        {
            return Err("duplicate resolved VCET pre-final dataset".to_string());
        }
    }
    let planned_by_id = canonical_planning
        .datasets
        .iter()
        .map(|dataset| (dataset.dataset_id.clone(), dataset))
        .collect::<BTreeMap<_, _>>();
    let minimum_support = &universe.plan.minimum_support;
    let mut datasets = Vec::with_capacity(descriptor.dataset_ids.len());

    for dataset_id in &descriptor.dataset_ids {
        let binding = binding_by_dataset
            .get(dataset_id)
            .ok_or_else(|| format!("missing VCET pre-final binding for dataset '{dataset_id}'"))?;
        let resolved = loaded_by_dataset
            .remove(dataset_id)
            .ok_or_else(|| format!("missing resolved VCET pre-final dataset '{dataset_id}'"))?;
        if resolved.loaded_data_id != binding.loaded_data_id {
            return Err(format!(
                "VCET pre-final loaded ID mismatch for dataset '{dataset_id}'"
            ));
        }
        let entry = (*universe_entries
            .get(dataset_id)
            .ok_or_else(|| format!("missing universe entry for VCET dataset '{dataset_id}'"))?)
        .clone();
        let timeline = timeline_by_id
            .get(dataset_id)
            .ok_or_else(|| format!("missing VCET pre-final timeline for '{dataset_id}'"))?;
        let planned = planned_by_id.get(dataset_id).ok_or_else(|| {
            format!("missing VCET pre-final planning evidence for '{dataset_id}'")
        })?;
        validate_loaded_dataset_shape(dataset_id, &resolved.candles)?;
        if resolved.candles.timestamp != timeline.timestamps_utc_ms {
            return Err(format!(
                "VCET pre-final timestamps differ from the sealed timeline for '{dataset_id}'"
            ));
        }
        let snapshot_sha256 = loaded_candles_snapshot_sha256(&resolved.candles);
        if snapshot_sha256 != entry.loaded_candles_snapshot_sha256
            || planned.source_dataset_sha256 != entry.source_dataset_sha256
            || planned.timeline_sha256 != entry.timeline_sha256
            || planned.fold_count != entry.fold_count
        {
            return Err(format!(
                "VCET pre-final source, timeline, fold, or snapshot evidence differs for '{dataset_id}'"
            ));
        }
        let interval_sha256 = recompute_vcet_temporal_interval_fingerprint_sha256(&planned.layout)?;
        if interval_sha256 != entry.temporal_interval_fingerprint_sha256
            || interval_sha256 != universe.temporal_interval_fingerprint_sha256
        {
            return Err(format!(
                "VCET pre-final temporal interval differs for '{dataset_id}'"
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
        if coverage != entry.dataset_coverage || coverage_sha256 != entry.dataset_coverage_sha256 {
            return Err(format!(
                "VCET pre-final actual coverage differs from the universe for '{dataset_id}'"
            ));
        }

        let mut accumulators = (0..candidate_params.len())
            .map(|_| DatasetCandidateAccumulator::default())
            .collect::<Vec<_>>();
        for fold in &planned.layout.folds {
            let train = evaluate_window(
                dataset_id,
                &resolved.candles,
                &universe.execution_spec.backend,
                &universe.execution_spec.ma_source,
                &candidate_params,
                &candidate_ids,
                fold.train.scored_bars,
                planned.layout.position_boundary_policy,
            )?;
            let validation = evaluate_window(
                dataset_id,
                &resolved.candles,
                &universe.execution_spec.backend,
                &universe.execution_spec.ma_source,
                &candidate_params,
                &candidate_ids,
                fold.validation.scored_bars,
                planned.layout.position_boundary_policy,
            )?;
            validate_strict_backend_provenance(&train.backend, &universe.execution_spec.backend)?;
            validate_strict_backend_provenance(
                &validation.backend,
                &universe.execution_spec.backend,
            )?;
            update_dataset_candidate_accumulators(
                &mut accumulators,
                dataset_id,
                fold.fold_index,
                &train,
                &validation,
                minimum_support.min_train_trades_per_fold,
                minimum_support.min_validation_trades_per_fold,
            )?;
        }
        let candidates = accumulators
            .into_iter()
            .enumerate()
            .map(|(local_index, accumulator)| {
                dataset_candidate_evidence(
                    descriptor.candidate_start_index + local_index,
                    candidate_ids[local_index].clone(),
                    accumulator,
                )
            })
            .collect::<Vec<_>>();
        datasets.push(VcetTemporalPreFinalDatasetEvidenceV1 {
            dataset_id: dataset_id.clone(),
            loaded_data_id: binding.loaded_data_id.clone(),
            source_dataset_sha256: entry.source_dataset_sha256,
            dataset_attestation_sha256: entry.dataset_attestation_sha256,
            timeline_sha256: entry.timeline_sha256,
            loaded_candles_snapshot_sha256: snapshot_sha256,
            dataset_coverage: coverage,
            dataset_coverage_sha256: coverage_sha256,
            temporal_interval_fingerprint_sha256: interval_sha256,
            fold_count: planned.fold_count,
            candidates,
        });
    }
    if !loaded_by_dataset.is_empty() {
        return Err("unrequested resolved VCET pre-final datasets were supplied".to_string());
    }

    let mut candidates = Vec::with_capacity(candidate_params.len());
    for local_index in 0..candidate_params.len() {
        let mut aggregate = VcetTemporalPreFinalShardCandidateAggregateV1 {
            global_candidate_ordinal: descriptor.candidate_start_index + local_index,
            candidate_id: candidate_ids[local_index].clone(),
            params: candidate_params[local_index].clone(),
            validation_sharpe_sum: 0.0,
            validation_observation_count: 0,
            total_train_trades: 0,
            total_validation_trades: 0,
            support_failure_count: 0,
            non_finite_metric_count: 0,
        };
        for dataset in &datasets {
            let row = &dataset.candidates[local_index];
            aggregate.validation_sharpe_sum += row.validation_sharpe_sum;
            aggregate.validation_observation_count = aggregate
                .validation_observation_count
                .checked_add(row.finite_validation_observation_count)
                .ok_or_else(|| "VCET pre-final observation count overflow".to_string())?;
            aggregate.total_train_trades = aggregate
                .total_train_trades
                .checked_add(row.total_train_trades)
                .ok_or_else(|| "VCET pre-final train trade count overflow".to_string())?;
            aggregate.total_validation_trades = aggregate
                .total_validation_trades
                .checked_add(row.total_validation_trades)
                .ok_or_else(|| "VCET pre-final validation trade count overflow".to_string())?;
            aggregate.support_failure_count = aggregate
                .support_failure_count
                .checked_add(row.support_failure_count)
                .ok_or_else(|| "VCET pre-final support failure count overflow".to_string())?;
            aggregate.non_finite_metric_count = aggregate
                .non_finite_metric_count
                .checked_add(row.non_finite_metric_count)
                .ok_or_else(|| "VCET pre-final non-finite count overflow".to_string())?;
        }
        if !aggregate.validation_sharpe_sum.is_finite() {
            return Err("VCET pre-final validation Sharpe sum is non-finite".to_string());
        }
        candidates.push(aggregate);
    }
    let execution_spec_sha256 = execution_spec_sha256(&universe.execution_spec)?;
    let payload = VcetTemporalPreFinalShardArtifactPayloadV1 {
        schema_version: VCET_TEMPORAL_PRE_FINAL_SHARD_ARTIFACT_SCHEMA_VERSION,
        evidence_kind: "vcet_temporal_pre_final_shard_artifact".to_string(),
        universe_sha256: req.sealed_universe.universe_sha256.clone(),
        candidate_manifest_sha256: universe.candidate_manifest_sha256.clone(),
        plan_sha256: universe.plan_identity.plan_sha256.clone(),
        execution_spec_sha256,
        execution_spec: universe.execution_spec.clone(),
        temporal_interval_fingerprint_sha256: universe.temporal_interval_fingerprint_sha256.clone(),
        selection_shard: descriptor,
        planning_evidence_sha256: canonical_planning.evidence_sha256,
        dataset_count: datasets.len(),
        candidate_count: candidate_params.len(),
        fold_count_per_dataset: universe.fold_count_per_dataset,
        expected_candidate_window_trial_count: expected_trials,
        evaluated_candidate_window_trial_count: expected_trials,
        final_test_scoring_access_count: 0,
        candidates,
        datasets,
        limitations: pre_final_shard_limitations(),
    };
    let artifact_sha256 = recompute_vcet_temporal_pre_final_shard_artifact_sha256(&payload)?;
    let response = ExecuteVcetTemporalPreFinalShardResponseV1 {
        payload,
        artifact_sha256,
    };
    validate_vcet_temporal_pre_final_shard_artifact(&response)?;
    Ok(response)
}

pub(super) fn execute_vcet_temporal_pre_final_shard_blocking_with_loaded_candles(
    req: ExecuteVcetTemporalPreFinalShardRequestV1,
    loaded: Vec<(VcetTemporalLoadedDatasetBindingV1, Arc<Candles>)>,
) -> Result<ExecuteVcetTemporalPreFinalShardResponseV1, String> {
    let loaded = loaded
        .into_iter()
        .map(|(binding, candles)| LoadedPreFinalDataset {
            dataset_id: binding.dataset_id,
            loaded_data_id: binding.loaded_data_id,
            candles,
        })
        .collect();
    execute_vcet_temporal_pre_final_shard_blocking_with_resolved_candles(req, loaded)
}

pub(super) fn recompute_vcet_temporal_pre_final_shard_artifact_sha256(
    payload: &VcetTemporalPreFinalShardArtifactPayloadV1,
) -> Result<String, String> {
    hash_enveloped(
        PRE_FINAL_SHARD_ARTIFACT_HASH_CONTRACT,
        payload,
        "VCET pre-final shard artifact",
    )
}

pub(super) fn validate_vcet_temporal_pre_final_shard_artifact(
    artifact: &ExecuteVcetTemporalPreFinalShardResponseV1,
) -> Result<(), String> {
    let payload = &artifact.payload;
    let candidate_span = payload
        .selection_shard
        .candidate_end_index_exclusive
        .checked_sub(payload.selection_shard.candidate_start_index)
        .ok_or_else(|| "VCET pre-final shard candidate interval is reversed".to_string())?;
    let expected_trials = payload
        .dataset_count
        .checked_mul(payload.candidate_count)
        .and_then(|value| value.checked_mul(payload.fold_count_per_dataset))
        .and_then(|value| value.checked_mul(2))
        .ok_or_else(|| "VCET pre-final shard trial count overflow".to_string())?;
    if payload.schema_version != VCET_TEMPORAL_PRE_FINAL_SHARD_ARTIFACT_SCHEMA_VERSION
        || payload.evidence_kind != "vcet_temporal_pre_final_shard_artifact"
        || payload.final_test_scoring_access_count != 0
        || payload.dataset_count == 0
        || payload.candidate_count == 0
        || payload.fold_count_per_dataset == 0
        || payload.dataset_count != payload.datasets.len()
        || payload.candidate_count != payload.candidates.len()
        || payload.execution_spec_sha256 != execution_spec_sha256(&payload.execution_spec)?
        || payload
            .selection_shard
            .expected_candidate_window_trial_count
            != expected_trials
        || payload.expected_candidate_window_trial_count != expected_trials
        || payload.expected_candidate_window_trial_count
            != payload.evaluated_candidate_window_trial_count
        || payload.limitations != pre_final_shard_limitations()
    {
        return Err("VCET pre-final shard artifact structural contract mismatch".to_string());
    }
    if payload.selection_shard.role != VcetTemporalDatasetCohortRoleV1::Selection
        || payload.selection_shard.locked_winner_only
        || payload.selection_shard.dataset_ids.len() != payload.dataset_count
        || candidate_span != payload.candidate_count
    {
        return Err("VCET pre-final shard descriptor/cardinality mismatch".to_string());
    }
    let observed_dataset_ids = payload
        .datasets
        .iter()
        .map(|dataset| dataset.dataset_id.as_str())
        .collect::<Vec<_>>();
    let descriptor_dataset_ids = payload
        .selection_shard
        .dataset_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if observed_dataset_ids != descriptor_dataset_ids
        || observed_dataset_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            != observed_dataset_ids.len()
    {
        return Err(
            "VCET pre-final dataset evidence does not exactly follow descriptor order".to_string(),
        );
    }
    for (local_index, candidate) in payload.candidates.iter().enumerate() {
        if candidate.global_candidate_ordinal
            != payload.selection_shard.candidate_start_index + local_index
            || candidate.candidate_id.trim() != candidate.candidate_id
            || candidate.candidate_id.is_empty()
            || !candidate.validation_sharpe_sum.is_finite()
        {
            return Err("VCET pre-final shard candidate identity/metric mismatch".to_string());
        }
    }
    for dataset in &payload.datasets {
        if dataset.candidates.len() != payload.candidate_count
            || dataset.fold_count != payload.fold_count_per_dataset
            || dataset.temporal_interval_fingerprint_sha256
                != payload.temporal_interval_fingerprint_sha256
        {
            return Err("VCET pre-final dataset evidence cardinality mismatch".to_string());
        }
        for (local_index, candidate) in dataset.candidates.iter().enumerate() {
            if candidate.global_candidate_ordinal
                != payload.selection_shard.candidate_start_index + local_index
                || candidate
                    .mean_validation_pnl
                    .is_some_and(|value| !value.is_finite())
                || candidate
                    .mean_validation_sharpe
                    .is_some_and(|value| !value.is_finite())
                || !candidate.validation_pnl_sum.is_finite()
                || !candidate.validation_sharpe_sum.is_finite()
                || candidate.candidate_id != payload.candidates[local_index].candidate_id
                || candidate.validation_fold_count != payload.fold_count_per_dataset
                || candidate.finite_validation_observation_count > candidate.validation_fold_count
                || candidate.non_finite_metric_count
                    != candidate.validation_fold_count
                        - candidate.finite_validation_observation_count
                || candidate.hard_support_passed != (candidate.support_failure_count == 0)
                || candidate.metric_support_passed
                    != (candidate.non_finite_metric_count == 0
                        && candidate.finite_validation_observation_count
                            == candidate.validation_fold_count)
                || candidate.support_failure_examples.len()
                    != candidate
                        .support_failure_count
                        .min(MAX_SUPPORT_FAILURE_EXAMPLES_PER_DATASET_CANDIDATE)
            {
                return Err("VCET pre-final dataset candidate evidence is invalid".to_string());
            }
            let expected_mean_pnl = canonical_mean_from_sum_bits(
                candidate.validation_pnl_sum.to_bits(),
                candidate.finite_validation_observation_count,
            );
            let expected_mean_sharpe = canonical_mean_from_sum_bits(
                candidate.validation_sharpe_sum.to_bits(),
                candidate.finite_validation_observation_count,
            );
            if !same_optional_f64_bits(candidate.mean_validation_pnl, expected_mean_pnl)
                || !same_optional_f64_bits(candidate.mean_validation_sharpe, expected_mean_sharpe)
            {
                return Err(
                    "VCET pre-final dataset candidate sum/mean evidence is inconsistent"
                        .to_string(),
                );
            }
            for failure in &candidate.support_failure_examples {
                if failure.dataset_id != dataset.dataset_id
                    || failure.fold_index >= payload.fold_count_per_dataset
                    || !matches!(failure.stage.as_str(), "train" | "validation")
                    || failure.observed_trades >= failure.minimum_trades
                {
                    return Err("VCET pre-final support-failure evidence is invalid".to_string());
                }
            }
        }
    }
    for local_index in 0..payload.candidates.len() {
        let recomputed = recompute_shard_candidate_aggregate(payload, local_index)?;
        let declared = &payload.candidates[local_index];
        if recomputed.global_candidate_ordinal != declared.global_candidate_ordinal
            || recomputed.candidate_id != declared.candidate_id
            || !serialized_eq(&recomputed.params, &declared.params)?
            || recomputed.validation_sharpe_sum.to_bits()
                != declared.validation_sharpe_sum.to_bits()
            || recomputed.validation_observation_count != declared.validation_observation_count
            || recomputed.total_train_trades != declared.total_train_trades
            || recomputed.total_validation_trades != declared.total_validation_trades
            || recomputed.support_failure_count != declared.support_failure_count
            || recomputed.non_finite_metric_count != declared.non_finite_metric_count
        {
            return Err(
                "VCET pre-final shard aggregate does not match dataset evidence".to_string(),
            );
        }
    }
    let actual = recompute_vcet_temporal_pre_final_shard_artifact_sha256(payload)?;
    if actual != artifact.artifact_sha256 {
        return Err(format!(
            "VCET pre-final shard artifact SHA mismatch (declared {}, actual {actual})",
            artifact.artifact_sha256
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn execute_vcet_temporal_pre_final_shard(
    req: ExecuteVcetTemporalPreFinalShardRequestV1,
    state: tauri::State<'_, AppState>,
) -> Result<ExecuteVcetTemporalPreFinalShardResponseV1, String> {
    let loaded = resolve_pre_final_candles(&req.loaded_datasets, &state)?;
    tauri::async_runtime::spawn_blocking(move || {
        execute_vcet_temporal_pre_final_shard_blocking_with_resolved_candles(req, loaded)
    })
    .await
    .map_err(|error| format!("VCET pre-final shard task failed: {error}"))?
}

#[derive(Default)]
struct GlobalCandidateAccumulator {
    validation_sharpe_sum: f64,
    validation_observation_count: usize,
    total_train_trades: u64,
    total_validation_trades: u64,
    support_failure_count: usize,
    non_finite_metric_count: usize,
    datasets: BTreeMap<String, VcetTemporalPreFinalDatasetCandidateEvidenceV1>,
}

#[derive(Serialize)]
struct GlobalEvidenceHashInput<'a> {
    hash_contract: &'static str,
    universe_sha256: &'a str,
    candidate_manifest_sha256: &'a str,
    plan_sha256: &'a str,
    execution_spec_sha256: &'a str,
    temporal_interval_fingerprint_sha256: &'a str,
    selection_reduction_contract: &'a str,
    ordered_shards: &'a [VcetTemporalPreFinalShardReductionEvidenceV1],
    candidates: &'a [VcetTemporalGlobalCandidateEvidenceV1],
    pre_final_candidate_window_trial_count: usize,
}

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

/// Applies the sealed dataset-share policy. Raw fold-level failure counts are
/// retained as integrity evidence; using them as zero-only gates would silently
/// strengthen every configured support-share floor to 100%.
fn global_candidate_meets_cross_universe_admission(
    candidate: &VcetTemporalGlobalCandidateEvidenceV1,
    admission: &VcetTemporalCrossUniverseAdmissionV1,
) -> bool {
    candidate.mean_validation_sharpe.is_some_and(f64::is_finite)
        && candidate.requested_dataset_coverage_share
            >= admission.min_requested_dataset_coverage_share
        && candidate.evaluated_dataset_share_requested
            >= admission.min_evaluated_dataset_share_requested
        && candidate.profitable_dataset_share_requested >= admission.min_profitable_dataset_share
        && candidate.hard_support_dataset_share_requested
            >= admission.min_hard_support_dataset_share
        && candidate.metric_support_dataset_share_requested
            >= admission.min_metric_support_dataset_share
        && candidate
            .validation_pnl_p10
            .is_some_and(|value| value >= admission.min_validation_pnl_p10)
        && candidate
            .validation_sharpe_p10
            .is_some_and(|value| value >= admission.min_validation_sharpe_p10)
}

fn validate_final_stage_nonce(value: &str) -> Result<String, String> {
    let value = value.trim();
    if !(16..=128).contains(&value.len())
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
    {
        return Err(
            "VCET final_stage_nonce must contain 16..128 ASCII letters, digits, '-', '_', or ':'"
                .to_string(),
        );
    }
    Ok(value.to_string())
}

fn recompute_shard_candidate_aggregate(
    payload: &VcetTemporalPreFinalShardArtifactPayloadV1,
    local_index: usize,
) -> Result<VcetTemporalPreFinalShardCandidateAggregateV1, String> {
    let candidate = payload
        .candidates
        .get(local_index)
        .ok_or_else(|| "missing VCET pre-final shard candidate aggregate".to_string())?;
    let mut recomputed = VcetTemporalPreFinalShardCandidateAggregateV1 {
        global_candidate_ordinal: candidate.global_candidate_ordinal,
        candidate_id: candidate.candidate_id.clone(),
        params: candidate.params.clone(),
        validation_sharpe_sum: 0.0,
        validation_observation_count: 0,
        total_train_trades: 0,
        total_validation_trades: 0,
        support_failure_count: 0,
        non_finite_metric_count: 0,
    };
    for dataset in &payload.datasets {
        let row = dataset
            .candidates
            .get(local_index)
            .ok_or_else(|| "missing VCET pre-final dataset candidate row".to_string())?;
        if row.global_candidate_ordinal != candidate.global_candidate_ordinal
            || row.candidate_id != candidate.candidate_id
        {
            return Err("VCET pre-final dataset/shard candidate identity mismatch".to_string());
        }
        recomputed.validation_sharpe_sum += row.validation_sharpe_sum;
        recomputed.validation_observation_count = recomputed
            .validation_observation_count
            .checked_add(row.finite_validation_observation_count)
            .ok_or_else(|| "VCET pre-final observation count overflow".to_string())?;
        recomputed.total_train_trades = recomputed
            .total_train_trades
            .checked_add(row.total_train_trades)
            .ok_or_else(|| "VCET pre-final train trade count overflow".to_string())?;
        recomputed.total_validation_trades = recomputed
            .total_validation_trades
            .checked_add(row.total_validation_trades)
            .ok_or_else(|| "VCET pre-final validation trade count overflow".to_string())?;
        recomputed.support_failure_count = recomputed
            .support_failure_count
            .checked_add(row.support_failure_count)
            .ok_or_else(|| "VCET pre-final support failure count overflow".to_string())?;
        recomputed.non_finite_metric_count = recomputed
            .non_finite_metric_count
            .checked_add(row.non_finite_metric_count)
            .ok_or_else(|| "VCET pre-final non-finite count overflow".to_string())?;
    }
    if !recomputed.validation_sharpe_sum.is_finite() {
        return Err("VCET pre-final recomputed Sharpe sum is non-finite".to_string());
    }
    Ok(recomputed)
}

fn validate_shard_against_universe(
    artifact: &ExecuteVcetTemporalPreFinalShardResponseV1,
    universe: &VcetTemporalSealedDatasetUniverseV1,
    expected_descriptor: &VcetTemporalUniverseShardV1,
) -> Result<(), String> {
    validate_vcet_temporal_pre_final_shard_artifact(artifact)?;
    let payload = &artifact.payload;
    if payload.universe_sha256 != universe.universe_sha256
        || !serialized_eq(&payload.selection_shard, expected_descriptor)?
        || payload.candidate_manifest_sha256 != universe.payload.candidate_manifest_sha256
        || payload.plan_sha256 != universe.payload.plan_identity.plan_sha256
        || payload.temporal_interval_fingerprint_sha256
            != universe.payload.temporal_interval_fingerprint_sha256
        || !serialized_eq(&payload.execution_spec, &universe.payload.execution_spec)?
        || payload.execution_spec_sha256 != execution_spec_sha256(&universe.payload.execution_spec)?
        || payload.fold_count_per_dataset != universe.payload.fold_count_per_dataset
        || payload.expected_candidate_window_trial_count
            != expected_descriptor.expected_candidate_window_trial_count
    {
        return Err(format!(
            "VCET pre-final artifact '{}' identity differs from the universe descriptor",
            expected_descriptor.shard_id
        ));
    }
    let expected_dataset_ids = expected_descriptor
        .dataset_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let observed_dataset_ids = payload
        .datasets
        .iter()
        .map(|dataset| dataset.dataset_id.clone())
        .collect::<BTreeSet<_>>();
    if expected_dataset_ids != observed_dataset_ids
        || observed_dataset_ids.len() != payload.datasets.len()
    {
        return Err("VCET pre-final artifact dataset coverage differs from descriptor".to_string());
    }
    let entries = universe_entry_map(&universe.payload)?;
    for dataset in &payload.datasets {
        let entry = entries.get(&dataset.dataset_id).ok_or_else(|| {
            format!(
                "VCET pre-final artifact dataset '{}' is absent from universe",
                dataset.dataset_id
            )
        })?;
        if entry.role != VcetTemporalDatasetCohortRoleV1::Selection
            || dataset.source_dataset_sha256 != entry.source_dataset_sha256
            || dataset.dataset_attestation_sha256 != entry.dataset_attestation_sha256
            || dataset.timeline_sha256 != entry.timeline_sha256
            || dataset.loaded_candles_snapshot_sha256 != entry.loaded_candles_snapshot_sha256
            || dataset.dataset_coverage != entry.dataset_coverage
            || dataset.dataset_coverage_sha256 != entry.dataset_coverage_sha256
            || dataset.temporal_interval_fingerprint_sha256
                != entry.temporal_interval_fingerprint_sha256
            || dataset.fold_count != entry.fold_count
        {
            return Err(format!(
                "VCET pre-final dataset '{}' evidence differs from universe",
                dataset.dataset_id
            ));
        }
    }
    for local_index in 0..payload.candidates.len() {
        let recomputed = recompute_shard_candidate_aggregate(payload, local_index)?;
        let declared = &payload.candidates[local_index];
        if recomputed.global_candidate_ordinal != declared.global_candidate_ordinal
            || recomputed.candidate_id != declared.candidate_id
            || !serialized_eq(&recomputed.params, &declared.params)?
            || recomputed.validation_sharpe_sum.to_bits()
                != declared.validation_sharpe_sum.to_bits()
            || recomputed.validation_observation_count != declared.validation_observation_count
            || recomputed.total_train_trades != declared.total_train_trades
            || recomputed.total_validation_trades != declared.total_validation_trades
            || recomputed.support_failure_count != declared.support_failure_count
            || recomputed.non_finite_metric_count != declared.non_finite_metric_count
        {
            return Err("VCET pre-final shard candidate aggregate is inconsistent".to_string());
        }
    }
    Ok(())
}

fn recompute_winner_lock_sha256(lock: &VcetTemporalGlobalWinnerLockV1) -> Result<String, String> {
    sha256_serializable(
        &WinnerLockHashInput {
            hash_contract: GLOBAL_WINNER_LOCK_HASH_CONTRACT,
            global_candidate_ordinal: lock.global_candidate_ordinal,
            candidate_id: &lock.candidate_id,
            params: &lock.params,
            selection_score_bits: lock.selection_score.to_bits(),
            runner_up_candidate_id: &lock.runner_up_candidate_id,
            runner_up_selection_score_bits: lock.runner_up_selection_score.map(f64::to_bits),
            winner_to_runner_up_score_margin_bits: lock
                .winner_to_runner_up_score_margin
                .map(f64::to_bits),
            candidate_id_tie_break_used: lock.candidate_id_tie_break_used,
            selection_reduction_contract: &lock.selection_reduction_contract,
            universe_sha256: &lock.universe_sha256,
            candidate_manifest_sha256: &lock.candidate_manifest_sha256,
            plan_sha256: &lock.plan_sha256,
            execution_spec_sha256: &lock.execution_spec_sha256,
            global_pre_final_evidence_sha256: &lock.global_pre_final_evidence_sha256,
        },
        "VCET temporal global winner lock",
    )
}

fn recompute_final_prior_state_sha256(
    universe_sha256: &str,
    winner_lock_sha256: &str,
    global_pre_final_evidence_sha256: &str,
    final_stage_nonce: &str,
    shard: &VcetTemporalUniverseShardV1,
) -> Result<String, String> {
    sha256_serializable(
        &FinalPriorStateHashInput {
            hash_contract: FINAL_SHARD_PRIOR_STATE_HASH_CONTRACT,
            universe_sha256,
            winner_lock_sha256,
            global_pre_final_evidence_sha256,
            final_stage_nonce,
            shard,
            state: "authorized_unconsumed",
        },
        "VCET final-shard authorized prior state",
    )
}

fn recompute_final_authorization_sha256(
    authorization: &VcetTemporalFinalShardAuthorizationV1,
) -> Result<String, String> {
    let mut canonical = authorization.clone();
    canonical.authorization_sha256.clear();
    hash_enveloped(
        FINAL_SHARD_AUTHORIZATION_HASH_CONTRACT,
        &canonical,
        "VCET final-shard authorization",
    )
}

pub(super) fn validate_vcet_temporal_final_shard_authorization(
    authorization: &VcetTemporalFinalShardAuthorizationV1,
    universe: &VcetTemporalSealedDatasetUniverseV1,
    winner: &VcetTemporalGlobalWinnerLockV1,
) -> Result<(), String> {
    if authorization.shard.role != VcetTemporalDatasetCohortRoleV1::FinalTest
        || !authorization.shard.locked_winner_only
        || authorization.universe_sha256 != universe.universe_sha256
        || authorization.winner_lock_sha256 != winner.winner_lock_sha256
        || authorization.global_candidate_ordinal != winner.global_candidate_ordinal
        || authorization.candidate_id != winner.candidate_id
        || authorization.global_pre_final_evidence_sha256 != winner.global_pre_final_evidence_sha256
    {
        return Err("VCET final-shard authorization identity mismatch".to_string());
    }
    let expected_shards = universe_shards_for_role(
        &universe.payload,
        VcetTemporalDatasetCohortRoleV1::FinalTest,
    );
    if !expected_shards.iter().any(|shard| {
        shard.shard_id == authorization.shard.shard_id
            && serialized_eq(*shard, &authorization.shard).unwrap_or(false)
    }) {
        return Err("VCET authorization references a non-canonical final shard".to_string());
    }
    let expected_prior = recompute_final_prior_state_sha256(
        &authorization.universe_sha256,
        &authorization.winner_lock_sha256,
        &authorization.global_pre_final_evidence_sha256,
        &authorization.final_stage_nonce,
        &authorization.shard,
    )?;
    if expected_prior != authorization.expected_prior_state_sha256 {
        return Err("VCET final-shard authorized prior state SHA mismatch".to_string());
    }
    let actual = recompute_final_authorization_sha256(authorization)?;
    if actual != authorization.authorization_sha256 {
        return Err("VCET final-shard authorization SHA mismatch".to_string());
    }
    Ok(())
}

pub fn reduce_vcet_temporal_pre_final_shards_blocking(
    req: ReduceVcetTemporalPreFinalShardsRequestV1,
) -> Result<ReduceVcetTemporalPreFinalShardsResponseV1, String> {
    validate_sealed_vcet_temporal_universe(&req.sealed_universe, &req.expected_universe_sha256)?;
    let final_stage_nonce = validate_final_stage_nonce(&req.final_stage_nonce)?;
    let universe = &req.sealed_universe.payload;
    let expected_shards =
        universe_shards_for_role(universe, VcetTemporalDatasetCohortRoleV1::Selection);
    if expected_shards.is_empty() {
        return Err("VCET pre-final reduction has no selection shard descriptors".to_string());
    }
    if expected_shards.len() > MAX_VCET_TEMPORAL_INLINE_PRE_FINAL_SHARD_FAN_IN {
        return Err(format!(
            "VCET inline pre-final reduction requires {} shards, exceeding the bounded fan-in of {MAX_VCET_TEMPORAL_INLINE_PRE_FINAL_SHARD_FAN_IN}; use the future persisted hierarchical/streaming reducer",
            expected_shards.len()
        ));
    }
    if req.pre_final_shard_artifacts.len() != expected_shards.len() {
        return Err(format!(
            "VCET pre-final reduction received {} artifacts for {} sealed selection descriptors",
            req.pre_final_shard_artifacts.len(),
            expected_shards.len()
        ));
    }
    let mut artifact_by_shard = BTreeMap::new();
    for artifact in &req.pre_final_shard_artifacts {
        if artifact_by_shard
            .insert(artifact.payload.selection_shard.shard_id.clone(), artifact)
            .is_some()
        {
            return Err(format!(
                "duplicate VCET pre-final shard artifact '{}'",
                artifact.payload.selection_shard.shard_id
            ));
        }
    }
    let manifest = &universe.candidate_manifest.payload.candidates;
    let mut accumulators = (0..manifest.len())
        .map(|_| GlobalCandidateAccumulator::default())
        .collect::<Vec<_>>();
    let params_by_candidate = manifest
        .iter()
        .map(|candidate| {
            candidate_params_from_manifest(
                &candidate.identity_inputs,
                &universe.execution_spec.strategy,
            )
        })
        .collect::<Vec<_>>();
    let mut dataset_identity_by_id: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut ordered_shards = Vec::with_capacity(expected_shards.len());
    let mut total_trials = 0usize;

    for descriptor in expected_shards {
        let artifact = artifact_by_shard
            .remove(&descriptor.shard_id)
            .ok_or_else(|| {
                format!(
                    "missing VCET pre-final shard artifact '{}'",
                    descriptor.shard_id
                )
            })?;
        validate_shard_against_universe(artifact, &req.sealed_universe, descriptor)?;
        total_trials = total_trials
            .checked_add(artifact.payload.evaluated_candidate_window_trial_count)
            .ok_or_else(|| "VCET global pre-final trial count overflow".to_string())?;
        ordered_shards.push(VcetTemporalPreFinalShardReductionEvidenceV1 {
            shard_ordinal: descriptor.shard_ordinal,
            shard_id: descriptor.shard_id.clone(),
            artifact_sha256: artifact.artifact_sha256.clone(),
            dataset_count: artifact.payload.dataset_count,
            candidate_count: artifact.payload.candidate_count,
            candidate_window_trial_count: artifact.payload.evaluated_candidate_window_trial_count,
        });
        for dataset in &artifact.payload.datasets {
            #[derive(Serialize)]
            struct DatasetIdentity<'a> {
                dataset_id: &'a str,
                source_dataset_sha256: &'a str,
                dataset_attestation_sha256: &'a str,
                timeline_sha256: &'a str,
                loaded_candles_snapshot_sha256: &'a str,
                dataset_coverage_sha256: &'a str,
                temporal_interval_fingerprint_sha256: &'a str,
                fold_count: usize,
            }
            let identity = serde_json::to_vec(&DatasetIdentity {
                dataset_id: &dataset.dataset_id,
                source_dataset_sha256: &dataset.source_dataset_sha256,
                dataset_attestation_sha256: &dataset.dataset_attestation_sha256,
                timeline_sha256: &dataset.timeline_sha256,
                loaded_candles_snapshot_sha256: &dataset.loaded_candles_snapshot_sha256,
                dataset_coverage_sha256: &dataset.dataset_coverage_sha256,
                temporal_interval_fingerprint_sha256: &dataset.temporal_interval_fingerprint_sha256,
                fold_count: dataset.fold_count,
            })
            .map_err(|error| format!("failed to serialize VCET dataset identity: {error}"))?;
            if let Some(existing) = dataset_identity_by_id.get(&dataset.dataset_id) {
                if existing != &identity {
                    return Err(format!(
                        "VCET pre-final repeated dataset '{}' has inconsistent identity across candidate shards",
                        dataset.dataset_id
                    ));
                }
            } else {
                dataset_identity_by_id.insert(dataset.dataset_id.clone(), identity);
            }
            for row in &dataset.candidates {
                let global = row.global_candidate_ordinal;
                let manifest_candidate = manifest.get(global).ok_or_else(|| {
                    format!("VCET pre-final candidate ordinal {global} is outside manifest")
                })?;
                if row.candidate_id != manifest_candidate.candidate_id {
                    return Err(format!(
                        "VCET pre-final candidate {global} ID differs from manifest"
                    ));
                }
                let accumulator = accumulators.get_mut(global).ok_or_else(|| {
                    "VCET pre-final candidate accumulator lookup failed".to_string()
                })?;
                if accumulator
                    .datasets
                    .insert(dataset.dataset_id.clone(), row.clone())
                    .is_some()
                {
                    return Err(format!(
                        "VCET pre-final dataset/candidate cell overlaps for '{}' candidate {global}",
                        dataset.dataset_id
                    ));
                }
                accumulator.validation_sharpe_sum += row.validation_sharpe_sum;
                accumulator.validation_observation_count = accumulator
                    .validation_observation_count
                    .checked_add(row.finite_validation_observation_count)
                    .ok_or_else(|| "VCET global observation count overflow".to_string())?;
                accumulator.total_train_trades = accumulator
                    .total_train_trades
                    .checked_add(row.total_train_trades)
                    .ok_or_else(|| "VCET global train trade count overflow".to_string())?;
                accumulator.total_validation_trades = accumulator
                    .total_validation_trades
                    .checked_add(row.total_validation_trades)
                    .ok_or_else(|| "VCET global validation trade count overflow".to_string())?;
                accumulator.support_failure_count = accumulator
                    .support_failure_count
                    .checked_add(row.support_failure_count)
                    .ok_or_else(|| "VCET global support failure count overflow".to_string())?;
                accumulator.non_finite_metric_count = accumulator
                    .non_finite_metric_count
                    .checked_add(row.non_finite_metric_count)
                    .ok_or_else(|| "VCET global non-finite count overflow".to_string())?;
            }
        }
    }
    if !artifact_by_shard.is_empty() {
        return Err("unsealed VCET pre-final shard artifacts were supplied".to_string());
    }
    if dataset_identity_by_id.len() != universe.selection_dataset_count {
        return Err(format!(
            "VCET pre-final shard set covers {} unique datasets, expected {}",
            dataset_identity_by_id.len(),
            universe.selection_dataset_count
        ));
    }

    let requested = universe.selection_dataset_count;
    let denominator = requested as f64;
    let admission = universe.execution_spec.cross_universe_admission;
    let mut candidates = Vec::with_capacity(manifest.len());
    for (global, accumulator) in accumulators.into_iter().enumerate() {
        if accumulator.datasets.len() != requested {
            return Err(format!(
                "VCET candidate {global} covers {} requested datasets, expected {requested}",
                accumulator.datasets.len()
            ));
        }
        if !accumulator.validation_sharpe_sum.is_finite() {
            return Err(format!(
                "VCET candidate {global} has a non-finite global validation Sharpe sum"
            ));
        }
        let rows = accumulator.datasets.values().collect::<Vec<_>>();
        let covered = rows.len();
        let evaluated = rows
            .iter()
            .filter(|row| row.mean_validation_pnl.is_some() && row.mean_validation_sharpe.is_some())
            .count();
        let profitable = rows
            .iter()
            .filter(|row| row.mean_validation_pnl.is_some_and(|value| value > 0.0))
            .count();
        let hard_support = rows.iter().filter(|row| row.hard_support_passed).count();
        let metric_support = rows.iter().filter(|row| row.metric_support_passed).count();
        let pnl_values = rows
            .iter()
            .filter_map(|row| row.mean_validation_pnl)
            .collect::<Vec<_>>();
        let sharpe_values = rows
            .iter()
            .filter_map(|row| row.mean_validation_sharpe)
            .collect::<Vec<_>>();
        let validation_pnl_p10 = finite_percentile_over_requested(pnl_values, requested, 0.10);
        let validation_sharpe_p10 =
            finite_percentile_over_requested(sharpe_values, requested, 0.10);
        let validation_sharpe_sum_bits = accumulator.validation_sharpe_sum.to_bits();
        let mean_validation_sharpe = canonical_mean_from_sum_bits(
            validation_sharpe_sum_bits,
            accumulator.validation_observation_count,
        );
        let requested_dataset_coverage_share = covered as f64 / denominator;
        let evaluated_dataset_share_requested = evaluated as f64 / denominator;
        let profitable_dataset_share_requested = profitable as f64 / denominator;
        let hard_support_dataset_share_requested = hard_support as f64 / denominator;
        let metric_support_dataset_share_requested = metric_support as f64 / denominator;
        let mut candidate = VcetTemporalGlobalCandidateEvidenceV1 {
            global_candidate_ordinal: global,
            candidate_id: manifest[global].candidate_id.clone(),
            params: params_by_candidate[global].clone(),
            requested_dataset_count: requested,
            covered_dataset_count: covered,
            evaluated_dataset_count: evaluated,
            profitable_dataset_count: profitable,
            hard_support_dataset_count: hard_support,
            metric_support_dataset_count: metric_support,
            requested_dataset_coverage_share,
            evaluated_dataset_share_requested,
            profitable_dataset_share_requested,
            hard_support_dataset_share_requested,
            metric_support_dataset_share_requested,
            validation_pnl_p10,
            validation_sharpe_p10,
            validation_sharpe_sum: f64::from_bits(validation_sharpe_sum_bits),
            mean_validation_sharpe,
            validation_observation_count: accumulator.validation_observation_count,
            total_train_trades: accumulator.total_train_trades,
            total_validation_trades: accumulator.total_validation_trades,
            support_failure_count: accumulator.support_failure_count,
            non_finite_metric_count: accumulator.non_finite_metric_count,
            admission_passed: false,
            selected_before_final_test: false,
        };
        candidate.admission_passed =
            global_candidate_meets_cross_universe_admission(&candidate, &admission);
        candidates.push(candidate);
    }

    let mut eligible = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.admission_passed)
        .map(|(index, candidate)| {
            (
                index,
                candidate
                    .mean_validation_sharpe
                    .expect("admitted candidates have finite selection scores"),
                candidate.candidate_id.clone(),
            )
        })
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.2.cmp(&right.2))
    });
    let (winner_index, winner_score, _) = eligible.first().cloned().ok_or_else(|| {
        "no VCET temporal candidate passed strict support and cross-universe admission".to_string()
    })?;
    candidates[winner_index].selected_before_final_test = true;
    let runner = eligible.get(1).cloned();
    let tie_break_used = runner
        .as_ref()
        .is_some_and(|(_, score, _)| winner_score.total_cmp(score) == Ordering::Equal);
    let margin = runner.as_ref().and_then(|(_, score, _)| {
        let margin = winner_score - *score;
        margin.is_finite().then_some(margin)
    });
    let execution_spec_sha256 = execution_spec_sha256(&universe.execution_spec)?;
    let global_pre_final_evidence_sha256 = sha256_serializable(
        &GlobalEvidenceHashInput {
            hash_contract: PRE_FINAL_GLOBAL_EVIDENCE_HASH_CONTRACT,
            universe_sha256: &req.sealed_universe.universe_sha256,
            candidate_manifest_sha256: &universe.candidate_manifest_sha256,
            plan_sha256: &universe.plan_identity.plan_sha256,
            execution_spec_sha256: &execution_spec_sha256,
            temporal_interval_fingerprint_sha256: &universe.temporal_interval_fingerprint_sha256,
            selection_reduction_contract: &universe.execution_spec.selection_reduction_contract,
            ordered_shards: &ordered_shards,
            candidates: &candidates,
            pre_final_candidate_window_trial_count: total_trials,
        },
        "VCET global pre-final evidence",
    )?;
    let winner = &candidates[winner_index];
    let mut winner_lock = VcetTemporalGlobalWinnerLockV1 {
        global_candidate_ordinal: winner.global_candidate_ordinal,
        candidate_id: winner.candidate_id.clone(),
        params: winner.params.clone(),
        selection_policy: universe.execution_spec.selection_policy,
        selection_score: winner_score,
        runner_up_candidate_id: runner.as_ref().map(|(_, _, id)| id.clone()),
        runner_up_selection_score: runner.as_ref().map(|(_, score, _)| *score),
        winner_to_runner_up_score_margin: margin,
        candidate_id_tie_break_used: tie_break_used,
        selection_reduction_contract: universe.execution_spec.selection_reduction_contract.clone(),
        universe_sha256: req.sealed_universe.universe_sha256.clone(),
        candidate_manifest_sha256: universe.candidate_manifest_sha256.clone(),
        plan_sha256: universe.plan_identity.plan_sha256.clone(),
        execution_spec_sha256: execution_spec_sha256.clone(),
        global_pre_final_evidence_sha256: global_pre_final_evidence_sha256.clone(),
        winner_lock_sha256: String::new(),
    };
    winner_lock.winner_lock_sha256 = recompute_winner_lock_sha256(&winner_lock)?;
    let final_shards =
        universe_shards_for_role(universe, VcetTemporalDatasetCohortRoleV1::FinalTest);
    let mut final_shard_authorizations = Vec::with_capacity(final_shards.len());
    for shard in final_shards {
        let expected_prior_state_sha256 = recompute_final_prior_state_sha256(
            &req.sealed_universe.universe_sha256,
            &winner_lock.winner_lock_sha256,
            &global_pre_final_evidence_sha256,
            &final_stage_nonce,
            shard,
        )?;
        let mut authorization = VcetTemporalFinalShardAuthorizationV1 {
            shard: shard.clone(),
            universe_sha256: req.sealed_universe.universe_sha256.clone(),
            winner_lock_sha256: winner_lock.winner_lock_sha256.clone(),
            global_candidate_ordinal: winner_lock.global_candidate_ordinal,
            candidate_id: winner_lock.candidate_id.clone(),
            global_pre_final_evidence_sha256: global_pre_final_evidence_sha256.clone(),
            final_stage_nonce: final_stage_nonce.clone(),
            expected_prior_state_sha256,
            authorization_sha256: String::new(),
        };
        authorization.authorization_sha256 = recompute_final_authorization_sha256(&authorization)?;
        final_shard_authorizations.push(authorization);
    }
    let payload = VcetTemporalPreFinalReductionArtifactPayloadV1 {
        schema_version: VCET_TEMPORAL_PRE_FINAL_REDUCTION_ARTIFACT_SCHEMA_VERSION,
        evidence_kind: "vcet_temporal_pre_final_reduction_artifact".to_string(),
        universe_sha256: req.sealed_universe.universe_sha256.clone(),
        candidate_manifest_sha256: universe.candidate_manifest_sha256.clone(),
        plan_sha256: universe.plan_identity.plan_sha256.clone(),
        execution_spec_sha256,
        execution_spec: universe.execution_spec.clone(),
        temporal_interval_fingerprint_sha256: universe.temporal_interval_fingerprint_sha256.clone(),
        selection_reduction_contract: universe.execution_spec.selection_reduction_contract.clone(),
        global_pre_final_evidence_sha256,
        requested_selection_dataset_count: universe.selection_dataset_count,
        covered_selection_dataset_count: dataset_identity_by_id.len(),
        expected_selection_shard_count: ordered_shards.len(),
        reduced_selection_shard_count: ordered_shards.len(),
        pre_final_candidate_window_trial_count: total_trials,
        final_test_scoring_access_count: 0,
        ordered_shards,
        candidates,
        winner_lock,
        final_stage_nonce,
        final_shard_authorizations,
        limitations: pre_final_reduction_limitations(),
    };
    let artifact_sha256 = recompute_vcet_temporal_pre_final_reduction_artifact_sha256(&payload)?;
    let response = ReduceVcetTemporalPreFinalShardsResponseV1 {
        payload,
        artifact_sha256,
    };
    validate_vcet_temporal_pre_final_reduction_artifact(&response, &response.artifact_sha256)?;
    Ok(response)
}

pub(super) fn recompute_vcet_temporal_pre_final_reduction_artifact_sha256(
    payload: &VcetTemporalPreFinalReductionArtifactPayloadV1,
) -> Result<String, String> {
    hash_enveloped(
        PRE_FINAL_REDUCTION_ARTIFACT_HASH_CONTRACT,
        payload,
        "VCET temporal pre-final reduction artifact",
    )
}

fn share(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then(|| numerator as f64 / denominator as f64)
}

fn same_optional_f64_bits(left: Option<f64>, right: Option<f64>) -> bool {
    left.map(f64::to_bits) == right.map(f64::to_bits)
}

pub(super) fn validate_vcet_temporal_pre_final_reduction_artifact(
    artifact: &ReduceVcetTemporalPreFinalShardsResponseV1,
    expected_artifact_sha256: &str,
) -> Result<(), String> {
    let payload = &artifact.payload;
    if expected_artifact_sha256 != artifact.artifact_sha256 {
        return Err(format!(
            "VCET pre-final reduction artifact SHA differs from the expected authority (expected {expected_artifact_sha256}, declared {})",
            artifact.artifact_sha256
        ));
    }
    if payload.schema_version != VCET_TEMPORAL_PRE_FINAL_REDUCTION_ARTIFACT_SCHEMA_VERSION
        || payload.evidence_kind != "vcet_temporal_pre_final_reduction_artifact"
    {
        return Err(
            "unsupported VCET pre-final reduction artifact schema or evidence kind".to_string(),
        );
    }
    if payload.final_test_scoring_access_count != 0 {
        return Err(
            "VCET pre-final reduction must record zero final-test scoring accesses".to_string(),
        );
    }
    if payload.requested_selection_dataset_count == 0
        || payload.covered_selection_dataset_count != payload.requested_selection_dataset_count
        || payload.expected_selection_shard_count == 0
        || payload.expected_selection_shard_count != payload.reduced_selection_shard_count
        || payload.ordered_shards.len() != payload.reduced_selection_shard_count
        || payload.ordered_shards.len() > MAX_VCET_TEMPORAL_INLINE_PRE_FINAL_SHARD_FAN_IN
        || payload.candidates.is_empty()
        || payload.final_shard_authorizations.is_empty()
    {
        return Err(
            "VCET pre-final reduction cardinality or exact coverage evidence is invalid"
                .to_string(),
        );
    }
    if validate_final_stage_nonce(&payload.final_stage_nonce)? != payload.final_stage_nonce {
        return Err("VCET pre-final reduction final-stage nonce is not canonical".to_string());
    }
    let execution_spec_hash = execution_spec_sha256(&payload.execution_spec)?;
    if execution_spec_hash != payload.execution_spec_sha256
        || payload.execution_spec.selection_reduction_contract
            != payload.selection_reduction_contract
        || payload.selection_reduction_contract != "mean_validation_sharpe_total_cmp_v1"
    {
        return Err(
            "VCET pre-final reduction execution-spec or selection contract mismatch".to_string(),
        );
    }
    if payload.limitations != pre_final_reduction_limitations() {
        return Err("VCET pre-final reduction limitations are not canonical".to_string());
    }

    let mut seen_shard_ids = BTreeSet::new();
    let mut expected_trial_count = 0usize;
    for (index, shard) in payload.ordered_shards.iter().enumerate() {
        if shard.shard_ordinal != index
            || shard.shard_id.trim() != shard.shard_id
            || shard.shard_id.is_empty()
            || !seen_shard_ids.insert(shard.shard_id.clone())
            || shard.dataset_count == 0
            || shard.candidate_count == 0
            || shard.candidate_window_trial_count == 0
        {
            return Err(
                "VCET pre-final reduction shard ordering or evidence is invalid".to_string(),
            );
        }
        expected_trial_count = expected_trial_count
            .checked_add(shard.candidate_window_trial_count)
            .ok_or_else(|| "VCET pre-final reduction trial count overflow".to_string())?;
    }
    if expected_trial_count != payload.pre_final_candidate_window_trial_count {
        return Err("VCET pre-final reduction trial count is inconsistent".to_string());
    }

    let requested = payload.requested_selection_dataset_count;
    let mut seen_candidate_ids = BTreeSet::new();
    let mut selected_indices = Vec::new();
    for (index, candidate) in payload.candidates.iter().enumerate() {
        if candidate.global_candidate_ordinal != index
            || candidate.candidate_id.trim() != candidate.candidate_id
            || candidate.candidate_id.is_empty()
            || !seen_candidate_ids.insert(candidate.candidate_id.clone())
            || candidate.requested_dataset_count != requested
            || candidate.covered_dataset_count != requested
            || candidate.evaluated_dataset_count > requested
            || candidate.profitable_dataset_count > requested
            || candidate.hard_support_dataset_count > requested
            || candidate.metric_support_dataset_count > requested
            || !candidate.validation_sharpe_sum.is_finite()
        {
            return Err(
                "VCET pre-final global candidate identity or count evidence is invalid".to_string(),
            );
        }
        let expected_shares = [
            share(candidate.covered_dataset_count, requested),
            share(candidate.evaluated_dataset_count, requested),
            share(candidate.profitable_dataset_count, requested),
            share(candidate.hard_support_dataset_count, requested),
            share(candidate.metric_support_dataset_count, requested),
        ];
        let declared_shares = [
            candidate.requested_dataset_coverage_share,
            candidate.evaluated_dataset_share_requested,
            candidate.profitable_dataset_share_requested,
            candidate.hard_support_dataset_share_requested,
            candidate.metric_support_dataset_share_requested,
        ];
        if expected_shares
            .into_iter()
            .zip(declared_shares)
            .any(|(expected, declared)| {
                !declared.is_finite() || expected.map(f64::to_bits) != Some(declared.to_bits())
            })
            || candidate
                .validation_pnl_p10
                .is_some_and(|value| !value.is_finite())
            || candidate
                .validation_sharpe_p10
                .is_some_and(|value| !value.is_finite())
            || candidate
                .mean_validation_sharpe
                .is_some_and(|value| !value.is_finite())
        {
            return Err("VCET pre-final global candidate metric evidence is invalid".to_string());
        }
        let expected_mean = canonical_mean_from_sum_bits(
            candidate.validation_sharpe_sum.to_bits(),
            candidate.validation_observation_count,
        );
        if !same_optional_f64_bits(candidate.mean_validation_sharpe, expected_mean) {
            return Err(
                "VCET pre-final global candidate Sharpe sum/mean evidence is inconsistent"
                    .to_string(),
            );
        }
        let hard_support_is_complete = candidate.hard_support_dataset_count == requested;
        let metric_support_is_complete = candidate.metric_support_dataset_count == requested;
        if (candidate.support_failure_count == 0) != hard_support_is_complete
            || (candidate.non_finite_metric_count == 0) != metric_support_is_complete
        {
            return Err(
                "VCET pre-final global candidate raw failure counts are inconsistent with dataset support counts"
                    .to_string(),
            );
        }
        let admission = payload.execution_spec.cross_universe_admission;
        let expected_admission =
            global_candidate_meets_cross_universe_admission(candidate, &admission);
        if candidate.admission_passed != expected_admission {
            return Err(
                "VCET pre-final global candidate admission evidence is inconsistent".to_string(),
            );
        }
        if candidate.selected_before_final_test {
            selected_indices.push(index);
        }
    }
    if selected_indices.len() != 1 {
        return Err("VCET pre-final reduction must lock exactly one candidate".to_string());
    }

    let mut eligible = payload
        .candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.admission_passed)
        .map(|(index, candidate)| {
            candidate
                .mean_validation_sharpe
                .map(|score| (index, score, candidate.candidate_id.as_str()))
                .ok_or_else(|| "VCET admitted candidate is missing its selection score".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    eligible.sort_by(|left, right| right.1.total_cmp(&left.1).then_with(|| left.2.cmp(right.2)));
    let (winner_index, winner_score, _) = eligible
        .first()
        .copied()
        .ok_or_else(|| "VCET pre-final artifact has no admitted candidate".to_string())?;
    if selected_indices[0] != winner_index {
        return Err(
            "VCET pre-final selected candidate is not the deterministic admitted winner"
                .to_string(),
        );
    }
    let winner_candidate = &payload.candidates[winner_index];
    let winner = &payload.winner_lock;
    if winner.global_candidate_ordinal != winner_index
        || winner.candidate_id != winner_candidate.candidate_id
        || !serialized_eq(&winner.params, &winner_candidate.params)?
        || winner.selection_score.to_bits() != winner_score.to_bits()
        || winner.selection_policy != payload.execution_spec.selection_policy
        || winner.selection_reduction_contract != payload.selection_reduction_contract
        || winner.universe_sha256 != payload.universe_sha256
        || winner.candidate_manifest_sha256 != payload.candidate_manifest_sha256
        || winner.plan_sha256 != payload.plan_sha256
        || winner.execution_spec_sha256 != payload.execution_spec_sha256
        || winner.global_pre_final_evidence_sha256 != payload.global_pre_final_evidence_sha256
        || !winner.selection_score.is_finite()
    {
        return Err("VCET pre-final winner lock identity is inconsistent".to_string());
    }
    let runner = eligible.get(1).copied();
    let expected_runner_id = runner.map(|(_, _, id)| id);
    let expected_runner_score = runner.map(|(_, score, _)| score);
    let expected_tie =
        runner.is_some_and(|(_, score, _)| winner_score.total_cmp(&score) == Ordering::Equal);
    let expected_margin = runner.and_then(|(_, score, _)| {
        let margin = winner_score - score;
        margin.is_finite().then_some(margin)
    });
    if winner.runner_up_candidate_id.as_deref() != expected_runner_id
        || !same_optional_f64_bits(winner.runner_up_selection_score, expected_runner_score)
        || !same_optional_f64_bits(winner.winner_to_runner_up_score_margin, expected_margin)
        || winner.candidate_id_tie_break_used != expected_tie
        || recompute_winner_lock_sha256(winner)? != winner.winner_lock_sha256
    {
        return Err("VCET pre-final winner/runner-up lock evidence is invalid".to_string());
    }

    let global_hash = sha256_serializable(
        &GlobalEvidenceHashInput {
            hash_contract: PRE_FINAL_GLOBAL_EVIDENCE_HASH_CONTRACT,
            universe_sha256: &payload.universe_sha256,
            candidate_manifest_sha256: &payload.candidate_manifest_sha256,
            plan_sha256: &payload.plan_sha256,
            execution_spec_sha256: &payload.execution_spec_sha256,
            temporal_interval_fingerprint_sha256: &payload.temporal_interval_fingerprint_sha256,
            selection_reduction_contract: &payload.selection_reduction_contract,
            ordered_shards: &payload.ordered_shards,
            candidates: &payload.candidates,
            pre_final_candidate_window_trial_count: payload.pre_final_candidate_window_trial_count,
        },
        "VCET global pre-final evidence",
    )?;
    if global_hash != payload.global_pre_final_evidence_sha256 {
        return Err("VCET global pre-final evidence SHA mismatch".to_string());
    }

    let mut authorization_shard_ids = BTreeSet::new();
    for authorization in &payload.final_shard_authorizations {
        if authorization.shard.role != VcetTemporalDatasetCohortRoleV1::FinalTest
            || !authorization.shard.locked_winner_only
            || authorization.shard.dataset_ids.is_empty()
            || authorization.shard.candidate_start_index != 0
            || authorization.shard.candidate_end_index_exclusive != 0
            || authorization.shard.expected_candidate_window_trial_count
                != authorization.shard.dataset_ids.len()
            || !authorization_shard_ids.insert(authorization.shard.shard_id.clone())
            || authorization.universe_sha256 != payload.universe_sha256
            || authorization.winner_lock_sha256 != winner.winner_lock_sha256
            || authorization.global_candidate_ordinal != winner.global_candidate_ordinal
            || authorization.candidate_id != winner.candidate_id
            || authorization.global_pre_final_evidence_sha256
                != payload.global_pre_final_evidence_sha256
            || authorization.final_stage_nonce != payload.final_stage_nonce
        {
            return Err("VCET final-shard authorization set is inconsistent".to_string());
        }
        let prior = recompute_final_prior_state_sha256(
            &authorization.universe_sha256,
            &authorization.winner_lock_sha256,
            &authorization.global_pre_final_evidence_sha256,
            &authorization.final_stage_nonce,
            &authorization.shard,
        )?;
        if prior != authorization.expected_prior_state_sha256
            || recompute_final_authorization_sha256(authorization)?
                != authorization.authorization_sha256
        {
            return Err("VCET final-shard authorization hash evidence is invalid".to_string());
        }
    }

    let actual = recompute_vcet_temporal_pre_final_reduction_artifact_sha256(payload)?;
    if actual != artifact.artifact_sha256 {
        return Err(format!(
            "VCET pre-final reduction artifact SHA mismatch (declared {}, actual {actual})",
            artifact.artifact_sha256
        ));
    }
    Ok(())
}

pub(super) fn validate_vcet_temporal_pre_final_reduction_against_universe(
    response: &ReduceVcetTemporalPreFinalShardsResponseV1,
    sealed_universe: &VcetTemporalSealedDatasetUniverseV1,
) -> Result<(), String> {
    validate_sealed_vcet_temporal_universe(sealed_universe, &sealed_universe.universe_sha256)?;
    validate_vcet_temporal_pre_final_reduction_artifact(response, &response.artifact_sha256)?;
    let payload = &response.payload;
    let universe = &sealed_universe.payload;
    let execution_spec_sha256 = execution_spec_sha256(&universe.execution_spec)?;
    if payload.universe_sha256 != sealed_universe.universe_sha256
        || payload.candidate_manifest_sha256 != universe.candidate_manifest_sha256
        || payload.plan_sha256 != universe.plan_identity.plan_sha256
        || payload.execution_spec_sha256 != execution_spec_sha256
        || !serialized_eq(&payload.execution_spec, &universe.execution_spec)?
        || payload.temporal_interval_fingerprint_sha256
            != universe.temporal_interval_fingerprint_sha256
    {
        return Err(
            "VCET pre-final winner-lock response does not match the sealed universe authority"
                .to_string(),
        );
    }

    let expected_shards =
        universe_shards_for_role(universe, VcetTemporalDatasetCohortRoleV1::FinalTest);
    if payload.final_shard_authorizations.len() != expected_shards.len() {
        return Err(format!(
            "VCET pre-final winner-lock authorizes {} final shards, but the sealed universe requires {}",
            payload.final_shard_authorizations.len(),
            expected_shards.len()
        ));
    }
    let mut seen_shard_ids = BTreeSet::new();
    let mut seen_authorizations = BTreeSet::new();
    for (authorization, expected_shard) in payload
        .final_shard_authorizations
        .iter()
        .zip(expected_shards)
    {
        if !seen_shard_ids.insert(authorization.shard.shard_id.clone())
            || !seen_authorizations.insert(authorization.authorization_sha256.clone())
        {
            return Err(
                "VCET pre-final winner-lock contains duplicate final-shard authorization identity"
                    .to_string(),
            );
        }
        if !serialized_eq(&authorization.shard, expected_shard)? {
            return Err(format!(
                "VCET pre-final winner-lock authorization '{}' does not exactly match its sealed final-shard descriptor",
                authorization.shard.shard_id
            ));
        }
        validate_vcet_temporal_final_shard_authorization(
            authorization,
            sealed_universe,
            &payload.winner_lock,
        )?;
    }
    Ok(())
}

fn record_durable_pre_final_failure(claim: &DurableFinalClaim, error: String) -> String {
    match claim.fail(&error) {
        Ok(()) => error,
        Err(ledger_error) => format!(
            "{error}; additionally failed to publish the durable VCET pre-final winner-lock failure state: {ledger_error}"
        ),
    }
}

pub(super) fn reduce_vcet_temporal_pre_final_shards_durable_blocking(
    req: ReduceVcetTemporalPreFinalShardsRequestV1,
    app_local_data_dir: &Path,
) -> Result<ReduceVcetTemporalPreFinalShardsResponseV1, String> {
    let computed = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reduce_vcet_temporal_pre_final_shards_blocking(req.clone())
    })) {
        Ok(result) => result?,
        Err(_) => return Err("VCET pre-final reduction panicked before durable claim".to_string()),
    };
    validate_vcet_temporal_pre_final_reduction_against_universe(&computed, &req.sealed_universe)?;

    let universe_sha256 = req.sealed_universe.universe_sha256.clone();
    let request_sha256 = hash_enveloped(
        PRE_FINAL_REDUCTION_REQUEST_HASH_CONTRACT,
        &req,
        "VCET temporal pre-final reduction request",
    )?;
    match admit_durable_pre_final_winner_lock::<ReduceVcetTemporalPreFinalShardsResponseV1>(
        app_local_data_dir,
        &universe_sha256,
        &request_sha256,
    )? {
        DurableFinalAdmission::Replay(response) => {
            validate_vcet_temporal_pre_final_reduction_against_universe(
                &response,
                &req.sealed_universe,
            )?;
            if !serialized_eq(&response, &computed)? {
                return Err(
                    "VCET durable pre-final winner-lock replay differs from the freshly computed canonical response"
                        .to_string(),
                );
            }
            Ok(response)
        }
        DurableFinalAdmission::Acquired(claim) => {
            if let Err(error) = claim.complete(&computed) {
                return Err(record_durable_pre_final_failure(&claim, error));
            }
            Ok(computed)
        }
    }
}

#[tauri::command]
pub async fn reduce_vcet_temporal_pre_final_shards(
    req: ReduceVcetTemporalPreFinalShardsRequestV1,
    app: tauri::AppHandle,
) -> Result<ReduceVcetTemporalPreFinalShardsResponseV1, String> {
    let app_local_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        reduce_vcet_temporal_pre_final_shards_durable_blocking(req, &app_local_data_dir)
    })
    .await
    .map_err(|error| format!("VCET pre-final reduction task failed: {error}"))?
}

#[tauri::command]
pub async fn load_vcet_temporal_pre_final_winner_lock(
    req: LoadVcetTemporalPreFinalWinnerLockRequestV1,
    app: tauri::AppHandle,
) -> Result<ReduceVcetTemporalPreFinalShardsResponseV1, String> {
    let app_local_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let response = load_durable_pre_final_winner_lock::<
            ReduceVcetTemporalPreFinalShardsResponseV1,
        >(&app_local_data_dir, &req.universe_sha256)?;
        validate_vcet_temporal_pre_final_reduction_artifact(&response, &response.artifact_sha256)?;
        let requested = req
            .universe_sha256
            .strip_prefix("sha256:")
            .unwrap_or(&req.universe_sha256);
        let observed = response
            .payload
            .universe_sha256
            .strip_prefix("sha256:")
            .unwrap_or(&response.payload.universe_sha256);
        if !requested.eq_ignore_ascii_case(observed) {
            return Err(
                "VCET durable pre-final winner-lock payload does not match the requested universe"
                    .to_string(),
            );
        }
        Ok(response)
    })
    .await
    .map_err(|error| format!("VCET pre-final winner-lock recovery task failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::super::commands_vcet_temporal::{
        seal_vcet_temporal_candidate_manifest, VcetTemporalDatasetTimelineInputV1,
    };
    use super::super::commands_vcet_temporal_universe::{
        recompute_vcet_dataset_attestation_sha256, seal_vcet_temporal_dataset_universe_blocking,
        SealVcetTemporalDatasetUniverseRequestV1, VcetTemporalShardPolicyV1,
        VcetTemporalUniverseDatasetV1,
    };
    use super::*;
    use ta_app_core::vcet_dataset_coverage::{
        profile_vcet_dataset_coverage, VcetDatasetCoverageOptions, VcetRequestedCoverageBoundsMs,
    };
    use ta_app_core::vcet_search::VcetCandidateIdentityInputs;
    use ta_app_core::vcet_temporal::{
        vcet_temporal_plan_identity, VcetTemporalFoldModeV1, VcetTemporalMinimumSupportV1,
        VcetTemporalRequestedBoundsV1, VcetTemporalUtcBoundsV1, VcetTemporalValidationPlanV1,
        VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION,
    };
    use ta_strategies::double_ma::StrategyConfig;

    fn hash(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn durable_temp_dir(label: &str) -> std::path::PathBuf {
        let sequence = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "vectorgrid_vcet_prefinal_{label}_{}_{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create durable test directory");
        dir
    }

    fn candidate_identity() -> VcetCandidateIdentityInputs {
        VcetCandidateIdentityInputs {
            vidya_short_period: 10,
            vidya_long_period: 30,
            vidya_alpha_micros: 200_000,
            ce_period: 20,
            ce_mult_micros: 3_000_000,
            ce_use_close: true,
            require_vidya_slope: true,
            max_trade_bars: 40,
        }
    }

    fn params() -> CompositeParams {
        candidate_params_from_manifest(&candidate_identity(), &StrategyConfig::default())
    }

    fn execution_spec() -> VcetTemporalPhasedExecutionSpecV1 {
        VcetTemporalPhasedExecutionSpecV1 {
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            strategy: StrategyConfig::default(),
            selection_policy:
                super::super::commands_vcet_temporal_execute::VcetTemporalSelectionPolicyV1::MeanValidationSharpe,
            selection_reduction_contract: "mean_validation_sharpe_total_cmp_v1".to_string(),
            cross_universe_admission:
                super::super::commands_vcet_temporal_universe::VcetTemporalCrossUniverseAdmissionV1 {
                    schema_version: 1,
                    min_requested_dataset_coverage_share: 1.0,
                    min_evaluated_dataset_share_requested: 1.0,
                    min_profitable_dataset_share: 0.0,
                    min_hard_support_dataset_share: 1.0,
                    min_metric_support_dataset_share: 1.0,
                    min_validation_pnl_p10: -100.0,
                    min_validation_sharpe_p10: -100.0,
                },
        }
    }

    fn temporal_plan() -> VcetTemporalValidationPlanV1 {
        VcetTemporalValidationPlanV1 {
            schema_version: VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION,
            requested_bounds: VcetTemporalRequestedBoundsV1 {
                utc: VcetTemporalUtcBoundsV1 {
                    start_utc_ms_inclusive: 0,
                    end_utc_ms_inclusive: 1_000_000,
                },
                session: None,
            },
            fold_mode: VcetTemporalFoldModeV1::Anchored,
            train_bars: 100,
            validation_bars: 20,
            final_test_bars: 30,
            step_bars: 20,
            purge_bars: 2,
            embargo_bars: 0,
            warmup_bars: 50,
            position_boundary_policy:
                ta_app_core::vcet_temporal::VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen,
            minimum_support: VcetTemporalMinimumSupportV1 {
                min_folds: 2,
                min_train_bars: 100,
                min_validation_bars: 20,
                min_final_test_bars: 30,
                min_train_trades_per_fold: 1,
                min_validation_trades_per_fold: 1,
                min_final_test_trades: 1,
            },
        }
    }

    fn valid_shard_artifact() -> ExecuteVcetTemporalPreFinalShardResponseV1 {
        let spec = execution_spec();
        let candidate_params = params();
        let dataset_candidate = VcetTemporalPreFinalDatasetCandidateEvidenceV1 {
            global_candidate_ordinal: 0,
            candidate_id: "candidate-0".to_string(),
            validation_fold_count: 2,
            finite_validation_observation_count: 2,
            validation_pnl_sum: 2.0,
            validation_sharpe_sum: 4.0,
            mean_validation_pnl: Some(1.0),
            mean_validation_sharpe: Some(2.0),
            total_train_trades: 8,
            total_validation_trades: 4,
            support_failure_count: 0,
            support_failure_examples: Vec::new(),
            non_finite_metric_count: 0,
            hard_support_passed: true,
            metric_support_passed: true,
        };
        let coverage =
            profile_vcet_dataset_coverage(&[0, 60_000], VcetDatasetCoverageOptions::default())
                .expect("coverage");
        let descriptor = VcetTemporalUniverseShardV1 {
            role: VcetTemporalDatasetCohortRoleV1::Selection,
            shard_ordinal: 0,
            shard_id: "selection-0".to_string(),
            dataset_ids: vec!["dataset-0".to_string()],
            candidate_start_index: 0,
            candidate_end_index_exclusive: 1,
            locked_winner_only: false,
            expected_candidate_window_trial_count: 4,
        };
        let payload = VcetTemporalPreFinalShardArtifactPayloadV1 {
            schema_version: VCET_TEMPORAL_PRE_FINAL_SHARD_ARTIFACT_SCHEMA_VERSION,
            evidence_kind: "vcet_temporal_pre_final_shard_artifact".to_string(),
            universe_sha256: hash('a'),
            candidate_manifest_sha256: hash('b'),
            plan_sha256: hash('c'),
            execution_spec_sha256: execution_spec_sha256(&spec).expect("spec hash"),
            execution_spec: spec,
            temporal_interval_fingerprint_sha256: hash('d'),
            selection_shard: descriptor,
            planning_evidence_sha256: hash('e'),
            dataset_count: 1,
            candidate_count: 1,
            fold_count_per_dataset: 2,
            expected_candidate_window_trial_count: 4,
            evaluated_candidate_window_trial_count: 4,
            final_test_scoring_access_count: 0,
            candidates: vec![VcetTemporalPreFinalShardCandidateAggregateV1 {
                global_candidate_ordinal: 0,
                candidate_id: "candidate-0".to_string(),
                params: candidate_params,
                validation_sharpe_sum: 4.0,
                validation_observation_count: 2,
                total_train_trades: 8,
                total_validation_trades: 4,
                support_failure_count: 0,
                non_finite_metric_count: 0,
            }],
            datasets: vec![VcetTemporalPreFinalDatasetEvidenceV1 {
                dataset_id: "dataset-0".to_string(),
                loaded_data_id: "loaded-0".to_string(),
                source_dataset_sha256: hash('f'),
                dataset_attestation_sha256: hash('1'),
                timeline_sha256: hash('2'),
                loaded_candles_snapshot_sha256: hash('3'),
                dataset_coverage: coverage,
                dataset_coverage_sha256: hash('4'),
                temporal_interval_fingerprint_sha256: hash('d'),
                fold_count: 2,
                candidates: vec![dataset_candidate],
            }],
            limitations: pre_final_shard_limitations(),
        };
        ExecuteVcetTemporalPreFinalShardResponseV1 {
            artifact_sha256: recompute_vcet_temporal_pre_final_shard_artifact_sha256(&payload)
                .expect("artifact hash"),
            payload,
        }
    }

    fn sealed_universe_reduction_fixture() -> (
        VcetTemporalSealedDatasetUniverseV1,
        ExecuteVcetTemporalPreFinalShardResponseV1,
        VcetTemporalUniverseShardV1,
    ) {
        let manifest = seal_vcet_temporal_candidate_manifest(
            hash('a'),
            hash('b'),
            None,
            vec![candidate_identity()],
        )
        .expect("candidate manifest");
        let plan = temporal_plan();
        let plan_identity = vcet_temporal_plan_identity(&plan).expect("plan identity");
        let timestamps = (0..=1_000).map(|index| index * 1_000).collect::<Vec<_>>();
        let planning_request = PlanVcetTemporalValidationRequestV1 {
            sealed_candidate_manifest: manifest.clone(),
            expected_candidate_manifest_sha256: manifest.manifest_sha256.clone(),
            plan: plan.clone(),
            expected_plan_identity: plan_identity.clone(),
            datasets: vec![
                VcetTemporalDatasetTimelineInputV1 {
                    dataset_id: "selection-dataset".to_string(),
                    source_dataset_sha256: hash('c'),
                    timestamps_utc_ms: timestamps.clone(),
                    session_calendar_id: None,
                    session_open_utc_ms: None,
                },
                VcetTemporalDatasetTimelineInputV1 {
                    dataset_id: "final-dataset".to_string(),
                    source_dataset_sha256: hash('d'),
                    timestamps_utc_ms: timestamps.clone(),
                    session_calendar_id: None,
                    session_open_utc_ms: None,
                },
            ],
        };
        let planning =
            plan_vcet_temporal_validation_blocking(planning_request).expect("canonical planning");
        let coverage = profile_vcet_dataset_coverage(
            &timestamps,
            VcetDatasetCoverageOptions {
                requested_bounds: Some(VcetRequestedCoverageBoundsMs {
                    start_ms: 0,
                    end_exclusive_ms: 1_000_001,
                }),
                expected_cadence_ms: Some(1_000),
                session_interval_mask: None,
            },
        )
        .expect("coverage");
        let universe_datasets = [
            (
                "selection-dataset",
                hash('c'),
                hash('e'),
                VcetTemporalDatasetCohortRoleV1::Selection,
            ),
            (
                "final-dataset",
                hash('d'),
                hash('f'),
                VcetTemporalDatasetCohortRoleV1::FinalTest,
            ),
        ]
        .into_iter()
        .map(|(dataset_id, source_sha256, snapshot_sha256, role)| {
            let planned = planning
                .datasets
                .iter()
                .find(|dataset| dataset.dataset_id == dataset_id)
                .expect("planned dataset");
            VcetTemporalUniverseDatasetV1 {
                dataset_id: dataset_id.to_string(),
                source_dataset_sha256: source_sha256.clone(),
                loaded_candles_snapshot_sha256: snapshot_sha256.clone(),
                dataset_attestation_sha256: recompute_vcet_dataset_attestation_sha256(
                    dataset_id,
                    &source_sha256,
                    &snapshot_sha256,
                    role,
                )
                .expect("row attestation"),
                timeline_sha256: planned.timeline_sha256.clone(),
                temporal_interval_fingerprint_sha256:
                    recompute_vcet_temporal_interval_fingerprint_sha256(&planned.layout)
                        .expect("interval fingerprint"),
                dataset_coverage: coverage.clone(),
                dataset_coverage_sha256: String::new(),
                fold_count: planned.fold_count,
                role,
            }
        })
        .collect::<Vec<_>>();
        let sealed = seal_vcet_temporal_dataset_universe_blocking(
            SealVcetTemporalDatasetUniverseRequestV1 {
                sealed_candidate_manifest: manifest,
                expected_candidate_manifest_sha256: planning
                    .candidate_manifest
                    .manifest_sha256
                    .clone(),
                plan,
                expected_plan_identity: plan_identity,
                execution_spec: execution_spec(),
                shard_policy: VcetTemporalShardPolicyV1::default(),
                datasets: universe_datasets,
            },
        )
        .expect("sealed universe")
        .sealed_universe;
        let selection_shard =
            universe_shards_for_role(&sealed.payload, VcetTemporalDatasetCohortRoleV1::Selection)
                .into_iter()
                .next()
                .expect("selection shard")
                .clone();
        let final_shard =
            universe_shards_for_role(&sealed.payload, VcetTemporalDatasetCohortRoleV1::FinalTest)
                .into_iter()
                .next()
                .expect("final shard")
                .clone();
        let selection_dataset = sealed
            .payload
            .datasets
            .iter()
            .find(|dataset| dataset.role == VcetTemporalDatasetCohortRoleV1::Selection)
            .expect("selection dataset");
        let candidate = &sealed.payload.candidate_manifest.payload.candidates[0];
        let candidate_params = candidate_params_from_manifest(
            &candidate.identity_inputs,
            &sealed.payload.execution_spec.strategy,
        );
        let fold_count = selection_dataset.fold_count;
        let fold_count_u64 = u64::try_from(fold_count).expect("fold count fits u64");
        let dataset_candidate = VcetTemporalPreFinalDatasetCandidateEvidenceV1 {
            global_candidate_ordinal: 0,
            candidate_id: candidate.candidate_id.clone(),
            validation_fold_count: fold_count,
            finite_validation_observation_count: fold_count,
            validation_pnl_sum: fold_count as f64,
            validation_sharpe_sum: fold_count as f64 * 2.0,
            mean_validation_pnl: Some(1.0),
            mean_validation_sharpe: Some(2.0),
            total_train_trades: fold_count_u64 * 2,
            total_validation_trades: fold_count_u64,
            support_failure_count: 0,
            support_failure_examples: Vec::new(),
            non_finite_metric_count: 0,
            hard_support_passed: true,
            metric_support_passed: true,
        };
        let execution_spec_sha256 =
            execution_spec_sha256(&sealed.payload.execution_spec).expect("execution spec hash");
        let payload = VcetTemporalPreFinalShardArtifactPayloadV1 {
            schema_version: VCET_TEMPORAL_PRE_FINAL_SHARD_ARTIFACT_SCHEMA_VERSION,
            evidence_kind: "vcet_temporal_pre_final_shard_artifact".to_string(),
            universe_sha256: sealed.universe_sha256.clone(),
            candidate_manifest_sha256: sealed.payload.candidate_manifest_sha256.clone(),
            plan_sha256: sealed.payload.plan_identity.plan_sha256.clone(),
            execution_spec_sha256,
            execution_spec: sealed.payload.execution_spec.clone(),
            temporal_interval_fingerprint_sha256: sealed
                .payload
                .temporal_interval_fingerprint_sha256
                .clone(),
            selection_shard,
            planning_evidence_sha256: planning.evidence_sha256,
            dataset_count: 1,
            candidate_count: 1,
            fold_count_per_dataset: fold_count,
            expected_candidate_window_trial_count: fold_count * 2,
            evaluated_candidate_window_trial_count: fold_count * 2,
            final_test_scoring_access_count: 0,
            candidates: vec![VcetTemporalPreFinalShardCandidateAggregateV1 {
                global_candidate_ordinal: 0,
                candidate_id: candidate.candidate_id.clone(),
                params: candidate_params,
                validation_sharpe_sum: fold_count as f64 * 2.0,
                validation_observation_count: fold_count,
                total_train_trades: fold_count_u64 * 2,
                total_validation_trades: fold_count_u64,
                support_failure_count: 0,
                non_finite_metric_count: 0,
            }],
            datasets: vec![VcetTemporalPreFinalDatasetEvidenceV1 {
                dataset_id: selection_dataset.dataset_id.clone(),
                loaded_data_id: "loaded-selection-dataset".to_string(),
                source_dataset_sha256: selection_dataset.source_dataset_sha256.clone(),
                dataset_attestation_sha256: selection_dataset.dataset_attestation_sha256.clone(),
                timeline_sha256: selection_dataset.timeline_sha256.clone(),
                loaded_candles_snapshot_sha256: selection_dataset
                    .loaded_candles_snapshot_sha256
                    .clone(),
                dataset_coverage: selection_dataset.dataset_coverage.clone(),
                dataset_coverage_sha256: selection_dataset.dataset_coverage_sha256.clone(),
                temporal_interval_fingerprint_sha256: selection_dataset
                    .temporal_interval_fingerprint_sha256
                    .clone(),
                fold_count,
                candidates: vec![dataset_candidate],
            }],
            limitations: pre_final_shard_limitations(),
        };
        let artifact = ExecuteVcetTemporalPreFinalShardResponseV1 {
            artifact_sha256: recompute_vcet_temporal_pre_final_shard_artifact_sha256(&payload)
                .expect("shard artifact hash"),
            payload,
        };
        (sealed, artifact, final_shard)
    }

    pub(crate) fn temporal_report_pre_final_fixture() -> (
        VcetTemporalSealedDatasetUniverseV1,
        ReduceVcetTemporalPreFinalShardsResponseV1,
    ) {
        let (sealed_universe, selection_artifact, _) = sealed_universe_reduction_fixture();
        let reduction = reduce_vcet_temporal_pre_final_shards_blocking(
            ReduceVcetTemporalPreFinalShardsRequestV1 {
                expected_universe_sha256: sealed_universe.universe_sha256.clone(),
                sealed_universe: sealed_universe.clone(),
                final_stage_nonce: "temporal-report-fixture-final-stage-0001".to_string(),
                pre_final_shard_artifacts: vec![selection_artifact],
            },
        )
        .expect("reduce temporal-report pre-final fixture");
        validate_vcet_temporal_pre_final_reduction_against_universe(&reduction, &sealed_universe)
            .expect("validate temporal-report pre-final fixture");
        (sealed_universe, reduction)
    }

    fn valid_reduction_artifact() -> ReduceVcetTemporalPreFinalShardsResponseV1 {
        let spec = execution_spec();
        let spec_sha256 = execution_spec_sha256(&spec).expect("spec hash");
        let ordered_shards = vec![VcetTemporalPreFinalShardReductionEvidenceV1 {
            shard_ordinal: 0,
            shard_id: "selection-0".to_string(),
            artifact_sha256: hash('e'),
            dataset_count: 1,
            candidate_count: 1,
            candidate_window_trial_count: 4,
        }];
        let candidates = vec![VcetTemporalGlobalCandidateEvidenceV1 {
            global_candidate_ordinal: 0,
            candidate_id: "candidate-0".to_string(),
            params: params(),
            requested_dataset_count: 1,
            covered_dataset_count: 1,
            evaluated_dataset_count: 1,
            profitable_dataset_count: 1,
            hard_support_dataset_count: 1,
            metric_support_dataset_count: 1,
            requested_dataset_coverage_share: 1.0,
            evaluated_dataset_share_requested: 1.0,
            profitable_dataset_share_requested: 1.0,
            hard_support_dataset_share_requested: 1.0,
            metric_support_dataset_share_requested: 1.0,
            validation_pnl_p10: Some(1.0),
            validation_sharpe_p10: Some(2.0),
            validation_sharpe_sum: 4.0,
            mean_validation_sharpe: Some(2.0),
            validation_observation_count: 2,
            total_train_trades: 8,
            total_validation_trades: 4,
            support_failure_count: 0,
            non_finite_metric_count: 0,
            admission_passed: true,
            selected_before_final_test: true,
        }];
        let universe_sha256 = hash('a');
        let candidate_manifest_sha256 = hash('b');
        let plan_sha256 = hash('c');
        let interval_sha256 = hash('d');
        let global_pre_final_evidence_sha256 = sha256_serializable(
            &GlobalEvidenceHashInput {
                hash_contract: PRE_FINAL_GLOBAL_EVIDENCE_HASH_CONTRACT,
                universe_sha256: &universe_sha256,
                candidate_manifest_sha256: &candidate_manifest_sha256,
                plan_sha256: &plan_sha256,
                execution_spec_sha256: &spec_sha256,
                temporal_interval_fingerprint_sha256: &interval_sha256,
                selection_reduction_contract: &spec.selection_reduction_contract,
                ordered_shards: &ordered_shards,
                candidates: &candidates,
                pre_final_candidate_window_trial_count: 4,
            },
            "test global evidence",
        )
        .expect("global hash");
        let mut winner_lock = VcetTemporalGlobalWinnerLockV1 {
            global_candidate_ordinal: 0,
            candidate_id: "candidate-0".to_string(),
            params: params(),
            selection_policy: spec.selection_policy,
            selection_score: 2.0,
            runner_up_candidate_id: None,
            runner_up_selection_score: None,
            winner_to_runner_up_score_margin: None,
            candidate_id_tie_break_used: false,
            selection_reduction_contract: spec.selection_reduction_contract.clone(),
            universe_sha256: universe_sha256.clone(),
            candidate_manifest_sha256: candidate_manifest_sha256.clone(),
            plan_sha256: plan_sha256.clone(),
            execution_spec_sha256: spec_sha256.clone(),
            global_pre_final_evidence_sha256: global_pre_final_evidence_sha256.clone(),
            winner_lock_sha256: String::new(),
        };
        winner_lock.winner_lock_sha256 =
            recompute_winner_lock_sha256(&winner_lock).expect("winner hash");
        let final_stage_nonce = "nonce-for-unit-test-0001".to_string();
        let final_shard = VcetTemporalUniverseShardV1 {
            role: VcetTemporalDatasetCohortRoleV1::FinalTest,
            shard_ordinal: 0,
            shard_id: "final-0".to_string(),
            dataset_ids: vec!["final-dataset-0".to_string()],
            candidate_start_index: 0,
            candidate_end_index_exclusive: 0,
            locked_winner_only: true,
            expected_candidate_window_trial_count: 1,
        };
        let prior = recompute_final_prior_state_sha256(
            &universe_sha256,
            &winner_lock.winner_lock_sha256,
            &global_pre_final_evidence_sha256,
            &final_stage_nonce,
            &final_shard,
        )
        .expect("prior hash");
        let mut authorization = VcetTemporalFinalShardAuthorizationV1 {
            shard: final_shard,
            universe_sha256: universe_sha256.clone(),
            winner_lock_sha256: winner_lock.winner_lock_sha256.clone(),
            global_candidate_ordinal: 0,
            candidate_id: "candidate-0".to_string(),
            global_pre_final_evidence_sha256: global_pre_final_evidence_sha256.clone(),
            final_stage_nonce: final_stage_nonce.clone(),
            expected_prior_state_sha256: prior,
            authorization_sha256: String::new(),
        };
        authorization.authorization_sha256 =
            recompute_final_authorization_sha256(&authorization).expect("authorization hash");
        let payload = VcetTemporalPreFinalReductionArtifactPayloadV1 {
            schema_version: VCET_TEMPORAL_PRE_FINAL_REDUCTION_ARTIFACT_SCHEMA_VERSION,
            evidence_kind: "vcet_temporal_pre_final_reduction_artifact".to_string(),
            universe_sha256,
            candidate_manifest_sha256,
            plan_sha256,
            execution_spec_sha256: spec_sha256,
            execution_spec: spec,
            temporal_interval_fingerprint_sha256: interval_sha256,
            selection_reduction_contract: "mean_validation_sharpe_total_cmp_v1".to_string(),
            global_pre_final_evidence_sha256,
            requested_selection_dataset_count: 1,
            covered_selection_dataset_count: 1,
            expected_selection_shard_count: 1,
            reduced_selection_shard_count: 1,
            pre_final_candidate_window_trial_count: 4,
            final_test_scoring_access_count: 0,
            ordered_shards,
            candidates,
            winner_lock,
            final_stage_nonce,
            final_shard_authorizations: vec![authorization],
            limitations: pre_final_reduction_limitations(),
        };
        ReduceVcetTemporalPreFinalShardsResponseV1 {
            artifact_sha256: recompute_vcet_temporal_pre_final_reduction_artifact_sha256(&payload)
                .expect("artifact hash"),
            payload,
        }
    }

    fn rehash_reduction_lineage_for_test(
        artifact: &mut ReduceVcetTemporalPreFinalShardsResponseV1,
    ) {
        artifact.payload.execution_spec_sha256 =
            execution_spec_sha256(&artifact.payload.execution_spec).expect("execution spec hash");
        artifact.payload.global_pre_final_evidence_sha256 = sha256_serializable(
            &GlobalEvidenceHashInput {
                hash_contract: PRE_FINAL_GLOBAL_EVIDENCE_HASH_CONTRACT,
                universe_sha256: &artifact.payload.universe_sha256,
                candidate_manifest_sha256: &artifact.payload.candidate_manifest_sha256,
                plan_sha256: &artifact.payload.plan_sha256,
                execution_spec_sha256: &artifact.payload.execution_spec_sha256,
                temporal_interval_fingerprint_sha256: &artifact
                    .payload
                    .temporal_interval_fingerprint_sha256,
                selection_reduction_contract: &artifact.payload.selection_reduction_contract,
                ordered_shards: &artifact.payload.ordered_shards,
                candidates: &artifact.payload.candidates,
                pre_final_candidate_window_trial_count: artifact
                    .payload
                    .pre_final_candidate_window_trial_count,
            },
            "test global evidence",
        )
        .expect("global evidence hash");

        let winner_candidate = artifact.payload.candidates[0].clone();
        let winner = &mut artifact.payload.winner_lock;
        winner.global_candidate_ordinal = winner_candidate.global_candidate_ordinal;
        winner.candidate_id = winner_candidate.candidate_id.clone();
        winner.params = winner_candidate.params;
        winner.selection_policy = artifact.payload.execution_spec.selection_policy;
        winner.selection_score = winner_candidate
            .mean_validation_sharpe
            .expect("test winner score");
        winner.runner_up_candidate_id = None;
        winner.runner_up_selection_score = None;
        winner.winner_to_runner_up_score_margin = None;
        winner.candidate_id_tie_break_used = false;
        winner.selection_reduction_contract = artifact.payload.selection_reduction_contract.clone();
        winner.execution_spec_sha256 = artifact.payload.execution_spec_sha256.clone();
        winner.global_pre_final_evidence_sha256 =
            artifact.payload.global_pre_final_evidence_sha256.clone();
        winner.winner_lock_sha256 = recompute_winner_lock_sha256(winner).expect("winner-lock hash");

        for authorization in &mut artifact.payload.final_shard_authorizations {
            authorization.winner_lock_sha256 = winner.winner_lock_sha256.clone();
            authorization.global_candidate_ordinal = winner.global_candidate_ordinal;
            authorization.candidate_id = winner.candidate_id.clone();
            authorization.global_pre_final_evidence_sha256 =
                artifact.payload.global_pre_final_evidence_sha256.clone();
            authorization.expected_prior_state_sha256 = recompute_final_prior_state_sha256(
                &authorization.universe_sha256,
                &authorization.winner_lock_sha256,
                &authorization.global_pre_final_evidence_sha256,
                &authorization.final_stage_nonce,
                &authorization.shard,
            )
            .expect("final prior-state hash");
            authorization.authorization_sha256 =
                recompute_final_authorization_sha256(authorization)
                    .expect("final authorization hash");
        }
        artifact.artifact_sha256 =
            recompute_vcet_temporal_pre_final_reduction_artifact_sha256(&artifact.payload)
                .expect("reduction artifact hash");
    }

    fn partial_support_reduction_artifact() -> ReduceVcetTemporalPreFinalShardsResponseV1 {
        let mut artifact = valid_reduction_artifact();
        artifact.payload.requested_selection_dataset_count = 2;
        artifact.payload.covered_selection_dataset_count = 2;
        artifact.payload.ordered_shards[0].dataset_count = 2;
        artifact.payload.ordered_shards[0].candidate_window_trial_count = 8;
        artifact.payload.pre_final_candidate_window_trial_count = 8;
        artifact
            .payload
            .execution_spec
            .cross_universe_admission
            .min_hard_support_dataset_share = 0.5;
        artifact
            .payload
            .execution_spec
            .cross_universe_admission
            .min_metric_support_dataset_share = 0.5;
        let candidate = &mut artifact.payload.candidates[0];
        candidate.requested_dataset_count = 2;
        candidate.covered_dataset_count = 2;
        candidate.evaluated_dataset_count = 2;
        candidate.profitable_dataset_count = 2;
        candidate.hard_support_dataset_count = 1;
        candidate.metric_support_dataset_count = 1;
        candidate.requested_dataset_coverage_share = 1.0;
        candidate.evaluated_dataset_share_requested = 1.0;
        candidate.profitable_dataset_share_requested = 1.0;
        candidate.hard_support_dataset_share_requested = 0.5;
        candidate.metric_support_dataset_share_requested = 0.5;
        candidate.validation_sharpe_sum = 6.0;
        candidate.validation_observation_count = 3;
        candidate.total_train_trades = 12;
        candidate.total_validation_trades = 6;
        candidate.support_failure_count = 1;
        candidate.non_finite_metric_count = 1;
        candidate.admission_passed = true;
        rehash_reduction_lineage_for_test(&mut artifact);
        artifact
    }

    #[test]
    fn three_observation_means_are_derived_from_the_exact_stored_sum_bits() {
        let pnl_sum_bits = 0x3fa7_c208_8b0e_e350;
        let sharpe_sum_bits = 0x3fd4_b3f8_b104_c427;
        let evidence = dataset_candidate_evidence(
            0,
            "candidate-0".to_string(),
            DatasetCandidateAccumulator {
                pnl_sum: f64::from_bits(pnl_sum_bits),
                sharpe_sum: f64::from_bits(sharpe_sum_bits),
                finite_count: 3,
                validation_fold_count: 3,
                ..DatasetCandidateAccumulator::default()
            },
        );

        assert_eq!(evidence.validation_pnl_sum.to_bits(), pnl_sum_bits);
        assert_eq!(evidence.validation_sharpe_sum.to_bits(), sharpe_sum_bits);
        assert_eq!(
            evidence.mean_validation_pnl.map(f64::to_bits),
            Some(0x3f8f_ad60_b969_2f15)
        );
        assert_eq!(
            evidence.mean_validation_sharpe.map(f64::to_bits),
            Some(0x3fbb_9aa0_ec06_5adf)
        );
        assert!(same_optional_f64_bits(
            evidence.mean_validation_pnl,
            canonical_mean_from_sum_bits(evidence.validation_pnl_sum.to_bits(), 3)
        ));
        assert!(same_optional_f64_bits(
            evidence.mean_validation_sharpe,
            canonical_mean_from_sum_bits(evidence.validation_sharpe_sum.to_bits(), 3)
        ));
    }

    #[test]
    fn serde_json_preserves_prefinal_mean_bits() {
        let value = f64::from_bits(0x3fbb_9aa0_ec06_5adf);
        let encoded = serde_json::to_vec(&value).expect("serialize pre-final mean");
        let decoded: f64 = serde_json::from_slice(&encoded).expect("deserialize pre-final mean");

        assert_eq!(decoded.to_bits(), value.to_bits());
    }

    #[test]
    fn shard_json_round_trip_preserves_three_fold_sum_mean_contract() {
        let pnl_sum_bits = 0x3fa7_c208_8b0e_e350;
        let sharpe_sum_bits = 0x3fd4_b3f8_b104_c427;
        let mut artifact = valid_shard_artifact();
        artifact
            .payload
            .selection_shard
            .expected_candidate_window_trial_count = 6;
        artifact.payload.fold_count_per_dataset = 3;
        artifact.payload.expected_candidate_window_trial_count = 6;
        artifact.payload.evaluated_candidate_window_trial_count = 6;

        let dataset = &mut artifact.payload.datasets[0];
        dataset.fold_count = 3;
        let candidate = &mut dataset.candidates[0];
        candidate.validation_fold_count = 3;
        candidate.finite_validation_observation_count = 3;
        candidate.validation_pnl_sum = f64::from_bits(pnl_sum_bits);
        candidate.validation_sharpe_sum = f64::from_bits(sharpe_sum_bits);
        candidate.mean_validation_pnl = canonical_mean_from_sum_bits(pnl_sum_bits, 3);
        candidate.mean_validation_sharpe = canonical_mean_from_sum_bits(sharpe_sum_bits, 3);

        let aggregate = &mut artifact.payload.candidates[0];
        aggregate.validation_sharpe_sum = f64::from_bits(sharpe_sum_bits);
        aggregate.validation_observation_count = 3;
        artifact.artifact_sha256 =
            recompute_vcet_temporal_pre_final_shard_artifact_sha256(&artifact.payload)
                .expect("three-fold artifact hash");

        validate_vcet_temporal_pre_final_shard_artifact(&artifact)
            .expect("valid in-memory three-fold artifact");
        let encoded = serde_json::to_vec(&artifact).expect("serialize three-fold shard artifact");
        let decoded: ExecuteVcetTemporalPreFinalShardResponseV1 =
            serde_json::from_slice(&encoded).expect("deserialize three-fold shard artifact");

        let decoded_candidate = &decoded.payload.datasets[0].candidates[0];
        assert_eq!(decoded_candidate.validation_pnl_sum.to_bits(), pnl_sum_bits);
        assert_eq!(
            decoded_candidate.validation_sharpe_sum.to_bits(),
            sharpe_sum_bits
        );
        assert_eq!(
            decoded_candidate.mean_validation_pnl.map(f64::to_bits),
            Some(0x3f8f_ad60_b969_2f15)
        );
        assert_eq!(
            decoded_candidate.mean_validation_sharpe.map(f64::to_bits),
            Some(0x3fbb_9aa0_ec06_5adf)
        );
        validate_vcet_temporal_pre_final_shard_artifact(&decoded)
            .expect("valid round-tripped three-fold artifact");

        let mut tampered = decoded;
        tampered.payload.datasets[0].candidates[0].mean_validation_sharpe =
            Some(f64::from_bits(0x3fbb_9aa0_ec06_5ade));
        tampered.artifact_sha256 =
            recompute_vcet_temporal_pre_final_shard_artifact_sha256(&tampered.payload)
                .expect("tampered artifact hash");
        assert!(validate_vcet_temporal_pre_final_shard_artifact(&tampered)
            .unwrap_err()
            .contains("sum/mean"));
    }

    #[test]
    fn shard_artifact_validator_rejects_semantic_tamper_even_when_rehashed() {
        let mut artifact = valid_shard_artifact();
        validate_vcet_temporal_pre_final_shard_artifact(&artifact).expect("valid artifact");
        artifact.payload.datasets[0].candidates[0].mean_validation_sharpe = Some(9.0);
        artifact.artifact_sha256 =
            recompute_vcet_temporal_pre_final_shard_artifact_sha256(&artifact.payload)
                .expect("tampered hash");
        assert!(validate_vcet_temporal_pre_final_shard_artifact(&artifact)
            .unwrap_err()
            .contains("sum/mean"));
    }

    #[test]
    fn shard_artifact_validator_rejects_reversed_candidate_interval_without_panicking() {
        let mut artifact = valid_shard_artifact();
        artifact.payload.selection_shard.candidate_start_index = 1;
        artifact
            .payload
            .selection_shard
            .candidate_end_index_exclusive = 0;
        artifact.artifact_sha256 =
            recompute_vcet_temporal_pre_final_shard_artifact_sha256(&artifact.payload)
                .expect("tampered hash");
        assert!(validate_vcet_temporal_pre_final_shard_artifact(&artifact)
            .unwrap_err()
            .contains("reversed"));
    }

    #[test]
    fn reduction_artifact_validator_rejects_admission_tamper_even_when_rehashed() {
        let mut artifact = valid_reduction_artifact();
        validate_vcet_temporal_pre_final_reduction_artifact(&artifact, &artifact.artifact_sha256)
            .expect("valid reduction");
        artifact.payload.candidates[0].admission_passed = false;
        artifact.artifact_sha256 =
            recompute_vcet_temporal_pre_final_reduction_artifact_sha256(&artifact.payload)
                .expect("tampered hash");
        assert!(validate_vcet_temporal_pre_final_reduction_artifact(
            &artifact,
            &artifact.artifact_sha256,
        )
        .unwrap_err()
        .contains("admission"));
    }

    #[test]
    fn cross_universe_admission_honors_configured_partial_support_floors() {
        let artifact = partial_support_reduction_artifact();
        let candidate = &artifact.payload.candidates[0];
        let admission = &artifact.payload.execution_spec.cross_universe_admission;

        assert_eq!(candidate.support_failure_count, 1);
        assert_eq!(candidate.non_finite_metric_count, 1);
        assert!(global_candidate_meets_cross_universe_admission(
            candidate, admission
        ));
        validate_vcet_temporal_pre_final_reduction_artifact(&artifact, &artifact.artifact_sha256)
            .expect("an exact-threshold partial-support winner must remain admissible");
    }

    #[test]
    fn cross_universe_admission_rejects_each_support_share_below_its_floor() {
        let artifact = partial_support_reduction_artifact();
        let mut admission = artifact.payload.execution_spec.cross_universe_admission;
        let mut candidate = artifact.payload.candidates[0].clone();

        candidate.hard_support_dataset_share_requested = 0.5 - f64::EPSILON;
        assert!(!global_candidate_meets_cross_universe_admission(
            &candidate, &admission
        ));

        candidate.hard_support_dataset_share_requested = 0.5;
        candidate.metric_support_dataset_share_requested = 0.5 - f64::EPSILON;
        assert!(!global_candidate_meets_cross_universe_admission(
            &candidate, &admission
        ));

        candidate.metric_support_dataset_share_requested = 0.5;
        admission.min_hard_support_dataset_share = 1.0;
        assert!(!global_candidate_meets_cross_universe_admission(
            &candidate, &admission
        ));
        admission.min_hard_support_dataset_share = 0.5;
        admission.min_metric_support_dataset_share = 1.0;
        assert!(!global_candidate_meets_cross_universe_admission(
            &candidate, &admission
        ));
    }

    #[test]
    fn cross_universe_admission_still_fails_closed_without_required_metric_evidence() {
        let artifact = partial_support_reduction_artifact();
        let mut admission = artifact.payload.execution_spec.cross_universe_admission;
        admission.min_hard_support_dataset_share = 0.0;
        admission.min_metric_support_dataset_share = 0.0;
        let mut candidate = artifact.payload.candidates[0].clone();

        assert!(global_candidate_meets_cross_universe_admission(
            &candidate, &admission
        ));
        candidate.mean_validation_sharpe = None;
        assert!(!global_candidate_meets_cross_universe_admission(
            &candidate, &admission
        ));
        candidate.mean_validation_sharpe = Some(2.0);
        candidate.validation_pnl_p10 = None;
        assert!(!global_candidate_meets_cross_universe_admission(
            &candidate, &admission
        ));
        candidate.validation_pnl_p10 = Some(1.0);
        candidate.validation_sharpe_p10 = None;
        assert!(!global_candidate_meets_cross_universe_admission(
            &candidate, &admission
        ));
    }

    #[test]
    fn reduction_validator_rejects_raw_failure_count_support_share_mutations() {
        for mutate in [(true, "hard support"), (false, "metric support")] {
            let mut artifact = valid_reduction_artifact();
            if mutate.0 {
                artifact.payload.candidates[0].support_failure_count = 1;
            } else {
                artifact.payload.candidates[0].non_finite_metric_count = 1;
            }
            artifact.artifact_sha256 =
                recompute_vcet_temporal_pre_final_reduction_artifact_sha256(&artifact.payload)
                    .expect("tampered artifact hash");
            let error = validate_vcet_temporal_pre_final_reduction_artifact(
                &artifact,
                &artifact.artifact_sha256,
            )
            .expect_err(mutate.1);
            assert!(error.contains("raw failure counts"), "{error}");
        }

        for mutate in [
            (true, "missing hard failures"),
            (false, "missing non-finite metrics"),
        ] {
            let mut artifact = partial_support_reduction_artifact();
            if mutate.0 {
                artifact.payload.candidates[0].support_failure_count = 0;
            } else {
                artifact.payload.candidates[0].non_finite_metric_count = 0;
            }
            artifact.artifact_sha256 =
                recompute_vcet_temporal_pre_final_reduction_artifact_sha256(&artifact.payload)
                    .expect("tampered artifact hash");
            let error = validate_vcet_temporal_pre_final_reduction_artifact(
                &artifact,
                &artifact.artifact_sha256,
            )
            .expect_err(mutate.1);
            assert!(error.contains("raw failure counts"), "{error}");
        }
    }

    #[test]
    fn reduction_validator_rejects_below_floor_partial_support_marked_admitted() {
        let mut artifact = partial_support_reduction_artifact();
        let candidate = &mut artifact.payload.candidates[0];
        candidate.hard_support_dataset_count = 0;
        candidate.hard_support_dataset_share_requested = 0.0;
        rehash_reduction_lineage_for_test(&mut artifact);

        let error = validate_vcet_temporal_pre_final_reduction_artifact(
            &artifact,
            &artifact.artifact_sha256,
        )
        .expect_err("below-floor support must not be admitted");
        assert!(error.contains("admission"), "{error}");
    }

    #[test]
    fn sealed_universe_final_descriptor_sentinel_survives_winner_lock_reduction() {
        let (sealed_universe, selection_artifact, expected_final_shard) =
            sealed_universe_reduction_fixture();
        assert_eq!(expected_final_shard.candidate_start_index, 0);
        assert_eq!(expected_final_shard.candidate_end_index_exclusive, 0);
        assert!(expected_final_shard.locked_winner_only);

        let response = reduce_vcet_temporal_pre_final_shards_blocking(
            ReduceVcetTemporalPreFinalShardsRequestV1 {
                expected_universe_sha256: sealed_universe.universe_sha256.clone(),
                sealed_universe: sealed_universe.clone(),
                final_stage_nonce: "sealed-universe-happy-path-0001".to_string(),
                pre_final_shard_artifacts: vec![selection_artifact],
            },
        )
        .expect("sealed-universe pre-final reduction");
        validate_vcet_temporal_pre_final_reduction_artifact(&response, &response.artifact_sha256)
            .expect("reduction artifact validation");
        validate_vcet_temporal_pre_final_reduction_against_universe(&response, &sealed_universe)
            .expect("reduction exactly covers the sealed final descriptors");

        assert_eq!(response.payload.final_shard_authorizations.len(), 1);
        let authorization = &response.payload.final_shard_authorizations[0];
        assert_eq!(authorization.shard, expected_final_shard);
        assert_eq!(authorization.shard.candidate_start_index, 0);
        assert_eq!(authorization.shard.candidate_end_index_exclusive, 0);
        assert_eq!(
            authorization.global_candidate_ordinal,
            response.payload.winner_lock.global_candidate_ordinal
        );
        assert_eq!(
            authorization.candidate_id,
            response.payload.winner_lock.candidate_id
        );
    }

    #[test]
    fn durable_reduction_replays_exactly_and_invalid_request_does_not_claim() {
        let (sealed_universe, selection_artifact, _) = sealed_universe_reduction_fixture();
        let req = ReduceVcetTemporalPreFinalShardsRequestV1 {
            expected_universe_sha256: sealed_universe.universe_sha256.clone(),
            sealed_universe: sealed_universe.clone(),
            final_stage_nonce: "durable-prefinal-happy-path-0001".to_string(),
            pre_final_shard_artifacts: vec![selection_artifact.clone()],
        };
        let root = durable_temp_dir("exact_replay");
        let first = reduce_vcet_temporal_pre_final_shards_durable_blocking(req.clone(), &root)
            .expect("first durable reduction");
        let replay = reduce_vcet_temporal_pre_final_shards_durable_blocking(req, &root)
            .expect("exact durable replay");
        assert!(serialized_eq(&first, &replay).expect("serialize replay"));
        validate_vcet_temporal_pre_final_reduction_against_universe(&replay, &sealed_universe)
            .expect("replay exact universe coverage");

        let invalid_root = durable_temp_dir("invalid_no_claim");
        let invalid = ReduceVcetTemporalPreFinalShardsRequestV1 {
            expected_universe_sha256: sealed_universe.universe_sha256.clone(),
            sealed_universe,
            final_stage_nonce: "bad".to_string(),
            pre_final_shard_artifacts: vec![selection_artifact],
        };
        assert!(
            reduce_vcet_temporal_pre_final_shards_durable_blocking(invalid, &invalid_root)
                .expect_err("invalid reduction must fail before claim")
                .contains("nonce")
        );
        assert_eq!(
            std::fs::read_dir(&invalid_root)
                .expect("read invalid root")
                .count(),
            0,
            "invalid request must not create a durable ledger root or claim"
        );
    }

    #[test]
    fn direct_durable_reduction_reports_live_claim_as_retryable_then_interrupted_after_drop() {
        let (sealed_universe, selection_artifact, _) = sealed_universe_reduction_fixture();
        let req = ReduceVcetTemporalPreFinalShardsRequestV1 {
            expected_universe_sha256: sealed_universe.universe_sha256.clone(),
            sealed_universe: sealed_universe.clone(),
            final_stage_nonce: "durable-prefinal-live-claim-0001".to_string(),
            pre_final_shard_artifacts: vec![selection_artifact],
        };
        let request_sha256 = hash_enveloped(
            PRE_FINAL_REDUCTION_REQUEST_HASH_CONTRACT,
            &req,
            "VCET temporal pre-final reduction request",
        )
        .expect("request identity");
        let root = durable_temp_dir("live_claim_direct_command");
        let claim = match admit_durable_pre_final_winner_lock::<
            ReduceVcetTemporalPreFinalShardsResponseV1,
        >(&root, &sealed_universe.universe_sha256, &request_sha256)
        .expect("seed live direct-command claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("fresh claim cannot replay"),
        };

        let live_error = reduce_vcet_temporal_pre_final_shards_durable_blocking(req.clone(), &root)
            .expect_err("same-process live command must be retryable");
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
        let interrupted_error = reduce_vcet_temporal_pre_final_shards_durable_blocking(req, &root)
            .expect_err("dropped unfinished direct command must remain fail-closed");
        assert!(
            interrupted_error.contains("uncompleted durable claim"),
            "{interrupted_error}"
        );
    }

    #[test]
    fn universe_validator_rejects_rehashed_noncanonical_final_descriptor() {
        let (sealed_universe, selection_artifact, _) = sealed_universe_reduction_fixture();
        let mut response = reduce_vcet_temporal_pre_final_shards_blocking(
            ReduceVcetTemporalPreFinalShardsRequestV1 {
                expected_universe_sha256: sealed_universe.universe_sha256.clone(),
                sealed_universe: sealed_universe.clone(),
                final_stage_nonce: "descriptor-tamper-test-0001".to_string(),
                pre_final_shard_artifacts: vec![selection_artifact],
            },
        )
        .expect("canonical reduction");
        let authorization = &mut response.payload.final_shard_authorizations[0];
        authorization.shard.dataset_ids[0] = "substituted-final-dataset".to_string();
        authorization.expected_prior_state_sha256 = recompute_final_prior_state_sha256(
            &authorization.universe_sha256,
            &authorization.winner_lock_sha256,
            &authorization.global_pre_final_evidence_sha256,
            &authorization.final_stage_nonce,
            &authorization.shard,
        )
        .expect("rehashed prior");
        authorization.authorization_sha256 =
            recompute_final_authorization_sha256(authorization).expect("rehashed authorization");
        response.artifact_sha256 =
            recompute_vcet_temporal_pre_final_reduction_artifact_sha256(&response.payload)
                .expect("rehashed reduction");
        validate_vcet_temporal_pre_final_reduction_artifact(&response, &response.artifact_sha256)
            .expect("self-contained artifact remains internally valid");
        assert!(validate_vcet_temporal_pre_final_reduction_against_universe(
            &response,
            &sealed_universe,
        )
        .expect_err("sealed universe must reject substituted descriptor")
        .contains("exactly match"));
    }

    #[test]
    fn requested_denominator_percentile_fails_when_lower_tail_is_missing() {
        assert_eq!(
            finite_percentile_over_requested(vec![1.0; 9], 10, 0.10),
            None
        );
        assert_eq!(
            finite_percentile_over_requested(vec![1.0; 10], 10, 0.10),
            Some(1.0)
        );
    }

    #[test]
    fn reduction_limitations_describe_the_single_terminal_crash_scope() {
        let limitations = pre_final_reduction_limitations();
        for expected in [
            "cooperating_local_processes",
            "process_crash",
            "power_loss",
            "adversarial_local_modification",
        ] {
            assert!(limitations.iter().any(|item| item.contains(expected)));
        }
        assert!(!limitations.iter().any(|item| item.contains("create_new")));
        assert!(!limitations
            .iter()
            .any(|item| item.contains("completed_or_failed_record")));
    }
}

#[cfg(test)]
pub(crate) use tests::temporal_report_pre_final_fixture;
