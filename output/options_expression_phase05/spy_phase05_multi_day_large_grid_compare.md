# Options Expression Phase 0.5 Summary

- Artifact: `C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-07T03-56-14.824717100_00-00.json`
- Dataset: `spy_phase05_2026_03_31_to_2026_04_02_calls_645_665` (`SPY` / MinuteAggregate)
- Coverage: `1774963800000` -> `1775160840000`
- Preview backend: `cpu_reference`

## Phase 0.5 gates
- Candidate density: 0.2111 (gate >= 0.2500) FAIL
- Invalid slot rate: 0.7889 (gate <= 0.7500) FAIL
- Structural cache reuse: 1.0000 (gate >= 0.9000)
- Summary cache reuse: 1.0000 (gate >= 0.9000)
- GPU scorer speedup vs CPU: n/a (gate >= 3.0000)

Notes:
- GPU speedup is unavailable on this run; use a CUDA build plus imported options data to measure the product-promotion gate

## Backend comparison
- CPU preview median wall ms: 232
- Accelerated preview median wall ms: 232 (`Auto` / `cpu_reference`)
- Preview wall speedup vs CPU: 1.0000

Comparison notes:
- accelerated backend comparison resolved to the same backend as cpu_reference; verify a CUDA build/runtime is available before treating the speedup as meaningful
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine
