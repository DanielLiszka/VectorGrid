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
    PerturbConfig, PerturbStats, ReoptWalkForwardConfigOptional, ReoptWalkForwardStats,
    ReoptWalkForwardWindowStats, SpaConfigOptional, SpaValidationStats, TradeBootstrapConfig,
    TradeBootstrapStats, WalkForwardConfigOptional, WalkForwardStats,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
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

pub trait ValidationPostCandidate {
    fn in_sample_metrics(&self) -> &Metrics;
    fn iterative_in_sample_metrics(&self) -> Option<&Metrics>;
    fn walk_forward_stats(&self) -> Option<&WalkForwardStats>;
    fn cscv_oos_metric_p50(&self) -> Option<f64>;
    fn cscv_oos_rank_frac_p50(&self) -> Option<f64>;

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
    fn validation_verdict(&self) -> &ValidationCandidateVerdict;
    fn set_validation_verdict(&mut self, v: ValidationCandidateVerdict);
}

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
    let sharpe_peers: Vec<f64> = out
        .iter()
        .map(|c| {
            c.iterative_in_sample_metrics()
                .unwrap_or_else(|| c.in_sample_metrics())
                .sharpe
        })
        .collect();

    let (peer_mean, peer_std) = crate::sharpe_stats::mean_std(&sharpe_peers);
    let trials = sharpe_peers.iter().filter(|x| x.is_finite()).count().max(1);

    for (i, cand) in out.iter_mut().enumerate() {
        let m = cand
            .iterative_in_sample_metrics()
            .unwrap_or_else(|| cand.in_sample_metrics());
        let requested_obs = sample_obs.get(i).copied().unwrap_or(2);
        let obs = requested_obs.max(2);
        let (psr, dsr) =
            crate::validation_stats::compute_psr_dsr(m.sharpe, obs, trials, peer_mean, peer_std);
        let mintrl =
            crate::sharpe_stats::minimum_track_record_length(m.sharpe, 0.0, 0.0, 3.0, 0.95);
        cand.set_psr(psr);
        cand.set_dsr(dsr);
        cand.set_track_record_bars(requested_obs);
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
    let candidates_survived_hard_gates = out
        .iter()
        .filter(|cand| cand.validation_verdict().passed)
        .count();
    let approved_candidates: Vec<(usize, &C)> = out
        .iter()
        .enumerate()
        .filter(|(_, cand)| {
            cand.validation_verdict().passed
                && cand
                    .min_track_record_length_bars()
                    .map(|mintrl| cand.track_record_bars().max(2) >= mintrl)
                    .unwrap_or(false)
        })
        .collect();
    let approved_candidate_count = approved_candidates.len();
    let first_approved_rank = approved_candidates
        .first()
        .map(|(idx, _)| idx.saturating_add(1));

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

fn estimated_window_count(n: usize, start_t: usize, window: usize, step: usize) -> usize {
    if n < 3 || start_t + 2 > n {
        return 0;
    }
    let mut count = 0usize;
    let mut t0 = start_t;
    while t0 + 2 <= n {
        let end = t0.saturating_add(window).min(n);
        if end <= t0 + 1 {
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
        return values[0];
    }
    let idx = quantile_index(values.len(), q).min(values.len() - 1);
    let (_, nth, _) = values.select_nth_unstable_by(idx, |a, b| a.total_cmp(b));
    *nth
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
    let mut trades: Vec<u32> = Vec::with_capacity(estimated_windows);

    let mut t0 = start_t;
    while t0 + 2 <= n {
        let end = t0.saturating_add(window).min(n);
        if end <= t0 + 1 {
            break;
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
        trades.push(m.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    let sharpe_worst = worst_min(&sharpes);
    let max_dd_worst = worst_max(&dds);
    let pnl_worst = worst_min(&pnls);
    let downside_vol_ratio_worst = worst_max(&downside_ratios);
    let ulcer_index_worst = worst_max(&ulcers);
    let martin_ratio_worst = worst_min(&martins);
    let trades_worst = worst_min_u32(&trades);

    WalkForwardStats {
        windows: sharpes.len(),
        sharpe_median: select_median_in_place(&mut sharpes),
        sharpe_worst,
        max_dd_median: select_median_in_place(&mut dds),
        max_dd_worst,
        pnl_median: select_median_in_place(&mut pnls),
        pnl_worst,
        downside_vol_ratio_median: select_median_in_place(&mut downside_ratios),
        downside_vol_ratio_worst,
        ulcer_index_median: select_median_in_place(&mut ulcers),
        ulcer_index_worst,
        martin_ratio_median: select_median_in_place(&mut martins),
        martin_ratio_worst,
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
    while t0 + 2 < n {
        let train_end = t0.saturating_add(window).min(n);
        if train_end <= t0 + 1 {
            break;
        }
        let test_end = train_end.saturating_add(step).min(n);
        if test_end <= train_end + 1 {
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

pub fn compute_trade_bootstrap_stats(
    trades: &[Trade],
    cfg: &TradeBootstrapConfig,
    rng: &mut StdRng,
) -> TradeBootstrapStats {
    let simulations = cfg.simulations.min(50_000);
    let closed: Vec<f64> = trades.iter().filter(|t| !t.open).map(|t| t.pnl).collect();
    let trade_count = closed.len();
    if simulations == 0 || trade_count == 0 {
        return TradeBootstrapStats {
            simulations,
            trade_count,
            prob_pnl_positive: 0.0,
            pnl_p05: 0.0,
            pnl_p50: 0.0,
            pnl_p95: 0.0,
            max_dd_p50: 0.0,
            max_dd_p95: 0.0,
            stationary_mean_block_len: None,
            stationary_prob_pnl_positive: None,
            stationary_pnl_p05: None,
            stationary_pnl_p50: None,
            stationary_pnl_p95: None,
            stationary_max_dd_p50: None,
            stationary_max_dd_p95: None,
        };
    }

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
    let pnl_p05 = select_quantile_in_place(&mut pnls, 0.05);
    let pnl_p50 = select_quantile_in_place(&mut pnls, 0.5);
    let pnl_p95 = select_quantile_in_place(&mut pnls, 0.95);
    let max_dd_p50 = select_quantile_in_place(&mut dds, 0.5);
    let max_dd_p95 = select_quantile_in_place(&mut dds, 0.95);
    if cfg.stationary_mean_block_len == Some(0) {
        return TradeBootstrapStats {
            simulations,
            trade_count,
            prob_pnl_positive: pos as f64 / simulations as f64,
            pnl_p05,
            pnl_p50,
            pnl_p95,
            max_dd_p50,
            max_dd_p95,
            stationary_mean_block_len: None,
            stationary_prob_pnl_positive: None,
            stationary_pnl_p05: None,
            stationary_pnl_p50: None,
            stationary_pnl_p95: None,
            stationary_max_dd_p50: None,
            stationary_max_dd_p95: None,
        };
    }
    let stationary_mean_block_len = cfg
        .stationary_mean_block_len
        .map(|value| value.clamp(1, trade_count.max(1)))
        .unwrap_or_else(|| trade_bootstrap_stationary_mean_block_len(trade_count));
    let mut stationary_pnls: Vec<f64> = Vec::with_capacity(simulations);
    let mut stationary_dds: Vec<f64> = Vec::with_capacity(simulations);
    let mut stationary_pos = 0usize;
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
            stationary_pos += 1;
        }
        stationary_pnls.push(pnl);
        stationary_dds.push(max_dd);
    }
    let stationary_pnl_p05 = select_quantile_in_place(&mut stationary_pnls, 0.05);
    let stationary_pnl_p50 = select_quantile_in_place(&mut stationary_pnls, 0.5);
    let stationary_pnl_p95 = select_quantile_in_place(&mut stationary_pnls, 0.95);
    let stationary_max_dd_p50 = select_quantile_in_place(&mut stationary_dds, 0.5);
    let stationary_max_dd_p95 = select_quantile_in_place(&mut stationary_dds, 0.95);
    TradeBootstrapStats {
        simulations,
        trade_count,
        prob_pnl_positive: pos as f64 / simulations as f64,
        pnl_p05,
        pnl_p50,
        pnl_p95,
        max_dd_p50,
        max_dd_p95,
        stationary_mean_block_len: Some(stationary_mean_block_len),
        stationary_prob_pnl_positive: Some(stationary_pos as f64 / simulations as f64),
        stationary_pnl_p05: Some(stationary_pnl_p05),
        stationary_pnl_p50: Some(stationary_pnl_p50),
        stationary_pnl_p95: Some(stationary_pnl_p95),
        stationary_max_dd_p50: Some(stationary_max_dd_p50),
        stationary_max_dd_p95: Some(stationary_max_dd_p95),
    }
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
    mut build_slice_stats_for_candidate: F,
) -> Result<Option<CscvPboResult>, String>
where
    C: ValidationPostCandidate,
    F: FnMut(&C, usize, usize, usize) -> Result<Vec<SliceReturnStats>, String>,
{
    compute_probability_metrics(out, sample_obs);
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
        let t = (sqrt_n * mean / std).max(0.0);
        if mean > best_mean {
            best_mean = mean;
            best_idx = idx;
        }
        diffs.push(diff);
        t_stats.push(t);
    }

    let observed_stat = t_stats.iter().copied().fold(0.0_f64, f64::max);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation_pipeline::ValidationRecommendation;
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
            sharpe_median: 2.0,
            sharpe_worst: 1.0,
            max_dd_median: 0.2,
            max_dd_worst: 0.3,
            pnl_median: 0.1,
            pnl_worst: 0.0,
            downside_vol_ratio_median: 0.5,
            downside_vol_ratio_worst: 0.8,
            ulcer_index_median: 0.05,
            ulcer_index_worst: 0.1,
            martin_ratio_median: 1.0,
            martin_ratio_worst: 0.5,
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
        assert!(stats.stationary_pnl_p05.is_some());
        assert!(stats.stationary_max_dd_p95.is_some());
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
        assert!(stats.stationary_mean_block_len.is_none());
        assert!(stats.stationary_pnl_p05.is_none());
        assert!(stats.stationary_max_dd_p95.is_none());
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
}
