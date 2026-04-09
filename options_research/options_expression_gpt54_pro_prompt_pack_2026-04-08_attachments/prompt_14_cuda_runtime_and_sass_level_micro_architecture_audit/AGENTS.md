# AGENTS.md — `ta_desktop_demo`

Instructions for AI coding agents working in this repository.

## Repo layout

- Rust workspace root: `Cargo.toml`
- Tauri app crate (UI + commands): `src-tauri/` (`ta_desktop_demo_app`)
- Core/logic crates:
  - `crates/app_core/` (`ta_app_core`)
  - `crates/strategies/` (`ta_strategies`)
  - `crates/optimizer/` (`ta_optimizer`)
- Generated artifacts: `dist/`, `target/` (avoid editing unless required)

## Repo navigation

- Start with `REPO_MAP.md` before broad repo search. It is the canonical high-level routing document for current workspace structure, task entry points, strategy coverage, runtime ownership, and verification surfaces.
- Use `REPO_MAP.md` to narrow the search to the relevant backend/frontend/runtime area, then open the specific files for the task.
- Keep `AGENTS.md` focused on agent policy, invariants, and workflow constraints; keep `REPO_MAP.md` focused on project structure and task-oriented navigation.

## Common commands (PowerShell)

- Unit tests (recommended): `cargo test --workspace`
- Single crate: `cargo test -p ta_app_core`
- Run app (release): `cargo run -p ta_desktop_demo_app --release`
- Run with CUDA features:
  - `cargo run -p ta_desktop_demo_app --release --features cuda`
  - `cargo run -p ta_desktop_demo_app --release --features cuda,cuda-backtest-kernel`

## VectorTA (`vector-ta`) crate usage

- Dependency name in `Cargo.toml` is `vector-ta` (hyphenated).
- Rust import path is `vector_ta` (hyphen becomes underscore), e.g. `use vector_ta::...`.
- Keep the `vector-ta` version consistent across all workspace crates when updating.
- CUDA feature wiring:
  - `src-tauri/Cargo.toml`: `cuda` and `cuda-backtest-kernel` forward to `ta_app_core`.
  - `crates/app_core/Cargo.toml`: forwards `cuda` to `vector-ta/cuda`.

## Coding conventions

