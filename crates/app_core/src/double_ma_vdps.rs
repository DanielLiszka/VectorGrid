use std::collections::BTreeSet;

use rand::rngs::StdRng;
use rand::Rng;
use ta_optimizer::{ObjectiveKind, OptimizationResult};
use ta_strategies::double_ma::{DoubleMaParams, Metrics};

use crate::double_ma::{
    effective_score, estimate_pairs_fast_lt_slow, objective_score, BackendUsed, DoubleMaScoreMode,
    VdpsTelemetry,
};

#[inline]
pub(crate) fn vdps_target_pairs(total_pairs: usize, rung_fraction: f64) -> usize {
    if total_pairs == 0 {
        return 0;
    }
    let frac = if rung_fraction.is_finite() {
        rung_fraction.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let target = ((total_pairs as f64) * frac).ceil() as usize;
    target.max(1).min(total_pairs)
}

pub(crate) fn vdps_decimated_periods(periods: &[u16], stride: usize) -> Vec<u16> {
    if periods.is_empty() {
        return Vec::new();
    }
    let step = stride.max(1);
    let mut out: Vec<u16> =
        Vec::with_capacity(periods.len().saturating_div(step).saturating_add(2));
    out.push(periods[0]);
    for (i, &p) in periods.iter().enumerate() {
        if (i % step) == 0 {
            out.push(p);
        }
    }
    out.push(*periods.last().unwrap_or(&periods[0]));
    out.sort_unstable();
    out.dedup();
    out
}

pub(crate) fn vdps_period_window_slice(periods: &[u16], center: u16, window: u16) -> &[u16] {
    if periods.is_empty() {
        return periods;
    }
    let lo = center.saturating_sub(window);
    let hi = center.saturating_add(window);
    let start = periods.partition_point(|&x| x < lo);
    let end = periods.partition_point(|&x| x <= hi);
    &periods[start..end]
}

pub(crate) fn vdps_rank_candidates(
    mut rows: Vec<(DoubleMaParams, Metrics)>,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    limit: usize,
) -> Vec<(DoubleMaParams, Metrics)> {
    if rows.is_empty() || limit == 0 {
        return Vec::new();
    }
    rows.sort_by(|(pa, ma), (pb, mb)| {
        let sa = {
            let v = effective_score(ma, objective, score_mode);
            if v.is_finite() {
                v
            } else {
                f64::NEG_INFINITY
            }
        };
        let sb = {
            let v = effective_score(mb, objective, score_mode);
            if v.is_finite() {
                v
            } else {
                f64::NEG_INFINITY
            }
        };
        sb.total_cmp(&sa)
            .then_with(|| ma.max_dd.total_cmp(&mb.max_dd))
            .then_with(|| mb.trades.cmp(&ma.trades))
            .then_with(|| pa.fast_len.cmp(&pb.fast_len))
            .then_with(|| pa.slow_len.cmp(&pb.slow_len))
            .then_with(|| pa.fast_ma_id.cmp(&pb.fast_ma_id))
            .then_with(|| pa.slow_ma_id.cmp(&pb.slow_ma_id))
    });
    rows.dedup_by(|(pa, _), (pb, _)| {
        pa.fast_len == pb.fast_len
            && pa.slow_len == pb.slow_len
            && pa.fast_ma_id == pb.fast_ma_id
            && pa.slow_ma_id == pb.slow_ma_id
    });
    rows.truncate(limit.min(rows.len()));
    rows
}

pub(crate) fn vdps_local_anchor_axes(
    center_f: u16,
    center_s: u16,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_window: u16,
    slow_window: u16,
    max_period_candidates: usize,
    target_pairs: usize,
) -> (Vec<u16>, Vec<u16>) {
    let mut fast_candidates =
        vdps_period_window_slice(fast_periods, center_f, fast_window).to_vec();
    let mut slow_candidates =
        vdps_period_window_slice(slow_periods, center_s, slow_window).to_vec();
    if fast_candidates.is_empty() || slow_candidates.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let trim_around_center = |candidates: &mut Vec<u16>, center: u16| {
        if candidates.len() <= max_period_candidates {
            return;
        }
        candidates.sort_by_key(|&p| (p.abs_diff(center), p));
        candidates.truncate(max_period_candidates);
        candidates.sort_unstable();
        candidates.dedup();
    };
    trim_around_center(&mut fast_candidates, center_f);
    trim_around_center(&mut slow_candidates, center_s);

    let mut thin_stride = 1usize;
    let target_pairs = target_pairs.max(1);
    let mut fast_sel = fast_candidates.clone();
    let mut slow_sel = slow_candidates.clone();
    let mut pair_count = estimate_pairs_fast_lt_slow(&fast_sel, &slow_sel);
    while pair_count > target_pairs.saturating_mul(3)
        && (fast_sel.len() > 8 || slow_sel.len() > 8)
        && thin_stride < 128
    {
        thin_stride = thin_stride.saturating_add(1);
        fast_sel = vdps_decimated_periods(&fast_candidates, thin_stride);
        slow_sel = vdps_decimated_periods(&slow_candidates, thin_stride);
        if !fast_sel.contains(&center_f) {
            fast_sel.push(center_f);
            fast_sel.sort_unstable();
            fast_sel.dedup();
        }
        if !slow_sel.contains(&center_s) {
            slow_sel.push(center_s);
            slow_sel.sort_unstable();
            slow_sel.dedup();
        }
        pair_count = estimate_pairs_fast_lt_slow(&fast_sel, &slow_sel);
    }

    (fast_sel, slow_sel)
}

pub(crate) fn vdps_merge_partial_results(
    partials: Vec<OptimizationResult>,
    objective: ObjectiveKind,
    top_k: usize,
    num_candles: usize,
) -> Option<OptimizationResult> {
    if partials.is_empty() {
        return None;
    }

    let mut total_num_combos: usize = 0;
    let mut ranked_input: Vec<(DoubleMaParams, Metrics)> = Vec::new();
    for res in partials {
        total_num_combos = total_num_combos.saturating_add(res.num_combos);
        ranked_input.push((res.best_params.clone(), res.best_metrics.clone()));
        ranked_input.extend(res.top);
    }

    let ranked = vdps_rank_candidates(ranked_input, objective, None, top_k.max(1));
    let (best_params, best_metrics) = ranked.first().cloned()?;
    let top = if top_k > 0 { ranked } else { Vec::new() };
    let best_score = Some(effective_score(&best_metrics, objective, None));
    let top_scores = Some(
        top.iter()
            .map(|(_, metrics)| effective_score(metrics, objective, None))
            .collect(),
    );
    Some(OptimizationResult {
        best_params,
        best_metrics,
        best_score,
        top,
        top_scores,
        all: None,
        all_scores: None,
        heatmap: None,
        num_combos: total_num_combos,
        num_candles,
    })
}

pub(crate) fn vdps_inject_random_seed_pairs(
    current: &mut Vec<(u16, u16)>,
    fast: &[u16],
    slow: &[u16],
    target_count: usize,
    rng: &mut StdRng,
) {
    if target_count == 0 || fast.is_empty() || slow.is_empty() {
        return;
    }
    let mut seen: BTreeSet<(u16, u16)> = current.iter().copied().collect();
    let mut attempts: usize = 0;
    let max_attempts = target_count.saturating_mul(32).max(128);
    while attempts < max_attempts && seen.len() < current.len().saturating_add(target_count) {
        attempts = attempts.saturating_add(1);
        let f = fast[rng.gen_range(0..fast.len())];
        let s = slow[rng.gen_range(0..slow.len())];
        if f >= s {
            continue;
        }
        if seen.insert((f, s)) {
            current.push((f, s));
        }
    }
}

pub(crate) fn vdps_nearest_period(periods: &[u16], value: i32) -> Option<u16> {
    if periods.is_empty() {
        return None;
    }
    let v = value.clamp(0, u16::MAX as i32) as u16;
    let idx = periods.partition_point(|&p| p < v);
    if idx == 0 {
        return periods.first().copied();
    }
    if idx >= periods.len() {
        return periods.last().copied();
    }
    let lo = periods[idx - 1];
    let hi = periods[idx];
    if v.abs_diff(lo) <= v.abs_diff(hi) {
        Some(lo)
    } else {
        Some(hi)
    }
}

pub(crate) fn vdps_inject_elite_mutation_pairs(
    current: &mut Vec<(u16, u16)>,
    elite_seeds: &[(u16, u16)],
    fast: &[u16],
    slow: &[u16],
    target_count: usize,
    f_step: u16,
    s_step: u16,
    rng: &mut StdRng,
) {
    if target_count == 0 || elite_seeds.is_empty() || fast.is_empty() || slow.is_empty() {
        return;
    }
    let mut seen: BTreeSet<(u16, u16)> = current.iter().copied().collect();
    let mut attempts: usize = 0;
    let max_attempts = target_count.saturating_mul(48).max(192);
    let f_delta = (f_step.max(1) as i32).saturating_mul(2);
    let s_delta = (s_step.max(1) as i32).saturating_mul(2);
    while attempts < max_attempts && seen.len() < current.len().saturating_add(target_count) {
        attempts = attempts.saturating_add(1);
        let (base_f, base_s) = elite_seeds[rng.gen_range(0..elite_seeds.len())];
        let cand_f = (base_f as i32).saturating_add(rng.gen_range(-f_delta..=f_delta));
        let cand_s = (base_s as i32).saturating_add(rng.gen_range(-s_delta..=s_delta));
        let Some(f) = vdps_nearest_period(fast, cand_f) else {
            continue;
        };
        let Some(s) = vdps_nearest_period(slow, cand_s) else {
            continue;
        };
        if f >= s {
            continue;
        }
        if seen.insert((f, s)) {
            current.push((f, s));
        }
    }
}

pub(crate) fn vdps_inject_differential_seed_pairs(
    current: &mut Vec<(u16, u16)>,
    elite_seeds: &[(u16, u16)],
    fast: &[u16],
    slow: &[u16],
    target_count: usize,
    rng: &mut StdRng,
) {
    if target_count == 0 || elite_seeds.len() < 3 || fast.is_empty() || slow.is_empty() {
        return;
    }
    let mut seen: BTreeSet<(u16, u16)> = current.iter().copied().collect();
    let mut attempts: usize = 0;
    let max_attempts = target_count.saturating_mul(64).max(256);
    while attempts < max_attempts && seen.len() < current.len().saturating_add(target_count) {
        attempts = attempts.saturating_add(1);
        let ia = rng.gen_range(0..elite_seeds.len());
        let ib = rng.gen_range(0..elite_seeds.len());
        let ic = rng.gen_range(0..elite_seeds.len());
        if ia == ib || ib == ic || ia == ic {
            continue;
        }
        let (a_f, a_s) = elite_seeds[ia];
        let (b_f, b_s) = elite_seeds[ib];
        let (c_f, c_s) = elite_seeds[ic];
        let f_coeff = rng.gen_range(0.55..0.90);
        let s_coeff = rng.gen_range(0.55..0.90);
        let cand_f = (a_f as f64) + ((b_f as f64) - (c_f as f64)) * f_coeff;
        let cand_s = (a_s as f64) + ((b_s as f64) - (c_s as f64)) * s_coeff;
        let Some(f) = vdps_nearest_period(fast, cand_f.round() as i32) else {
            continue;
        };
        let Some(s) = vdps_nearest_period(slow, cand_s.round() as i32) else {
            continue;
        };
        if f >= s {
            continue;
        }
        if seen.insert((f, s)) {
            current.push((f, s));
        }
    }
}

pub(crate) fn merge_vdps_telemetry(
    target: &mut Option<VdpsTelemetry>,
    incoming: Option<VdpsTelemetry>,
) {
    let Some(mut inc) = incoming else {
        return;
    };
    if let Some(cur) = target.as_mut() {
        cur.evaluated_pairs_total = cur
            .evaluated_pairs_total
            .saturating_add(inc.evaluated_pairs_total);
        cur.full_grid_pairs = cur.full_grid_pairs.saturating_add(inc.full_grid_pairs);
        cur.full_fidelity_pairs_evaluated = cur
            .full_fidelity_pairs_evaluated
            .saturating_add(inc.full_fidelity_pairs_evaluated);
        cur.full_fidelity_pairs_saved = cur
            .full_fidelity_pairs_saved
            .saturating_add(inc.full_fidelity_pairs_saved);
        cur.runtime_ms_total = cur.runtime_ms_total.saturating_add(inc.runtime_ms_total);
        cur.quality_check_top_k_overlap = None;
        cur.quality_check_best_score_regret = None;
        cur.quality_check_baseline_runtime_ms = None;
        cur.quality_check_baseline_backend = None;
        cur.quality_check_baseline_pairs = None;
        if cur.rungs.len() == inc.rungs.len() {
            for (a, b) in cur.rung_stats.iter_mut().zip(inc.rung_stats.iter()) {
                a.target_pairs = a.target_pairs.saturating_add(b.target_pairs);
                a.evaluated_pairs = a.evaluated_pairs.saturating_add(b.evaluated_pairs);
                a.promoted_pairs = a.promoted_pairs.saturating_add(b.promoted_pairs);
                a.dropped_pairs = a.dropped_pairs.saturating_add(b.dropped_pairs);
                a.fast_candidates = a.fast_candidates.saturating_add(b.fast_candidates);
                a.slow_candidates = a.slow_candidates.saturating_add(b.slow_candidates);
                a.runtime_ms = a.runtime_ms.saturating_add(b.runtime_ms);
                a.throughput_pairs_per_sec = if a.runtime_ms > 0 {
                    (a.evaluated_pairs as f64) * 1000.0 / (a.runtime_ms as f64)
                } else {
                    a.evaluated_pairs as f64
                };
                if matches!(b.backend_used, BackendUsed::Gpu) {
                    a.backend_used = BackendUsed::Gpu;
                }
            }
        } else {
            cur.rung_stats.append(&mut inc.rung_stats);
        }
    } else {
        *target = Some(inc);
    }
}
