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
- GPU hybrid-ranking speedup vs CPU: 32.4595 (gate >= 3.0000)

## Backend comparison
- CPU preview median wall ms: 409 (estimated backend-only `407` ms)
- CPU hybrid-ranking median ms: 133.084
- Accelerated preview median wall ms: 497 (estimated backend-only `348` ms) (`Auto` / `gpu_compiled_tensor_hybrid`)
- Accelerated hybrid-ranking median ms: 4.100 (`gpu_compiled_tensor_hybrid`)
- Estimated backend preview wall speedup vs CPU: 1.1695
- Hybrid-ranking speedup vs CPU: 32.4595

Comparison notes:
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine
