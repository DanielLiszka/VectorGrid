//! Shareable XLSX evidence report for the sealed, phased VCET temporal workflow.
//!
//! This module is intentionally a report projection, not an execution path. The public builder
//! validates every supplied evidence artifact before projecting it into a bounded workbook.

use super::super::commands_vcet_temporal::VcetTemporalCandidateEntryV1;
use super::super::commands_vcet_temporal_final::{
    validate_vcet_temporal_final_reduction_artifact, ReduceVcetTemporalFinalTestShardsResponseV1,
    VcetTemporalFinalShardReductionEvidenceV1, VcetTemporalFinalTestDatasetExecutionV1,
};
use super::super::commands_vcet_temporal_prefinal::{
    validate_vcet_temporal_pre_final_reduction_against_universe,
    ReduceVcetTemporalPreFinalShardsResponseV1, VcetTemporalFinalShardAuthorizationV1,
    VcetTemporalGlobalCandidateEvidenceV1, VcetTemporalPreFinalShardReductionEvidenceV1,
};
use super::super::commands_vcet_temporal_universe::{
    universe_shards_for_role, validate_sealed_vcet_temporal_universe,
    VcetTemporalDatasetCohortRoleV1, VcetTemporalDatasetUniversePayloadV1,
    VcetTemporalSealedDatasetUniverseV1, VcetTemporalUniverseDatasetV1,
    VcetTemporalUniverseShardV1,
};
use rust_xlsxwriter::{
    Chart, ChartFormat, ChartMarker, ChartMarkerType, ChartSolidFill, ChartType, Color,
    ConditionalFormat3ColorScale, ConditionalFormatCell, ConditionalFormatCellRule,
    ConditionalFormatDataBar, ConditionalFormatType, DocProperties, ExcelDateTime, Format,
    FormatAlign, FormatBorder, Table, TableColumn, TableStyle, Workbook, Worksheet, XlsxError,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const XLSX_MAX_ROW_INDEX: usize = 1_048_575;
const XLSX_CELL_CHAR_LIMIT: usize = 32_767;
const XLSX_EXACT_INTEGER_LIMIT: u64 = 9_007_199_254_740_991;
const CANDIDATE_ROW_CAP: usize = ta_app_core::vcet_search::MAX_VCET_SEARCH_BUDGET;
const DATASET_ROW_CAP: usize =
    super::super::commands_vcet_temporal::MAX_VCET_TEMPORAL_DATASETS_PER_REQUEST;
const LIMITATION_ROW_CAP: usize = 512;
const TRUNCATION_SUFFIX: &str = " … [truncated for Excel]";

const SUMMARY_SHEET: &str = "Summary";
const CANDIDATE_SHEET: &str = "Candidate Selection";
const COVERAGE_SHEET: &str = "Dataset Coverage";
const FINAL_SHEET: &str = "Final Test";
const PROVENANCE_SHEET: &str = "Provenance";

#[derive(Clone)]
struct ReportFormats {
    title: Format,
    subtitle: Format,
    section: Format,
    header: Format,
    label: Format,
    text: Format,
    wrap: Format,
    mono: Format,
    integer: Format,
    decimal: Format,
    pnl: Format,
    percent: Format,
    datetime: Format,
    pass: Format,
    warn: Format,
    fail: Format,
    kpi_label: Format,
    kpi_text: Format,
    kpi_number: Format,
    kpi_percent: Format,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BaseArtifactLineage {
    universe_sha256: String,
    candidate_manifest_sha256: String,
    plan_sha256: String,
    temporal_interval_fingerprint_sha256: String,
}

#[derive(Clone, Debug)]
struct CandidateReportRow {
    ordinal: usize,
    candidate_id: String,
    selected: bool,
    admitted: bool,
    coverage_share: f64,
    evaluated_share: f64,
    profitable_share: f64,
    hard_support_share: f64,
    metric_support_share: f64,
    validation_pnl_p10: Option<f64>,
    validation_sharpe_p10: Option<f64>,
    mean_validation_sharpe: Option<f64>,
    score_gap_to_winner: Option<f64>,
    validation_observations: usize,
    total_train_trades: u64,
    total_validation_trades: u64,
    support_failures: usize,
    non_finite_metrics: usize,
    vidya_short_period: Option<u16>,
    vidya_long_period: Option<u16>,
    vidya_alpha: Option<f64>,
    ce_period: Option<u16>,
    ce_mult: Option<f64>,
    ce_use_close: Option<bool>,
    require_vidya_slope: Option<bool>,
    max_trade_bars: Option<u16>,
    params_json: String,
}

#[derive(Clone, Debug)]
struct CoverageReportRow {
    role: String,
    dataset_id: String,
    timestamp_count: usize,
    requested_start_ms: Option<i64>,
    requested_end_exclusive_ms: Option<i64>,
    observed_min_ms: Option<i64>,
    observed_max_ms: Option<i64>,
    observed_span_days: Option<f64>,
    within_requested: Option<usize>,
    before_requested: Option<usize>,
    after_requested: Option<usize>,
    strictly_increasing: bool,
    duplicate_intervals: usize,
    descending_intervals: usize,
    gap_status: String,
    gap_basis: String,
    cadence_used_ms: Option<i64>,
    analyzed_intervals: usize,
    irregular_intervals: Option<usize>,
    gap_intervals: Option<usize>,
    missing_intervals: Option<u64>,
    longest_gap_minutes: Option<f64>,
    fold_count: usize,
    coverage_sha256: String,
}

#[derive(Clone, Debug)]
struct FinalReportRow {
    dataset_id: String,
    support_passed: bool,
    pnl: f64,
    sharpe: f64,
    sortino: f64,
    max_drawdown: f64,
    trades: u64,
    exposure: f64,
    net_exposure: f64,
    final_start_utc_ms: i64,
    final_end_utc_ms: i64,
    scored_bars: usize,
    warmup_bars: usize,
    backend_route: String,
    device_id: Option<u32>,
    metric_contract: String,
    unavailable_metric_fields: String,
    source_dataset_sha256: String,
    loaded_snapshot_sha256: String,
    coverage_sha256: String,
    attestation_sha256: String,
}

#[derive(Clone, Debug)]
struct EvidenceReportRow {
    stage: String,
    evidence_kind: String,
    sha256: String,
    status: String,
}

#[derive(Clone, Debug)]
struct AdmissionReportRow {
    metric: String,
    threshold: f64,
    winner_observed: Option<f64>,
    passed: bool,
    format_as_percent: bool,
}

#[derive(Clone, Debug)]
struct TemporalReportModel {
    source_campaign_manifest_hash: String,
    source_candidate_manifest_hash: String,
    source_search_id: Option<String>,
    execution_spec_json: String,
    strategy_json: String,
    requested_backend: String,
    ma_source: String,
    selection_reduction_contract: String,
    candidate_count: usize,
    selection_dataset_count: usize,
    final_dataset_count: usize,
    fold_count_per_dataset: usize,
    pre_final_trial_count: usize,
    final_trial_count: Option<usize>,
    winner_candidate_id: String,
    winner_ordinal: usize,
    winner_score: f64,
    runner_up_candidate_id: Option<String>,
    runner_up_score: Option<f64>,
    winner_margin: Option<f64>,
    candidate_id_tie_break_used: bool,
    winner_params_json: String,
    winner_profitable_share: f64,
    winner_validation_pnl_p10: Option<f64>,
    winner_validation_sharpe_p10: Option<f64>,
    final_complete: bool,
    final_support_passed: Option<bool>,
    final_mean_pnl: Option<f64>,
    final_mean_sharpe: Option<f64>,
    final_support_share: Option<f64>,
    candidate_rows: Vec<CandidateReportRow>,
    coverage_rows: Vec<CoverageReportRow>,
    final_rows: Vec<FinalReportRow>,
    evidence_rows: Vec<EvidenceReportRow>,
    admission_rows: Vec<AdmissionReportRow>,
    limitations: Vec<(String, String)>,
}

fn excel_safe_text(value: &str) -> String {
    if value.chars().count() <= XLSX_CELL_CHAR_LIMIT {
        return value.to_string();
    }
    let keep = XLSX_CELL_CHAR_LIMIT.saturating_sub(TRUNCATION_SUFFIX.chars().count());
    let mut out: String = value.chars().take(keep).collect();
    out.push_str(TRUNCATION_SUFFIX);
    out
}

fn enum_label(value: &impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn serialized_eq(left: &impl Serialize, right: &impl Serialize) -> Result<bool, String> {
    let left = serde_json::to_vec(left)
        .map_err(|error| format!("failed to serialize report lineage authority: {error}"))?;
    let right = serde_json::to_vec(right)
        .map_err(|error| format!("failed to serialize report lineage observation: {error}"))?;
    Ok(left == right)
}

fn ensure_base_lineage_matches(
    authority: &BaseArtifactLineage,
    observed: &BaseArtifactLineage,
    stage: &str,
) -> Result<(), String> {
    for (field, expected, actual) in [
        (
            "universe_sha256",
            authority.universe_sha256.as_str(),
            observed.universe_sha256.as_str(),
        ),
        (
            "candidate_manifest_sha256",
            authority.candidate_manifest_sha256.as_str(),
            observed.candidate_manifest_sha256.as_str(),
        ),
        (
            "plan_sha256",
            authority.plan_sha256.as_str(),
            observed.plan_sha256.as_str(),
        ),
        (
            "temporal_interval_fingerprint_sha256",
            authority.temporal_interval_fingerprint_sha256.as_str(),
            observed.temporal_interval_fingerprint_sha256.as_str(),
        ),
    ] {
        if actual != expected {
            return Err(format!(
                "VCET temporal XLSX {stage} lineage mismatch for {field} (expected {expected}, actual {actual})"
            ));
        }
    }
    Ok(())
}

fn validate_candidate_manifest_binding(
    expected: &VcetTemporalCandidateEntryV1,
    observed: &VcetTemporalGlobalCandidateEvidenceV1,
    strategy: &ta_strategies::double_ma::StrategyConfig,
) -> Result<(), String> {
    let expected_params =
        super::super::commands_vcet_temporal_execute::candidate_params_from_manifest(
            &expected.identity_inputs,
            strategy,
        );
    if observed.global_candidate_ordinal != expected.ordinal
        || observed.candidate_id != expected.candidate_id
        || !serialized_eq(&observed.params, &expected_params)?
    {
        return Err(format!(
            "VCET temporal XLSX pre-final candidate at manifest ordinal {} does not exactly match its sealed ordinal, ID, or effective parameters",
            expected.ordinal
        ));
    }
    Ok(())
}

fn validate_selection_shard_binding(
    expected: &VcetTemporalUniverseShardV1,
    observed: &VcetTemporalPreFinalShardReductionEvidenceV1,
) -> Result<(), String> {
    let expected_candidate_count = expected
        .candidate_end_index_exclusive
        .checked_sub(expected.candidate_start_index)
        .ok_or_else(|| {
            format!(
                "VCET temporal XLSX sealed selection shard '{}' has an invalid candidate window",
                expected.shard_id
            )
        })?;
    if expected.role != VcetTemporalDatasetCohortRoleV1::Selection
        || expected.locked_winner_only
        || observed.shard_ordinal != expected.shard_ordinal
        || observed.shard_id != expected.shard_id
        || observed.dataset_count != expected.dataset_ids.len()
        || observed.candidate_count != expected_candidate_count
        || observed.candidate_window_trial_count != expected.expected_candidate_window_trial_count
    {
        return Err(format!(
            "VCET temporal XLSX pre-final shard '{}' does not exactly match its sealed selection descriptor",
            observed.shard_id
        ));
    }
    Ok(())
}

fn validate_final_dataset_binding(
    expected: &VcetTemporalUniverseDatasetV1,
    observed: &VcetTemporalFinalTestDatasetExecutionV1,
    winner: &super::super::commands_vcet_temporal_prefinal::VcetTemporalGlobalWinnerLockV1,
    expected_minimum_trades: u64,
    expected_scored_bars: usize,
) -> Result<(), String> {
    if expected.role != VcetTemporalDatasetCohortRoleV1::FinalTest
        || observed.dataset_id != expected.dataset_id
        || observed.source_dataset_sha256 != expected.source_dataset_sha256
        || observed.loaded_candles_snapshot_sha256 != expected.loaded_candles_snapshot_sha256
        || observed.dataset_attestation_sha256 != expected.dataset_attestation_sha256
        || observed.timeline_sha256 != expected.timeline_sha256
        || observed.temporal_interval_fingerprint_sha256
            != expected.temporal_interval_fingerprint_sha256
        || observed.dataset_coverage_sha256 != expected.dataset_coverage_sha256
        || observed.dataset_coverage != expected.dataset_coverage
        || observed.global_candidate_ordinal != winner.global_candidate_ordinal
        || observed.candidate_id != winner.candidate_id
        || !serialized_eq(&observed.params, &winner.params)?
        || observed.minimum_trades != expected_minimum_trades
        || observed.final_test.scored_bars.len() != expected_scored_bars
    {
        return Err(format!(
            "VCET temporal XLSX final dataset '{}' does not exactly match its sealed identity, attestation, timeline, coverage, final-window support contract, or locked-winner parameters",
            observed.dataset_id
        ));
    }
    Ok(())
}

fn validate_final_shard_binding(
    expected: &VcetTemporalUniverseShardV1,
    authorization: &VcetTemporalFinalShardAuthorizationV1,
    observed: &VcetTemporalFinalShardReductionEvidenceV1,
    observed_datasets: &[VcetTemporalFinalTestDatasetExecutionV1],
) -> Result<(), String> {
    let observed_dataset_ids = observed_datasets
        .iter()
        .map(|dataset| dataset.dataset_id.as_str())
        .collect::<Vec<_>>();
    let expected_dataset_ids = expected
        .dataset_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let expected_support = observed_datasets
        .iter()
        .all(|dataset| dataset.minimum_trade_support_passed);
    if expected.role != VcetTemporalDatasetCohortRoleV1::FinalTest
        || !expected.locked_winner_only
        || !serialized_eq(&authorization.shard, expected)?
        || observed.shard_ordinal != expected.shard_ordinal
        || observed.shard_id != expected.shard_id
        || observed.authorization_sha256 != authorization.authorization_sha256
        || observed.authorized_prior_state_sha256 != authorization.expected_prior_state_sha256
        || observed.dataset_count != expected.dataset_ids.len()
        || observed_dataset_ids != expected_dataset_ids
        || observed.candidate_window_trial_count != expected.expected_candidate_window_trial_count
        || observed.minimum_trade_support_passed != expected_support
    {
        return Err(format!(
            "VCET temporal XLSX final shard '{}' does not exactly match its sealed descriptor or pre-final authorization",
            observed.shard_id
        ));
    }
    Ok(())
}

fn validate_pre_final_bindings(
    universe: &VcetTemporalDatasetUniversePayloadV1,
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
) -> Result<(), String> {
    let pre = &pre_final.payload;
    let manifest = &universe.candidate_manifest.payload.candidates;
    if pre.candidates.len() != manifest.len() {
        return Err(format!(
            "VCET temporal XLSX pre-final candidate count {} differs from sealed manifest count {}",
            pre.candidates.len(),
            manifest.len()
        ));
    }
    for (expected, observed) in manifest.iter().zip(&pre.candidates) {
        validate_candidate_manifest_binding(expected, observed, &universe.execution_spec.strategy)?;
    }

    let expected_shards =
        universe_shards_for_role(universe, VcetTemporalDatasetCohortRoleV1::Selection);
    if pre.ordered_shards.len() != expected_shards.len() {
        return Err(format!(
            "VCET temporal XLSX pre-final selection shard count {} differs from sealed count {}",
            pre.ordered_shards.len(),
            expected_shards.len()
        ));
    }
    for (expected, observed) in expected_shards.into_iter().zip(&pre.ordered_shards) {
        validate_selection_shard_binding(expected, observed)?;
    }
    Ok(())
}

fn validate_final_bindings(
    universe: &VcetTemporalDatasetUniversePayloadV1,
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
    final_test: &ReduceVcetTemporalFinalTestShardsResponseV1,
) -> Result<(), String> {
    let final_payload = &final_test.payload;
    let expected_datasets = universe
        .datasets
        .iter()
        .filter(|dataset| dataset.role == VcetTemporalDatasetCohortRoleV1::FinalTest)
        .map(|dataset| (dataset.dataset_id.as_str(), dataset))
        .collect::<BTreeMap<_, _>>();
    let expected_dataset_ids = universe
        .datasets
        .iter()
        .filter(|dataset| dataset.role == VcetTemporalDatasetCohortRoleV1::FinalTest)
        .map(|dataset| dataset.dataset_id.as_str())
        .collect::<Vec<_>>();
    let observed_dataset_ids = final_payload
        .datasets
        .iter()
        .map(|dataset| dataset.dataset_id.as_str())
        .collect::<Vec<_>>();
    let datasets_by_id = final_payload
        .datasets
        .iter()
        .map(|dataset| (dataset.dataset_id.as_str(), dataset))
        .collect::<BTreeMap<_, _>>();
    if observed_dataset_ids != expected_dataset_ids
        || datasets_by_id.len() != final_payload.datasets.len()
        || datasets_by_id.len() != expected_datasets.len()
    {
        return Err(
            "VCET temporal XLSX final dataset set does not exactly cover the sealed final cohort"
                .to_string(),
        );
    }
    for (dataset_id, expected) in expected_datasets {
        let observed = datasets_by_id.get(dataset_id).ok_or_else(|| {
            format!("VCET temporal XLSX final reduction omits sealed dataset '{dataset_id}'")
        })?;
        validate_final_dataset_binding(
            expected,
            observed,
            &pre_final.payload.winner_lock,
            universe.plan.minimum_support.min_final_test_trades,
            universe.plan.final_test_bars,
        )?;
    }

    let expected_shards =
        universe_shards_for_role(universe, VcetTemporalDatasetCohortRoleV1::FinalTest);
    let authorizations = &pre_final.payload.final_shard_authorizations;
    if final_payload.shards.len() != expected_shards.len()
        || authorizations.len() != expected_shards.len()
    {
        return Err(
            "VCET temporal XLSX final shard/authorization set does not exactly cover the sealed final cohort"
                .to_string(),
        );
    }
    let mut dataset_offset = 0usize;
    for ((expected, authorization), observed) in expected_shards
        .into_iter()
        .zip(authorizations)
        .zip(&final_payload.shards)
    {
        let dataset_end = dataset_offset
            .checked_add(expected.dataset_ids.len())
            .ok_or_else(|| "VCET temporal XLSX final shard dataset offset overflow".to_string())?;
        let observed_datasets = final_payload
            .datasets
            .get(dataset_offset..dataset_end)
            .ok_or_else(|| {
                format!(
                    "VCET temporal XLSX final shard '{}' dataset partition is out of bounds",
                    expected.shard_id
                )
            })?;
        validate_final_shard_binding(expected, authorization, observed, observed_datasets)?;
        dataset_offset = dataset_end;
    }
    if dataset_offset != final_payload.datasets.len() {
        return Err(
            "VCET temporal XLSX final shard partitions do not exactly cover the final cohort"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_report_inputs(
    sealed_universe: &VcetTemporalSealedDatasetUniverseV1,
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
    final_test: Option<&ReduceVcetTemporalFinalTestShardsResponseV1>,
) -> Result<(), String> {
    validate_sealed_vcet_temporal_universe(sealed_universe, &sealed_universe.universe_sha256)?;
    validate_vcet_temporal_pre_final_reduction_against_universe(pre_final, sealed_universe)?;

    let universe = &sealed_universe.payload;
    let pre = &pre_final.payload;
    let authority = BaseArtifactLineage {
        universe_sha256: sealed_universe.universe_sha256.clone(),
        candidate_manifest_sha256: universe.candidate_manifest_sha256.clone(),
        plan_sha256: universe.plan_identity.plan_sha256.clone(),
        temporal_interval_fingerprint_sha256: universe.temporal_interval_fingerprint_sha256.clone(),
    };
    let pre_lineage = BaseArtifactLineage {
        universe_sha256: pre.universe_sha256.clone(),
        candidate_manifest_sha256: pre.candidate_manifest_sha256.clone(),
        plan_sha256: pre.plan_sha256.clone(),
        temporal_interval_fingerprint_sha256: pre.temporal_interval_fingerprint_sha256.clone(),
    };
    ensure_base_lineage_matches(&authority, &pre_lineage, "pre-final reduction")?;
    validate_pre_final_bindings(universe, pre_final)?;
    if !serialized_eq(&universe.execution_spec, &pre.execution_spec)? {
        return Err(
            "VCET temporal XLSX pre-final execution spec differs from the sealed universe"
                .to_string(),
        );
    }
    if pre.requested_selection_dataset_count != universe.selection_dataset_count
        || pre.covered_selection_dataset_count != universe.selection_dataset_count
        || pre.candidates.len() != universe.candidate_manifest.payload.candidates.len()
    {
        return Err(
            "VCET temporal XLSX pre-final candidate/dataset coverage differs from the sealed universe"
                .to_string(),
        );
    }

    if let Some(final_test) = final_test {
        validate_vcet_temporal_final_reduction_artifact(final_test)?;
        let final_payload = &final_test.payload;
        let final_lineage = BaseArtifactLineage {
            universe_sha256: final_payload.universe_sha256.clone(),
            candidate_manifest_sha256: pre.candidate_manifest_sha256.clone(),
            plan_sha256: pre.plan_sha256.clone(),
            temporal_interval_fingerprint_sha256: final_payload
                .temporal_interval_fingerprint_sha256
                .clone(),
        };
        ensure_base_lineage_matches(&authority, &final_lineage, "final-test reduction")?;
        if final_payload.global_winner_reduction_artifact_sha256 != pre_final.artifact_sha256
            || final_payload.global_pre_final_evidence_sha256
                != pre.global_pre_final_evidence_sha256
            || final_payload.execution_spec_sha256 != pre.execution_spec_sha256
            || final_payload.final_stage_nonce != pre.final_stage_nonce
            || final_payload.winner_lock.winner_lock_sha256 != pre.winner_lock.winner_lock_sha256
            || !serialized_eq(&final_payload.winner_lock, &pre.winner_lock)?
            || !serialized_eq(&final_payload.execution_spec, &pre.execution_spec)?
            || final_payload.expected_final_test_dataset_count != universe.final_test_dataset_count
        {
            return Err(
                "VCET temporal XLSX final-test authority does not exactly match the locked pre-final winner and sealed universe"
                    .to_string(),
            );
        }
        validate_final_bindings(universe, pre_final, final_test)?;
    }
    Ok(())
}

fn validate_row_budget(sheet: &str, header_row: usize, row_count: usize) -> Result<(), String> {
    let last_row = header_row
        .checked_add(row_count)
        .ok_or_else(|| format!("{sheet} XLSX row calculation overflowed"))?;
    if last_row > XLSX_MAX_ROW_INDEX {
        return Err(format!(
            "{sheet} requires zero-based row index {last_row}; Excel supports at most {XLSX_MAX_ROW_INDEX}"
        ));
    }
    Ok(())
}

fn validate_model_limits(model: &TemporalReportModel) -> Result<(), String> {
    if model.candidate_rows.is_empty() || model.candidate_rows.len() > CANDIDATE_ROW_CAP {
        return Err(format!(
            "VCET temporal XLSX candidate rows must stay within 1..={CANDIDATE_ROW_CAP}; received {}",
            model.candidate_rows.len()
        ));
    }
    if model.candidate_count != model.candidate_rows.len()
        || model
            .candidate_rows
            .iter()
            .filter(|row| row.selected)
            .count()
            != 1
    {
        return Err(
            "VCET temporal XLSX candidate model must contain exactly one selected row and a matching candidate count"
                .to_string(),
        );
    }
    if model.coverage_rows.is_empty() || model.coverage_rows.len() > DATASET_ROW_CAP {
        return Err(format!(
            "VCET temporal XLSX coverage rows must stay within 1..={DATASET_ROW_CAP}; received {}",
            model.coverage_rows.len()
        ));
    }
    if model
        .selection_dataset_count
        .checked_add(model.final_dataset_count)
        != Some(model.coverage_rows.len())
    {
        return Err(
            "VCET temporal XLSX cohort counts do not exactly cover the dataset rows".to_string(),
        );
    }
    if model.final_rows.len() > DATASET_ROW_CAP {
        return Err(format!(
            "VCET temporal XLSX final rows exceed the {DATASET_ROW_CAP}-row safety cap"
        ));
    }
    if model.final_complete != !model.final_rows.is_empty()
        || (model.final_complete && model.final_rows.len() != model.final_dataset_count)
    {
        return Err(
            "VCET temporal XLSX final completion state does not match its final dataset rows"
                .to_string(),
        );
    }
    if model.limitations.len() > LIMITATION_ROW_CAP {
        return Err(format!(
            "VCET temporal XLSX limitations exceed the {LIMITATION_ROW_CAP}-row safety cap"
        ));
    }
    validate_row_budget(CANDIDATE_SHEET, 19, model.candidate_rows.len())?;
    validate_row_budget(COVERAGE_SHEET, 6, model.coverage_rows.len())?;
    validate_row_budget(FINAL_SHEET, 10, model.final_rows.len())?;
    let provenance_prefix = 19usize
        .checked_add(model.evidence_rows.len())
        .and_then(|rows| rows.checked_add(model.admission_rows.len()))
        .ok_or_else(|| "VCET temporal XLSX provenance row calculation overflowed".to_string())?;
    validate_row_budget(PROVENANCE_SHEET, provenance_prefix, model.limitations.len())?;
    Ok(())
}

fn backend_label(backend: &ta_app_core::double_ma::Backend) -> String {
    match backend {
        ta_app_core::double_ma::Backend::Auto { device_id } => {
            format!("Auto (device {device_id})")
        }
        ta_app_core::double_ma::Backend::CpuOnly => "CPU only".to_string(),
        ta_app_core::double_ma::Backend::GpuOnly { device_id } => {
            format!("GPU only (device {device_id})")
        }
        ta_app_core::double_ma::Backend::GpuHybridKernel { device_id } => {
            format!("GPU hybrid kernel (device {device_id})")
        }
        ta_app_core::double_ma::Backend::GpuKernel { device_id } => {
            format!("GPU kernel (device {device_id})")
        }
    }
}

fn project_candidate(
    candidate: &VcetTemporalGlobalCandidateEvidenceV1,
    winner_score: f64,
) -> Result<CandidateReportRow, String> {
    let vcet = candidate.params.vcet.as_ref();
    let score_gap_to_winner = candidate.mean_validation_sharpe.and_then(|score| {
        let gap = score - winner_score;
        gap.is_finite().then_some(gap)
    });
    Ok(CandidateReportRow {
        ordinal: candidate.global_candidate_ordinal,
        candidate_id: candidate.candidate_id.clone(),
        selected: candidate.selected_before_final_test,
        admitted: candidate.admission_passed,
        coverage_share: candidate.requested_dataset_coverage_share,
        evaluated_share: candidate.evaluated_dataset_share_requested,
        profitable_share: candidate.profitable_dataset_share_requested,
        hard_support_share: candidate.hard_support_dataset_share_requested,
        metric_support_share: candidate.metric_support_dataset_share_requested,
        validation_pnl_p10: candidate.validation_pnl_p10,
        validation_sharpe_p10: candidate.validation_sharpe_p10,
        mean_validation_sharpe: candidate.mean_validation_sharpe,
        score_gap_to_winner,
        validation_observations: candidate.validation_observation_count,
        total_train_trades: candidate.total_train_trades,
        total_validation_trades: candidate.total_validation_trades,
        support_failures: candidate.support_failure_count,
        non_finite_metrics: candidate.non_finite_metric_count,
        vidya_short_period: vcet.map(|params| params.vidya_short_period),
        vidya_long_period: vcet.map(|params| params.vidya_long_period),
        vidya_alpha: vcet.map(|params| params.vidya_alpha),
        ce_period: vcet.map(|params| params.ce_period),
        ce_mult: vcet.map(|params| params.ce_mult),
        ce_use_close: vcet.map(|params| params.ce_use_close),
        require_vidya_slope: vcet.map(|params| params.require_vidya_slope),
        max_trade_bars: vcet.map(|params| params.max_trade_bars),
        params_json: serde_json::to_string(&candidate.params)
            .map_err(|error| format!("failed to serialize VCET candidate params: {error}"))?,
    })
}

fn build_report_model(
    sealed_universe: &VcetTemporalSealedDatasetUniverseV1,
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
    final_test: Option<&ReduceVcetTemporalFinalTestShardsResponseV1>,
) -> Result<TemporalReportModel, String> {
    let universe = &sealed_universe.payload;
    let pre = &pre_final.payload;
    let winner = &pre.winner_lock;
    let winner_candidate = pre
        .candidates
        .iter()
        .find(|candidate| {
            candidate.global_candidate_ordinal == winner.global_candidate_ordinal
                && candidate.candidate_id == winner.candidate_id
        })
        .ok_or_else(|| "VCET temporal XLSX could not resolve the locked winner row".to_string())?;

    let candidate_rows = pre
        .candidates
        .iter()
        .map(|candidate| project_candidate(candidate, winner.selection_score))
        .collect::<Result<Vec<_>, _>>()?;
    let coverage_rows = universe
        .datasets
        .iter()
        .map(|dataset| {
            let coverage = &dataset.dataset_coverage;
            CoverageReportRow {
                role: match dataset.role {
                    VcetTemporalDatasetCohortRoleV1::Selection => "Selection".to_string(),
                    VcetTemporalDatasetCohortRoleV1::FinalTest => "Final test".to_string(),
                },
                dataset_id: dataset.dataset_id.clone(),
                timestamp_count: coverage.timestamp_count,
                requested_start_ms: coverage.requested_bounds.map(|bounds| bounds.start_ms),
                requested_end_exclusive_ms: coverage
                    .requested_bounds
                    .map(|bounds| bounds.end_exclusive_ms),
                observed_min_ms: coverage.observed_bounds.map(|bounds| bounds.min_ms),
                observed_max_ms: coverage.observed_bounds.map(|bounds| bounds.max_ms),
                observed_span_days: coverage
                    .observed_bounds
                    .map(|bounds| bounds.span_ms as f64 / 86_400_000.0),
                within_requested: coverage.requested_window_counts.map(|counts| counts.within),
                before_requested: coverage
                    .requested_window_counts
                    .map(|counts| counts.before_start),
                after_requested: coverage
                    .requested_window_counts
                    .map(|counts| counts.at_or_after_end),
                strictly_increasing: coverage.ordering.strictly_increasing,
                duplicate_intervals: coverage.ordering.duplicate_interval_count,
                descending_intervals: coverage.ordering.descending_interval_count,
                gap_status: enum_label(&coverage.gaps.status),
                gap_basis: enum_label(&coverage.gaps.basis),
                cadence_used_ms: coverage.gaps.cadence_used_ms,
                analyzed_intervals: coverage.gaps.analyzed_interval_count,
                irregular_intervals: coverage.gaps.irregular_interval_count,
                gap_intervals: coverage.gaps.gap_interval_count,
                missing_intervals: coverage.gaps.missing_interval_count,
                longest_gap_minutes: coverage
                    .gaps
                    .longest_gap_ms
                    .map(|value| value as f64 / 60_000.0),
                fold_count: dataset.fold_count,
                coverage_sha256: dataset.dataset_coverage_sha256.clone(),
            }
        })
        .collect::<Vec<_>>();

    let mut final_rows = Vec::new();
    let mut final_trial_count = None;
    let mut final_support_passed = None;
    let mut final_mean_pnl = None;
    let mut final_mean_sharpe = None;
    let mut final_support_share = None;
    if let Some(final_test) = final_test {
        let payload = &final_test.payload;
        final_trial_count = Some(payload.final_test_candidate_window_trial_count);
        final_support_passed = Some(payload.minimum_trade_support_passed);
        for dataset in &payload.datasets {
            final_rows.push(FinalReportRow {
                dataset_id: dataset.dataset_id.clone(),
                support_passed: dataset.minimum_trade_support_passed,
                pnl: dataset.metrics.pnl,
                sharpe: dataset.metrics.sharpe,
                sortino: dataset.metrics.sortino,
                max_drawdown: dataset.metrics.max_dd,
                trades: u64::from(dataset.metrics.trades),
                exposure: dataset.metrics.exposure,
                net_exposure: dataset.metrics.net_exposure,
                final_start_utc_ms: dataset.final_test.scored_utc.start_utc_ms_inclusive,
                final_end_utc_ms: dataset.final_test.scored_utc.end_utc_ms_inclusive,
                scored_bars: dataset.final_test.scored_bars.len(),
                warmup_bars: dataset.final_test.warmup_bars.len(),
                backend_route: dataset.backend.backend_route.clone(),
                device_id: dataset.backend.device_id,
                metric_contract: dataset.backend.metric_contract.clone(),
                unavailable_metric_fields: dataset.backend.unavailable_metric_fields.join(", "),
                source_dataset_sha256: dataset.source_dataset_sha256.clone(),
                loaded_snapshot_sha256: dataset.loaded_candles_snapshot_sha256.clone(),
                coverage_sha256: dataset.dataset_coverage_sha256.clone(),
                attestation_sha256: dataset.dataset_attestation_sha256.clone(),
            });
        }
        if !final_rows.is_empty() {
            let denominator = final_rows.len() as f64;
            final_mean_pnl = Some(final_rows.iter().map(|row| row.pnl).sum::<f64>() / denominator);
            final_mean_sharpe =
                Some(final_rows.iter().map(|row| row.sharpe).sum::<f64>() / denominator);
            final_support_share = Some(
                final_rows.iter().filter(|row| row.support_passed).count() as f64 / denominator,
            );
        }
    }

    let mut evidence_rows = vec![
        EvidenceReportRow {
            stage: "Universe".to_string(),
            evidence_kind: universe.evidence_kind.clone(),
            sha256: sealed_universe.universe_sha256.clone(),
            status: "Verified".to_string(),
        },
        EvidenceReportRow {
            stage: "Candidate manifest".to_string(),
            evidence_kind: universe.candidate_manifest.payload.strategy_kind.clone(),
            sha256: universe.candidate_manifest_sha256.clone(),
            status: "Verified".to_string(),
        },
        EvidenceReportRow {
            stage: "Temporal plan".to_string(),
            evidence_kind: universe.plan_identity.plan_id.clone(),
            sha256: universe.plan_identity.plan_sha256.clone(),
            status: "Verified".to_string(),
        },
        EvidenceReportRow {
            stage: "Pre-final reduction".to_string(),
            evidence_kind: pre.evidence_kind.clone(),
            sha256: pre_final.artifact_sha256.clone(),
            status: "Verified".to_string(),
        },
        EvidenceReportRow {
            stage: "Global pre-final evidence".to_string(),
            evidence_kind: pre.selection_reduction_contract.clone(),
            sha256: pre.global_pre_final_evidence_sha256.clone(),
            status: "Verified".to_string(),
        },
        EvidenceReportRow {
            stage: "Winner lock".to_string(),
            evidence_kind: "locked before final-test access".to_string(),
            sha256: pre.winner_lock.winner_lock_sha256.clone(),
            status: "Verified".to_string(),
        },
    ];
    if let Some(final_test) = final_test {
        evidence_rows.push(EvidenceReportRow {
            stage: "Final-test reduction".to_string(),
            evidence_kind: final_test.payload.evidence_kind.clone(),
            sha256: final_test.artifact_sha256.clone(),
            status: "Verified".to_string(),
        });
        evidence_rows.push(EvidenceReportRow {
            stage: "Final shard-set root".to_string(),
            evidence_kind: "winner-only final scoring".to_string(),
            sha256: final_test.payload.shard_set_root_sha256.clone(),
            status: "Verified".to_string(),
        });
    } else {
        evidence_rows.push(EvidenceReportRow {
            stage: "Final-test reduction".to_string(),
            evidence_kind: "not supplied".to_string(),
            sha256: "-".to_string(),
            status: "Pending".to_string(),
        });
    }

    let admission = pre.execution_spec.cross_universe_admission;
    let admission_rows = vec![
        AdmissionReportRow {
            metric: "Requested dataset coverage share".to_string(),
            threshold: admission.min_requested_dataset_coverage_share,
            winner_observed: Some(winner_candidate.requested_dataset_coverage_share),
            passed: winner_candidate.requested_dataset_coverage_share
                >= admission.min_requested_dataset_coverage_share,
            format_as_percent: true,
        },
        AdmissionReportRow {
            metric: "Evaluated share of requested datasets".to_string(),
            threshold: admission.min_evaluated_dataset_share_requested,
            winner_observed: Some(winner_candidate.evaluated_dataset_share_requested),
            passed: winner_candidate.evaluated_dataset_share_requested
                >= admission.min_evaluated_dataset_share_requested,
            format_as_percent: true,
        },
        AdmissionReportRow {
            metric: "Profitable share of requested datasets".to_string(),
            threshold: admission.min_profitable_dataset_share,
            winner_observed: Some(winner_candidate.profitable_dataset_share_requested),
            passed: winner_candidate.profitable_dataset_share_requested
                >= admission.min_profitable_dataset_share,
            format_as_percent: true,
        },
        AdmissionReportRow {
            metric: "Hard-support share of requested datasets".to_string(),
            threshold: admission.min_hard_support_dataset_share,
            winner_observed: Some(winner_candidate.hard_support_dataset_share_requested),
            passed: winner_candidate.hard_support_dataset_share_requested
                >= admission.min_hard_support_dataset_share,
            format_as_percent: true,
        },
        AdmissionReportRow {
            metric: "Metric-support share of requested datasets".to_string(),
            threshold: admission.min_metric_support_dataset_share,
            winner_observed: Some(winner_candidate.metric_support_dataset_share_requested),
            passed: winner_candidate.metric_support_dataset_share_requested
                >= admission.min_metric_support_dataset_share,
            format_as_percent: true,
        },
        AdmissionReportRow {
            metric: "Validation PnL p10".to_string(),
            threshold: admission.min_validation_pnl_p10,
            winner_observed: winner_candidate.validation_pnl_p10,
            passed: winner_candidate
                .validation_pnl_p10
                .is_some_and(|value| value >= admission.min_validation_pnl_p10),
            format_as_percent: false,
        },
        AdmissionReportRow {
            metric: "Validation Sharpe p10".to_string(),
            threshold: admission.min_validation_sharpe_p10,
            winner_observed: winner_candidate.validation_sharpe_p10,
            passed: winner_candidate
                .validation_sharpe_p10
                .is_some_and(|value| value >= admission.min_validation_sharpe_p10),
            format_as_percent: false,
        },
    ];

    let mut limitations = Vec::new();
    for value in &universe.limitations {
        limitations.push(("Sealed universe".to_string(), value.clone()));
    }
    for value in &pre.limitations {
        limitations.push(("Pre-final reduction".to_string(), value.clone()));
    }
    if let Some(final_test) = final_test {
        for value in &final_test.payload.limitations {
            limitations.push(("Final-test reduction".to_string(), value.clone()));
        }
    } else {
        limitations.push((
            "Final-test reduction".to_string(),
            "No verified final-test reduction was supplied; final-test cells are intentionally pending and no outcome is inferred."
                .to_string(),
        ));
    }
    limitations.extend([
        (
            "Workbook".to_string(),
            "This workbook is a bounded reporting projection of verified artifacts; it is not execution authority and cannot authorize deployment or live trading."
                .to_string(),
        ),
        (
            "Selection receipt scope".to_string(),
            "The pre-final reduction embeds canonical selection-shard IDs, geometry, and immutable artifact hashes rather than each shard's per-dataset receipt rows. This report exactly binds those shard descriptors to the sealed universe; independent revalidation of omitted per-dataset shard receipts requires the original shard artifacts."
                .to_string(),
        ),
        (
            "Interpretation".to_string(),
            "Backtest profitability and cross-dataset breadth are research evidence, not a guarantee of future returns."
                .to_string(),
        ),
        (
            "Metric semantics".to_string(),
            "PnL values preserve the engine return contract and are intentionally not relabeled as currency. Trade support uses the strategy metric's transition-count basis."
                .to_string(),
        ),
        (
            "Durable authority".to_string(),
            "Desktop one-shot authorization is cooperative-process and process-crash fail-closed while the app-local ledger remains intact, and ledger records are corruption-detecting. It is not unconditional power-loss or tamper-resistant authority; adversarial local mutation and cross-machine authority are out of scope."
                .to_string(),
        ),
        (
            "Chart".to_string(),
            format!(
                "The candidate scatter includes all {} verified candidate rows; rejected candidates remain visible and the locked winner is highlighted separately.",
                candidate_rows.len()
            ),
        ),
    ]);

    let execution_spec_json = serde_json::to_string(&universe.execution_spec)
        .map_err(|error| format!("failed to serialize VCET execution spec: {error}"))?;
    let strategy_json = serde_json::to_string(&universe.execution_spec.strategy)
        .map_err(|error| format!("failed to serialize VCET StrategyConfig: {error}"))?;
    let winner_params_json = serde_json::to_string(&winner.params)
        .map_err(|error| format!("failed to serialize locked VCET winner params: {error}"))?;

    Ok(TemporalReportModel {
        source_campaign_manifest_hash: universe
            .candidate_manifest
            .payload
            .source_campaign_manifest_hash
            .clone(),
        source_candidate_manifest_hash: universe
            .candidate_manifest
            .payload
            .source_candidate_manifest_hash
            .clone(),
        source_search_id: universe.candidate_manifest.payload.source_search_id.clone(),
        execution_spec_json,
        strategy_json,
        requested_backend: backend_label(&universe.execution_spec.backend),
        ma_source: universe.execution_spec.ma_source.clone(),
        selection_reduction_contract: pre.selection_reduction_contract.clone(),
        candidate_count: candidate_rows.len(),
        selection_dataset_count: universe.selection_dataset_count,
        final_dataset_count: universe.final_test_dataset_count,
        fold_count_per_dataset: universe.fold_count_per_dataset,
        pre_final_trial_count: pre.pre_final_candidate_window_trial_count,
        final_trial_count,
        winner_candidate_id: winner.candidate_id.clone(),
        winner_ordinal: winner.global_candidate_ordinal,
        winner_score: winner.selection_score,
        runner_up_candidate_id: winner.runner_up_candidate_id.clone(),
        runner_up_score: winner.runner_up_selection_score,
        winner_margin: winner.winner_to_runner_up_score_margin,
        candidate_id_tie_break_used: winner.candidate_id_tie_break_used,
        winner_params_json,
        winner_profitable_share: winner_candidate.profitable_dataset_share_requested,
        winner_validation_pnl_p10: winner_candidate.validation_pnl_p10,
        winner_validation_sharpe_p10: winner_candidate.validation_sharpe_p10,
        final_complete: final_test.is_some(),
        final_support_passed,
        final_mean_pnl,
        final_mean_sharpe,
        final_support_share,
        candidate_rows,
        coverage_rows,
        final_rows,
        evidence_rows,
        admission_rows,
        limitations,
    })
}

fn build_formats() -> ReportFormats {
    let title = Format::new()
        .set_bold()
        .set_font_size(18)
        .set_font_color(Color::RGB(0xF8FAFC))
        .set_background_color(Color::RGB(0x0B1220))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);
    let subtitle = Format::new()
        .set_font_color(Color::RGB(0x334155))
        .set_background_color(Color::RGB(0xE2E8F0))
        .set_text_wrap()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);
    let section = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xFFFFFF))
        .set_background_color(Color::RGB(0x0F766E))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Left);
    let header = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xF8FAFC))
        .set_background_color(Color::RGB(0x1E293B))
        .set_border(FormatBorder::Thin)
        .set_text_wrap()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);
    let label = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xF1F5F9))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::VerticalCenter);
    let text = Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::VerticalCenter);
    let wrap = Format::new()
        .set_border(FormatBorder::Thin)
        .set_text_wrap()
        .set_align(FormatAlign::Left)
        .set_align(FormatAlign::Top);
    let mono = Format::new()
        .set_font_name("Consolas")
        .set_font_size(9)
        .set_border(FormatBorder::Thin)
        .set_text_wrap()
        .set_align(FormatAlign::Top);
    let integer = Format::new()
        .set_num_format("#,##0;[Red](#,##0);-")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Right);
    let decimal = Format::new()
        .set_num_format("0.0000;[Red](0.0000);-")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Right);
    let pnl = Format::new()
        .set_num_format("0.000000;[Red](0.000000);-")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Right);
    let percent = Format::new()
        .set_num_format("0.0%;[Red](0.0%);-")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Right);
    let datetime = Format::new()
        .set_num_format("yyyy-mm-dd hh:mm:ss")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Right);
    let pass = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0x166534))
        .set_background_color(Color::RGB(0xDCFCE7))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let warn = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0x92400E))
        .set_background_color(Color::RGB(0xFEF3C7))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let fail = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0x991B1B))
        .set_background_color(Color::RGB(0xFEE2E2))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let kpi_label = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0x475569))
        .set_background_color(Color::RGB(0xF8FAFC))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let kpi_text = Format::new()
        .set_bold()
        .set_font_size(12)
        .set_font_color(Color::RGB(0x0F172A))
        .set_background_color(Color::RGB(0xFFFFFF))
        .set_border(FormatBorder::Thin)
        .set_text_wrap()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);
    let kpi_number = kpi_text.clone().set_num_format("0.0000");
    let kpi_percent = kpi_text.clone().set_num_format("0.0%");
    ReportFormats {
        title,
        subtitle,
        section,
        header,
        label,
        text,
        wrap,
        mono,
        integer,
        decimal,
        pnl,
        percent,
        datetime,
        pass,
        warn,
        fail,
        kpi_label,
        kpi_text,
        kpi_number,
        kpi_percent,
    }
}

