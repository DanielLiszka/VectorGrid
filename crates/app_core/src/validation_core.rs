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
    spa_requested: bool,
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
    let tmp = match crate::validation_pipeline::select_validation_collection_mode(
        hard_gates,
        cscv_cfg,
        spa_requested,
    ) {
        crate::validation_pipeline::ValidationCollectionMode::SequentialFastPath => {
            crate::validation_pipeline::sequential_collect_validation_candidates(
                candidates,
                seed,
                cancel,
                progress,
                validate_phase,
                |idx, candidate, rng| process_one(idx, candidate, rng),
            )?
        }
        crate::validation_pipeline::ValidationCollectionMode::ParallelFullSet => {
            crate::validation_pipeline::parallel_collect_validation_candidates(
                candidates,
                seed,
                cancel,
                progress,
                validate_phase,
                process_one,
            )?
        }
    };

    let mut out: Vec<C> = Vec::with_capacity(tmp.len());
    let mut sample_obs: Vec<usize> = Vec::with_capacity(tmp.len());
    let mut common_start_t = 0usize;
    for (cand, st, obs) in tmp {
        out.push(cand);
        common_start_t = common_start_t.max(st);
        sample_obs.push(obs.max(2));
    }

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
        build_run_verdict, ValidationCandidateVerdict, ValidationHardGateConfig,
        ValidationPostCandidate,
    };
    use crate::validation_shared::WalkForwardStats;
    use rand::Rng;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::{AtomicUsize, Ordering};
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

        fn walk_forward_stats(&self) -> Option<&WalkForwardStats> {
            None
        }

        fn cscv_oos_metric_p50(&self) -> Option<f64> {
            self.cscv_pbo.as_ref().and_then(|v| v.oos_metric_p50)
        }

        fn cscv_oos_rank_frac_p50(&self) -> Option<f64> {
            self.cscv_pbo.as_ref().and_then(|v| v.oos_rank_frac_p50)
        }

        fn cscv_selected_frac(&self) -> Option<f64> {
            self.cscv_pbo.as_ref().map(|v| v.is_selected_frac)
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
            false,
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
            false,
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
            false,
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

    #[test]
    fn shared_validation_core_uses_max_candidate_start_for_cscv_alignment() {
        let candidates = vec![1usize, 2, 3];
        let cancel = AtomicBool::new(false);
        let observed_starts = Mutex::new(Vec::new());

        let (_out, cscv) = run_validation_candidates(
            &candidates,
            11,
            &cancel,
            None,
            "unit_validate",
            "unit_cscv",
            &ValidationHardGateConfig::default(),
            Some(&crate::cscv_pbo::CscvPboConfig {
                slices: 4,
                max_combinations: 16,
                threshold: 0.0,
            }),
            false,
            256,
            |_, p, _| {
                let start_t = match *p {
                    1 => 5,
                    2 => 17,
                    _ => 9,
                };
                Ok((
                    DummyCandidate {
                        id: *p,
                        in_sample: Metrics {
                            sharpe: *p as f64,
                            max_dd: 0.1,
                            ..Metrics::default()
                        },
                        psr: None,
                        dsr: None,
                        track_record_bars: 0,
                        min_track_record_length_bars: None,
                        robust_score: *p as f64,
                        robust_pareto: false,
                        cscv_pbo: None,
                        validation_verdict: ValidationCandidateVerdict::default(),
                    },
                    start_t,
                    64,
                ))
            },
            |cand, cscv_start_t, _slice_bars, slices| {
                observed_starts
                    .lock()
                    .expect("start observations lock")
                    .push((cand.id, cscv_start_t));
                Ok((0..slices)
                    .map(|s| {
                        let v = (cand.id as f64) * (s as f64 + 1.0) * 0.001;
                        SliceReturnStats {
                            sum: v,
                            sumsq: v * v,
                        }
                    })
                    .collect())
            },
        )
        .expect("validation core run");

        assert!(cscv.is_some(), "expected CSCV output");
        let mut observed = observed_starts
            .lock()
            .expect("start observations lock")
            .clone();
        observed.sort_by_key(|(id, _)| *id);
        assert_eq!(observed, vec![(1, 17), (2, 17), (3, 17)]);
    }

    #[test]
    fn shared_validation_core_skips_slice_builder_when_cscv_is_disabled() {
        let candidates = vec![1usize, 2, 3];
        let cancel = AtomicBool::new(false);
        let slice_builder_calls = AtomicUsize::new(0);

        let (_out, cscv_pbo) = run_validation_candidates(
            &candidates,
            7,
            &cancel,
            None,
            "unit_validate",
            "unit_cscv",
            &ValidationHardGateConfig::default(),
            None,
            false,
            128,
            |_, p, _| {
                Ok((
                    DummyCandidate {
                        id: *p,
                        in_sample: Metrics {
                            sharpe: *p as f64,
                            max_dd: 0.1,
                            ..Metrics::default()
                        },
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
            |_cand, _cscv_start_t, _slice_bars, _slices| {
                slice_builder_calls.fetch_add(1, Ordering::Relaxed);
                Ok(Vec::<SliceReturnStats>::new())
            },
        )
        .expect("validation core run");

        assert!(cscv_pbo.is_none());
        assert_eq!(slice_builder_calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn shared_validation_core_replays_deterministically_with_same_seed() {
        fn run_once(seed: u64) -> Vec<(usize, u64)> {
            let candidates = vec![1usize, 2, 3, 4];
            let cancel = AtomicBool::new(false);
            let (out, cscv_pbo) = run_validation_candidates(
                &candidates,
                seed,
                &cancel,
                None,
                "unit_validate",
                "unit_cscv",
                &ValidationHardGateConfig::default(),
                None,
                false,
                128,
                |_, p, rng| {
                    let score = rng.gen_range(0.0..1.0);
                    Ok((
                        DummyCandidate {
                            id: *p,
                            in_sample: Metrics {
                                sharpe: score,
                                max_dd: 0.1,
                                ..Metrics::default()
                            },
                            psr: None,
                            dsr: None,
                            track_record_bars: 0,
                            min_track_record_length_bars: None,
                            robust_score: score,
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
            out.into_iter()
                .map(|cand| (cand.id, cand.robust_score.to_bits()))
                .collect()
        }

        assert_eq!(run_once(29), run_once(29));
    }

    #[test]
    fn shared_validation_core_end_to_end_preserves_rank_one_metadata_when_later_candidate_is_first_approved(
    ) {
        let candidates = vec![1usize, 2];
        let cancel = AtomicBool::new(false);
        let gates = ValidationHardGateConfig {
            min_psr: Some(0.90),
            ..ValidationHardGateConfig::default()
        };

        let (out, cscv_pbo) = run_validation_candidates(
            &candidates,
            7,
            &cancel,
            None,
            "unit_validate",
            "unit_cscv",
            &gates,
            None,
            false,
            256,
            |_, p, _| {
                let (sharpe, sample_obs) = if *p == 1 { (1.1, 256) } else { (1.4, 2) };
                Ok((
                    DummyCandidate {
                        id: *p,
                        in_sample: Metrics {
                            sharpe,
                            ..Metrics::default()
                        },
                        psr: None,
                        dsr: None,
                        track_record_bars: 0,
                        min_track_record_length_bars: None,
                        robust_score: sharpe,
                        robust_pareto: false,
                        cscv_pbo: None,
                        validation_verdict: ValidationCandidateVerdict::default(),
                    },
                    0,
                    sample_obs,
                ))
            },
            |_cand, _cscv_start_t, _slice_bars, _slices| Ok(Vec::<SliceReturnStats>::new()),
        )
        .expect("validation core run");

        assert!(cscv_pbo.is_none());

        let verdict = build_run_verdict(&out, out.len(), &gates, None, None);
        let top_candidate = out.first().expect("sorted validation output");

        assert!(verdict.approved);
        assert_eq!(verdict.candidates_survived_hard_gates, 1);
        assert_eq!(verdict.approved_candidate_count, 1);
        assert_eq!(verdict.first_approved_rank, Some(2));
        assert_eq!(verdict.top_candidate_psr, top_candidate.psr());
        assert_eq!(verdict.top_candidate_dsr, top_candidate.dsr());
    }
}
