use crate::cscv_pbo::{
    compute_cscv_pbo_from_slices, CscvCandidatePboStats, CscvPboConfig, CscvPboResult,
    SliceReturnStats,
};
use crate::iterative_sim::{
    simulate_iterative_from_signals, IterativeMetricsDelta, IterativeSimConfig, OhlcSlice,
    ValidationEngine,
};
use crate::progress::ProgressSink;
use crate::stats_common::{worst_max, worst_min, worst_min_u32};
use crate::validation_shared::{
    CrossTickerValidationSummary, EvidenceGateRole, EvidenceMetricClass, EvidencePolicyMetric,
    EvidencePolicySummary, MonteCarloAdvisory, MonteCarloDistributionBins, MonteCarloMode,
    MonteCarloModeStats, MonteCarloThresholdConfig, MonteCarloValidationConfig,
    MonteCarloValidationStats, PerturbConfig, PerturbStats, ReoptWalkForwardConfigOptional,
    ReoptWalkForwardStats, ReoptWalkForwardWindowStats, SpaConfigOptional, SpaValidationStats,
    TradeBootstrapConfig, TradeBootstrapStats, ValidationArtifactContext,
    ValidationEvidenceAvailability, ValidationEvidenceCandidateUniverseV1,
    ValidationEvidenceDatasetV1, ValidationEvidenceMetricV1, ValidationEvidenceModuleV1,
    ValidationEvidencePackStatus, ValidationEvidencePackV1, ValidationEvidenceRouteV1,
    ValidationEvidenceVerdictV1, ValidationSemanticsProvenance, WalkForwardConfigOptional,
    WalkForwardStats, VALIDATION_EVIDENCE_PACK_SCHEMA_VERSION, VALIDATION_EVIDENCE_PACK_VERSION,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use ta_strategies::double_ma::{Metrics, StrategyConfig, Trade};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ValidationHardGateConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_psr: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_dsr: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_cscv_oos_rank_frac: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_cscv_oos_metric: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_pbo: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationCandidateVerdict {
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warning_flags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fail_reasons: Vec<String>,
}

impl Default for ValidationCandidateVerdict {
    fn default() -> Self {
        Self {
            passed: true,
            warning_flags: Vec::new(),
            fail_reasons: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRecommendation {
    NoCandidateSurvived,
    NoCandidateApproved,
    CandidatesSurvivedButSearchNotStatisticallySuperior,
    EvidenceTooShortMinTrackRecord,
    ApprovedWithWarnings,
    SingleCandidateSupported,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationRunVerdict {
    pub passed: bool,
    pub approved: bool,
    pub candidates_tested: usize,
    pub candidates_survived_hard_gates: usize,
    pub approved_candidate_count: usize,
    pub final_recommendation: ValidationRecommendation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warning_flags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fail_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_approved_rank: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_candidate_track_record_bars: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_candidate_min_track_record_length_bars: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_candidate_psr: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_candidate_dsr: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pbo: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_pbo: Option<f64>,
}

fn evidence_role_for_gate(enabled: bool) -> EvidenceGateRole {
    if enabled {
        EvidenceGateRole::HardGate
    } else {
        EvidenceGateRole::Advisory
    }
}

fn evidence_policy_metric(
    metric: &str,
    metric_class: EvidenceMetricClass,
    gate_role: EvidenceGateRole,
    sample_unit: &str,
    interpretation: &str,
    caveats: &[&str],
) -> EvidencePolicyMetric {
    EvidencePolicyMetric {
        metric: metric.to_string(),
        metric_class,
        gate_role,
        sample_unit: sample_unit.to_string(),
        interpretation: interpretation.to_string(),
        caveats: caveats.iter().map(|entry| (*entry).to_string()).collect(),
    }
}

pub fn build_general_validation_evidence_policy_summary(
    hard_gates: &ValidationHardGateConfig,
    cscv_requested: bool,
    cscv_pbo: Option<&CscvPboResult>,
) -> EvidencePolicySummary {
    let cscv_role = evidence_role_for_gate(
        hard_gates.min_cscv_oos_rank_frac.is_some()
            || hard_gates.min_cscv_oos_metric.is_some()
            || hard_gates.max_pbo.is_some(),
    );
    let cscv_state = if !cscv_requested {
        "not_requested"
    } else if let Some(result) = cscv_pbo {
        if result.skipped_reason.is_some() || result.combinations == 0 {
            "requested_but_skipped"
        } else {
            "evaluated"
        }
    } else {
        "requested_but_unavailable"
    };

    EvidencePolicySummary {
        version: 1,
        workflow: "general_validation".to_string(),
        summary_role: "interpretation_metadata_only".to_string(),
        metrics: vec![
            evidence_policy_metric(
                "psr",
                EvidenceMetricClass::ConfidenceBounded,
                evidence_role_for_gate(hard_gates.min_psr.is_some()),
                "raw_bars",
                "Probabilistic Sharpe Ratio used as candidate evidence and optional hard gate.",
                &["General validation PSR currently uses raw bar counts unless a later effective-count audit supplies a narrower unit."],
            ),
            evidence_policy_metric(
                "dsr",
                EvidenceMetricClass::ConfidenceBounded,
                evidence_role_for_gate(hard_gates.min_dsr.is_some()),
                "raw_bars",
                "Deflated Sharpe Ratio used as candidate evidence and optional hard gate.",
                &["General validation DSR currently uses raw bar counts unless a later effective-count audit supplies a narrower unit."],
            ),
            evidence_policy_metric(
                "min_track_record_length",
                EvidenceMetricClass::ConfidenceBounded,
                EvidenceGateRole::RunRecommendation,
                "raw_bars",
                "Track-record length support used by the retained run recommendation layer.",
                &["This recommendation layer can make approved diverge from passed without changing hard-gate survival."],
            ),
            evidence_policy_metric(
                "cscv_pbo",
                EvidenceMetricClass::Approximate,
                cscv_role,
                "candidate_set_slices",
                &format!("CSCV/PBO set-level overfit evidence state: {cscv_state}."),
                &["CSCV/PBO is set-level overfit evidence, not a candidate-specific proof."],
            ),
            evidence_policy_metric(
                "robust_score",
                EvidenceMetricClass::RankingOnly,
                EvidenceGateRole::RunRecommendation,
                "candidate_metric_surface",
                "Ranking score used to order retained validation candidates.",
                &["Robust score is not a hard-gate proof and does not change passed semantics."],
            ),
            evidence_policy_metric(
                "passed_approved_contract",
                EvidenceMetricClass::MetadataOnly,
                EvidenceGateRole::Provenance,
                "validation_run",
                "passed is hard-gate survival; approved also requires retained final verdict support.",
                &[],
            ),
        ],
        caveats: vec![
            "Evidence policy summary is additive metadata and does not alter candidate math, order, passed, or approved semantics.".to_string(),
            "General validation probability metrics are labeled raw-bar based until an explicit effective-count audit is implemented.".to_string(),
            "CSCV/PBO is set-level overfit evidence rather than a candidate-specific proof.".to_string(),
        ],
    }
}

pub fn build_ml_signal_evidence_policy_summary(
    hard_gates: &ValidationHardGateConfig,
    dependence_summary_present: bool,
    score_scope: Option<&str>,
) -> EvidencePolicySummary {
    let sample_unit = if dependence_summary_present {
        "effective_score_observations_lower_bound"
    } else {
        "score_observations"
    };
    let score_scope = score_scope
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .unwrap_or("unknown");

    EvidencePolicySummary {
        version: 1,
        workflow: "ml_signal_validation".to_string(),
        summary_role: "interpretation_metadata_only".to_string(),
        metrics: vec![
            evidence_policy_metric(
                "psr",
                EvidenceMetricClass::ConfidenceBounded,
                evidence_role_for_gate(hard_gates.min_psr.is_some()),
                sample_unit,
                "Probabilistic Sharpe Ratio over score-cache validation outcomes.",
                &["ML Signal PSR uses dependence-adjusted effective score observations when dependence metadata is present."],
            ),
            evidence_policy_metric(
                "dsr",
                EvidenceMetricClass::ConfidenceBounded,
                evidence_role_for_gate(hard_gates.min_dsr.is_some()),
                sample_unit,
                "Deflated Sharpe Ratio over score-cache validation outcomes.",
                &["ML Signal DSR uses dependence-adjusted effective score observations when dependence metadata is present."],
            ),
            evidence_policy_metric(
                "score_scope",
                EvidenceMetricClass::MetadataOnly,
                EvidenceGateRole::Provenance,
                "score_cache",
                &format!("Score cache scope recorded as {score_scope}."),
                &["Non-walk-forward score scopes remain advisory through existing warning and deployability metadata."],
            ),
            evidence_policy_metric(
                "deployability",
                EvidenceMetricClass::MetadataOnly,
                EvidenceGateRole::Provenance,
                "validation_run",
                "Deployability state summarizes existing review gates for the ML Signal artifact.",
                &["Deployability metadata is not deployment proof and does not replace validation evidence review."],
            ),
            evidence_policy_metric(
                "robust_score",
                EvidenceMetricClass::RankingOnly,
                EvidenceGateRole::RunRecommendation,
                "candidate_metric_surface",
                "Ranking score used to order retained ML Signal validation candidates.",
                &["Robust score is not a hard-gate proof and does not change passed semantics."],
            ),
            evidence_policy_metric(
                "passed_approved_contract",
                EvidenceMetricClass::MetadataOnly,
                EvidenceGateRole::Provenance,
                "validation_run",
                "passed is hard-gate survival; approved also requires retained final verdict support.",
                &[],
            ),
        ],
        caveats: vec![
            "Evidence policy summary is additive metadata and does not alter candidate math, order, passed, approved, or deployability semantics.".to_string(),
            "ML Signal probability metrics are labeled effective-score-observation based when dependence metadata is present.".to_string(),
            "Deployability metadata is a review summary, not proof that a model is production safe.".to_string(),
        ],
    }
}

pub trait ValidationPostCandidate {
    fn in_sample_metrics(&self) -> &Metrics;
    fn iterative_in_sample_metrics(&self) -> Option<&Metrics>;
    fn walk_forward_stats(&self) -> Option<&WalkForwardStats>;
    fn cscv_oos_metric_p50(&self) -> Option<f64>;
    fn cscv_oos_rank_frac_p50(&self) -> Option<f64>;
    fn cscv_selected_frac(&self) -> Option<f64>;

    fn psr(&self) -> Option<f64>;
    fn dsr(&self) -> Option<f64>;
    fn set_psr(&mut self, v: Option<f64>);
    fn set_dsr(&mut self, v: Option<f64>);
    fn track_record_bars(&self) -> usize;
    fn min_track_record_length_bars(&self) -> Option<usize>;
    fn set_track_record_bars(&mut self, v: usize);
    fn set_min_track_record_length_bars(&mut self, v: Option<usize>);

    fn robust_score(&self) -> f64;
    fn set_robust_score(&mut self, v: f64);
    fn set_robust_pareto(&mut self, v: bool);
    fn set_cscv_pbo(&mut self, v: Option<CscvCandidatePboStats>);
    fn monte_carlo_stats(&self) -> Option<&MonteCarloValidationStats> {
        None
    }
    fn validation_verdict(&self) -> &ValidationCandidateVerdict;
    fn set_validation_verdict(&mut self, v: ValidationCandidateVerdict);
}

const VALIDATION_STAGE_SEQUENCE: [&str; 5] = [
    "local_path_evidence",
    "probability_metrics",
    "cscv_pbo_layer",
    "cross_ticker_aggregation",
    "run_verdict_rebuild",
];

pub fn downside_vol_ratio_from_metrics(metrics: &Metrics) -> Option<f64> {
    if metrics.downside_deviation.is_finite()
        && metrics.volatility.is_finite()
        && metrics.volatility > 1.0e-12
    {
        Some((metrics.downside_deviation / metrics.volatility).clamp(0.0, 10.0))
    } else {
        None
    }
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn robust_score_from_components(
    sharpe_like: f64,
    max_dd_like: f64,
    ulcer_like: f64,
    martin_like: f64,
    downside_vol_ratio_like: Option<f64>,
) -> f64 {
    let base = sharpe_like - 2.0 * max_dd_like;
    let martin_bonus = if martin_like.is_finite() {
        0.05 * martin_like.clamp(-10.0, 10.0)
    } else {
        0.0
    };
    let ulcer_penalty = if ulcer_like.is_finite() {
        0.5 * ulcer_like.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let downside_penalty = downside_vol_ratio_like
        .filter(|ratio| ratio.is_finite())
        .map(|ratio| 0.25 * ratio.clamp(0.0, 2.0))
        .unwrap_or(0.0);

    base + martin_bonus - ulcer_penalty - downside_penalty
}

pub fn robust_score_from_metrics(in_sample: &Metrics, wf: Option<&WalkForwardStats>) -> f64 {
    let sharpe_like = wf
        .map(|stats| finite_or(stats.sharpe_median, in_sample.sharpe))
        .unwrap_or(in_sample.sharpe);
    let max_dd_like = wf
        .map(|stats| finite_or(stats.max_dd_worst, in_sample.max_dd))
        .unwrap_or(in_sample.max_dd);
    let ulcer_like = wf
        .map(|stats| finite_or(stats.ulcer_index_worst, in_sample.ulcer_index))
        .unwrap_or(in_sample.ulcer_index);
    let martin_like = wf
        .map(|stats| finite_or(stats.martin_ratio_median, in_sample.martin_ratio))
        .unwrap_or(in_sample.martin_ratio);
    let downside_vol_ratio_like = wf
        .and_then(|stats| {
            if stats.downside_vol_ratio_median.is_finite() {
                Some(stats.downside_vol_ratio_median)
            } else {
                None
            }
        })
        .or_else(|| downside_vol_ratio_from_metrics(in_sample));

    robust_score_from_components(
        sharpe_like,
        max_dd_like,
        ulcer_like,
        martin_like,
        downside_vol_ratio_like,
    )
}

fn risk_warning_flags_from_metrics(
    in_sample: &Metrics,
    wf: Option<&WalkForwardStats>,
) -> Vec<String> {
    const HIGH_ULCER_INDEX_WARNING: f64 = 0.10;
    const WEAK_MARTIN_RATIO_WARNING: f64 = 0.25;
    const HIGH_DOWNSIDE_VOL_RATIO_WARNING: f64 = 0.85;

    let ulcer = wf
        .map(|stats| finite_or(stats.ulcer_index_worst, in_sample.ulcer_index))
        .unwrap_or(in_sample.ulcer_index);
    let martin = wf
        .map(|stats| finite_or(stats.martin_ratio_median, in_sample.martin_ratio))
        .unwrap_or(in_sample.martin_ratio);
    let downside_ratio = wf
        .and_then(|stats| {
            if stats.downside_vol_ratio_median.is_finite() {
                Some(stats.downside_vol_ratio_median)
            } else {
                None
            }
        })
        .or_else(|| downside_vol_ratio_from_metrics(in_sample));

    let mut flags = Vec::new();
    if ulcer.is_finite() && ulcer > HIGH_ULCER_INDEX_WARNING {
        flags.push(format!("ulcer_index_high:{ulcer:.3}"));
    }
    if martin.is_finite() && martin < WEAK_MARTIN_RATIO_WARNING {
        flags.push(format!("martin_ratio_weak:{martin:.3}"));
    }
    if let Some(ratio) = downside_ratio.filter(|ratio| ratio.is_finite()) {
        if ratio > HIGH_DOWNSIDE_VOL_RATIO_WARNING {
            flags.push(format!("downside_vol_ratio_high:{ratio:.3}"));
        }
    }
    if let Some(stats) = wf.filter(|stats| {
        stats.windows > 0
            && stats
                .effective_windows_upper_bound
                .map(|effective| effective < stats.windows)
                .unwrap_or(false)
    }) {
        flags.push(format!(
            "walk_forward_overlap_adjusted:{}/{}",
            stats.effective_windows_upper_bound.unwrap_or(stats.windows),
            stats.windows
        ));
    }
    flags
}

pub fn fold_cscv_metric(base: f64, oos_metric_p50: Option<f64>) -> f64 {
    match oos_metric_p50 {
        Some(oos) if oos.is_finite() => base.min(oos),
        _ => base,
    }
}

pub fn sanitize_hard_gates(
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
    min_cscv_oos_rank_frac: Option<f64>,
    min_cscv_oos_metric: Option<f64>,
    max_pbo: Option<f64>,
) -> ValidationHardGateConfig {
    ValidationHardGateConfig {
        min_psr: crate::validation_stats::sanitize_probability_gate(min_psr),
        min_dsr: crate::validation_stats::sanitize_probability_gate(min_dsr),
        min_cscv_oos_rank_frac: crate::validation_stats::sanitize_probability_gate(
            min_cscv_oos_rank_frac,
        ),
        min_cscv_oos_metric: min_cscv_oos_metric.and_then(|v| {
            if v.is_finite() {
                Some(v)
            } else {
                None
            }
        }),
        max_pbo: crate::validation_stats::sanitize_probability_gate(max_pbo),
    }
}

pub fn compute_probability_metrics<C: ValidationPostCandidate>(
    out: &mut Vec<C>,
    sample_obs: &[usize],
) {
    compute_probability_metrics_with_effective_counts(out, sample_obs, None, None);
}

pub fn compute_probability_metrics_with_effective_counts<C: ValidationPostCandidate>(
    out: &mut Vec<C>,
    sample_obs: &[usize],
    effective_obs: Option<&[usize]>,
    effective_trials: Option<usize>,
) {
    compute_probability_metrics_with_research_context(
        out,
        sample_obs,
        effective_obs,
        effective_trials,
        None,
        None,
    );
}

pub fn compute_probability_metrics_with_research_context<C: ValidationPostCandidate>(
    out: &mut Vec<C>,
    sample_obs: &[usize],
    effective_obs: Option<&[usize]>,
    effective_trials: Option<usize>,
    optimization_trials: Option<usize>,
    return_moments: Option<&[Option<crate::sharpe_stats::ReturnDistributionMoments>]>,
) {
    compute_probability_metrics_with_search_distribution(
        out,
        sample_obs,
        effective_obs,
        effective_trials,
        optimization_trials,
        None,
        None,
        return_moments,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn compute_probability_metrics_with_search_distribution<C: ValidationPostCandidate>(
    out: &mut Vec<C>,
    sample_obs: &[usize],
    effective_obs: Option<&[usize]>,
    effective_trials: Option<usize>,
    optimization_trials: Option<usize>,
    search_distribution: Option<&ta_optimizer::SearchDistributionSummary>,
    expected_sharpe_metric_id: Option<&str>,
    return_moments: Option<&[Option<crate::sharpe_stats::ReturnDistributionMoments>]>,
) {
    let sharpe_peers: Vec<f64> = out
        .iter()
        .map(|c| {
            c.iterative_in_sample_metrics()
                .unwrap_or_else(|| c.in_sample_metrics())
                .sharpe
        })
        .collect();

    let retained_trials = sharpe_peers.iter().filter(|x| x.is_finite()).count().max(1);
    let comparable_distribution = expected_sharpe_metric_id.and_then(|expected_id| {
        search_distribution.filter(|summary| summary.is_comparable_with(expected_id))
    });
    let (peer_mean, peer_std, raw_trials) = if let Some(summary) = comparable_distribution {
        (
            summary
                .finite_sharpe_mean
                .expect("comparable distribution has a finite mean"),
            summary
                .finite_sharpe_sample_std
                .expect("comparable distribution has a finite sample deviation"),
            summary.finite_sharpe_count.max(1),
        )
    } else {
        let (peer_mean, peer_std) = crate::sharpe_stats::mean_std(&sharpe_peers);
        let raw_trials = crate::sharpe_stats::normalize_optimization_trial_count(
            optimization_trials,
            retained_trials,
        )
        .unwrap_or(retained_trials);
        (peer_mean, peer_std, raw_trials)
    };

    for (i, cand) in out.iter_mut().enumerate() {
        let m = cand
            .iterative_in_sample_metrics()
            .unwrap_or_else(|| cand.in_sample_metrics());
        let raw_obs = sample_obs.get(i).copied().unwrap_or(2);
        let effective_obs_i = effective_obs
            .and_then(|counts| counts.get(i).copied())
            .or(Some(raw_obs.max(2)));
        let effective_obs_i = crate::sharpe_stats::effective_sample_count(raw_obs, effective_obs_i);
        let moments = return_moments
            .and_then(|all_moments| all_moments.get(i).copied())
            .flatten()
            .unwrap_or(crate::sharpe_stats::ReturnDistributionMoments {
                observations: raw_obs.max(2),
                skewness: 0.0,
                kurtosis: 3.0,
            });
        let (psr, dsr) = crate::validation_stats::compute_psr_dsr_with_effective_counts_and_moments(
            m.sharpe,
            raw_obs,
            Some(effective_obs_i),
            raw_trials,
            effective_trials,
            peer_mean,
            peer_std,
            moments.skewness,
            moments.kurtosis,
        );
        let mintrl = crate::sharpe_stats::minimum_track_record_length(
            m.sharpe,
            0.0,
            moments.skewness,
            moments.kurtosis,
            0.95,
        )
        .map(|required_eff_obs| {
            crate::sharpe_stats::inflate_effective_requirement_to_raw_bars(
                required_eff_obs,
                raw_obs,
                effective_obs_i,
            )
        });
        cand.set_psr(psr);
        cand.set_dsr(dsr);
        cand.set_track_record_bars(raw_obs);
        cand.set_min_track_record_length_bars(mintrl);
    }
}

pub fn mark_robust_pareto<C: ValidationPostCandidate>(out: &mut [C]) {
    #[derive(Clone, Copy)]
    struct RobustParetoPoint {
        score: f64,
        ulcer: f64,
        dd: f64,
    }

    let points: Vec<RobustParetoPoint> = out
        .iter()
        .map(|cand| {
            let base = cand
                .iterative_in_sample_metrics()
                .unwrap_or_else(|| cand.in_sample_metrics());
            let walk_forward = cand.walk_forward_stats();
            RobustParetoPoint {
                score: robust_score_from_metrics(base, walk_forward),
                ulcer: walk_forward
                    .map(|stats| finite_or(stats.ulcer_index_worst, base.ulcer_index))
                    .unwrap_or(base.ulcer_index),
                dd: walk_forward
                    .map(|stats| finite_or(stats.max_dd_worst, base.max_dd))
                    .unwrap_or(base.max_dd),
            }
        })
        .collect();

    for i in 0..out.len() {
        let point_i = points[i];
        let mut dominated = false;
        for j in 0..out.len() {
            if i == j {
                continue;
            }
            let point_j = points[j];
            if point_j.score >= point_i.score
                && point_j.ulcer <= point_i.ulcer
                && point_j.dd <= point_i.dd
                && (point_j.score > point_i.score
                    || point_j.ulcer < point_i.ulcer
                    || point_j.dd < point_i.dd)
            {
                dominated = true;
                break;
            }
        }
        out[i].set_robust_pareto(!dominated);
    }
}

pub fn fold_cscv_into_robust_and_sort<C: ValidationPostCandidate>(out: &mut [C]) {
    for cand in out.iter_mut() {
        let base = cand.robust_score();
        let folded = fold_cscv_metric(base, cand.cscv_oos_metric_p50());
        cand.set_robust_score(folded);
    }
    out.sort_by(|a, b| b.robust_score().total_cmp(&a.robust_score()));
}

pub fn apply_candidate_hard_gates<C: ValidationPostCandidate>(
    out: &mut [C],
    gates: &ValidationHardGateConfig,
    cscv_pbo: Option<&CscvPboResult>,
) {
    let cscv_available = cscv_pbo
        .map(|res| res.combinations > 0 && res.skipped_reason.is_none())
        .unwrap_or(false);

    for cand in out.iter_mut() {
        let mut verdict = ValidationCandidateVerdict::default();
        let base_metrics = cand
            .iterative_in_sample_metrics()
            .unwrap_or_else(|| cand.in_sample_metrics());
        let walk_forward = cand.walk_forward_stats();

        if let Some(thr) = gates.min_psr {
            let psr = cand.psr().unwrap_or(0.0);
            if !psr.is_finite() || psr < thr {
                verdict.fail_reasons.push(format!("psr_below_min:{thr:.2}"));
            }
        }

        if let Some(thr) = gates.min_dsr {
            let dsr = cand.dsr().unwrap_or(0.0);
            if !dsr.is_finite() || dsr < thr {
                verdict.fail_reasons.push(format!("dsr_below_min:{thr:.2}"));
            }
        }

        if let Some(thr) = gates.min_cscv_oos_rank_frac {
            if !cscv_available {
                verdict
                    .fail_reasons
                    .push("cscv_rank_gate_requested_but_cscv_unavailable".to_string());
            } else {
                let rank = cand.cscv_oos_rank_frac_p50().unwrap_or(f64::NEG_INFINITY);
                if !rank.is_finite() || rank < thr {
                    verdict
                        .fail_reasons
                        .push(format!("cscv_oos_rank_frac_below_min:{thr:.2}"));
                }
            }
        }

        if let Some(thr) = gates.min_cscv_oos_metric {
            if !cscv_available {
                verdict
                    .fail_reasons
                    .push("cscv_metric_gate_requested_but_cscv_unavailable".to_string());
            } else {
                let metric = cand.cscv_oos_metric_p50().unwrap_or(f64::NEG_INFINITY);
                if !metric.is_finite() || metric < thr {
                    verdict
                        .fail_reasons
                        .push(format!("cscv_oos_metric_below_min:{thr:.6}"));
                }
            }
        }

        if let Some(mintrl) = cand.min_track_record_length_bars() {
            let obs = cand.track_record_bars().max(2);
            if obs < mintrl {
                verdict
                    .warning_flags
                    .push(format!("min_track_record_length_not_met:{obs}/{mintrl}"));
            }
        }

        verdict
            .warning_flags
            .extend(risk_warning_flags_from_metrics(base_metrics, walk_forward));
        if let Some(monte_carlo) = cand.monte_carlo_stats() {
            for flag in &monte_carlo.advisory.warning_flags {
                if !verdict
                    .warning_flags
                    .iter()
                    .any(|existing| existing == flag)
                {
                    verdict.warning_flags.push(flag.clone());
                }
            }
            for reason in &monte_carlo.advisory.fail_reasons {
                if !verdict
                    .fail_reasons
                    .iter()
                    .any(|existing| existing == reason)
                {
                    verdict.fail_reasons.push(reason.clone());
                }
            }
        }

        verdict.passed = verdict.fail_reasons.is_empty();
        cand.set_validation_verdict(verdict);
    }
}

pub fn build_run_verdict<C: ValidationPostCandidate>(
    out: &[C],
    candidates_tested: usize,
    gates: &ValidationHardGateConfig,
    cscv_pbo: Option<&CscvPboResult>,
    spa: Option<&SpaValidationStats>,
) -> ValidationRunVerdict {
    let mut candidates_survived_hard_gates = 0usize;
    let mut approved_candidate_count = 0usize;
    let mut first_approved_rank = None;
    for (idx, cand) in out.iter().enumerate() {
        if !cand.validation_verdict().passed {
            continue;
        }
        candidates_survived_hard_gates += 1;
        let approved = cand
            .min_track_record_length_bars()
            .map(|mintrl| cand.track_record_bars().max(2) >= mintrl)
            .unwrap_or(false);
        if approved {
            approved_candidate_count += 1;
            first_approved_rank.get_or_insert_with(|| idx.saturating_add(1));
        }
    }

    let mut warning_flags: Vec<String> = Vec::new();
    let mut fail_reasons: Vec<String> = Vec::new();

    let pbo = cscv_pbo.and_then(|res| {
        if res.combinations > 0 && res.pbo.is_finite() {
            Some(res.pbo)
        } else {
            None
        }
    });

    if let Some(res) = cscv_pbo {
        if res.skipped_reason.is_some() {
            warning_flags.push("cscv_skipped".to_string());
        }
    }

    if candidates_survived_hard_gates == 0 {
        fail_reasons.push("no_candidates_survived_hard_gates".to_string());
    }

    let top_candidate = out.first();
    let top_candidate_track_record_bars = top_candidate.map(|cand| cand.track_record_bars().max(2));
    let top_candidate_min_track_record_length_bars =
        top_candidate.and_then(|cand| cand.min_track_record_length_bars());
    let top_candidate_psr = top_candidate.and_then(|cand| cand.psr());
    let top_candidate_dsr = top_candidate.and_then(|cand| cand.dsr());
    if let Some(top) = top_candidate {
        for flag in &top.validation_verdict().warning_flags {
            if !warning_flags.iter().any(|existing| existing == flag) {
                warning_flags.push(flag.clone());
            }
        }
    }

    if candidates_survived_hard_gates > 0 && approved_candidate_count == 0 {
        warning_flags.push("no_candidate_approved".to_string());
        if let (Some(obs), Some(mintrl)) = (
            top_candidate_track_record_bars,
            top_candidate_min_track_record_length_bars,
        ) {
            if obs < mintrl {
                warning_flags.push("top_candidate_min_track_record_length_not_met".to_string());
            }
        }
    }

    if let Some(max_pbo) = gates.max_pbo {
        match pbo {
            Some(v) if v > max_pbo => fail_reasons.push(format!("pbo_above_max:{max_pbo:.2}")),
            Some(_) => {}
            None => fail_reasons.push("pbo_gate_requested_but_cscv_unavailable".to_string()),
        }
    } else if let Some(v) = pbo {
        if v > 0.30 {
            warning_flags.push("high_pbo".to_string());
        }
    }
    if let Some(spa) = spa {
        if let Some(reason) = spa.skipped_reason.as_deref() {
            warning_flags.push(reason.to_string());
        } else if !spa.passed {
            warning_flags.push("spa_not_significant".to_string());
        }
    }

    let (approved, final_recommendation) = if !fail_reasons.is_empty() {
        (false, ValidationRecommendation::NoCandidateSurvived)
    } else if spa
        .map(|spa| spa.skipped_reason.is_none() && !spa.passed)
        .unwrap_or(false)
    {
        (
            false,
            ValidationRecommendation::CandidatesSurvivedButSearchNotStatisticallySuperior,
        )
    } else if approved_candidate_count == 0 {
        let top_short = match (
            top_candidate_track_record_bars,
            top_candidate_min_track_record_length_bars,
        ) {
            (Some(obs), Some(mintrl)) => obs < mintrl,
            _ => false,
        };
        if top_short {
            (
                false,
                ValidationRecommendation::EvidenceTooShortMinTrackRecord,
            )
        } else {
            (false, ValidationRecommendation::NoCandidateApproved)
        }
    } else if !warning_flags.is_empty() || first_approved_rank.unwrap_or(1) > 1 {
        (true, ValidationRecommendation::ApprovedWithWarnings)
    } else {
        (true, ValidationRecommendation::SingleCandidateSupported)
    };

    ValidationRunVerdict {
        passed: fail_reasons.is_empty(),
        approved,
        candidates_tested,
        candidates_survived_hard_gates,
        approved_candidate_count,
        final_recommendation,
        warning_flags,
        fail_reasons,
        first_approved_rank,
        top_candidate_track_record_bars,
        top_candidate_min_track_record_length_bars,
        top_candidate_psr,
        top_candidate_dsr,
        pbo,
        max_pbo: gates.max_pbo,
    }
}

pub struct ValidationEvidencePackBuildInput<'a> {
    pub workflow: &'a str,
    pub data_id: &'a str,
    pub validation_context: Option<&'a ValidationArtifactContext>,
    pub requested_backend: &'a str,
    pub realized_backend: &'a str,
    pub backend_route: Option<&'a str>,
    pub residency_tier: &'a str,
    pub fallback_reason: Option<&'a str>,
    pub route_diagnostics: &'a [String],
    pub candidate_source: &'a str,
    pub requested_candidate_count: usize,
    pub evaluated_candidate_count: usize,
    pub retained_candidate_count: usize,
    pub optimization_trials: Option<usize>,
    pub admission_policy: Option<&'a str>,
    pub hard_gates: &'a ValidationHardGateConfig,
    pub run_verdict: &'a ValidationRunVerdict,
    pub evidence_policy_summary: Option<&'a EvidencePolicySummary>,
    pub validation_provenance: Option<&'a ValidationSemanticsProvenance>,
    pub cscv_requested: bool,
    pub cscv_pbo: Option<&'a CscvPboResult>,
    pub spa_requested: bool,
    pub spa: Option<&'a SpaValidationStats>,
}

fn finite_f64(value: Option<f64>) -> Option<f64> {
    value.filter(|next| next.is_finite())
}

fn serde_scalar_label<T>(value: &T) -> String
where
    T: Serialize + Debug,
{
    serde_json::to_value(value)
        .ok()
        .and_then(|json| json.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("{value:?}"))
}

fn policy_metric<'a>(
    policy: Option<&'a EvidencePolicySummary>,
    metric: &str,
) -> Option<&'a EvidencePolicyMetric> {
    policy.and_then(|summary| {
        summary
            .metrics
            .iter()
            .find(|entry| entry.metric.eq_ignore_ascii_case(metric))
    })
}

fn metric_entry(
    policy: Option<&EvidencePolicySummary>,
    metric: &str,
    availability: ValidationEvidenceAvailability,
    value: Option<f64>,
    threshold: Option<f64>,
    scope: &str,
    comparability_basis: &str,
    unavailable_reason: Option<String>,
) -> ValidationEvidenceMetricV1 {
    let policy_metric = policy_metric(policy, metric);
    ValidationEvidenceMetricV1 {
        metric: metric.to_string(),
        availability,
        value: finite_f64(value),
        threshold: finite_f64(threshold),
        scope: scope.to_string(),
        sample_unit: policy_metric
            .map(|entry| entry.sample_unit.clone())
            .unwrap_or_default(),
        metric_class: policy_metric
            .map(|entry| serde_scalar_label(&entry.metric_class))
            .unwrap_or_default(),
        gate_role: policy_metric
            .map(|entry| serde_scalar_label(&entry.gate_role))
            .unwrap_or_default(),
        comparability_basis: comparability_basis.to_string(),
        interpretation: policy_metric
            .map(|entry| entry.interpretation.clone())
            .unwrap_or_default(),
        caveats: policy_metric
            .map(|entry| entry.caveats.clone())
            .unwrap_or_default(),
        unavailable_reason,
    }
}

fn cscv_availability(
    requested: bool,
    result: Option<&CscvPboResult>,
) -> ValidationEvidenceAvailability {
    if !requested {
        ValidationEvidenceAvailability::NotRequested
    } else if let Some(result) = result {
        if result.skipped_reason.is_some() || result.combinations == 0 {
            ValidationEvidenceAvailability::Skipped
        } else {
            ValidationEvidenceAvailability::Available
        }
    } else {
        ValidationEvidenceAvailability::Unavailable
    }
}

fn spa_availability(
    requested: bool,
    result: Option<&SpaValidationStats>,
) -> ValidationEvidenceAvailability {
    if !requested {
        ValidationEvidenceAvailability::NotRequested
    } else if let Some(result) = result {
        if result.skipped_reason.is_some() {
            ValidationEvidenceAvailability::Skipped
        } else {
            ValidationEvidenceAvailability::Available
        }
    } else {
        ValidationEvidenceAvailability::Unavailable
    }
}

fn evidence_status_reason(
    missing_requested_modules: &[&str],
    missing_realism_evidence: &[String],
) -> String {
    let mut reasons = Vec::new();
    if !missing_requested_modules.is_empty() {
        reasons.push(format!(
            "requested validation evidence incomplete: {}",
            missing_requested_modules.join(", ")
        ));
    }
    if !missing_realism_evidence.is_empty() {
        reasons.push(format!(
            "deployment-realism evidence unknown: {}",
            missing_realism_evidence.join(", ")
        ));
    }
    if reasons.is_empty() {
        "all requested validation modules and deployment-realism declarations produced review metadata"
            .to_string()
    } else {
        reasons.join("; ")
    }
}

fn validation_verdict_summary(verdict: &ValidationRunVerdict) -> String {
    if verdict.approved {
        "approved".to_string()
    } else if verdict.passed {
        "passed_not_approved".to_string()
    } else {
        "failed".to_string()
    }
}

pub fn build_validation_evidence_pack(
    input: ValidationEvidencePackBuildInput<'_>,
) -> ValidationEvidencePackV1 {
    let cscv_availability = cscv_availability(input.cscv_requested, input.cscv_pbo);
    let spa_availability = spa_availability(input.spa_requested, input.spa);
    let mut missing_requested_modules: Vec<&str> = Vec::new();
    if input.cscv_requested && cscv_availability != ValidationEvidenceAvailability::Available {
        missing_requested_modules.push("cscv_pbo");
    }
    if input.spa_requested && spa_availability != ValidationEvidenceAvailability::Available {
        missing_requested_modules.push("spa");
    }

    let data_realism =
        crate::validation_realism::build_validation_realism_evidence(input.validation_context);
    let promotion_eligibility = crate::validation_realism::build_validation_promotion_eligibility(
        input.run_verdict.passed,
        input.run_verdict.approved,
        &data_realism,
    );
    let status = if missing_requested_modules.is_empty() && data_realism.complete {
        ValidationEvidencePackStatus::Complete
    } else {
        ValidationEvidencePackStatus::Incomplete
    };
    let status_reason =
        evidence_status_reason(&missing_requested_modules, &data_realism.missing_or_unknown);
    let context = input.validation_context;
    let expected_sharpe_metric_id = match input.workflow {
        "double_ma_validation" => Some(ta_optimizer::DOUBLE_MA_SHARPE_METRIC_ID),
        "composite_validation" => Some(crate::composite_mo::COMPOSITE_SHARPE_METRIC_ID),
        _ => None,
    };
    let comparable_search_distribution = expected_sharpe_metric_id.and_then(|expected_id| {
        context
            .and_then(|entry| entry.search_distribution.as_ref())
            .filter(|summary| summary.is_comparable_with(expected_id))
    });
    let probability_source = input
        .validation_provenance
        .map(|entry| entry.probability_metric_source.as_str())
        .filter(|entry| !entry.trim().is_empty())
        .unwrap_or("raw_bar_probability_metrics");
    let probability_basis = if let Some(summary) = comparable_search_distribution {
        format!(
            "{probability_source}; DSR peer moments and trial count use the comparable full-search '{}' distribution (metric '{}'); probability metrics are comparable only within the same validation workflow and sample-unit contract",
            summary.metric_surface_kind, summary.sharpe_metric_id
        )
    } else {
        format!(
            "{probability_source}; probability metrics are comparable only within the same validation workflow and sample-unit contract"
        )
    };
    let cscv_basis =
        "candidate_set_slices; set-level CSCV/PBO evidence, not candidate-specific proof";
    let spa_basis = "candidate_set_slices; SPA compares the retained search set against the configured benchmark";
    let track_record_basis = "raw_bars; recommendation support, not a statistical approval proof";

    let cscv_pbo_value = input.cscv_pbo.and_then(|result| {
        if cscv_availability == ValidationEvidenceAvailability::Available {
            finite_f64(Some(result.pbo))
        } else {
            None
        }
    });
    let spa_p_value = input.spa.and_then(|result| {
        if spa_availability == ValidationEvidenceAvailability::Available {
            finite_f64(Some(result.p_value))
        } else {
            None
        }
    });
    let psr = finite_f64(input.run_verdict.top_candidate_psr);
    let dsr = finite_f64(input.run_verdict.top_candidate_dsr);
    let track_record_bars = input
        .run_verdict
        .top_candidate_track_record_bars
        .map(|value| value as f64);
    let min_track_record_bars = input
        .run_verdict
        .top_candidate_min_track_record_length_bars
        .map(|value| value as f64);

    let mut warnings = input.run_verdict.warning_flags.clone();
    if let Some(reason) = input
        .cscv_pbo
        .and_then(|result| result.skipped_reason.clone())
    {
        warnings.push(format!("cscv_pbo_skipped:{reason}"));
    }
    if let Some(reason) = input.spa.and_then(|result| result.skipped_reason.clone()) {
        warnings.push(format!("spa_skipped:{reason}"));
    }
    if !data_realism.complete {
        warnings.push("promotion_blocked_missing_realism_evidence".to_string());
        warnings.extend(
            data_realism
                .missing_or_unknown
                .iter()
                .map(|field| format!("data_realism_unknown:{field}")),
        );
    }
    warnings.sort();
    warnings.dedup();

    ValidationEvidencePackV1 {
        schema_version: VALIDATION_EVIDENCE_PACK_SCHEMA_VERSION.to_string(),
        version: VALIDATION_EVIDENCE_PACK_VERSION,
        workflow: input.workflow.to_string(),
        status,
        status_reason,
        generated_by: "ta_app_core.validation_pipeline".to_string(),
        dataset: ValidationEvidenceDatasetV1 {
            data_id: input.data_id.to_string(),
            strategy_kind: context
                .map(|entry| entry.strategy_kind.clone())
                .filter(|entry| !entry.trim().is_empty()),
            csv_path: context.and_then(|entry| entry.csv_path.clone()),
            timeframe: context.and_then(|entry| entry.timeframe.clone()),
            label: context.and_then(|entry| entry.label.clone()),
            data_realism,
        },
        route: ValidationEvidenceRouteV1 {
            requested_backend: input.requested_backend.to_string(),
            realized_backend: input.realized_backend.to_string(),
            route_label: input.backend_route.map(ToOwned::to_owned),
            residency_tier: input.residency_tier.to_string(),
            fallback_reason: input.fallback_reason.map(ToOwned::to_owned),
            diagnostics: input.route_diagnostics.to_vec(),
        },
        candidate_universe: ValidationEvidenceCandidateUniverseV1 {
            source: input.candidate_source.to_string(),
            requested_candidates: input.requested_candidate_count,
            evaluated_candidates: input.evaluated_candidate_count,
            retained_candidates: input.retained_candidate_count,
            survived_hard_gates: input.run_verdict.candidates_survived_hard_gates,
            approved_candidates: input.run_verdict.approved_candidate_count,
            optimization_trials: input.optimization_trials,
            search_distribution: comparable_search_distribution.cloned(),
            admission_policy: input.admission_policy.map(ToOwned::to_owned),
            selection_scope: "retained_validation_candidates".to_string(),
        },
        verdict: ValidationEvidenceVerdictV1 {
            passed: input.run_verdict.passed,
            approved: input.run_verdict.approved,
            recommendation: serde_scalar_label(&input.run_verdict.final_recommendation),
            first_approved_rank: input.run_verdict.first_approved_rank,
            summary: validation_verdict_summary(input.run_verdict),
            warning_flags: input.run_verdict.warning_flags.clone(),
            fail_reasons: input.run_verdict.fail_reasons.clone(),
        },
        promotion_eligibility,
        metrics: vec![
            metric_entry(
                input.evidence_policy_summary,
                "psr",
                if psr.is_some() {
                    ValidationEvidenceAvailability::Available
                } else {
                    ValidationEvidenceAvailability::NotComputed
                },
                psr,
                input.hard_gates.min_psr,
                "top_retained_candidate",
                &probability_basis,
                None,
            ),
            metric_entry(
                input.evidence_policy_summary,
                "dsr",
                if dsr.is_some() {
                    ValidationEvidenceAvailability::Available
                } else {
                    ValidationEvidenceAvailability::NotComputed
                },
                dsr,
                input.hard_gates.min_dsr,
                "top_retained_candidate",
                &probability_basis,
                None,
            ),
            metric_entry(
                input.evidence_policy_summary,
                "cscv_pbo",
                cscv_availability.clone(),
                cscv_pbo_value,
                input.hard_gates.max_pbo,
                "candidate_universe",
                cscv_basis,
                input
                    .cscv_pbo
                    .and_then(|result| result.skipped_reason.clone())
                    .or_else(|| {
                        if input.cscv_requested && input.cscv_pbo.is_none() {
                            Some("requested but no CSCV/PBO result was returned".to_string())
                        } else {
                            None
                        }
                    }),
            ),
            metric_entry(
                input.evidence_policy_summary,
                "spa_p_value",
                spa_availability.clone(),
                spa_p_value,
                None,
                "candidate_universe",
                spa_basis,
                input
                    .spa
                    .and_then(|result| result.skipped_reason.clone())
                    .or_else(|| {
                        if input.spa_requested && input.spa.is_none() {
                            Some("requested but no SPA result was returned".to_string())
                        } else {
                            None
                        }
                    }),
            ),
            metric_entry(
                input.evidence_policy_summary,
                "min_track_record_length",
                if track_record_bars.is_some() || min_track_record_bars.is_some() {
                    ValidationEvidenceAvailability::Available
                } else {
                    ValidationEvidenceAvailability::NotComputed
                },
                track_record_bars,
                min_track_record_bars,
                "top_retained_candidate",
                track_record_basis,
                None,
            ),
        ],
        modules: vec![
            ValidationEvidenceModuleV1 {
                module: "run_verdict".to_string(),
                availability: ValidationEvidenceAvailability::Available,
                requested: true,
                scope: "validation_run".to_string(),
                summary: validation_verdict_summary(input.run_verdict),
                value: None,
                threshold: None,
                p_value: None,
                candidate_count: Some(input.run_verdict.candidates_tested),
                slice_count: None,
                combinations: None,
                combinations_possible: None,
                skipped_reason: None,
                caveats: vec![
                    "approved is a retained-run recommendation; passed remains hard-gate survival"
                        .to_string(),
                ],
            },
            ValidationEvidenceModuleV1 {
                module: "cscv_pbo".to_string(),
                availability: cscv_availability,
                requested: input.cscv_requested,
                scope: "candidate_universe".to_string(),
                summary: input
                    .cscv_pbo
                    .map(|result| {
                        if result.skipped_reason.is_some() || result.combinations == 0 {
                            "skipped".to_string()
                        } else {
                            "evaluated".to_string()
                        }
                    })
                    .unwrap_or_else(|| {
                        if input.cscv_requested {
                            "requested_missing".to_string()
                        } else {
                            "not_requested".to_string()
                        }
                    }),
                value: cscv_pbo_value,
                threshold: input.hard_gates.max_pbo,
                p_value: None,
                candidate_count: input.cscv_pbo.map(|result| result.candidates),
                slice_count: input.cscv_pbo.map(|result| result.slices),
                combinations: input.cscv_pbo.map(|result| result.combinations),
                combinations_possible: input.cscv_pbo.map(|result| result.combinations_possible),
                skipped_reason: input
                    .cscv_pbo
                    .and_then(|result| result.skipped_reason.clone()),
                caveats: vec![
                    "CSCV/PBO is set-level overfit evidence and cannot certify a single candidate."
                        .to_string(),
                ],
            },
            ValidationEvidenceModuleV1 {
                module: "spa".to_string(),
                availability: spa_availability,
                requested: input.spa_requested,
                scope: "candidate_universe".to_string(),
                summary: input
                    .spa
                    .map(|result| {
                        if result.skipped_reason.is_some() {
                            "skipped".to_string()
                        } else if result.passed {
                            "passed".to_string()
                        } else {
                            "not_significant".to_string()
                        }
                    })
                    .unwrap_or_else(|| {
                        if input.spa_requested {
                            "requested_missing".to_string()
                        } else {
                            "not_requested".to_string()
                        }
                    }),
                value: input.spa.and_then(|result| finite_f64(Some(result.best_candidate_t_stat))),
                threshold: None,
                p_value: spa_p_value,
                candidate_count: input.spa.map(|result| result.candidate_count),
                slice_count: input.spa.map(|result| result.slice_count),
                combinations: None,
                combinations_possible: None,
                skipped_reason: input.spa.and_then(|result| result.skipped_reason.clone()),
                caveats: vec![
                    "SPA is search-set evidence against the configured benchmark and remains review metadata."
                        .to_string(),
                ],
            },
        ],
        policy_summary: input.evidence_policy_summary.cloned(),
        warnings,
        caveats: {
            let mut caveats = input
            .evidence_policy_summary
            .map(|summary| summary.caveats.clone())
            .unwrap_or_default();
            caveats.push(
                "validation approval is statistical research evidence, not deployment authorization"
                    .to_string(),
            );
            caveats.push(
                "fee and slippage stress does not by itself establish liquidity, capacity, market-impact, or borrow availability"
                    .to_string(),
            );
            caveats
        },
    }
}

pub fn build_validation_semantics_provenance<C: ValidationPostCandidate>(
    out: &[C],
    cscv_requested: bool,
    cscv_pbo: Option<&CscvPboResult>,
    cross_summary: Option<&CrossTickerValidationSummary>,
    regime_summary_family: Option<&str>,
) -> ValidationSemanticsProvenance {
    let score_channel = if out
        .iter()
        .any(|cand| cand.iterative_in_sample_metrics().is_some())
    {
        "iterative_in_sample_preferred"
    } else {
        "in_sample_only"
    };

    let cscv_coverage_state = if !cscv_requested {
        "not_requested"
    } else if let Some(result) = cscv_pbo {
        if result.skipped_reason.is_some() || result.combinations == 0 {
            "skipped"
        } else {
            "evaluated"
        }
    } else {
        "requested_but_unavailable"
    };

    let cscv_candidates_with_stats = out
        .iter()
        .filter(|cand| {
            cand.cscv_selected_frac()
                .filter(|value| value.is_finite())
                .is_some()
                || cand.cscv_oos_metric_p50().is_some()
                || cand.cscv_oos_rank_frac_p50().is_some()
        })
        .count();
    let cscv_selected_candidate_count = out
        .iter()
        .filter_map(|cand| cand.cscv_selected_frac())
        .filter(|value| value.is_finite() && *value > 0.0)
        .count();
    let cscv_candidate_stat_coverage = if !cscv_requested {
        "not_requested"
    } else if cscv_candidates_with_stats == 0 {
        "none"
    } else if cscv_candidates_with_stats < out.len() {
        "partial"
    } else {
        "complete"
    };
    let cscv_selection_state = if !cscv_requested {
        "not_requested"
    } else if cscv_coverage_state != "evaluated" {
        "unavailable"
    } else if cscv_selected_candidate_count == 0 {
        "zero_selected"
    } else {
        "selected_candidates_present"
    };
    let monte_carlo_candidates_with_stats = out
        .iter()
        .filter(|cand| cand.monte_carlo_stats().is_some())
        .count();
    let monte_carlo_candidates_with_modes = out
        .iter()
        .filter(|cand| {
            cand.monte_carlo_stats()
                .map(|stats| !stats.modes.is_empty())
                .unwrap_or(false)
        })
        .count();
    let monte_carlo_coverage_state = if monte_carlo_candidates_with_stats == 0 {
        "not_requested"
    } else if monte_carlo_candidates_with_modes == 0 {
        "skipped_no_closed_trades"
    } else if monte_carlo_candidates_with_stats < out.len() {
        "partial"
    } else {
        "evaluated"
    };
    let monte_carlo_policy = out
        .iter()
        .filter_map(|cand| cand.monte_carlo_stats())
        .find_map(|stats| {
            if !stats.advisory.fail_reasons.is_empty() {
                Some("hard_gate")
            } else if !stats.advisory.warning_flags.is_empty() {
                Some("advisory")
            } else {
                None
            }
        })
        .unwrap_or(if monte_carlo_candidates_with_stats > 0 {
            "evidence_only"
        } else {
            "not_requested"
        });
    let (cross_evidence_depth, cross_modules_preserved, cross_modules_omitted) =
        match cross_summary.map(|summary| &summary.mode) {
            Some(crate::validation_shared::CrossTickerMode::Lightweight) => (
                Some("cross_lightweight".to_string()),
                Vec::new(),
                vec![
                    "walk_forward",
                    "reopt_walk_forward",
                    "spa",
                    "perturb",
                    "trade_bootstrap",
                    "monte_carlo",
                    "cscv_pbo",
                    "hard_gates",
                ]
                .into_iter()
                .map(str::to_string)
                .collect(),
            ),
            Some(crate::validation_shared::CrossTickerMode::PreservedFinalists) => (
                Some("cross_preserved_finalists".to_string()),
                vec!["cscv_pbo", "hard_gates", "psr_dsr"]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                vec![
                    "walk_forward",
                    "reopt_walk_forward",
                    "spa",
                    "perturb",
                    "trade_bootstrap",
                    "monte_carlo",
                ]
                .into_iter()
                .map(str::to_string)
                .collect(),
            ),
            Some(crate::validation_shared::CrossTickerMode::FullStack) => (
                Some("cross_full_stack".to_string()),
                vec![
                    "walk_forward",
                    "reopt_walk_forward",
                    "spa",
                    "perturb",
                    "trade_bootstrap",
                    "monte_carlo",
                    "cscv_pbo",
                    "hard_gates",
                    "psr_dsr",
                ]
                .into_iter()
                .map(str::to_string)
                .collect(),
                Vec::new(),
            ),
            None => (None, Vec::new(), Vec::new()),
        };

    ValidationSemanticsProvenance {
        stage_sequence: VALIDATION_STAGE_SEQUENCE
            .iter()
            .map(|stage| (*stage).to_string())
            .collect(),
        score_channel: score_channel.to_string(),
        probability_metric_source: "candidate_sharpe_peer_distribution_psr_dsr".to_string(),
        track_record_source: "sample_obs_raw_bars".to_string(),
        cscv_coverage_state: cscv_coverage_state.to_string(),
        cscv_candidate_stat_coverage: cscv_candidate_stat_coverage.to_string(),
        cscv_selection_state: cscv_selection_state.to_string(),
        cscv_selected_candidate_count,
        cscv_skipped_reason: cscv_pbo.and_then(|result| result.skipped_reason.clone()),
        monte_carlo_coverage_state: monte_carlo_coverage_state.to_string(),
        monte_carlo_policy: monte_carlo_policy.to_string(),
        cross_mode: cross_summary.map(|summary| summary.mode.clone()),
        cross_evidence_depth,
        cross_modules_preserved,
        cross_modules_omitted,
        cross_summary_scope: if cross_summary.is_some() {
            "passed_candidates_only".to_string()
        } else {
            "not_requested".to_string()
        },
        regime_summary_family: regime_summary_family.map(str::to_string),
        regime_summary_role: regime_summary_family.map(|_| "descriptive_only".to_string()),
        passed_semantics: "hard_gates_only".to_string(),
        approved_semantics: "passed_and_min_track_record_length_satisfied".to_string(),
        passed_can_diverge_from_approved: true,
        passed_vs_approved_contract:
            "passed is hard-gate survival; approved also requires retained final verdict support"
                .to_string(),
        cross_summary_mode: cross_summary.map(|summary| match summary.mode {
            crate::validation_shared::CrossTickerMode::Lightweight => "lightweight".to_string(),
            crate::validation_shared::CrossTickerMode::PreservedFinalists => {
                "preserved_finalists".to_string()
            }
            crate::validation_shared::CrossTickerMode::FullStack => "full_stack".to_string(),
        }),
        regime_summary_semantics: regime_summary_family.map(|family| {
            format!("{family} summaries are descriptive-only unless a later gate promotes them")
        }),
        regime_summary_descriptive_only: regime_summary_family.map(|_| true),
    }
}

pub fn parallel_collect_validation_candidates<P, C, F>(
    candidates: &[P],
    seed: u64,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    phase: &'static str,
    process_one: F,
) -> Result<Vec<(C, usize, usize)>, String>
where
    P: Sync,
    C: Send,
    F: Fn(usize, &P, &mut StdRng) -> Result<(C, usize, usize), String> + Sync,
{
    let total = candidates.len();
    let processed = AtomicUsize::new(0);

    candidates
        .par_iter()
        .enumerate()
        .map(|(i, p)| {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let mut rng = StdRng::seed_from_u64(seed ^ (i as u64).wrapping_mul(0x9e3779b97f4a7c15));
            let out = process_one(i, p, &mut rng)?;

            let done = processed.fetch_add(1, Ordering::Relaxed).saturating_add(1);
            if (done & 0x3) == 0 || done == total {
                crate::progress::emit_double_ma_progress(progress, done, total, phase);
            }

            Ok(out)
        })
        .collect::<Result<Vec<_>, String>>()
}

pub fn sequential_collect_validation_candidates<P, C, F>(
    candidates: &[P],
    seed: u64,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    phase: &'static str,
    process_one: F,
) -> Result<Vec<(C, usize, usize)>, String>
where
    F: Fn(usize, &P, &mut StdRng) -> Result<(C, usize, usize), String>,
{
    let total = candidates.len();
    let mut out = Vec::with_capacity(total);
    for (i, candidate) in candidates.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let mut rng = StdRng::seed_from_u64(seed ^ (i as u64).wrapping_mul(0x9e3779b97f4a7c15));
        out.push(process_one(i, candidate, &mut rng)?);
        let done = i.saturating_add(1);
        if (done & 0x3) == 0 || done == total {
            crate::progress::emit_double_ma_progress(progress, done, total, phase);
        }
    }
    Ok(out)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationCollectionMode {
    SequentialFastPath,
    ParallelFullSet,
}

pub fn select_validation_collection_mode(
    hard_gates: &ValidationHardGateConfig,
    cscv_cfg: Option<&CscvPboConfig>,
    spa_requested: bool,
) -> ValidationCollectionMode {
    let global_set_level_obligations = spa_requested
        || cscv_cfg.is_some()
        || hard_gates.min_cscv_oos_rank_frac.is_some()
        || hard_gates.min_cscv_oos_metric.is_some()
        || hard_gates.max_pbo.is_some();
    if global_set_level_obligations {
        ValidationCollectionMode::ParallelFullSet
    } else {
        ValidationCollectionMode::SequentialFastPath
    }
}

fn estimated_window_count(n: usize, start_t: usize, window: usize, step: usize) -> usize {
    if n < 3 || start_t > n.saturating_sub(2) {
        return 0;
    }
    let mut count = 0usize;
    let mut t0 = start_t;
    while t0 <= n.saturating_sub(2) {
        let end = t0.saturating_add(window).min(n);
        if end <= t0.saturating_add(1) {
            break;
        }
        count = count.saturating_add(1);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }
    count
}

fn quantile_index(len: usize, q: f64) -> usize {
    let qq = q.clamp(0.0, 1.0);
    (qq * (len.saturating_sub(1) as f64)).round() as usize
}

fn select_quantile_in_place(values: &mut [f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    if values.len() == 1 {
        return if values[0].is_finite() {
            values[0]
        } else {
            0.0
        };
    }
    let idx = quantile_index(values.len(), q).min(values.len() - 1);
    let (_, nth, _) = values.select_nth_unstable_by(idx, |a, b| a.total_cmp(b));
    if nth.is_finite() {
        *nth
    } else {
        0.0
    }
}

fn select_lower_tail_mean_in_place(values: &mut [f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let idx = quantile_index(values.len(), q).min(values.len() - 1);
    let (_, _, _) = values.select_nth_unstable_by(idx, |a, b| a.total_cmp(b));
    let mut sum = 0.0;
    let mut count = 0usize;
    for &value in &values[..=idx] {
        if value.is_finite() {
            sum += value;
            count += 1;
        }
    }
    if count > 0 {
        sum / count as f64
    } else {
        0.0
    }
}

fn select_upper_tail_mean_in_place(values: &mut [f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let idx = quantile_index(values.len(), q).min(values.len() - 1);
    let (_, _, _) = values.select_nth_unstable_by(idx, |a, b| a.total_cmp(b));
    let mut sum = 0.0;
    let mut count = 0usize;
    for &value in &values[idx..] {
        if value.is_finite() {
            sum += value;
            count += 1;
        }
    }
    if count > 0 {
        sum / count as f64
    } else {
        0.0
    }
}

fn select_median_in_place(values: &mut [f64]) -> f64 {
    select_quantile_in_place(values, 0.5)
}

fn select_median_u32_in_place(values: &mut [u32]) -> u32 {
    if values.is_empty() {
        return 0;
    }
    let idx = values.len() / 2;
    let (_, nth, _) = values.select_nth_unstable(idx);
    *nth
}

pub fn compute_walk_forward_stats<F>(
    n: usize,
    start_t: usize,
    cfg: &WalkForwardConfigOptional,
    mut eval_metrics_window: F,
) -> WalkForwardStats
where
    F: FnMut(usize, usize) -> Metrics,
{
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let estimated_windows = estimated_window_count(n, start_t, window, step);

    let mut sharpes: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut dds: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut pnls: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut downside_ratios: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut ulcers: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut martins: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut underwater_fracs: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut underwater_durations: Vec<u32> = Vec::with_capacity(estimated_windows);
    let mut trades: Vec<u32> = Vec::with_capacity(estimated_windows);
    let mut bars_covered = 0usize;
    let mut merged_start = 0usize;
    let mut merged_end = 0usize;
    let mut has_merged_interval = false;
    let mut effective_windows_upper_bound = 0usize;
    let mut last_disjoint_end = 0usize;

    let mut t0 = start_t;
    while t0 <= n.saturating_sub(2) {
        let end = t0.saturating_add(window).min(n);
        if end <= t0.saturating_add(1) {
            break;
        }
        if !has_merged_interval {
            merged_start = t0;
            merged_end = end;
            has_merged_interval = true;
        } else if t0 <= merged_end {
            merged_end = merged_end.max(end);
        } else {
            bars_covered = bars_covered.saturating_add(merged_end.saturating_sub(merged_start));
            merged_start = t0;
            merged_end = end;
        }
        if effective_windows_upper_bound == 0 || t0 >= last_disjoint_end {
            effective_windows_upper_bound = effective_windows_upper_bound.saturating_add(1);
            last_disjoint_end = end;
        }

        let m = eval_metrics_window(t0, end);
        sharpes.push(m.sharpe);
        dds.push(m.max_dd);
        pnls.push(m.pnl);
        if let Some(ratio) = downside_vol_ratio_from_metrics(&m) {
            downside_ratios.push(ratio);
        }
        ulcers.push(m.ulcer_index);
        martins.push(m.martin_ratio);
        underwater_fracs.push(m.underwater_fraction);
        underwater_durations.push(m.max_underwater_duration_bars);
        trades.push(m.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }
    if has_merged_interval {
        bars_covered = bars_covered.saturating_add(merged_end.saturating_sub(merged_start));
    }

    let sharpe_worst = worst_min(&sharpes);
    let max_dd_worst = worst_max(&dds);
    let max_dd_tail_mean_p95 = select_upper_tail_mean_in_place(&mut dds, 0.95);
    let pnl_worst = worst_min(&pnls);
    let pnl_tail_mean_p05 = select_lower_tail_mean_in_place(&mut pnls, 0.05);
    let downside_vol_ratio_worst = worst_max(&downside_ratios);
    let ulcer_index_worst = worst_max(&ulcers);
    let martin_ratio_worst = worst_min(&martins);
    let underwater_fraction_worst = worst_max(&underwater_fracs);
    let max_underwater_duration_bars_worst =
        crate::stats_common::worst_max_u32(&underwater_durations);
    let trades_worst = worst_min_u32(&trades);

    WalkForwardStats {
        windows: sharpes.len(),
        effective_windows_upper_bound: Some(effective_windows_upper_bound),
        bars_covered: Some(bars_covered),
        sharpe_median: select_median_in_place(&mut sharpes),
        sharpe_worst,
        max_dd_median: select_median_in_place(&mut dds),
        max_dd_worst,
        max_dd_tail_mean_p95,
        pnl_median: select_median_in_place(&mut pnls),
        pnl_worst,
        pnl_tail_mean_p05,
        downside_vol_ratio_median: select_median_in_place(&mut downside_ratios),
        downside_vol_ratio_worst,
        ulcer_index_median: select_median_in_place(&mut ulcers),
        ulcer_index_worst,
        martin_ratio_median: select_median_in_place(&mut martins),
        martin_ratio_worst,
        underwater_fraction_median: select_median_in_place(&mut underwater_fracs),
        underwater_fraction_worst,
        max_underwater_duration_bars_median: select_median_u32_in_place(&mut underwater_durations),
        max_underwater_duration_bars_worst,
        trades_median: select_median_u32_in_place(&mut trades),
        trades_worst,
    }
}

fn normalized_json_drift_value(left: &JsonValue, right: &JsonValue) -> f64 {
    match (left, right) {
        (JsonValue::Null, JsonValue::Null) => 0.0,
        (JsonValue::Bool(a), JsonValue::Bool(b)) => {
            if a == b {
                0.0
            } else {
                1.0
            }
        }
        (JsonValue::String(a), JsonValue::String(b)) => {
            if a == b {
                0.0
            } else {
                1.0
            }
        }
        (JsonValue::Number(a), JsonValue::Number(b)) => {
            let left = a.as_f64().unwrap_or(0.0);
            let right = b.as_f64().unwrap_or(0.0);
            let scale = left.abs().max(right.abs()).max(1.0);
            ((left - right).abs() / scale).clamp(0.0, 1.0)
        }
        (JsonValue::Array(a), JsonValue::Array(b)) => {
            let len = a.len().max(b.len());
            if len == 0 {
                return 0.0;
            }
            let mut sum = 0.0;
            for idx in 0..len {
                let drift = match (a.get(idx), b.get(idx)) {
                    (Some(left), Some(right)) => normalized_json_drift_value(left, right),
                    (Some(_), None) | (None, Some(_)) => 1.0,
                    (None, None) => 0.0,
                };
                sum += drift;
            }
            (sum / len as f64).clamp(0.0, 1.0)
        }
        (JsonValue::Object(a), JsonValue::Object(b)) => {
            let mut keys: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
            keys.extend(a.keys().map(|key| key.as_str()));
            keys.extend(b.keys().map(|key| key.as_str()));
            if keys.is_empty() {
                return 0.0;
            }
            let mut sum = 0.0;
            for key in keys {
                let drift = match (a.get(key), b.get(key)) {
                    (Some(left), Some(right)) => normalized_json_drift_value(left, right),
                    (Some(_), None) | (None, Some(_)) => 1.0,
                    (None, None) => 0.0,
                };
                sum += drift;
            }
            (sum / a.len().max(b.len()) as f64).clamp(0.0, 1.0)
        }
        _ => 1.0,
    }
}

fn normalized_param_drift_from_key_text(left: &str, right: &str) -> f64 {
    if left == right {
        return 0.0;
    }
    let left = serde_json::from_str::<JsonValue>(left);
    let right = serde_json::from_str::<JsonValue>(right);
    match (left, right) {
        (Ok(left), Ok(right)) => normalized_json_drift_value(&left, &right),
        _ => 1.0,
    }
}

fn normalized_param_drift_from_parsed_key_values(
    left_key: &str,
    right_key: &str,
    left: Option<&JsonValue>,
    right: Option<&JsonValue>,
) -> f64 {
    if left_key == right_key {
        return 0.0;
    }
    match (left, right) {
        (Some(left), Some(right)) => normalized_json_drift_value(left, right),
        _ => normalized_param_drift_from_key_text(left_key, right_key),
    }
}

pub fn compute_reopt_walk_forward_stats<P, FStart, FEval, FKey>(
    candidates: &[P],
    n: usize,
    cfg: &ReoptWalkForwardConfigOptional,
    mut candidate_start_t: FStart,
    mut eval_metrics_window: FEval,
    mut candidate_key: FKey,
) -> Option<ReoptWalkForwardStats>
where
    FStart: FnMut(&P) -> usize,
    FEval: FnMut(&P, usize, usize) -> Metrics,
    FKey: FnMut(&P) -> String,
{
    if candidates.is_empty() || n < 3 {
        return None;
    }

    let top_n_candidates = cfg.top_n_candidates.max(1).min(candidates.len());
    let finalists = &candidates[..top_n_candidates];
    let common_start_t = finalists
        .iter()
        .map(&mut candidate_start_t)
        .max()
        .unwrap_or(0)
        .min(n.saturating_sub(2));
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let estimated_windows = estimated_window_count(n, common_start_t, window, step);
    let finalist_keys: Vec<String> = finalists.iter().map(&mut candidate_key).collect();
    let finalist_key_values: Vec<Option<JsonValue>> = finalist_keys
        .iter()
        .map(|key| serde_json::from_str::<JsonValue>(key).ok())
        .collect();

    let mut train_scores: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut sharpes: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut dds: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut pnls: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut ulcers: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut martins: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut trades: Vec<u32> = Vec::with_capacity(estimated_windows);
    let mut winner_indices: Vec<usize> = Vec::with_capacity(estimated_windows);
    let mut window_stats: Vec<ReoptWalkForwardWindowStats> = Vec::with_capacity(estimated_windows);
    let mut pnl_efficiencies: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut sharpe_efficiencies: Vec<f64> = Vec::with_capacity(estimated_windows);
    let mut positive_forward_pnl = 0usize;

    let mut t0 = common_start_t;
    while t0 < n.saturating_sub(2) {
        let train_end = t0.saturating_add(window).min(n);
        if train_end <= t0.saturating_add(1) {
            break;
        }
        let test_end = train_end.saturating_add(step).min(n);
        if test_end <= train_end.saturating_add(1) {
            break;
        }

        let mut best_idx: Option<usize> = None;
        let mut best_score = f64::NEG_INFINITY;
        let mut best_train_metrics: Option<Metrics> = None;
        for (idx, cand) in finalists.iter().enumerate() {
            let metrics = eval_metrics_window(cand, t0, train_end);
            let score = robust_score_from_metrics(&metrics, None);
            if score.is_finite() && score > best_score {
                best_score = score;
                best_idx = Some(idx);
                best_train_metrics = Some(metrics);
            }
        }

        let Some(best_idx) = best_idx else {
            break;
        };
        let winner = &finalists[best_idx];
        let train_metrics = best_train_metrics.expect("reopt winner train metrics");
        let forward = eval_metrics_window(winner, train_end, test_end);
        let winner_key = finalist_keys[best_idx].clone();
        let pnl_efficiency = if train_metrics.pnl.is_finite() && train_metrics.pnl.abs() > 1e-12 {
            Some(forward.pnl / train_metrics.pnl)
        } else {
            None
        };
        let sharpe_efficiency =
            if train_metrics.sharpe.is_finite() && train_metrics.sharpe.abs() > 1e-12 {
                Some(forward.sharpe / train_metrics.sharpe)
            } else {
                None
            };

        train_scores.push(best_score);
        sharpes.push(forward.sharpe);
        dds.push(forward.max_dd);
        pnls.push(forward.pnl);
        ulcers.push(forward.ulcer_index);
        martins.push(forward.martin_ratio);
        trades.push(forward.trades);
        if forward.pnl > 0.0 {
            positive_forward_pnl = positive_forward_pnl.saturating_add(1);
        }
        if let Some(value) = pnl_efficiency.filter(|value| value.is_finite()) {
            pnl_efficiencies.push(value);
        }
        if let Some(value) = sharpe_efficiency.filter(|value| value.is_finite()) {
            sharpe_efficiencies.push(value);
        }
        winner_indices.push(best_idx);
        window_stats.push(ReoptWalkForwardWindowStats {
            train_start: t0,
            train_end,
            test_end,
            winner_key,
            train_score: best_score,
            train_pnl: train_metrics.pnl,
            train_sharpe: train_metrics.sharpe,
            train_ulcer_index: train_metrics.ulcer_index,
            train_martin_ratio: train_metrics.martin_ratio,
            forward_pnl: forward.pnl,
            forward_sharpe: forward.sharpe,
            forward_max_dd: forward.max_dd,
            forward_ulcer_index: forward.ulcer_index,
            forward_martin_ratio: forward.martin_ratio,
            forward_trades: forward.trades,
            pnl_efficiency,
            sharpe_efficiency,
        });

        if test_end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    if sharpes.is_empty() {
        return None;
    }

    let forward_sharpe_worst = worst_min(&sharpes);
    let forward_max_dd_worst = worst_max(&dds);
    let forward_pnl_worst = worst_min(&pnls);
    let forward_ulcer_index_worst = worst_max(&ulcers);
    let forward_martin_ratio_worst = worst_min(&martins);
    let forward_trades_worst = worst_min_u32(&trades);
    let pnl_eff_worst = worst_min(&pnl_efficiencies);
    let sharpe_eff_worst = worst_min(&sharpe_efficiencies);

    let mut winner_counts = vec![0usize; finalists.len()];
    let mut winner_changes = 0usize;
    let mut prev_idx: Option<usize> = None;
    let mut drift_values: Vec<f64> = Vec::with_capacity(winner_indices.len().saturating_sub(1));
    for &winner_idx in &winner_indices {
        winner_counts[winner_idx] = winner_counts[winner_idx].saturating_add(1);
        if let Some(prev) = prev_idx {
            if prev != winner_idx {
                winner_changes = winner_changes.saturating_add(1);
            }
            drift_values.push(normalized_param_drift_from_parsed_key_values(
                &finalist_keys[prev],
                &finalist_keys[winner_idx],
                finalist_key_values[prev].as_ref(),
                finalist_key_values[winner_idx].as_ref(),
            ));
        }
        prev_idx = Some(winner_idx);
    }
    let max_winner_count = winner_counts.iter().copied().max().unwrap_or(0);
    let drift_mean = if drift_values.is_empty() {
        0.0
    } else {
        drift_values.iter().sum::<f64>() / drift_values.len() as f64
    };
    let drift_max = drift_values.iter().copied().fold(0.0_f64, f64::max);
    let sharpe_eff_mean = if sharpe_efficiencies.is_empty() {
        None
    } else {
        Some(sharpe_efficiencies.iter().sum::<f64>() / sharpe_efficiencies.len() as f64)
    };
    let winner_sequence: Vec<String> = winner_indices
        .iter()
        .map(|&winner_idx| finalist_keys[winner_idx].clone())
        .collect();

    Some(ReoptWalkForwardStats {
        windows: winner_indices.len(),
        top_n_candidates,
        unique_winners: winner_counts.iter().filter(|&&count| count > 0).count(),
        winner_changes,
        dominant_winner_frac: if winner_indices.is_empty() {
            0.0
        } else {
            max_winner_count as f64 / winner_indices.len() as f64
        },
        winner_param_drift_mean: drift_mean,
        winner_param_drift_max: drift_max,
        train_score_median: select_median_in_place(&mut train_scores),
        forward_positive_pnl_frac: if winner_sequence.is_empty() {
            0.0
        } else {
            positive_forward_pnl as f64 / winner_sequence.len() as f64
        },
        forward_sharpe_median: select_median_in_place(&mut sharpes),
        forward_sharpe_worst,
        forward_max_dd_median: select_median_in_place(&mut dds),
        forward_max_dd_worst,
        forward_pnl_median: select_median_in_place(&mut pnls),
        forward_pnl_worst,
        forward_ulcer_index_median: select_median_in_place(&mut ulcers),
        forward_ulcer_index_worst,
        forward_martin_ratio_median: select_median_in_place(&mut martins),
        forward_martin_ratio_worst,
        forward_trades_median: select_median_u32_in_place(&mut trades),
        forward_trades_worst,
        window_pnl_efficiency_median: if pnl_efficiencies.is_empty() {
            None
        } else {
            Some(select_median_in_place(&mut pnl_efficiencies))
        },
        window_pnl_efficiency_worst: if pnl_efficiencies.is_empty() {
            None
        } else {
            Some(pnl_eff_worst)
        },
        window_sharpe_efficiency_median: if sharpe_efficiencies.is_empty() {
            None
        } else {
            Some(select_median_in_place(&mut sharpe_efficiencies))
        },
        window_sharpe_efficiency_worst: if sharpe_efficiencies.is_empty() {
            None
        } else {
            Some(sharpe_eff_worst)
        },
        window_sharpe_efficiency_std: sharpe_eff_mean
            .map(|mean| sample_std(&sharpe_efficiencies, mean)),
        winner_sequence,
        window_stats,
    })
}

pub fn summarize_perturb_stats(
    cfg: &PerturbConfig,
    iters: usize,
    sharpes: &[f64],
    dds: &[f64],
    pnls: &[f64],
    pnl_pos: usize,
) -> PerturbStats {
    fn mean_finite(xs: &[f64]) -> f64 {
        if xs.is_empty() {
            return 0.0;
        }
        let mut sum = 0.0_f64;
        let mut c = 0usize;
        for &x in xs {
            if x.is_finite() {
                sum += x;
                c += 1;
            }
        }
        if c > 0 {
            sum / c as f64
        } else {
            0.0
        }
    }

    let sharpe_mean = mean_finite(sharpes);
    let dd_mean = mean_finite(dds);
    let pnl_mean = mean_finite(pnls);

    PerturbStats {
        iterations: iters,
        pct_min: cfg.pct_min,
        pct_max: cfg.pct_max,
        sharpe_mean,
        sharpe_worst: worst_min(sharpes),
        max_dd_mean: dd_mean,
        max_dd_worst: worst_max(dds),
        pnl_mean,
        pnl_worst: worst_min(pnls),
        pnl_positive_frac: if iters > 0 {
            pnl_pos as f64 / iters as f64
        } else {
            0.0
        },
    }
}

pub fn compute_perturb_stats<P, FPerturb, FEval>(
    base_params: &P,
    cfg: &PerturbConfig,
    cancel: &AtomicBool,
    rng: &mut StdRng,
    mut perturb_params: FPerturb,
    mut eval_metrics: FEval,
) -> Result<Option<PerturbStats>, String>
where
    FPerturb: FnMut(&P, &PerturbConfig, &mut StdRng) -> P,
    FEval: FnMut(&P) -> Result<Metrics, String>,
{
    let iters = cfg.iterations.min(1024);
    if iters == 0 {
        return Ok(None);
    }

    let mut sharpe_sum = 0.0_f64;
    let mut sharpe_count = 0usize;
    let mut sharpe_worst = f64::INFINITY;
    let mut dd_sum = 0.0_f64;
    let mut dd_count = 0usize;
    let mut dd_worst = f64::NEG_INFINITY;
    let mut pnl_sum = 0.0_f64;
    let mut pnl_count = 0usize;
    let mut pnl_worst = f64::INFINITY;
    let mut pnl_pos: usize = 0;

    for _ in 0..iters {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let pp = perturb_params(base_params, cfg, rng);
        let m = eval_metrics(&pp)?;
        if m.pnl > 0.0 {
            pnl_pos += 1;
        }
        if m.sharpe.is_finite() {
            sharpe_sum += m.sharpe;
            sharpe_count = sharpe_count.saturating_add(1);
        }
        sharpe_worst = sharpe_worst.min(m.sharpe);
        if m.max_dd.is_finite() {
            dd_sum += m.max_dd;
            dd_count = dd_count.saturating_add(1);
        }
        dd_worst = dd_worst.max(m.max_dd);
        if m.pnl.is_finite() {
            pnl_sum += m.pnl;
            pnl_count = pnl_count.saturating_add(1);
        }
        pnl_worst = pnl_worst.min(m.pnl);
    }

    Ok(Some(PerturbStats {
        iterations: iters,
        pct_min: cfg.pct_min,
        pct_max: cfg.pct_max,
        sharpe_mean: if sharpe_count > 0 {
            sharpe_sum / sharpe_count as f64
        } else {
            0.0
        },
        sharpe_worst: if sharpe_worst.is_finite() {
            sharpe_worst
        } else {
            0.0
        },
        max_dd_mean: if dd_count > 0 {
            dd_sum / dd_count as f64
        } else {
            0.0
        },
        max_dd_worst: if dd_worst.is_finite() { dd_worst } else { 0.0 },
        pnl_mean: if pnl_count > 0 {
            pnl_sum / pnl_count as f64
        } else {
            0.0
        },
        pnl_worst: if pnl_worst.is_finite() {
            pnl_worst
        } else {
            0.0
        },
        pnl_positive_frac: pnl_pos as f64 / iters as f64,
    }))
}

fn sanitized_monte_carlo_modes(cfg: &MonteCarloValidationConfig) -> Vec<MonteCarloMode> {
    let source = if cfg.modes.is_empty() {
        vec![MonteCarloMode::TradeIid, MonteCarloMode::TradeStationary]
    } else {
        cfg.modes.clone()
    };
    let mut out: Vec<MonteCarloMode> = Vec::with_capacity(source.len());
    for mode in source {
        if !out.iter().any(|existing| *existing == mode) {
            out.push(mode);
        }
    }
    out
}

fn monte_carlo_histogram(values: &[f64], bins: usize) -> Option<MonteCarloDistributionBins> {
    let bins = bins.clamp(4, 200);
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &value in values {
        if value.is_finite() {
            min = min.min(value);
            max = max.max(value);
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return None;
    }
    let mut counts = vec![0usize; bins];
    if (max - min).abs() <= 1.0e-12 {
        counts[0] = values.iter().filter(|value| value.is_finite()).count();
        return Some(MonteCarloDistributionBins { min, max, counts });
    }
    let span = max - min;
    for &value in values {
        if !value.is_finite() {
            continue;
        }
        let idx = (((value - min) / span) * bins as f64).floor() as usize;
        counts[idx.min(bins - 1)] += 1;
    }
    Some(MonteCarloDistributionBins { min, max, counts })
}

fn summarize_monte_carlo_mode(
    simulations: usize,
    pos: usize,
    pnls: &mut [f64],
    dds: &mut [f64],
    stationary_mean_block_len: Option<usize>,
    distribution_bins: usize,
) -> MonteCarloModeStats {
    let pnl_p05 = select_quantile_in_place(pnls, 0.05);
    let pnl_tail_mean_p05 = select_lower_tail_mean_in_place(pnls, 0.05);
    let pnl_p50 = select_quantile_in_place(pnls, 0.5);
    let pnl_p95 = select_quantile_in_place(pnls, 0.95);
    let max_dd_p50 = select_quantile_in_place(dds, 0.5);
    let max_dd_p95 = select_quantile_in_place(dds, 0.95);
    let max_dd_tail_mean_p95 = select_upper_tail_mean_in_place(dds, 0.95);
    MonteCarloModeStats {
        simulations,
        prob_pnl_positive: if simulations > 0 {
            pos as f64 / simulations as f64
        } else {
            0.0
        },
        pnl_p05,
        pnl_tail_mean_p05,
        pnl_p50,
        pnl_p95,
        max_dd_p50,
        max_dd_p95,
        max_dd_tail_mean_p95,
        stationary_mean_block_len,
        pnl_histogram: monte_carlo_histogram(pnls, distribution_bins),
        drawdown_histogram: monte_carlo_histogram(dds, distribution_bins),
    }
}

fn compute_iid_closed_trade_monte_carlo(
    closed: &[f64],
    simulations: usize,
    distribution_bins: usize,
    rng: &mut StdRng,
) -> MonteCarloModeStats {
    let trade_count = closed.len();
    let mut pnls: Vec<f64> = Vec::with_capacity(simulations);
    let mut dds: Vec<f64> = Vec::with_capacity(simulations);
    let mut pos = 0usize;
    for _ in 0..simulations {
        let mut eq = 1.0_f64;
        let mut peak = 1.0_f64;
        let mut max_dd = 0.0_f64;
        for _ in 0..trade_count {
            let idx = rng.gen_range(0..trade_count);
            let r = (1.0 + closed[idx]).max(0.000_000_1);
            eq *= r;
            if eq > peak {
                peak = eq;
            }
            let dd = (peak - eq) / peak.max(0.000_000_1);
            if dd > max_dd {
                max_dd = dd;
            }
        }
        let pnl = eq - 1.0;
        if pnl > 0.0 {
            pos += 1;
        }
        pnls.push(pnl);
        dds.push(max_dd);
    }
    summarize_monte_carlo_mode(
        simulations,
        pos,
        &mut pnls,
        &mut dds,
        None,
        distribution_bins,
    )
}

fn compute_stationary_closed_trade_monte_carlo(
    closed: &[f64],
    simulations: usize,
    stationary_mean_block_len: usize,
    distribution_bins: usize,
    rng: &mut StdRng,
) -> MonteCarloModeStats {
    let trade_count = closed.len();
    let mut pnls: Vec<f64> = Vec::with_capacity(simulations);
    let mut dds: Vec<f64> = Vec::with_capacity(simulations);
    let mut pos = 0usize;
    let mut stationary_scratch = StationaryBootstrapScratch::default();
    for _ in 0..simulations {
        let indices = stationary_scratch.indices(trade_count, stationary_mean_block_len, rng);
        let mut eq = 1.0_f64;
        let mut peak = 1.0_f64;
        let mut max_dd = 0.0_f64;
        for &idx in indices {
            let r = (1.0 + closed[idx]).max(0.000_000_1);
            eq *= r;
            if eq > peak {
                peak = eq;
            }
            let dd = (peak - eq) / peak.max(0.000_000_1);
            if dd > max_dd {
                max_dd = dd;
            }
        }
        let pnl = eq - 1.0;
        if pnl > 0.0 {
            pos += 1;
        }
        pnls.push(pnl);
        dds.push(max_dd);
    }
    summarize_monte_carlo_mode(
        simulations,
        pos,
        &mut pnls,
        &mut dds,
        Some(stationary_mean_block_len),
        distribution_bins,
    )
}

fn apply_monte_carlo_thresholds(
    stats: Option<&MonteCarloModeStats>,
    trade_count: usize,
    cfg: &MonteCarloThresholdConfig,
    out: &mut Vec<String>,
    prefix: &str,
) {
    if let Some(min_trade_count) = cfg.min_trade_count {
        if trade_count < min_trade_count {
            out.push(format!(
                "{prefix}_low_trade_count:{trade_count}/{min_trade_count}"
            ));
        }
    }
    let Some(stats) = stats else {
        return;
    };
    if let Some(thr) = cfg.min_prob_pnl_positive {
        if !stats.prob_pnl_positive.is_finite() || stats.prob_pnl_positive < thr {
            out.push(format!("{prefix}_low_profit_probability:{thr:.2}"));
        }
    }
    if let Some(thr) = cfg.min_pnl_p05 {
        if !stats.pnl_p05.is_finite() || stats.pnl_p05 < thr {
            out.push(format!("{prefix}_left_tail_loss:{thr:.6}"));
        }
    }
    if let Some(thr) = cfg.max_drawdown_p95 {
        if !stats.max_dd_p95.is_finite() || stats.max_dd_p95 > thr {
            out.push(format!("{prefix}_drawdown_tail:{thr:.6}"));
        }
    }
}

fn monte_carlo_advisory(
    cfg: &MonteCarloValidationConfig,
    trade_count: usize,
    modes: &BTreeMap<String, MonteCarloModeStats>,
) -> MonteCarloAdvisory {
    let primary = modes
        .get(MonteCarloMode::TradeStationary.as_key())
        .or_else(|| modes.get(MonteCarloMode::TradeIid.as_key()));
    let mut warning_flags = Vec::new();
    let mut fail_reasons = Vec::new();
    if let Some(warn) = cfg.warn.as_ref() {
        apply_monte_carlo_thresholds(
            primary,
            trade_count,
            warn,
            &mut warning_flags,
            "monte_carlo",
        );
    }
    if let Some(gate) = cfg.gate.as_ref() {
        apply_monte_carlo_thresholds(primary, trade_count, gate, &mut fail_reasons, "monte_carlo");
    }
    MonteCarloAdvisory {
        passed: fail_reasons.is_empty(),
        warning_flags,
        fail_reasons,
    }
}

pub fn compute_monte_carlo_validation_stats(
    trades: &[Trade],
    cfg: &MonteCarloValidationConfig,
    rng: &mut StdRng,
) -> MonteCarloValidationStats {
    let simulations = cfg.simulations.min(50_000);
    let closed: Vec<f64> = trades.iter().filter(|t| !t.open).map(|t| t.pnl).collect();
    let trade_count = closed.len();
    let tail_alpha = cfg.tail_alpha.clamp(0.001, 0.25);
    let distribution_bins = cfg.distribution_bins.clamp(4, 200);
    let mut modes: BTreeMap<String, MonteCarloModeStats> = BTreeMap::new();
    if simulations > 0 && trade_count > 0 {
        for mode in sanitized_monte_carlo_modes(cfg) {
            match mode {
                MonteCarloMode::TradeIid => {
                    let stats = compute_iid_closed_trade_monte_carlo(
                        &closed,
                        simulations,
                        distribution_bins,
                        rng,
                    );
                    modes.insert(mode.as_key().to_string(), stats);
                }
                MonteCarloMode::TradeStationary => {
                    let block_len = cfg
                        .stationary_mean_block_len
                        .map(|value| value.clamp(1, trade_count.max(1)))
                        .unwrap_or_else(|| trade_bootstrap_stationary_mean_block_len(trade_count));
                    let stats = compute_stationary_closed_trade_monte_carlo(
                        &closed,
                        simulations,
                        block_len,
                        distribution_bins,
                        rng,
                    );
                    modes.insert(mode.as_key().to_string(), stats);
                }
            }
        }
    }
    let advisory = monte_carlo_advisory(cfg, trade_count, &modes);
    MonteCarloValidationStats {
        schema_version: cfg.schema_version.max(1),
        simulations,
        trade_count,
        tail_alpha,
        modes,
        advisory,
    }
}

pub fn compute_trade_bootstrap_stats(
    trades: &[Trade],
    cfg: &TradeBootstrapConfig,
    rng: &mut StdRng,
) -> TradeBootstrapStats {
    let mc_cfg = MonteCarloValidationConfig::from_trade_bootstrap(cfg);
    compute_monte_carlo_validation_stats(trades, &mc_cfg, rng).to_trade_bootstrap_stats()
}

fn trade_bootstrap_stationary_mean_block_len(trade_count: usize) -> usize {
    if trade_count <= 1 {
        return 1;
    }
    let heuristic = (trade_count as f64).sqrt().round() as usize;
    heuristic.clamp(2, trade_count.min(32).max(2))
}

pub fn maybe_run_iterative_validation<F>(
    validation_engine: ValidationEngine,
    in_sample: &Metrics,
    lr: &[f64],
    ohlc: OhlcSlice<'_>,
    start_t: usize,
    cfg: &StrategyConfig,
    iterative_cfg: &IterativeSimConfig,
    build_desired_positions: F,
) -> (Option<Metrics>, Option<IterativeMetricsDelta>)
where
    F: FnOnce() -> Vec<i32>,
{
    if validation_engine != ValidationEngine::Iterative {
        return (None, None);
    }

    let desired = build_desired_positions();
    let iterative =
        simulate_iterative_from_signals(lr, ohlc, &desired, start_t, cfg, iterative_cfg);
    let delta = IterativeMetricsDelta::from_pair(in_sample, &iterative);
    (Some(iterative), Some(delta))
}

pub fn finalize_with_probability_and_optional_cscv<C, F>(
    out: &mut Vec<C>,
    sample_obs: &[usize],
    hard_gates: &ValidationHardGateConfig,
    cscv_cfg: Option<&CscvPboConfig>,
    total_bars: usize,
    common_start_t: usize,
    seed: u64,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    cscv_prep_phase: &'static str,
    build_slice_stats_for_candidate: F,
) -> Result<Option<CscvPboResult>, String>
where
    C: ValidationPostCandidate,
    F: FnMut(&C, usize, usize, usize) -> Result<Vec<SliceReturnStats>, String>,
{
    finalize_with_probability_and_optional_cscv_with_research_context(
        out,
        sample_obs,
        None,
        None,
        hard_gates,
        cscv_cfg,
        total_bars,
        common_start_t,
        seed,
        cancel,
        progress,
        cscv_prep_phase,
        build_slice_stats_for_candidate,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn finalize_with_probability_and_optional_cscv_with_research_context<C, F>(
    out: &mut Vec<C>,
    sample_obs: &[usize],
    optimization_trials: Option<usize>,
    return_moments: Option<&[Option<crate::sharpe_stats::ReturnDistributionMoments>]>,
    hard_gates: &ValidationHardGateConfig,
    cscv_cfg: Option<&CscvPboConfig>,
    total_bars: usize,
    common_start_t: usize,
    seed: u64,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    cscv_prep_phase: &'static str,
    build_slice_stats_for_candidate: F,
) -> Result<Option<CscvPboResult>, String>
where
    C: ValidationPostCandidate,
    F: FnMut(&C, usize, usize, usize) -> Result<Vec<SliceReturnStats>, String>,
{
    finalize_with_probability_and_optional_cscv_with_search_distribution(
        out,
        sample_obs,
        optimization_trials,
        None,
        None,
        return_moments,
        hard_gates,
        cscv_cfg,
        total_bars,
        common_start_t,
        seed,
        cancel,
        progress,
        cscv_prep_phase,
        build_slice_stats_for_candidate,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn finalize_with_probability_and_optional_cscv_with_search_distribution<C, F>(
    out: &mut Vec<C>,
    sample_obs: &[usize],
    optimization_trials: Option<usize>,
    search_distribution: Option<&ta_optimizer::SearchDistributionSummary>,
    expected_sharpe_metric_id: Option<&str>,
    return_moments: Option<&[Option<crate::sharpe_stats::ReturnDistributionMoments>]>,
    hard_gates: &ValidationHardGateConfig,
    cscv_cfg: Option<&CscvPboConfig>,
    total_bars: usize,
    common_start_t: usize,
    seed: u64,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    cscv_prep_phase: &'static str,
    mut build_slice_stats_for_candidate: F,
) -> Result<Option<CscvPboResult>, String>
where
    C: ValidationPostCandidate,
    F: FnMut(&C, usize, usize, usize) -> Result<Vec<SliceReturnStats>, String>,
{
    compute_probability_metrics_with_search_distribution(
        out,
        sample_obs,
        None,
        None,
        optimization_trials,
        search_distribution,
        expected_sharpe_metric_id,
        return_moments,
    );
    mark_robust_pareto(out);

    let mut cscv_pbo: Option<CscvPboResult> = None;
    if let Some(cfg) = cscv_cfg {
        let slices = sanitize_cscv_slices(cfg.slices);
        let slice_bars = total_bars.saturating_sub(common_start_t) / slices;
        if slice_bars >= 2 && out.len() >= 2 {
            crate::progress::emit_double_ma_progress(progress, 0, out.len(), cscv_prep_phase);
            let mut by_cand: Vec<Vec<SliceReturnStats>> = Vec::with_capacity(out.len());
            for (i, cand) in out.iter().enumerate() {
                if cancel.load(Ordering::Relaxed) {
                    return Err("cancelled".to_string());
                }
                by_cand.push(build_slice_stats_for_candidate(
                    cand,
                    common_start_t,
                    slice_bars,
                    slices,
                )?);
                let done = i.saturating_add(1);
                if (done & 0x7) == 0 || done == out.len() {
                    crate::progress::emit_double_ma_progress(
                        progress,
                        done,
                        out.len(),
                        cscv_prep_phase,
                    );
                }
            }

            let (res, cand_stats) =
                compute_cscv_pbo_from_slices(&by_cand, slice_bars, cfg, seed, cancel, progress)?;
            for (cand, st) in out.iter_mut().zip(cand_stats.into_iter()) {
                cand.set_cscv_pbo(Some(st));
            }
            cscv_pbo = Some(res);
        } else {
            cscv_pbo = Some(CscvPboResult {
                slices,
                slice_bars,
                candidates: out.len(),
                combinations: 0,
                combinations_possible: 0,
                pbo: 0.0,
                lambda_p25: 0.0,
                lambda_p50: 0.0,
                lambda_p75: 0.0,
                degradation_slope: 0.0,
                degradation_intercept: 0.0,
                degradation_r2: 0.0,
                threshold: cfg.threshold,
                prob_oos_below_threshold: 0.0,
                skipped_reason: Some(
                    "CSCV/PBO skipped (not enough bars after warmup).".to_string(),
                ),
            });
        }
    }

    apply_candidate_hard_gates(out, hard_gates, cscv_pbo.as_ref());
    fold_cscv_into_robust_and_sort(out);
    Ok(cscv_pbo)
}

fn sanitize_cscv_slices(slices: usize) -> usize {
    let s = slices.clamp(4, 32);
    if (s & 1) == 0 {
        s
    } else {
        s.saturating_sub(1).max(4)
    }
}

#[derive(Default)]
struct StationaryBootstrapScratch {
    indices: Vec<usize>,
}

impl StationaryBootstrapScratch {
    fn indices<'a>(&'a mut self, n: usize, mean_block: usize, rng: &mut StdRng) -> &'a [usize] {
        stationary_bootstrap_indices_into(&mut self.indices, n, mean_block, rng);
        &self.indices
    }
}

fn stationary_bootstrap_indices_into(
    out: &mut Vec<usize>,
    n: usize,
    mean_block: usize,
    rng: &mut StdRng,
) {
    out.clear();
    if n == 0 {
        return;
    }
    let restart_prob = (1.0 / mean_block.max(1) as f64).clamp(1e-6, 1.0);
    let mut idx = rng.gen_range(0..n);
    out.push(idx);
    while out.len() < n {
        if rng.gen::<f64>() < restart_prob {
            idx = rng.gen_range(0..n);
        } else {
            idx = (idx + 1) % n;
        }
        out.push(idx);
    }
}

#[cfg(test)]
fn stationary_bootstrap_indices(n: usize, mean_block: usize, rng: &mut StdRng) -> Vec<usize> {
    let mut out = Vec::with_capacity(n);
    stationary_bootstrap_indices_into(&mut out, n, mean_block, rng);
    out
}

fn sample_mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().copied().sum::<f64>() / xs.len() as f64
}

fn sample_std(xs: &[f64], mean: f64) -> f64 {
    if xs.len() <= 1 {
        return 0.0;
    }
    let mut sumsq = 0.0;
    for &x in xs {
        let d = x - mean;
        sumsq += d * d;
    }
    (sumsq / (xs.len() - 1) as f64).sqrt()
}

pub fn compute_spa_validation_stats(
    candidate_keys: &[String],
    candidate_slice_returns: &[Vec<f64>],
    benchmark_slice_returns: &[f64],
    cfg: &SpaConfigOptional,
    seed: u64,
    cancel: &AtomicBool,
) -> Option<SpaValidationStats> {
    struct SpaCandidateBootstrapStats {
        diff: Vec<f64>,
        mean: f64,
        std: f64,
        t_stat: f64,
    }

    let candidate_count = candidate_slice_returns.len();
    if candidate_count == 0 || candidate_keys.len() != candidate_count {
        return None;
    }

    let slice_count = candidate_slice_returns
        .iter()
        .map(Vec::len)
        .min()
        .unwrap_or(0)
        .min(benchmark_slice_returns.len())
        .min(cfg.slices.max(4));
    if slice_count < 4 {
        return Some(SpaValidationStats {
            scope: "validation_candidates".to_string(),
            candidate_count,
            slice_count,
            bootstrap_samples: cfg.bootstrap_samples.max(1),
            block_length: cfg.block_length.max(1),
            benchmark: cfg.benchmark.clone(),
            best_candidate_key: String::new(),
            best_candidate_mean_diff: 0.0,
            best_candidate_t_stat: 0.0,
            p_value: 1.0,
            passed: false,
            skipped_reason: Some("spa_skipped_not_enough_slices".to_string()),
        });
    }

    let benchmark = &benchmark_slice_returns[..slice_count];
    let mut diffs: Vec<SpaCandidateBootstrapStats> = Vec::with_capacity(candidate_count);
    let mut best_idx = 0usize;
    let mut best_mean = f64::NEG_INFINITY;
    let sqrt_n = (slice_count as f64).sqrt();

    for (idx, candidate) in candidate_slice_returns.iter().enumerate() {
        let candidate = &candidate[..slice_count];
        let diff: Vec<f64> = candidate
            .iter()
            .zip(benchmark.iter())
            .map(|(cand, bench)| *cand - *bench)
            .collect();
        let mean = sample_mean(&diff);
        let std = sample_std(&diff, mean).max(1e-12);
        let t_stat = (sqrt_n * mean / std).max(0.0);
        if mean > best_mean {
            best_mean = mean;
            best_idx = idx;
        }
        diffs.push(SpaCandidateBootstrapStats {
            diff,
            mean,
            std,
            t_stat,
        });
    }

    let observed_stat = diffs
        .iter()
        .map(|stats| stats.t_stat)
        .fold(0.0_f64, f64::max);
    let bootstrap_samples = cfg.bootstrap_samples.max(100);
    let block_length = cfg.block_length.max(1);
    let mut exceed = 0usize;
    let mut rng = StdRng::seed_from_u64(seed ^ 0x5eed_5aa5_u64);
    let mut stationary_scratch = StationaryBootstrapScratch::default();

    for _ in 0..bootstrap_samples {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let indices = stationary_scratch.indices(slice_count, block_length, &mut rng);
        let mut boot_stat = 0.0_f64;
        for diff in &diffs {
            let mut centered_sum = 0.0_f64;
            for &sample_idx in indices {
                centered_sum += diff.diff[sample_idx] - diff.mean;
            }
            let centered_mean = centered_sum / slice_count as f64;
            let t_star = (sqrt_n * centered_mean / diff.std).max(0.0);
            if t_star > boot_stat {
                boot_stat = t_star;
            }
        }
        if boot_stat >= observed_stat {
            exceed = exceed.saturating_add(1);
        }
    }

    let p_value = exceed as f64 / bootstrap_samples as f64;
    Some(SpaValidationStats {
        scope: "validation_candidates".to_string(),
        candidate_count,
        slice_count,
        bootstrap_samples,
        block_length,
        benchmark: cfg.benchmark.clone(),
        best_candidate_key: candidate_keys.get(best_idx).cloned().unwrap_or_default(),
        best_candidate_mean_diff: best_mean,
        best_candidate_t_stat: diffs.get(best_idx).map(|stats| stats.t_stat).unwrap_or(0.0),
        p_value,
        passed: p_value <= 0.05 && observed_stat.is_finite() && observed_stat > 0.0,
        skipped_reason: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation_pipeline::ValidationRecommendation;
    use crate::validation_shared::{
        ValidationPriceAdjustmentState, ValidationRealismContext, ValidationRealismEvidenceStatus,
    };
    use rand::Rng;
    use std::sync::atomic::AtomicBool;
    use ta_strategies::double_ma::Metrics;

    #[derive(Clone, Debug)]
    struct DummyCandidate {
        in_sample: Metrics,
        psr: Option<f64>,
        dsr: Option<f64>,
        track_record_bars: usize,
        min_track_record_length_bars: Option<usize>,
        robust_score: f64,
        robust_pareto: bool,
        cscv_pbo: Option<CscvCandidatePboStats>,
        validation_verdict: ValidationCandidateVerdict,
    }

    impl ValidationPostCandidate for DummyCandidate {
        fn in_sample_metrics(&self) -> &Metrics {
            &self.in_sample
        }

        fn iterative_in_sample_metrics(&self) -> Option<&Metrics> {
            None
        }

        fn walk_forward_stats(&self) -> Option<&WalkForwardStats> {
            None
        }

        fn cscv_oos_metric_p50(&self) -> Option<f64> {
            self.cscv_pbo.as_ref().and_then(|v| v.oos_metric_p50)
        }

        fn cscv_oos_rank_frac_p50(&self) -> Option<f64> {
            self.cscv_pbo.as_ref().and_then(|v| v.oos_rank_frac_p50)
        }

        fn cscv_selected_frac(&self) -> Option<f64> {
            self.cscv_pbo.as_ref().map(|v| v.is_selected_frac)
        }

        fn psr(&self) -> Option<f64> {
            self.psr
        }

        fn dsr(&self) -> Option<f64> {
            self.dsr
        }

        fn set_psr(&mut self, v: Option<f64>) {
            self.psr = v;
        }

        fn set_dsr(&mut self, v: Option<f64>) {
            self.dsr = v;
        }

        fn track_record_bars(&self) -> usize {
            self.track_record_bars
        }

        fn min_track_record_length_bars(&self) -> Option<usize> {
            self.min_track_record_length_bars
        }

        fn set_track_record_bars(&mut self, v: usize) {
            self.track_record_bars = v;
        }

        fn set_min_track_record_length_bars(&mut self, v: Option<usize>) {
            self.min_track_record_length_bars = v;
        }

        fn robust_score(&self) -> f64 {
            self.robust_score
        }

        fn set_robust_score(&mut self, v: f64) {
            self.robust_score = v;
        }

        fn set_robust_pareto(&mut self, v: bool) {
            self.robust_pareto = v;
        }

        fn set_cscv_pbo(&mut self, v: Option<CscvCandidatePboStats>) {
            self.cscv_pbo = v;
        }

        fn validation_verdict(&self) -> &ValidationCandidateVerdict {
            &self.validation_verdict
        }

        fn set_validation_verdict(&mut self, v: ValidationCandidateVerdict) {
            self.validation_verdict = v;
        }
    }

    #[test]
    fn robust_score_prefers_walk_forward_when_present() {
        let m = Metrics {
            pnl: 0.0,
            sharpe: 1.0,
            max_dd: 0.2,
            trades: 0,
            ..Metrics::default()
        };
        let a = robust_score_from_metrics(&m, None);
        let wf = WalkForwardStats {
            windows: 3,
            effective_windows_upper_bound: Some(3),
            bars_covered: Some(128),
            sharpe_median: 2.0,
            sharpe_worst: 1.0,
            max_dd_median: 0.2,
            max_dd_worst: 0.3,
            max_dd_tail_mean_p95: 0.3,
            pnl_median: 0.1,
            pnl_worst: 0.0,
            pnl_tail_mean_p05: 0.0,
            downside_vol_ratio_median: 0.5,
            downside_vol_ratio_worst: 0.8,
            ulcer_index_median: 0.05,
            ulcer_index_worst: 0.1,
            martin_ratio_median: 1.0,
            martin_ratio_worst: 0.5,
            underwater_fraction_median: 0.4,
            underwater_fraction_worst: 0.6,
            max_underwater_duration_bars_median: 12,
            max_underwater_duration_bars_worst: 18,
            trades_median: 10,
            trades_worst: 5,
        };
        let b = robust_score_from_metrics(&m, Some(&wf));
        assert!((a - 0.6).abs() < 1e-12);
        assert!((b - 1.275).abs() < 1e-12);
    }

    #[test]
    fn apply_candidate_hard_gates_preserves_new_metric_warning_flags() {
        let mut out = vec![DummyCandidate {
            in_sample: Metrics {
                pnl: 0.10,
                sharpe: 1.0,
                volatility: 0.10,
                downside_deviation: 0.095,
                ulcer_index: 0.15,
                martin_ratio: 0.20,
                max_dd: 0.2,
                trades: 32,
                ..Metrics::default()
            },
            psr: Some(0.95),
            dsr: Some(0.90),
            track_record_bars: 256,
            min_track_record_length_bars: Some(64),
            robust_score: 1.0,
            robust_pareto: true,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];

        apply_candidate_hard_gates(&mut out, &ValidationHardGateConfig::default(), None);
        let flags = &out[0].validation_verdict.warning_flags;
        assert!(
            flags.iter().any(|flag| flag == "ulcer_index_high:0.150"),
            "expected ulcer warning in {flags:?}"
        );
        assert!(
            flags.iter().any(|flag| flag == "martin_ratio_weak:0.200"),
            "expected martin warning in {flags:?}"
        );
        assert!(
            flags
                .iter()
                .any(|flag| flag == "downside_vol_ratio_high:0.950"),
            "expected downside ratio warning in {flags:?}"
        );
    }

    #[test]
    fn risk_warning_flags_surface_walk_forward_overlap_adjustment() {
        let flags = risk_warning_flags_from_metrics(
            &Metrics::default(),
            Some(&WalkForwardStats {
                windows: 3,
                effective_windows_upper_bound: Some(1),
                bars_covered: Some(66),
                sharpe_median: 0.0,
                sharpe_worst: 0.0,
                max_dd_median: 0.0,
                max_dd_worst: 0.0,
                max_dd_tail_mean_p95: 0.0,
                pnl_median: 0.0,
                pnl_worst: 0.0,
                pnl_tail_mean_p05: 0.0,
                downside_vol_ratio_median: 0.0,
                downside_vol_ratio_worst: 0.0,
                ulcer_index_median: 0.0,
                ulcer_index_worst: 0.0,
                martin_ratio_median: 0.0,
                martin_ratio_worst: 0.0,
                underwater_fraction_median: 0.0,
                underwater_fraction_worst: 0.0,
                max_underwater_duration_bars_median: 0,
                max_underwater_duration_bars_worst: 0,
                trades_median: 0,
                trades_worst: 0,
            }),
        );

        assert!(flags
            .iter()
            .any(|flag| flag == "walk_forward_overlap_adjusted:1/3"));
    }

    #[test]
    fn apply_candidate_hard_gates_marks_cscv_rank_gate_unavailable_when_cscv_missing() {
        let gates = ValidationHardGateConfig {
            min_cscv_oos_rank_frac: Some(0.60),
            ..ValidationHardGateConfig::default()
        };
        let mut out = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.0,
                ..Metrics::default()
            },
            psr: Some(0.95),
            dsr: Some(0.90),
            track_record_bars: 256,
            min_track_record_length_bars: Some(64),
            robust_score: 1.0,
            robust_pareto: true,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];

        apply_candidate_hard_gates(&mut out, &gates, None);

        let verdict = out[0].validation_verdict();
        assert!(!verdict.passed);
        assert!(verdict
            .fail_reasons
            .iter()
            .any(|reason| reason == "cscv_rank_gate_requested_but_cscv_unavailable"));
    }

    #[test]
    fn apply_candidate_hard_gates_marks_cscv_metric_gate_unavailable_when_cscv_missing() {
        let gates = ValidationHardGateConfig {
            min_cscv_oos_metric: Some(0.25),
            ..ValidationHardGateConfig::default()
        };
        let mut out = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.0,
                ..Metrics::default()
            },
            psr: Some(0.95),
            dsr: Some(0.90),
            track_record_bars: 256,
            min_track_record_length_bars: Some(64),
            robust_score: 1.0,
            robust_pareto: true,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];

        apply_candidate_hard_gates(&mut out, &gates, None);

        let verdict = out[0].validation_verdict();
        assert!(!verdict.passed);
        assert!(verdict
            .fail_reasons
            .iter()
            .any(|reason| reason == "cscv_metric_gate_requested_but_cscv_unavailable"));
    }

    #[test]
    fn fold_cscv_metric_uses_oos_as_lower_bound() {
        assert!((fold_cscv_metric(1.0, Some(0.25)) - 0.25).abs() < 1e-12);
        assert!((fold_cscv_metric(1.0, Some(2.0)) - 1.0).abs() < 1e-12);
        assert!((fold_cscv_metric(1.0, Some(f64::NAN)) - 1.0).abs() < 1e-12);
        assert!((fold_cscv_metric(1.0, None) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn compute_probability_metrics_populates_track_record_and_mintrl() {
        let mut out = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.2,
                ..Metrics::default()
            },
            psr: None,
            dsr: None,
            track_record_bars: 0,
            min_track_record_length_bars: None,
            robust_score: 1.2,
            robust_pareto: false,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        compute_probability_metrics(&mut out, &[120]);
        assert_eq!(out[0].track_record_bars, 120);
        assert!(out[0].min_track_record_length_bars.is_some());
    }

    #[test]
    fn compute_probability_metrics_falls_back_to_two_obs_when_sample_obs_is_short_or_too_small() {
        let mut out = vec![
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 1.20,
                    ..Metrics::default()
                },
                psr: None,
                dsr: None,
                track_record_bars: 0,
                min_track_record_length_bars: None,
                robust_score: 1.2,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 1.00,
                    ..Metrics::default()
                },
                psr: None,
                dsr: None,
                track_record_bars: 0,
                min_track_record_length_bars: None,
                robust_score: 1.0,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 0.80,
                    ..Metrics::default()
                },
                psr: None,
                dsr: None,
                track_record_bars: 0,
                min_track_record_length_bars: None,
                robust_score: 0.8,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
        ];

        compute_probability_metrics(&mut out, &[0, 7]);

        assert_eq!(out[0].track_record_bars, 0);
        assert_eq!(out[1].track_record_bars, 7);
        assert_eq!(out[2].track_record_bars, 2);
        assert!(out.iter().all(|cand| cand.psr.is_some()));
        assert!(out.iter().all(|cand| cand.dsr.is_some()));
    }

    #[test]
    fn compute_probability_metrics_with_effective_counts_matches_legacy_when_overlap_is_absent() {
        let mut legacy = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.20,
                ..Metrics::default()
            },
            psr: None,
            dsr: None,
            track_record_bars: 0,
            min_track_record_length_bars: None,
            robust_score: 1.2,
            robust_pareto: false,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        let mut overlap = legacy.clone();

        compute_probability_metrics(&mut legacy, &[120]);
        compute_probability_metrics_with_effective_counts(
            &mut overlap,
            &[120],
            Some(&[120]),
            Some(1),
        );

        assert_eq!(legacy[0].track_record_bars, overlap[0].track_record_bars);
        assert_eq!(
            legacy[0].min_track_record_length_bars,
            overlap[0].min_track_record_length_bars
        );
        assert_eq!(legacy[0].psr, overlap[0].psr);
        assert_eq!(legacy[0].dsr, overlap[0].dsr);
    }

    #[test]
    fn compute_probability_metrics_preserves_raw_track_record_bars_when_effective_counts_shrink() {
        let mut baseline = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.20,
                ..Metrics::default()
            },
            psr: None,
            dsr: None,
            track_record_bars: 0,
            min_track_record_length_bars: None,
            robust_score: 1.2,
            robust_pareto: false,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        let mut overlap = baseline.clone();

        compute_probability_metrics(&mut baseline, &[120]);
        compute_probability_metrics_with_effective_counts(
            &mut overlap,
            &[120],
            Some(&[40]),
            Some(1),
        );

        assert_eq!(overlap[0].track_record_bars, 120);
        assert!(overlap[0].psr.expect("overlap psr") < baseline[0].psr.expect("baseline psr"));
        assert!(overlap[0].dsr.expect("overlap dsr") <= baseline[0].dsr.expect("baseline dsr"));
        assert!(
            overlap[0]
                .min_track_record_length_bars
                .expect("overlap mintrl")
                > baseline[0]
                    .min_track_record_length_bars
                    .expect("baseline mintrl")
        );
    }

    #[test]
    fn probability_metrics_use_full_optimization_trial_count_when_available() {
        let mut retained_only = vec![
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 1.20,
                    ..Metrics::default()
                },
                psr: None,
                dsr: None,
                track_record_bars: 0,
                min_track_record_length_bars: None,
                robust_score: 1.2,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 0.60,
                    ..Metrics::default()
                },
                psr: None,
                dsr: None,
                track_record_bars: 0,
                min_track_record_length_bars: None,
                robust_score: 0.6,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 0.20,
                    ..Metrics::default()
                },
                psr: None,
                dsr: None,
                track_record_bars: 0,
                min_track_record_length_bars: None,
                robust_score: 0.2,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
        ];
        let mut full_search = retained_only.clone();

        compute_probability_metrics(&mut retained_only, &[128, 128, 128]);
        compute_probability_metrics_with_research_context(
            &mut full_search,
            &[128, 128, 128],
            None,
            None,
            Some(500),
            None,
        );

        assert_eq!(retained_only[0].psr, full_search[0].psr);
        assert!(
            full_search[0].dsr.expect("full-search dsr")
                < retained_only[0].dsr.expect("retained-only dsr")
        );
    }

    #[test]
    fn probability_metrics_use_comparable_full_search_sharpe_moments_and_finite_count() {
        let mut candidates = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.20,
                ..Metrics::default()
            },
            psr: None,
            dsr: None,
            track_record_bars: 0,
            min_track_record_length_bars: None,
            robust_score: 1.2,
            robust_pareto: false,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        let summary = ta_optimizer::SearchDistributionSummary {
            metric_surface_kind: "core17".to_string(),
            sharpe_metric_id: ta_optimizer::DOUBLE_MA_SHARPE_METRIC_ID.to_string(),
            attempted_count: 600,
            unique_count: 550,
            feasible_count: 510,
            finite_sharpe_count: 500,
            finite_sharpe_mean: Some(0.10),
            finite_sharpe_sample_std: Some(0.80),
        };

        compute_probability_metrics_with_search_distribution(
            &mut candidates,
            &[128],
            None,
            None,
            Some(999),
            Some(&summary),
            Some(ta_optimizer::DOUBLE_MA_SHARPE_METRIC_ID),
            None,
        );

        let expected = crate::validation_stats::compute_psr_dsr_with_effective_counts_and_moments(
            1.20,
            128,
            Some(128),
            500,
            None,
            0.10,
            0.80,
            0.0,
            3.0,
        );
        assert_eq!(candidates[0].psr, expected.0);
        assert_eq!(candidates[0].dsr, expected.1);
    }

    #[test]
    fn probability_metrics_ignore_incompatible_full_search_distribution() {
        let candidate = DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.20,
                ..Metrics::default()
            },
            psr: None,
            dsr: None,
            track_record_bars: 0,
            min_track_record_length_bars: None,
            robust_score: 1.2,
            robust_pareto: false,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        };
        let mut baseline = vec![candidate.clone()];
        let mut incompatible = vec![candidate];
        let summary = ta_optimizer::SearchDistributionSummary {
            metric_surface_kind: "core17".to_string(),
            sharpe_metric_id: "different_sharpe_semantics".to_string(),
            attempted_count: 600,
            unique_count: 550,
            feasible_count: 510,
            finite_sharpe_count: 500,
            finite_sharpe_mean: Some(-10.0),
            finite_sharpe_sample_std: Some(12.0),
        };

        compute_probability_metrics_with_research_context(
            &mut baseline,
            &[128],
            None,
            None,
            Some(17),
            None,
        );
        compute_probability_metrics_with_search_distribution(
            &mut incompatible,
            &[128],
            None,
            None,
            Some(17),
            Some(&summary),
            Some(ta_optimizer::DOUBLE_MA_SHARPE_METRIC_ID),
            None,
        );

        assert_eq!(incompatible[0].psr, baseline[0].psr);
        assert_eq!(incompatible[0].dsr, baseline[0].dsr);
    }

    #[test]
    fn probability_metrics_use_observed_return_moments_when_available() {
        let mut normal_fallback = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.20,
                ..Metrics::default()
            },
            psr: None,
            dsr: None,
            track_record_bars: 0,
            min_track_record_length_bars: None,
            robust_score: 1.2,
            robust_pareto: false,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        let mut observed = normal_fallback.clone();
        let moments = [Some(crate::sharpe_stats::ReturnDistributionMoments {
            observations: 128,
            skewness: -1.0,
            kurtosis: 9.0,
        })];

        compute_probability_metrics(&mut normal_fallback, &[128]);
        compute_probability_metrics_with_research_context(
            &mut observed,
            &[128],
            None,
            None,
            None,
            Some(&moments),
        );

        assert!(
            observed[0].psr.expect("observed psr") < normal_fallback[0].psr.expect("normal psr")
        );
        assert!(
            observed[0]
                .min_track_record_length_bars
                .expect("observed mintrl")
                > normal_fallback[0]
                    .min_track_record_length_bars
                    .expect("normal mintrl")
        );
    }

    #[test]
    fn compute_walk_forward_stats_clamps_window_and_step_to_minimums() {
        let cfg = WalkForwardConfigOptional {
            window_bars: 0,
            step_bars: 0,
        };
        let mut observed_windows = Vec::new();

        let stats = compute_walk_forward_stats(66, 0, &cfg, |start, end| {
            observed_windows.push((start, end));
            Metrics {
                sharpe: (end - start) as f64,
                trades: 1,
                ..Metrics::default()
            }
        });

        assert_eq!(observed_windows, vec![(0, 64), (1, 65), (2, 66)]);
        assert_eq!(stats.windows, 3);
        assert_eq!(stats.effective_windows_upper_bound, Some(1));
        assert_eq!(stats.bars_covered, Some(66));
    }

    #[test]
    fn compute_walk_forward_stats_rejects_extreme_start_without_overflow() {
        let cfg = WalkForwardConfigOptional {
            window_bars: 64,
            step_bars: 1,
        };

        let stats = compute_walk_forward_stats(100, usize::MAX, &cfg, |_start, _end| {
            panic!("start_t beyond data length must not evaluate windows");
        });

        assert_eq!(stats.windows, 0);
        assert_eq!(stats.effective_windows_upper_bound, Some(0));
        assert_eq!(stats.bars_covered, Some(0));
    }

    #[test]
    fn mark_robust_pareto_uses_same_dominance_rules_with_walk_forward_data() {
        let mut out = vec![
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 1.5,
                    max_dd: 0.18,
                    ulcer_index: 0.10,
                    martin_ratio: 0.9,
                    ..Metrics::default()
                },
                psr: None,
                dsr: None,
                track_record_bars: 0,
                min_track_record_length_bars: None,
                robust_score: 0.0,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 1.0,
                    max_dd: 0.20,
                    ulcer_index: 0.14,
                    martin_ratio: 0.7,
                    ..Metrics::default()
                },
                psr: None,
                dsr: None,
                track_record_bars: 0,
                min_track_record_length_bars: None,
                robust_score: 0.0,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 1.1,
                    max_dd: 0.24,
                    ulcer_index: 0.20,
                    martin_ratio: 0.6,
                    ..Metrics::default()
                },
                psr: None,
                dsr: None,
                track_record_bars: 0,
                min_track_record_length_bars: None,
                robust_score: 0.0,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
        ];

        mark_robust_pareto(&mut out);

        assert!(
            out[0].robust_pareto,
            "best candidate should remain on Pareto frontier"
        );
        assert!(
            !out[1].robust_pareto,
            "dominated candidate should be excluded"
        );
        assert!(
            !out[2].robust_pareto,
            "clearly weaker candidate should be excluded"
        );
    }

    #[test]
    fn normalized_param_drift_from_parsed_key_values_matches_text_path() {
        let left = r#"{"fast":8,"nested":{"alpha":1.0,"beta":true}}"#;
        let right = r#"{"fast":13,"nested":{"alpha":1.5,"beta":true}}"#;
        let left_value = serde_json::from_str::<JsonValue>(left).expect("left json");
        let right_value = serde_json::from_str::<JsonValue>(right).expect("right json");

        let text = normalized_param_drift_from_key_text(left, right);
        let parsed = normalized_param_drift_from_parsed_key_values(
            left,
            right,
            Some(&left_value),
            Some(&right_value),
        );

        assert!((text - parsed).abs() < 1e-12);
    }

    #[test]
    fn select_quantile_in_place_matches_sorted_quantile() {
        let values = vec![3.0, f64::NAN, -1.0, 2.5, 7.0, 2.5];
        for q in [0.05, 0.5, 0.95] {
            let mut unsorted = values.clone();
            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.total_cmp(b));
            let expected = crate::stats_common::quantile(&sorted, q);
            let actual = select_quantile_in_place(&mut unsorted, q);
            assert!(
                expected.to_bits() == actual.to_bits(),
                "q={q} expected={expected:?} actual={actual:?}"
            );
        }
    }

    #[test]
    fn select_tail_mean_helpers_match_sorted_tail_means() {
        let values = vec![5.0, 1.0, 4.0, 2.0, 3.0];

        let mut lower = values.clone();
        let lower_tail = select_lower_tail_mean_in_place(&mut lower, 0.25);
        assert!((lower_tail - 1.5).abs() <= 1.0e-12);

        let mut upper = values;
        let upper_tail = select_upper_tail_mean_in_place(&mut upper, 0.75);
        assert!((upper_tail - 4.5).abs() <= 1.0e-12);
    }

    #[test]
    fn compute_walk_forward_stats_retains_tail_means_and_underwater_persistence() {
        let pnls: Vec<f64> = (-10..10).map(|value| value as f64).collect();
        let dds: Vec<f64> = (1..=20).map(|value| value as f64 / 100.0).collect();
        let underwater_fracs: Vec<f64> = (1..=20).map(|value| value as f64 / 20.0).collect();
        let underwater_durations: Vec<u32> = (1..=20).collect();
        let metrics: Vec<Metrics> = (0..20)
            .map(|idx| Metrics {
                sharpe: idx as f64 / 10.0,
                max_dd: dds[idx],
                pnl: pnls[idx],
                ulcer_index: dds[idx] / 2.0,
                martin_ratio: 1.0 - dds[idx],
                trades: underwater_durations[idx],
                underwater_fraction: underwater_fracs[idx],
                max_underwater_duration_bars: underwater_durations[idx],
                ..Metrics::default()
            })
            .collect();
        let cfg = WalkForwardConfigOptional {
            window_bars: 64,
            step_bars: 1,
        };

        let mut next_metric = 0usize;
        let stats = compute_walk_forward_stats(83, 0, &cfg, |_start, _end| {
            let metric = metrics[next_metric].clone();
            next_metric = next_metric.saturating_add(1);
            metric
        });

        let mut expected_pnls = pnls.clone();
        let mut expected_dds = dds.clone();
        let mut expected_underwater_fracs = underwater_fracs.clone();
        let mut expected_underwater_durations = underwater_durations.clone();

        assert_eq!(stats.windows, 20);
        assert_eq!(stats.effective_windows_upper_bound, Some(1));
        assert_eq!(stats.bars_covered, Some(83));
        assert!(
            (stats.pnl_tail_mean_p05
                - crate::stats_common::lower_tail_mean_in_place(&mut expected_pnls, 0.05))
            .abs()
                <= 1.0e-12
        );
        assert!(
            (stats.max_dd_tail_mean_p95
                - crate::stats_common::upper_tail_mean_in_place(&mut expected_dds, 0.95))
            .abs()
                <= 1.0e-12
        );
        assert!(
            (stats.underwater_fraction_median
                - crate::stats_common::median_in_place(&mut expected_underwater_fracs))
            .abs()
                <= 1.0e-12
        );
        assert_eq!(
            stats.underwater_fraction_worst,
            crate::stats_common::worst_max(&underwater_fracs)
        );
        assert_eq!(
            stats.max_underwater_duration_bars_median,
            crate::stats_common::median_u32_in_place(&mut expected_underwater_durations)
        );
        assert_eq!(
            stats.max_underwater_duration_bars_worst,
            crate::stats_common::worst_max_u32(&underwater_durations)
        );
    }

    #[test]
    fn select_median_u32_in_place_matches_sorted_path() {
        let mut values = vec![9_u32, 2, 4, 2, 7, 1, 6];
        let expected = crate::stats_common::median_u32(&values);
        let actual = select_median_u32_in_place(&mut values);
        assert_eq!(actual, expected);
    }

    #[test]
    fn compute_perturb_stats_matches_summary_function() {
        let cfg = PerturbConfig {
            iterations: 4,
            pct_min: -0.1,
            pct_max: 0.1,
            flip_bool_prob: 0.0,
        };
        let cancel = AtomicBool::new(false);
        let expected_metrics = vec![
            Metrics {
                sharpe: 1.0,
                max_dd: 0.20,
                pnl: 0.10,
                ..Metrics::default()
            },
            Metrics {
                sharpe: 0.5,
                max_dd: 0.30,
                pnl: -0.05,
                ..Metrics::default()
            },
            Metrics {
                sharpe: 1.5,
                max_dd: 0.10,
                pnl: 0.20,
                ..Metrics::default()
            },
            Metrics {
                sharpe: -0.25,
                max_dd: 0.40,
                pnl: 0.00,
                ..Metrics::default()
            },
        ];

        let expected_sharpes: Vec<f64> = expected_metrics.iter().map(|m| m.sharpe).collect();
        let expected_dds: Vec<f64> = expected_metrics.iter().map(|m| m.max_dd).collect();
        let expected_pnls: Vec<f64> = expected_metrics.iter().map(|m| m.pnl).collect();
        let expected_pnl_pos = expected_metrics.iter().filter(|m| m.pnl > 0.0).count();
        let expected = summarize_perturb_stats(
            &cfg,
            cfg.iterations,
            &expected_sharpes,
            &expected_dds,
            &expected_pnls,
            expected_pnl_pos,
        );

        let mut idx = 0usize;
        let mut rng = StdRng::seed_from_u64(7);
        let actual = compute_perturb_stats(
            &0usize,
            &cfg,
            &cancel,
            &mut rng,
            |base, _, _| *base,
            |_| {
                let metric = expected_metrics
                    .get(idx)
                    .cloned()
                    .expect("expected perturb metric");
                idx = idx.saturating_add(1);
                Ok(metric)
            },
        )
        .expect("perturb stats")
        .expect("perturb stats present");

        let expected = serde_json::to_value(expected).expect("serialize expected");
        let actual = serde_json::to_value(actual).expect("serialize actual");
        assert_eq!(actual, expected);
    }

    #[test]
    fn stationary_bootstrap_indices_into_matches_allocating_helper_across_draws() {
        let mut scratch = StationaryBootstrapScratch::default();
        let mut into_rng = StdRng::seed_from_u64(19);
        let mut alloc_rng = StdRng::seed_from_u64(19);

        for _ in 0..16 {
            let expected = stationary_bootstrap_indices(12, 3, &mut alloc_rng);
            let actual = scratch.indices(12, 3, &mut into_rng).to_vec();
            assert_eq!(actual, expected);
        }

        assert!(scratch.indices.iter().all(|&idx| idx < 12));
    }

    #[test]
    fn trade_bootstrap_honors_stationary_block_len_override() {
        let trades = vec![
            Trade {
                entry_t: 0,
                exit_t: 1,
                direction: 1,
                pnl: 0.05,
                bars: 1,
                open: false,
            },
            Trade {
                entry_t: 2,
                exit_t: 3,
                direction: 1,
                pnl: -0.03,
                bars: 1,
                open: false,
            },
            Trade {
                entry_t: 4,
                exit_t: 5,
                direction: 1,
                pnl: 0.08,
                bars: 1,
                open: false,
            },
            Trade {
                entry_t: 6,
                exit_t: 7,
                direction: 1,
                pnl: -0.02,
                bars: 1,
                open: false,
            },
            Trade {
                entry_t: 8,
                exit_t: 9,
                direction: 1,
                pnl: 0.04,
                bars: 1,
                open: false,
            },
        ];
        let cfg = TradeBootstrapConfig {
            simulations: 128,
            stationary_mean_block_len: Some(3),
        };
        let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(7);
        let stats = compute_trade_bootstrap_stats(&trades, &cfg, &mut rng);
        assert_eq!(stats.simulations, 128);
        assert_eq!(stats.trade_count, 5);
        assert_eq!(stats.stationary_mean_block_len, Some(3));
        assert!(stats.pnl_tail_mean_p05 <= stats.pnl_p05 + 1.0e-12);
        assert!(stats.max_dd_tail_mean_p95 + 1.0e-12 >= stats.max_dd_p95);
        assert!(stats.stationary_pnl_p05.is_some());
        assert!(stats.stationary_pnl_tail_mean_p05.is_some());
        assert!(stats.stationary_max_dd_p95.is_some());
        assert!(stats.stationary_max_dd_tail_mean_p95.is_some());
    }

    #[test]
    fn trade_bootstrap_is_deterministic_with_same_seed() {
        let trades = vec![
            Trade {
                entry_t: 0,
                exit_t: 1,
                direction: 1,
                pnl: 0.04,
                bars: 1,
                open: false,
            },
            Trade {
                entry_t: 2,
                exit_t: 4,
                direction: 1,
                pnl: -0.02,
                bars: 2,
                open: false,
            },
            Trade {
                entry_t: 5,
                exit_t: 6,
                direction: -1,
                pnl: 0.07,
                bars: 1,
                open: false,
            },
            Trade {
                entry_t: 7,
                exit_t: 9,
                direction: 1,
                pnl: -0.01,
                bars: 2,
                open: false,
            },
        ];
        let cfg = TradeBootstrapConfig {
            simulations: 64,
            stationary_mean_block_len: Some(2),
        };
        let mut rng_a = StdRng::seed_from_u64(42);
        let mut rng_b = StdRng::seed_from_u64(42);
        let stats_a = compute_trade_bootstrap_stats(&trades, &cfg, &mut rng_a);
        let stats_b = compute_trade_bootstrap_stats(&trades, &cfg, &mut rng_b);

        let stats_a = serde_json::to_value(&stats_a).expect("serialize trade bootstrap stats");
        let stats_b = serde_json::to_value(&stats_b).expect("serialize trade bootstrap stats");
        assert_eq!(stats_a, stats_b);
    }

    #[test]
    fn trade_bootstrap_iid_only_mode_skips_stationary_stats() {
        let trades = vec![
            Trade {
                entry_t: 0,
                exit_t: 1,
                direction: 1,
                pnl: 0.05,
                bars: 1,
                open: false,
            },
            Trade {
                entry_t: 2,
                exit_t: 4,
                direction: 1,
                pnl: -0.05,
                bars: 2,
                open: false,
            },
        ];
        let cfg = TradeBootstrapConfig {
            simulations: 32,
            stationary_mean_block_len: Some(0),
        };
        let mut rng = StdRng::seed_from_u64(7);
        let stats = compute_trade_bootstrap_stats(&trades, &cfg, &mut rng);
        assert_eq!(stats.trade_count, 2);
        assert!(stats.pnl_p50.is_finite());
        assert!(stats.pnl_tail_mean_p05.is_finite());
        assert!(stats.max_dd_tail_mean_p95.is_finite());
        assert!(stats.stationary_mean_block_len.is_none());
        assert!(stats.stationary_pnl_p05.is_none());
        assert!(stats.stationary_pnl_tail_mean_p05.is_none());
        assert!(stats.stationary_max_dd_p95.is_none());
        assert!(stats.stationary_max_dd_tail_mean_p95.is_none());
    }

    #[test]
    fn monte_carlo_stats_include_modes_histograms_and_legacy_projection() {
        let trades = vec![
            Trade {
                entry_t: 0,
                exit_t: 1,
                direction: 1,
                pnl: 0.05,
                bars: 1,
                open: false,
            },
            Trade {
                entry_t: 2,
                exit_t: 3,
                direction: 1,
                pnl: -0.02,
                bars: 1,
                open: false,
            },
            Trade {
                entry_t: 4,
                exit_t: 6,
                direction: -1,
                pnl: 0.03,
                bars: 2,
                open: false,
            },
        ];
        let cfg = MonteCarloValidationConfig {
            schema_version: 1,
            simulations: 48,
            modes: vec![MonteCarloMode::TradeIid, MonteCarloMode::TradeStationary],
            stationary_mean_block_len: Some(2),
            distribution_bins: 12,
            tail_alpha: 0.05,
            warn: Some(MonteCarloThresholdConfig {
                min_trade_count: Some(4),
                ..MonteCarloThresholdConfig::default()
            }),
            gate: None,
        };
        let mut rng = StdRng::seed_from_u64(123);
        let stats = compute_monte_carlo_validation_stats(&trades, &cfg, &mut rng);

        assert_eq!(stats.simulations, 48);
        assert_eq!(stats.trade_count, 3);
        assert!(stats.modes.contains_key(MonteCarloMode::TradeIid.as_key()));
        assert!(stats
            .modes
            .contains_key(MonteCarloMode::TradeStationary.as_key()));
        let iid = stats
            .modes
            .get(MonteCarloMode::TradeIid.as_key())
            .expect("iid mode");
        assert_eq!(
            iid.pnl_histogram
                .as_ref()
                .expect("pnl histogram")
                .counts
                .len(),
            12
        );
        assert_eq!(
            iid.drawdown_histogram
                .as_ref()
                .expect("drawdown histogram")
                .counts
                .len(),
            12
        );
        assert!(stats
            .advisory
            .warning_flags
            .iter()
            .any(|flag| flag.starts_with("monte_carlo_low_trade_count:")));

        let legacy = stats.to_trade_bootstrap_stats();
        assert_eq!(legacy.simulations, stats.simulations);
        assert_eq!(legacy.trade_count, stats.trade_count);
        assert!(legacy.stationary_pnl_p05.is_some());
        assert!(legacy.stationary_max_dd_p95.is_some());
    }

    #[test]
    fn spa_validation_is_deterministic_with_same_seed() {
        let candidate_keys = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let candidate_slice_returns = vec![
            vec![0.03, 0.01, 0.02, 0.04, 0.01, 0.02],
            vec![0.01, 0.02, 0.00, 0.01, 0.02, 0.01],
            vec![0.00, -0.01, 0.01, 0.00, 0.01, -0.01],
        ];
        let benchmark_slice_returns = vec![0.0, 0.01, 0.0, 0.01, 0.0, 0.0];
        let cfg = SpaConfigOptional {
            slices: 6,
            bootstrap_samples: 128,
            block_length: 2,
            benchmark: crate::validation_shared::SpaBenchmarkKind::BuyAndHold,
        };
        let cancel = AtomicBool::new(false);

        let stats_a = compute_spa_validation_stats(
            &candidate_keys,
            &candidate_slice_returns,
            &benchmark_slice_returns,
            &cfg,
            99,
            &cancel,
        );
        let stats_b = compute_spa_validation_stats(
            &candidate_keys,
            &candidate_slice_returns,
            &benchmark_slice_returns,
            &cfg,
            99,
            &cancel,
        );

        let stats_a = serde_json::to_value(&stats_a).expect("serialize SPA stats");
        let stats_b = serde_json::to_value(&stats_b).expect("serialize SPA stats");
        assert_eq!(stats_a, stats_b);
    }

    #[test]
    fn spa_validation_matches_legacy_bootstrap_statistics() {
        fn compute_spa_validation_stats_legacy(
            candidate_keys: &[String],
            candidate_slice_returns: &[Vec<f64>],
            benchmark_slice_returns: &[f64],
            cfg: &SpaConfigOptional,
            seed: u64,
        ) -> Option<SpaValidationStats> {
            let candidate_count = candidate_slice_returns.len();
            if candidate_count == 0 || candidate_keys.len() != candidate_count {
                return None;
            }

            let slice_count = candidate_slice_returns
                .iter()
                .map(Vec::len)
                .min()
                .unwrap_or(0)
                .min(benchmark_slice_returns.len())
                .min(cfg.slices.max(4));
            if slice_count < 4 {
                return Some(SpaValidationStats {
                    scope: "validation_candidates".to_string(),
                    candidate_count,
                    slice_count,
                    bootstrap_samples: cfg.bootstrap_samples.max(1),
                    block_length: cfg.block_length.max(1),
                    benchmark: cfg.benchmark.clone(),
                    best_candidate_key: String::new(),
                    best_candidate_mean_diff: 0.0,
                    best_candidate_t_stat: 0.0,
                    p_value: 1.0,
                    passed: false,
                    skipped_reason: Some("spa_skipped_not_enough_slices".to_string()),
                });
            }

            let benchmark = &benchmark_slice_returns[..slice_count];
            let mut diffs: Vec<Vec<f64>> = Vec::with_capacity(candidate_count);
            let mut best_idx = 0usize;
            let mut best_mean = f64::NEG_INFINITY;
            let mut t_stats: Vec<f64> = Vec::with_capacity(candidate_count);
            let sqrt_n = (slice_count as f64).sqrt();

            for (idx, candidate) in candidate_slice_returns.iter().enumerate() {
                let candidate = &candidate[..slice_count];
                let diff: Vec<f64> = candidate
                    .iter()
                    .zip(benchmark.iter())
                    .map(|(cand, bench)| *cand - *bench)
                    .collect();
                let mean = sample_mean(&diff);
                let std = sample_std(&diff, mean).max(1e-12);
                let t_stat = (sqrt_n * mean / std).max(0.0);
                if mean > best_mean {
                    best_mean = mean;
                    best_idx = idx;
                }
                diffs.push(diff);
                t_stats.push(t_stat);
            }

            let observed_stat = t_stats.iter().copied().fold(0.0_f64, f64::max);
            let bootstrap_samples = cfg.bootstrap_samples.max(100);
            let block_length = cfg.block_length.max(1);
            let mut exceed = 0usize;
            let mut rng = StdRng::seed_from_u64(seed ^ 0x5eed_5aa5_u64);
            let mut stationary_scratch = StationaryBootstrapScratch::default();

            for _ in 0..bootstrap_samples {
                let indices = stationary_scratch.indices(slice_count, block_length, &mut rng);
                let mut boot_stat = 0.0_f64;
                for diff in &diffs {
                    let mean = sample_mean(diff);
                    let std = sample_std(diff, mean).max(1e-12);
                    let mut centered_sum = 0.0_f64;
                    for &sample_idx in indices {
                        centered_sum += diff[sample_idx] - mean;
                    }
                    let centered_mean = centered_sum / slice_count as f64;
                    let t_star = (sqrt_n * centered_mean / std).max(0.0);
                    if t_star > boot_stat {
                        boot_stat = t_star;
                    }
                }
                if boot_stat >= observed_stat {
                    exceed = exceed.saturating_add(1);
                }
            }

            let p_value = exceed as f64 / bootstrap_samples as f64;
            Some(SpaValidationStats {
                scope: "validation_candidates".to_string(),
                candidate_count,
                slice_count,
                bootstrap_samples,
                block_length,
                benchmark: cfg.benchmark.clone(),
                best_candidate_key: candidate_keys.get(best_idx).cloned().unwrap_or_default(),
                best_candidate_mean_diff: best_mean,
                best_candidate_t_stat: t_stats.get(best_idx).copied().unwrap_or(0.0),
                p_value,
                passed: p_value <= 0.05 && observed_stat.is_finite() && observed_stat > 0.0,
                skipped_reason: None,
            })
        }

        let candidate_keys = vec![
            "alpha".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
            "delta".to_string(),
        ];
        let candidate_slice_returns = vec![
            vec![0.05, 0.01, 0.03, 0.04, -0.01, 0.02, 0.01, 0.00],
            vec![0.02, 0.02, 0.01, 0.01, 0.00, 0.02, 0.03, 0.01],
            vec![-0.01, 0.01, 0.00, 0.02, -0.02, 0.01, 0.00, 0.01],
            vec![0.03, 0.00, 0.04, 0.01, 0.02, -0.01, 0.02, 0.03],
        ];
        let benchmark_slice_returns = vec![0.0, 0.01, 0.0, 0.01, 0.0, 0.0, 0.01, 0.0];
        let cfg = SpaConfigOptional {
            slices: 8,
            bootstrap_samples: 192,
            block_length: 3,
            benchmark: crate::validation_shared::SpaBenchmarkKind::BuyAndHold,
        };
        let cancel = AtomicBool::new(false);

        let actual = compute_spa_validation_stats(
            &candidate_keys,
            &candidate_slice_returns,
            &benchmark_slice_returns,
            &cfg,
            777,
            &cancel,
        )
        .expect("spa stats");
        let expected = compute_spa_validation_stats_legacy(
            &candidate_keys,
            &candidate_slice_returns,
            &benchmark_slice_returns,
            &cfg,
            777,
        )
        .expect("legacy spa stats");

        let actual = serde_json::to_value(actual).expect("serialize SPA stats");
        let expected = serde_json::to_value(expected).expect("serialize legacy SPA stats");
        assert_eq!(actual, expected);
    }

    #[test]
    fn spa_validation_clamps_low_bootstrap_and_block_length_inputs() {
        let candidate_keys = vec!["a".to_string()];
        let candidate_slice_returns = vec![vec![0.03, 0.04, 0.05, 0.06]];
        let benchmark_slice_returns = vec![0.0, 0.0, 0.0, 0.0];
        let cfg = SpaConfigOptional {
            slices: 4,
            bootstrap_samples: 0,
            block_length: 0,
            benchmark: crate::validation_shared::SpaBenchmarkKind::BuyAndHold,
        };
        let cancel = AtomicBool::new(false);

        let stats = compute_spa_validation_stats(
            &candidate_keys,
            &candidate_slice_returns,
            &benchmark_slice_returns,
            &cfg,
            13,
            &cancel,
        )
        .expect("spa stats");

        assert_eq!(stats.slice_count, 4);
        assert_eq!(stats.bootstrap_samples, 100);
        assert_eq!(stats.block_length, 1);
    }

    #[test]
    fn run_verdict_marks_unapproved_when_mintrl_not_met() {
        let mut out = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.0,
                ..Metrics::default()
            },
            psr: Some(0.95),
            dsr: Some(0.90),
            track_record_bars: 64,
            min_track_record_length_bars: Some(256),
            robust_score: 1.0,
            robust_pareto: true,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        apply_candidate_hard_gates(&mut out, &ValidationHardGateConfig::default(), None);
        let verdict = build_run_verdict(&out, 1, &ValidationHardGateConfig::default(), None, None);
        assert!(!verdict.approved);
        assert_eq!(
            verdict.final_recommendation,
            ValidationRecommendation::EvidenceTooShortMinTrackRecord
        );
        assert_eq!(verdict.approved_candidate_count, 0);
    }

    #[test]
    fn build_run_verdict_fails_when_max_pbo_requested_but_cscv_missing() {
        let gates = ValidationHardGateConfig {
            max_pbo: Some(0.30),
            ..ValidationHardGateConfig::default()
        };
        let mut out = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.1,
                ..Metrics::default()
            },
            psr: Some(0.95),
            dsr: Some(0.90),
            track_record_bars: 256,
            min_track_record_length_bars: Some(64),
            robust_score: 1.1,
            robust_pareto: true,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        apply_candidate_hard_gates(&mut out, &ValidationHardGateConfig::default(), None);

        let verdict = build_run_verdict(&out, 1, &gates, None, None);

        assert!(!verdict.passed);
        assert!(!verdict.approved);
        assert_eq!(verdict.max_pbo, Some(0.30));
        assert!(verdict
            .fail_reasons
            .iter()
            .any(|reason| reason == "pbo_gate_requested_but_cscv_unavailable"));
    }

    #[test]
    fn build_run_verdict_warns_when_cscv_result_is_skipped() {
        let mut out = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.1,
                ..Metrics::default()
            },
            psr: Some(0.95),
            dsr: Some(0.90),
            track_record_bars: 256,
            min_track_record_length_bars: Some(64),
            robust_score: 1.1,
            robust_pareto: true,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        apply_candidate_hard_gates(&mut out, &ValidationHardGateConfig::default(), None);

        let cscv = CscvPboResult {
            slices: 8,
            slice_bars: 32,
            candidates: 1,
            combinations: 0,
            combinations_possible: 0,
            pbo: 0.0,
            lambda_p25: 0.0,
            lambda_p50: 0.0,
            lambda_p75: 0.0,
            degradation_slope: 0.0,
            degradation_intercept: 0.0,
            degradation_r2: 0.0,
            threshold: 0.0,
            prob_oos_below_threshold: 0.0,
            skipped_reason: Some("CSCV/PBO requires at least 2 candidates.".to_string()),
        };

        let verdict = build_run_verdict(
            &out,
            1,
            &ValidationHardGateConfig::default(),
            Some(&cscv),
            None,
        );

        assert!(verdict.passed);
        assert!(verdict.approved);
        assert_eq!(
            verdict.final_recommendation,
            ValidationRecommendation::ApprovedWithWarnings
        );
        assert!(verdict
            .warning_flags
            .iter()
            .any(|flag| flag == "cscv_skipped"));
        assert_eq!(verdict.pbo, None);
    }

    #[test]
    fn validation_evidence_pack_records_schema_route_and_metric_scope() {
        let gates = ValidationHardGateConfig {
            min_psr: Some(0.75),
            min_dsr: Some(0.60),
            max_pbo: Some(0.25),
            ..ValidationHardGateConfig::default()
        };
        let mut out = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.2,
                ..Metrics::default()
            },
            psr: Some(0.91),
            dsr: Some(0.72),
            track_record_bars: 512,
            min_track_record_length_bars: Some(256),
            robust_score: 1.2,
            robust_pareto: true,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        apply_candidate_hard_gates(&mut out, &gates, None);
        let cscv = CscvPboResult {
            slices: 8,
            slice_bars: 32,
            candidates: 3,
            combinations: 70,
            combinations_possible: 70,
            pbo: 0.14,
            lambda_p25: -0.2,
            lambda_p50: 0.1,
            lambda_p75: 0.4,
            degradation_slope: 0.0,
            degradation_intercept: 0.0,
            degradation_r2: 0.0,
            threshold: 0.0,
            prob_oos_below_threshold: 0.2,
            skipped_reason: None,
        };
        let verdict = build_run_verdict(&out, 3, &gates, Some(&cscv), None);
        let policy = build_general_validation_evidence_policy_summary(&gates, true, Some(&cscv));
        let validation_context = ValidationArtifactContext {
            search_distribution: Some(ta_optimizer::SearchDistributionSummary {
                metric_surface_kind: "core17".to_string(),
                sharpe_metric_id: ta_optimizer::DOUBLE_MA_SHARPE_METRIC_ID.to_string(),
                attempted_count: 58_300,
                unique_count: 58_300,
                feasible_count: 58_000,
                finite_sharpe_count: 57_900,
                finite_sharpe_mean: Some(0.15),
                finite_sharpe_sample_std: Some(0.35),
            }),
            data_realism: Some(ValidationRealismContext {
                price_adjustment: ValidationPriceAdjustmentState::Adjusted,
                corporate_actions: ValidationRealismEvidenceStatus::Documented,
                survivorship_bias: ValidationRealismEvidenceStatus::Documented,
                liquidity_capacity: ValidationRealismEvidenceStatus::Documented,
                market_impact: ValidationRealismEvidenceStatus::Documented,
                borrow_availability: ValidationRealismEvidenceStatus::Documented,
            }),
            ..ValidationArtifactContext::default()
        };
        let pack = build_validation_evidence_pack(ValidationEvidencePackBuildInput {
            workflow: "double_ma_validation",
            data_id: "dataset-a",
            validation_context: Some(&validation_context),
            requested_backend: "GpuKernel",
            realized_backend: "Gpu",
            backend_route: Some("GpuKernel"),
            residency_tier: "gpu_kernel",
            fallback_reason: None,
            route_diagnostics: &[],
            candidate_source: "validation_request_candidates",
            requested_candidate_count: 3,
            evaluated_candidate_count: 3,
            retained_candidate_count: out.len(),
            optimization_trials: Some(58_300),
            admission_policy: Some("scalar_union"),
            hard_gates: &gates,
            run_verdict: &verdict,
            evidence_policy_summary: Some(&policy),
            validation_provenance: None,
            cscv_requested: true,
            cscv_pbo: Some(&cscv),
            spa_requested: false,
            spa: None,
        });

        assert_eq!(pack.version, VALIDATION_EVIDENCE_PACK_VERSION);
        assert_eq!(pack.schema_version, VALIDATION_EVIDENCE_PACK_SCHEMA_VERSION);
        assert_eq!(pack.status, ValidationEvidencePackStatus::Complete);
        assert!(pack.dataset.data_realism.complete);
        assert!(pack.promotion_eligibility.eligible);
        assert_eq!(
            pack.promotion_eligibility.state,
            crate::validation_shared::ValidationPromotionEligibilityState::EligibleForReview
        );
        assert_eq!(pack.route.route_label.as_deref(), Some("GpuKernel"));
        assert_eq!(pack.candidate_universe.optimization_trials, Some(58_300));
        assert_eq!(
            pack.candidate_universe
                .search_distribution
                .as_ref()
                .map(|summary| summary.finite_sharpe_count),
            Some(57_900)
        );
        let dsr = pack
            .metrics
            .iter()
            .find(|metric| metric.metric == "dsr")
            .expect("dsr metric");
        assert!(dsr.comparability_basis.contains("full-search 'core17'"));
        let pbo = pack
            .metrics
            .iter()
            .find(|metric| metric.metric == "cscv_pbo")
            .expect("cscv metric");
        assert_eq!(pbo.availability, ValidationEvidenceAvailability::Available);
        assert_eq!(pbo.scope, "candidate_universe");
        assert_eq!(pbo.sample_unit, "candidate_set_slices");
        assert_eq!(pbo.metric_class, "approximate");
        assert!(pbo.comparability_basis.contains("set-level CSCV/PBO"));
        assert_eq!(pbo.value, Some(0.14));
        assert_eq!(pbo.threshold, Some(0.25));
    }

    #[test]
    fn validation_evidence_pack_marks_requested_skipped_modules_incomplete() {
        let gates = ValidationHardGateConfig::default();
        let mut out = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.1,
                ..Metrics::default()
            },
            psr: Some(0.88),
            dsr: Some(0.70),
            track_record_bars: 256,
            min_track_record_length_bars: Some(128),
            robust_score: 1.1,
            robust_pareto: true,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        apply_candidate_hard_gates(&mut out, &gates, None);
        let cscv = CscvPboResult {
            slices: 8,
            slice_bars: 32,
            candidates: 1,
            combinations: 0,
            combinations_possible: 0,
            pbo: 0.0,
            lambda_p25: 0.0,
            lambda_p50: 0.0,
            lambda_p75: 0.0,
            degradation_slope: 0.0,
            degradation_intercept: 0.0,
            degradation_r2: 0.0,
            threshold: 0.0,
            prob_oos_below_threshold: 0.0,
            skipped_reason: Some("CSCV/PBO requires at least 2 candidates.".to_string()),
        };
        let verdict = build_run_verdict(&out, 1, &gates, Some(&cscv), None);
        let policy = build_general_validation_evidence_policy_summary(&gates, true, Some(&cscv));
        let pack = build_validation_evidence_pack(ValidationEvidencePackBuildInput {
            workflow: "composite_validation",
            data_id: "dataset-b",
            validation_context: None,
            requested_backend: "CpuOnly",
            realized_backend: "Cpu",
            backend_route: Some("CpuOnly"),
            residency_tier: "cpu",
            fallback_reason: None,
            route_diagnostics: &[],
            candidate_source: "validation_request_candidates",
            requested_candidate_count: 1,
            evaluated_candidate_count: 1,
            retained_candidate_count: out.len(),
            optimization_trials: None,
            admission_policy: None,
            hard_gates: &gates,
            run_verdict: &verdict,
            evidence_policy_summary: Some(&policy),
            validation_provenance: None,
            cscv_requested: true,
            cscv_pbo: Some(&cscv),
            spa_requested: true,
            spa: None,
        });

        assert_eq!(pack.status, ValidationEvidencePackStatus::Incomplete);
        assert!(pack.status_reason.contains("cscv_pbo"));
        assert!(pack.status_reason.contains("spa"));
        assert!(pack.status_reason.contains("deployment-realism"));
        assert!(!pack.promotion_eligibility.eligible);
        let cscv_module = pack
            .modules
            .iter()
            .find(|module| module.module == "cscv_pbo")
            .expect("cscv module");
        assert_eq!(
            cscv_module.availability,
            ValidationEvidenceAvailability::Skipped
        );
        assert_eq!(
            cscv_module.skipped_reason.as_deref(),
            Some("CSCV/PBO requires at least 2 candidates.")
        );
        let spa_module = pack
            .modules
            .iter()
            .find(|module| module.module == "spa")
            .expect("spa module");
        assert_eq!(
            spa_module.availability,
            ValidationEvidenceAvailability::Unavailable
        );
    }

    #[test]
    fn validation_semantics_provenance_marks_zero_selected_cscv_state() {
        let out = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.0,
                ..Metrics::default()
            },
            psr: Some(0.9),
            dsr: Some(0.8),
            track_record_bars: 64,
            min_track_record_length_bars: Some(32),
            robust_score: 1.0,
            robust_pareto: true,
            cscv_pbo: Some(CscvCandidatePboStats {
                is_selected_frac: 0.0,
                oos_rank_frac_p50: None,
                oos_metric_p50: None,
            }),
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        let cscv = CscvPboResult {
            slices: 8,
            slice_bars: 32,
            candidates: 1,
            combinations: 12,
            combinations_possible: 12,
            pbo: 0.2,
            lambda_p25: -0.1,
            lambda_p50: 0.0,
            lambda_p75: 0.1,
            degradation_slope: 0.0,
            degradation_intercept: 0.0,
            degradation_r2: 1.0,
            threshold: 0.0,
            prob_oos_below_threshold: 0.2,
            skipped_reason: None,
        };

        let provenance = build_validation_semantics_provenance(
            &out,
            true,
            Some(&cscv),
            None,
            Some("volatility_terciles_plus_trend_range"),
        );

        assert_eq!(provenance.cscv_coverage_state, "evaluated");
        assert_eq!(provenance.cscv_candidate_stat_coverage, "complete");
        assert_eq!(provenance.cscv_selection_state, "zero_selected");
        assert_eq!(provenance.cscv_selected_candidate_count, 0);
        assert_eq!(provenance.cross_summary_scope, "not_requested");
        assert_eq!(
            provenance.approved_semantics,
            "passed_and_min_track_record_length_satisfied"
        );
        assert!(provenance.passed_can_diverge_from_approved);
    }

    #[test]
    fn validation_semantics_provenance_marks_partial_cscv_candidate_coverage() {
        let out = vec![
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 1.1,
                    ..Metrics::default()
                },
                psr: Some(0.9),
                dsr: Some(0.8),
                track_record_bars: 64,
                min_track_record_length_bars: Some(32),
                robust_score: 1.1,
                robust_pareto: true,
                cscv_pbo: Some(CscvCandidatePboStats {
                    is_selected_frac: 0.4,
                    oos_rank_frac_p50: Some(0.7),
                    oos_metric_p50: Some(1.0),
                }),
                validation_verdict: ValidationCandidateVerdict::default(),
            },
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 0.9,
                    ..Metrics::default()
                },
                psr: Some(0.85),
                dsr: Some(0.8),
                track_record_bars: 64,
                min_track_record_length_bars: Some(32),
                robust_score: 0.9,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
        ];
        let cscv = CscvPboResult {
            slices: 8,
            slice_bars: 32,
            candidates: 2,
            combinations: 12,
            combinations_possible: 12,
            pbo: 0.2,
            lambda_p25: -0.1,
            lambda_p50: 0.0,
            lambda_p75: 0.1,
            degradation_slope: 0.0,
            degradation_intercept: 0.0,
            degradation_r2: 1.0,
            threshold: 0.0,
            prob_oos_below_threshold: 0.2,
            skipped_reason: None,
        };

        let provenance = build_validation_semantics_provenance(&out, true, Some(&cscv), None, None);

        assert_eq!(provenance.cscv_coverage_state, "evaluated");
        assert_eq!(provenance.cscv_candidate_stat_coverage, "partial");
        assert_eq!(
            provenance.cscv_selection_state,
            "selected_candidates_present"
        );
        assert_eq!(provenance.cscv_selected_candidate_count, 1);
        assert_eq!(provenance.regime_summary_family, None);
    }

    #[test]
    fn validation_semantics_provenance_marks_success_only_cross_summary_scope() {
        let summary = CrossTickerValidationSummary {
            mode: crate::validation_shared::CrossTickerMode::PreservedFinalists,
            datasets: Vec::new(),
        };
        let provenance = build_validation_semantics_provenance::<DummyCandidate>(
            &[],
            false,
            None,
            Some(&summary),
            Some("volatility_terciles_plus_trend_range"),
        );

        assert_eq!(provenance.cross_summary_scope, "passed_candidates_only");
        assert_eq!(
            provenance.cross_mode,
            Some(crate::validation_shared::CrossTickerMode::PreservedFinalists)
        );
        assert_eq!(
            provenance.regime_summary_role.as_deref(),
            Some("descriptive_only")
        );
    }

    #[test]
    fn general_evidence_policy_labels_probability_metrics_as_raw_bars() {
        let gates = ValidationHardGateConfig {
            min_psr: Some(0.80),
            max_pbo: Some(0.35),
            ..ValidationHardGateConfig::default()
        };
        let summary = build_general_validation_evidence_policy_summary(&gates, true, None);

        let psr = summary
            .metrics
            .iter()
            .find(|metric| metric.metric == "psr")
            .expect("psr metric");
        assert_eq!(psr.sample_unit, "raw_bars");
        assert_eq!(psr.gate_role, EvidenceGateRole::HardGate);
        assert_eq!(psr.metric_class, EvidenceMetricClass::ConfidenceBounded);

        let cscv = summary
            .metrics
            .iter()
            .find(|metric| metric.metric == "cscv_pbo")
            .expect("cscv metric");
        assert_eq!(cscv.sample_unit, "candidate_set_slices");
        assert_eq!(cscv.gate_role, EvidenceGateRole::HardGate);
        assert!(cscv
            .caveats
            .iter()
            .any(|caveat| caveat.contains("set-level overfit evidence")));
        assert!(summary
            .caveats
            .iter()
            .any(|caveat| caveat.contains("does not alter candidate math")));
    }

    #[test]
    fn ml_signal_evidence_policy_labels_effective_score_observations() {
        let gates = ValidationHardGateConfig {
            min_dsr: Some(0.40),
            ..ValidationHardGateConfig::default()
        };
        let summary =
            build_ml_signal_evidence_policy_summary(&gates, true, Some("walkforward_test"));

        let dsr = summary
            .metrics
            .iter()
            .find(|metric| metric.metric == "dsr")
            .expect("dsr metric");
        assert_eq!(dsr.sample_unit, "effective_score_observations_lower_bound");
        assert_eq!(dsr.gate_role, EvidenceGateRole::HardGate);

        let deployability = summary
            .metrics
            .iter()
            .find(|metric| metric.metric == "deployability")
            .expect("deployability metric");
        assert_eq!(
            deployability.metric_class,
            EvidenceMetricClass::MetadataOnly
        );
        assert!(deployability
            .caveats
            .iter()
            .any(|caveat| caveat.contains("not deployment proof")));
    }

    #[test]
    fn run_verdict_preserves_rank_one_stats_when_later_candidate_is_first_approved() {
        let gates = ValidationHardGateConfig {
            min_psr: Some(0.70),
            ..ValidationHardGateConfig::default()
        };
        let mut out = vec![
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 1.4,
                    ..Metrics::default()
                },
                psr: Some(0.40),
                dsr: Some(0.95),
                track_record_bars: 256,
                min_track_record_length_bars: Some(64),
                robust_score: 1.4,
                robust_pareto: true,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 1.1,
                    ..Metrics::default()
                },
                psr: Some(0.92),
                dsr: Some(0.90),
                track_record_bars: 256,
                min_track_record_length_bars: Some(64),
                robust_score: 1.1,
                robust_pareto: true,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
        ];
        apply_candidate_hard_gates(&mut out, &gates, None);

        let verdict = build_run_verdict(&out, 2, &gates, None, None);

        assert!(verdict.approved);
        assert_eq!(verdict.candidates_survived_hard_gates, 1);
        assert_eq!(verdict.approved_candidate_count, 1);
        assert_eq!(verdict.first_approved_rank, Some(2));
        assert_eq!(verdict.top_candidate_psr, Some(0.40));
        assert_eq!(verdict.top_candidate_dsr, Some(0.95));
        assert_eq!(
            verdict.final_recommendation,
            ValidationRecommendation::ApprovedWithWarnings
        );
    }

    #[test]
    fn finalize_with_probability_and_optional_cscv_returns_skipped_result_without_slice_builder_when_out_len_lt_2_or_slice_bars_lt_2(
    ) {
        let cancel = AtomicBool::new(false);
        let cfg = CscvPboConfig {
            slices: 4,
            max_combinations: 16,
            threshold: 0.0,
        };

        let mut single_out = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.0,
                ..Metrics::default()
            },
            psr: None,
            dsr: None,
            track_record_bars: 0,
            min_track_record_length_bars: None,
            robust_score: 1.0,
            robust_pareto: false,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        let single_calls = AtomicUsize::new(0);
        let single_cscv = finalize_with_probability_and_optional_cscv(
            &mut single_out,
            &[128],
            &ValidationHardGateConfig::default(),
            Some(&cfg),
            256,
            0,
            7,
            &cancel,
            None,
            "unit_cscv",
            |_cand, _start, _slice_bars, _slices| {
                single_calls.fetch_add(1, Ordering::Relaxed);
                Ok(Vec::<SliceReturnStats>::new())
            },
        )
        .expect("single candidate finalize")
        .expect("single candidate cscv result");

        assert_eq!(single_calls.load(Ordering::Relaxed), 0);
        assert_eq!(single_cscv.combinations, 0);
        assert!(single_cscv.skipped_reason.is_some());

        let mut short_slice_out = vec![
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 1.1,
                    ..Metrics::default()
                },
                psr: None,
                dsr: None,
                track_record_bars: 0,
                min_track_record_length_bars: None,
                robust_score: 1.1,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
            DummyCandidate {
                in_sample: Metrics {
                    sharpe: 0.9,
                    ..Metrics::default()
                },
                psr: None,
                dsr: None,
                track_record_bars: 0,
                min_track_record_length_bars: None,
                robust_score: 0.9,
                robust_pareto: false,
                cscv_pbo: None,
                validation_verdict: ValidationCandidateVerdict::default(),
            },
        ];
        let short_slice_calls = AtomicUsize::new(0);
        let short_slice_cscv = finalize_with_probability_and_optional_cscv(
            &mut short_slice_out,
            &[128, 128],
            &ValidationHardGateConfig::default(),
            Some(&cfg),
            5,
            2,
            11,
            &cancel,
            None,
            "unit_cscv",
            |_cand, _start, _slice_bars, _slices| {
                short_slice_calls.fetch_add(1, Ordering::Relaxed);
                Ok(Vec::<SliceReturnStats>::new())
            },
        )
        .expect("short slice finalize")
        .expect("short slice cscv result");

        assert_eq!(short_slice_calls.load(Ordering::Relaxed), 0);
        assert_eq!(short_slice_cscv.slice_bars, 0);
        assert_eq!(short_slice_cscv.combinations, 0);
        assert!(short_slice_cscv.skipped_reason.is_some());
    }

    #[test]
    fn run_verdict_marks_search_not_statistically_superior_when_spa_fails() {
        let mut out = vec![DummyCandidate {
            in_sample: Metrics {
                sharpe: 1.2,
                ..Metrics::default()
            },
            psr: Some(0.95),
            dsr: Some(0.90),
            track_record_bars: 256,
            min_track_record_length_bars: Some(64),
            robust_score: 1.1,
            robust_pareto: true,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        apply_candidate_hard_gates(&mut out, &ValidationHardGateConfig::default(), None);
        let spa = SpaValidationStats {
            scope: "validation".to_string(),
            candidate_count: 1,
            slice_count: 8,
            block_length: 2,
            benchmark: crate::validation_shared::SpaBenchmarkKind::BuyAndHold,
            bootstrap_samples: 256,
            best_candidate_key: "alpha".to_string(),
            best_candidate_mean_diff: 0.1,
            best_candidate_t_stat: 0.2,
            p_value: 0.2,
            passed: false,
            skipped_reason: None,
        };

        let verdict = build_run_verdict(
            &out,
            1,
            &ValidationHardGateConfig::default(),
            None,
            Some(&spa),
        );

        assert!(!verdict.approved);
        assert_eq!(
            verdict.final_recommendation,
            ValidationRecommendation::CandidatesSurvivedButSearchNotStatisticallySuperior
        );
        assert!(verdict
            .warning_flags
            .iter()
            .any(|flag| flag == "spa_not_significant"));
    }

    #[test]
    fn run_verdict_marks_approved_with_warnings_when_candidate_has_nonfatal_flags() {
        let mut out = vec![DummyCandidate {
            in_sample: Metrics {
                pnl: 0.10,
                sharpe: 1.0,
                volatility: 0.10,
                downside_deviation: 0.095,
                ulcer_index: 0.15,
                martin_ratio: 0.20,
                max_dd: 0.2,
                trades: 32,
                ..Metrics::default()
            },
            psr: Some(0.95),
            dsr: Some(0.90),
            track_record_bars: 256,
            min_track_record_length_bars: Some(64),
            robust_score: 1.1,
            robust_pareto: true,
            cscv_pbo: None,
            validation_verdict: ValidationCandidateVerdict::default(),
        }];
        apply_candidate_hard_gates(&mut out, &ValidationHardGateConfig::default(), None);
        let verdict = build_run_verdict(&out, 1, &ValidationHardGateConfig::default(), None, None);

        assert!(verdict.approved);
        assert_eq!(
            verdict.final_recommendation,
            ValidationRecommendation::ApprovedWithWarnings
        );
        assert!(!verdict.warning_flags.is_empty());
        assert!(verdict
            .warning_flags
            .iter()
            .any(|flag| flag == "ulcer_index_high:0.150"));
    }

    #[test]
    fn select_validation_collection_mode_uses_sequential_fast_path_without_set_level_obligations() {
        let mode =
            select_validation_collection_mode(&ValidationHardGateConfig::default(), None, false);
        assert_eq!(mode, ValidationCollectionMode::SequentialFastPath);
    }

    #[test]
    fn select_validation_collection_mode_falls_back_when_set_level_obligations_exist() {
        let cscv_mode = select_validation_collection_mode(
            &ValidationHardGateConfig::default(),
            Some(&CscvPboConfig {
                slices: 4,
                max_combinations: 16,
                threshold: 0.0,
            }),
            false,
        );
        assert_eq!(cscv_mode, ValidationCollectionMode::ParallelFullSet);

        let spa_mode =
            select_validation_collection_mode(&ValidationHardGateConfig::default(), None, true);
        assert_eq!(spa_mode, ValidationCollectionMode::ParallelFullSet);

        let pbo_gate_mode = select_validation_collection_mode(
            &ValidationHardGateConfig {
                max_pbo: Some(0.30),
                ..ValidationHardGateConfig::default()
            },
            None,
            false,
        );
        assert_eq!(pbo_gate_mode, ValidationCollectionMode::ParallelFullSet);
    }

    #[test]
    fn sequential_collection_and_finalize_matches_parallel_full_collection_without_cscv() {
        let candidates = vec![1usize, 2, 3, 4];
        let cancel = AtomicBool::new(false);
        let process_one = |idx: usize, p: &usize, rng: &mut StdRng| {
            let noise = rng.gen_range(0.0..0.05);
            let sharpe = (*p as f64) + noise;
            Ok((
                DummyCandidate {
                    in_sample: Metrics {
                        sharpe,
                        max_dd: 0.1 * (*p as f64),
                        ..Metrics::default()
                    },
                    psr: None,
                    dsr: None,
                    track_record_bars: 0,
                    min_track_record_length_bars: None,
                    robust_score: sharpe,
                    robust_pareto: false,
                    cscv_pbo: None,
                    validation_verdict: ValidationCandidateVerdict::default(),
                },
                idx,
                64 + idx,
            ))
        };

        let mut sequential = sequential_collect_validation_candidates(
            &candidates,
            17,
            &cancel,
            None,
            "unit_validate",
            &process_one,
        )
        .expect("sequential collect");
        let mut parallel = parallel_collect_validation_candidates(
            &candidates,
            17,
            &cancel,
            None,
            "unit_validate",
            &process_one,
        )
        .expect("parallel collect");

        let finalize = |collected: &mut Vec<(DummyCandidate, usize, usize)>| {
            let mut out = Vec::with_capacity(collected.len());
            let mut sample_obs = Vec::with_capacity(collected.len());
            for (cand, _start_t, obs) in collected.drain(..) {
                out.push(cand);
                sample_obs.push(obs.max(2));
            }
            compute_probability_metrics(&mut out, &sample_obs);
            mark_robust_pareto(&mut out);
            apply_candidate_hard_gates(&mut out, &ValidationHardGateConfig::default(), None);
            fold_cscv_into_robust_and_sort(&mut out);
            out
        };

        let sequential_out = finalize(&mut sequential);
        let parallel_out = finalize(&mut parallel);

        let summarize = |out: Vec<DummyCandidate>| {
            out.into_iter()
                .map(|cand| {
                    (
                        cand.in_sample.sharpe.to_bits(),
                        cand.psr.map(f64::to_bits),
                        cand.dsr.map(f64::to_bits),
                        cand.track_record_bars,
                        cand.min_track_record_length_bars,
                        cand.robust_pareto,
                        cand.validation_verdict.passed,
                    )
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(summarize(sequential_out), summarize(parallel_out));
    }
}