fn apply_sheet_layout(sheet: &mut Worksheet, title: &str, zoom: u16) {
    sheet
        .set_screen_gridlines(false)
        .set_print_gridlines(false)
        .set_landscape()
        .set_print_fit_to_pages(1, 0)
        .set_print_center_horizontally(true)
        .set_zoom(zoom)
        .set_header(format!("&C{title}"))
        .set_footer("&LVectorGrid&CPage &P of &N&R&D");
}

fn table_columns(headers: &[&str]) -> Vec<TableColumn> {
    headers
        .iter()
        .map(|header| TableColumn::new().set_header(*header))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn add_native_table(
    sheet: &mut Worksheet,
    first_row: u32,
    first_col: u16,
    last_row: u32,
    last_col: u16,
    name: &str,
    headers: &[&str],
    style: TableStyle,
) -> Result<(), String> {
    if last_row <= first_row {
        return Ok(());
    }
    let columns = table_columns(headers);
    let table = Table::new()
        .set_name(name)
        .set_style(style)
        .set_columns(&columns);
    sheet
        .add_table(first_row, first_col, last_row, last_col, &table)
        .map_err(|error: XlsxError| error.to_string())?;
    Ok(())
}

fn write_usize(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: usize,
    format: &Format,
) -> Result<(), String> {
    let value = u64::try_from(value)
        .map_err(|_| "VCET temporal XLSX integer cannot be represented as u64".to_string())?;
    write_u64(sheet, row, col, value, format)
}

fn write_u64(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: u64,
    format: &Format,
) -> Result<(), String> {
    if value > XLSX_EXACT_INTEGER_LIMIT {
        return Err(format!(
            "VCET temporal XLSX integer {value} exceeds Excel's exact integer limit"
        ));
    }
    sheet
        .write_number_with_format(row, col, value as f64, format)
        .map_err(|error: XlsxError| error.to_string())?;
    Ok(())
}

fn write_optional_number(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: Option<f64>,
    format: &Format,
) -> Result<(), String> {
    match value.filter(|value| value.is_finite()) {
        Some(value) => sheet
            .write_number_with_format(row, col, value, format)
            .map_err(|error: XlsxError| error.to_string())?,
        None => sheet
            .write_string_with_format(row, col, "-", format)
            .map_err(|error: XlsxError| error.to_string())?,
    };
    Ok(())
}

fn write_optional_usize(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: Option<usize>,
    format: &Format,
) -> Result<(), String> {
    match value {
        Some(value) => write_usize(sheet, row, col, value, format),
        None => {
            sheet
                .write_string_with_format(row, col, "-", format)
                .map_err(|error: XlsxError| error.to_string())?;
            Ok(())
        }
    }
}

fn write_optional_u64(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: Option<u64>,
    format: &Format,
) -> Result<(), String> {
    match value {
        Some(value) => write_u64(sheet, row, col, value, format),
        None => {
            sheet
                .write_string_with_format(row, col, "-", format)
                .map_err(|error: XlsxError| error.to_string())?;
            Ok(())
        }
    }
}

fn unix_ms_to_excel_serial(timestamp_ms: i64) -> Result<f64, String> {
    if timestamp_ms < 0 {
        return Err(format!(
            "VCET temporal XLSX cannot encode pre-1970 timestamp {timestamp_ms}"
        ));
    }
    Ok(timestamp_ms as f64 / 86_400_000.0 + 25_569.0)
}

fn write_optional_datetime(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    timestamp_ms: Option<i64>,
    format: &Format,
) -> Result<(), String> {
    match timestamp_ms {
        Some(value) => sheet
            .write_number_with_format(row, col, unix_ms_to_excel_serial(value)?, format)
            .map_err(|error: XlsxError| error.to_string())?,
        None => sheet
            .write_string_with_format(row, col, "-", format)
            .map_err(|error: XlsxError| error.to_string())?,
    };
    Ok(())
}

fn write_merged_kpi_text(
    sheet: &mut Worksheet,
    label_row: u32,
    first_col: u16,
    last_col: u16,
    label: &str,
    value: &str,
    formats: &ReportFormats,
) -> Result<(), String> {
    sheet
        .merge_range(
            label_row,
            first_col,
            label_row,
            last_col,
            label,
            &formats.kpi_label,
        )
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .merge_range(
            label_row + 1,
            first_col,
            label_row + 1,
            last_col,
            &excel_safe_text(value),
            &formats.kpi_text,
        )
        .map_err(|error: XlsxError| error.to_string())?;
    Ok(())
}

fn write_merged_kpi_number(
    sheet: &mut Worksheet,
    label_row: u32,
    first_col: u16,
    last_col: u16,
    label: &str,
    value: Option<f64>,
    percent: bool,
    formats: &ReportFormats,
) -> Result<(), String> {
    sheet
        .merge_range(
            label_row,
            first_col,
            label_row,
            last_col,
            label,
            &formats.kpi_label,
        )
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .merge_range(
            label_row + 1,
            first_col,
            label_row + 1,
            last_col,
            "",
            if percent {
                &formats.kpi_percent
            } else {
                &formats.kpi_number
            },
        )
        .map_err(|error: XlsxError| error.to_string())?;
    write_optional_number(
        sheet,
        label_row + 1,
        first_col,
        value,
        if percent {
            &formats.kpi_percent
        } else {
            &formats.kpi_number
        },
    )?;
    Ok(())
}

fn write_merged_text_value(
    sheet: &mut Worksheet,
    row: u32,
    first_col: u16,
    last_col: u16,
    value: &str,
    format: &Format,
) -> Result<(), String> {
    sheet
        .merge_range(
            row,
            first_col,
            row,
            last_col,
            &excel_safe_text(value),
            format,
        )
        .map_err(|error: XlsxError| error.to_string())?;
    Ok(())
}

fn write_merged_optional_number_value(
    sheet: &mut Worksheet,
    row: u32,
    first_col: u16,
    last_col: u16,
    value: Option<f64>,
    missing: &str,
    number_format: &Format,
    text_format: &Format,
) -> Result<(), String> {
    sheet
        .merge_range(row, first_col, row, last_col, "", number_format)
        .map_err(|error: XlsxError| error.to_string())?;
    match value.filter(|value| value.is_finite()) {
        Some(value) => sheet
            .write_number_with_format(row, first_col, value, number_format)
            .map_err(|error: XlsxError| error.to_string())?,
        None => sheet
            .write_string_with_format(row, first_col, missing, text_format)
            .map_err(|error: XlsxError| error.to_string())?,
    };
    Ok(())
}

fn write_merged_optional_usize_value(
    sheet: &mut Worksheet,
    row: u32,
    first_col: u16,
    last_col: u16,
    value: Option<usize>,
    missing: &str,
    number_format: &Format,
    text_format: &Format,
) -> Result<(), String> {
    sheet
        .merge_range(row, first_col, row, last_col, "", number_format)
        .map_err(|error: XlsxError| error.to_string())?;
    match value {
        Some(value) => write_usize(sheet, row, first_col, value, number_format)?,
        None => {
            sheet
                .write_string_with_format(row, first_col, missing, text_format)
                .map_err(|error: XlsxError| error.to_string())?;
        }
    }
    Ok(())
}

fn apply_zero_centered_formatting(
    sheet: &mut Worksheet,
    first_row: u32,
    first_col: u16,
    last_row: u32,
    last_col: u16,
) -> Result<(), String> {
    let positive = ConditionalFormatCell::new()
        .set_rule(ConditionalFormatCellRule::GreaterThan(0))
        .set_format(
            Format::new()
                .set_font_color("166534")
                .set_background_color("DCFCE7"),
        );
    let neutral = ConditionalFormatCell::new()
        .set_rule(ConditionalFormatCellRule::EqualTo(0))
        .set_format(
            Format::new()
                .set_font_color("92400E")
                .set_background_color("FEF3C7"),
        );
    let negative = ConditionalFormatCell::new()
        .set_rule(ConditionalFormatCellRule::LessThan(0))
        .set_format(
            Format::new()
                .set_font_color("991B1B")
                .set_background_color("FEE2E2"),
        );
    for rule in [&positive, &neutral, &negative] {
        sheet
            .add_conditional_format(first_row, first_col, last_row, last_col, rule)
            .map_err(|error: XlsxError| error.to_string())?;
    }
    Ok(())
}

fn add_title(
    sheet: &mut Worksheet,
    title: &str,
    subtitle: &str,
    last_col: u16,
    formats: &ReportFormats,
) -> Result<(), String> {
    sheet
        .merge_range(0, 0, 0, last_col, title, &formats.title)
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .merge_range(
            1,
            0,
            1,
            last_col,
            &excel_safe_text(subtitle),
            &formats.subtitle,
        )
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .set_row_height(0, 30.0)
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .set_row_height(1, 34.0)
        .map_err(|error: XlsxError| error.to_string())?;
    Ok(())
}

fn write_summary_sheet(
    workbook: &mut Workbook,
    model: &TemporalReportModel,
    formats: &ReportFormats,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name(SUMMARY_SHEET)
        .map_err(|error: XlsxError| error.to_string())?;
    apply_sheet_layout(sheet, "VCET Temporal Evidence Summary", 95);
    add_title(
        sheet,
        "VCET Temporal Validation Evidence",
        "Sealed candidate selection, cross-dataset admission, winner lock, and optional winner-only final-test evidence",
        7,
        formats,
    )?;

    let final_status = match (model.final_complete, model.final_support_passed) {
        (true, Some(true)) => "Complete — support passed",
        (true, Some(false)) => "Complete — support failed",
        _ => "Pending — no final reduction supplied",
    };
    write_merged_kpi_text(
        sheet,
        3,
        0,
        1,
        "Locked winner",
        &model.winner_candidate_id,
        formats,
    )?;
    write_merged_kpi_number(
        sheet,
        3,
        2,
        3,
        "Selection score",
        Some(model.winner_score),
        false,
        formats,
    )?;
    write_merged_kpi_number(
        sheet,
        3,
        4,
        5,
        "Profitable dataset share",
        Some(model.winner_profitable_share),
        true,
        formats,
    )?;
    write_merged_kpi_text(sheet, 3, 6, 7, "Final-test status", final_status, formats)?;
    sheet
        .set_row_height(4, 44.0)
        .map_err(|error: XlsxError| error.to_string())?;

    sheet
        .merge_range(7, 0, 7, 7, "Protocol snapshot", &formats.section)
        .map_err(|error: XlsxError| error.to_string())?;
    let protocol_numbers = [
        ("Candidates", Some(model.candidate_count)),
        ("Selection datasets", Some(model.selection_dataset_count)),
        ("Final-test datasets", Some(model.final_dataset_count)),
        ("Folds per dataset", Some(model.fold_count_per_dataset)),
        (
            "Pre-final candidate-window trials",
            Some(model.pre_final_trial_count),
        ),
        (
            "Final-test candidate-window trials",
            model.final_trial_count,
        ),
    ];
    for (index, (label, value)) in protocol_numbers.into_iter().enumerate() {
        let row = 8 + (index / 2) as u32;
        let col = ((index % 2) * 4) as u16;
        sheet
            .merge_range(row, col, row, col + 1, label, &formats.label)
            .map_err(|error: XlsxError| error.to_string())?;
        write_merged_optional_usize_value(
            sheet,
            row,
            col + 2,
            col + 3,
            value,
            "Pending",
            &formats.integer,
            &formats.text,
        )?;
    }
    for (index, (label, value)) in [
        ("Requested backend", model.requested_backend.as_str()),
        ("MA source", model.ma_source.as_str()),
    ]
    .into_iter()
    .enumerate()
    {
        let row = 11;
        let col = (index * 4) as u16;
        sheet
            .merge_range(row, col, row, col + 1, label, &formats.label)
            .map_err(|error: XlsxError| error.to_string())?;
        write_merged_text_value(sheet, row, col + 2, col + 3, value, &formats.text)?;
    }

    sheet
        .merge_range(
            13,
            0,
            13,
            7,
            "Winner and final-test diagnostics",
            &formats.section,
        )
        .map_err(|error: XlsxError| error.to_string())?;
    let diagnostic_labels = [
        "Validation PnL p10",
        "Validation Sharpe p10",
        "Runner-up",
        "Winner margin",
        "Final mean PnL",
        "Final mean Sharpe",
        "Final support share",
        "Candidate-ID tie-break",
    ];
    for (index, label) in diagnostic_labels.into_iter().enumerate() {
        let row = 14 + (index / 2) as u32;
        let col = ((index % 2) * 4) as u16;
        sheet
            .merge_range(row, col, row, col + 1, label, &formats.label)
            .map_err(|error: XlsxError| error.to_string())?;
    }
    write_merged_optional_number_value(
        sheet,
        14,
        2,
        3,
        model.winner_validation_pnl_p10,
        "-",
        &formats.pnl,
        &formats.text,
    )?;
    write_merged_optional_number_value(
        sheet,
        14,
        6,
        7,
        model.winner_validation_sharpe_p10,
        "-",
        &formats.decimal,
        &formats.text,
    )?;
    write_merged_text_value(
        sheet,
        15,
        2,
        3,
        model
            .runner_up_candidate_id
            .as_deref()
            .unwrap_or("No admitted runner-up"),
        &formats.text,
    )?;
    write_merged_optional_number_value(
        sheet,
        15,
        6,
        7,
        model.winner_margin,
        "-",
        &formats.decimal,
        &formats.text,
    )?;
    write_merged_optional_number_value(
        sheet,
        16,
        2,
        3,
        model.final_mean_pnl,
        "Pending",
        &formats.pnl,
        &formats.text,
    )?;
    write_merged_optional_number_value(
        sheet,
        16,
        6,
        7,
        model.final_mean_sharpe,
        "Pending",
        &formats.decimal,
        &formats.text,
    )?;
    write_merged_optional_number_value(
        sheet,
        17,
        2,
        3,
        model.final_support_share,
        "Pending",
        &formats.percent,
        &formats.text,
    )?;
    write_merged_text_value(
        sheet,
        17,
        6,
        7,
        if model.candidate_id_tie_break_used {
            "Used"
        } else {
            "Not used"
        },
        &formats.text,
    )?;

    sheet
        .merge_range(19, 0, 19, 7, "Interpretation guardrails", &formats.section)
        .map_err(|error: XlsxError| error.to_string())?;
    let note = if model.final_complete {
        "The winner was locked from selection/validation evidence before the final-test stage. Final rows report that one locked candidate only; the final cohort never reranks the winner."
    } else {
        "The winner is locked from selection/validation evidence. Final-test evidence is pending; this report does not infer or simulate a final outcome."
    };
    sheet
        .merge_range(20, 0, 21, 7, note, &formats.wrap)
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .merge_range(
            23,
            0,
            23,
            7,
            "Research evidence is not deployment authorization or a guarantee of future profitability. Review Candidate Selection, Dataset Coverage, Final Test, and Provenance before drawing conclusions.",
            &formats.warn,
        )
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .set_row_height(23, 34.0)
        .map_err(|error: XlsxError| error.to_string())?;

    for column in 0..8u16 {
        sheet
            .set_column_width(column, if column % 2 == 0 { 20.0 } else { 24.0 })
            .map_err(|error: XlsxError| error.to_string())?;
    }
    sheet
        .set_freeze_panes(2, 0)
        .map_err(|error: XlsxError| error.to_string())?;
    Ok(())
}

fn write_candidate_sheet(
    workbook: &mut Workbook,
    model: &TemporalReportModel,
    formats: &ReportFormats,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name(CANDIDATE_SHEET)
        .map_err(|error: XlsxError| error.to_string())?;
    apply_sheet_layout(sheet, "VCET Candidate Selection", 80);
    add_title(
        sheet,
        "Candidate Selection",
        "All sealed candidates are retained. Admission is evaluated against requested-dataset denominators; the winner is locked before final-test access.",
        27,
        formats,
    )?;

    for (index, label) in [
        "Winner candidate",
        "Winner ordinal",
        "Winner score",
        "Runner-up score",
        "Winner margin",
        "Tie-break used",
    ]
    .into_iter()
    .enumerate()
    {
        let row = 3 + index as u32;
        sheet
            .write_string_with_format(row, 0, label, &formats.label)
            .map_err(|error: XlsxError| error.to_string())?;
    }
    write_merged_text_value(sheet, 3, 1, 5, &model.winner_candidate_id, &formats.text)?;
    write_merged_optional_usize_value(
        sheet,
        4,
        1,
        5,
        Some(model.winner_ordinal),
        "-",
        &formats.integer,
        &formats.text,
    )?;
    write_merged_optional_number_value(
        sheet,
        5,
        1,
        5,
        Some(model.winner_score),
        "-",
        &formats.decimal,
        &formats.text,
    )?;
    write_merged_optional_number_value(
        sheet,
        6,
        1,
        5,
        model.runner_up_score,
        "-",
        &formats.decimal,
        &formats.text,
    )?;
    write_merged_optional_number_value(
        sheet,
        7,
        1,
        5,
        model.winner_margin,
        "-",
        &formats.decimal,
        &formats.text,
    )?;
    write_merged_text_value(
        sheet,
        8,
        1,
        5,
        if model.candidate_id_tie_break_used {
            "Yes"
        } else {
            "No"
        },
        &formats.text,
    )?;
    sheet
        .write_string_with_format(10, 0, "Locked winner params", &formats.label)
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .merge_range(
            10,
            1,
            12,
            12,
            &excel_safe_text(&model.winner_params_json),
            &formats.mono,
        )
        .map_err(|error: XlsxError| error.to_string())?;

    let headers = [
        "Ordinal",
        "Candidate ID",
        "Selected",
        "Admitted",
        "Requested coverage share",
        "Evaluated share",
        "Profitable share",
        "Hard-support share",
        "Metric-support share",
        "Validation PnL p10",
        "Validation Sharpe p10",
        "Mean validation Sharpe",
        "Score gap to winner",
        "Validation observations",
        "Train trades",
        "Validation trades",
        "Support failures",
        "Non-finite metrics",
        "VIDYA short",
        "VIDYA long",
        "VIDYA alpha",
        "CE period",
        "CE multiplier",
        "CE use close",
        "Require VIDYA slope",
        "Max trade bars",
        "Candidate params JSON",
        "Evidence state",
    ];
    let header_row = 19u32;
    let first_data_row = header_row + 1;
    for (column, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(header_row, column as u16, *header, &formats.header)
            .map_err(|error: XlsxError| error.to_string())?;
    }
    sheet
        .set_row_height(header_row, 42.0)
        .map_err(|error: XlsxError| error.to_string())?;

    let selected_index = model
        .candidate_rows
        .iter()
        .position(|row| row.selected)
        .ok_or_else(|| "VCET temporal XLSX model has no selected candidate".to_string())?;
    for (index, candidate) in model.candidate_rows.iter().enumerate() {
        let row = first_data_row + index as u32;
        write_usize(sheet, row, 0, candidate.ordinal, &formats.integer)?;
        sheet
            .write_string_with_format(
                row,
                1,
                &excel_safe_text(&candidate.candidate_id),
                &formats.mono,
            )
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_string_with_format(
                row,
                2,
                if candidate.selected { "Yes" } else { "No" },
                if candidate.selected {
                    &formats.pass
                } else {
                    &formats.text
                },
            )
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_string_with_format(
                row,
                3,
                if candidate.admitted { "Yes" } else { "No" },
                if candidate.admitted {
                    &formats.pass
                } else {
                    &formats.fail
                },
            )
            .map_err(|error: XlsxError| error.to_string())?;
        for (column, value) in [
            (4u16, candidate.coverage_share),
            (5, candidate.evaluated_share),
            (6, candidate.profitable_share),
            (7, candidate.hard_support_share),
            (8, candidate.metric_support_share),
        ] {
            sheet
                .write_number_with_format(row, column, value, &formats.percent)
                .map_err(|error: XlsxError| error.to_string())?;
        }
        write_optional_number(sheet, row, 9, candidate.validation_pnl_p10, &formats.pnl)?;
        write_optional_number(
            sheet,
            row,
            10,
            candidate.validation_sharpe_p10,
            &formats.decimal,
        )?;
        write_optional_number(
            sheet,
            row,
            11,
            candidate.mean_validation_sharpe,
            &formats.decimal,
        )?;
        write_optional_number(
            sheet,
            row,
            12,
            candidate.score_gap_to_winner,
            &formats.decimal,
        )?;
        write_usize(
            sheet,
            row,
            13,
            candidate.validation_observations,
            &formats.integer,
        )?;
        write_u64(
            sheet,
            row,
            14,
            candidate.total_train_trades,
            &formats.integer,
        )?;
        write_u64(
            sheet,
            row,
            15,
            candidate.total_validation_trades,
            &formats.integer,
        )?;
        write_usize(sheet, row, 16, candidate.support_failures, &formats.integer)?;
        write_usize(
            sheet,
            row,
            17,
            candidate.non_finite_metrics,
            &formats.integer,
        )?;
        write_optional_usize(
            sheet,
            row,
            18,
            candidate.vidya_short_period.map(usize::from),
            &formats.integer,
        )?;
        write_optional_usize(
            sheet,
            row,
            19,
            candidate.vidya_long_period.map(usize::from),
            &formats.integer,
        )?;
        write_optional_number(sheet, row, 20, candidate.vidya_alpha, &formats.decimal)?;
        write_optional_usize(
            sheet,
            row,
            21,
            candidate.ce_period.map(usize::from),
            &formats.integer,
        )?;
        write_optional_number(sheet, row, 22, candidate.ce_mult, &formats.decimal)?;
        for (column, value) in [
            (23u16, candidate.ce_use_close),
            (24u16, candidate.require_vidya_slope),
        ] {
            sheet
                .write_string_with_format(
                    row,
                    column,
                    match value {
                        Some(true) => "Yes",
                        Some(false) => "No",
                        None => "-",
                    },
                    &formats.text,
                )
                .map_err(|error: XlsxError| error.to_string())?;
        }
        write_optional_usize(
            sheet,
            row,
            25,
            candidate.max_trade_bars.map(usize::from),
            &formats.integer,
        )?;
        sheet
            .write_string_with_format(
                row,
                26,
                &excel_safe_text(&candidate.params_json),
                &formats.mono,
            )
            .map_err(|error: XlsxError| error.to_string())?;
        let evidence_state = if candidate.selected {
            "Locked winner"
        } else if candidate.admitted {
            "Admitted"
        } else {
            "Not admitted"
        };
        sheet
            .write_string_with_format(row, 27, evidence_state, &formats.text)
            .map_err(|error: XlsxError| error.to_string())?;
    }

    let last_data_row = first_data_row + model.candidate_rows.len() as u32 - 1;
    add_native_table(
        sheet,
        header_row,
        0,
        last_data_row,
        27,
        "VcetTemporalCandidatesTable",
        &headers,
        TableStyle::Medium2,
    )?;
    let share_scale = ConditionalFormat3ColorScale::new()
        .set_minimum(ConditionalFormatType::Number, 0.0)
        .set_midpoint(ConditionalFormatType::Number, 0.5)
        .set_maximum(ConditionalFormatType::Number, 1.0)
        .set_minimum_color("F8696B")
        .set_midpoint_color("FFEB84")
        .set_maximum_color("63BE7B");
    sheet
        .add_conditional_format(first_data_row, 4, last_data_row, 8, &share_scale)
        .map_err(|error: XlsxError| error.to_string())?;
    apply_zero_centered_formatting(sheet, first_data_row, 9, last_data_row, 12)?;
    let zero_good = ConditionalFormatCell::new()
        .set_rule(ConditionalFormatCellRule::EqualTo(0))
        .set_format(
            Format::new()
                .set_font_color("166534")
                .set_background_color("DCFCE7"),
        );
    let positive_bad = ConditionalFormatCell::new()
        .set_rule(ConditionalFormatCellRule::GreaterThan(0))
        .set_format(
            Format::new()
                .set_font_color("991B1B")
                .set_background_color("FEE2E2"),
        );
    for rule in [&zero_good, &positive_bad] {
        sheet
            .add_conditional_format(first_data_row, 16, last_data_row, 17, rule)
            .map_err(|error: XlsxError| error.to_string())?;
    }

    let mut population_marker = ChartMarker::new();
    population_marker
        .set_type(ChartMarkerType::Circle)
        .set_size(4)
        .set_format(ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("#0F766E")));
    let mut winner_marker = ChartMarker::new();
    winner_marker
        .set_type(ChartMarkerType::Diamond)
        .set_size(9)
        .set_format(ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("#F97316")));
    let mut chart = Chart::new(ChartType::Scatter);
    chart
        .add_series()
        .set_name("All verified candidates")
        .set_categories((CANDIDATE_SHEET, first_data_row, 6, last_data_row, 6))
        .set_values((CANDIDATE_SHEET, first_data_row, 11, last_data_row, 11))
        .set_marker(&population_marker);
    let selected_row = first_data_row + selected_index as u32;
    chart
        .add_series()
        .set_name("Locked winner")
        .set_categories((CANDIDATE_SHEET, selected_row, 6, selected_row, 6))
        .set_values((CANDIDATE_SHEET, selected_row, 11, selected_row, 11))
        .set_marker(&winner_marker);
    chart
        .title()
        .set_name("Profitability breadth vs mean validation Sharpe");
    chart
        .x_axis()
        .set_name("Profitable share of requested datasets")
        .set_num_format("0%")
        .set_min(0.0)
        .set_max(1.0);
    chart
        .y_axis()
        .set_name("Mean validation Sharpe")
        .set_num_format("0.00");
    chart
        .set_width(720)
        .set_height(300)
        .set_alt_text("Scatter chart of every verified candidate's profitable dataset share against mean validation Sharpe, with the locked winner highlighted.");
    sheet
        .insert_chart_with_offset(2, 14, &chart, 8, 8)
        .map_err(|error: XlsxError| error.to_string())?;

    let widths = [
        10.0, 46.0, 11.0, 11.0, 17.0, 15.0, 15.0, 16.0, 17.0, 16.0, 17.0, 18.0, 17.0, 17.0, 14.0,
        16.0, 15.0, 16.0, 12.0, 12.0, 13.0, 12.0, 13.0, 13.0, 17.0, 15.0, 64.0, 17.0,
    ];
    for (column, width) in widths.into_iter().enumerate() {
        sheet
            .set_column_width(column as u16, width)
            .map_err(|error: XlsxError| error.to_string())?;
    }
    sheet
        .set_freeze_panes(first_data_row, 2)
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .set_repeat_rows(header_row, header_row)
        .map_err(|error: XlsxError| error.to_string())?;
    Ok(())
}

