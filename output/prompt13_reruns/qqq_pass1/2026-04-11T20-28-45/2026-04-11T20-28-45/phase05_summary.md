# Options Expression Phase 0.5 Summary

- Artifact: `C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-08T05-54-53.090196400_00-00.json`
- Split assignment: `event_chronological`
- Dataset: `qqq_phase05_2026_01_26_to_2026_01_28_calls_625_640_dte7` (`QQQ` / MinuteAggregate)
- Coverage: `1769437800000` -> `1769634840000`
- Preview backend: `gpu_compiled_tensor_hybrid`
- Auto recommendation: `gpu_compiled_tensor_hybrid`
- GPU planner: `free_memory_headroom` / outcome fits / chunk fits

## Replay/build timing
- Total replay/build: 318.039
- Parent artifact validation: 0.964
- Source resolution: 0.119
- Parent-signal process cache lookup: 0.008 (hit: `false`)
- Persisted signal-manifest cache lookup: 6.361 (hit: `true`)
- Candle replay load/resample: 0.000
- Parent replay + drilldown: 0.000
- Signal-manifest materialization: 0.000
- Options dataset manifest validation: 0.461
- Options dataset process cache lookup: 0.003 (hit: `false`)
- Persisted options dataset cache lookup: 0.191 (hit: `false`)
- Options dataset CSV hash/load: 309.861
- Request normalization: 0.001

## Phase 0.5 gates
- Candidate density: 0.2513 (gate >= 0.2500) PASS
- Invalid slot rate: 0.7487 (gate <= 0.7500) PASS
- Structural cache reuse: 1.0000 (gate >= 0.9000)
- Summary cache reuse: 1.0000 (gate >= 0.9000)
- GPU hybrid-ranking speedup vs CPU: 2.6606 (gate >= 2.7500)

## Backend comparison
- CPU preview median wall ms: 19 (estimated backend-only `19` ms)
- CPU hybrid-ranking median ms: 8.184
- Accelerated preview median wall ms: 22 (estimated backend-only `14` ms) (`Auto` / `gpu_compiled_tensor_hybrid`)
- Accelerated hybrid-ranking median ms: 3.076 (`gpu_compiled_tensor_hybrid`)
- Accelerated auto recommendation: `gpu_compiled_tensor_hybrid`
- Estimated backend preview wall speedup vs CPU: 1.3571
- Hybrid-ranking speedup vs CPU: 2.6606

Comparison notes:
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine

## Recommendations
- `entry_past_cutoff` accounts for 33.5% of invalid slots. Trim late delay buckets, lower max entry lag, or avoid using late-session parent signals for this options profile.
- Estimated backend-only preview wall time now favors the accelerated path (1.36x vs CPU), but the large-grid defaults still need denser candidate coverage before product promotion.
