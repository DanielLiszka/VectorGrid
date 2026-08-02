use super::commands_vcet_temporal::{
    plan_vcet_temporal_validation_blocking, PlanVcetTemporalValidationRequestV1,
    PlanVcetTemporalValidationResponseV1, VcetTemporalCandidateManifestEvidenceV1,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use ta_app_core::composite_cpu::{MaczGateParams, OttGateParams};
use ta_app_core::composite_validate::{
    evaluate_vcet_fixed_candidates_scored_window_with_candles, CompositeParams,
    CompositeValidationRequest, VcetFixedCandidateScoredWindowResult,
    VcetFixedCandidateWindowBackendProvenance,
};
use ta_app_core::double_ma::{Backend, HalfTrendRegimeFilter};
use ta_app_core::iterative_sim::{IterativeSimConfig, ValidationEngine};
use ta_app_core::vcet::VcetStrategyParams;
use ta_app_core::vcet_dataset_coverage::{
    profile_vcet_dataset_coverage, VcetDatasetCoverageEvidence, VcetDatasetCoverageOptions,
    VcetRequestedCoverageBoundsMs,
};
use ta_app_core::vcet_temporal::{
    VcetTemporalBarRangeV1, VcetTemporalLayoutV1, VcetTemporalPlanIdentityV1,
    VcetTemporalPositionBoundaryPolicyV1,
};
use ta_strategies::composite_cpu::CompositeSignalMode;
use ta_strategies::double_ma::{Metrics, StrategyConfig};
use vector_ta::utilities::data_loader::Candles;

pub const VCET_TEMPORAL_EXECUTION_ARTIFACT_SCHEMA_VERSION: u16 = 1;
pub const MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST: usize = 64;
pub const MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS: usize = 100_000;

const EXECUTION_ARTIFACT_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_execution_artifact.v1";
const PRE_FINAL_EVIDENCE_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_pre_final_evidence.v1";
const WINNER_LOCK_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_winner_lock.v1";
const LOADED_CANDLES_HASH_CONTRACT: &str = "vectorgrid.vcet.loaded_candles_snapshot.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalLoadedDatasetBindingV1 {
    pub dataset_id: String,
    pub loaded_data_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VcetTemporalSelectionPolicyV1 {
    MeanValidationSharpe,
}

impl VcetTemporalSelectionPolicyV1 {
    fn label(self) -> &'static str {
        match self {
            Self::MeanValidationSharpe => "mean_validation_sharpe",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteVcetTemporalValidationRequestV1 {
    pub planning_request: PlanVcetTemporalValidationRequestV1,
    pub planning_evidence: PlanVcetTemporalValidationResponseV1,
    pub expected_planning_evidence_sha256: String,
    pub loaded_datasets: Vec<VcetTemporalLoadedDatasetBindingV1>,
    pub backend: Backend,
    pub ma_source: String,
    pub strategy: StrategyConfig,
    pub selection_policy: VcetTemporalSelectionPolicyV1,
}

#[derive(Clone, Debug, Serialize)]
pub struct VcetTemporalExecutionSpecEvidenceV1 {
    pub requested_backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<u32>,
    pub ma_source: String,
    pub strategy: StrategyConfig,
    pub position_boundary_policy: VcetTemporalPositionBoundaryPolicyV1,
    pub selection_policy: VcetTemporalSelectionPolicyV1,
    pub selection_metric: String,
    pub trade_support_basis: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct VcetTemporalWindowCandidateMetricsV1 {
    pub candidate_index: usize,
    pub candidate_id: String,
    pub metrics: Metrics,
}

#[derive(Clone, Debug, Serialize)]
pub struct VcetTemporalWindowExecutionV1 {
    pub scored_bars: VcetTemporalBarRangeV1,
    pub backend: VcetFixedCandidateWindowBackendProvenance,
    pub requested_candidate_count: usize,
    pub evaluated_candidate_count: usize,
    pub candidates: Vec<VcetTemporalWindowCandidateMetricsV1>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VcetTemporalFoldExecutionV1 {
    pub fold_index: usize,
    pub train: VcetTemporalWindowExecutionV1,
    pub validation: VcetTemporalWindowExecutionV1,
}

#[derive(Clone, Debug, Serialize)]
pub struct VcetTemporalDatasetExecutionV1 {
    pub dataset_id: String,
    pub loaded_data_id: String,
    pub source_dataset_sha256: String,
    pub timeline_sha256: String,
    pub loaded_candles_snapshot_sha256: String,
    pub dataset_coverage: VcetDatasetCoverageEvidence,
    pub layout: VcetTemporalLayoutV1,
    pub folds: Vec<VcetTemporalFoldExecutionV1>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VcetTemporalTradeSupportFailureV1 {
    pub dataset_id: String,
    pub fold_index: usize,
    pub stage: String,
    pub observed_trades: u64,
    pub minimum_trades: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct VcetTemporalCandidateSelectionEvidenceV1 {
    pub candidate_index: usize,
    pub candidate_id: String,
    pub params: CompositeParams,
    pub validation_observation_count: usize,
    pub mean_validation_sharpe: f64,
    pub total_train_trades: u64,
    pub total_validation_trades: u64,
    pub minimum_trade_support_passed: bool,
    pub support_failures: Vec<VcetTemporalTradeSupportFailureV1>,
    pub selected_before_final_test: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct VcetTemporalWinnerLockV1 {
    pub candidate_index: usize,
    pub candidate_id: String,
    pub params: CompositeParams,
    pub selection_policy: VcetTemporalSelectionPolicyV1,
    pub selection_score: f64,
    pub candidate_manifest_sha256: String,
    pub plan_sha256: String,
    pub pre_final_evidence_sha256: String,
    pub winner_lock_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct VcetTemporalFinalTestDatasetResultV1 {
    pub dataset_id: String,
    pub loaded_data_id: String,
    pub scored_bars: VcetTemporalBarRangeV1,
    pub candidate_index: usize,
    pub candidate_id: String,
    pub params: CompositeParams,
    pub metrics: Metrics,
    pub backend: VcetFixedCandidateWindowBackendProvenance,
    pub observed_trades: u64,
    pub minimum_trades: u64,
    pub minimum_trade_support_passed: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VcetTemporalExecutionStatusV1 {
    CompletedFinalTestPassed,
    CompletedFinalTestSupportFailed,
}

#[derive(Clone, Debug, Serialize)]
pub struct VcetTemporalExecutionArtifactPayloadV1 {
    pub schema_version: u16,
    pub evidence_kind: String,
    pub execution_status: VcetTemporalExecutionStatusV1,
    pub planning_evidence_sha256: String,
    pub candidate_manifest: VcetTemporalCandidateManifestEvidenceV1,
    pub plan_identity: VcetTemporalPlanIdentityV1,
    pub execution_spec: VcetTemporalExecutionSpecEvidenceV1,
    pub dataset_count: usize,
    pub candidate_count: usize,
    pub fold_count_per_dataset: usize,
    pub pre_final_candidate_window_trial_count: usize,
    pub final_test_candidate_window_trial_count: usize,
    pub total_candidate_window_trial_count: usize,
    pub final_test_access_count: usize,
    pub final_test_winner_count: usize,
    pub final_test_dataset_evaluation_count: usize,
    pub selection_used_final_test: bool,
    pub winner_reranked_after_final_test: bool,
    pub pre_final_evidence_sha256: String,
    pub candidates: Vec<VcetTemporalCandidateSelectionEvidenceV1>,
    pub datasets: Vec<VcetTemporalDatasetExecutionV1>,
    pub winner_lock: VcetTemporalWinnerLockV1,
    pub final_test_results: Vec<VcetTemporalFinalTestDatasetResultV1>,
    pub final_test_minimum_trade_support_passed: bool,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ExecuteVcetTemporalValidationResponseV1 {
    pub payload: VcetTemporalExecutionArtifactPayloadV1,
    pub artifact_sha256: String,
}

#[derive(Serialize)]
struct PreFinalEvidenceHashInput<'a> {
    hash_contract: &'static str,
    planning_evidence_sha256: &'a str,
    candidate_manifest_sha256: &'a str,
    plan_sha256: &'a str,
    execution_spec: &'a VcetTemporalExecutionSpecEvidenceV1,
    candidates: &'a [VcetTemporalCandidateSelectionEvidenceV1],
    datasets: &'a [VcetTemporalDatasetExecutionV1],
    pre_final_candidate_window_trial_count: usize,
}

#[derive(Serialize)]
struct WinnerLockHashInput<'a> {
    hash_contract: &'static str,
    candidate_index: usize,
    candidate_id: &'a str,
    params: &'a CompositeParams,
    selection_policy: VcetTemporalSelectionPolicyV1,
    selection_score: f64,
    candidate_manifest_sha256: &'a str,
    plan_sha256: &'a str,
    pre_final_evidence_sha256: &'a str,
}

#[derive(Serialize)]
struct ExecutionArtifactHashInput<'a> {
    hash_contract: &'static str,
    payload: &'a VcetTemporalExecutionArtifactPayloadV1,
}

#[derive(Default)]
struct CandidateAccumulator {
    validation_sharpe_sum: f64,
    validation_observation_count: usize,
    total_train_trades: u64,
    total_validation_trades: u64,
    support_failures: Vec<VcetTemporalTradeSupportFailureV1>,
}

fn first_non_finite_metric_field(metrics: &Metrics) -> Option<&'static str> {
    [
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
    ]
    .into_iter()
    .find_map(|(field, value)| (!value.is_finite()).then_some(field))
}

fn validate_finite_metrics(
    metrics: &Metrics,
    context: std::fmt::Arguments<'_>,
) -> Result<(), String> {
    if let Some(field) = first_non_finite_metric_field(metrics) {
        return Err(format!(
            "VCET temporal execution produced non-finite {context} metric '{field}'; artifact publication was rejected"
        ));
    }
    Ok(())
}

fn validate_finite_score(value: f64, context: std::fmt::Arguments<'_>) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!(
            "VCET temporal execution produced non-finite {context}; artifact publication was rejected"
        ));
    }
    Ok(())
}

fn checked_validation_sharpe_sum(
    current: f64,
    observation: f64,
    candidate_index: usize,
) -> Result<f64, String> {
    validate_finite_score(
        current,
        format_args!("candidate {candidate_index} validation Sharpe accumulator"),
    )?;
    validate_finite_score(
        observation,
        format_args!("candidate {candidate_index} validation Sharpe observation"),
    )?;
    let next = current + observation;
    validate_finite_score(
        next,
        format_args!("candidate {candidate_index} validation Sharpe accumulator"),
    )?;
    Ok(next)
}

fn candidate_mean_validation_sharpe(
    accumulator: &CandidateAccumulator,
    candidate_index: usize,
) -> Result<f64, String> {
    if accumulator.validation_observation_count == 0 {
        return Err(format!(
            "VCET temporal candidate {candidate_index} has no validation Sharpe observations; artifact publication was rejected"
        ));
    }
    validate_finite_score(
        accumulator.validation_sharpe_sum,
        format_args!("candidate {candidate_index} validation Sharpe accumulator"),
    )?;
    let mean = accumulator.validation_sharpe_sum / accumulator.validation_observation_count as f64;
    validate_finite_score(
        mean,
        format_args!("candidate {candidate_index} mean_validation_sharpe"),
    )?;
    Ok(mean)
}

fn validate_pre_final_numeric_contract(
    candidates: &[VcetTemporalCandidateSelectionEvidenceV1],
    datasets: &[VcetTemporalDatasetExecutionV1],
) -> Result<(), String> {
    for candidate in candidates {
        validate_finite_score(
            candidate.mean_validation_sharpe,
            format_args!(
                "candidate {} mean_validation_sharpe",
                candidate.candidate_index
            ),
        )?;
    }
    for dataset in datasets {
        for fold in &dataset.folds {
            for candidate in &fold.train.candidates {
                validate_finite_metrics(
                    &candidate.metrics,
                    format_args!(
                        "dataset '{}' fold {} train candidate {}",
                        dataset.dataset_id, fold.fold_index, candidate.candidate_index
                    ),
                )?;
            }
            for candidate in &fold.validation.candidates {
                validate_finite_metrics(
                    &candidate.metrics,
                    format_args!(
                        "dataset '{}' fold {} validation candidate {}",
                        dataset.dataset_id, fold.fold_index, candidate.candidate_index
                    ),
                )?;
            }
        }
    }
    Ok(())
}

fn validate_execution_artifact_numeric_contract(
    payload: &VcetTemporalExecutionArtifactPayloadV1,
) -> Result<(), String> {
    validate_pre_final_numeric_contract(&payload.candidates, &payload.datasets)?;
    validate_finite_score(
        payload.winner_lock.selection_score,
        format_args!("winner lock selection_score"),
    )?;
    for result in &payload.final_test_results {
        validate_finite_metrics(
            &result.metrics,
            format_args!("dataset '{}' final-test winner", result.dataset_id),
        )?;
    }
    Ok(())
}

pub(super) struct LoadedVcetTemporalDataset {
    pub(super) dataset_id: String,
    pub(super) loaded_data_id: String,
    pub(super) candles: Arc<Candles>,
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

pub(super) fn sha256_serializable(value: &impl Serialize, label: &str) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_prefixed(&bytes))
        .map_err(|error| format!("failed to serialize {label} for SHA-256: {error}"))
}

fn hash_f64_slice(hasher: &mut Sha256, label: &[u8], values: &[f64]) {
    hasher.update((label.len() as u64).to_le_bytes());
    hasher.update(label);
    hasher.update((values.len() as u64).to_le_bytes());
    for value in values {
        hasher.update(value.to_bits().to_le_bytes());
    }
}

pub(super) fn loaded_candles_snapshot_sha256(candles: &Candles) -> String {
    let mut hasher = Sha256::new();
    hasher.update(LOADED_CANDLES_HASH_CONTRACT.as_bytes());
    hasher.update([0]);
    hasher.update((candles.timestamp.len() as u64).to_le_bytes());
    for timestamp in &candles.timestamp {
        hasher.update(timestamp.to_le_bytes());
    }
    hash_f64_slice(&mut hasher, b"open", &candles.open);
    hash_f64_slice(&mut hasher, b"high", &candles.high);
    hash_f64_slice(&mut hasher, b"low", &candles.low);
    hash_f64_slice(&mut hasher, b"close", &candles.close);
    hash_f64_slice(&mut hasher, b"volume", &candles.volume);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(71);
    out.push_str("sha256:");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn backend_request_evidence(backend: &Backend) -> Result<(String, Option<u32>), String> {
    match backend {
        Backend::CpuOnly => Ok(("CpuOnly".to_string(), None)),
        Backend::GpuKernel { device_id } => Ok(("GpuKernel".to_string(), Some(*device_id))),
        _ => Err(
            "VCET temporal execution requires CpuOnly or strict GpuKernel; fallback-capable routes are rejected"
                .to_string(),
        ),
    }
}

pub(super) fn canonical_vcet_execution_ma_source(raw: &str) -> Result<String, String> {
    let source = raw.trim().to_ascii_lowercase();
    match source.as_str() {
        "open" | "high" | "low" | "close" | "volume" | "hl2" | "hlc3" | "ohlc4"
        | "hlcc4" => Ok(source),
        "hlcc" => Ok("hlcc4".to_string()),
        _ => Err(format!(
            "VCET temporal ma_source '{raw}' is unsupported; expected open, high, low, close, volume, hl2, hlc3, ohlc4, or hlcc4"
        )),
    }
}

pub(super) fn candidate_params_from_manifest(
    identity: &ta_app_core::vcet_search::VcetCandidateIdentityInputs,
    strategy: &StrategyConfig,
) -> CompositeParams {
    CompositeParams {
        fast_ma_type: "ema".to_string(),
        slow_ma_type: "ema".to_string(),
        fast_len: identity.vidya_short_period,
        slow_len: identity.vidya_long_period,
        halftrend: HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        },
        asr: None,
        vcc: None,
        vemr: None,
        rocb: None,
        vcet: Some(VcetStrategyParams {
            map_fast_to_vidya_short_period: false,
            map_slow_to_vidya_long_period: false,
            map_slow_to_ce_period: false,
            vidya_short_period: identity.vidya_short_period,
            vidya_long_period: identity.vidya_long_period,
            vidya_alpha: identity.vidya_alpha_micros as f64 / 1_000_000.0,
            ce_period: identity.ce_period,
            ce_mult: identity.ce_mult_micros as f64 / 1_000_000.0,
            ce_use_close: identity.ce_use_close,
            require_vidya_slope: identity.require_vidya_slope,
            max_trade_bars: identity.max_trade_bars,
        }),
        mkcs: None,
        kqsr: None,
        carm: None,
        kvdf: None,
        cdmc: None,
        macz: MaczGateParams::default(),
        ott: OttGateParams::default(),
        signal_mode: CompositeSignalMode::HardGate,
        strategy: strategy.clone(),
    }
}

fn direct_window_request(
    data_id: String,
    backend: Backend,
    ma_source: String,
    candidates: Vec<CompositeParams>,
) -> CompositeValidationRequest {
    CompositeValidationRequest {
        data_id,
        backend,
        ma_source,
        fast_ma_params: None,
        slow_ma_params: None,
        candidates,
        walk_forward: None,
        reopt_walk_forward: None,
        spa: None,
        perturb: None,
        trade_bootstrap: None,
        monte_carlo: None,
        cross_ticker: None,
        cscv_pbo: None,
        seed: 0,
        max_candidates: usize::MAX,
        validation_engine: ValidationEngine::Vectorized,
        iterative: IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: None,
    }
}

fn map_window_result(
    result: VcetFixedCandidateScoredWindowResult,
    candidate_ids: &[String],
) -> Result<VcetTemporalWindowExecutionV1, String> {
    if result.candidates.len() != candidate_ids.len()
        || result.evaluated_candidate_count != candidate_ids.len()
        || result.requested_candidate_count != candidate_ids.len()
    {
        return Err(format!(
            "VCET temporal window returned {} candidates for {} sealed candidates",
            result.candidates.len(),
            candidate_ids.len()
        ));
    }
    let candidates = result
        .candidates
        .into_iter()
        .enumerate()
        .map(|(expected_index, candidate)| {
            if candidate.candidate_index != expected_index {
                return Err(format!(
                    "VCET temporal window candidate index {} does not match sealed ordinal {expected_index}",
                    candidate.candidate_index
                ));
            }
            validate_finite_metrics(
                &candidate.metrics,
                format_args!("window candidate {expected_index}"),
            )?;
            Ok(VcetTemporalWindowCandidateMetricsV1 {
                candidate_index: expected_index,
                candidate_id: candidate_ids[expected_index].clone(),
                metrics: candidate.metrics,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(VcetTemporalWindowExecutionV1 {
        scored_bars: result.scored_bars,
        backend: result.backend,
        requested_candidate_count: result.requested_candidate_count,
        evaluated_candidate_count: result.evaluated_candidate_count,
        candidates,
    })
}

pub(super) fn evaluate_window(
    dataset_id: &str,
    candles: &Candles,
    backend: &Backend,
    ma_source: &str,
    candidates: &[CompositeParams],
    candidate_ids: &[String],
    scored_bars: VcetTemporalBarRangeV1,
    position_boundary_policy: VcetTemporalPositionBoundaryPolicyV1,
) -> Result<VcetTemporalWindowExecutionV1, String> {
    let direct_request = direct_window_request(
        dataset_id.to_string(),
        backend.clone(),
        ma_source.to_string(),
        candidates.to_vec(),
    );
    let result = evaluate_vcet_fixed_candidates_scored_window_with_candles(
        &direct_request,
        candles,
        scored_bars,
        position_boundary_policy,
    )?;
    map_window_result(result, candidate_ids)
}

fn checked_trial_counts(
    candidate_count: usize,
    dataset_count: usize,
    fold_count: usize,
) -> Result<(usize, usize, usize), String> {
    let pre_final = candidate_count
        .checked_mul(dataset_count)
        .and_then(|value| value.checked_mul(fold_count))
        .and_then(|value| value.checked_mul(2))
        .ok_or_else(|| "VCET temporal pre-final trial count overflow".to_string())?;
    let final_test = dataset_count;
    let total = pre_final
        .checked_add(final_test)
        .ok_or_else(|| "VCET temporal total trial count overflow".to_string())?;
    if total > MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS {
        return Err(format!(
            "VCET temporal execution requires {total} candidate-window trials, exceeding the bounded limit of {MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS}"
        ));
    }
    Ok((pre_final, final_test, total))
}

pub(super) fn validate_loaded_dataset_shape(
    dataset_id: &str,
    candles: &Candles,
) -> Result<(), String> {
    let n = candles.timestamp.len();
    if n < 2 {
        return Err(format!(
            "VCET temporal loaded dataset '{dataset_id}' requires at least two candles"
        ));
    }
    for (field, len) in [
        ("open", candles.open.len()),
        ("high", candles.high.len()),
        ("low", candles.low.len()),
        ("close", candles.close.len()),
        ("volume", candles.volume.len()),
    ] {
        if len != n {
            return Err(format!(
                "VCET temporal loaded dataset '{dataset_id}' {field} length {len} does not match timestamp length {n}"
            ));
        }
    }
    Ok(())
}

pub(super) fn profile_loaded_dataset_coverage(
    req: &ExecuteVcetTemporalValidationRequestV1,
    dataset_id: &str,
    candles: &Candles,
) -> Result<VcetDatasetCoverageEvidence, String> {
    profile_loaded_dataset_coverage_for_planning(&req.planning_request, dataset_id, candles)
}

pub(super) fn profile_loaded_dataset_coverage_for_planning(
    planning_request: &PlanVcetTemporalValidationRequestV1,
    dataset_id: &str,
    candles: &Candles,
) -> Result<VcetDatasetCoverageEvidence, String> {
    let timeline = planning_request
        .datasets
        .iter()
        .find(|dataset| dataset.dataset_id.trim() == dataset_id)
        .ok_or_else(|| {
            format!(
                "VCET temporal coverage could not find sealed timeline for dataset '{dataset_id}'"
            )
        })?;
    let end_exclusive_ms = planning_request
        .plan
        .requested_bounds
        .utc
        .end_utc_ms_inclusive
        .checked_add(1)
        .ok_or_else(|| {
            "VCET temporal requested UTC end cannot convert to half-open coverage bounds"
                .to_string()
        })?;
    let requested_bounds = VcetRequestedCoverageBoundsMs {
        start_ms: planning_request
            .plan
            .requested_bounds
            .utc
            .start_utc_ms_inclusive,
        end_exclusive_ms,
    };
    let session_interval_mask = timeline.session_open_utc_ms.as_ref().map(|sessions| {
        sessions
            .windows(2)
            .map(|pair| pair[0] == pair[1])
            .collect::<Vec<_>>()
    });
    profile_vcet_dataset_coverage(
        &candles.timestamp,
        VcetDatasetCoverageOptions {
            requested_bounds: Some(requested_bounds),
            expected_cadence_ms: None,
            session_interval_mask: session_interval_mask.as_deref(),
        },
    )
    .map_err(|error| format!("VCET temporal dataset '{dataset_id}' coverage failed: {error}"))
}

fn prepare_loaded_datasets(
    req: &ExecuteVcetTemporalValidationRequestV1,
    loaded: Vec<LoadedVcetTemporalDataset>,
    canonical_planning: &PlanVcetTemporalValidationResponseV1,
) -> Result<Vec<LoadedVcetTemporalDataset>, String> {
    if req.loaded_datasets.is_empty() {
        return Err("VCET temporal execution requires at least one loaded dataset".to_string());
    }
    if req.loaded_datasets.len() > MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST {
        return Err(format!(
            "VCET temporal execution accepts at most {MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST} loaded datasets per request"
        ));
    }
    if req.loaded_datasets.len() != canonical_planning.datasets.len() {
        return Err(format!(
            "VCET temporal execution received {} loaded bindings for {} planned datasets",
            req.loaded_datasets.len(),
            canonical_planning.datasets.len()
        ));
    }

    let mut bindings = BTreeMap::new();
    let mut loaded_data_ids = BTreeMap::new();
    for binding in &req.loaded_datasets {
        let dataset_id = binding.dataset_id.trim();
        let loaded_data_id = binding.loaded_data_id.trim();
        if dataset_id.is_empty() || loaded_data_id.is_empty() {
            return Err(
                "VCET temporal dataset_id and loaded_data_id bindings must not be empty"
                    .to_string(),
            );
        }
        if bindings
            .insert(dataset_id.to_string(), loaded_data_id.to_string())
            .is_some()
        {
            return Err(format!(
                "duplicate VCET temporal loaded binding for dataset_id '{dataset_id}'"
            ));
        }
        if loaded_data_ids
            .insert(loaded_data_id.to_string(), dataset_id.to_string())
            .is_some()
        {
            return Err(format!(
                "VCET temporal loaded_data_id '{loaded_data_id}' cannot be reused for multiple planned datasets"
            ));
        }
    }

    let mut loaded_by_dataset = BTreeMap::new();
    for item in loaded {
        if loaded_by_dataset
            .insert(item.dataset_id.clone(), item)
            .is_some()
        {
            return Err("duplicate resolved VCET temporal loaded dataset".to_string());
        }
    }

    let mut planning_timeline_by_dataset = BTreeMap::new();
    for dataset in &req.planning_request.datasets {
        if planning_timeline_by_dataset
            .insert(dataset.dataset_id.trim().to_string(), dataset)
            .is_some()
        {
            return Err(format!(
                "duplicate dataset_id '{}' in VCET temporal planning request",
                dataset.dataset_id
            ));
        }
    }

    let mut out = Vec::with_capacity(canonical_planning.datasets.len());
    for planned in &canonical_planning.datasets {
        let expected_loaded_data_id = bindings.get(&planned.dataset_id).ok_or_else(|| {
            format!(
                "missing loaded binding for planned VCET dataset '{}'",
                planned.dataset_id
            )
        })?;
        let item = loaded_by_dataset
            .remove(&planned.dataset_id)
            .ok_or_else(|| {
                format!(
                    "loaded candles for planned VCET dataset '{}' were not resolved",
                    planned.dataset_id
                )
            })?;
        if &item.loaded_data_id != expected_loaded_data_id {
            return Err(format!(
                "resolved loaded_data_id '{}' does not match binding '{}' for VCET dataset '{}'",
                item.loaded_data_id, expected_loaded_data_id, planned.dataset_id
            ));
        }
        validate_loaded_dataset_shape(&planned.dataset_id, &item.candles)?;
        let timeline = planning_timeline_by_dataset
            .get(&planned.dataset_id)
            .ok_or_else(|| {
                format!(
                    "planned VCET timeline '{}' is missing from the sealed request",
                    planned.dataset_id
                )
            })?;
        if item.candles.timestamp != timeline.timestamps_utc_ms {
            return Err(format!(
                "loaded candle timestamps for VCET dataset '{}' do not exactly match the sealed planning timeline",
                planned.dataset_id
            ));
        }
        if item.candles.timestamp.len() != planned.observed_bar_count
            || item.candles.timestamp.len() != planned.layout.dataset.observation.bar_count
        {
            return Err(format!(
                "loaded candle count for VCET dataset '{}' does not match planning evidence",
                planned.dataset_id
            ));
        }
        out.push(item);
    }
    if !loaded_by_dataset.is_empty() {
        return Err("unplanned loaded VCET datasets were supplied".to_string());
    }
    Ok(out)
}

fn update_candidate_accumulators(
    accumulators: &mut [CandidateAccumulator],
    dataset_id: &str,
    fold_index: usize,
    train: &VcetTemporalWindowExecutionV1,
    validation: &VcetTemporalWindowExecutionV1,
    min_train_trades: u64,
    min_validation_trades: u64,
) -> Result<(), String> {
    if train.candidates.len() != accumulators.len()
        || validation.candidates.len() != accumulators.len()
    {
        return Err("VCET temporal accumulator candidate count mismatch".to_string());
    }
    for candidate_index in 0..accumulators.len() {
        let train_candidate = &train.candidates[candidate_index];
        let validation_candidate = &validation.candidates[candidate_index];
        if train_candidate.candidate_index != candidate_index
            || validation_candidate.candidate_index != candidate_index
            || train_candidate.candidate_id != validation_candidate.candidate_id
        {
            return Err(format!(
                "VCET temporal fold candidate order mismatch at candidate {candidate_index}"
            ));
        }
        validate_finite_metrics(
            &train_candidate.metrics,
            format_args!(
                "dataset '{dataset_id}' fold {fold_index} train candidate {candidate_index}"
            ),
        )?;
        validate_finite_metrics(
            &validation_candidate.metrics,
            format_args!(
                "dataset '{dataset_id}' fold {fold_index} validation candidate {candidate_index}"
            ),
        )?;

        let accumulator = &mut accumulators[candidate_index];
        let train_trades = u64::from(train_candidate.metrics.trades);
        let validation_trades = u64::from(validation_candidate.metrics.trades);
        accumulator.total_train_trades =
            accumulator.total_train_trades.saturating_add(train_trades);
        accumulator.total_validation_trades = accumulator
            .total_validation_trades
            .saturating_add(validation_trades);
        if train_trades < min_train_trades {
            accumulator
                .support_failures
                .push(VcetTemporalTradeSupportFailureV1 {
                    dataset_id: dataset_id.to_string(),
                    fold_index,
                    stage: "train".to_string(),
                    observed_trades: train_trades,
                    minimum_trades: min_train_trades,
                });
        }
        if validation_trades < min_validation_trades {
            accumulator
                .support_failures
                .push(VcetTemporalTradeSupportFailureV1 {
                    dataset_id: dataset_id.to_string(),
                    fold_index,
                    stage: "validation".to_string(),
                    observed_trades: validation_trades,
                    minimum_trades: min_validation_trades,
                });
        }
        accumulator.validation_sharpe_sum = checked_validation_sharpe_sum(
            accumulator.validation_sharpe_sum,
            validation_candidate.metrics.sharpe,
            candidate_index,
        )?;
        accumulator.validation_observation_count = accumulator
            .validation_observation_count
            .checked_add(1)
            .ok_or_else(|| {
                format!(
                    "VCET temporal candidate {candidate_index} validation observation count overflow"
                )
            })?;
    }
    Ok(())
}

fn execute_vcet_temporal_validation_blocking_with_loaded_candles(
    req: ExecuteVcetTemporalValidationRequestV1,
    loaded: Vec<LoadedVcetTemporalDataset>,
) -> Result<ExecuteVcetTemporalValidationResponseV1, String> {
    let canonical_planning = plan_vcet_temporal_validation_blocking(req.planning_request.clone())?;
    if req.planning_evidence != canonical_planning {
        return Err(
            "VCET temporal planning evidence does not exactly match a fresh reconstruction of the sealed planning request"
                .to_string(),
        );
    }
    if req.expected_planning_evidence_sha256 != canonical_planning.evidence_sha256 {
        return Err(format!(
            "VCET temporal planning evidence SHA-256 mismatch (expected {}, actual {})",
            req.expected_planning_evidence_sha256, canonical_planning.evidence_sha256
        ));
    }
    if canonical_planning.evaluation_executed || canonical_planning.evaluation_result_count != 0 {
        return Err(
            "VCET temporal execution requires planning evidence that explicitly reports no prior evaluation"
                .to_string(),
        );
    }
    if req.planning_request.plan.position_boundary_policy
        != VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen
    {
        return Err(
            "VCET temporal execution currently supports only reset_flat_discard_open; force-liquidation and require-closed plans are rejected"
                .to_string(),
        );
    }
    let (requested_backend, device_id) = backend_request_evidence(&req.backend)?;
    let ma_source = canonical_vcet_execution_ma_source(&req.ma_source)?;
    if req.strategy.uses_execution_overlays()
        || req.strategy.stop_loss_pct().is_some()
        || req.strategy.take_profit_pct().is_some()
    {
        return Err(
            "VCET temporal execution does not support execution overlays/stop-loss/take-profit on the strict CPU/GPU window surface"
                .to_string(),
        );
    }

    let loaded = prepare_loaded_datasets(&req, loaded, &canonical_planning)?;
    let candidate_entries = &req
        .planning_request
        .sealed_candidate_manifest
        .payload
        .candidates;
    let candidate_ids = candidate_entries
        .iter()
        .map(|candidate| candidate.candidate_id.clone())
        .collect::<Vec<_>>();
    let candidate_params = candidate_entries
        .iter()
        .map(|candidate| candidate_params_from_manifest(&candidate.identity_inputs, &req.strategy))
        .collect::<Vec<_>>();
    if candidate_params.is_empty() {
        return Err("VCET temporal execution candidate manifest is empty".to_string());
    }
    let fold_count = canonical_planning
        .datasets
        .first()
        .map(|dataset| dataset.layout.folds.len())
        .unwrap_or(0);
    if fold_count == 0 {
        return Err("VCET temporal execution requires at least one planned fold".to_string());
    }
    if canonical_planning
        .datasets
        .iter()
        .any(|dataset| dataset.layout.folds.len() != fold_count)
    {
        return Err("VCET temporal execution requires an aligned fold count".to_string());
    }
    let (pre_final_trial_count, final_test_trial_count, total_trial_count) =
        checked_trial_counts(candidate_params.len(), loaded.len(), fold_count)?;

    let execution_spec = VcetTemporalExecutionSpecEvidenceV1 {
        requested_backend,
        device_id,
        ma_source: ma_source.clone(),
        strategy: req.strategy.clone(),
        position_boundary_policy: req.planning_request.plan.position_boundary_policy,
        selection_policy: req.selection_policy,
        selection_metric: req.selection_policy.label().to_string(),
        trade_support_basis: "metrics.trades_transition_count_per_scored_window".to_string(),
    };
    let minimum_support = &req.planning_request.plan.minimum_support;
    let mut accumulators = (0..candidate_params.len())
        .map(|_| CandidateAccumulator::default())
        .collect::<Vec<_>>();
    let mut dataset_results = Vec::with_capacity(loaded.len());

    for (dataset_index, loaded_dataset) in loaded.iter().enumerate() {
        let planned = &canonical_planning.datasets[dataset_index];
        if planned.dataset_id != loaded_dataset.dataset_id {
            return Err(format!(
                "VCET temporal canonical dataset order mismatch: planned '{}', loaded '{}'",
                planned.dataset_id, loaded_dataset.dataset_id
            ));
        }
        let mut fold_results = Vec::with_capacity(fold_count);
        for fold in &planned.layout.folds {
            let train = evaluate_window(
                &planned.dataset_id,
                &loaded_dataset.candles,
                &req.backend,
                &ma_source,
                &candidate_params,
                &candidate_ids,
                fold.train.scored_bars,
                planned.layout.position_boundary_policy,
            )?;
            let validation = evaluate_window(
                &planned.dataset_id,
                &loaded_dataset.candles,
                &req.backend,
                &ma_source,
                &candidate_params,
                &candidate_ids,
                fold.validation.scored_bars,
                planned.layout.position_boundary_policy,
            )?;
            update_candidate_accumulators(
                &mut accumulators,
                &planned.dataset_id,
                fold.fold_index,
                &train,
                &validation,
                minimum_support.min_train_trades_per_fold,
                minimum_support.min_validation_trades_per_fold,
            )?;
            fold_results.push(VcetTemporalFoldExecutionV1 {
                fold_index: fold.fold_index,
                train,
                validation,
            });
        }
        let dataset_coverage =
            profile_loaded_dataset_coverage(&req, &planned.dataset_id, &loaded_dataset.candles)?;
        dataset_results.push(VcetTemporalDatasetExecutionV1 {
            dataset_id: planned.dataset_id.clone(),
            loaded_data_id: loaded_dataset.loaded_data_id.clone(),
            source_dataset_sha256: planned.source_dataset_sha256.clone(),
            timeline_sha256: planned.timeline_sha256.clone(),
            loaded_candles_snapshot_sha256: loaded_candles_snapshot_sha256(&loaded_dataset.candles),
            dataset_coverage,
            layout: planned.layout.clone(),
            folds: fold_results,
        });
    }

    let mut candidate_evidence = candidate_entries
        .iter()
        .zip(candidate_params.iter())
        .zip(accumulators.iter())
        .enumerate()
        .map(|(candidate_index, ((entry, params), accumulator))| {
            let mean_validation_sharpe =
                candidate_mean_validation_sharpe(accumulator, candidate_index)?;
            Ok(VcetTemporalCandidateSelectionEvidenceV1 {
                candidate_index,
                candidate_id: entry.candidate_id.clone(),
                params: params.clone(),
                validation_observation_count: accumulator.validation_observation_count,
                mean_validation_sharpe,
                total_train_trades: accumulator.total_train_trades,
                total_validation_trades: accumulator.total_validation_trades,
                minimum_trade_support_passed: accumulator.support_failures.is_empty()
                    && mean_validation_sharpe.is_finite(),
                support_failures: accumulator.support_failures.clone(),
                selected_before_final_test: false,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let winner_index = candidate_evidence
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.minimum_trade_support_passed)
        .max_by(|(_, left), (_, right)| {
            left.mean_validation_sharpe
                .total_cmp(&right.mean_validation_sharpe)
                .then_with(|| right.candidate_id.cmp(&left.candidate_id))
        })
        .map(|(index, _)| index)
        .ok_or_else(|| {
            "no VCET temporal candidate satisfied minimum train/validation trade-transition support with finite validation evidence"
                .to_string()
    })?;
    candidate_evidence[winner_index].selected_before_final_test = true;

    validate_pre_final_numeric_contract(&candidate_evidence, &dataset_results)?;
    let pre_final_evidence_sha256 = sha256_serializable(
        &PreFinalEvidenceHashInput {
            hash_contract: PRE_FINAL_EVIDENCE_HASH_CONTRACT,
            planning_evidence_sha256: &canonical_planning.evidence_sha256,
            candidate_manifest_sha256: &canonical_planning.candidate_manifest.manifest_sha256,
            plan_sha256: &canonical_planning.plan_identity.plan_sha256,
            execution_spec: &execution_spec,
            candidates: &candidate_evidence,
            datasets: &dataset_results,
            pre_final_candidate_window_trial_count: pre_final_trial_count,
        },
        "VCET temporal pre-final evidence",
    )?;
    let selected = &candidate_evidence[winner_index];
    validate_finite_score(
        selected.mean_validation_sharpe,
        format_args!("winner lock selection_score"),
    )?;
    let winner_lock_sha256 = sha256_serializable(
        &WinnerLockHashInput {
            hash_contract: WINNER_LOCK_HASH_CONTRACT,
            candidate_index: winner_index,
            candidate_id: &selected.candidate_id,
            params: &selected.params,
            selection_policy: req.selection_policy,
            selection_score: selected.mean_validation_sharpe,
            candidate_manifest_sha256: &canonical_planning.candidate_manifest.manifest_sha256,
            plan_sha256: &canonical_planning.plan_identity.plan_sha256,
            pre_final_evidence_sha256: &pre_final_evidence_sha256,
        },
        "VCET temporal winner lock",
    )?;
    let winner_lock = VcetTemporalWinnerLockV1 {
        candidate_index: winner_index,
        candidate_id: selected.candidate_id.clone(),
        params: selected.params.clone(),
        selection_policy: req.selection_policy,
        selection_score: selected.mean_validation_sharpe,
        candidate_manifest_sha256: canonical_planning
            .candidate_manifest
            .manifest_sha256
            .clone(),
        plan_sha256: canonical_planning.plan_identity.plan_sha256.clone(),
        pre_final_evidence_sha256: pre_final_evidence_sha256.clone(),
        winner_lock_sha256,
    };

    // The final-test seal is opened exactly once, after the immutable winner lock exists. Only
    // that winner is evaluated, and final-test metrics never flow back into ranking.
    let winner_params = vec![candidate_params[winner_index].clone()];
    let winner_ids = vec![candidate_ids[winner_index].clone()];
    let mut final_test_results = Vec::with_capacity(loaded.len());
    for (dataset_index, loaded_dataset) in loaded.iter().enumerate() {
        let planned = &canonical_planning.datasets[dataset_index];
        let result = evaluate_window(
            &planned.dataset_id,
            &loaded_dataset.candles,
            &req.backend,
            &ma_source,
            &winner_params,
            &winner_ids,
            planned.layout.final_test.scored_bars,
            planned.layout.position_boundary_policy,
        )?;
        let only =
            result.candidates.into_iter().next().ok_or_else(|| {
                "VCET final-test winner evaluation returned no metrics".to_string()
            })?;
        let observed_trades = u64::from(only.metrics.trades);
        let support_passed = observed_trades >= minimum_support.min_final_test_trades;
        final_test_results.push(VcetTemporalFinalTestDatasetResultV1 {
            dataset_id: planned.dataset_id.clone(),
            loaded_data_id: loaded_dataset.loaded_data_id.clone(),
            scored_bars: result.scored_bars,
            candidate_index: winner_index,
            candidate_id: winner_ids[0].clone(),
            params: winner_params[0].clone(),
            metrics: only.metrics,
            backend: result.backend,
            observed_trades,
            minimum_trades: minimum_support.min_final_test_trades,
            minimum_trade_support_passed: support_passed,
        });
    }
    let final_test_minimum_trade_support_passed = final_test_results
        .iter()
        .all(|result| result.minimum_trade_support_passed);
    let execution_status = if final_test_minimum_trade_support_passed {
        VcetTemporalExecutionStatusV1::CompletedFinalTestPassed
    } else {
        VcetTemporalExecutionStatusV1::CompletedFinalTestSupportFailed
    };
    let limitations = vec![
        "selection_policy_v1_is_fixed_to_mean_validation_sharpe_with_candidate_id_tie_break"
            .to_string(),
        "trade_support_v1_uses_metrics_trades_transition_counts_and_discards_open_positions_at_each_window_end"
            .to_string(),
        "loaded_dataset_source_hashes_remain_lineage_assertions_while_execution_adds_a_separate_in_memory_candle_snapshot_hash"
            .to_string(),
        format!(
            "single_request_execution_is_bounded_to_{MAX_VCET_TEMPORAL_EXECUTION_DATASETS_PER_REQUEST}_datasets_and_{MAX_VCET_TEMPORAL_CANDIDATE_WINDOW_TRIALS}_candidate_window_trials"
        ),
        "execution_v1_is_a_bounded_cohort_or_shard_primitive_and_does_not_yet_provide_a_global_multi_shard_accumulator_winner_lock_or_final_test_coordinator"
            .to_string(),
        "final_test_access_count_v1_means_one_logical_scoring_stage_open_not_literal_raw_candle_access_or_durable_replay_prevention"
            .to_string(),
        "window_evaluation_recomputes_strategy_signals_per_fold_and_does_not_yet_cache_dataset_level_indicator_tables_across_folds"
            .to_string(),
    ];
    let payload = VcetTemporalExecutionArtifactPayloadV1 {
        schema_version: VCET_TEMPORAL_EXECUTION_ARTIFACT_SCHEMA_VERSION,
        evidence_kind: "vcet_temporal_validation_execution_artifact".to_string(),
        execution_status,
        planning_evidence_sha256: canonical_planning.evidence_sha256.clone(),
        candidate_manifest: canonical_planning.candidate_manifest.clone(),
        plan_identity: canonical_planning.plan_identity.clone(),
        execution_spec,
        dataset_count: loaded.len(),
        candidate_count: candidate_params.len(),
        fold_count_per_dataset: fold_count,
        pre_final_candidate_window_trial_count: pre_final_trial_count,
        final_test_candidate_window_trial_count: final_test_trial_count,
        total_candidate_window_trial_count: total_trial_count,
        final_test_access_count: 1,
        final_test_winner_count: 1,
        final_test_dataset_evaluation_count: loaded.len(),
        selection_used_final_test: false,
        winner_reranked_after_final_test: false,
        pre_final_evidence_sha256,
        candidates: candidate_evidence,
        datasets: dataset_results,
        winner_lock,
        final_test_results,
        final_test_minimum_trade_support_passed,
        limitations,
    };
    validate_execution_artifact_numeric_contract(&payload)?;
    let artifact_sha256 = sha256_serializable(
        &ExecutionArtifactHashInput {
            hash_contract: EXECUTION_ARTIFACT_HASH_CONTRACT,
            payload: &payload,
        },
        "VCET temporal execution artifact",
    )?;
    Ok(ExecuteVcetTemporalValidationResponseV1 {
        payload,
        artifact_sha256,
    })
}

fn resolve_loaded_candles(
    req: &ExecuteVcetTemporalValidationRequestV1,
    state: &AppState,
) -> Result<Vec<LoadedVcetTemporalDataset>, String> {
    let mut seen = BTreeMap::new();
    let mut out = Vec::with_capacity(req.loaded_datasets.len());
    for binding in &req.loaded_datasets {
        let dataset_id = binding.dataset_id.trim().to_string();
        let loaded_data_id = binding.loaded_data_id.trim().to_string();
        if seen.insert(dataset_id.clone(), ()).is_some() {
            return Err(format!(
                "duplicate VCET temporal loaded binding for dataset_id '{dataset_id}'"
            ));
        }
        let candles = state
            .get_candles_for_run(&loaded_data_id)
            .map_err(|error| {
                format!(
                    "failed to resolve loaded candles '{}' for VCET dataset '{}': {error}",
                    loaded_data_id, dataset_id
                )
            })?;
        out.push(LoadedVcetTemporalDataset {
            dataset_id,
            loaded_data_id,
            candles,
        });
    }
    Ok(out)
}

#[tauri::command]
pub async fn execute_vcet_temporal_validation(
    req: ExecuteVcetTemporalValidationRequestV1,
    state: tauri::State<'_, AppState>,
) -> Result<ExecuteVcetTemporalValidationResponseV1, String> {
    let loaded = resolve_loaded_candles(&req, &state)?;
    tauri::async_runtime::spawn_blocking(move || {
        execute_vcet_temporal_validation_blocking_with_loaded_candles(req, loaded)
    })
    .await
    .map_err(|error| format!("VCET temporal execution task failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::super::commands_vcet_temporal::seal_vcet_temporal_candidate_manifest;
    use super::*;
    use ta_app_core::vcet_search::VcetCandidateIdentityInputs;
    use ta_app_core::vcet_temporal::{
        vcet_temporal_plan_identity, VcetTemporalFoldModeV1, VcetTemporalMinimumSupportV1,
        VcetTemporalRequestedBoundsV1, VcetTemporalUtcBoundsV1, VcetTemporalValidationPlanV1,
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
            max_trade_bars: 4,
        }
    }

    fn timestamps(count: usize) -> Vec<i64> {
        (0..count).map(|index| index as i64 * 60_000).collect()
    }

    fn plan(
        timestamps: &[i64],
        min_train: u64,
        min_validation: u64,
        min_final: u64,
        boundary: VcetTemporalPositionBoundaryPolicyV1,
    ) -> VcetTemporalValidationPlanV1 {
        VcetTemporalValidationPlanV1 {
            schema_version: VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION,
            requested_bounds: VcetTemporalRequestedBoundsV1 {
                utc: VcetTemporalUtcBoundsV1 {
                    start_utc_ms_inclusive: timestamps[0],
                    end_utc_ms_inclusive: *timestamps.last().expect("timestamps"),
                },
                session: None,
            },
            fold_mode: VcetTemporalFoldModeV1::Anchored,
            train_bars: 64,
            validation_bars: 24,
            final_test_bars: 32,
            step_bars: 32,
            purge_bars: 2,
            embargo_bars: 2,
            warmup_bars: 64,
            position_boundary_policy: boundary,
            minimum_support: VcetTemporalMinimumSupportV1 {
                min_folds: 2,
                min_train_bars: 64,
                min_validation_bars: 24,
                min_final_test_bars: 32,
                min_train_trades_per_fold: min_train,
                min_validation_trades_per_fold: min_validation,
                min_final_test_trades: min_final,
            },
        }
    }

    fn make_candles(timestamps: &[i64], slope: f64) -> Candles {
        let mut open = Vec::with_capacity(timestamps.len());
        let mut high = Vec::with_capacity(timestamps.len());
        let mut low = Vec::with_capacity(timestamps.len());
        let mut close = Vec::with_capacity(timestamps.len());
        let mut volume = Vec::with_capacity(timestamps.len());
        for index in 0..timestamps.len() {
            let wave = (index as f64 * 0.35).sin() * 4.0;
            let price = 100.0 + index as f64 * slope + wave;
            open.push(price - 0.05);
            high.push(price + 0.5);
            low.push(price - 0.5);
            close.push(price);
            volume.push(1_000.0 + index as f64);
        }
        Candles::new(timestamps.to_vec(), open, high, low, close, volume)
    }

    fn execution_fixture(
        min_train: u64,
        min_validation: u64,
        min_final: u64,
        boundary: VcetTemporalPositionBoundaryPolicyV1,
    ) -> (
        ExecuteVcetTemporalValidationRequestV1,
        Vec<LoadedVcetTemporalDataset>,
    ) {
        let timestamps = timestamps(280);
        let plan = plan(&timestamps, min_train, min_validation, min_final, boundary);
        let manifest = seal_vcet_temporal_candidate_manifest(
            lineage_hash('a'),
            lineage_hash('b'),
            Some(format!("vcet_search_v1_{}", "c".repeat(64))),
            vec![identity(12, 48), identity(8, 32)],
        )
        .expect("sealed candidates");
        let planning_request = PlanVcetTemporalValidationRequestV1 {
            expected_candidate_manifest_sha256: manifest.manifest_sha256.clone(),
            sealed_candidate_manifest: manifest,
            expected_plan_identity: vcet_temporal_plan_identity(&plan).expect("plan identity"),
            plan,
            datasets: vec![
                crate::commands::VcetTemporalDatasetTimelineInputV1 {
                    dataset_id: "AAPL".to_string(),
                    source_dataset_sha256: lineage_hash('d'),
                    timestamps_utc_ms: timestamps.clone(),
                    session_calendar_id: None,
                    session_open_utc_ms: None,
                },
                crate::commands::VcetTemporalDatasetTimelineInputV1 {
                    dataset_id: "MSFT".to_string(),
                    source_dataset_sha256: lineage_hash('e'),
                    timestamps_utc_ms: timestamps.clone(),
                    session_calendar_id: None,
                    session_open_utc_ms: None,
                },
            ],
        };
        let planning_evidence = plan_vcet_temporal_validation_blocking(planning_request.clone())
            .expect("planning evidence");
        let request = ExecuteVcetTemporalValidationRequestV1 {
            expected_planning_evidence_sha256: planning_evidence.evidence_sha256.clone(),
            planning_request,
            planning_evidence,
            loaded_datasets: vec![
                VcetTemporalLoadedDatasetBindingV1 {
                    dataset_id: "MSFT".to_string(),
                    loaded_data_id: "loaded-msft".to_string(),
                },
                VcetTemporalLoadedDatasetBindingV1 {
                    dataset_id: "AAPL".to_string(),
                    loaded_data_id: "loaded-aapl".to_string(),
                },
            ],
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            strategy: StrategyConfig::default(),
            selection_policy: VcetTemporalSelectionPolicyV1::MeanValidationSharpe,
        };
        let loaded = vec![
            LoadedVcetTemporalDataset {
                dataset_id: "MSFT".to_string(),
                loaded_data_id: "loaded-msft".to_string(),
                candles: Arc::new(make_candles(&timestamps, 0.035)),
            },
            LoadedVcetTemporalDataset {
                dataset_id: "AAPL".to_string(),
                loaded_data_id: "loaded-aapl".to_string(),
                candles: Arc::new(make_candles(&timestamps, 0.025)),
            },
        ];
        (request, loaded)
    }

    #[test]
    fn temporal_execution_locks_one_winner_before_one_final_test_access() {
        let (request, loaded) = execution_fixture(
            1,
            1,
            1,
            VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen,
        );
        let first = execute_vcet_temporal_validation_blocking_with_loaded_candles(
            request.clone(),
            loaded
                .iter()
                .map(|dataset| LoadedVcetTemporalDataset {
                    dataset_id: dataset.dataset_id.clone(),
                    loaded_data_id: dataset.loaded_data_id.clone(),
                    candles: dataset.candles.clone(),
                })
                .collect(),
        )
        .expect("first execution");
        let second = execute_vcet_temporal_validation_blocking_with_loaded_candles(request, loaded)
            .expect("second execution");

        assert_eq!(first.artifact_sha256, second.artifact_sha256);
        assert_eq!(first.payload.dataset_count, 2);
        assert_eq!(first.payload.candidate_count, 2);
        assert_eq!(first.payload.final_test_access_count, 1);
        assert_eq!(first.payload.final_test_winner_count, 1);
        assert_eq!(first.payload.final_test_dataset_evaluation_count, 2);
        assert!(!first.payload.selection_used_final_test);
        assert!(!first.payload.winner_reranked_after_final_test);
        assert_eq!(
            first.payload.pre_final_candidate_window_trial_count,
            first.payload.candidate_count
                * first.payload.dataset_count
                * first.payload.fold_count_per_dataset
                * 2
        );
        assert_eq!(
            first
                .payload
                .candidates
                .iter()
                .filter(|candidate| candidate.selected_before_final_test)
                .count(),
            1
        );
        assert!(first.payload.final_test_results.iter().all(|result| {
            result.candidate_id == first.payload.winner_lock.candidate_id
                && result.candidate_index == first.payload.winner_lock.candidate_index
        }));
        assert!(first.payload.datasets.iter().all(|dataset| dataset
            .loaded_candles_snapshot_sha256
            .starts_with("sha256:")));
        assert!(first.payload.datasets.iter().all(|dataset| {
            dataset.dataset_coverage.ordering.strictly_increasing
                && dataset.dataset_coverage.timestamp_count == 280
        }));
    }

    #[test]
    fn temporal_execution_rejects_tampered_planning_evidence_and_unsupported_boundary() {
        let (mut tampered, loaded) = execution_fixture(
            1,
            1,
            1,
            VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen,
        );
        tampered.planning_evidence.evidence_sha256 = lineage_hash('f');
        let error = execute_vcet_temporal_validation_blocking_with_loaded_candles(tampered, loaded)
            .unwrap_err();
        assert!(error.contains("does not exactly match"));

        let (unsupported, loaded) = execution_fixture(
            1,
            1,
            1,
            VcetTemporalPositionBoundaryPolicyV1::ResetFlatForceLiquidate,
        );
        let error =
            execute_vcet_temporal_validation_blocking_with_loaded_candles(unsupported, loaded)
                .unwrap_err();
        assert!(error.contains("only reset_flat_discard_open"));

        let (mut invalid_source, loaded) = execution_fixture(
            1,
            1,
            1,
            VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen,
        );
        invalid_source.ma_source = "silently-defaulted-source".to_string();
        let error =
            execute_vcet_temporal_validation_blocking_with_loaded_candles(invalid_source, loaded)
                .unwrap_err();
        assert!(error.contains("ma_source"));
    }

    #[test]
    fn temporal_execution_enforces_pre_final_and_final_trade_support_without_reranking() {
        let (unsupported, loaded) = execution_fixture(
            10_000,
            10_000,
            1,
            VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen,
        );
        let error =
            execute_vcet_temporal_validation_blocking_with_loaded_candles(unsupported, loaded)
                .unwrap_err();
        assert!(error.contains("no VCET temporal candidate satisfied minimum"));

        let (final_fail, loaded) = execution_fixture(
            1,
            1,
            10_000,
            VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen,
        );
        let result =
            execute_vcet_temporal_validation_blocking_with_loaded_candles(final_fail, loaded)
                .expect("final support failure remains an evidence-bearing outcome");
        assert!(matches!(
            result.payload.execution_status,
            VcetTemporalExecutionStatusV1::CompletedFinalTestSupportFailed
        ));
        assert!(!result.payload.final_test_minimum_trade_support_passed);
        assert_eq!(result.payload.final_test_access_count, 1);
        assert!(!result.payload.selection_used_final_test);
        assert!(!result.payload.winner_reranked_after_final_test);
    }

    #[test]
    fn metrics_wire_contract_rejects_every_non_finite_float_field() {
        let mutations: [(&str, fn(&mut Metrics, f64)); 20] = [
            ("pnl", |metrics, value| metrics.pnl = value),
            ("sharpe", |metrics, value| metrics.sharpe = value),
            ("sortino", |metrics, value| metrics.sortino = value),
            ("downside_deviation", |metrics, value| {
                metrics.downside_deviation = value
            }),
            ("downside_frequency", |metrics, value| {
                metrics.downside_frequency = value
            }),
            ("downside_severity", |metrics, value| {
                metrics.downside_severity = value
            }),
            ("calmar", |metrics, value| metrics.calmar = value),
            ("omega", |metrics, value| metrics.omega = value),
            ("kappa_tau", |metrics, value| metrics.kappa_tau = value),
            ("volatility", |metrics, value| metrics.volatility = value),
            ("ulcer_index", |metrics, value| metrics.ulcer_index = value),
            ("martin_ratio", |metrics, value| {
                metrics.martin_ratio = value
            }),
            ("max_dd", |metrics, value| metrics.max_dd = value),
            ("exposure", |metrics, value| metrics.exposure = value),
            ("net_exposure", |metrics, value| {
                metrics.net_exposure = value
            }),
            ("long_mean_return", |metrics, value| {
                metrics.long_mean_return = value
            }),
            ("short_mean_return", |metrics, value| {
                metrics.short_mean_return = value
            }),
            ("gross_profit", |metrics, value| {
                metrics.gross_profit = value
            }),
            ("gross_loss", |metrics, value| metrics.gross_loss = value),
            ("underwater_fraction", |metrics, value| {
                metrics.underwater_fraction = value
            }),
        ];
        let non_finite_values = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

        validate_finite_metrics(&Metrics::default(), format_args!("test metrics"))
            .expect("default metrics are finite");
        for (index, (field, mutate)) in mutations.into_iter().enumerate() {
            let mut metrics = Metrics::default();
            mutate(
                &mut metrics,
                non_finite_values[index % non_finite_values.len()],
            );

            assert_eq!(first_non_finite_metric_field(&metrics), Some(field));
            let error =
                validate_finite_metrics(&metrics, format_args!("test metrics")).unwrap_err();
            assert!(error.contains(&format!("metric '{field}'")));
            assert!(error.contains("artifact publication was rejected"));

            // serde_json encodes a non-finite f64 as null. The execution boundary must reject
            // this value before it can participate in any canonical artifact hash or response.
            let serialized = serde_json::to_value(&metrics).expect("metrics JSON");
            assert!(serialized[field].is_null());
        }
    }

    #[test]
    fn validation_score_contract_rejects_empty_and_overflowed_aggregates() {
        let empty = CandidateAccumulator::default();
        let error = candidate_mean_validation_sharpe(&empty, 4).unwrap_err();
        assert!(error.contains("candidate 4 has no validation Sharpe observations"));
        assert!(error.contains("artifact publication was rejected"));

        let error = checked_validation_sharpe_sum(f64::MAX, f64::MAX, 7).unwrap_err();
        assert!(error.contains("candidate 7 validation Sharpe accumulator"));
        assert!(error.contains("artifact publication was rejected"));

        let finite = CandidateAccumulator {
            validation_sharpe_sum: 3.0,
            validation_observation_count: 2,
            ..CandidateAccumulator::default()
        };
        assert_eq!(
            candidate_mean_validation_sharpe(&finite, 0).expect("finite mean"),
            1.5
        );
    }

    #[test]
    fn completed_payload_is_revalidated_before_artifact_hashing() {
        let (request, loaded) = execution_fixture(
            1,
            1,
            1,
            VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen,
        );
        let response =
            execute_vcet_temporal_validation_blocking_with_loaded_candles(request, loaded)
                .expect("finite execution payload");
        validate_execution_artifact_numeric_contract(&response.payload)
            .expect("completed payload numeric contract");

        let mut non_finite_metric = response.payload.clone();
        non_finite_metric.final_test_results[0].metrics.pnl = f64::NEG_INFINITY;
        let error = validate_execution_artifact_numeric_contract(&non_finite_metric).unwrap_err();
        assert!(error.contains("final-test winner metric 'pnl'"));
        let serialized = serde_json::to_value(&ExecutionArtifactHashInput {
            hash_contract: EXECUTION_ARTIFACT_HASH_CONTRACT,
            payload: &non_finite_metric,
        })
        .expect("JSON demonstrates the unsafe serializer fallback");
        assert!(serialized["payload"]["final_test_results"][0]["metrics"]["pnl"].is_null());

        let mut non_finite_candidate_score = response.payload.clone();
        non_finite_candidate_score.candidates[0].mean_validation_sharpe = f64::INFINITY;
        let error =
            validate_execution_artifact_numeric_contract(&non_finite_candidate_score).unwrap_err();
        assert!(error.contains("candidate 0 mean_validation_sharpe"));

        let mut non_finite_winner_score = response.payload;
        non_finite_winner_score.winner_lock.selection_score = f64::NAN;
        let error =
            validate_execution_artifact_numeric_contract(&non_finite_winner_score).unwrap_err();
        assert!(error.contains("winner lock selection_score"));
    }
}