fn write_coverage_sheet(
    workbook: &mut Workbook,
    model: &TemporalReportModel,
    formats: &ReportFormats,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name(COVERAGE_SHEET)
        .map_err(|error: XlsxError| error.to_string())?;
    apply_sheet_layout(sheet, "VCET Dataset Coverage", 80);
    add_title(
        sheet,
        "Dataset Coverage",
        "Chronological coverage, ordering, cadence, and gap evidence for every sealed selection and final-test dataset.",
        23,
        formats,
    )?;
    sheet
        .write_string_with_format(3, 0, "Selection datasets", &formats.label)
        .map_err(|error: XlsxError| error.to_string())?;
    write_usize(sheet, 3, 1, model.selection_dataset_count, &formats.integer)?;
    sheet
        .write_string_with_format(3, 3, "Final-test datasets", &formats.label)
        .map_err(|error: XlsxError| error.to_string())?;
    write_usize(sheet, 3, 4, model.final_dataset_count, &formats.integer)?;
    sheet
        .write_string_with_format(3, 6, "Common folds", &formats.label)
        .map_err(|error: XlsxError| error.to_string())?;
    write_usize(sheet, 3, 7, model.fold_count_per_dataset, &formats.integer)?;
    sheet
        .merge_range(
            4,
            0,
            4,
            23,
            "Gap counts retain the evidence artifact's declared basis and caveat. Market closures may be expected unless a verified session mask was used upstream.",
            &formats.warn,
        )
        .map_err(|error: XlsxError| error.to_string())?;

    let headers = [
        "Cohort role",
        "Dataset ID",
        "Timestamp count",
        "Requested start (UTC)",
        "Requested end exclusive (UTC)",
        "Observed first (UTC)",
        "Observed last (UTC)",
        "Observed span (days)",
        "Within requested",
        "Before requested",
        "At/after requested end",
        "Strictly increasing",
        "Duplicate intervals",
        "Descending intervals",
        "Gap status",
        "Gap basis",
        "Cadence used (ms)",
        "Analyzed intervals",
        "Irregular intervals",
        "Gap intervals",
        "Missing intervals",
        "Longest gap (minutes)",
        "Fold count",
        "Coverage SHA-256",
    ];
    let header_row = 6u32;
    let first_data_row = header_row + 1;
    for (column, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(header_row, column as u16, *header, &formats.header)
            .map_err(|error: XlsxError| error.to_string())?;
    }
    sheet
        .set_row_height(header_row, 42.0)
        .map_err(|error: XlsxError| error.to_string())?;
    for (index, coverage) in model.coverage_rows.iter().enumerate() {
        let row = first_data_row + index as u32;
        sheet
            .write_string_with_format(row, 0, &coverage.role, &formats.text)
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_string_with_format(
                row,
                1,
                &excel_safe_text(&coverage.dataset_id),
                &formats.text,
            )
            .map_err(|error: XlsxError| error.to_string())?;
        write_usize(sheet, row, 2, coverage.timestamp_count, &formats.integer)?;
        for (column, value) in [
            (3u16, coverage.requested_start_ms),
            (4, coverage.requested_end_exclusive_ms),
            (5, coverage.observed_min_ms),
            (6, coverage.observed_max_ms),
        ] {
            write_optional_datetime(sheet, row, column, value, &formats.datetime)?;
        }
        write_optional_number(sheet, row, 7, coverage.observed_span_days, &formats.decimal)?;
        write_optional_usize(sheet, row, 8, coverage.within_requested, &formats.integer)?;
        write_optional_usize(sheet, row, 9, coverage.before_requested, &formats.integer)?;
        write_optional_usize(sheet, row, 10, coverage.after_requested, &formats.integer)?;
        sheet
            .write_string_with_format(
                row,
                11,
                if coverage.strictly_increasing {
                    "Yes"
                } else {
                    "No"
                },
                if coverage.strictly_increasing {
                    &formats.pass
                } else {
                    &formats.fail
                },
            )
            .map_err(|error: XlsxError| error.to_string())?;
        write_usize(
            sheet,
            row,
            12,
            coverage.duplicate_intervals,
            &formats.integer,
        )?;
        write_usize(
            sheet,
            row,
            13,
            coverage.descending_intervals,
            &formats.integer,
        )?;
        sheet
            .write_string_with_format(row, 14, &coverage.gap_status, &formats.text)
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_string_with_format(row, 15, &coverage.gap_basis, &formats.text)
            .map_err(|error: XlsxError| error.to_string())?;
        write_optional_number(
            sheet,
            row,
            16,
            coverage.cadence_used_ms.map(|value| value as f64),
            &formats.integer,
        )?;
        write_usize(
            sheet,
            row,
            17,
            coverage.analyzed_intervals,
            &formats.integer,
        )?;
        write_optional_usize(
            sheet,
            row,
            18,
            coverage.irregular_intervals,
            &formats.integer,
        )?;
        write_optional_usize(sheet, row, 19, coverage.gap_intervals, &formats.integer)?;
        write_optional_u64(sheet, row, 20, coverage.missing_intervals, &formats.integer)?;
        write_optional_number(
            sheet,
            row,
            21,
            coverage.longest_gap_minutes,
            &formats.decimal,
        )?;
        write_usize(sheet, row, 22, coverage.fold_count, &formats.integer)?;
        sheet
            .write_string_with_format(
                row,
                23,
                &excel_safe_text(&coverage.coverage_sha256),
                &formats.mono,
            )
            .map_err(|error: XlsxError| error.to_string())?;
    }
    let last_data_row = first_data_row + model.coverage_rows.len() as u32 - 1;
    add_native_table(
        sheet,
        header_row,
        0,
        last_data_row,
        23,
        "VcetTemporalCoverageTable",
        &headers,
        TableStyle::Medium4,
    )?;
    let count_bar = ConditionalFormatDataBar::new().set_fill_color("5A8AC6");
    sheet
        .add_conditional_format(first_data_row, 8, last_data_row, 10, &count_bar)
        .map_err(|error: XlsxError| error.to_string())?;
    let zero_good = ConditionalFormatCell::new()
        .set_rule(ConditionalFormatCellRule::EqualTo(0))
        .set_format(
            Format::new()
                .set_font_color("166534")
                .set_background_color("DCFCE7"),
        );
    let positive_bad = ConditionalFormatCell::new()
        .set_rule(ConditionalFormatCellRule::GreaterThan(0))
        .set_format(
            Format::new()
                .set_font_color("991B1B")
                .set_background_color("FEE2E2"),
        );
    for rule in [&zero_good, &positive_bad] {
        for column in [12u16, 13, 19, 20] {
            sheet
                .add_conditional_format(first_data_row, column, last_data_row, column, rule)
                .map_err(|error: XlsxError| error.to_string())?;
        }
    }
    let widths = [
        14.0, 32.0, 16.0, 21.0, 25.0, 21.0, 21.0, 19.0, 17.0, 17.0, 21.0, 19.0, 18.0, 19.0, 18.0,
        31.0, 18.0, 19.0, 19.0, 16.0, 18.0, 20.0, 12.0, 70.0,
    ];
    for (column, width) in widths.into_iter().enumerate() {
        sheet
            .set_column_width(column as u16, width)
            .map_err(|error: XlsxError| error.to_string())?;
    }
    sheet
        .set_freeze_panes(first_data_row, 2)
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .set_repeat_rows(header_row, header_row)
        .map_err(|error: XlsxError| error.to_string())?;
    Ok(())
}

