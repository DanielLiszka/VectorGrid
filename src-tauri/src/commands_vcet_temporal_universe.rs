use super::commands_vcet_temporal::{
    validate_sealed_candidate_manifest, VcetTemporalSealedCandidateManifestV1,
    MAX_VCET_TEMPORAL_DATASETS_PER_REQUEST,
};
use super::commands_vcet_temporal_execute::{
    loaded_candles_snapshot_sha256, validate_loaded_dataset_shape, VcetTemporalSelectionPolicyV1,
    MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS, MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use ta_app_core::composite_validate::validate_vcet_fixed_candidate_strategy_config;
use ta_app_core::double_ma::Backend;
use ta_app_core::vcet_dataset_coverage::{
    VcetDatasetCoverageEvidence, VCET_DATASET_COVERAGE_SCHEMA_VERSION,
};
use ta_app_core::vcet_temporal::{
    vcet_temporal_plan_identity, VcetTemporalBarRangeV1, VcetTemporalEvaluationSliceV1,
    VcetTemporalFoldV1, VcetTemporalLayoutV1, VcetTemporalPlanIdentityV1,
    VcetTemporalPositionBoundaryPolicyV1, VcetTemporalSessionBoundsV1, VcetTemporalUtcBoundsV1,
    VcetTemporalValidationPlanV1,
};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;

pub const VCET_TEMPORAL_DATASET_ATTESTATION_SCHEMA_VERSION: u16 = 1;
pub const VCET_TEMPORAL_DATASET_UNIVERSE_SCHEMA_VERSION: u16 = 1;
pub const VCET_TEMPORAL_CROSS_UNIVERSE_ADMISSION_SCHEMA_VERSION: u16 = 1;
pub const VCET_TEMPORAL_SELECTION_REDUCTION_CONTRACT_V1: &str =
    "mean_validation_sharpe_total_cmp_v1";
pub const MAX_VCET_TEMPORAL_UNIVERSE_SHARD_DESCRIPTORS: usize = 4_096;
pub const DEFAULT_VCET_TEMPORAL_MAX_DATASETS_PER_SHARD: usize =
    MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST;

const DATASET_ATTESTATION_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_dataset_attestation.v1";
const DATASET_ATTESTATION_ROW_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_dataset_attestation_row.v1";
const DATASET_UNIVERSE_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_dataset_universe.v1";
const DATASET_UNIVERSE_SHARD_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_dataset_universe_shard.v1";
const DATASET_COVERAGE_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_dataset_coverage_evidence.v1";
const TEMPORAL_INTERVAL_FINGERPRINT_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_interval_fingerprint.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VcetTemporalDatasetCohortRoleV1 {
    Selection,
    FinalTest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttestVcetTemporalDatasetInputV1 {
    pub dataset_id: String,
    pub loaded_data_id: String,
    pub source_dataset_sha256: String,
    pub role: VcetTemporalDatasetCohortRoleV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttestVcetTemporalDatasetsRequestV1 {
    pub datasets: Vec<AttestVcetTemporalDatasetInputV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalDatasetAttestationV1 {
    pub dataset_id: String,
    pub loaded_data_id: String,
    pub source_dataset_sha256: String,
    pub loaded_candles_snapshot_sha256: String,
    pub dataset_attestation_sha256: String,
    pub role: VcetTemporalDatasetCohortRoleV1,
    pub observed_bar_count: usize,
    pub first_timestamp_utc_ms: i64,
    pub last_timestamp_utc_ms: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttestVcetTemporalDatasetsResponseV1 {
    pub schema_version: u16,
    pub evidence_kind: String,
    pub dataset_count: usize,
    pub selection_dataset_count: usize,
    pub final_test_dataset_count: usize,
    pub datasets: Vec<VcetTemporalDatasetAttestationV1>,
    pub limitations: Vec<String>,
    pub attestation_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalCrossUniverseAdmissionV1 {
    pub schema_version: u16,
    pub min_requested_dataset_coverage_share: f64,
    pub min_evaluated_dataset_share_requested: f64,
    pub min_profitable_dataset_share: f64,
    pub min_hard_support_dataset_share: f64,
    pub min_metric_support_dataset_share: f64,
    pub min_validation_pnl_p10: f64,
    pub min_validation_sharpe_p10: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalPhasedExecutionSpecV1 {
    pub backend: Backend,
    pub ma_source: String,
    pub strategy: StrategyConfig,
    pub selection_policy: VcetTemporalSelectionPolicyV1,
    pub selection_reduction_contract: String,
    pub cross_universe_admission: VcetTemporalCrossUniverseAdmissionV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalUniverseDatasetV1 {
    pub dataset_id: String,
    pub source_dataset_sha256: String,
    pub loaded_candles_snapshot_sha256: String,
    pub dataset_attestation_sha256: String,
    pub timeline_sha256: String,
    pub temporal_interval_fingerprint_sha256: String,
    pub dataset_coverage: VcetDatasetCoverageEvidence,
    #[serde(default)]
    pub dataset_coverage_sha256: String,
    pub fold_count: usize,
    pub role: VcetTemporalDatasetCohortRoleV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalUniverseShardV1 {
    pub role: VcetTemporalDatasetCohortRoleV1,
    pub shard_ordinal: usize,
    pub shard_id: String,
    pub dataset_ids: Vec<String>,
    pub candidate_start_index: usize,
    pub candidate_end_index_exclusive: usize,
    pub locked_winner_only: bool,
    pub expected_candidate_window_trial_count: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalShardPolicyV1 {
    pub max_datasets_per_shard: usize,
}

impl Default for VcetTemporalShardPolicyV1 {
    fn default() -> Self {
        Self {
            max_datasets_per_shard: DEFAULT_VCET_TEMPORAL_MAX_DATASETS_PER_SHARD,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalDatasetUniversePayloadV1 {
    pub schema_version: u16,
    pub evidence_kind: String,
    pub candidate_manifest: VcetTemporalSealedCandidateManifestV1,
    pub candidate_manifest_sha256: String,
    pub plan: VcetTemporalValidationPlanV1,
    pub plan_identity: VcetTemporalPlanIdentityV1,
    pub execution_spec: VcetTemporalPhasedExecutionSpecV1,
    pub dataset_count: usize,
    pub selection_dataset_count: usize,
    pub final_test_dataset_count: usize,
    pub fold_count_per_dataset: usize,
    pub temporal_interval_fingerprint_sha256: String,
    #[serde(default)]
    pub shard_policy: VcetTemporalShardPolicyV1,
    pub datasets: Vec<VcetTemporalUniverseDatasetV1>,
    pub shards: Vec<VcetTemporalUniverseShardV1>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalSealedDatasetUniverseV1 {
    pub payload: VcetTemporalDatasetUniversePayloadV1,
    pub universe_sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealVcetTemporalDatasetUniverseRequestV1 {
    pub sealed_candidate_manifest: VcetTemporalSealedCandidateManifestV1,
    pub expected_candidate_manifest_sha256: String,
    pub plan: VcetTemporalValidationPlanV1,
    pub expected_plan_identity: VcetTemporalPlanIdentityV1,
    pub execution_spec: VcetTemporalPhasedExecutionSpecV1,
    #[serde(default)]
    pub shard_policy: VcetTemporalShardPolicyV1,
    pub datasets: Vec<VcetTemporalUniverseDatasetV1>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SealVcetTemporalDatasetUniverseResponseV1 {
    pub sealed_universe: VcetTemporalSealedDatasetUniverseV1,
}

#[derive(Serialize)]
struct DatasetAttestationHashInput<'a> {
    hash_contract: &'static str,
    schema_version: u16,
    evidence_kind: &'a str,
    dataset_count: usize,
    selection_dataset_count: usize,
    final_test_dataset_count: usize,
    datasets: &'a [VcetTemporalDatasetAttestationV1],
    limitations: &'a [String],
}

#[derive(Serialize)]
struct DatasetAttestationRowHashInput<'a> {
    hash_contract: &'static str,
    schema_version: u16,
    dataset_id: &'a str,
    source_dataset_sha256: &'a str,
    loaded_candles_snapshot_sha256: &'a str,
    role: VcetTemporalDatasetCohortRoleV1,
}

#[derive(Serialize)]
struct DatasetUniverseHashInput<'a> {
    hash_contract: &'static str,
    payload: &'a VcetTemporalDatasetUniversePayloadV1,
}

#[derive(Serialize)]
struct DatasetUniverseShardHashInput<'a> {
    hash_contract: &'static str,
    role: VcetTemporalDatasetCohortRoleV1,
    shard_ordinal: usize,
    candidate_manifest_sha256: &'a str,
    plan_sha256: &'a str,
    temporal_interval_fingerprint_sha256: &'a str,
    fold_count_per_dataset: usize,
    datasets: &'a [VcetTemporalUniverseDatasetV1],
    candidate_start_index: usize,
    candidate_end_index_exclusive: usize,
    locked_winner_only: bool,
    expected_candidate_window_trial_count: usize,
}

#[derive(Serialize)]
struct DatasetCoverageHashInput<'a> {
    hash_contract: &'static str,
    dataset_id: &'a str,
    loaded_candles_snapshot_sha256: &'a str,
    timeline_sha256: &'a str,
    coverage: &'a VcetDatasetCoverageEvidence,
}

#[derive(Serialize)]
struct TemporalIntervalFingerprintHashInput<'a> {
    hash_contract: &'static str,
    plan_identity: &'a VcetTemporalPlanIdentityV1,
    selected_bars: VcetTemporalBarRangeV1,
    selected_utc: VcetTemporalUtcBoundsV1,
    selected_session: &'a Option<VcetTemporalSessionBoundsV1>,
    folds: &'a [VcetTemporalFoldV1],
    sealed_selection_remainder: VcetTemporalBarRangeV1,
    purge_before_final_test: VcetTemporalBarRangeV1,
    final_test: &'a VcetTemporalEvaluationSliceV1,
    position_boundary_policy: VcetTemporalPositionBoundaryPolicyV1,
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

fn sha256_serializable(value: &impl Serialize, label: &str) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_prefixed(&bytes))
        .map_err(|error| format!("failed to serialize {label} for SHA-256: {error}"))
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

fn validate_stable_id(value: &str, field: &str) -> Result<String, String> {
    let canonical = value.trim();
    if canonical.is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if canonical.len() > 1_024 {
        return Err(format!(
            "{field} exceeds the 1024-byte bounded identifier limit"
        ));
    }
    Ok(canonical.to_string())
}

pub(super) fn canonical_vcet_ma_source(value: &str) -> Result<String, String> {
    let canonical = value.trim().to_ascii_lowercase();
    match canonical.as_str() {
        "open" | "high" | "low" | "close" | "volume" | "hl2" | "hlc3" | "ohlc4"
        | "hlcc4" => Ok(canonical),
        "hlcc" => Ok("hlcc4".to_string()),
        _ => Err(format!(
            "VCET phased ma_source '{value}' is unsupported; expected open, high, low, close, volume, hl2, hlc3, ohlc4, or hlcc4"
        )),
    }
}

pub(super) fn recompute_vcet_dataset_attestation_sha256(
    dataset_id: &str,
    source_dataset_sha256: &str,
    loaded_candles_snapshot_sha256: &str,
    role: VcetTemporalDatasetCohortRoleV1,
) -> Result<String, String> {
    sha256_serializable(
        &DatasetAttestationRowHashInput {
            hash_contract: DATASET_ATTESTATION_ROW_HASH_CONTRACT,
            schema_version: VCET_TEMPORAL_DATASET_ATTESTATION_SCHEMA_VERSION,
            dataset_id,
            source_dataset_sha256,
            loaded_candles_snapshot_sha256,
            role,
        },
        "VCET temporal dataset row attestation",
    )
}

pub(super) fn recompute_vcet_dataset_coverage_sha256(
    dataset_id: &str,
    loaded_candles_snapshot_sha256: &str,
    timeline_sha256: &str,
    coverage: &VcetDatasetCoverageEvidence,
) -> Result<String, String> {
    sha256_serializable(
        &DatasetCoverageHashInput {
            hash_contract: DATASET_COVERAGE_HASH_CONTRACT,
            dataset_id,
            loaded_candles_snapshot_sha256,
            timeline_sha256,
            coverage,
        },
        "VCET temporal dataset coverage evidence",
    )
}

fn normalize_interval_range(
    range: VcetTemporalBarRangeV1,
    origin: usize,
    field: &str,
) -> Result<VcetTemporalBarRangeV1, String> {
    let start_index = range.start_index.checked_sub(origin).ok_or_else(|| {
        format!("VCET temporal interval {field} begins before the selected dataset origin")
    })?;
    let end_index_exclusive = range
        .end_index_exclusive
        .checked_sub(origin)
        .ok_or_else(|| {
            format!("VCET temporal interval {field} ends before the selected dataset origin")
        })?;
    Ok(VcetTemporalBarRangeV1 {
        start_index,
        end_index_exclusive,
    })
}

fn normalize_evaluation_slice(
    slice: &VcetTemporalEvaluationSliceV1,
    origin: usize,
    field: &str,
) -> Result<VcetTemporalEvaluationSliceV1, String> {
    Ok(VcetTemporalEvaluationSliceV1 {
        warmup_bars: normalize_interval_range(
            slice.warmup_bars,
            origin,
            &format!("{field}.warmup_bars"),
        )?,
        scored_bars: normalize_interval_range(
            slice.scored_bars,
            origin,
            &format!("{field}.scored_bars"),
        )?,
        scored_utc: slice.scored_utc,
        scored_session: slice.scored_session.clone(),
    })
}

fn normalized_interval_folds(
    folds: &[VcetTemporalFoldV1],
    origin: usize,
) -> Result<Vec<VcetTemporalFoldV1>, String> {
    folds
        .iter()
        .map(|fold| {
            Ok(VcetTemporalFoldV1 {
                fold_index: fold.fold_index,
                train: normalize_evaluation_slice(
                    &fold.train,
                    origin,
                    &format!("folds[{}].train", fold.fold_index),
                )?,
                purge_before_validation: normalize_interval_range(
                    fold.purge_before_validation,
                    origin,
                    &format!("folds[{}].purge_before_validation", fold.fold_index),
                )?,
                validation: normalize_evaluation_slice(
                    &fold.validation,
                    origin,
                    &format!("folds[{}].validation", fold.fold_index),
                )?,
                embargo_after_validation: normalize_interval_range(
                    fold.embargo_after_validation,
                    origin,
                    &format!("folds[{}].embargo_after_validation", fold.fold_index),
                )?,
            })
        })
        .collect()
}

pub(super) fn recompute_vcet_temporal_interval_fingerprint_sha256(
    layout: &VcetTemporalLayoutV1,
) -> Result<String, String> {
    let origin = layout.dataset.selected_bars.start_index;
    let selected_bars = normalize_interval_range(
        layout.dataset.selected_bars,
        origin,
        "dataset.selected_bars",
    )?;
    let folds = normalized_interval_folds(&layout.folds, origin)?;
    let sealed_selection_remainder = normalize_interval_range(
        layout.sealed_selection_remainder,
        origin,
        "sealed_selection_remainder",
    )?;
    let purge_before_final_test = normalize_interval_range(
        layout.purge_before_final_test,
        origin,
        "purge_before_final_test",
    )?;
    let final_test = normalize_evaluation_slice(&layout.final_test, origin, "final_test")?;
    sha256_serializable(
        &TemporalIntervalFingerprintHashInput {
            hash_contract: TEMPORAL_INTERVAL_FINGERPRINT_HASH_CONTRACT,
            plan_identity: &layout.identity,
            selected_bars,
            selected_utc: layout.dataset.selected_utc,
            selected_session: &layout.dataset.selected_session,
            folds: &folds,
            sealed_selection_remainder,
            purge_before_final_test,
            final_test: &final_test,
            position_boundary_policy: layout.position_boundary_policy,
        },
        "VCET temporal interval fingerprint",
    )
}

fn validate_cross_universe_admission(
    admission: &VcetTemporalCrossUniverseAdmissionV1,
) -> Result<(), String> {
    if admission.schema_version != VCET_TEMPORAL_CROSS_UNIVERSE_ADMISSION_SCHEMA_VERSION {
        return Err(format!(
            "unsupported VCET cross-universe admission schema version {}; expected {}",
            admission.schema_version, VCET_TEMPORAL_CROSS_UNIVERSE_ADMISSION_SCHEMA_VERSION
        ));
    }
    for (field, value) in [
        (
            "min_requested_dataset_coverage_share",
            admission.min_requested_dataset_coverage_share,
        ),
        (
            "min_evaluated_dataset_share_requested",
            admission.min_evaluated_dataset_share_requested,
        ),
        (
            "min_profitable_dataset_share",
            admission.min_profitable_dataset_share,
        ),
        (
            "min_hard_support_dataset_share",
            admission.min_hard_support_dataset_share,
        ),
        (
            "min_metric_support_dataset_share",
            admission.min_metric_support_dataset_share,
        ),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!(
                "VCET cross-universe admission {field} must be finite and within [0, 1]"
            ));
        }
    }
    for (field, value) in [
        ("min_validation_pnl_p10", admission.min_validation_pnl_p10),
        (
            "min_validation_sharpe_p10",
            admission.min_validation_sharpe_p10,
        ),
    ] {
        if !value.is_finite() {
            return Err(format!(
                "VCET cross-universe admission {field} must be finite"
            ));
        }
    }
    Ok(())
}

fn validate_finite_strategy_config(strategy: &StrategyConfig) -> Result<(), String> {
    for (field, value) in [
        ("fee_bps", strategy.fee_bps),
        ("spread_bps", strategy.spread_bps),
        ("slippage_bps", strategy.slippage_bps),
        ("kaufman_stop_mult", strategy.kaufman_stop_mult),
        (
            "trend_quality_angle_min_deg",
            strategy.trend_quality_angle_min_deg,
        ),
        ("trend_quality_bbw_mult", strategy.trend_quality_bbw_mult),
        ("trend_quality_bbw_min", strategy.trend_quality_bbw_min),
        ("trend_quality_adx_min", strategy.trend_quality_adx_min),
        ("regime_hybrid_adx_enter", strategy.regime_hybrid_adx_enter),
        ("regime_hybrid_adx_exit", strategy.regime_hybrid_adx_exit),
        (
            "regime_hybrid_reversion_entry_rel",
            strategy.regime_hybrid_reversion_entry_rel,
        ),
        (
            "regime_hybrid_reversion_exit_rel",
            strategy.regime_hybrid_reversion_exit_rel,
        ),
        ("breakout_bbands_mult", strategy.breakout_bbands_mult),
        ("breakout_cmo_min_abs", strategy.breakout_cmo_min_abs),
        ("breakout_adx_min", strategy.breakout_adx_min),
        (
            "volume_flow_mfi_long_min",
            strategy.volume_flow_mfi_long_min,
        ),
        (
            "volume_flow_mfi_short_max",
            strategy.volume_flow_mfi_short_max,
        ),
        ("set_angle_min_deg", strategy.set_angle_min_deg),
        ("set_er_min", strategy.set_er_min),
        ("set_chop_max", strategy.set_chop_max),
        ("pit_cfo_scalar", strategy.pit_cfo_scalar),
        ("commission", strategy.commission),
        ("eps_rel", strategy.eps_rel),
    ] {
        if !value.is_finite() {
            return Err(format!("VCET phased StrategyConfig.{field} must be finite"));
        }
    }
    for (field, value) in [
        ("stop_loss_pct", strategy.stop_loss_pct),
        ("take_profit_pct", strategy.take_profit_pct),
        ("hurdle_tau", strategy.hurdle_tau),
    ] {
        if value.is_some_and(|value| !value.is_finite()) {
            return Err(format!(
                "VCET phased StrategyConfig.{field} must be finite when present"
            ));
        }
    }
    Ok(())
}

fn canonical_execution_spec(
    mut spec: VcetTemporalPhasedExecutionSpecV1,
) -> Result<VcetTemporalPhasedExecutionSpecV1, String> {
    match spec.backend {
        Backend::CpuOnly | Backend::GpuKernel { .. } => {}
        _ => {
            return Err(
                "VCET phased execution requires CpuOnly or strict GpuKernel; fallback-capable backends are rejected"
                    .to_string(),
            )
        }
    }
    spec.ma_source = canonical_vcet_ma_source(&spec.ma_source)?;
    validate_finite_strategy_config(&spec.strategy)?;
    validate_vcet_fixed_candidate_strategy_config(&spec.strategy)
        .map_err(|error| format!("invalid VCET phased StrategyConfig: {error}"))?;
    if spec.strategy.uses_execution_overlays()
        || spec.strategy.stop_loss_pct.is_some()
        || spec.strategy.take_profit_pct.is_some()
    {
        return Err(
            "VCET phased execution does not support execution overlays, stop-loss, or take-profit"
                .to_string(),
        );
    }
    if spec.selection_policy != VcetTemporalSelectionPolicyV1::MeanValidationSharpe {
        return Err(
            "VCET phased execution supports only mean_validation_sharpe selection".to_string(),
        );
    }
    spec.selection_reduction_contract = spec.selection_reduction_contract.trim().to_string();
    if spec.selection_reduction_contract != VCET_TEMPORAL_SELECTION_REDUCTION_CONTRACT_V1 {
        return Err(format!(
            "VCET phased selection_reduction_contract must be '{VCET_TEMPORAL_SELECTION_REDUCTION_CONTRACT_V1}'"
        ));
    }
    validate_cross_universe_admission(&spec.cross_universe_admission)?;
    Ok(spec)
}

fn attestation_limitations() -> Vec<String> {
    vec![
        "source_dataset_sha256_is_lineage_only_while_loaded_candles_snapshot_sha256_attests_actual_in_memory_ohlcv_content"
            .to_string(),
        "provider_data_calendar_normalization_and_gap_repair_are_prerequisites_and_are_not_performed_by_attestation"
            .to_string(),
        format!(
            "each_attestation_invocation_is_bounded_to_{MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST}_loaded_datasets"
        ),
    ]
}

fn universe_limitations() -> Vec<String> {
    vec![
        "source_dataset_sha256_is_lineage_only_while_each_universe_entry_requires_an_actual_loaded_candles_snapshot_sha256_attestation"
            .to_string(),
        "the_universe_and_its_shards_are_bounded_cohort_primitives_and_do_not_themselves_execute_or_coordinate_a_global_winner_lock"
            .to_string(),
        "provider_data_calendar_normalization_and_gap_repair_must_be_completed_before_attestation_planning_and_universe_sealing"
            .to_string(),
        "universe_sealing_is_lightweight_and_does_not_reload_candles_each_executor_must_fail_closed_on_attestation_timeline_snapshot_and_coverage_identity"
            .to_string(),
        format!(
            "universe_shard_descriptor_materialization_is_bounded_to_{MAX_VCET_TEMPORAL_UNIVERSE_SHARD_DESCRIPTORS}_descriptors"
        ),
        "the_sealed_selection_rows_define_the_requested_denominator_attestation_or_load_failures_before_sealing_must_block_the_campaign_and_must_not_be_silently_omitted"
            .to_string(),
        "after_sealing_missing_dataset_or_shard_artifacts_fail_closed_while_missing_or_non_finite_candidate_metrics_remain_in_the_requested_denominator"
            .to_string(),
    ]
}

fn canonical_attestation_inputs(
    mut datasets: Vec<AttestVcetTemporalDatasetInputV1>,
) -> Result<Vec<AttestVcetTemporalDatasetInputV1>, String> {
    if datasets.is_empty() {
        return Err("VCET dataset attestation requires at least one dataset".to_string());
    }
    if datasets.len() > MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST {
        return Err(format!(
            "VCET dataset attestation accepts at most {MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST} loaded datasets per invocation"
        ));
    }
    for dataset in &mut datasets {
        dataset.dataset_id = validate_stable_id(&dataset.dataset_id, "dataset_id")?;
        dataset.loaded_data_id = validate_stable_id(&dataset.loaded_data_id, "loaded_data_id")?;
        dataset.source_dataset_sha256 =
            normalize_sha256(&dataset.source_dataset_sha256, "source_dataset_sha256")?;
    }
    datasets.sort_by(|left, right| {
        left.role
            .cmp(&right.role)
            .then_with(|| left.dataset_id.cmp(&right.dataset_id))
    });
    let mut dataset_ids = BTreeMap::new();
    let mut loaded_data_ids = BTreeMap::new();
    for dataset in &datasets {
        if dataset_ids
            .insert(dataset.dataset_id.clone(), dataset.role)
            .is_some()
        {
            return Err(format!(
                "duplicate VCET dataset_id '{}' in attestation request",
                dataset.dataset_id
            ));
        }
        if loaded_data_ids
            .insert(dataset.loaded_data_id.clone(), dataset.dataset_id.clone())
            .is_some()
        {
            return Err(format!(
                "duplicate VCET loaded_data_id '{}' in attestation request",
                dataset.loaded_data_id
            ));
        }
    }
    Ok(datasets)
}

pub(super) fn attest_vcet_temporal_datasets_blocking_with_loaded_candles(
    req: AttestVcetTemporalDatasetsRequestV1,
    mut loaded_by_id: BTreeMap<String, Arc<Candles>>,
) -> Result<AttestVcetTemporalDatasetsResponseV1, String> {
    let datasets = canonical_attestation_inputs(req.datasets)?;
    let mut attestations = Vec::with_capacity(datasets.len());
    for dataset in datasets {
        let candles = loaded_by_id
            .remove(&dataset.loaded_data_id)
            .ok_or_else(|| {
                format!(
                    "loaded candles '{}' were not supplied for VCET dataset '{}'",
                    dataset.loaded_data_id, dataset.dataset_id
                )
            })?;
        validate_loaded_dataset_shape(&dataset.dataset_id, &candles)?;
        let loaded_candles_snapshot_sha256 = loaded_candles_snapshot_sha256(&candles);
        let dataset_attestation_sha256 = recompute_vcet_dataset_attestation_sha256(
            &dataset.dataset_id,
            &dataset.source_dataset_sha256,
            &loaded_candles_snapshot_sha256,
            dataset.role,
        )?;
        attestations.push(VcetTemporalDatasetAttestationV1 {
            dataset_id: dataset.dataset_id,
            loaded_data_id: dataset.loaded_data_id,
            source_dataset_sha256: dataset.source_dataset_sha256,
            loaded_candles_snapshot_sha256,
            dataset_attestation_sha256,
            role: dataset.role,
            observed_bar_count: candles.timestamp.len(),
            first_timestamp_utc_ms: candles.timestamp[0],
            last_timestamp_utc_ms: candles.timestamp[candles.timestamp.len() - 1],
        });
    }
    if !loaded_by_id.is_empty() {
        return Err("unrequested loaded candles were supplied for VCET attestation".to_string());
    }

    let selection_dataset_count = attestations
        .iter()
        .filter(|dataset| dataset.role == VcetTemporalDatasetCohortRoleV1::Selection)
        .count();
    let final_test_dataset_count = attestations.len() - selection_dataset_count;
    let evidence_kind = "vcet_temporal_dataset_attestation".to_string();
    let limitations = attestation_limitations();
    let attestation_sha256 = sha256_serializable(
        &DatasetAttestationHashInput {
            hash_contract: DATASET_ATTESTATION_HASH_CONTRACT,
            schema_version: VCET_TEMPORAL_DATASET_ATTESTATION_SCHEMA_VERSION,
            evidence_kind: &evidence_kind,
            dataset_count: attestations.len(),
            selection_dataset_count,
            final_test_dataset_count,
            datasets: &attestations,
            limitations: &limitations,
        },
        "VCET temporal dataset attestation",
    )?;
    Ok(AttestVcetTemporalDatasetsResponseV1 {
        schema_version: VCET_TEMPORAL_DATASET_ATTESTATION_SCHEMA_VERSION,
        evidence_kind,
        dataset_count: attestations.len(),
        selection_dataset_count,
        final_test_dataset_count,
        datasets: attestations,
        limitations,
        attestation_sha256,
    })
}

#[tauri::command]
pub async fn attest_vcet_temporal_datasets(
    req: AttestVcetTemporalDatasetsRequestV1,
    state: tauri::State<'_, AppState>,
) -> Result<AttestVcetTemporalDatasetsResponseV1, String> {
    let canonical_inputs = canonical_attestation_inputs(req.datasets)?;
    let mut loaded_by_id = BTreeMap::new();
    for dataset in &canonical_inputs {
        let candles = state
            .get_candles_for_run(&dataset.loaded_data_id)
            .map_err(|error| {
                format!(
                    "failed to resolve loaded candles '{}' for VCET dataset '{}': {error}",
                    dataset.loaded_data_id, dataset.dataset_id
                )
            })?;
        loaded_by_id.insert(dataset.loaded_data_id.clone(), candles);
    }
    let request = AttestVcetTemporalDatasetsRequestV1 {
        datasets: canonical_inputs,
    };
    tauri::async_runtime::spawn_blocking(move || {
        attest_vcet_temporal_datasets_blocking_with_loaded_candles(request, loaded_by_id)
    })
    .await
    .map_err(|error| format!("VCET temporal dataset attestation task failed: {error}"))?
}

fn canonical_universe_datasets(
    mut datasets: Vec<VcetTemporalUniverseDatasetV1>,
    plan: &VcetTemporalValidationPlanV1,
) -> Result<
    (
        Vec<VcetTemporalUniverseDatasetV1>,
        usize,
        usize,
        usize,
        String,
    ),
    String,
> {
    if datasets.is_empty() {
        return Err("VCET dataset universe requires at least one dataset".to_string());
    }
    if datasets.len() > MAX_VCET_TEMPORAL_DATASETS_PER_REQUEST {
        return Err(format!(
            "VCET dataset universe accepts at most {MAX_VCET_TEMPORAL_DATASETS_PER_REQUEST} datasets"
        ));
    }
    for dataset in &mut datasets {
        dataset.dataset_id = validate_stable_id(&dataset.dataset_id, "dataset_id")?;
        dataset.source_dataset_sha256 =
            normalize_sha256(&dataset.source_dataset_sha256, "source_dataset_sha256")?;
        dataset.loaded_candles_snapshot_sha256 = normalize_sha256(
            &dataset.loaded_candles_snapshot_sha256,
            "loaded_candles_snapshot_sha256",
        )?;
        let declared_dataset_attestation_sha256 = normalize_sha256(
            &dataset.dataset_attestation_sha256,
            "dataset_attestation_sha256",
        )?;
        let actual_dataset_attestation_sha256 = recompute_vcet_dataset_attestation_sha256(
            &dataset.dataset_id,
            &dataset.source_dataset_sha256,
            &dataset.loaded_candles_snapshot_sha256,
            dataset.role,
        )?;
        if declared_dataset_attestation_sha256 != actual_dataset_attestation_sha256 {
            return Err(format!(
                "VCET universe dataset '{}' row attestation SHA-256 mismatch (declared {}, actual {})",
                dataset.dataset_id,
                declared_dataset_attestation_sha256,
                actual_dataset_attestation_sha256
            ));
        }
        dataset.dataset_attestation_sha256 = declared_dataset_attestation_sha256;
        dataset.timeline_sha256 = normalize_sha256(&dataset.timeline_sha256, "timeline_sha256")?;
        dataset.temporal_interval_fingerprint_sha256 = normalize_sha256(
            &dataset.temporal_interval_fingerprint_sha256,
            "temporal_interval_fingerprint_sha256",
        )?;
        if dataset.dataset_coverage.schema_version != VCET_DATASET_COVERAGE_SCHEMA_VERSION {
            return Err(format!(
                "VCET universe dataset '{}' has unsupported coverage schema version {}",
                dataset.dataset_id, dataset.dataset_coverage.schema_version
            ));
        }
        if dataset.dataset_coverage.timestamp_count < 2
            || !dataset.dataset_coverage.ordering.strictly_increasing
        {
            return Err(format!(
                "VCET universe dataset '{}' coverage must attest at least two strictly increasing timestamps",
                dataset.dataset_id
            ));
        }
        let expected_end_exclusive = plan
            .requested_bounds
            .utc
            .end_utc_ms_inclusive
            .checked_add(1)
            .ok_or_else(|| {
                "VCET plan requested UTC end cannot convert to half-open coverage bounds"
                    .to_string()
            })?;
        let coverage_bounds = dataset.dataset_coverage.requested_bounds.ok_or_else(|| {
            format!(
                "VCET universe dataset '{}' coverage must include requested bounds",
                dataset.dataset_id
            )
        })?;
        if coverage_bounds.start_ms != plan.requested_bounds.utc.start_utc_ms_inclusive
            || coverage_bounds.end_exclusive_ms != expected_end_exclusive
        {
            return Err(format!(
                "VCET universe dataset '{}' coverage requested bounds do not match the sealed plan",
                dataset.dataset_id
            ));
        }
        dataset.dataset_coverage_sha256 = recompute_vcet_dataset_coverage_sha256(
            &dataset.dataset_id,
            &dataset.loaded_candles_snapshot_sha256,
            &dataset.timeline_sha256,
            &dataset.dataset_coverage,
        )?;
        if dataset.fold_count == 0 {
            return Err(format!(
                "VCET universe dataset '{}' fold_count must be greater than zero",
                dataset.dataset_id
            ));
        }
        if dataset.fold_count < plan.minimum_support.min_folds {
            return Err(format!(
                "VCET universe dataset '{}' fold_count {} is below plan minimum {}",
                dataset.dataset_id, dataset.fold_count, plan.minimum_support.min_folds
            ));
        }
    }
    datasets.sort_by(|left, right| {
        left.role
            .cmp(&right.role)
            .then_with(|| left.dataset_id.cmp(&right.dataset_id))
    });
    if datasets
        .windows(2)
        .any(|pair| pair[0].dataset_id == pair[1].dataset_id)
    {
        return Err("VCET dataset universe contains duplicate dataset_id values".to_string());
    }
    let mut source_hashes = BTreeMap::new();
    let mut snapshot_hashes = BTreeMap::new();
    for dataset in &datasets {
        if let Some(prior_dataset_id) = source_hashes.insert(
            dataset.source_dataset_sha256.clone(),
            dataset.dataset_id.clone(),
        ) {
            return Err(format!(
                "VCET dataset universe cannot reuse source_dataset_sha256 across datasets '{}' and '{}'",
                prior_dataset_id, dataset.dataset_id
            ));
        }
        if let Some(prior_dataset_id) = snapshot_hashes.insert(
            dataset.loaded_candles_snapshot_sha256.clone(),
            dataset.dataset_id.clone(),
        ) {
            return Err(format!(
                "VCET dataset universe cannot reuse loaded_candles_snapshot_sha256 across datasets '{}' and '{}'",
                prior_dataset_id, dataset.dataset_id
            ));
        }
    }
    let fold_count = datasets[0].fold_count;
    let interval_fingerprint = datasets[0].temporal_interval_fingerprint_sha256.clone();
    for dataset in &datasets[1..] {
        if dataset.fold_count != fold_count {
            return Err(format!(
                "VCET dataset universe requires one common fold_count; dataset '{}' has {} instead of {}",
                dataset.dataset_id, dataset.fold_count, fold_count
            ));
        }
        if dataset.temporal_interval_fingerprint_sha256 != interval_fingerprint {
            return Err(format!(
                "VCET dataset universe requires one common temporal interval fingerprint; dataset '{}' differs",
                dataset.dataset_id
            ));
        }
    }
    let selection_count = datasets
        .iter()
        .filter(|dataset| dataset.role == VcetTemporalDatasetCohortRoleV1::Selection)
        .count();
    let final_count = datasets.len() - selection_count;
    if selection_count == 0 || final_count == 0 {
        return Err(
            "VCET dataset universe requires at least one selection dataset and one final_test dataset"
                .to_string(),
        );
    }
    Ok((
        datasets,
        selection_count,
        final_count,
        fold_count,
        interval_fingerprint,
    ))
}

fn shard_id(
    role: VcetTemporalDatasetCohortRoleV1,
    shard_ordinal: usize,
    candidate_manifest_sha256: &str,
    plan_sha256: &str,
    temporal_interval_fingerprint_sha256: &str,
    fold_count_per_dataset: usize,
    datasets: &[VcetTemporalUniverseDatasetV1],
    candidate_start_index: usize,
    candidate_end_index_exclusive: usize,
    locked_winner_only: bool,
    expected_candidate_window_trial_count: usize,
) -> Result<String, String> {
    let digest = sha256_serializable(
        &DatasetUniverseShardHashInput {
            hash_contract: DATASET_UNIVERSE_SHARD_HASH_CONTRACT,
            role,
            shard_ordinal,
            candidate_manifest_sha256,
            plan_sha256,
            temporal_interval_fingerprint_sha256,
            fold_count_per_dataset,
            datasets,
            candidate_start_index,
            candidate_end_index_exclusive,
            locked_winner_only,
            expected_candidate_window_trial_count,
        },
        "VCET temporal universe shard",
    )?;
    Ok(format!(
        "vcet-temporal-shard-v1-{}",
        digest
            .strip_prefix("sha256:")
            .expect("internally generated SHA-256 uses prefix")
    ))
}

fn validate_shard_policy(
    policy: VcetTemporalShardPolicyV1,
) -> Result<VcetTemporalShardPolicyV1, String> {
    if policy.max_datasets_per_shard == 0
        || policy.max_datasets_per_shard > MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST
    {
        return Err(format!(
            "VCET max_datasets_per_shard must be between 1 and {MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST}"
        ));
    }
    Ok(policy)
}

fn derive_universe_shards(
    datasets: &[VcetTemporalUniverseDatasetV1],
    candidate_count: usize,
    candidate_manifest_sha256: &str,
    plan_sha256: &str,
    temporal_interval_fingerprint_sha256: &str,
    fold_count_per_dataset: usize,
    shard_policy: VcetTemporalShardPolicyV1,
) -> Result<Vec<VcetTemporalUniverseShardV1>, String> {
    let shard_policy = validate_shard_policy(shard_policy)?;
    let selection = datasets
        .iter()
        .filter(|dataset| dataset.role == VcetTemporalDatasetCohortRoleV1::Selection)
        .cloned()
        .collect::<Vec<_>>();
    let final_test = datasets
        .iter()
        .filter(|dataset| dataset.role == VcetTemporalDatasetCohortRoleV1::FinalTest)
        .cloned()
        .collect::<Vec<_>>();
    if candidate_count == 0 {
        return Err("VCET universe cannot shard an empty candidate manifest".to_string());
    }
    let trials_per_dataset_candidate = fold_count_per_dataset
        .checked_mul(2)
        .ok_or_else(|| "VCET selection shard trial multiplier overflow".to_string())?;
    if trials_per_dataset_candidate == 0
        || trials_per_dataset_candidate > MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS
    {
        return Err(format!(
            "VCET fold_count {fold_count_per_dataset} cannot fit one selection dataset/candidate inside the {MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS}-trial shard bound"
        ));
    }
    let selection_dataset_chunk_size = shard_policy
        .max_datasets_per_shard
        .min(MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS / trials_per_dataset_candidate);
    if selection_dataset_chunk_size == 0 {
        return Err("VCET selection dataset shard size resolved to zero".to_string());
    }

    let mut descriptor_count = 0usize;
    for dataset_chunk in selection.chunks(selection_dataset_chunk_size) {
        let trials_per_candidate = dataset_chunk
            .len()
            .checked_mul(trials_per_dataset_candidate)
            .ok_or_else(|| "VCET selection shard trial count overflow".to_string())?;
        let candidate_chunk_size =
            candidate_count.min(MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS / trials_per_candidate);
        if candidate_chunk_size == 0 {
            return Err("VCET selection candidate shard size resolved to zero".to_string());
        }
        let candidate_descriptor_count = candidate_count / candidate_chunk_size
            + usize::from(candidate_count % candidate_chunk_size != 0);
        descriptor_count = descriptor_count
            .checked_add(candidate_descriptor_count)
            .ok_or_else(|| "VCET universe shard descriptor count overflow".to_string())?;
    }
    let final_descriptor_count = final_test.len() / shard_policy.max_datasets_per_shard
        + usize::from(final_test.len() % shard_policy.max_datasets_per_shard != 0);
    descriptor_count = descriptor_count
        .checked_add(final_descriptor_count)
        .ok_or_else(|| "VCET universe shard descriptor count overflow".to_string())?;
    if descriptor_count > MAX_VCET_TEMPORAL_UNIVERSE_SHARD_DESCRIPTORS {
        return Err(format!(
            "VCET universe would materialize {descriptor_count} shard descriptors, exceeding the bounded limit of {MAX_VCET_TEMPORAL_UNIVERSE_SHARD_DESCRIPTORS}"
        ));
    }

    let mut shards = Vec::with_capacity(descriptor_count);
    let mut selection_ordinal = 0usize;
    for dataset_chunk in selection.chunks(selection_dataset_chunk_size) {
        let trials_per_candidate = dataset_chunk
            .len()
            .checked_mul(trials_per_dataset_candidate)
            .ok_or_else(|| "VCET selection shard trial count overflow".to_string())?;
        let candidate_chunk_size =
            candidate_count.min(MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS / trials_per_candidate);
        if candidate_chunk_size == 0 {
            return Err("VCET selection candidate shard size resolved to zero".to_string());
        }
        let mut candidate_start_index = 0usize;
        while candidate_start_index < candidate_count {
            let candidate_end_index_exclusive = candidate_start_index
                .saturating_add(candidate_chunk_size)
                .min(candidate_count);
            let candidate_span = candidate_end_index_exclusive - candidate_start_index;
            let expected_candidate_window_trial_count = trials_per_candidate
                .checked_mul(candidate_span)
                .ok_or_else(|| "VCET selection shard trial count overflow".to_string())?;
            if expected_candidate_window_trial_count > MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS {
                return Err("VCET selection shard exceeds the bounded trial limit".to_string());
            }
            let id = shard_id(
                VcetTemporalDatasetCohortRoleV1::Selection,
                selection_ordinal,
                candidate_manifest_sha256,
                plan_sha256,
                temporal_interval_fingerprint_sha256,
                fold_count_per_dataset,
                dataset_chunk,
                candidate_start_index,
                candidate_end_index_exclusive,
                false,
                expected_candidate_window_trial_count,
            )?;
            shards.push(VcetTemporalUniverseShardV1 {
                role: VcetTemporalDatasetCohortRoleV1::Selection,
                shard_ordinal: selection_ordinal,
                shard_id: id,
                dataset_ids: dataset_chunk
                    .iter()
                    .map(|dataset| dataset.dataset_id.clone())
                    .collect(),
                candidate_start_index,
                candidate_end_index_exclusive,
                locked_winner_only: false,
                expected_candidate_window_trial_count,
            });
            selection_ordinal = selection_ordinal.saturating_add(1);
            candidate_start_index = candidate_end_index_exclusive;
        }
    }

    for (shard_ordinal, dataset_chunk) in final_test
        .chunks(shard_policy.max_datasets_per_shard)
        .enumerate()
    {
        let expected_candidate_window_trial_count = dataset_chunk.len();
        let id = shard_id(
            VcetTemporalDatasetCohortRoleV1::FinalTest,
            shard_ordinal,
            candidate_manifest_sha256,
            plan_sha256,
            temporal_interval_fingerprint_sha256,
            fold_count_per_dataset,
            dataset_chunk,
            0,
            0,
            true,
            expected_candidate_window_trial_count,
        )?;
        shards.push(VcetTemporalUniverseShardV1 {
            role: VcetTemporalDatasetCohortRoleV1::FinalTest,
            shard_ordinal,
            shard_id: id,
            dataset_ids: dataset_chunk
                .iter()
                .map(|dataset| dataset.dataset_id.clone())
                .collect(),
            candidate_start_index: 0,
            candidate_end_index_exclusive: 0,
            locked_winner_only: true,
            expected_candidate_window_trial_count,
        });
    }
    debug_assert_eq!(shards.len(), descriptor_count);
    Ok(shards)
}

pub(super) fn recompute_vcet_temporal_universe_sha256(
    payload: &VcetTemporalDatasetUniversePayloadV1,
) -> Result<String, String> {
    sha256_serializable(
        &DatasetUniverseHashInput {
            hash_contract: DATASET_UNIVERSE_HASH_CONTRACT,
            payload,
        },
        "VCET temporal dataset universe",
    )
}

pub(super) fn universe_entry_map<'a>(
    payload: &'a VcetTemporalDatasetUniversePayloadV1,
) -> Result<BTreeMap<String, &'a VcetTemporalUniverseDatasetV1>, String> {
    let mut entries = BTreeMap::new();
    for dataset in &payload.datasets {
        if entries
            .insert(dataset.dataset_id.clone(), dataset)
            .is_some()
        {
            return Err(format!(
                "duplicate VCET universe dataset_id '{}'",
                dataset.dataset_id
            ));
        }
    }
    Ok(entries)
}

pub(super) fn universe_shards_for_role(
    payload: &VcetTemporalDatasetUniversePayloadV1,
    role: VcetTemporalDatasetCohortRoleV1,
) -> Vec<&VcetTemporalUniverseShardV1> {
    payload
        .shards
        .iter()
        .filter(|shard| shard.role == role)
        .collect()
}

fn serialized_eq(left: &impl Serialize, right: &impl Serialize) -> Result<bool, String> {
    let left = serde_json::to_vec(left)
        .map_err(|error| format!("failed to serialize left canonical value: {error}"))?;
    let right = serde_json::to_vec(right)
        .map_err(|error| format!("failed to serialize right canonical value: {error}"))?;
    Ok(left == right)
}

pub(super) fn validate_sealed_vcet_temporal_universe(
    sealed: &VcetTemporalSealedDatasetUniverseV1,
    expected_universe_sha256: &str,
) -> Result<(), String> {
    let payload = &sealed.payload;
    if payload.schema_version != VCET_TEMPORAL_DATASET_UNIVERSE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported VCET dataset universe schema version {}; expected {}",
            payload.schema_version, VCET_TEMPORAL_DATASET_UNIVERSE_SCHEMA_VERSION
        ));
    }
    if payload.evidence_kind != "vcet_temporal_dataset_universe" {
        return Err("VCET dataset universe evidence_kind is not canonical".to_string());
    }
    validate_sealed_candidate_manifest(
        &payload.candidate_manifest,
        &payload.candidate_manifest_sha256,
    )?;
    if payload.candidate_manifest.manifest_sha256 != payload.candidate_manifest_sha256 {
        return Err(
            "VCET universe candidate_manifest_sha256 must exactly match the sealed manifest"
                .to_string(),
        );
    }
    let plan_identity =
        vcet_temporal_plan_identity(&payload.plan).map_err(|error| error.to_string())?;
    if payload.plan_identity != plan_identity {
        return Err("VCET universe temporal plan identity does not match its plan".to_string());
    }
    if payload.plan.position_boundary_policy
        != VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen
    {
        return Err(
            "VCET dataset universe supports only reset_flat_discard_open position boundaries"
                .to_string(),
        );
    }
    let canonical_spec = canonical_execution_spec(payload.execution_spec.clone())?;
    if !serialized_eq(&canonical_spec, &payload.execution_spec)? {
        return Err("VCET universe execution spec is not canonical".to_string());
    }
    let (datasets, selection_count, final_count, fold_count, interval_fingerprint) =
        canonical_universe_datasets(payload.datasets.clone(), &payload.plan)?;
    let shard_policy = validate_shard_policy(payload.shard_policy)?;
    if datasets != payload.datasets {
        return Err("VCET universe datasets are not in canonical role/ID order".to_string());
    }
    if payload.dataset_count != datasets.len()
        || payload.selection_dataset_count != selection_count
        || payload.final_test_dataset_count != final_count
        || payload.fold_count_per_dataset != fold_count
        || payload.temporal_interval_fingerprint_sha256 != interval_fingerprint
    {
        return Err(
            "VCET universe dataset counts or common temporal evidence mismatch".to_string(),
        );
    }
    let shards = derive_universe_shards(
        &datasets,
        payload.candidate_manifest.payload.candidates.len(),
        &payload.candidate_manifest_sha256,
        &payload.plan_identity.plan_sha256,
        &interval_fingerprint,
        fold_count,
        shard_policy,
    )?;
    if shards != payload.shards {
        return Err("VCET universe shard descriptors are not canonical".to_string());
    }
    if payload.limitations != universe_limitations() {
        return Err("VCET universe limitations are not canonical".to_string());
    }
    let actual = recompute_vcet_temporal_universe_sha256(payload)?;
    let declared = normalize_sha256(&sealed.universe_sha256, "universe_sha256")?;
    let expected = normalize_sha256(expected_universe_sha256, "expected_universe_sha256")?;
    if declared != sealed.universe_sha256 {
        return Err("VCET universe SHA-256 must use canonical lowercase sha256: form".to_string());
    }
    if declared != actual || expected != actual {
        return Err(format!(
            "VCET dataset universe SHA-256 mismatch (declared {declared}, expected {expected}, actual {actual})"
        ));
    }
    universe_entry_map(payload)?;
    Ok(())
}

pub fn seal_vcet_temporal_dataset_universe_blocking(
    req: SealVcetTemporalDatasetUniverseRequestV1,
) -> Result<SealVcetTemporalDatasetUniverseResponseV1, String> {
    validate_sealed_candidate_manifest(
        &req.sealed_candidate_manifest,
        &req.expected_candidate_manifest_sha256,
    )?;
    let plan_identity =
        vcet_temporal_plan_identity(&req.plan).map_err(|error| error.to_string())?;
    if plan_identity != req.expected_plan_identity {
        return Err(format!(
            "VCET dataset universe plan identity mismatch (expected {} / {}, actual {} / {})",
            req.expected_plan_identity.plan_id,
            req.expected_plan_identity.plan_sha256,
            plan_identity.plan_id,
            plan_identity.plan_sha256
        ));
    }
    if req.plan.position_boundary_policy
        != VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen
    {
        return Err(
            "VCET dataset universe supports only reset_flat_discard_open position boundaries"
                .to_string(),
        );
    }
    let execution_spec = canonical_execution_spec(req.execution_spec)?;
    let shard_policy = validate_shard_policy(req.shard_policy)?;
    let (datasets, selection_count, final_count, fold_count, interval_fingerprint) =
        canonical_universe_datasets(req.datasets, &req.plan)?;
    let candidate_manifest_sha256 = req.sealed_candidate_manifest.manifest_sha256.clone();
    let shards = derive_universe_shards(
        &datasets,
        req.sealed_candidate_manifest.payload.candidates.len(),
        &candidate_manifest_sha256,
        &plan_identity.plan_sha256,
        &interval_fingerprint,
        fold_count,
        shard_policy,
    )?;
    let payload = VcetTemporalDatasetUniversePayloadV1 {
        schema_version: VCET_TEMPORAL_DATASET_UNIVERSE_SCHEMA_VERSION,
        evidence_kind: "vcet_temporal_dataset_universe".to_string(),
        candidate_manifest: req.sealed_candidate_manifest,
        candidate_manifest_sha256,
        plan: req.plan,
        plan_identity,
        execution_spec,
        dataset_count: datasets.len(),
        selection_dataset_count: selection_count,
        final_test_dataset_count: final_count,
        fold_count_per_dataset: fold_count,
        temporal_interval_fingerprint_sha256: interval_fingerprint,
        shard_policy,
        datasets,
        shards,
        limitations: universe_limitations(),
    };
    let universe_sha256 = recompute_vcet_temporal_universe_sha256(&payload)?;
    let sealed_universe = VcetTemporalSealedDatasetUniverseV1 {
        payload,
        universe_sha256: universe_sha256.clone(),
    };
    validate_sealed_vcet_temporal_universe(&sealed_universe, &universe_sha256)?;
    Ok(SealVcetTemporalDatasetUniverseResponseV1 { sealed_universe })
}

#[tauri::command]
pub async fn seal_vcet_temporal_dataset_universe(
    req: SealVcetTemporalDatasetUniverseRequestV1,
) -> Result<SealVcetTemporalDatasetUniverseResponseV1, String> {
    tauri::async_runtime::spawn_blocking(move || seal_vcet_temporal_dataset_universe_blocking(req))
        .await
        .map_err(|error| format!("VCET temporal dataset universe sealing task failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::super::commands_vcet_temporal::seal_vcet_temporal_candidate_manifest;
    use super::*;
    use ta_app_core::vcet_dataset_coverage::{
        profile_vcet_dataset_coverage, VcetDatasetCoverageOptions, VcetRequestedCoverageBoundsMs,
    };
    use ta_app_core::vcet_search::VcetCandidateIdentityInputs;
    use ta_app_core::vcet_temporal::{
        plan_vcet_temporal_folds, VcetTemporalFoldModeV1, VcetTemporalMinimumSupportV1,
        VcetTemporalRequestedBoundsV1, VcetTemporalUtcBoundsV1,
        VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION,
    };

    fn hash(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn labeled_hash(label: &str) -> String {
        sha256_prefixed(label.as_bytes())
    }

    fn refresh_row_attestation(dataset: &mut VcetTemporalUniverseDatasetV1) {
        dataset.dataset_attestation_sha256 = recompute_vcet_dataset_attestation_sha256(
            &dataset.dataset_id,
            &dataset.source_dataset_sha256,
            &dataset.loaded_candles_snapshot_sha256,
            dataset.role,
        )
        .expect("row attestation");
    }

    fn candidate(short: u16, long: u16) -> VcetCandidateIdentityInputs {
        VcetCandidateIdentityInputs {
            vidya_short_period: short,
            vidya_long_period: long,
            vidya_alpha_micros: 200_000,
            ce_period: 20,
            ce_mult_micros: 3_000_000,
            ce_use_close: true,
            require_vidya_slope: true,
            max_trade_bars: 40,
        }
    }

    fn plan(boundary: VcetTemporalPositionBoundaryPolicyV1) -> VcetTemporalValidationPlanV1 {
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
            position_boundary_policy: boundary,
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

    fn universe_dataset(
        id: &str,
        role: VcetTemporalDatasetCohortRoleV1,
    ) -> VcetTemporalUniverseDatasetV1 {
        let timestamps = (0..8).map(|index| index * 60_000).collect::<Vec<_>>();
        let dataset_coverage = profile_vcet_dataset_coverage(
            &timestamps,
            VcetDatasetCoverageOptions {
                requested_bounds: Some(VcetRequestedCoverageBoundsMs {
                    start_ms: 0,
                    end_exclusive_ms: 1_000_001,
                }),
                expected_cadence_ms: Some(60_000),
                session_interval_mask: None,
            },
        )
        .expect("coverage");
        let source_dataset_sha256 = labeled_hash(&format!("source:{id}"));
        let loaded_candles_snapshot_sha256 = labeled_hash(&format!("snapshot:{id}"));
        let dataset_attestation_sha256 = recompute_vcet_dataset_attestation_sha256(
            id,
            &source_dataset_sha256,
            &loaded_candles_snapshot_sha256,
            role,
        )
        .expect("row attestation");
        VcetTemporalUniverseDatasetV1 {
            dataset_id: id.to_string(),
            source_dataset_sha256,
            loaded_candles_snapshot_sha256,
            dataset_attestation_sha256,
            timeline_sha256: hash('f'),
            temporal_interval_fingerprint_sha256: hash('9'),
            dataset_coverage,
            dataset_coverage_sha256: String::new(),
            fold_count: 3,
            role,
        }
    }

    fn seal_request() -> SealVcetTemporalDatasetUniverseRequestV1 {
        let plan = plan(VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen);
        let manifest = seal_vcet_temporal_candidate_manifest(
            hash('a'),
            hash('b'),
            None,
            vec![candidate(12, 48), candidate(8, 32)],
        )
        .expect("manifest");
        SealVcetTemporalDatasetUniverseRequestV1 {
            expected_candidate_manifest_sha256: manifest.manifest_sha256.clone(),
            sealed_candidate_manifest: manifest,
            expected_plan_identity: vcet_temporal_plan_identity(&plan).expect("identity"),
            plan,
            execution_spec: VcetTemporalPhasedExecutionSpecV1 {
                backend: Backend::CpuOnly,
                ma_source: " Close ".to_string(),
                strategy: StrategyConfig::default(),
                selection_policy: VcetTemporalSelectionPolicyV1::MeanValidationSharpe,
                selection_reduction_contract: VCET_TEMPORAL_SELECTION_REDUCTION_CONTRACT_V1
                    .to_string(),
                cross_universe_admission: VcetTemporalCrossUniverseAdmissionV1 {
                    schema_version: VCET_TEMPORAL_CROSS_UNIVERSE_ADMISSION_SCHEMA_VERSION,
                    min_requested_dataset_coverage_share: 1.0,
                    min_evaluated_dataset_share_requested: 0.0,
                    min_profitable_dataset_share: 0.0,
                    min_hard_support_dataset_share: 0.0,
                    min_metric_support_dataset_share: 0.0,
                    min_validation_pnl_p10: -1.0e12,
                    min_validation_sharpe_p10: -1.0e12,
                },
            },
            shard_policy: VcetTemporalShardPolicyV1::default(),
            datasets: vec![
                universe_dataset("MSFT-FINAL", VcetTemporalDatasetCohortRoleV1::FinalTest),
                universe_dataset("MSFT", VcetTemporalDatasetCohortRoleV1::Selection),
                universe_dataset("AAPL", VcetTemporalDatasetCohortRoleV1::Selection),
            ],
        }
    }

    fn candles(offset: f64) -> Candles {
        let timestamps = (0..8).map(|index| index * 60_000).collect::<Vec<_>>();
        let close = (0..8)
            .map(|index| 100.0 + offset + index as f64)
            .collect::<Vec<_>>();
        Candles::new(
            timestamps,
            close.iter().map(|value| value - 0.1).collect(),
            close.iter().map(|value| value + 0.5).collect(),
            close.iter().map(|value| value - 0.5).collect(),
            close,
            vec![1_000.0; 8],
        )
    }

    #[test]
    fn universe_seal_is_deterministic_canonical_and_two_dimensional() {
        let first = seal_vcet_temporal_dataset_universe_blocking(seal_request()).expect("first");
        let mut reordered = seal_request();
        reordered.datasets.reverse();
        let second = seal_vcet_temporal_dataset_universe_blocking(reordered).expect("second seal");
        assert_eq!(
            first.sealed_universe.universe_sha256,
            second.sealed_universe.universe_sha256
        );
        assert_eq!(
            first.sealed_universe.payload.execution_spec.ma_source,
            "close"
        );
        assert!(first
            .sealed_universe
            .payload
            .datasets
            .iter()
            .all(|dataset| dataset.dataset_coverage_sha256.starts_with("sha256:")));
        assert_eq!(
            first
                .sealed_universe
                .payload
                .datasets
                .iter()
                .map(|dataset| dataset.dataset_id.as_str())
                .collect::<Vec<_>>(),
            vec!["AAPL", "MSFT", "MSFT-FINAL"]
        );
        let selection = universe_shards_for_role(
            &first.sealed_universe.payload,
            VcetTemporalDatasetCohortRoleV1::Selection,
        );
        assert!(!selection.is_empty());
        assert!(selection.iter().all(|shard| {
            !shard.locked_winner_only
                && shard.candidate_start_index < shard.candidate_end_index_exclusive
                && shard.dataset_ids.len() <= MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST
                && shard.expected_candidate_window_trial_count
                    <= MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS
        }));
        let final_test = universe_shards_for_role(
            &first.sealed_universe.payload,
            VcetTemporalDatasetCohortRoleV1::FinalTest,
        );
        assert_eq!(final_test.len(), 1);
        assert!(final_test[0].locked_winner_only);
        assert_eq!(final_test[0].candidate_start_index, 0);
        assert_eq!(final_test[0].candidate_end_index_exclusive, 0);
        validate_sealed_vcet_temporal_universe(
            &first.sealed_universe,
            &first.sealed_universe.universe_sha256,
        )
        .expect("validated");
    }

    #[test]
    fn universe_seals_and_hashes_the_dataset_shard_policy() {
        let mut one_per_shard = seal_request();
        one_per_shard.shard_policy.max_datasets_per_shard = 1;
        let first = seal_vcet_temporal_dataset_universe_blocking(one_per_shard)
            .expect("one-dataset shard universe");

        let mut reordered = seal_request();
        reordered.shard_policy.max_datasets_per_shard = 1;
        reordered.datasets.reverse();
        let second = seal_vcet_temporal_dataset_universe_blocking(reordered)
            .expect("reordered one-dataset shard universe");
        assert_eq!(
            first.sealed_universe.universe_sha256,
            second.sealed_universe.universe_sha256
        );
        assert_eq!(
            first.sealed_universe.payload.shard_policy,
            VcetTemporalShardPolicyV1 {
                max_datasets_per_shard: 1,
            }
        );
        assert!(first
            .sealed_universe
            .payload
            .shards
            .iter()
            .all(|shard| shard.dataset_ids.len() == 1));

        let default = seal_vcet_temporal_dataset_universe_blocking(seal_request())
            .expect("default shard universe");
        assert_ne!(
            first.sealed_universe.universe_sha256,
            default.sealed_universe.universe_sha256
        );

        let mut tampered = first.sealed_universe.clone();
        tampered.payload.shard_policy.max_datasets_per_shard = 2;
        let error = validate_sealed_vcet_temporal_universe(&tampered, &tampered.universe_sha256)
            .unwrap_err();
        assert!(error.contains("shard descriptors are not canonical"));
    }

    #[test]
    fn universe_rejects_zero_and_oversized_dataset_shard_policy() {
        let mut zero = seal_request();
        zero.shard_policy.max_datasets_per_shard = 0;
        assert!(seal_vcet_temporal_dataset_universe_blocking(zero)
            .unwrap_err()
            .contains("max_datasets_per_shard must be between 1 and 64"));

        let mut oversized = seal_request();
        oversized.shard_policy.max_datasets_per_shard =
            MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST + 1;
        assert!(seal_vcet_temporal_dataset_universe_blocking(oversized)
            .unwrap_err()
            .contains("max_datasets_per_shard must be between 1 and 64"));
    }

    #[test]
    fn attestation_binds_actual_snapshot_and_is_order_independent() {
        let request = AttestVcetTemporalDatasetsRequestV1 {
            datasets: vec![
                AttestVcetTemporalDatasetInputV1 {
                    dataset_id: "MSFT".to_string(),
                    loaded_data_id: "loaded-msft".to_string(),
                    source_dataset_sha256: hash('d'),
                    role: VcetTemporalDatasetCohortRoleV1::FinalTest,
                },
                AttestVcetTemporalDatasetInputV1 {
                    dataset_id: "AAPL".to_string(),
                    loaded_data_id: "loaded-aapl".to_string(),
                    source_dataset_sha256: hash('c'),
                    role: VcetTemporalDatasetCohortRoleV1::Selection,
                },
            ],
        };
        let loaded = BTreeMap::from([
            ("loaded-msft".to_string(), Arc::new(candles(2.0))),
            ("loaded-aapl".to_string(), Arc::new(candles(1.0))),
        ]);
        let first =
            attest_vcet_temporal_datasets_blocking_with_loaded_candles(request.clone(), loaded)
                .expect("attestation");
        let mut reverse = request;
        reverse.datasets.reverse();
        let second = attest_vcet_temporal_datasets_blocking_with_loaded_candles(
            reverse,
            BTreeMap::from([
                ("loaded-aapl".to_string(), Arc::new(candles(1.0))),
                ("loaded-msft".to_string(), Arc::new(candles(2.0))),
            ]),
        )
        .expect("reordered attestation");
        assert_eq!(first.attestation_sha256, second.attestation_sha256);
        for dataset in &first.datasets {
            assert_eq!(
                dataset.dataset_attestation_sha256,
                recompute_vcet_dataset_attestation_sha256(
                    &dataset.dataset_id,
                    &dataset.source_dataset_sha256,
                    &dataset.loaded_candles_snapshot_sha256,
                    dataset.role,
                )
                .expect("row attestation")
            );
        }

        let changed = attest_vcet_temporal_datasets_blocking_with_loaded_candles(
            AttestVcetTemporalDatasetsRequestV1 {
                datasets: first
                    .datasets
                    .iter()
                    .map(|dataset| AttestVcetTemporalDatasetInputV1 {
                        dataset_id: dataset.dataset_id.clone(),
                        loaded_data_id: dataset.loaded_data_id.clone(),
                        source_dataset_sha256: dataset.source_dataset_sha256.clone(),
                        role: dataset.role,
                    })
                    .collect(),
            },
            BTreeMap::from([
                ("loaded-aapl".to_string(), Arc::new(candles(9.0))),
                ("loaded-msft".to_string(), Arc::new(candles(2.0))),
            ]),
        )
        .expect("changed attestation");
        assert_ne!(first.attestation_sha256, changed.attestation_sha256);
        assert_ne!(
            first.datasets[0].loaded_candles_snapshot_sha256,
            changed.datasets[0].loaded_candles_snapshot_sha256
        );
    }

    #[test]
    fn universe_rejects_duplicates_missing_roles_temporal_drift_and_bad_specs() {
        let mut duplicate = seal_request();
        duplicate.datasets[2].dataset_id = "MSFT".to_string();
        refresh_row_attestation(&mut duplicate.datasets[2]);
        assert!(seal_vcet_temporal_dataset_universe_blocking(duplicate)
            .unwrap_err()
            .contains("duplicate"));

        let mut role = seal_request();
        for dataset in &mut role.datasets {
            dataset.role = VcetTemporalDatasetCohortRoleV1::Selection;
            refresh_row_attestation(dataset);
        }
        assert!(seal_vcet_temporal_dataset_universe_blocking(role)
            .unwrap_err()
            .contains("at least one selection dataset and one final_test"));

        let mut interval = seal_request();
        interval.datasets[0].temporal_interval_fingerprint_sha256 = hash('8');
        assert!(seal_vcet_temporal_dataset_universe_blocking(interval)
            .unwrap_err()
            .contains("common temporal interval fingerprint"));

        let mut folds = seal_request();
        folds.datasets[0].fold_count = 4;
        assert!(seal_vcet_temporal_dataset_universe_blocking(folds)
            .unwrap_err()
            .contains("common fold_count"));

        let mut source = seal_request();
        source.execution_spec.ma_source = "not-a-source".to_string();
        assert!(seal_vcet_temporal_dataset_universe_blocking(source)
            .unwrap_err()
            .contains("ma_source"));

        let mut backend = seal_request();
        backend.execution_spec.backend = Backend::Auto { device_id: 0 };
        assert!(seal_vcet_temporal_dataset_universe_blocking(backend)
            .unwrap_err()
            .contains("strict GpuKernel"));

        let mut overlays = seal_request();
        overlays.execution_spec.strategy.stop_loss_pct = Some(0.02);
        assert!(seal_vcet_temporal_dataset_universe_blocking(overlays)
            .unwrap_err()
            .contains("does not support execution overlays"));

        let mut invalid_strategy = seal_request();
        invalid_strategy.execution_spec.strategy.trade_on_next_bar = false;
        assert!(
            seal_vcet_temporal_dataset_universe_blocking(invalid_strategy)
                .unwrap_err()
                .contains("trade_on_next_bar=false")
        );

        let mut admission = seal_request();
        admission
            .execution_spec
            .cross_universe_admission
            .min_profitable_dataset_share = 1.1;
        assert!(seal_vcet_temporal_dataset_universe_blocking(admission)
            .unwrap_err()
            .contains("min_profitable_dataset_share"));
    }

    #[test]
    fn universe_rejects_forged_row_attestation_digest() {
        let mut request = seal_request();
        request.datasets[0].dataset_attestation_sha256 = hash('0');
        let error = seal_vcet_temporal_dataset_universe_blocking(request).unwrap_err();
        assert!(error.contains("row attestation SHA-256 mismatch"));
    }

    #[test]
    fn universe_rejects_reused_source_and_loaded_snapshot_hashes() {
        let mut duplicate_source = seal_request();
        duplicate_source.datasets[1].source_dataset_sha256 =
            duplicate_source.datasets[0].source_dataset_sha256.clone();
        refresh_row_attestation(&mut duplicate_source.datasets[1]);
        let source_error =
            seal_vcet_temporal_dataset_universe_blocking(duplicate_source).unwrap_err();
        assert!(source_error.contains("cannot reuse source_dataset_sha256"));

        let mut duplicate_snapshot = seal_request();
        duplicate_snapshot.datasets[1].loaded_candles_snapshot_sha256 = duplicate_snapshot.datasets
            [0]
        .loaded_candles_snapshot_sha256
        .clone();
        refresh_row_attestation(&mut duplicate_snapshot.datasets[1]);
        let snapshot_error =
            seal_vcet_temporal_dataset_universe_blocking(duplicate_snapshot).unwrap_err();
        assert!(snapshot_error.contains("cannot reuse loaded_candles_snapshot_sha256"));
    }

    #[test]
    fn universe_rejects_excessive_shard_descriptor_materialization() {
        let datasets = vec![
            universe_dataset("AAPL", VcetTemporalDatasetCohortRoleV1::Selection),
            universe_dataset("MSFT-FINAL", VcetTemporalDatasetCohortRoleV1::FinalTest),
        ];
        let error = derive_universe_shards(
            &datasets,
            MAX_VCET_TEMPORAL_UNIVERSE_SHARD_DESCRIPTORS,
            &hash('a'),
            &hash('b'),
            &hash('9'),
            MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS / 2,
            VcetTemporalShardPolicyV1::default(),
        )
        .unwrap_err();
        assert!(error.contains("exceeding the bounded limit of 4096"));
    }

    #[test]
    fn attestation_is_bounded_to_one_execution_shard() {
        let datasets = (0..=MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST)
            .map(|index| AttestVcetTemporalDatasetInputV1 {
                dataset_id: format!("dataset-{index}"),
                loaded_data_id: format!("loaded-{index}"),
                source_dataset_sha256: hash('a'),
                role: VcetTemporalDatasetCohortRoleV1::Selection,
            })
            .collect();
        let error = attest_vcet_temporal_datasets_blocking_with_loaded_candles(
            AttestVcetTemporalDatasetsRequestV1 { datasets },
            BTreeMap::new(),
        )
        .unwrap_err();
        assert!(error.contains("at most 64 loaded datasets"));
    }

    #[test]
    fn interval_fingerprint_ignores_dataset_id_but_binds_canonical_layout() {
        let plan = plan(VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen);
        let timestamps = (0..=1_000).map(|index| index * 1_000).collect::<Vec<_>>();
        let a = plan_vcet_temporal_folds(&plan, "AAPL", &timestamps, None).expect("AAPL plan");
        let b = plan_vcet_temporal_folds(&plan, "MSFT", &timestamps, None).expect("MSFT plan");
        assert_eq!(
            recompute_vcet_temporal_interval_fingerprint_sha256(&a).expect("AAPL fingerprint"),
            recompute_vcet_temporal_interval_fingerprint_sha256(&b).expect("MSFT fingerprint")
        );
    }

    #[test]
    fn interval_fingerprint_ignores_unscored_prefix_history_offsets() {
        let mut plan = plan(VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen);
        plan.requested_bounds.utc.start_utc_ms_inclusive = 100_000;
        let selected_only = (100..=1_000).map(|index| index * 1_000).collect::<Vec<_>>();
        let with_prefix = (0..=1_000).map(|index| index * 1_000).collect::<Vec<_>>();
        let a = plan_vcet_temporal_folds(&plan, "AAPL", &selected_only, None)
            .expect("selected-only plan");
        let b = plan_vcet_temporal_folds(&plan, "MSFT", &with_prefix, None)
            .expect("prefix-history plan");
        assert_ne!(
            a.dataset.selected_bars.start_index,
            b.dataset.selected_bars.start_index
        );
        assert_eq!(
            recompute_vcet_temporal_interval_fingerprint_sha256(&a)
                .expect("selected-only fingerprint"),
            recompute_vcet_temporal_interval_fingerprint_sha256(&b)
                .expect("prefix-history fingerprint")
        );
    }

    #[test]
    fn selection_shards_split_candidate_axis_to_preserve_trial_bound() {
        let datasets = vec![
            universe_dataset("AAPL", VcetTemporalDatasetCohortRoleV1::Selection),
            universe_dataset("MSFT", VcetTemporalDatasetCohortRoleV1::Selection),
        ];
        let candidate_count = 8_192;
        let shards = derive_universe_shards(
            &datasets,
            candidate_count,
            &hash('a'),
            &"b".repeat(64),
            &hash('9'),
            100,
            VcetTemporalShardPolicyV1::default(),
        )
        .expect("two-dimensional shards");
        assert!(shards.len() > 1);
        assert_eq!(shards[0].candidate_start_index, 0);
        assert_eq!(
            shards
                .last()
                .expect("last shard")
                .candidate_end_index_exclusive,
            candidate_count
        );
        for pair in shards.windows(2) {
            assert_eq!(
                pair[0].candidate_end_index_exclusive,
                pair[1].candidate_start_index
            );
        }
        assert!(shards.iter().all(|shard| {
            shard.expected_candidate_window_trial_count
                == shard.dataset_ids.len()
                    * (shard.candidate_end_index_exclusive - shard.candidate_start_index)
                    * 100
                    * 2
                && shard.expected_candidate_window_trial_count
                    <= MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS
        }));
    }
}
