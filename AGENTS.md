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
- For performance-sensitive changes, run `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` before handoff (or `scripts/run_workflow_gate.ps1 -Mode candidate` at minimum).
- For CUDA backtest-kernel changes, run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` (when GPU is available).

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
