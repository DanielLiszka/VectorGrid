use crate::double_ma::{compute_log_returns_f64, ma_matrix_cpu_f64};
use std::cmp::Ordering;
use std::collections::HashMap;

const MAX_SAMPLED_PERIODS: usize = 48;
const MAX_TIME_STEPS: usize = 20_000;
const MIN_OBS: usize = 32;
const EPS: f64 = 1e-12;

fn sample_periods_by_stride(periods: &[u16], stride: u16) -> Vec<u16> {
    if periods.is_empty() {
        return Vec::new();
    }
    if stride <= 1 {
        return periods.to_vec();
    }
    let mut out: Vec<u16> = Vec::new();
    let mut next_keep = periods[0];
    for &p in periods {
        if p >= next_keep {
            out.push(p);
            next_keep = p.saturating_add(stride);
        }
    }
    if out.last().copied() != periods.last().copied() {
        out.push(*periods.last().unwrap_or(&periods[0]));
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn cap_periods_even(mut periods: Vec<u16>, cap: usize) -> Vec<u16> {
    if periods.len() <= cap {
        return periods;
    }
    periods.sort_unstable();
    periods.dedup();
    if periods.len() <= cap {
        return periods;
    }

    let last_idx = periods.len().saturating_sub(1);
    if cap <= 1 || last_idx == 0 {
        return vec![periods[0]];
    }

    let mut out: Vec<u16> = Vec::with_capacity(cap);
    for i in 0..cap {
        let idx = i.saturating_mul(last_idx) / (cap - 1);
        out.push(periods[idx]);
    }
    out.sort_unstable();
    out.dedup();
    out
}

pub(crate) struct PredictivePrescanParams<'a> {
    pub candles: &'a vector_ta::utilities::data_loader::Candles,
    pub ma_source: &'a str,
    pub fast_periods: &'a [u16],
    pub slow_periods: &'a [u16],
    pub fast_ma_type: &'a str,
    pub slow_ma_type: &'a str,
    pub fast_ma_params: Option<&'a HashMap<String, f64>>,
    pub slow_ma_params: Option<&'a HashMap<String, f64>>,
    pub fast_alma_offset: f64,
    pub fast_alma_sigma: f64,
    pub slow_alma_offset: f64,
    pub slow_alma_sigma: f64,
    pub long_only: bool,
    pub top_k: usize,
    pub period_stride: u16,
}

pub(crate) fn predictive_prescan_seed_pairs(
    params: PredictivePrescanParams<'_>,
) -> Result<Vec<(u16, u16)>, String> {
    if params.top_k == 0 {
        return Ok(Vec::new());
    }
    if params.fast_periods.is_empty() || params.slow_periods.is_empty() {
        return Ok(Vec::new());
    }

    let sampled_fast = cap_periods_even(
        sample_periods_by_stride(params.fast_periods, params.period_stride),
        MAX_SAMPLED_PERIODS,
    );
    let sampled_slow = cap_periods_even(
        sample_periods_by_stride(params.slow_periods, params.period_stride),
        MAX_SAMPLED_PERIODS,
    );
    if sampled_fast.is_empty() || sampled_slow.is_empty() {
        return Ok(Vec::new());
    }

    let close = vector_ta::utilities::data_loader::source_type(params.candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(params.candles, params.ma_source);
    if close.len() != ma_prices.len() || close.len() < 3 {
        return Ok(Vec::new());
    }
    let lr = compute_log_returns_f64(close);
    let n = lr.len();
    if n < 3 {
        return Ok(Vec::new());
    }
    let time_stride = ((n.saturating_sub(1)) / MAX_TIME_STEPS).max(1);

    let (fast_mat, cols_fast) = ma_matrix_cpu_f64(
        params.candles,
        params.fast_ma_type,
        params.ma_source,
        &sampled_fast,
        params.fast_ma_params,
        params.fast_alma_offset,
        params.fast_alma_sigma,
    )?;
    let (slow_mat, cols_slow) = ma_matrix_cpu_f64(
        params.candles,
        params.slow_ma_type,
        params.ma_source,
        &sampled_slow,
        params.slow_ma_params,
        params.slow_alma_offset,
        params.slow_alma_sigma,
    )?;
    if cols_fast != n || cols_slow != n {
        return Err("predictive prescan MA matrix length mismatch".to_string());
    }

    let mut scored: Vec<(f64, u16, u16)> = Vec::new();
    for (fi, &fast_len) in sampled_fast.iter().enumerate() {
        let fast_row = &fast_mat[fi * n..(fi + 1) * n];
        for (si, &slow_len) in sampled_slow.iter().enumerate() {
            if fast_len >= slow_len {
                continue;
            }
            let slow_row = &slow_mat[si * n..(si + 1) * n];
            let start_t = (fast_len as usize).max(slow_len as usize).max(1);
            if start_t + 1 >= n {
                continue;
            }

            let mut sum_edge = 0.0_f64;
            let mut obs: usize = 0;
            let mut t = start_t;
            while t + 1 < n {
                let f = fast_row[t];
                let s = slow_row[t];
                if f.is_finite() && s.is_finite() {
                    let diff = f - s;
                    if diff.abs() > EPS {
                        let mut sign = if diff > 0.0 { 1.0 } else { -1.0 };
                        if params.long_only && sign < 0.0 {
                            sign = 0.0;
                        }
                        if sign != 0.0 {
                            let next_lr = lr[t + 1];
                            if next_lr.is_finite() {
                                sum_edge += sign * next_lr;
                                obs = obs.saturating_add(1);
                            }
                        }
                    }
                }
                t = t.saturating_add(time_stride);
            }

            if obs >= MIN_OBS {
                let score = sum_edge / obs as f64;
                if score.is_finite() && score > 0.0 {
                    scored.push((score, fast_len, slow_len));
                }
            }
        }
    }

    if scored.is_empty() {
        return Ok(Vec::new());
    }

    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    let mut out: Vec<(u16, u16)> = Vec::with_capacity(params.top_k.min(scored.len()));
    for (_, f, s) in scored.into_iter().take(params.top_k) {
        out.push((f, s));
    }
    Ok(out)
}

