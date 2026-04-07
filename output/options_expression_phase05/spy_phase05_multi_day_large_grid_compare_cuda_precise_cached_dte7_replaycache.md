# Options Expression Phase 0.5 Summary

- Artifact: `C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-07T03-56-14.824717100_00-00.json`
- Dataset: `spy_phase05_2026_03_31_to_2026_04_02_calls_645_665_dte7` (`SPY` / MinuteAggregate)
- Coverage: `1774963800000` -> `1775160840000`
- Preview backend: `gpu_compiled_tensor_hybrid`

## Phase 0.5 gates
- Candidate density: 0.5264 (gate >= 0.2500) PASS
- Invalid slot rate: 0.4736 (gate <= 0.7500) PASS
- Structural cache reuse: 1.0000 (gate >= 0.9000)
- Summary cache reuse: 1.0000 (gate >= 0.9000)
- GPU hybrid-ranking speedup vs CPU: 32.3181 (gate >= 3.0000)

## Backend comparison
- CPU preview median wall ms: 206 (estimated backend-only `206` ms)
- CPU hybrid-ranking median ms: 127.818
- Accelerated preview median wall ms: 269 (estimated backend-only `130` ms) (`Auto` / `gpu_compiled_tensor_hybrid`)
- Accelerated hybrid-ranking median ms: 3.955 (`gpu_compiled_tensor_hybrid`)
- Estimated backend preview wall speedup vs CPU: 1.5846
- Hybrid-ranking speedup vs CPU: 32.3181

Comparison notes:
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine

## Recommendations
- `no_strike_in_tolerance` accounts for 11.7% of invalid slots. Re-center the strike window closer to spot or relax the moneyness selector tolerance before widening the tuple grid.
- `entry_past_cutoff` accounts for 88.3% of invalid slots. Trim late delay buckets, lower max entry lag, or avoid using late-session parent signals for this options profile.
- GPU hybrid ranking is now materially faster on this workload (32.32x median vs CPU). The next blocker is grid quality, not scorer throughput.
- Estimated backend-only preview wall time now favors the accelerated path (1.58x vs CPU), but the large-grid defaults still need denser candidate coverage before product promotion.
