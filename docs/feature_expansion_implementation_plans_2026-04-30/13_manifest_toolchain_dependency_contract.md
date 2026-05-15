# Manifest, Toolchain, And Dependency Contract

Date: 2026-04-30

## Goal

Make feature expansion safe across Cargo features, npm dependencies, Tauri resources, native libraries, PTX assets, and Windows release manifests.

This is a planning contract. It does not change source behavior.

## Current Owner Files

Toolchain and manifests:

- `Cargo.toml`
- `rust-toolchain.toml`
- `src-tauri/Cargo.toml`
- `crates/app_core/Cargo.toml`
- `crates/strategies/Cargo.toml`
- `crates/optimizer/Cargo.toml`
- `web/package.json`
- `web/package-lock.json`
- `src-tauri/tauri.conf.json`
- `tauri.conf.json`
- `src-tauri/tauri.windows.conf.json`
- `tauri.windows.conf.json`
- `scripts/check_tauri_config_sync.mjs`

Release resources:

- `src-tauri/resources/sample-data/vectorgrid_synthetic_ohlcv.csv`
- `src-tauri/resources/native/catboostmodel.dll`
- `crates/app_core/src/kernels/double_crossover.prebuilt.ptx`
- `crates/app_core/src/kernels/options_expression.prebuilt.ptx`
- `scripts/generate_release_manifest.ps1`
- `scripts/capture_release_provenance.ps1`
- `scripts/run_release_doctor.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/run_public_release_gate.ps1`

Generated contracts:

- `src-tauri/src/command_bindings.rs`
- `src-tauri/tests/specta_bindings_export.rs`
- `web/src/tauri/generatedBindings.ts`
- `web/scripts/check-specta-bindings.mjs`
- `web/scripts/generate-tauri-commands.mjs`
- `web/scripts/check-tauri-parity.mjs`

## Current Toolchain Contract

Rust:

- workspace members are `src-tauri`, `crates/app_core`, `crates/strategies`, and `crates/optimizer`;
- workspace uses resolver `2`;
- `vector-ta` dependency name is hyphenated and Rust import path is `vector_ta`;
- root `Cargo.toml` currently patches `vector-ta = { path = "vendor/vector-ta-0.2.4" }`;
- CPU-only default must not require CUDA, NVCC, NVIDIA driver, GPU, CatBoost native DLL, or PTX loading.

Frontend:

- `web/package.json` pins `packageManager` to `npm@11.6.4`;
- Node engine is `^20.19.0 || >=22.12.0`;
- new npm packages must justify product value, bundle impact, test impact, and maintenance risk.

Tauri:

- canonical Tauri config is `src-tauri/tauri.conf.json`;
- root `tauri.conf.json` is a compatibility mirror and must stay synced;
- Windows resource mapping is in `src-tauri/tauri.windows.conf.json` and root `tauri.windows.conf.json`;
- config sync is checked by `scripts/check_tauri_config_sync.mjs`.

## Feature Wiring Rules

### Rust Features

Existing feature posture:

- `src-tauri/Cargo.toml`
  - `cuda` forwards to `ta_app_core/cuda` and `vector-ta/cuda`;
  - `cuda-backtest-kernel` implies `cuda` and forwards to `ta_app_core/cuda-backtest-kernel`;
  - `nvtx-profiling` forwards to `ta_app_core/nvtx-profiling`.
- `crates/app_core/Cargo.toml`
  - `cuda` forwards to `vector-ta/cuda` and owns optional `cust`;
  - `cuda-backtest-kernel` implies `cuda`;
  - `cli` enables `clap`.

Expansion rules:

- Any new GPU feature must be opt-in and must not enter `default`.
- Any new native/ML feature must expose explicit missing-resource behavior.
- Any new CLI-only dependency must stay behind `cli` or a narrower feature.
- Any new app-core dependency used by `src-tauri` must be checked under CPU-only `cargo check --workspace`.

### VectorTA Version Policy

When updating `vector-ta`:

- update every workspace dependency consistently;
- update or verify the root patch path;
- check CUDA feature forwarding in `src-tauri/Cargo.toml` and `crates/app_core/Cargo.toml`;
- run CPU-only checks before CUDA checks;
- document any CUDA pointer model change in the relevant GPU plan.

Verification:

- `cargo check -p ta_app_core --features cli`
- `cargo check --workspace`
- CUDA feature checks only when the environment supports them.

### Npm Dependency Adoption

Before adding a package:

- identify why existing Vue/Pinia/Tauri/Rust code cannot handle it;
- add package-specific adapter service rather than importing directly in many components;
- run bundle and UI perf budget;
- add tests near the adapter;
- update this contract if the package changes release or runtime assumptions.

Allowed high-ROI candidates from the planning package:

- narrow frontend pacing/backpressure service;
- `@date-fns/tz` or equivalent only if market-time handling cannot stay native;
- Perspective only behind feature flag;
- Arrow/Parquet only after profiling shows JSON/path cost;
- observability packages only behind explicit privacy/release decision.

## Release Resource Contract

Current resources to track:

| Resource | Current path | Release role |
|---|---|---|
| Synthetic sample CSV | `src-tauri/resources/sample-data/vectorgrid_synthetic_ohlcv.csv` | Simple Mode clean-machine first-run path. |
| CatBoost native DLL | `src-tauri/resources/native/catboostmodel.dll` | ML Signal native CPU fallback when release variant includes it. |
| Double crossover PTX | `crates/app_core/src/kernels/double_crossover.prebuilt.ptx` | GPU-kernel route resource evidence. |
| Options expression PTX | `crates/app_core/src/kernels/options_expression.prebuilt.ptx` | Tracked resource, but not public-required unless options GPU is promoted. |

Windows resource mapping:

- `src-tauri/tauri.windows.conf.json` maps sample CSV to `sample-data/vectorgrid_synthetic_ohlcv.csv`;
- `src-tauri/tauri.windows.conf.json` maps CatBoost DLL to `native/catboostmodel.dll`;
- root mirror must stay aligned.

Release manifest expectations:

- resource existence;
- SHA-256 where appropriate;
- bundle inclusion status;
- variant applicability;
- missing-resource policy.

## Manifest Contract Check

Current script:

- `scripts/check_manifest_toolchain_contract.mjs`

Current npm wrapper:

- `npm --prefix web run check:manifest-toolchain`

Inputs:

- Cargo manifests;
- `web/package.json`;
- Tauri configs;
- Windows resource configs;
- release manifest script;
- required resource paths.

Outputs:

- `output/qa/manifest-toolchain-contract.json`;
- missing resource list;
- drift warnings;
- dependency version risk notes.

Initial assertions:

- `vector-ta` version/patch consistency;
- Node/npm engine/package manager matches docs;
- `src-tauri/tauri.windows.conf.json` and root mirror agree;
- sample CSV and CatBoost resource mappings exist;
- prebuilt PTX files exist;
- new docs folder is unignored in `.gitignore`;
- CPU-only default features remain empty in app-core.

## Acceptance Criteria

- New features name their feature flags and release resources before implementation.
- CPU-only checks remain the first verification lane.
- Public release variants cannot silently depend on untracked native/PTX resources.
- Any new npm dependency has a measured bundle/perf plan.
- Manifest/toolchain drift is visible before release rehearsal.
