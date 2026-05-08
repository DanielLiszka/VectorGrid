# VectorGrid

Status/provenance: copied into `GPT5.5-review/` as review-pack context. The root `README.md` remains the current public-facing README authority.

GPU-accelerated Rust backend and CLI for exact-grid trading strategy optimization and validation.

The original desktop product and the benchmarked execution engine are both referred to as **VectorGrid** in this repository. `VectorBT` is mentioned only for explicit external benchmark comparisons.

## Public Status

This repository is currently published as a **backend/CLI preview**.

- The supported public surface is the Rust backend, CLI entry points, benchmark harnesses, and verification scripts.
- The Vue/Tauri frontend source remains in the tree for transparency and future development, but it is not a supported public application yet.
- Do not treat `web/`, `npm run dev`, `npm run build`, `cargo run -p ta_desktop_demo_app`, or Tauri packaging as the public quick-start path for this release.
- No financial, trading, broker, paper-trading, or live-execution workflow in this repo should be treated as production advice or a production trading system.

Desktop release work is tracked separately from the backend-preview status. Maintainers preparing a Windows desktop build should use `docs/PUBLIC_RELEASE.md` and `scripts/run_public_release_gate.ps1`; only artifacts that pass that gate from a clean, signed release environment should be treated as public desktop releases. The current frontend code defaults to the Advanced workbench and ignores legacy guided/simple mode requests, so the staged Simple Mode release docs are blocked until the UI, tests, runbooks, and public copy are realigned around the actual shipped first-run path.

Website-ready end-user documentation for that signed desktop release path is staged under `docs/public/`. Start with `docs/public/README.md` for the public documentation index, first-run guide, data setup, validation guide, troubleshooting, FAQ, and release-page template.

## Stored Benchmark Snapshots

These figures are saved faceoff artifacts, not current perf-gate outputs. The VectorGrid timings for the two active warm app-path rows below were refreshed on `2026-04-16`; the VectorBT timings remain the last stored runs for those same scenarios and keep their original artifact dates.

**Latest `58,300`-pair VectorGrid refresh (`2026-04-16` artifacts; VectorBT reference from `2026-03-13`)**

- Scenario: `Double MA`, `ALMA/ALMA`, exact grid
- Workload: `58,300` backtests over `200,000` bars
- Timing boundary: warm app-path faceoff, same-thread, dataset already loaded
- Hardware: `AMD Ryzen 9 9950X`, `NVIDIA GeForce RTX 4090`
- VectorGrid GPU kernel: `71.329 ms`
- VectorGrid CPU: `4852.416 ms`
- Historical VectorBT CPU workflow (`2026-03-13` artifact): `92377.801 ms`

Reference ratios using the stored VectorBT artifact:

- `1295x` faster than the last stored VectorBT CPU workflow for that scenario
- `68x` faster than the refreshed VectorGrid CPU path on the same scenario

**Latest `250,000`-pair VectorGrid refresh (`2026-04-16` artifacts; VectorBT reference from `2026-04-03`)**

- Scenario: `Double MA`, `ALMA/ALMA`, exact grid
- Workload: `250,000` backtests over `200,000` bars
- Timing boundary: warm app-path faceoff, same-thread, dataset already loaded
- Hardware: `AMD Ryzen 9 9950X`, `NVIDIA GeForce RTX 4090`
- VectorGrid GPU kernel: `172.519 ms`
- VectorGrid CPU: `25472.924 ms`
- Historical VectorBT CPU workflow (`2026-04-03` artifact): `422945.901 ms`

Reference ratios using the stored VectorBT artifact:

- `2452x` faster than the last stored VectorBT CPU workflow for that scenario
- `148x` faster than the refreshed VectorGrid CPU path on the same scenario

**Stored benchmark snapshots**

| Scenario | VectorGrid GPU kernel | VectorGrid CPU | VectorBT CPU |
| --- | --- | --- | --- |
| `200k bars / 58,300 pairs (VectorGrid refreshed 2026-04-16)` | `71.329 ms` | `4852.416 ms` | `92377.801 ms (2026-03-13)` |
| `200k bars / 250,000 pairs (VectorGrid refreshed 2026-04-16)` | `172.519 ms` | `25472.924 ms` | `422945.901 ms (2026-04-03)` |
| `200k bars / 250,000 pairs (March public chart)` | `233.660 ms` | `15202.400 ms` | `445240.337 ms` |