fn write_final_sheet(
    workbook: &mut Workbook,
    model: &TemporalReportModel,
    formats: &ReportFormats,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name(FINAL_SHEET)
        .map_err(|error: XlsxError| error.to_string())?;
    apply_sheet_layout(sheet, "VCET Winner-Only Final Test", 80);
    add_title(
        sheet,
        "Winner-Only Final Test",
        "The final cohort evaluates exactly one pre-locked winner. Final-test results never rerank candidate selection.",
        20,
        formats,
    )?;
    if !model.final_complete {
        sheet
            .merge_range(
                3,
                0,
                5,
                20,
                "Final test is pending. No verified final-test reduction was supplied, so this worksheet intentionally contains no inferred metrics or placeholder outcome.",
                &formats.warn,
            )
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .merge_range(
                7,
                0,
                7,
                20,
                "Required next step: execute the authorized winner-only final shards, reduce the complete final cohort, then regenerate this report with the verified final reduction.",
                &formats.wrap,
            )
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .set_column_width(0, 24.0)
            .map_err(|error: XlsxError| error.to_string())?;
        for column in 1..=20u16 {
            sheet
                .set_column_width(column, 14.0)
                .map_err(|error: XlsxError| error.to_string())?;
        }
        sheet
            .set_freeze_panes(2, 0)
            .map_err(|error: XlsxError| error.to_string())?;
        return Ok(());
    }

    let overall = if model.final_support_passed == Some(true) {
        "Support passed"
    } else {
        "Support failed"
    };
    sheet
        .write_string_with_format(3, 0, "Outcome", &formats.label)
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .merge_range(
            3,
            1,
            3,
            3,
            overall,
            if model.final_support_passed == Some(true) {
                &formats.pass
            } else {
                &formats.fail
            },
        )
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .write_string_with_format(4, 0, "Final datasets", &formats.label)
        .map_err(|error: XlsxError| error.to_string())?;
    write_usize(sheet, 4, 1, model.final_rows.len(), &formats.integer)?;
    sheet
        .write_string_with_format(4, 3, "Support share", &formats.label)
        .map_err(|error: XlsxError| error.to_string())?;
    write_optional_number(sheet, 4, 4, model.final_support_share, &formats.percent)?;
    sheet
        .write_string_with_format(5, 0, "Mean PnL", &formats.label)
        .map_err(|error: XlsxError| error.to_string())?;
    write_optional_number(sheet, 5, 1, model.final_mean_pnl, &formats.pnl)?;
    sheet
        .write_string_with_format(5, 3, "Mean Sharpe", &formats.label)
        .map_err(|error: XlsxError| error.to_string())?;
    write_optional_number(sheet, 5, 4, model.final_mean_sharpe, &formats.decimal)?;
    sheet
        .merge_range(
            7,
            0,
            7,
            20,
            "PnL is the engine return metric (not currency). 'Trades' is the execution metric's transition count used by the minimum-support contract. GPU metric fields unavailable under the sealed contract remain explicitly listed.",
            &formats.warn,
        )
        .map_err(|error: XlsxError| error.to_string())?;

    let headers = [
        "Dataset ID",
        "Support passed",
        "PnL (engine return)",
        "Sharpe",
        "Sortino",
        "Max drawdown",
        "Trades (transitions)",
        "Exposure",
        "Net exposure",
        "Final start (UTC)",
        "Final end (UTC)",
        "Scored bars",
        "Warmup bars",
        "Backend route",
        "Device ID",
        "Metric contract",
        "Unavailable metric fields",
        "Source dataset SHA-256",
        "Loaded snapshot SHA-256",
        "Coverage SHA-256",
        "Attestation SHA-256",
    ];
    let header_row = 10u32;
    let first_data_row = header_row + 1;
    for (column, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(header_row, column as u16, *header, &formats.header)
            .map_err(|error: XlsxError| error.to_string())?;
    }
    sheet
        .set_row_height(header_row, 42.0)
        .map_err(|error: XlsxError| error.to_string())?;
    for (index, final_row) in model.final_rows.iter().enumerate() {
        let row = first_data_row + index as u32;
        sheet
            .write_string_with_format(
                row,
                0,
                &excel_safe_text(&final_row.dataset_id),
                &formats.text,
            )
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_string_with_format(
                row,
                1,
                if final_row.support_passed {
                    "Yes"
                } else {
                    "No"
                },
                if final_row.support_passed {
                    &formats.pass
                } else {
                    &formats.fail
                },
            )
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_number_with_format(row, 2, final_row.pnl, &formats.pnl)
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_number_with_format(row, 3, final_row.sharpe, &formats.decimal)
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_number_with_format(row, 4, final_row.sortino, &formats.decimal)
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_number_with_format(row, 5, final_row.max_drawdown, &formats.percent)
            .map_err(|error: XlsxError| error.to_string())?;
        write_u64(sheet, row, 6, final_row.trades, &formats.integer)?;
        sheet
            .write_number_with_format(row, 7, final_row.exposure, &formats.percent)
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_number_with_format(row, 8, final_row.net_exposure, &formats.percent)
            .map_err(|error: XlsxError| error.to_string())?;
        write_optional_datetime(
            sheet,
            row,
            9,
            Some(final_row.final_start_utc_ms),
            &formats.datetime,
        )?;
        write_optional_datetime(
            sheet,
            row,
            10,
            Some(final_row.final_end_utc_ms),
            &formats.datetime,
        )?;
        write_usize(sheet, row, 11, final_row.scored_bars, &formats.integer)?;
        write_usize(sheet, row, 12, final_row.warmup_bars, &formats.integer)?;
        sheet
            .write_string_with_format(
                row,
                13,
                &excel_safe_text(&final_row.backend_route),
                &formats.text,
            )
            .map_err(|error: XlsxError| error.to_string())?;
        write_optional_usize(
            sheet,
            row,
            14,
            final_row.device_id.map(|value| value as usize),
            &formats.integer,
        )?;
        sheet
            .write_string_with_format(
                row,
                15,
                &excel_safe_text(&final_row.metric_contract),
                &formats.wrap,
            )
            .map_err(|error: XlsxError| error.to_string())?;
        let unavailable_metric_fields = if final_row.unavailable_metric_fields.is_empty() {
            "None".to_string()
        } else {
            excel_safe_text(&final_row.unavailable_metric_fields)
        };
        sheet
            .write_string_with_format(row, 16, &unavailable_metric_fields, &formats.wrap)
            .map_err(|error: XlsxError| error.to_string())?;
        for (column, value) in [
            (17u16, &final_row.source_dataset_sha256),
            (18, &final_row.loaded_snapshot_sha256),
            (19, &final_row.coverage_sha256),
            (20, &final_row.attestation_sha256),
        ] {
            sheet
                .write_string_with_format(row, column, &excel_safe_text(value), &formats.mono)
                .map_err(|error: XlsxError| error.to_string())?;
        }
        if !final_row.unavailable_metric_fields.is_empty() {
            sheet
                .set_row_height(row, 30.0)
                .map_err(|error: XlsxError| error.to_string())?;
        }
    }
    let last_data_row = first_data_row + model.final_rows.len() as u32 - 1;
    add_native_table(
        sheet,
        header_row,
        0,
        last_data_row,
        20,
        "VcetTemporalFinalTestTable",
        &headers,
        TableStyle::Medium4,
    )?;
    apply_zero_centered_formatting(sheet, first_data_row, 2, last_data_row, 4)?;
    let max_dd_scale = ConditionalFormat3ColorScale::new()
        .set_minimum_color("63BE7B")
        .set_midpoint_color("FFEB84")
        .set_maximum_color("F8696B");
    sheet
        .add_conditional_format(first_data_row, 5, last_data_row, 5, &max_dd_scale)
        .map_err(|error: XlsxError| error.to_string())?;
    let widths = [
        32.0, 16.0, 19.0, 13.0, 13.0, 16.0, 20.0, 13.0, 15.0, 21.0, 21.0, 14.0, 14.0, 30.0, 12.0,
        38.0, 42.0, 70.0, 70.0, 70.0, 70.0,
    ];
    for (column, width) in widths.into_iter().enumerate() {
        sheet
            .set_column_width(column as u16, width)
            .map_err(|error: XlsxError| error.to_string())?;
    }
    sheet
        .set_freeze_panes(first_data_row, 2)
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .set_repeat_rows(header_row, header_row)
        .map_err(|error: XlsxError| error.to_string())?;
    Ok(())
}

