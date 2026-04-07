# Options Expression Phase 0.5 Summary

- Artifact: `C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-07T03-56-14.824717100_00-00.json`
- Dataset: `spy_phase05_2026_03_31_to_2026_04_02_calls_645_665` (`SPY` / MinuteAggregate)
- Coverage: `1774963800000` -> `1775160840000`
- Preview backend: `gpu_compiled_tensor_hybrid`

## Phase 0.5 gates
- Candidate density: 0.2111 (gate >= 0.2500) FAIL
- Invalid slot rate: 0.7889 (gate <= 0.7500) FAIL
- Structural cache reuse: 1.0000 (gate >= 0.9000)
- Summary cache reuse: 1.0000 (gate >= 0.9000)
- GPU scorer speedup vs CPU: 1.3637 (gate >= 3.0000)

## Backend comparison
- CPU preview median wall ms: 229
- CPU scorer-only median rank ms: 97.474
- Accelerated preview median wall ms: 301 (`Auto` / `gpu_compiled_tensor_hybrid`)
- Accelerated scorer-only median rank ms: 71.476 (`gpu_compiled_tensor_hybrid`)
- Preview wall speedup vs CPU: 0.7608
- Scorer-only speedup vs CPU: 1.3637

Comparison notes:
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine
