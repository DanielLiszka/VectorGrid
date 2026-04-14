# Options Expression Phase 0.5 Summary

- Artifact: `C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-07T03-56-14.824717100_00-00.json`
- Split assignment: `event_chronological`
- Dataset: `spy_phase05_2026_03_31_to_2026_04_02_calls_645_665_dte7` (`SPY` / MinuteAggregate)
- Coverage: `1774963800000` -> `1775160840000`
- Preview backend: `gpu_compiled_tensor_hybrid`
- Auto recommendation: `gpu_compiled_tensor_hybrid`
- GPU planner: `free_memory_headroom` / outcome fits / chunk fits

## Replay/build timing
- Total replay/build: 48.802
- Parent artifact validation: 0.900
- Source resolution: 0.088
- Parent-signal process cache lookup: 0.010 (hit: `false`)
- Persisted signal-manifest cache lookup: 0.767 (hit: `true`)
- Candle replay load/resample: 0.000
- Parent replay + drilldown: 0.000
- Signal-manifest materialization: 0.000
- Options dataset manifest validation: 0.347
- Options dataset process cache lookup: 0.002 (hit: `false`)
- Persisted options dataset cache lookup: 46.266 (hit: `true`)
- Options dataset CSV hash/load: 0.000
- Request normalization: 0.000

## Phase 0.5 gates
- Candidate density: 0.6111 (gate >= 0.2500) PASS
- Invalid slot rate: 0.3889 (gate <= 0.7500) PASS
- Structural cache reuse: 1.0000 (gate >= 0.9000)
- Summary cache reuse: 1.0000 (gate >= 0.9000)
- GPU hybrid-ranking speedup vs CPU: 7.4659 (gate >= 2.7500)

## Backend comparison
- CPU preview median wall ms: 8 (estimated backend-only `8` ms)
- CPU hybrid-ranking median ms: 4.054
- Accelerated preview median wall ms: 9 (estimated backend-only `4` ms) (`Auto` / `gpu_compiled_tensor_hybrid`)
- Accelerated hybrid-ranking median ms: 0.543 (`gpu_compiled_tensor_hybrid`)
- Accelerated auto recommendation: `gpu_compiled_tensor_hybrid`
- Estimated backend preview wall speedup vs CPU: 2.0000
- Hybrid-ranking speedup vs CPU: 7.4659

Comparison notes:
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine

## Recommendations
- `entry_past_cutoff` accounts for 100.0% of invalid slots. Trim late delay buckets, lower max entry lag, or avoid using late-session parent signals for this options profile.
- GPU hybrid ranking is now materially faster on this workload (7.47x median vs CPU). The next blocker is grid quality, not scorer throughput.
- Estimated backend-only preview wall time now favors the accelerated path (2.00x vs CPU), but the large-grid defaults still need denser candidate coverage before product promotion.