- Prefer small, surgical changes; avoid unrelated refactors.
- Keep CPU-only builds working without any CUDA toolchain installed.
- Add/adjust tests close to the crate being changed (usually under `crates/*`).
- Never run multiple benchmarks at the same time. Before starting any benchmark or profiling run, verify no other benchmark process is still active, and serialize benchmark executions so timing conclusions are not contaminated by self-inflicted load.
- For performance-sensitive changes, run `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` before handoff (or `scripts/run_workflow_gate.ps1 -Mode candidate` at minimum).
- For CUDA backtest-kernel changes, run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` (when GPU is available).
- Nvidia Nsight and Nvidia CUDA profiling must be launched from an elevated PowerShell terminal. If `Nsight Compute`, `Nsight Systems`, or CUDA profiling scripts fail due to attach/permission issues, first verify the PowerShell session is running as Administrator.

## Frontend work policy

- For frontend-only work, stay within the Vue 3/web surface unless the user explicitly asks for backend or Tauri changes.
- When working on frontend UI, presentation, Vue 3 behavior, or frontend verification, prefer the relevant available Codex plugins and skills when they improve implementation or review quality.
- Default frontend tooling preference:
  - `build-web-apps:frontend-skill` for UI implementation and presentation work
  - `build-web-apps:web-design-guidelines` for UI/UX/accessibility review
  - `Figma` skills when matching or implementing a provided design
  - `playwright` and `screenshot` for browser-side and visual verification
- Treat backend contracts as fixed during frontend-only work unless the user explicitly expands scope.

## Autonomy Policy
If you're working towards goals, do NOT end your turn. This allows for continuous autonomous work.

The user will interrupt you when required, but they will mostly provide steering messages.

Do not pester the user by ending your turn after a unit of work, as that requires them to keep nudging you to keep working.

You MUST continue working autonomously towards any known objectives until the user interrupts you. Do NOT end your turn until there is absolutely nothing left to do.

## Execution quality

- Use the full set of available tools and skills when it improves solution quality, verification quality, or speed of understanding.
- Always use sub-agents whenever delegation is beneficial to gather context about the codebase, check verification quality, or context preservation. Use the GPT-5.4 model with high reasoning effort for all sub-agents.
- Prefer maximal beneficial use of sub-agents for bounded analysis, context gathering, profiling, and verification tasks, especially when scanning separate backend/frontend/runtime surfaces in parallel will preserve context and reduce mistakes.
- Use profiling, targeted diagnostics, and existing repo tooling before guessing about performance or parity issues.
- Favor evidence-backed implementation over assumption-backed implementation, especially for CUDA/runtime-path changes.

## Strategy-local architecture

- Prefer strategy-local code organization over large generic execution layers when adding new optimization or validation backends.
- For a strategy-specific backend/frontend feature, make it obvious where the code lives so contributors do not need to read the whole repo to change one strategy.
- Keep backend strategy code grouped by strategy whenever practical. Preferred shape:
  - request/config
  - CPU path
  - GPU path
  - hybrid path
  - validation adapter
  - tests
- Keep frontend strategy code grouped by strategy whenever practical:
  - request builder/normalization
  - capability summary
  - strategy-specific form/config handling
  - tests
- Shared infrastructure should stay shared only for truly cross-cutting primitives such as:
  - CUDA upload/buffers
  - top-k/reduction
  - common metric/stat helpers
  - artifact/report plumbing
- Current architecture target for supported GPU strategies:
  - optimization stage should be GPU-executed end to end within that stage, with prices/derived series/candidate buffers kept VRAM-resident until compact results are returned
  - validation stage should separately support GPU-executed end to end execution within that stage, rerunning backtests as needed rather than depending on a chained optimization-to-validation device pipeline
  - CPU fallback must remain supported
  - hybrid routes are acceptable when a specific component such as optional `ADX` remains CPU-preferred
- For `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`, the current product optimization route is the `double_ma` family, not `CompositeCpu`.
- For `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`, the target route policy is:
  - `CompositeTrend`: `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on, `CpuOnly` fallback
  - `BreakoutMomentum`: `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on, `CpuOnly` fallback
  - `RegimeHybrid`: `GpuHybridKernel` in its current ADX-bearing form, `CpuOnly` fallback, with full `GpuKernel` only if a true no-ADX effective variant is later implemented

### Profile file map

- For `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`, contributors should start in these locations before broader repo search:
  - request shaping / backend selection:
    - `web/src/services/optimizeRequestBuilder.ts`
    - `web/src/services/optimizeService.ts`
    - `web/src/utils/strategyOptimizationCapabilities.ts`
  - optimization execution and route metadata:
    - `crates/app_core/src/double_ma.rs`
    - `crates/app_core/src/composite_profile_route.rs`
  - validation execution:
    - `crates/app_core/src/double_ma_validate.rs`
    - `src-tauri/src/commands_validation.rs`
  - strategy preset/test helpers:
    - `crates/app_core/src/strategy_presets.rs`
    - `crates/app_core/src/double_ma_test_support.rs`
  - representative tests:
    - `web/src/services/optimizeRequestBuilder.test.ts`
    - `web/src/services/optimizeService.test.ts`
    - `crates/app_core/src/double_ma.rs`
    - `crates/app_core/src/double_ma_validate.rs`
- Shared primitives that intentionally remain shared:
  - CUDA upload and kernel runtime plumbing in `crates/app_core/src/gpu_backtest_kernel.rs`
  - common regime/filter computation in `crates/app_core/src/double_ma.rs`
  - validation reduction logic in `crates/app_core/src/validation_pipeline.rs`
  - Tauri cross-ticker orchestration in `src-tauri/src/commands_validation.rs`

## Allocation policy (performance-critical)

- Default to malloc-like allocation (`Vec::with_capacity` + `set_len` / `MaybeUninit`) only when every element is guaranteed to be written before any read.
- For warmup-prefix semantics (for example a leading `0` or `NaN` region), initialize only the required prefix and leave the tail uninitialized when downstream code fully overwrites it.
- Keep full-fill allocation (`vec![0; n]`, `vec![f64::NAN; n]`, etc.) when the fill value is part of behavior and may be observed without guaranteed overwrite.
- Limit uninitialized-allocation usage to plain data types (`Copy` types) and keep the safety assumptions next to the code path.

## Skills (Codex CLI)

If the user explicitly requests one of these skills (or the task clearly matches the description), open the
corresponding `SKILL.md` and follow its workflow for that turn.

### Available skills

- `skill-creator`: Guide for creating effective skills. (file: `C:/Users/dlisz/.codex/skills/.system/skill-creator/SKILL.md`)
- `skill-installer`: Install Codex skills into `$CODEX_HOME/skills` from a curated list or a GitHub repo path. (file: `C:/Users/dlisz/.codex/skills/.system/skill-installer/SKILL.md`)
