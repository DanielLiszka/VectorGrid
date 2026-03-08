# VectorTA Optimizer

Tauri desktop app for brute-force double moving-average optimization (and future strategy expansion).

## Frontend stack

- Vue 3 + Vite frontend workspace in `web/`
- Vue-first app shell in `web/src/App.vue` mounted from minimal `web/index.html`
- `web/src/App.vue` now defers loading heavy migrated workspace panels until their tab/workspace is first activated (async component lazy-load).
- Frontend runtime is now fully Vue-owned (legacy `web/src/legacy/*` runtime path removed).
- Vue task/progress overlay in `web/src/components/TaskCenterOverlay.vue` (wired through Tauri event/invoke instrumentation)
- Vue-native run/cancel footer panel in `web/src/components/RunFooterPanel.vue`
- Vue-native controls/workspaces:
  - Data: `web/src/components/DataWorkspacePanel.vue`
  - Optimize: `web/src/components/OptimizeWorkspacePanel.vue`
  - Strategy: `web/src/components/StrategyWorkspacePanel.vue`
  - Advanced: `web/src/components/AdvancedWorkspacePanel.vue`
- Vue-native Validation results tab panel in `web/src/components/ValidationWorkspacePanel.vue` (core controls, advanced validation options, cross-ticker dataset controls/list, shortlist table, raw JSON preview, and report XLSX export flow render in Vue)
- Vue-native Overview results tab panel in `web/src/components/OverviewWorkspacePanel.vue` (empty-state, best card/stats/details, and compare controls render in Vue)
- Vue-native Top results tab panel in `web/src/components/TopWorkspacePanel.vue`
- Top/Validation/Meta Vue result tables now use row-window virtualization to avoid rendering entire large result sets at once.
- Vue-native Pareto results tab panel in `web/src/components/ParetoWorkspacePanel.vue`
- Vue-native Drilldown results tab panel in `web/src/components/DrillWorkspacePanel.vue`
- Vue-native Heatmap results tab panel in `web/src/components/HeatmapWorkspacePanel.vue`
- Vue-native Meta results tab panel in `web/src/components/MetaWorkspacePanel.vue` (strategy/filter controls, holdout/grouping tuning, batch controls/list/output, meta table, compare flow, saved-meta manager, saved-stage4 manager, and ML confidence/stage4 run tuning now render in Vue)
- Vue-native Debug results tab panel in `web/src/components/DebugWorkspacePanel.vue` (debug JSON output now renders via Vue panel state)
- Vue-native Settings workspace panel in `web/src/components/SettingsWorkspacePanel.vue`
- Vue-native History tab panel in `web/src/components/HistoryWorkspacePanel.vue`
- Vue-native Distributed tab panel in `web/src/components/DistributedWorkspacePanel.vue`
- Vue-native Execution settings pane in `web/src/components/ExecutionWorkspacePanel.vue` (includes Vue-rendered broker orders table + chart host)

## First-time setup

From repo root:

```powershell
cd web
npm install
```

## Run

From the repo root:

```powershell
cd ta_desktop_demo
npm --prefix web run build
cargo run -p ta_desktop_demo_app --release
```

Tauri config is wired to build frontend from `web/` automatically via:

- `src-tauri/tauri.conf.json`:
  - `beforeDevCommand`: `npm --prefix ./web run dev -- --host 127.0.0.1 --port 1420`
  - `beforeBuildCommand`: `npm --prefix ./web run build`

## Validation/meta adapter notes

- Validation runner orchestration is shared in `crates/app_core/src/validation_core.rs`.
- Strategy modules stay adapter-focused:
  - `crates/app_core/src/double_ma_validate.rs`
  - `crates/app_core/src/composite_validate.rs`
- To add a new strategy with validation support:
  - keep strategy-specific series/evaluation code in its strategy module,
  - implement `ValidationPostCandidate` for the strategy’s candidate result type,
  - route candidate execution/finalization through `validation_core::run_validation_candidates(...)`.
- Meta grouping supports optional strategy-agnostic context fields:
  - `feature_context`
  - `run_context`
  Legacy MA grouping fields are still supported for backward compatibility.

## Frontend dev/test commands

