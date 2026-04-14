# Options Expression Phase 0.5 Summary

- Artifact: `C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-07T03-56-14.824717100_00-00.json`
- Split assignment: `event_chronological`
- Dataset: `spy_phase05_2026_03_31_to_2026_04_02_calls_645_665_dte7` (`SPY` / MinuteAggregate)
- Coverage: `1774963800000` -> `1775160840000`
- Preview backend: `gpu_compiled_tensor_hybrid`
- Auto recommendation: `gpu_compiled_tensor_hybrid`
- GPU planner: `free_memory_headroom` / outcome fits / chunk fits

## Replay/build timing
- Total replay/build: 50.246
- Parent artifact validation: 0.785
- Source resolution: 0.066
- Parent-signal process cache lookup: 0.002 (hit: `false`)
- Persisted signal-manifest cache lookup: 0.678 (hit: `true`)
- Candle replay load/resample: 0.000
- Parent replay + drilldown: 0.000
- Signal-manifest materialization: 0.000
- Options dataset manifest validation: 0.345
- Options dataset process cache lookup: 0.003 (hit: `false`)
- Persisted options dataset cache lookup: 47.389 (hit: `true`)
- Options dataset CSV hash/load: 0.000
- Request normalization: 0.000

## Phase 0.5 gates
- Candidate density: 0.6111 (gate >= 0.2500) PASS
- Invalid slot rate: 0.3889 (gate <= 0.7500) PASS
- Structural cache reuse: n/a (gate >= 0.9000)
- Summary cache reuse: n/a (gate >= 0.9000)
- GPU hybrid-ranking speedup vs CPU: 2.6657 (gate >= 2.7500)

Notes:
- run with --warm-repeats >= 1 to measure the documented immediate-rerun cache reuse gates
- GPU speedup currently comes from the single-preview hybrid ranking timing; rerun with --compare-backends for repeated hybrid-ranking medians before treating the Phase 0.5 gate as closed

## Recommendations
- `entry_past_cutoff` accounts for 100.0% of invalid slots. Trim late delay buckets, lower max entry lag, or avoid using late-session parent signals for this options profile.