fn write_provenance_sheet(
    workbook: &mut Workbook,
    model: &TemporalReportModel,
    formats: &ReportFormats,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name(PROVENANCE_SHEET)
        .map_err(|error: XlsxError| error.to_string())?;
    apply_sheet_layout(sheet, "VCET Temporal Provenance", 85);
    add_title(
        sheet,
        "Provenance and Limitations",
        "Artifact identity, execution contract, admission thresholds, and explicit interpretation boundaries.",
        7,
        formats,
    )?;

    let evidence_headers = ["Stage", "Evidence kind / contract", "SHA-256", "Status"];
    let evidence_header_row = 3u32;
    for (column, header) in evidence_headers.iter().enumerate() {
        sheet
            .write_string_with_format(evidence_header_row, column as u16, *header, &formats.header)
            .map_err(|error: XlsxError| error.to_string())?;
    }
    for (index, evidence) in model.evidence_rows.iter().enumerate() {
        let row = evidence_header_row + 1 + index as u32;
        sheet
            .write_string_with_format(row, 0, &evidence.stage, &formats.text)
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_string_with_format(
                row,
                1,
                &excel_safe_text(&evidence.evidence_kind),
                &formats.wrap,
            )
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_string_with_format(row, 2, &excel_safe_text(&evidence.sha256), &formats.mono)
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .write_string_with_format(
                row,
                3,
                &evidence.status,
                if evidence.status == "Verified" {
                    &formats.pass
                } else {
                    &formats.warn
                },
            )
            .map_err(|error: XlsxError| error.to_string())?;
    }
    let evidence_last_row = evidence_header_row + model.evidence_rows.len() as u32;
    add_native_table(
        sheet,
        evidence_header_row,
        0,
        evidence_last_row,
        3,
        "VcetTemporalEvidenceChainTable",
        &evidence_headers,
        TableStyle::Medium2,
    )?;

    let execution_row = evidence_last_row + 2;
    sheet
        .merge_range(
            execution_row,
            0,
            execution_row,
            7,
            "Execution and source lineage",
            &formats.section,
        )
        .map_err(|error: XlsxError| error.to_string())?;
    let execution_rows = [
        ("Requested backend", model.requested_backend.as_str()),
        ("MA source", model.ma_source.as_str()),
        (
            "Selection reduction contract",
            model.selection_reduction_contract.as_str(),
        ),
        (
            "Source campaign manifest",
            model.source_campaign_manifest_hash.as_str(),
        ),
        (
            "Source candidate manifest",
            model.source_candidate_manifest_hash.as_str(),
        ),
        (
            "Source search ID",
            model.source_search_id.as_deref().unwrap_or("-"),
        ),
        ("Execution spec JSON", model.execution_spec_json.as_str()),
        ("StrategyConfig JSON", model.strategy_json.as_str()),
    ];
    for (index, (label, value)) in execution_rows.into_iter().enumerate() {
        let row = execution_row + 1 + index as u32;
        sheet
            .write_string_with_format(row, 0, label, &formats.label)
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .merge_range(
                row,
                1,
                row,
                7,
                &excel_safe_text(value),
                if label.ends_with("JSON") || label.contains("manifest") {
                    &formats.mono
                } else {
                    &formats.wrap
                },
            )
            .map_err(|error: XlsxError| error.to_string())?;
        if label.ends_with("JSON") {
            sheet
                .set_row_height(row, 46.0)
                .map_err(|error: XlsxError| error.to_string())?;
        }
    }

    let admission_header_row = execution_row + 11;
    let admission_headers = ["Admission metric", "Threshold", "Winner observed", "Passed"];
    for (column, header) in admission_headers.iter().enumerate() {
        sheet
            .write_string_with_format(
                admission_header_row,
                column as u16,
                *header,
                &formats.header,
            )
            .map_err(|error: XlsxError| error.to_string())?;
    }
    for (index, admission) in model.admission_rows.iter().enumerate() {
        let row = admission_header_row + 1 + index as u32;
        sheet
            .write_string_with_format(row, 0, &admission.metric, &formats.text)
            .map_err(|error: XlsxError| error.to_string())?;
        let numeric_format = if admission.format_as_percent {
            &formats.percent
        } else {
            &formats.decimal
        };
        sheet
            .write_number_with_format(row, 1, admission.threshold, numeric_format)
            .map_err(|error: XlsxError| error.to_string())?;
        write_optional_number(sheet, row, 2, admission.winner_observed, numeric_format)?;
        sheet
            .write_string_with_format(
                row,
                3,
                if admission.passed { "Yes" } else { "No" },
                if admission.passed {
                    &formats.pass
                } else {
                    &formats.fail
                },
            )
            .map_err(|error: XlsxError| error.to_string())?;
    }
    let admission_last_row = admission_header_row + model.admission_rows.len() as u32;
    add_native_table(
        sheet,
        admission_header_row,
        0,
        admission_last_row,
        3,
        "VcetTemporalAdmissionTable",
        &admission_headers,
        TableStyle::Medium4,
    )?;

    let limitation_header_row = admission_last_row + 3;
    let limitation_headers = ["Stage", "Explicit limitation / interpretation boundary"];
    sheet
        .write_string_with_format(
            limitation_header_row,
            0,
            limitation_headers[0],
            &formats.header,
        )
        .map_err(|error: XlsxError| error.to_string())?;
    sheet
        .merge_range(
            limitation_header_row,
            1,
            limitation_header_row,
            7,
            limitation_headers[1],
            &formats.header,
        )
        .map_err(|error: XlsxError| error.to_string())?;
    let mut seen = BTreeSet::new();
    let mut written = 0usize;
    for (stage, limitation) in &model.limitations {
        if !seen.insert((stage.clone(), limitation.clone())) {
            continue;
        }
        let row = limitation_header_row + 1 + written as u32;
        sheet
            .write_string_with_format(row, 0, &excel_safe_text(stage), &formats.text)
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .merge_range(row, 1, row, 7, &excel_safe_text(limitation), &formats.wrap)
            .map_err(|error: XlsxError| error.to_string())?;
        sheet
            .set_row_height(row, 34.0)
            .map_err(|error: XlsxError| error.to_string())?;
        written += 1;
    }
    let widths = [20.0, 34.0, 74.0, 14.0, 18.0, 18.0, 18.0, 18.0];
    for (column, width) in widths.into_iter().enumerate() {
        sheet
            .set_column_width(column as u16, width)
            .map_err(|error: XlsxError| error.to_string())?;
    }
    sheet
        .set_freeze_panes(3, 1)
        .map_err(|error: XlsxError| error.to_string())?;
    Ok(())
}

