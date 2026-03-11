use crate::cscv_pbo::{CscvPboConfig, CscvPboResult, SliceReturnStats};
use crate::progress::ProgressSink;
use crate::validation_pipeline::{ValidationHardGateConfig, ValidationPostCandidate};
use rand::rngs::StdRng;
use std::sync::atomic::AtomicBool;

pub fn run_validation_candidates<P, C, FProcess, FSliceStats>(
    candidates: &[P],
    seed: u64,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    validate_phase: &'static str,
    cscv_prep_phase: &'static str,
    hard_gates: &ValidationHardGateConfig,
    cscv_cfg: Option<&CscvPboConfig>,
    total_bars: usize,
    process_one: FProcess,
    build_slice_stats_for_candidate: FSliceStats,
) -> Result<(Vec<C>, Option<CscvPboResult>), String>
where
    P: Sync,
    C: Send + ValidationPostCandidate,
    FProcess: Fn(usize, &P, &mut StdRng) -> Result<(C, usize, usize), String> + Sync,
    FSliceStats: FnMut(&C, usize, usize, usize) -> Result<Vec<SliceReturnStats>, String>,
{
    let tmp = crate::validation_pipeline::parallel_collect_validation_candidates(
        candidates,
        seed,
        cancel,
        progress,
        validate_phase,
        process_one,
    )?;

    let mut out: Vec<C> = Vec::with_capacity(tmp.len());
    let mut start_ts: Vec<usize> = Vec::with_capacity(tmp.len());
    let mut sample_obs: Vec<usize> = Vec::with_capacity(tmp.len());
    for (cand, st, obs) in tmp {
        out.push(cand);
        start_ts.push(st);
        sample_obs.push(obs.max(2));
    }

    let common_start_t = start_ts.into_iter().max().unwrap_or(0);
    let cscv_pbo = crate::validation_pipeline::finalize_with_probability_and_optional_cscv(
        &mut out,
        &sample_obs,
        hard_gates,
        cscv_cfg,
        total_bars,
        common_start_t,
        seed,
        cancel,
        progress,
        cscv_prep_phase,
        build_slice_stats_for_candidate,
    )?;

    Ok((out, cscv_pbo))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cscv_pbo::CscvCandidatePboStats;
    use crate::progress::{DoubleMaProgressPayload, ProgressSink};
    use crate::validation_pipeline::{
        ValidationCandidateVerdict, ValidationHardGateConfig, ValidationPostCandidate,
    };
    use std::collections::HashMap;
    use std::sync::atomic::AtomicBool;
    use std::sync::Mutex;
    use ta_strategies::double_ma::Metrics;

    #[derive(Clone, Debug)]
    struct DummyCandidate {
        id: usize,
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

        fn walk_forward_sharpe_maxdd(&self) -> Option<(f64, f64)> {
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

    #[derive(Default)]
    struct RecordingProgressSink {
        events: Mutex<Vec<DoubleMaProgressPayload>>,
    }

    impl RecordingProgressSink {
        fn take(&self) -> Vec<DoubleMaProgressPayload> {
            let mut g = self.events.lock().expect("progress lock");
            std::mem::take(&mut *g)
        }
    }

    impl ProgressSink for RecordingProgressSink {
        fn emit_double_ma_progress(&self, payload: DoubleMaProgressPayload) {
            self.events.lock().expect("progress lock").push(payload);
        }
    }

    #[test]
    fn shared_validation_core_collects_and_sorts_candidates_without_cscv() {
        let candidates = vec![1usize, 2, 3];
        let cancel = AtomicBool::new(false);
        let (out, cscv_pbo) = run_validation_candidates(
            &candidates,
            7,
            &cancel,
            None,
            "unit_validate",
            "unit_cscv",
            &ValidationHardGateConfig::default(),
            None,
            128,
            |_, p, _| {
                let m = Metrics {
                    sharpe: *p as f64,
                    max_dd: 0.1,
                    ..Metrics::default()
                };
                Ok((
                    DummyCandidate {
                        id: *p,
                        in_sample: m,
                        psr: None,
                        dsr: None,
                        track_record_bars: 0,
                        min_track_record_length_bars: None,
                        robust_score: *p as f64,
                        robust_pareto: false,
                        cscv_pbo: None,
                        validation_verdict: ValidationCandidateVerdict::default(),
                    },
                    0,
                    64,
                ))
            },
            |_cand, _cscv_start_t, _slice_bars, _slices| Ok(Vec::<SliceReturnStats>::new()),
        )
        .expect("validation core run");

        assert!(cscv_pbo.is_none());
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].id, 3);
        assert_eq!(out[1].id, 2);
        assert_eq!(out[2].id, 1);
    }

    #[test]
    fn shared_validation_core_respects_cancel_token() {
        let candidates = vec![1usize, 2, 3];
        let cancel = AtomicBool::new(true);
        let err = run_validation_candidates(
            &candidates,
            7,
            &cancel,
            None,
            "unit_validate",
            "unit_cscv",
            &ValidationHardGateConfig::default(),
            None,
            128,
            |_, p, _| {
                let m = Metrics {
                    sharpe: *p as f64,
                    max_dd: 0.1,
                    ..Metrics::default()
                };
                Ok((
                    DummyCandidate {
                        id: *p,
                        in_sample: m,
                        psr: None,
                        dsr: None,
                        track_record_bars: 0,
                        min_track_record_length_bars: None,
                        robust_score: *p as f64,
                        robust_pareto: false,
                        cscv_pbo: None,
                        validation_verdict: ValidationCandidateVerdict::default(),
                    },
                    0,
                    64,
                ))
            },
            |_cand, _cscv_start_t, _slice_bars, _slices| Ok(Vec::<SliceReturnStats>::new()),
        )
        .expect_err("expected cancellation error");
        assert_eq!(err, "cancelled");
    }

    #[test]
    fn shared_validation_core_progress_phase_names_and_monotonic_counts() {
        let candidates = vec![1usize, 2, 3];
        let cancel = AtomicBool::new(false);
        let progress = RecordingProgressSink::default();

        let (_out, cscv) = run_validation_candidates(
            &candidates,
            7,
            &cancel,
            Some(&progress),
            "unit_validate",
            "unit_cscv",
            &ValidationHardGateConfig::default(),
            Some(&crate::cscv_pbo::CscvPboConfig {
                slices: 4,
                max_combinations: 16,
                threshold: 0.0,
            }),
            256,
            |_, p, _| {
                let m = Metrics {
                    sharpe: *p as f64,
                    max_dd: 0.1 * (*p as f64),
                    ..Metrics::default()
                };
                Ok((
                    DummyCandidate {
                        id: *p,
                        in_sample: m,
                        psr: None,
                        dsr: None,
                        track_record_bars: 0,
                        min_track_record_length_bars: None,
                        robust_score: *p as f64,
                        robust_pareto: false,
                        cscv_pbo: None,
                        validation_verdict: ValidationCandidateVerdict::default(),
                    },
                    0,
                    64,
                ))
            },
            |cand, _cscv_start_t, _slice_bars, slices| {
                let mut out = Vec::with_capacity(slices);
                for s in 0..slices {
                    let v = (cand.id as f64) * (s as f64 + 1.0) * 0.001;
                    out.push(SliceReturnStats {
                        sum: v,
                        sumsq: v * v,
                    });
                }
                Ok(out)
            },
        )
        .expect("validation core run");

        assert!(
            cscv.is_some(),
            "expected CSCV result for progress phase coverage"
        );

        let events = progress.take();
        assert!(!events.is_empty(), "expected progress events");

        let mut last_by_phase: HashMap<&'static str, usize> = HashMap::new();
        let mut saw_validate = false;
        let mut saw_cscv_prep = false;
        let mut saw_cscv_kernel = false;

        for ev in events {
            match ev.phase {
                "unit_validate" => {
                    saw_validate = true;
                    let last = *last_by_phase.get(ev.phase).unwrap_or(&0usize);
                    assert!(
                        ev.processed_pairs >= last,
                        "validate progress must be monotonic"
                    );
                    last_by_phase.insert(ev.phase, ev.processed_pairs);
                }
                "unit_cscv" => {
                    saw_cscv_prep = true;
                    let last = *last_by_phase.get(ev.phase).unwrap_or(&0usize);
                    assert!(
                        ev.processed_pairs >= last,
                        "cscv prep progress must be monotonic"
                    );
                    last_by_phase.insert(ev.phase, ev.processed_pairs);
                }
                "composite_validate_cscv" => {
                    saw_cscv_kernel = true;
                    let last = *last_by_phase.get(ev.phase).unwrap_or(&0usize);
                    assert!(
                        ev.processed_pairs >= last,
                        "cscv kernel progress must be monotonic"
                    );
                    last_by_phase.insert(ev.phase, ev.processed_pairs);
                }
                other => panic!("unexpected phase emitted: {other}"),
            }
        }

        assert!(saw_validate, "missing validate phase progress");
        assert!(saw_cscv_prep, "missing cscv prep phase progress");
        assert!(saw_cscv_kernel, "missing cscv kernel phase progress");
    }
}
