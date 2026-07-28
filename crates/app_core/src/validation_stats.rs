pub fn sanitize_probability_gate(v: Option<f64>) -> Option<f64> {
    v.and_then(|x| {
        if x.is_finite() {
            Some(x.clamp(0.0, 1.0))
        } else {
            None
        }
    })
}

pub fn compute_psr_dsr(
    sharpe: f64,
    obs: usize,
    trials: usize,
    peer_mean: f64,
    peer_std: f64,
) -> (Option<f64>, Option<f64>) {
    let psr = crate::sharpe_stats::probabilistic_sharpe_ratio(sharpe, 0.0, obs, 0.0, 3.0);
    let dsr = crate::sharpe_stats::deflated_sharpe_ratio(
        sharpe, obs, 0.0, 3.0, trials, peer_mean, peer_std,
    );

    let psr_out = if psr.is_finite() {
        Some(psr.clamp(0.0, 1.0))
    } else {
        None
    };
    let dsr_out = if dsr.is_finite() {
        Some(dsr.clamp(0.0, 1.0))
    } else {
        None
    };
    (psr_out, dsr_out)
}

pub fn compute_psr_dsr_with_effective_counts(
    sharpe: f64,
    raw_obs: usize,
    effective_obs: Option<usize>,
    raw_trials: usize,
    effective_trials: Option<usize>,
    peer_mean: f64,
    peer_std: f64,
) -> (Option<f64>, Option<f64>) {
    compute_psr_dsr_with_effective_counts_and_moments(
        sharpe,
        raw_obs,
        effective_obs,
        raw_trials,
        effective_trials,
        peer_mean,
        peer_std,
        0.0,
        3.0,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn compute_psr_dsr_with_effective_counts_and_moments(
    sharpe: f64,
    raw_obs: usize,
    effective_obs: Option<usize>,
    raw_trials: usize,
    effective_trials: Option<usize>,
    peer_mean: f64,
    peer_std: f64,
    skewness: f64,
    kurtosis: f64,
) -> (Option<f64>, Option<f64>) {
    let psr = crate::sharpe_stats::probabilistic_sharpe_ratio_with_effective_samples(
        sharpe,
        0.0,
        raw_obs,
        effective_obs,
        skewness,
        kurtosis,
    );
    let dsr = crate::sharpe_stats::deflated_sharpe_ratio_with_effective_counts(
        sharpe,
        raw_obs,
        effective_obs,
        skewness,
        kurtosis,
        raw_trials,
        effective_trials,
        peer_mean,
        peer_std,
    );

    let psr_out = if psr.is_finite() {
        Some(psr.clamp(0.0, 1.0))
    } else {
        None
    };
    let dsr_out = if dsr.is_finite() {
        Some(dsr.clamp(0.0, 1.0))
    } else {
        None
    };
    (psr_out, dsr_out)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingProbabilityGatePolicy {
    TreatMissingAsZero,
    RequirePresent,
}

pub fn passes_probability_gates(
    psr: Option<f64>,
    dsr: Option<f64>,
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
) -> bool {
    passes_probability_gates_with_policy(
        psr,
        dsr,
        min_psr,
        min_dsr,
        MissingProbabilityGatePolicy::TreatMissingAsZero,
    )
}

pub fn passes_probability_gates_with_policy(
    psr: Option<f64>,
    dsr: Option<f64>,
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
    missing_policy: MissingProbabilityGatePolicy,
) -> bool {
    let metric_passes = |value: Option<f64>, threshold: Option<f64>| match threshold {
        None => true,
        Some(thr) => match missing_policy {
            MissingProbabilityGatePolicy::TreatMissingAsZero => value.unwrap_or(0.0) >= thr,
            MissingProbabilityGatePolicy::RequirePresent => {
                value.is_some_and(|metric| metric >= thr)
            }
        },
    };
    let psr_ok = metric_passes(psr, min_psr);
    let dsr_ok = metric_passes(dsr, min_dsr);
    psr_ok && dsr_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_probability_gate_clamps_valid_values() {
        assert_eq!(sanitize_probability_gate(Some(-1.0)), Some(0.0));
        assert_eq!(sanitize_probability_gate(Some(2.0)), Some(1.0));
        assert_eq!(sanitize_probability_gate(Some(0.3)), Some(0.3));
        assert_eq!(sanitize_probability_gate(Some(f64::NAN)), None);
    }

    #[test]
    fn probability_gates_require_both_thresholds_when_set() {
        assert!(passes_probability_gates(
            Some(0.8),
            Some(0.7),
            Some(0.7),
            Some(0.6)
        ));
        assert!(!passes_probability_gates(
            Some(0.8),
            Some(0.5),
            Some(0.7),
            Some(0.6)
        ));
        assert!(!passes_probability_gates(
            Some(0.5),
            Some(0.9),
            Some(0.7),
            Some(0.6)
        ));
    }

    #[test]
    fn compute_psr_dsr_returns_bounded_optional_outputs_for_typical_inputs() {
        let (psr, dsr) = compute_psr_dsr(1.25, 128, 12, 0.4, 0.2);
        assert!(psr.is_some());
        assert!(dsr.is_some());
        assert!((0.0..=1.0).contains(&psr.expect("psr")));
        assert!((0.0..=1.0).contains(&dsr.expect("dsr")));
    }

    #[test]
    fn compute_psr_dsr_preserves_legacy_short_sample_fallback() {
        let (psr, dsr) = compute_psr_dsr(1.0, 1, 12, 0.4, 0.2);
        assert_eq!(psr, Some(0.5));
        assert!(dsr.is_some());
    }

    #[test]
    fn compute_psr_dsr_with_effective_counts_matches_legacy_when_counts_are_unchanged() {
        let legacy = compute_psr_dsr(1.25, 128, 12, 0.4, 0.2);
        let overlap =
            compute_psr_dsr_with_effective_counts(1.25, 128, Some(128), 12, Some(12), 0.4, 0.2);
        assert_eq!(legacy, overlap);
    }

    #[test]
    fn compute_psr_dsr_with_effective_counts_weakens_outputs_when_overlap_reduces_evidence() {
        let legacy = compute_psr_dsr(1.25, 128, 12, 0.4, 0.2);
        let overlap =
            compute_psr_dsr_with_effective_counts(1.25, 128, Some(32), 12, Some(4), 0.4, 0.2);
        assert!(overlap.0.expect("overlap psr") < legacy.0.expect("legacy psr"));
        assert!(overlap.1.expect("overlap dsr") < legacy.1.expect("legacy dsr"));
    }

    #[test]
    fn moment_aware_probability_path_preserves_normal_moment_compatibility() {
        let legacy =
            compute_psr_dsr_with_effective_counts(1.25, 128, Some(96), 12, Some(8), 0.4, 0.2);
        let moment_aware = compute_psr_dsr_with_effective_counts_and_moments(
            1.25,
            128,
            Some(96),
            12,
            Some(8),
            0.4,
            0.2,
            0.0,
            3.0,
        );
        assert_eq!(legacy, moment_aware);
    }

    #[test]
    fn moment_aware_probability_path_changes_tail_probability_for_nonnormal_returns() {
        let normal = compute_psr_dsr_with_effective_counts_and_moments(
            1.25,
            128,
            Some(128),
            12,
            Some(12),
            0.4,
            0.2,
            0.0,
            3.0,
        );
        let nonnormal = compute_psr_dsr_with_effective_counts_and_moments(
            1.25,
            128,
            Some(128),
            12,
            Some(12),
            0.4,
            0.2,
            -0.75,
            8.0,
        );
        assert_ne!(normal, nonnormal);
    }

    #[test]
    fn probability_gates_treat_missing_stats_as_failures_when_thresholds_are_present() {
        assert!(!passes_probability_gates(None, Some(0.8), Some(0.7), None));
        assert!(!passes_probability_gates(Some(0.8), None, None, Some(0.7)));
        assert!(passes_probability_gates(None, None, None, None));
    }

    #[test]
    fn probability_gates_allow_missing_stats_at_zero_threshold() {
        assert!(passes_probability_gates(None, Some(0.8), Some(0.0), None));
        assert!(passes_probability_gates(Some(0.8), None, None, Some(0.0)));
        assert!(passes_probability_gates(None, None, Some(0.0), Some(0.0)));
    }

    #[test]
    fn probability_gates_with_policy_preserve_default_zero_imputation_behavior() {
        let legacy = passes_probability_gates(None, Some(0.8), Some(0.0), Some(0.7));
        let explicit = passes_probability_gates_with_policy(
            None,
            Some(0.8),
            Some(0.0),
            Some(0.7),
            MissingProbabilityGatePolicy::TreatMissingAsZero,
        );
        assert_eq!(legacy, explicit);
    }

    #[test]
    fn probability_gates_with_policy_can_require_present_stats() {
        assert!(!passes_probability_gates_with_policy(
            None,
            Some(0.8),
            Some(0.0),
            None,
            MissingProbabilityGatePolicy::RequirePresent,
        ));
        assert!(passes_probability_gates_with_policy(
            Some(0.8),
            Some(0.8),
            Some(0.7),
            Some(0.7),
            MissingProbabilityGatePolicy::RequirePresent,
        ));
    }
}