## What This Measures

These README benchmark snapshots are **warm app-path** faceoff numbers, not the tighter Criterion microbenchmark and not the CPU/GPU perf-gate outputs.

- Dataset is loaded once.
- Warmup runs happen before measured trials.
- GPU measured trials reuse resident market-series buffers.
- The `58,300`-pair and `250,000`-pair snapshots use `top_k=10` and `heatmap_bins=0`.
- The latest `250,000`-pair rerun set uses the real-workflow `10`-row benchmark lane.
- VectorBT is measured as a CPU Python workflow on `vectorbt 0.28.4`, with local ALMA generation and local metric aggregation that keeps the faceoff semantics aligned with the app's next-bar workflow.
- Treat the scenario identity and saved-artifact date above as part of the claim. Some rows intentionally combine refreshed VectorGrid-only timings with the last stored VectorBT reference for the same scenario; keep both dates when repeating those comparisons.

This repo also has a tighter engine benchmark boundary for the classic `58,300`-pair ALMA case:

- Criterion anchor: `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`
- Historical anchor: about `89 ms`

Use the warm app-path numbers only for scenario-matched saved-snapshot comparisons with explicit artifact/date context. Use the Criterion benchmark for lower-level engine tracking.

Local preserved benchmark, profiling, and report artifacts now belong under `output/`. Treat `target/` as disposable Cargo/build scratch that can be deleted between runs without intentionally preserving historical evidence.

## What The Backend Does

- Exact-grid optimization for trading strategies, with a mature GPU path for the `Double MA` family.
- GPU-backed validation coverage for the active supported strategy families, including walk-forward and related validation flows.
- CPU fallback support throughout the product.
- Warm in-memory and VRAM-resident execution paths designed for repeated optimization and validation runs on the same dataset.

The desktop UI and execution workspace are not part of the public supported surface yet. They remain in source form so the backend/frontend contract is inspectable while the frontend is being reworked.

## Current GPU Coverage

Broadly, the repo is in a good backend state for the current target set.

- Mature `DoubleMa` GPU optimization and validation path.
- `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid` route through the `DoubleMa` product path.
- Composite-side GPU support exists for `ASR`, `ROCB`, `VEMR`, `MKCS`, `KQSR`, `CARM`, `KVDF`, and `CDMC`.
- `VCC` remains parity-gated.
- `CompositeMo` remains CPU-bound.

The README tracks the public backend coverage status. Internal tracker history and research notes are intentionally kept out of the public source tree.

## Backend Quick Start

Toolchain posture:

- the repo now pins Rust via `rust-toolchain.toml`
- expected channel remains `stable`

Run a CPU-only synthetic `Double MA` optimization through the backend CLI:

```powershell
cargo run -p ta_app_core --features cli --bin ta_cli -- `
  --backend cpu `
  --synth-len 1000 `
  --fast-ma alma `
  --slow-ma alma `
  --fast-period 5:15:5 `
  --slow-period 20:40:10 `
  --heatmap-bins 0 `
  --top-k 3
```

Build the backend workspace without CUDA:

```powershell
cargo check -p ta_app_core --features cli
```

Optional GPU-kernel CLI run, for machines with a working NVIDIA/CUDA runtime:

```powershell
cargo run -p ta_app_core --release --features cli,cuda,cuda-backtest-kernel --bin ta_cli -- `
  --backend gpu-kernel `
  --synth-len 200000 `
  --fast-ma alma `
  --slow-ma alma `
  --fast-period 5:50:1 `
  --slow-period 20:200:5 `
  --heatmap-bins 0 `
  --top-k 10
```

The Tauri desktop app can still be built by maintainers, but it is intentionally not documented as a supported public run path in this backend-preview release.

## Local Quality Gate

For backend-preview changes, start with the Rust backend checks:

```powershell
cargo check -p ta_app_core --features cli
cargo test -p ta_app_core
```

Use the repo-level quality gate before handoff for broad maintainer changes and public-facing cleanup:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1
```

The full maintainer lane still runs frontend checks because the source remains in-tree:

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets --no-deps`
- `npm --prefix web run lint`
- `npm --prefix web run typecheck`