fn build_report_workbook_bytes(model: &TemporalReportModel) -> Result<Vec<u8>, String> {
    validate_model_limits(model)?;
    let mut workbook = Workbook::new();
    let creation_time =
        ExcelDateTime::from_ymd(2000, 1, 1).map_err(|error: XlsxError| error.to_string())?;
    let properties = DocProperties::new()
        .set_title("VectorGrid VCET Temporal Validation Evidence")
        .set_subject("Sealed VCET candidate selection and winner-only temporal validation")
        .set_author("VectorGrid")
        .set_category("Quantitative Research")
        .set_keywords("VCET, temporal validation, candidate selection, final test, provenance")
        .set_comment(
            "Research evidence is not deployment authorization or a guarantee of future returns.",
        )
        .set_creation_datetime(&creation_time);
    workbook.set_properties(&properties);
    let formats = build_formats();
    write_summary_sheet(&mut workbook, model, &formats)?;
    write_candidate_sheet(&mut workbook, model, &formats)?;
    write_coverage_sheet(&mut workbook, model, &formats)?;
    write_final_sheet(&mut workbook, model, &formats)?;
    write_provenance_sheet(&mut workbook, model, &formats)?;
    workbook
        .save_to_buffer()
        .map_err(|error: XlsxError| error.to_string())
}

