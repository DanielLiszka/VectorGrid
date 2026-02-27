use crate::composite_validate::{
    PerturbConfig, PerturbStats, TradeBootstrapStats, WalkForwardConfigOptional, WalkForwardStats,
};
use crate::cscv_pbo::{
    compute_cscv_pbo_from_slices, CscvCandidatePboStats, CscvPboConfig, CscvPboResult,
    SliceReturnStats,
};
use crate::iterative_sim::{
    simulate_iterative_from_signals, IterativeMetricsDelta, IterativeSimConfig, OhlcSlice,
    ValidationEngine,
};
use crate::progress::ProgressSink;
use crate::stats_common::{median, median_u32, quantile, worst_max, worst_min, worst_min_u32};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use ta_strategies::double_ma::{Metrics, StrategyConfig, Trade};

pub trait ValidationPostCandidate {
    fn in_sample_metrics(&self) -> &Metrics;
    fn iterative_in_sample_metrics(&self) -> Option<&Metrics>;
    fn walk_forward_sharpe_maxdd(&self) -> Option<(f64, f64)>;
    fn cscv_oos_metric_p50(&self) -> Option<f64>;

    fn psr(&self) -> Option<f64>;
    fn dsr(&self) -> Option<f64>;
    fn set_psr(&mut self, v: Option<f64>);
    fn set_dsr(&mut self, v: Option<f64>);

    fn robust_score(&self) -> f64;
    fn set_robust_score(&mut self, v: f64);
    fn set_robust_pareto(&mut self, v: bool);
    fn set_cscv_pbo(&mut self, v: Option<CscvCandidatePboStats>);
}

pub fn robust_score_from_metrics(in_sample: &Metrics, wf: Option<(f64, f64)>) -> f64 {
    if let Some((sharpe_median, max_dd_worst)) = wf {
        sharpe_median - 2.0 * max_dd_worst
    } else {
        in_sample.sharpe - 2.0 * in_sample.max_dd
    }
}

pub fn fold_cscv_metric(base: f64, oos_metric_p50: Option<f64>) -> f64 {
    match oos_metric_p50 {
        Some(oos) if oos.is_finite() => base.min(oos),
        _ => base,
    }
}

pub fn compute_probability_metrics_and_gate<C: ValidationPostCandidate>(
    out: &mut Vec<C>,
    sample_obs: &[usize],
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
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
        let obs = sample_obs.get(i).copied().unwrap_or(2).max(2);
        let (psr, dsr) =
            crate::validation_stats::compute_psr_dsr(m.sharpe, obs, trials, peer_mean, peer_std);
        cand.set_psr(psr);
        cand.set_dsr(dsr);
    }

    if min_psr.is_some() || min_dsr.is_some() {
        out.retain(|cand| {
            crate::validation_stats::passes_probability_gates(
                cand.psr(),
                cand.dsr(),
                min_psr,
                min_dsr,
            )
        });
    }
}

pub fn mark_robust_pareto<C: ValidationPostCandidate>(out: &mut [C]) {
    for i in 0..out.len() {
        let (xi, yi) = out[i].walk_forward_sharpe_maxdd().unwrap_or_else(|| {
            let m = out[i].in_sample_metrics();
            (m.sharpe, m.max_dd)
        });

        let mut dominated = false;
        for j in 0..out.len() {
            if i == j {
                continue;
            }
            let (xj, yj) = out[j].walk_forward_sharpe_maxdd().unwrap_or_else(|| {
                let m = out[j].in_sample_metrics();
                (m.sharpe, m.max_dd)
            });
            if xj >= xi && yj <= yi && (xj > xi || yj < yi) {
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

    let mut sharpes: Vec<f64> = Vec::new();
    let mut dds: Vec<f64> = Vec::new();
    let mut pnls: Vec<f64> = Vec::new();
    let mut trades: Vec<u32> = Vec::new();

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
        trades.push(m.trades);
        if end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }

    sharpes.sort_by(|a, b| a.total_cmp(b));
    dds.sort_by(|a, b| a.total_cmp(b));
    pnls.sort_by(|a, b| a.total_cmp(b));

    WalkForwardStats {
        windows: sharpes.len(),
        sharpe_median: median(&sharpes),
        sharpe_worst: worst_min(&sharpes),
        max_dd_median: median(&dds),
        max_dd_worst: worst_max(&dds),
        pnl_median: median(&pnls),
        pnl_worst: worst_min(&pnls),
        trades_median: median_u32(&trades),
        trades_worst: worst_min_u32(&trades),
    }
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

    let mut sharpes: Vec<f64> = Vec::with_capacity(iters);
    let mut dds: Vec<f64> = Vec::with_capacity(iters);
    let mut pnls: Vec<f64> = Vec::with_capacity(iters);
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
        sharpes.push(m.sharpe);
        dds.push(m.max_dd);
        pnls.push(m.pnl);
    }

    Ok(Some(summarize_perturb_stats(
        cfg, iters, &sharpes, &dds, &pnls, pnl_pos,
    )))
}

pub fn compute_trade_bootstrap_stats(
    trades: &[Trade],
    simulations: usize,
    rng: &mut StdRng,
) -> TradeBootstrapStats {
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
    pnls.sort_by(|a, b| a.total_cmp(b));
    dds.sort_by(|a, b| a.total_cmp(b));
    TradeBootstrapStats {
        simulations,
        trade_count,
        prob_pnl_positive: pos as f64 / simulations as f64,
        pnl_p05: quantile(&pnls, 0.05),
        pnl_p50: quantile(&pnls, 0.5),
        pnl_p95: quantile(&pnls, 0.95),
        max_dd_p50: quantile(&dds, 0.5),
        max_dd_p95: quantile(&dds, 0.95),
    }
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
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
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
    compute_probability_metrics_and_gate(out, sample_obs, min_psr, min_dsr);
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let b = robust_score_from_metrics(&m, Some((2.0, 0.3)));
        assert!((a - 0.6).abs() < 1e-12);
        assert!((b - 1.4).abs() < 1e-12);
    }

    #[test]
    fn fold_cscv_metric_uses_oos_as_lower_bound() {
        assert!((fold_cscv_metric(1.0, Some(0.25)) - 0.25).abs() < 1e-12);
        assert!((fold_cscv_metric(1.0, Some(2.0)) - 1.0).abs() < 1e-12);
        assert!((fold_cscv_metric(1.0, Some(f64::NAN)) - 1.0).abs() < 1e-12);
        assert!((fold_cscv_metric(1.0, None) - 1.0).abs() < 1e-12);
    }
}
