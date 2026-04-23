use crate::double_ma::{compute_log_returns_f64, ma_matrix_cpu_f64};
use std::cmp::Ordering;
use std::collections::HashMap;

const MAX_SAMPLED_PERIODS: usize = 48;
const MAX_TIME_STEPS: usize = 20_000;
const MIN_OBS: usize = 32;
const EPS: f64 = 1e-12;

fn sample_periods_by_stride(periods: &[u16], stride: u16) -> Vec<u16> {
    let mut normalized = periods.to_vec();
    normalized.sort_unstable();
    normalized.dedup();
    if normalized.is_empty() {
        return Vec::new();
    }
    if stride <= 1 {
        return normalized;
    }
    let mut out: Vec<u16> = Vec::new();
    let mut next_keep = normalized[0];
    for &p in &normalized {
        if p >= next_keep {
            out.push(p);
            next_keep = p.saturating_add(stride);
        }
    }
    if out.last().copied() != normalized.last().copied() {
        out.push(*normalized.last().unwrap_or(&normalized[0]));
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

/// Inputs for the crate-private predictive prescan rescue heuristic.
///
/// The heuristic computes MA series from `ma_source`, scores each `(fast, slow)` pair
/// against the next-close log return, and returns up to `top_k` strictly ordered
/// `fast < slow` seeds. When `long_only` is enabled, bearish bars are skipped from the
/// score denominator instead of being averaged as zero. Time sampling is approximate:
/// `period_stride` and `MAX_TIME_STEPS` cap work without guaranteeing an exact ceiling.
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

/// Scores sampled `(fast, slow)` MA pairs and returns the highest-ranked seeds.
///
/// The scoring target is the next-close log return. Duplicate or unsorted period domains
/// are normalized before scoring so the output remains unique on the common
/// `period_stride <= 1` path.
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
    let ma_prices =
        vector_ta::utilities::data_loader::source_type(params.candles, params.ma_source);
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

#[cfg(test)]
mod tests {
    use super::{
        cap_periods_even, predictive_prescan_seed_pairs, sample_periods_by_stride,
        PredictivePrescanParams,
    };
    use std::collections::HashSet;
    use vector_ta::utilities::data_loader::Candles;

    fn make_trending_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);
        let mut price = 100.0_f64;
        for i in 0..len {
            let drift = 0.18 + (i as f64 * 0.003).sin() * 0.04;
            price += drift;
            let open_px = price - 0.12;
            let close_px = price + 0.12;
            ts.push(i as i64);
            open.push(open_px);
            high.push(close_px + 0.18);
            low.push(open_px - 0.18);
            close.push(close_px);
            vol.push(1_000.0 + i as f64);
        }
        Candles::new(ts, open, high, low, close, vol)
    }

    #[test]
    fn sample_periods_by_stride_keeps_endpoints_and_stride_steps() {
        let periods = vec![5, 6, 7, 9, 13, 14, 20];
        assert_eq!(sample_periods_by_stride(&periods, 1), periods);
        assert_eq!(sample_periods_by_stride(&periods, 4), vec![5, 9, 13, 20]);
    }

    #[test]
    fn sample_periods_by_stride_normalizes_unsorted_duplicate_domains() {
        let periods = vec![13, 5, 8, 5, 21, 13];
        assert_eq!(sample_periods_by_stride(&periods, 1), vec![5, 8, 13, 21]);
    }

    #[test]
    fn cap_periods_even_preserves_extremes_and_evenly_spaced_samples() {
        let capped = cap_periods_even(vec![5, 7, 9, 11, 13, 15, 17], 4);
        assert_eq!(capped, vec![5, 9, 13, 17]);
    }

    #[test]
    fn predictive_prescan_seed_pairs_returns_ranked_long_only_pairs() {
        let candles = make_trending_candles(512);
        let pairs = predictive_prescan_seed_pairs(PredictivePrescanParams {
            candles: &candles,
            ma_source: "close",
            fast_periods: &[5, 8, 13, 21],
            slow_periods: &[34, 55, 89],
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_ma_params: None,
            slow_ma_params: None,
            fast_alma_offset: 0.85,
            fast_alma_sigma: 6.0,
            slow_alma_offset: 0.85,
            slow_alma_sigma: 6.0,
            long_only: true,
            top_k: 3,
            period_stride: 2,
        })
        .expect("predictive prescan");
        let top_one = predictive_prescan_seed_pairs(PredictivePrescanParams {
            candles: &candles,
            ma_source: "close",
            fast_periods: &[5, 8, 13, 21],
            slow_periods: &[34, 55, 89],
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_ma_params: None,
            slow_ma_params: None,
            fast_alma_offset: 0.85,
            fast_alma_sigma: 6.0,
            slow_alma_offset: 0.85,
            slow_alma_sigma: 6.0,
            long_only: true,
            top_k: 1,
            period_stride: 2,
        })
        .expect("predictive prescan top one");

        assert!(!pairs.is_empty());
        assert!(pairs.len() <= 3);
        assert!(pairs.iter().all(|(fast, slow)| fast < slow));
        assert_eq!(top_one, pairs.iter().copied().take(1).collect::<Vec<_>>());
    }

    #[test]
    fn predictive_prescan_seed_pairs_normalizes_duplicate_unsorted_domains() {
        let candles = make_trending_candles(512);
        let baseline = predictive_prescan_seed_pairs(PredictivePrescanParams {
            candles: &candles,
            ma_source: "close",
            fast_periods: &[5, 8, 13, 21],
            slow_periods: &[34, 55, 89],
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_ma_params: None,
            slow_ma_params: None,
            fast_alma_offset: 0.85,
            fast_alma_sigma: 6.0,
            slow_alma_offset: 0.85,
            slow_alma_sigma: 6.0,
            long_only: true,
            top_k: 5,
            period_stride: 1,
        })
        .expect("baseline predictive prescan");
        let normalized = predictive_prescan_seed_pairs(PredictivePrescanParams {
            candles: &candles,
            ma_source: "close",
            fast_periods: &[13, 5, 8, 5, 21, 13],
            slow_periods: &[89, 34, 55, 34],
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_ma_params: None,
            slow_ma_params: None,
            fast_alma_offset: 0.85,
            fast_alma_sigma: 6.0,
            slow_alma_offset: 0.85,
            slow_alma_sigma: 6.0,
            long_only: true,
            top_k: 5,
            period_stride: 1,
        })
        .expect("duplicate-domain predictive prescan");

        let unique_count = normalized.iter().copied().collect::<HashSet<_>>().len();
        assert_eq!(normalized, baseline);
        assert_eq!(unique_count, normalized.len());
    }

    #[test]
    fn predictive_prescan_seed_pairs_returns_empty_for_disabled_or_empty_inputs() {
        let candles = make_trending_candles(256);
        let disabled = predictive_prescan_seed_pairs(PredictivePrescanParams {
            candles: &candles,
            ma_source: "close",
            fast_periods: &[5, 8, 13],
            slow_periods: &[21, 34, 55],
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_ma_params: None,
            slow_ma_params: None,
            fast_alma_offset: 0.85,
            fast_alma_sigma: 6.0,
            slow_alma_offset: 0.85,
            slow_alma_sigma: 6.0,
            long_only: true,
            top_k: 0,
            period_stride: 2,
        })
        .expect("disabled predictive prescan");
        assert!(disabled.is_empty());

        let empty_fast = predictive_prescan_seed_pairs(PredictivePrescanParams {
            candles: &candles,
            ma_source: "close",
            fast_periods: &[],
            slow_periods: &[21, 34, 55],
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_ma_params: None,
            slow_ma_params: None,
            fast_alma_offset: 0.85,
            fast_alma_sigma: 6.0,
            slow_alma_offset: 0.85,
            slow_alma_sigma: 6.0,
            long_only: true,
            top_k: 3,
            period_stride: 2,
        })
        .expect("empty fast-period predictive prescan");
        assert!(empty_fast.is_empty());
    }
}
