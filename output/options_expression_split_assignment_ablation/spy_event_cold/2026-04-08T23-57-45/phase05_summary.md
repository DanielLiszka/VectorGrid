# Options Expression Phase 0.5 Summary

- Artifact: `C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-07T03-56-14.824717100_00-00.json`
- Dataset: `spy_phase05_2026_03_31_to_2026_04_02_calls_645_665_dte7` (`SPY` / MinuteAggregate)
- Coverage: `1774963800000` -> `1775160840000`
- Preview backend: `cpu_reference`
- Auto recommendation: `gpu_compiled_tensor_hybrid`
- GPU planner: `free_memory_headroom` / outcome fits / chunk fits

## Phase 0.5 gates
- Candidate density: 0.5247 (gate >= 0.2500) PASS
- Invalid slot rate: 0.4753 (gate <= 0.7500) PASS
- Structural cache reuse: n/a (gate >= 0.9000)
- Summary cache reuse: n/a (gate >= 0.9000)
- GPU hybrid-ranking speedup vs CPU: 0.9936 (gate >= 2.7500)

Notes:
- run with --warm-repeats >= 1 to measure the documented immediate-rerun cache reuse gates

## Backend comparison
- CPU preview median wall ms: 4 (estimated backend-only `4` ms)
- CPU hybrid-ranking median ms: 1.702
- Accelerated preview median wall ms: 23 (estimated backend-only `23` ms) (`Auto` / `cpu_reference`)
- Accelerated hybrid-ranking median ms: 1.713 (`cpu_reference`)
- Accelerated auto recommendation: `gpu_compiled_tensor_hybrid`
- Estimated backend preview wall speedup vs CPU: 0.1739
- Hybrid-ranking speedup vs CPU: 0.9936

Comparison notes:
- accelerated backend comparison resolved to the same backend as cpu_reference; verify a CUDA build/runtime is available before treating the speedup as meaningful
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine
- repeated hybrid-ranking medians still favor cpu_reference for this workload; do not treat the accelerated route as the recommended backend yet

## Recommendations
- `no_strike_in_tolerance` accounts for 11.7% of invalid slots. Re-center the strike window closer to spot or relax the moneyness selector tolerance before widening the tuple grid.
- `entry_past_cutoff` accounts for 88.3% of invalid slots. Trim late delay buckets, lower max entry lag, or avoid using late-session parent signals for this options profile.
