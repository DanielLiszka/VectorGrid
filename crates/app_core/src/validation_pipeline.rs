use ta_strategies::double_ma::Metrics;

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
        let (xi, yi) = out[i]
            .walk_forward_sharpe_maxdd()
            .unwrap_or_else(|| {
                let m = out[i].in_sample_metrics();
                (m.sharpe, m.max_dd)
            });

        let mut dominated = false;
        for j in 0..out.len() {
            if i == j {
                continue;
            }
            let (xj, yj) = out[j]
                .walk_forward_sharpe_maxdd()
                .unwrap_or_else(|| {
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
