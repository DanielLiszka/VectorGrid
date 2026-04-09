# Options Expression Phase 0.5 Summary

- Artifact: `C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-07T03-56-14.824717100_00-00.json`
- Dataset: `spy_phase05_2026_03_31_to_2026_04_02_calls_645_665_dte7` (`SPY` / MinuteAggregate)
- Coverage: `1774963800000` -> `1775160840000`
- Preview backend: `cpu_reference`
- Auto recommendation: `cpu_reference`
- GPU planner: `free_memory_headroom` / outcome fits / chunk fits

## Phase 0.5 gates
- Candidate density: 0.5247 (gate >= 0.2500) PASS
- Invalid slot rate: 0.4753 (gate <= 0.7500) PASS
- Structural cache reuse: n/a (gate >= 0.9000)
- Summary cache reuse: n/a (gate >= 0.9000)
- GPU hybrid-ranking speedup vs CPU: 3.0589 (gate >= 2.7500)

Notes:
- run with --warm-repeats >= 1 to measure the documented immediate-rerun cache reuse gates

## Backend comparison
- CPU preview median wall ms: 5 (estimated backend-only `5` ms)
- CPU hybrid-ranking median ms: 2.181
- Accelerated preview median wall ms: 6 (estimated backend-only `3` ms) (`Auto` / `gpu_compiled_tensor_hybrid`)
- Accelerated hybrid-ranking median ms: 0.713 (`gpu_compiled_tensor_hybrid`)
- Accelerated auto recommendation: `cpu_reference`
- Estimated backend preview wall speedup vs CPU: 1.6667
- Hybrid-ranking speedup vs CPU: 3.0589

Comparison notes:
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine

## Recommendations
- `no_strike_in_tolerance` accounts for 11.7% of invalid slots. Re-center the strike window closer to spot or relax the moneyness selector tolerance before widening the tuple grid.
- `entry_past_cutoff` accounts for 88.3% of invalid slots. Trim late delay buckets, lower max entry lag, or avoid using late-session parent signals for this options profile.
- GPU hybrid ranking is now materially faster on this workload (3.06x median vs CPU). The next blocker is grid quality, not scorer throughput.
- Estimated backend-only preview wall time now favors the accelerated path (1.67x vs CPU), but the large-grid defaults still need denser candidate coverage before product promotion.
