# Full Benchmark Run Summary (2026-02-26)

## Scope
- Crate: `ta_app_core`
- Features: `cuda,cuda-backtest-kernel`
- Bench targets discovered in `crates/app_core/Cargo.toml`: 7

## Command Pattern Used
```powershell
cargo bench -p ta_app_core --bench <bench_name> --features cuda,cuda-backtest-kernel -- --noplot
```

## Bench Target Status
- `double_ma_opt`: completed
- `double_ma_metrics`: completed
- `composite_mo_opt`: completed
- `composite_cpu_opt`: completed
- `composite_validation_workflow`: completed
- `meta_select_workflow`: completed
- `e2e_workflow_ml`: failed

## Failure Detail (`e2e_workflow_ml`)
- Panic message:
  - `ml training failed for all label thresholds: No matching meta artifacts found for this scope. No labeled rows found (need meta artifacts with holdout aggregates). Insufficient labeled rows (need at least 25).`
- Also failed when re-run with `VTA_E2E_BENCH_LENS=50000`.

## Artifacts
- Full console log: `target/bench_full_run.log`
- Consolidated benchmark results CSV (`Criterion new estimates` generated during this run window):
  - `benchmark_results_2026-02-26.csv`
  - Rows exported: 128
