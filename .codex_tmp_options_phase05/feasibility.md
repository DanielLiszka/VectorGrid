# Options Expression Phase 0.5 Summary

- Artifact: `.codex_tmp_options_phase05/appdata/runs/vectorgrid_run_2026-04-03T11-59-07.618919500_00-00.json`
- Dataset: `spy_phase05_demo` (`SPY` / MinuteAggregate)
- Coverage: `1699972200000` -> `1699986540000`
- Preview backend: `cpu_reference`

## Phase 0.5 gates
- Candidate density: 0.0000 (gate >= 0.2500) FAIL
- Invalid slot rate: 1.0000 (gate <= 0.7500) FAIL
- Structural cache reuse: 1.0000 (gate >= 0.9000)
- Summary cache reuse: 1.0000 (gate >= 0.9000)
- GPU scorer speedup vs CPU: n/a (gate >= 3.0000)

Notes:
- GPU speedup is unavailable on this run; use a CUDA build plus imported options data to measure the product-promotion gate

## Backend comparison
- CPU preview median wall ms: 10418
- Accelerated preview median wall ms: 10370 (`Auto` / `cpu_reference`)
- Preview wall speedup vs CPU: 1.0046
- CPU full-run median wall ms: 8058
- Accelerated full-run median wall ms: 6667 (`cpu_reference`)
- Full-run wall speedup vs CPU: 1.2086

Comparison notes:
- accelerated backend comparison resolved to the same backend as cpu_reference; verify a CUDA build/runtime is available before treating the speedup as meaningful
- accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine

## Full run
- Artifact: `.codex_tmp_options_phase05/appdata\runs\vectorgrid_options_expression_2026-04-06T01-38-59.853310200_00-00.json`
- Backend used: `cpu_reference`
- Tuple count: `25`
- Top tuple robust score: `0.000000`
- Top tuple selection EV / premium: `0.000000`