Add workspace tests for higher-risk backend or cross-layer changes:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeWorkspaceTests
```

For frontend workflow/shell changes, use the build-once browser QA bundle instead of manually repeating individual Playwright commands. The default bundle runs workbench smoke, action coverage, and core workflow coverage; add the responsive/regression switches when those surfaces changed.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_frontend_e2e_gate.ps1
powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeBrowserWorkflowQa
```

Add `-IncludeBrowserResponsiveE2E`, `-IncludeBrowserRegressionE2E`, or `-IncludeDesktopSmoke` to the repo gate when those surfaces changed and the local machine has the required browser/desktop smoke setup.

For a direct workspace-wide Rust test run, prefer the stable wrapper instead of raw `cargo test --workspace` on the large Windows desktop surface:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_workspace_tests_stable.ps1
```

The wrapper forces serialized Cargo jobs (`-j 1`) and clears Rust compiler wrappers for the run, which matches the existing repo hardening posture for Windows linker/pagefile-sensitive workspace test passes.

For paper/live trading changes that do not need benchmarks or real credentials, use the targeted execution safety gate:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_execution_safety_gate.ps1
```

Use `-StrictClippy` only when you explicitly want warnings treated as errors. The repo is not yet warning-free under Clippy, so the default lane keeps Clippy advisory while still making it part of the public quality bar.

For public contribution, support, security-reporting, privacy, disclaimer, and license terms, use `CONTRIBUTING.md`, `SUPPORT.md`, `SECURITY.md`, `PRIVACY.md`, `DISCLAIMER.md`, and `LICENSE`.

## Reproduce The Benchmark

Prepare the dedicated faceoff environment before refreshing any reusable VectorBT comparison artifact:

```powershell
py -m venv .venv-vectorbt-faceoff
.\.venv-vectorbt-faceoff\Scripts\python.exe -m pip install --upgrade pip
.\.venv-vectorbt-faceoff\Scripts\python.exe -m pip install -r scripts/requirements-vectorbt-faceoff.lock.txt
```

Run the real-workflow `250,000`-pair warm faceoff scenario with the same interpreter explicitly:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1 `
  -ScenarioPath scripts/vectorbt_double_ma_faceoff_scenario_real_workflow_250k.json `
  -OutDir output/benchmarks/vectorbt_vs_vectorgrid_faceoff_250k_real_workflow `
  -PythonExe .\.venv-vectorbt-faceoff\Scripts\python.exe
```

Scenario file:

- [scripts/vectorbt_double_ma_faceoff_scenario_real_workflow_250k.json](../scripts/vectorbt_double_ma_faceoff_scenario_real_workflow_250k.json)

The saved artifacts currently reflected in the README are historical snapshots. Rerun the matching scenario before repeating the numbers as current.

Each faceoff run now writes the lockfile hash plus same-interpreter Python provenance into the output directory and `summary.json`:

- `python-freeze.txt`
- `python-pip-version.txt`
- `summary.json` `environment.python.requirements_lock`
- `summary.json` `environment.python.pip`

New faceoff artifacts belong under `output/benchmarks/`, which is intentionally ignored. The rows above are historical snapshots; rerun the matching scenario before presenting the numbers as current.

## Performance Gates And Profiling

CPU perf gate:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_cpu_perf_gate.ps1 -Mode baseline
powershell -ExecutionPolicy Bypass -File scripts/run_cpu_perf_gate.ps1 -Mode candidate
```

GPU perf gate:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode baseline
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

ALMA contract-row GPU perf gate:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode baseline
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate
```

Perf baselines are intentionally machine-local and ignored by Git. Run the
`baseline` command first on the machine you are using, then use `candidate` for
same-machine regression checks.

Nsight Compute capture:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1
```

## Repo Layout

- `src-tauri/`: Tauri app crate, commands, desktop integration
- `crates/app_core/`: optimization, validation, CUDA/runtime plumbing
- `crates/strategies/`: strategy configuration and shared strategy logic
- `crates/optimizer/`: optimizer support crate
- `web/`: Vue 3 + Vite frontend source, included but not supported as a public application in this release

## Notes

- Keep CPU-only builds working; CUDA is optional at build time.
- Treat the frontend/Tauri app as internal work-in-progress until the public status section says otherwise.
- For CUDA backtest-kernel work, use the repo perf gates and Nsight before drawing conclusions from single ad-hoc timings.
- The benchmark table is intentionally static and explicit. It is meant to be readable and honest in a GitHub README without relying on generated media.
