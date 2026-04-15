use crate::progress::ProgressSink;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

fn default_slices() -> usize {
    16
}

fn default_max_combinations() -> usize {
    50_000
}

fn default_threshold() -> f64 {
    0.0
}

#[derive(Clone, Debug, Deserialize)]
pub struct CscvPboConfig {
    #[serde(default = "default_slices")]
    pub slices: usize,
    #[serde(default = "default_max_combinations")]
    pub max_combinations: usize,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SliceReturnStats {
    pub sum: f64,
    pub sumsq: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CscvCandidatePboStats {
    pub is_selected_frac: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oos_rank_frac_p50: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oos_metric_p50: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CscvPboResult {
    pub slices: usize,
    pub slice_bars: usize,
    pub candidates: usize,
    pub combinations: usize,
    pub combinations_possible: u64,
    pub pbo: f64,
    pub lambda_p25: f64,
    pub lambda_p50: f64,
    pub lambda_p75: f64,
    pub degradation_slope: f64,
    pub degradation_intercept: f64,
    pub degradation_r2: f64,
    pub threshold: f64,
    pub prob_oos_below_threshold: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
}

fn sanitize_slices(slices: usize) -> usize {
    let s = slices.clamp(4, 32);
    if (s & 1) == 0 {
        s
    } else {
        s.saturating_sub(1).max(4)
    }
}

fn n_choose_k(n: usize, k: usize) -> u128 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    if k == 0 {
        return 1;
    }
    let mut r: u128 = 1;
    for i in 1..=k {
        r = r * (n - k + i) as u128 / i as u128;
    }
    r
}

fn all_k_masks(s: usize, k: usize) -> Vec<u64> {
    if s == 0 || k == 0 || k > s || s > 63 {
        return Vec::new();
    }
    let limit: u64 = 1u64 << s;
    let mut x: u64 = (1u64 << k) - 1;
    let mut out: Vec<u64> = Vec::new();
    while x < limit {
        out.push(x);
        // Gosper's hack: next higher number with same bitcount.
        let u = x & x.wrapping_neg();
        let v = x.wrapping_add(u);
        if v == 0 || v >= limit {
            break;
        }
        x = v.wrapping_add(((v ^ x) / u) >> 2);
    }
    out
}

fn sample_k_masks(s: usize, k: usize, take: usize, seed: u64) -> Vec<u64> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut idx: Vec<u8> = (0..(s as u8)).collect();
    let mut seen: HashSet<u64> = HashSet::with_capacity(take.saturating_mul(2));
    let mut attempts: usize = 0;
    let max_attempts = take.saturating_mul(50).max(10_000);

    while seen.len() < take && attempts < max_attempts {
        attempts = attempts.saturating_add(1);
        idx.shuffle(&mut rng);
        let mut mask: u64 = 0;
        for i in 0..k {
            mask |= 1u64 << (idx[i] as u64);
        }
        seen.insert(mask);
    }

    let mut out: Vec<u64> = seen.into_iter().collect();
    out.sort_unstable();
    out
}

fn sharpe_from_sum_sumsq(n: usize, sum: f64, sumsq: f64) -> f64 {
    if n < 2 {
        return 0.0;
    }
    let nf = n as f64;
    let mean = sum / nf;
    let var = (sumsq - (sum * sum) / nf) / (nf - 1.0);
    if !var.is_finite() || var <= 0.0 {
        return 0.0;
    }
    mean / var.sqrt()
}

fn ranks_average_ascending(scores: &[f64]) -> Vec<f64> {
    let n = scores.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| scores[a].total_cmp(&scores[b]));