```powershell
cd web
npm run typecheck
npm run lint
npm run parity:outputs
npm run test
npm run build
node ../scripts/check_migration_freeze_scope.mjs
node ../scripts/export_baseline_samples.mjs
npm run baseline:ui
powershell -ExecutionPolicy Bypass -File ../scripts/capture_optimization_memory.ps1
node ../scripts/post_release_telemetry_review.mjs
```

## Performance benchmarking (CPU-only while GPU is busy)

Use the deterministic workflow gate + criterion microbench:

```powershell
# create/update CPU perf baseline
powershell -ExecutionPolicy Bypass -File scripts/run_cpu_perf_gate.ps1 -Mode baseline

# compare current code against baseline (fails on regression)
powershell -ExecutionPolicy Bypass -File scripts/run_cpu_perf_gate.ps1 -Mode candidate
```

Outputs:
- `target/workflow_regression_summary.json`
- `target/cpu_perf_summary.json`
- baseline snapshot at `scripts/baselines/cpu_perf_baseline.json`

When GPU is available again, keep this CPU gate and add Nsight runs on the same commit to isolate GPU-side gains.

## GPU backtest perf + Nsight

Use these when CUDA is available and you want to focus on grid backtest optimization performance.

```powershell
# create/update GPU perf baseline (criterion)
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode baseline

# compare current code against GPU baseline (fails on regression)
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate

# capture Nsight Compute report for the GPU kernel grid benchmark
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1

# example: profile a heavier grid case
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1 `
  -BenchFilter "double_ma/gpu_kernel/sma_sma/200k/medium"
```

Outputs:
- `target/gpu_perf_summary.json`
- baseline snapshot at `scripts/baselines/gpu_perf_baseline.json`
- Nsight report `target/nsight/gpu_kernel_small.ncu-rep` (default path)

Notes:
- GPU metrics-toggle bench defaults to `sma/sma` (backtest-focused). Set `$env:VECTORBT_BENCH_KERNEL_MA=\"alma\"` before running `double_ma_metrics` if you want the prior ALMA profile.
- For stable baseline comparisons, keep GPU load constant (close other CUDA workloads) and prefer fixed clocks/power settings when possible.

### In-app GPU demo (recommended)

After the app opens:
- Load any CSV (or use the sample CSV list).
- Confirm the `GPU:` status line shows build/runtime info (and `cuda_available=true` when applicable).
- Select **Strategy = Double MA**, then:
  - Click `Preset: GPU kernel (SMA/ALMA)` (kernel demo path), then `Run`.
  - Optionally click `CPU ↔ GPU parity check (tiny)` to sanity-check CPU vs GPU results on a tiny grid.

### GPU backends

GPU MA sweep (does not require `nvcc` - uses `vector-ta` prebuilt PTX):

```powershell
cargo run -p ta_desktop_demo_app --release --features cuda
```

GPU hybrid backend (phase 1 scaffolding, currently routes through kernel path constraints):

```powershell
cargo run -p ta_desktop_demo_app --release --features cuda,cuda-backtest-kernel
```

Hybrid rollout control:
- Enable/disable hybrid backend routing at runtime: `$env:VECTORBT_ENABLE_GPU_HYBRID_KERNEL="1"` (default `1`)

GPU kernel backend (VRAM-resident, fastest). Does **not** require `nvcc` when using the prebuilt PTX:

```powershell
cargo run -p ta_desktop_demo_app --release --features cuda,cuda-backtest-kernel
```

Notes:
- GPU kernel backend currently supports `sma`, `ema`, `wma`, and `alma` for the MA computation.
- GPU hybrid backend (`GpuHybridKernel`) is currently a phase-1 contract/wiring mode and uses the same MA support envelope as GPU kernel.
- Prebuilt PTX control (backtest kernel):
  - Force prebuilt PTX (recommended for CI/packaging): `$env:VECTORBT_USE_PREBUILT_PTX="1"`
  - Require `nvcc` (developer mode): `$env:VECTORBT_REQUIRE_NVCC="1"`
- Host-to-device upload control (kernel path):
  - Enable pinned host staging for select H2D uploads (regime series): `$env:VECTORBT_KERNEL_PINNED_H2D="1"` (default `1`)
