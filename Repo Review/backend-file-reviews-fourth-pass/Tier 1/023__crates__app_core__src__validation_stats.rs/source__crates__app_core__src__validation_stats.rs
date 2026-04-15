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

pub fn passes_probability_gates(
    psr: Option<f64>,
    dsr: Option<f64>,
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
) -> bool {
    let psr_ok = min_psr.map(|thr| psr.unwrap_or(0.0) >= thr).unwrap_or(true);
    let dsr_ok = min_dsr.map(|thr| dsr.unwrap_or(0.0) >= thr).unwrap_or(true);
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
    fn probability_gates_treat_missing_stats_as_failures_when_thresholds_are_present() {
        assert!(!passes_probability_gates(None, Some(0.8), Some(0.7), None));
        assert!(!passes_probability_gates(Some(0.8), None, None, Some(0.7)));
        assert!(passes_probability_gates(None, None, None, None));
    }
}