    let mut ranks = vec![0.0_f64; n];
    let mut i: usize = 0;
    while i < n {
        let mut j = i + 1;
        while j < n && scores[order[j]] == scores[order[i]] {
            j += 1;
        }
        // Ranks are 1-based positions.
        let rank_start = (i + 1) as f64;
        let rank_end = j as f64;
        let avg = (rank_start + rank_end) * 0.5;
        for &idx in &order[i..j] {
            ranks[idx] = avg;
        }
        i = j;
    }
    ranks
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let qq = q.clamp(0.0, 1.0);
    let idx = (qq * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn regression_slope_intercept_r2(x: &[f64], y: &[f64]) -> (f64, f64, f64) {
    let n = x.len().min(y.len());
    if n < 2 {
        return (0.0, 0.0, 0.0);
    }
    let mx = x.iter().take(n).sum::<f64>() / n as f64;
    let my = y.iter().take(n).sum::<f64>() / n as f64;
    let mut var_x = 0.0_f64;
    let mut cov = 0.0_f64;
    for i in 0..n {
        let dx = x[i] - mx;
        let dy = y[i] - my;
        var_x += dx * dx;
        cov += dx * dy;
    }
    if !var_x.is_finite() || var_x <= 0.0 {
        return (0.0, my, 0.0);
    }
    let slope = cov / var_x;
    let intercept = my - slope * mx;
    let mut sse = 0.0_f64;
    let mut sst = 0.0_f64;
    for i in 0..n {
        let yhat = intercept + slope * x[i];
        let e = y[i] - yhat;
        sse += e * e;
        let dy = y[i] - my;
        sst += dy * dy;
    }
    let r2 = if sst > 0.0 && sst.is_finite() {
        1.0 - sse / sst
    } else {
        0.0
    };
    (
        slope.if_finite_or(0.0),
        intercept.if_finite_or(0.0),
        r2.if_finite_or(0.0),
    )
}

trait FiniteOr {
    fn if_finite_or(self, fallback: f64) -> f64;
}

impl FiniteOr for f64 {
    fn if_finite_or(self, fallback: f64) -> f64 {
        if self.is_finite() {
            self
        } else {
            fallback
        }
    }
}

pub fn compute_cscv_pbo_from_slices(
    slices_by_candidate: &[Vec<SliceReturnStats>],
    slice_bars: usize,
    cfg: &CscvPboConfig,
    seed: u64,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<(CscvPboResult, Vec<CscvCandidatePboStats>), String> {
    let candidates = slices_by_candidate.len();
    let slices = sanitize_slices(cfg.slices);

    if candidates < 2 {
        return Ok((
            CscvPboResult {
                slices,
                slice_bars,
                candidates,
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
                skipped_reason: Some("CSCV/PBO requires at least 2 candidates.".to_string()),
            },
            Vec::new(),
        ));
    }
    if slice_bars < 2 {
        return Ok((
            CscvPboResult {
                slices,
                slice_bars,
                candidates,
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
                skipped_reason: Some("CSCV/PBO requires slice_bars >= 2.".to_string()),
            },
            Vec::new(),
        ));
    }
    for (i, v) in slices_by_candidate.iter().enumerate() {
        if v.len() != slices {
            return Err(format!(
                "CSCV/PBO candidate {i} has {} slices, expected {slices}.",
                v.len()
            ));
        }
    }

    let k = slices / 2;
    let combos_possible_u128 = n_choose_k(slices, k);
    let combinations_possible = combos_possible_u128.min(u64::MAX as u128) as u64;
    let max = cfg.max_combinations.max(1);

    let masks: Vec<u64> = if combos_possible_u128 <= max as u128 {
        all_k_masks(slices, k)
    } else {
        sample_k_masks(slices, k, max.min(combinations_possible as usize), seed)
    };
    let combos = masks.len();
    if combos == 0 {
        return Ok((
            CscvPboResult {
                slices,
                slice_bars,
                candidates,
                combinations: 0,
                combinations_possible,
                pbo: 0.0,
                lambda_p25: 0.0,
                lambda_p50: 0.0,
                lambda_p75: 0.0,
                degradation_slope: 0.0,
                degradation_intercept: 0.0,
                degradation_r2: 0.0,
                threshold: cfg.threshold,
                prob_oos_below_threshold: 0.0,
                skipped_reason: Some("CSCV/PBO produced 0 combinations.".to_string()),
            },
            Vec::new(),
        ));
    }

    let total_obs = slice_bars * k;
    let eps = 1e-12_f64;

    let mut lambdas: Vec<f64> = Vec::with_capacity(combos);
    let mut rn: Vec<f64> = Vec::with_capacity(combos);
    let mut rbn: Vec<f64> = Vec::with_capacity(combos);

    let mut selected_counts: Vec<usize> = vec![0; candidates];
    let mut selected_oos_rank_frac: Vec<Vec<f64>> = vec![Vec::new(); candidates];
    let mut selected_oos_metric: Vec<Vec<f64>> = vec![Vec::new(); candidates];

    let mut pbo_hits: usize = 0;
    let mut below_threshold: usize = 0;

    let mut is_scores: Vec<f64> = vec![0.0; candidates];
    let mut oos_scores: Vec<f64> = vec![0.0; candidates];

    crate::progress::emit_double_ma_progress(progress, 0, combos, "composite_validate_cscv");
    for (ci, &mask) in masks.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let oos_mask = (!mask) & ((1u64 << slices) - 1);
        for n in 0..candidates {
            let mut sum_is = 0.0_f64;
            let mut sumsq_is = 0.0_f64;
            let mut sum_oos = 0.0_f64;
            let mut sumsq_oos = 0.0_f64;
            for s in 0..slices {
                let bit = 1u64 << s;
                let st = slices_by_candidate[n][s];
                if (mask & bit) != 0 {
                    sum_is += st.sum;
                    sumsq_is += st.sumsq;
                } else if (oos_mask & bit) != 0 {
                    sum_oos += st.sum;
                    sumsq_oos += st.sumsq;
                }
            }
            is_scores[n] = sharpe_from_sum_sumsq(total_obs, sum_is, sumsq_is);
            oos_scores[n] = sharpe_from_sum_sumsq(total_obs, sum_oos, sumsq_oos);
        }

        // which.max: first max index
        let mut n_star = 0usize;
        let mut best = is_scores[0];
        for n in 1..candidates {
            let v = is_scores[n];
            if v > best {
                best = v;
                n_star = n;
            }
        }

        let ranks = ranks_average_ascending(&oos_scores);
        let os_rank = ranks[n_star];
        let omega = (os_rank / candidates as f64).clamp(eps, 1.0 - eps);
        let lambda = (omega / (1.0 - omega)).ln();

        if lambda <= 0.0 {
            pbo_hits += 1;
        }
        let rbn_val = oos_scores[n_star];
        if rbn_val < cfg.threshold {
            below_threshold += 1;
        }

        lambdas.push(lambda);
        rn.push(is_scores[n_star]);
        rbn.push(rbn_val);

        selected_counts[n_star] = selected_counts[n_star].saturating_add(1);
        selected_oos_rank_frac[n_star].push(os_rank / candidates as f64);
        selected_oos_metric[n_star].push(rbn_val);

        let done = ci.saturating_add(1);
        if (done & 0x3F) == 0 || done == combos {
            crate::progress::emit_double_ma_progress(
                progress,
                done,
                combos,
                "composite_validate_cscv",
            );
        }
    }

    let pbo = pbo_hits as f64 / combos as f64;
    let prob_oos_below_threshold = below_threshold as f64 / combos as f64;

    let mut lambda_sorted = lambdas.clone();
    lambda_sorted.sort_by(|a, b| a.total_cmp(b));
    let lambda_p25 = quantile(&lambda_sorted, 0.25);
    let lambda_p50 = quantile(&lambda_sorted, 0.5);
    let lambda_p75 = quantile(&lambda_sorted, 0.75);

    let (slope, intercept, r2) = regression_slope_intercept_r2(&rn, &rbn);

    let mut cand_stats: Vec<CscvCandidatePboStats> = Vec::with_capacity(candidates);
    for n in 0..candidates {
        let sel = selected_counts[n];
        let frac = sel as f64 / combos as f64;
        if sel == 0 {
            cand_stats.push(CscvCandidatePboStats {
                is_selected_frac: frac,
                oos_rank_frac_p50: None,
                oos_metric_p50: None,
            });
            continue;
        }

        let mut rks = selected_oos_rank_frac[n].clone();
        rks.sort_by(|a, b| a.total_cmp(b));
        let mut ms = selected_oos_metric[n].clone();
        ms.sort_by(|a, b| a.total_cmp(b));
        cand_stats.push(CscvCandidatePboStats {
            is_selected_frac: frac,
            oos_rank_frac_p50: Some(quantile(&rks, 0.5)),
            oos_metric_p50: Some(quantile(&ms, 0.5)),
        });
    }

    Ok((
        CscvPboResult {
            slices,
            slice_bars,
            candidates,
            combinations: combos,
            combinations_possible,
            pbo: pbo.if_finite_or(0.0),
            lambda_p25: lambda_p25.if_finite_or(0.0),
            lambda_p50: lambda_p50.if_finite_or(0.0),
            lambda_p75: lambda_p75.if_finite_or(0.0),
            degradation_slope: slope,
            degradation_intercept: intercept,
            degradation_r2: r2,
            threshold: cfg.threshold,
            prob_oos_below_threshold: prob_oos_below_threshold.if_finite_or(0.0),
            skipped_reason: None,
        },
        cand_stats,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cscv_pbo_overfit_pattern_has_expected_phi() {
        // 3 candidates, 4 slices, 2 slices per IS/OOS.
        // Candidate0 wins when IS picks {0,2} and loses OOS.
        // Candidate1 wins when IS picks {1,3} and loses OOS.
        // Other splits tie -> not overfit in rank-average scheme.
        let slices = 4usize;
        let slice_bars = 10usize;
        let pos = SliceReturnStats {
            sum: 10.0,
            sumsq: 20.0,
        };
        let neg = SliceReturnStats {
            sum: -10.0,
            sumsq: 20.0,
        };
        let zer = SliceReturnStats {
            sum: 0.0,
            sumsq: 0.0,
        };

        let c0 = vec![pos, neg, pos, neg];
        let c1 = vec![neg, pos, neg, pos];
        let c2 = vec![zer; slices];
        let by_cand = vec![c0, c1, c2];

        let cfg = CscvPboConfig {
            slices,
            max_combinations: 1000,
            threshold: 0.0,
        };
        let cancel = AtomicBool::new(false);
        let (res, cand) =
            compute_cscv_pbo_from_slices(&by_cand, slice_bars, &cfg, 1, &cancel, None)
                .expect("cscv");

        assert_eq!(res.combinations, 6);
        let expected = 2.0 / 6.0;
        assert!((res.pbo - expected).abs() < 1e-9);
        assert_eq!(cand.len(), 3);
        assert!((cand[0].is_selected_frac - (5.0 / 6.0)).abs() < 1e-9);
        assert!((cand[1].is_selected_frac - (1.0 / 6.0)).abs() < 1e-9);
        assert!((cand[2].is_selected_frac - 0.0).abs() < 1e-12);
        assert!(cand[2].oos_rank_frac_p50.is_none());
    }
}
