# VectorGrid

GPU-accelerated Tauri desktop app for exact-grid trading strategy optimization and validation.

The desktop product and the benchmarked execution engine are both referred to as **VectorGrid** in this repository. `VectorBT` is mentioned only for explicit external benchmark comparisons.

## Benchmark

![VectorGrid vs VectorBT 250k benchmark chart](docs/assets/benchmark-double-ma-250k.svg)

**Current headline benchmark**

- Scenario: `Double MA`, `ALMA/ALMA`, exact grid
- Workload: `250,000` backtests over `200,000` bars
- Hardware: `AMD Ryzen 9 9950X`, `NVIDIA GeForce RTX 4090`
- VectorGrid GPU kernel: `233.660 ms`
- VectorGrid CPU: `15202.400 ms`
- VectorBT CPU workflow: `445240.337 ms`

That is about:

- `1905x` faster than the measured VectorBT CPU workflow
- `65x` faster than the app's own CPU path on the same scenario

**Secondary benchmark snapshot**

| Scenario | VectorGrid GPU kernel | VectorGrid CPU | VectorBT CPU |
| --- | --- | --- | --- |
| `200k bars / 58,300 pairs` | `101.890 ms` | `3486.130 ms` | `92377.801 ms` |
| `200k bars / 250,000 pairs` | `233.660 ms` | `15202.400 ms` | `445240.337 ms` |

## What This Measures

The benchmark shown above is the **warm app-path** benchmark, not the tighter Criterion microbenchmark.

- Dataset is loaded once.
- Warmup runs happen before measured trials.
- GPU measured trials reuse resident market-series buffers.
- The `250,000`-pair scenario uses `top_k=10` and `heatmap_bins=0`.
- VectorBT is measured as a CPU Python workflow on `vectorbt 0.28.4`, with local ALMA generation and app-equivalent next-bar Sharpe scoring to keep the objective semantics aligned.

This repo also has a tighter engine benchmark boundary for the classic `58,300`-pair ALMA case:

- Criterion anchor: `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`
- Historical anchor: about `89 ms`

Use the warm app-path numbers for product-facing comparisons. Use the Criterion benchmark for lower-level engine tracking.

## What The App Does

- Exact-grid optimization for trading strategies, with a mature GPU path for the `Double MA` family.
- GPU-backed validation coverage for the active supported strategy families, including walk-forward and related validation flows.
- CPU fallback support throughout the product.
- Warm in-memory and VRAM-resident execution paths designed for repeated optimization and validation runs on the same dataset.
- Desktop UI for data loading, optimization, shortlist review, heatmaps, validation, history, and export/report workflows.

## Current GPU Coverage

Broadly, the repo is in a good backend state for the current target set.

- Mature `DoubleMa` GPU optimization and validation path.
- `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid` route through the `DoubleMa` product path.
- Composite-side GPU support exists for `ASR`, `ROCB`, `VEMR`, `MKCS`, `KQSR`, `CARM`, `KVDF`, and `CDMC`.
- `VCC` remains parity-gated.
- `CompositeMo` remains CPU-bound.

For the current detailed tracker set, see:

- [OPEN_IMPLEMENTATION_COMPLETION_TRACKER_2026-03-12.md](research/OPEN_IMPLEMENTATION_COMPLETION_TRACKER_2026-03-12.md)
- [GPU_VALIDATION_MIGRATION_PLAN_2026-03-10.md](research/GPU_VALIDATION_MIGRATION_PLAN_2026-03-10.md)
- [VALIDATION_HARDENING_PLAN_2026-03-10.md](research/VALIDATION_HARDENING_PLAN_2026-03-10.md)
- [STRATEGY_INDICATOR_GPU_COVERAGE_INVENTORY_2026-03-10.md](research/STRATEGY_INDICATOR_GPU_COVERAGE_INVENTORY_2026-03-10.md)

## Quick Start

Install frontend dependencies:

```powershell
cd web
npm install
```

Run the app from the repo root:

```powershell
npm --prefix web run build
cargo run -p ta_desktop_demo_app --release --features cuda,cuda-backtest-kernel
```

CPU-only app run:

```powershell
cargo run -p ta_desktop_demo_app --release
```

## Reproduce The Benchmark

Run the `250,000`-pair warm faceoff scenario:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1 `
  -ScenarioPath scripts/vectorbt_double_ma_faceoff_scenario_250k.json `
  -OutDir target/vectorbt_vs_vectorgrid_faceoff_250k
```

Scenario file:

- [scripts/vectorbt_double_ma_faceoff_scenario_250k.json](scripts/vectorbt_double_ma_faceoff_scenario_250k.json)

The app-only reruns used for the current README numbers were written to:

- `target/vectorgrid_app_faceoff_only_250k_20260313/`
- `target/vectorbt_only_faceoff_250k_20260313/`

## Performance Gates And Profiling

CPU perf gate:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_cpu_perf_gate.ps1 -Mode candidate
```

GPU perf gate:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

Nsight Compute capture:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1
```

## Repo Layout

- `src-tauri/`: Tauri app crate, commands, desktop integration
- `crates/app_core/`: optimization, validation, CUDA/runtime plumbing
- `crates/strategies/`: strategy configuration and shared strategy logic
- `crates/optimizer/`: optimizer support crate
- `web/`: Vue 3 + Vite frontend
- `research/`: trackers, migration plans, benchmark notes, and implementation docs

## Notes

- Keep CPU-only builds working; CUDA is optional at build time.
- For CUDA backtest-kernel work, use the repo perf gates and Nsight before drawing conclusions from single ad-hoc timings.
- The current benchmark chart is intentionally static and explicit. It is meant to be readable and honest in a GitHub README without relying on video playback.