/// Build a deterministic, bounded XLSX report from a sealed VCET universe and its verified
/// phased reductions. When `final_test` is `None`, the report explicitly marks final evidence as
/// pending rather than inferring an outcome.
pub(super) fn build_vcet_temporal_report_xlsx_bytes(
    sealed_universe: &VcetTemporalSealedDatasetUniverseV1,
    pre_final: &ReduceVcetTemporalPreFinalShardsResponseV1,
    final_test: Option<&ReduceVcetTemporalFinalTestShardsResponseV1>,
) -> Result<Vec<u8>, String> {
    validate_report_inputs(sealed_universe, pre_final, final_test)?;
    let model = build_report_model(sealed_universe, pre_final, final_test)?;
    build_report_workbook_bytes(&model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};
    use ta_app_core::composite_validate::VcetFixedCandidateWindowBackendProvenance;
    use ta_app_core::vcet_dataset_coverage::{
        profile_vcet_dataset_coverage, VcetDatasetCoverageOptions,
    };
    use ta_app_core::vcet_search::VcetCandidateIdentityInputs;
    use ta_app_core::vcet_temporal::{
        VcetTemporalBarRangeV1, VcetTemporalEvaluationSliceV1, VcetTemporalUtcBoundsV1,
    };
    use ta_strategies::double_ma::{Metrics, StrategyConfig};

    fn fixture_model() -> TemporalReportModel {
        let mut model = TemporalReportModel {
            source_campaign_manifest_hash: format!("sha256:{}", "c".repeat(64)),
            source_candidate_manifest_hash: format!("sha256:{}", "d".repeat(64)),
            source_search_id: Some("vcet_search_v1_fixture".to_string()),
            execution_spec_json: "{\"backend\":\"cpu\"}".to_string(),
            strategy_json: "{\"long_only\":false}".to_string(),
            requested_backend: "CPU only".to_string(),
            ma_source: "close".to_string(),
            selection_reduction_contract: "mean_validation_sharpe_total_cmp_v1".to_string(),
            candidate_count: 1,
            selection_dataset_count: 1,
            final_dataset_count: 1,
            fold_count_per_dataset: 2,
            pre_final_trial_count: 4,
            final_trial_count: None,
            winner_candidate_id: "candidate-fixture".to_string(),
            winner_ordinal: 0,
            winner_score: 1.25,
            runner_up_candidate_id: None,
            runner_up_score: None,
            winner_margin: None,
            candidate_id_tie_break_used: false,
            winner_params_json: "{\"vcet\":{\"vidya_short_period\":10}}".to_string(),
            winner_profitable_share: 0.75,
            winner_validation_pnl_p10: Some(0.12),
            winner_validation_sharpe_p10: Some(0.8),
            final_complete: false,
            final_support_passed: None,
            final_mean_pnl: None,
            final_mean_sharpe: None,
            final_support_share: None,
            candidate_rows: vec![CandidateReportRow {
                ordinal: 0,
                candidate_id: "candidate-fixture".to_string(),
                selected: true,
                admitted: true,
                coverage_share: 1.0,
                evaluated_share: 1.0,
                profitable_share: 0.75,
                hard_support_share: 1.0,
                metric_support_share: 1.0,
                validation_pnl_p10: Some(0.12),
                validation_sharpe_p10: Some(0.8),
                mean_validation_sharpe: Some(1.25),
                score_gap_to_winner: Some(0.0),
                validation_observations: 2,
                total_train_trades: 8,
                total_validation_trades: 4,
                support_failures: 0,
                non_finite_metrics: 0,
                vidya_short_period: Some(10),
                vidya_long_period: Some(30),
                vidya_alpha: Some(0.2),
                ce_period: Some(20),
                ce_mult: Some(3.0),
                ce_use_close: Some(true),
                require_vidya_slope: Some(true),
                max_trade_bars: Some(40),
                params_json: "{\"vcet\":{\"vidya_short_period\":10}}".to_string(),
            }],
            coverage_rows: vec![CoverageReportRow {
                role: "Selection".to_string(),
                dataset_id: "AAPL".to_string(),
                timestamp_count: 1_000,
                requested_start_ms: Some(0),
                requested_end_exclusive_ms: Some(86_400_000),
                observed_min_ms: Some(0),
                observed_max_ms: Some(86_340_000),
                observed_span_days: Some(0.999_305_555_6),
                within_requested: Some(1_000),
                before_requested: Some(0),
                after_requested: Some(0),
                strictly_increasing: true,
                duplicate_intervals: 0,
                descending_intervals: 0,
                gap_status: "complete".to_string(),
                gap_basis: "raw_adjacent_timestamps".to_string(),
                cadence_used_ms: Some(60_000),
                analyzed_intervals: 999,
                irregular_intervals: Some(0),
                gap_intervals: Some(0),
                missing_intervals: Some(0),
                longest_gap_minutes: Some(1.0),
                fold_count: 2,
                coverage_sha256: format!("sha256:{}", "4".repeat(64)),
            }],
            final_rows: Vec::new(),
            evidence_rows: vec![EvidenceReportRow {
                stage: "Universe".to_string(),
                evidence_kind: "vcet_temporal_dataset_universe".to_string(),
                sha256: format!("sha256:{}", "a".repeat(64)),
                status: "Verified".to_string(),
            }],
            admission_rows: vec![AdmissionReportRow {
                metric: "Profitable share of requested datasets".to_string(),
                threshold: 0.5,
                winner_observed: Some(0.75),
                passed: true,
                format_as_percent: true,
            }],
            limitations: vec![("Workbook".to_string(), "Fixture limitation".to_string())],
        };
        let mut final_coverage = model.coverage_rows[0].clone();
        final_coverage.role = "Final test".to_string();
        final_coverage.dataset_id = "MSFT-FINAL".to_string();
        final_coverage.coverage_sha256 = format!("sha256:{}", "5".repeat(64));
        model.coverage_rows.push(final_coverage);
        model
    }

    fn fixture_model_with_final() -> TemporalReportModel {
        let mut model = fixture_model();
        model.final_complete = true;
        model.final_support_passed = Some(true);
        model.final_mean_pnl = Some(0.2);
        model.final_mean_sharpe = Some(1.1);
        model.final_support_share = Some(1.0);
        model.final_trial_count = Some(1);
        model.final_rows.push(FinalReportRow {
            dataset_id: "MSFT-FINAL".to_string(),
            support_passed: true,
            pnl: 0.2,
            sharpe: 1.1,
            sortino: 1.4,
            max_drawdown: 0.08,
            trades: 12,
            exposure: 0.42,
            net_exposure: 0.10,
            final_start_utc_ms: 0,
            final_end_utc_ms: 86_340_000,
            scored_bars: 1_000,
            warmup_bars: 100,
            backend_route: "CpuOnly".to_string(),
            device_id: None,
            metric_contract: "vcet_metrics_v1".to_string(),
            unavailable_metric_fields: "closed_trades, winning_trades".to_string(),
            source_dataset_sha256: format!("sha256:{}", "6".repeat(64)),
            loaded_snapshot_sha256: format!("sha256:{}", "7".repeat(64)),
            coverage_sha256: format!("sha256:{}", "5".repeat(64)),
            attestation_sha256: format!("sha256:{}", "8".repeat(64)),
        });
        model
    }

    fn zip_text(bytes: &[u8], name: &str) -> String {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("valid zip");
        let mut entry = archive.by_name(name).expect("zip entry");
        let mut out = String::new();
        entry.read_to_string(&mut out).expect("UTF-8 XML");
        out
    }

    fn test_hash(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn candidate_binding_fixture() -> (
        VcetTemporalCandidateEntryV1,
        VcetTemporalGlobalCandidateEvidenceV1,
    ) {
        let identity_inputs = VcetCandidateIdentityInputs {
            vidya_short_period: 10,
            vidya_long_period: 30,
            vidya_alpha_micros: 200_000,
            ce_period: 20,
            ce_mult_micros: 3_000_000,
            ce_use_close: true,
            require_vidya_slope: true,
            max_trade_bars: 40,
        };
        let expected = VcetTemporalCandidateEntryV1 {
            ordinal: 0,
            candidate_id: "candidate-binding".to_string(),
            identity_inputs: identity_inputs.clone(),
        };
        let observed = VcetTemporalGlobalCandidateEvidenceV1 {
            global_candidate_ordinal: 0,
            candidate_id: expected.candidate_id.clone(),
            params:
                super::super::super::commands_vcet_temporal_execute::candidate_params_from_manifest(
                    &identity_inputs,
                    &StrategyConfig::default(),
                ),
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
            validation_sharpe_sum: 2.0,
            mean_validation_sharpe: Some(2.0),
            validation_observation_count: 1,
            total_train_trades: 2,
            total_validation_trades: 1,
            support_failure_count: 0,
            non_finite_metric_count: 0,
            admission_passed: true,
            selected_before_final_test: true,
        };
        (expected, observed)
    }

    fn final_dataset_binding_fixture() -> (
        VcetTemporalUniverseDatasetV1,
        VcetTemporalFinalTestDatasetExecutionV1,
        super::super::super::commands_vcet_temporal_prefinal::VcetTemporalGlobalWinnerLockV1,
    ) {
        let (_, candidate) = candidate_binding_fixture();
        let coverage = profile_vcet_dataset_coverage(
            &[0, 60_000, 120_000],
            VcetDatasetCoverageOptions::default(),
        )
        .expect("coverage fixture");
        let expected = VcetTemporalUniverseDatasetV1 {
            dataset_id: "FINAL-A".to_string(),
            source_dataset_sha256: test_hash('1'),
            loaded_candles_snapshot_sha256: test_hash('2'),
            dataset_attestation_sha256: test_hash('3'),
            timeline_sha256: test_hash('4'),
            temporal_interval_fingerprint_sha256: test_hash('5'),
            dataset_coverage: coverage.clone(),
            dataset_coverage_sha256: test_hash('6'),
            fold_count: 2,
            role: VcetTemporalDatasetCohortRoleV1::FinalTest,
        };
        let winner =
            super::super::super::commands_vcet_temporal_prefinal::VcetTemporalGlobalWinnerLockV1 {
                global_candidate_ordinal: 0,
                candidate_id: candidate.candidate_id.clone(),
                params: candidate.params.clone(),
                selection_policy: super::super::super::commands_vcet_temporal_execute::VcetTemporalSelectionPolicyV1::MeanValidationSharpe,
                selection_score: 2.0,
                runner_up_candidate_id: None,
                runner_up_selection_score: None,
                winner_to_runner_up_score_margin: None,
                candidate_id_tie_break_used: false,
                selection_reduction_contract: "mean_validation_sharpe_total_cmp_v1".to_string(),
                universe_sha256: test_hash('7'),
                candidate_manifest_sha256: test_hash('8'),
                plan_sha256: test_hash('9'),
                execution_spec_sha256: test_hash('a'),
                global_pre_final_evidence_sha256: test_hash('b'),
                winner_lock_sha256: test_hash('c'),
            };
        let observed = VcetTemporalFinalTestDatasetExecutionV1 {
            dataset_id: expected.dataset_id.clone(),
            loaded_data_id: "loaded-final-a".to_string(),
            source_dataset_sha256: expected.source_dataset_sha256.clone(),
            dataset_attestation_sha256: expected.dataset_attestation_sha256.clone(),
            timeline_sha256: expected.timeline_sha256.clone(),
            loaded_candles_snapshot_sha256: expected.loaded_candles_snapshot_sha256.clone(),
            dataset_coverage: coverage,
            dataset_coverage_sha256: expected.dataset_coverage_sha256.clone(),
            temporal_interval_fingerprint_sha256: expected
                .temporal_interval_fingerprint_sha256
                .clone(),
            final_test: VcetTemporalEvaluationSliceV1 {
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
            },
            global_candidate_ordinal: winner.global_candidate_ordinal,
            candidate_id: winner.candidate_id.clone(),
            params: winner.params.clone(),
            metrics: Metrics::default(),
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
            observed_trades: 0,
            minimum_trades: 0,
            minimum_trade_support_passed: true,
        };
        (expected, observed, winner)
    }

    #[test]
    fn excel_safe_text_truncates_unicode_without_splitting_scalars() {
        let safe = excel_safe_text(&"🚀".repeat(XLSX_CELL_CHAR_LIMIT + 8));
        assert_eq!(safe.chars().count(), XLSX_CELL_CHAR_LIMIT);
        assert!(safe.ends_with(TRUNCATION_SUFFIX));
    }

    #[test]
    fn unix_milliseconds_are_written_as_typed_excel_dates() {
        assert_eq!(unix_ms_to_excel_serial(0).expect("epoch"), 25_569.0);
        assert_eq!(
            unix_ms_to_excel_serial(43_200_000).expect("half day"),
            25_569.5
        );
        assert!(unix_ms_to_excel_serial(-1)
            .unwrap_err()
            .contains("pre-1970"));
    }

    #[test]
    fn lineage_guard_reports_the_exact_drifted_field() {
        let authority = BaseArtifactLineage {
            universe_sha256: "universe".to_string(),
            candidate_manifest_sha256: "manifest".to_string(),
            plan_sha256: "plan".to_string(),
            temporal_interval_fingerprint_sha256: "interval".to_string(),
        };
        let mut observed = authority.clone();
        observed.plan_sha256 = "other-plan".to_string();
        let error = ensure_base_lineage_matches(&authority, &observed, "test stage")
            .expect_err("drift must fail");
        assert!(error.contains("test stage"));
        assert!(error.contains("plan_sha256"));
    }

    #[test]
    fn candidate_binding_rejects_manifest_id_and_effective_parameter_tamper() {
        let (expected, observed) = candidate_binding_fixture();
        validate_candidate_manifest_binding(&expected, &observed, &StrategyConfig::default())
            .expect("canonical candidate binding");

        let mut id_tamper = observed.clone();
        id_tamper.candidate_id = "foreign-candidate".to_string();
        assert!(validate_candidate_manifest_binding(
            &expected,
            &id_tamper,
            &StrategyConfig::default()
        )
        .unwrap_err()
        .contains("sealed ordinal, ID, or effective parameters"));

        let mut params_tamper = observed;
        params_tamper
            .params
            .vcet
            .as_mut()
            .expect("VCET params")
            .vidya_short_period += 1;
        assert!(validate_candidate_manifest_binding(
            &expected,
            &params_tamper,
            &StrategyConfig::default()
        )
        .is_err());
    }

    #[test]
    fn selection_shard_binding_rejects_descriptor_geometry_tamper() {
        let expected = VcetTemporalUniverseShardV1 {
            role: VcetTemporalDatasetCohortRoleV1::Selection,
            shard_ordinal: 0,
            shard_id: "vcet-temporal-shard-v1-selection".to_string(),
            dataset_ids: vec!["SELECTION-A".to_string()],
            candidate_start_index: 0,
            candidate_end_index_exclusive: 1,
            locked_winner_only: false,
            expected_candidate_window_trial_count: 2,
        };
        let observed = VcetTemporalPreFinalShardReductionEvidenceV1 {
            shard_ordinal: 0,
            shard_id: expected.shard_id.clone(),
            artifact_sha256: test_hash('d'),
            dataset_count: 1,
            candidate_count: 1,
            candidate_window_trial_count: 2,
        };
        validate_selection_shard_binding(&expected, &observed)
            .expect("canonical selection shard binding");

        let mut tampered = observed;
        tampered.dataset_count = 2;
        assert!(validate_selection_shard_binding(&expected, &tampered)
            .unwrap_err()
            .contains("sealed selection descriptor"));
    }

    #[test]
    fn final_dataset_binding_rejects_attestation_timeline_coverage_and_winner_tamper() {
        let (expected, observed, winner) = final_dataset_binding_fixture();
        validate_final_dataset_binding(&expected, &observed, &winner, 0, 2)
            .expect("canonical final dataset binding");

        let mut attestation = observed.clone();
        attestation.dataset_attestation_sha256 = test_hash('e');
        assert!(validate_final_dataset_binding(&expected, &attestation, &winner, 0, 2).is_err());

        let mut timeline = observed.clone();
        timeline.timeline_sha256 = test_hash('f');
        assert!(validate_final_dataset_binding(&expected, &timeline, &winner, 0, 2).is_err());

        let mut coverage = observed.clone();
        coverage.dataset_coverage.timestamp_count += 1;
        assert!(validate_final_dataset_binding(&expected, &coverage, &winner, 0, 2).is_err());

        let mut winner_tamper = observed;
        winner_tamper.candidate_id = "foreign-winner".to_string();
        assert!(validate_final_dataset_binding(&expected, &winner_tamper, &winner, 0, 2).is_err());
    }

    #[test]
    fn final_shard_binding_rejects_authorization_and_dataset_partition_tamper() {
        let (_, dataset, winner) = final_dataset_binding_fixture();
        let expected = VcetTemporalUniverseShardV1 {
            role: VcetTemporalDatasetCohortRoleV1::FinalTest,
            shard_ordinal: 0,
            shard_id: "vcet-temporal-shard-v1-final".to_string(),
            dataset_ids: vec![dataset.dataset_id.clone()],
            candidate_start_index: 0,
            candidate_end_index_exclusive: 0,
            locked_winner_only: true,
            expected_candidate_window_trial_count: 1,
        };
        let authorization = VcetTemporalFinalShardAuthorizationV1 {
            shard: expected.clone(),
            universe_sha256: winner.universe_sha256,
            winner_lock_sha256: winner.winner_lock_sha256,
            global_candidate_ordinal: winner.global_candidate_ordinal,
            candidate_id: winner.candidate_id,
            global_pre_final_evidence_sha256: winner.global_pre_final_evidence_sha256,
            final_stage_nonce: "nonce-fixture".to_string(),
            expected_prior_state_sha256: test_hash('1'),
            authorization_sha256: test_hash('2'),
        };
        let observed = VcetTemporalFinalShardReductionEvidenceV1 {
            shard_ordinal: 0,
            shard_id: expected.shard_id.clone(),
            authorization_sha256: authorization.authorization_sha256.clone(),
            authorized_prior_state_sha256: authorization.expected_prior_state_sha256.clone(),
            completed_state_sha256: test_hash('3'),
            artifact_sha256: test_hash('4'),
            dataset_count: 1,
            candidate_window_trial_count: 1,
            minimum_trade_support_passed: true,
        };
        validate_final_shard_binding(
            &expected,
            &authorization,
            &observed,
            std::slice::from_ref(&dataset),
        )
        .expect("canonical final shard binding");

        let mut authorization_tamper = observed.clone();
        authorization_tamper.authorization_sha256 = test_hash('5');
        assert!(validate_final_shard_binding(
            &expected,
            &authorization,
            &authorization_tamper,
            std::slice::from_ref(&dataset),
        )
        .is_err());

        let mut foreign_dataset = dataset;
        foreign_dataset.dataset_id = "FINAL-FOREIGN".to_string();
        assert!(validate_final_shard_binding(
            &expected,
            &authorization,
            &observed,
            std::slice::from_ref(&foreign_dataset),
        )
        .is_err());
    }

    #[test]
    fn model_writer_emits_five_named_sheets_tables_chart_and_pending_final_state() {
        let bytes = build_report_workbook_bytes(&fixture_model()).expect("xlsx bytes");
        if let Some(path) = std::env::var_os("VECTORGRID_VCET_TEMPORAL_XLSX_FIXTURE_OUT") {
            let path = std::path::PathBuf::from(path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create fixture parent directory");
            }
            let complete_fixture = build_report_workbook_bytes(&fixture_model_with_final())
                .expect("complete fixture xlsx bytes");
            std::fs::write(&path, complete_fixture).expect("write VCET temporal XLSX fixture");
        }
        assert!(bytes.starts_with(b"PK"));
        let workbook_xml = zip_text(&bytes, "xl/workbook.xml");
        for name in [
            SUMMARY_SHEET,
            CANDIDATE_SHEET,
            COVERAGE_SHEET,
            FINAL_SHEET,
            PROVENANCE_SHEET,
        ] {
            assert!(workbook_xml.contains(&format!("name=\"{name}\"")));
        }
        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).expect("valid zip");
        let names = (0..archive.len())
            .map(|index| archive.by_index(index).expect("entry").name().to_string())
            .collect::<Vec<_>>();
        assert!(names.iter().any(|name| name.starts_with("xl/tables/table")));
        assert!(names.iter().any(|name| name.starts_with("xl/charts/chart")));
        drop(archive);
        let shared_strings = zip_text(&bytes, "xl/sharedStrings.xml");
        assert!(shared_strings.contains("Final test is pending"));
        assert!(shared_strings.contains("candidate-fixture"));
    }

    #[test]
    fn complete_final_model_emits_typed_final_table_and_support_evidence() {
        let bytes =
            build_report_workbook_bytes(&fixture_model_with_final()).expect("complete xlsx bytes");
        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).expect("valid zip");
        let table_names = (0..archive.len())
            .map(|index| archive.by_index(index).expect("entry").name().to_string())
            .filter(|name| name.starts_with("xl/tables/table"))
            .collect::<Vec<_>>();
        drop(archive);
        assert!(table_names.iter().any(|name| {
            zip_text(&bytes, name).contains("displayName=\"VcetTemporalFinalTestTable\"")
        }));
        let final_sheet = zip_text(&bytes, "xl/worksheets/sheet4.xml");
        assert!(final_sheet.contains("<v>0.2</v>"));
        assert!(final_sheet.contains("<v>1.1</v>"));
        let shared_strings = zip_text(&bytes, "xl/sharedStrings.xml");
        assert!(shared_strings.contains("MSFT-FINAL"));
        assert!(shared_strings.contains("Support passed"));
        assert!(shared_strings.contains("closed_trades, winning_trades"));
    }

    #[test]
    fn prominent_summary_and_candidate_kpis_are_typed_numeric_cells() {
        fn cell_xml<'a>(sheet: &'a str, reference: &str) -> &'a str {
            let marker = format!("<c r=\"{reference}\"");
            let start = sheet.find(&marker).expect("cell marker");
            let tail = &sheet[start..];
            let end = tail.find("</c>").expect("cell end") + "</c>".len();
            &tail[..end]
        }

        let bytes =
            build_report_workbook_bytes(&fixture_model_with_final()).expect("complete xlsx bytes");
        let summary = zip_text(&bytes, "xl/worksheets/sheet1.xml");
        for (reference, expected_value) in [
            ("C9", "<v>1</v>"),
            ("C15", "<v>0.12</v>"),
            ("C17", "<v>0.2</v>"),
            ("C18", "<v>1</v>"),
        ] {
            let cell = cell_xml(&summary, reference);
            assert!(!cell.contains("t=\"s\""), "{reference} must be numeric");
            assert!(cell.contains(expected_value), "{reference}: {cell}");
        }

        let candidates = zip_text(&bytes, "xl/worksheets/sheet2.xml");
        for (reference, expected_value) in [("B5", "<v>0</v>"), ("B6", "<v>1.25</v>")] {
            let cell = cell_xml(&candidates, reference);
            assert!(!cell.contains("t=\"s\""), "{reference} must be numeric");
            assert!(cell.contains(expected_value), "{reference}: {cell}");
        }
    }

    #[test]
    fn model_writer_is_deterministic_and_rejects_unbounded_candidate_rows() {
        let model = fixture_model();
        let first = build_report_workbook_bytes(&model).expect("first xlsx");
        let second = build_report_workbook_bytes(&model).expect("second xlsx");
        assert_eq!(first, second);

        let mut invalid = model;
        invalid.candidate_rows.clear();
        assert!(build_report_workbook_bytes(&invalid)
            .unwrap_err()
            .contains("candidate rows"));
    }
}
