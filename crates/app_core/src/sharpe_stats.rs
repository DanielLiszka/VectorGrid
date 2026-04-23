use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharpeStats {
    pub psr: f64,
    pub dsr: f64,
}

pub fn effective_sample_count(raw_samples: usize, effective_samples: Option<usize>) -> usize {
    let raw_floor = raw_samples.max(2);
    effective_samples.unwrap_or(raw_floor).clamp(2, raw_floor)
}

pub fn effective_trial_count(raw_trials: usize, effective_trials: Option<usize>) -> usize {
    let raw_floor = raw_trials.max(1);
    effective_trials.unwrap_or(raw_floor).clamp(1, raw_floor)
}

fn normal_cdf(x: f64) -> f64 {
    // Abramowitz and Stegun 7.1.26
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * z);
    let d = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let p = 1.0
        - d * (((((1.330_274_429 * t - 1.821_255_978) * t + 1.781_477_937) * t - 0.356_563_782)
            * t
            + 0.319_381_530)
            * t);
    if x >= 0.0 {
        p
    } else {
        1.0 - p
    }
}

fn normal_inv_cdf(p: f64) -> f64 {
    // Peter J. Acklam's rational approximation.
    const A: [f64; 6] = [
        -3.969_683_028_665_376e+01,
        2.209_460_984_245_205e+02,
        -2.759_285_104_469_687e+02,
        1.383_577_518_672_69e+02,
        -3.066_479_806_614_716e+01,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e+01,
        1.615_858_368_580_409e+02,
        -1.556_989_798_598_866e+02,
        6.680_131_188_771_972e+01,
        -1.328_068_155_288_572e+01,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-03,
        -3.223_964_580_411_365e-01,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-03,
        3.224_671_290_700_398e-01,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];

    let pp = p.clamp(1e-12, 1.0 - 1e-12);
    let plow = 0.024_25;
    let phigh = 1.0 - plow;

    if pp < plow {
        let q = (-2.0 * pp.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if pp > phigh {
        let q = (-2.0 * (1.0 - pp).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else {
        let q = pp - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    }
}

/// Computes the Sharpe-family exceedance probability against a benchmark Sharpe ratio.
///
/// `kurtosis` is interpreted as raw Pearson kurtosis, not excess kurtosis. This helper keeps
/// the current `0.5` invalid-input fallback because wrapper-level policy still depends on it.
pub fn probabilistic_sharpe_ratio(
    sr_hat: f64,
    benchmark_sr: f64,
    samples: usize,
    skewness: f64,
    kurtosis: f64,
) -> f64 {
    if !sr_hat.is_finite() || !benchmark_sr.is_finite() || samples < 2 {
        return 0.5;
    }
    let n = samples as f64;
    let g3 = if skewness.is_finite() { skewness } else { 0.0 };
    let g4 = if kurtosis.is_finite() {
        kurtosis.max(1.0)
    } else {
        3.0
    };
    let denom = (1.0 - g3 * sr_hat + ((g4 - 1.0) / 4.0) * sr_hat * sr_hat).max(1e-12);
    let z = (sr_hat - benchmark_sr) * (n - 1.0).sqrt() / denom.sqrt();
    normal_cdf(z).clamp(0.0, 1.0)
}

pub fn probabilistic_sharpe_ratio_with_effective_samples(
    sr_hat: f64,
    benchmark_sr: f64,
    raw_samples: usize,
    effective_samples: Option<usize>,
    skewness: f64,
    kurtosis: f64,
) -> f64 {
    probabilistic_sharpe_ratio(
        sr_hat,
        benchmark_sr,
        effective_sample_count(raw_samples, effective_samples),
        skewness,
        kurtosis,
    )
}

pub fn mean_std(xs: &[f64]) -> (f64, f64) {
    if xs.is_empty() {
        return (0.0, 0.0);
    }
    let mut n = 0.0;
    let mut mean = 0.0;
    let mut m2 = 0.0;
    for &x in xs {
        if !x.is_finite() {
            continue;
        }
        n += 1.0;
        let d = x - mean;
        mean += d / n;
        let d2 = x - mean;
        m2 += d * d2;
    }
    if n <= 1.0 {
        return (mean, 0.0);
    }
    (mean, (m2 / (n - 1.0)).sqrt())
}

/// Estimates the expected maximum Sharpe ratio across peer trials using the same normal-quantile
/// helper as `deflated_sharpe_ratio`.
pub fn expected_max_sharpe(num_trials: usize, sr_mean: f64, sr_std: f64) -> f64 {
    if num_trials <= 1 || !sr_mean.is_finite() || !sr_std.is_finite() {
        return sr_mean;
    }
    let n = num_trials as f64;
    let z1 = normal_inv_cdf(1.0 - (1.0 / n));
    let z2 = normal_inv_cdf(1.0 - (1.0 / (n * std::f64::consts::E)));
    sr_mean + sr_std.max(0.0) * ((1.0 - 0.577_215_664_9) * z1 + 0.577_215_664_9 * z2)
}

pub fn deflated_sharpe_ratio(
    sr_hat: f64,
    samples: usize,
    skewness: f64,
    kurtosis: f64,
    num_trials: usize,
    peer_sr_mean: f64,
    peer_sr_std: f64,
) -> f64 {
    let sr_star = expected_max_sharpe(num_trials.max(1), peer_sr_mean, peer_sr_std);
    probabilistic_sharpe_ratio(sr_hat, sr_star, samples, skewness, kurtosis)
}

pub fn deflated_sharpe_ratio_with_effective_counts(
    sr_hat: f64,
    raw_samples: usize,
    effective_samples: Option<usize>,
    skewness: f64,
    kurtosis: f64,
    raw_trials: usize,
    effective_trials: Option<usize>,
    peer_sr_mean: f64,
    peer_sr_std: f64,
) -> f64 {
    let sr_star = expected_max_sharpe(
        effective_trial_count(raw_trials, effective_trials),
        peer_sr_mean,
        peer_sr_std,
    );
    probabilistic_sharpe_ratio_with_effective_samples(
        sr_hat,
        sr_star,
        raw_samples,
        effective_samples,
        skewness,
        kurtosis,
    )
}

pub fn minimum_track_record_length(
    sr_hat: f64,
    benchmark_sr: f64,
    skewness: f64,
    kurtosis: f64,
    confidence_level: f64,
) -> Option<usize> {
    if !sr_hat.is_finite()
        || !benchmark_sr.is_finite()
        || !confidence_level.is_finite()
        || confidence_level <= 0.0
        || confidence_level >= 1.0
        || sr_hat <= benchmark_sr
    {
        return None;
    }

    let g3 = if skewness.is_finite() { skewness } else { 0.0 };
    let g4 = if kurtosis.is_finite() {
        kurtosis.max(1.0)
    } else {
        3.0
    };
    let denom = (1.0 - g3 * sr_hat + ((g4 - 1.0) / 4.0) * sr_hat * sr_hat).max(1e-12);
    let z = normal_inv_cdf(confidence_level.clamp(1e-12, 1.0 - 1e-12));
    let delta = (sr_hat - benchmark_sr).abs().max(1e-12);
    let required = 1.0 + denom * (z / delta) * (z / delta);
    if !required.is_finite() {
        None
    } else {
        Some(required.ceil().max(2.0) as usize)
    }
}

pub fn inflate_effective_requirement_to_raw_bars(
    required_effective_obs: usize,
    raw_obs: usize,
    effective_obs: usize,
) -> usize {
    let raw_floor = raw_obs.max(2);
    let effective_floor = effective_obs.clamp(2, raw_floor);
    if effective_floor >= raw_floor {
        return required_effective_obs.max(2);
    }
    ((required_effective_obs as f64) * (raw_floor as f64) / (effective_floor as f64))
        .ceil()
        .max(required_effective_obs as f64)
        .max(2.0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "left={left}, right={right}, tolerance={tolerance}"
        );
    }

    #[test]
    fn psr_monotonic_in_sr() {
        let a = probabilistic_sharpe_ratio(0.2, 0.0, 100, 0.0, 3.0);
        let b = probabilistic_sharpe_ratio(0.8, 0.0, 100, 0.0, 3.0);
        assert!(b >= a);
    }

    #[test]
    fn dsr_is_probability_range() {
        let v = deflated_sharpe_ratio(1.0, 120, 0.0, 3.0, 50, 0.4, 0.2);
        assert!(v.is_finite());
        assert!((0.0..=1.0).contains(&v));
    }

    #[test]
    fn normal_inv_cdf_tail_signs_match_probability_tail() {
        assert!(normal_inv_cdf(1e-6) < 0.0);
        assert!(normal_inv_cdf(1.0 - 1e-6) > 0.0);
    }

    #[test]
    fn normal_inv_cdf_round_trips_tail_and_central_probabilities() {
        for probability in [1e-6, 1e-3, 0.2, 0.5, 0.8, 1.0 - 1e-3, 1.0 - 1e-6] {
            let quantile = normal_inv_cdf(probability);
            approx_eq(normal_cdf(quantile), probability, 5e-7);
        }
    }

    #[test]
    fn expected_max_sharpe_is_deterministic() {
        let a = expected_max_sharpe(42, 0.4, 0.2);
        let b = expected_max_sharpe(42, 0.4, 0.2);
        assert!((a - b).abs() < 1e-15);
    }

    #[test]
    fn expected_max_sharpe_is_monotonic_in_trial_count() {
        let counts = [1, 2, 5, 10, 50, 250];
        let mut last = f64::NEG_INFINITY;
        for count in counts {
            let value = expected_max_sharpe(count, 0.4, 0.2);
            assert!(value >= last, "count={count}, value={value}, last={last}");
            last = value;
        }
    }

    #[test]
    fn probabilistic_sharpe_ratio_preserves_invalid_input_fallback() {
        assert_eq!(
            probabilistic_sharpe_ratio(f64::NAN, 0.0, 100, 0.0, 3.0),
            0.5
        );
        assert_eq!(
            probabilistic_sharpe_ratio(0.5, f64::NAN, 100, 0.0, 3.0),
            0.5
        );
        assert_eq!(probabilistic_sharpe_ratio(0.5, 0.0, 1, 0.0, 3.0), 0.5);
    }

    #[test]
    fn effective_count_wrappers_match_legacy_when_counts_are_unchanged() {
        let legacy_psr = probabilistic_sharpe_ratio(0.8, 0.1, 64, 0.0, 3.0);
        let wrapped_psr =
            probabilistic_sharpe_ratio_with_effective_samples(0.8, 0.1, 64, Some(64), 0.0, 3.0);
        approx_eq(legacy_psr, wrapped_psr, 1e-12);

        let legacy_dsr = deflated_sharpe_ratio(1.0, 64, 0.0, 3.0, 12, 0.4, 0.2);
        let wrapped_dsr = deflated_sharpe_ratio_with_effective_counts(
            1.0,
            64,
            Some(64),
            0.0,
            3.0,
            12,
            Some(12),
            0.4,
            0.2,
        );
        approx_eq(legacy_dsr, wrapped_dsr, 1e-12);
    }

    #[test]
    fn effective_count_wrappers_weaken_probability_when_effective_counts_shrink() {
        let legacy_psr = probabilistic_sharpe_ratio(0.8, 0.0, 128, 0.0, 3.0);
        let overlap_psr =
            probabilistic_sharpe_ratio_with_effective_samples(0.8, 0.0, 128, Some(32), 0.0, 3.0);
        assert!(overlap_psr < legacy_psr);

        let legacy_dsr = deflated_sharpe_ratio(1.0, 128, 0.0, 3.0, 16, 0.4, 0.2);
        let overlap_dsr = deflated_sharpe_ratio_with_effective_counts(
            1.0,
            128,
            Some(32),
            0.0,
            3.0,
            16,
            Some(4),
            0.4,
            0.2,
        );
        assert!(overlap_dsr < legacy_dsr);
    }

    #[test]
    fn minimum_track_record_length_increases_with_confidence() {
        let a = minimum_track_record_length(1.0, 0.0, 0.0, 3.0, 0.80).expect("mintrl");
        let b = minimum_track_record_length(1.0, 0.0, 0.0, 3.0, 0.95).expect("mintrl");
        assert!(b >= a);
    }

    #[test]
    fn minimum_track_record_length_requires_excess_sharpe() {
        assert!(minimum_track_record_length(0.0, 0.0, 0.0, 3.0, 0.95).is_none());
        assert!(minimum_track_record_length(-0.2, 0.0, 0.0, 3.0, 0.95).is_none());
    }

    #[test]
    fn inflate_effective_requirement_to_raw_bars_is_identity_without_overlap() {
        assert_eq!(inflate_effective_requirement_to_raw_bars(12, 48, 48), 12);
    }

    #[test]
    fn inflate_effective_requirement_to_raw_bars_increases_raw_threshold_under_overlap() {
        assert_eq!(inflate_effective_requirement_to_raw_bars(12, 48, 24), 24);
        assert_eq!(inflate_effective_requirement_to_raw_bars(5, 1, 2), 5);
    }
}
