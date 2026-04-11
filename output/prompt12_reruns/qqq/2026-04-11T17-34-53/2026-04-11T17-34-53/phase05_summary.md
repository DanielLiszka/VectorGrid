# Options Expression Phase 0.5 Summary

- Artifact: `C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-08T05-54-53.090196400_00-00.json`
- Split assignment: `event_chronological`
- Dataset: `qqq_phase05_2026_01_26_to_2026_01_28_calls_625_640_dte7` (`QQQ` / MinuteAggregate)
- Coverage: `1769437800000` -> `1769634840000`
- Preview backend: `gpu_compiled_tensor_hybrid`
- Auto recommendation: `gpu_compiled_tensor_hybrid`
- GPU planner: `free_memory_headroom` / outcome fits / chunk fits

## Phase 0.5 gates
- Candidate density: 0.2513 (gate >= 0.2500) PASS
- Invalid slot rate: 0.7487 (gate <= 0.7500) PASS
- Structural cache reuse: 1.0000 (gate >= 0.9000)
- Summary cache reuse: 1.0000 (gate >= 0.9000)
- GPU hybrid-ranking speedup vs CPU: 2.3411 (gate >= 2.7500)

## Backend comparison
- CPU preview median wall ms: 20 (estimated backend-only `20` ms)
- CPU hybrid-ranking median ms: 8.290
- Accelerated preview median wall ms: 26 (estimated backend-only `16` ms) (`Auto` / `gpu_compiled_tensor_hybrid`)
- Accelerated hybrid-ranking median ms: 3.541 (`gpu_compiled_tensor_hybrid`)
- Accelerated auto recommendation: `gpu_compiled_tensor_hybrid`
- Estimated backend preview wall speedup vs CPU: 1.2500
- Hybrid-ranking speedup vs CPU: 2.3411

Comparison notes:
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine

## Recommendations
- `entry_past_cutoff` accounts for 33.5% of invalid slots. Trim late delay buckets, lower max entry lag, or avoid using late-session parent signals for this options profile.
- Estimated backend-only preview wall time now favors the accelerated path (1.25x vs CPU), but the large-grid defaults still need denser candidate coverage before product promotion.
