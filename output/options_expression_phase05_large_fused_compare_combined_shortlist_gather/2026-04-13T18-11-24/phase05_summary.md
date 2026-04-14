# Options Expression Phase 0.5 Summary

- Artifact: `C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-07T03-56-14.824717100_00-00.json`
- Split assignment: `event_chronological`
- Dataset: `spy_phase05_2026_03_31_to_2026_04_02_calls_645_665_dte7` (`SPY` / MinuteAggregate)
- Coverage: `1774963800000` -> `1775160840000`
- Preview backend: `gpu_compiled_tensor_hybrid`
- Auto recommendation: `gpu_compiled_tensor_hybrid`
- GPU planner: `free_memory_headroom` / outcome fits / chunk fits

## Replay/build timing
- Total replay/build: 49.950
- Parent artifact validation: 0.812
- Source resolution: 0.060
- Parent-signal process cache lookup: 0.006 (hit: `false`)
- Persisted signal-manifest cache lookup: 0.812 (hit: `true`)
- Candle replay load/resample: 0.000
- Parent replay + drilldown: 0.000
- Signal-manifest materialization: 0.000
- Options dataset manifest validation: 0.291
- Options dataset process cache lookup: 0.002 (hit: `false`)
- Persisted options dataset cache lookup: 47.572 (hit: `true`)
- Options dataset CSV hash/load: 0.000
- Request normalization: 0.000

## Phase 0.5 gates
- Candidate density: 0.6111 (gate >= 0.2500) PASS
- Invalid slot rate: 0.3889 (gate <= 0.7500) PASS
- Structural cache reuse: 1.0000 (gate >= 0.9000)
- Summary cache reuse: 1.0000 (gate >= 0.9000)
- GPU hybrid-ranking speedup vs CPU: 5.1227 (gate >= 2.7500)

## Backend comparison
- CPU preview median wall ms: 16 (estimated backend-only `16` ms)
- CPU hybrid-ranking median ms: 8.186
- Accelerated preview median wall ms: 18 (estimated backend-only `9` ms) (`Auto` / `gpu_compiled_tensor_hybrid`)
- Accelerated hybrid-ranking median ms: 1.598 (`gpu_compiled_tensor_hybrid`)
- Accelerated auto recommendation: `gpu_compiled_tensor_hybrid`
- Estimated backend preview wall speedup vs CPU: 1.7778
- Hybrid-ranking speedup vs CPU: 5.1227

Comparison notes:
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine

## Recommendations
- `entry_past_cutoff` accounts for 100.0% of invalid slots. Trim late delay buckets, lower max entry lag, or avoid using late-session parent signals for this options profile.
- GPU hybrid ranking is now materially faster on this workload (5.12x median vs CPU). The next blocker is grid quality, not scorer throughput.
- Estimated backend-only preview wall time now favors the accelerated path (1.78x vs CPU), but the large-grid defaults still need denser candidate coverage before product promotion.
