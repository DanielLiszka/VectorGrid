# VectorGrid — Windows Release Notes

This repo currently contains both the app and the `vector-ta` library for convenience. Long-term, the desktop app will likely move to its own repo and depend on `vector-ta` as an external crate.

## Build Variants

There are three practical Windows variants:

1. **CPU-only** (default)
2. **GPU (MA sweep)** (`--features cuda`)
3. **GPU (kernel)** (`--features cuda-backtest-kernel`) — fastest path

## Prereqs (Windows)

- Rust stable
- For installers/bundles: `tauri-cli`
  - `cargo install tauri-cli --locked`

> GPU builds require an NVIDIA driver at runtime. The **CUDA Toolkit is not required** for the GPU paths when using prebuilt PTX.

## Local Dev Run (no bundler)

Runs directly via Cargo with the Vue + Vite frontend in `web/` (built to `web/dist` by Tauri config):

```powershell
cd ta_desktop_demo
cargo run -p ta_desktop_demo_app --release
```

GPU variants:

```powershell
cd ta_desktop_demo
cargo run -p ta_desktop_demo_app --release --features cuda
cargo run -p ta_desktop_demo_app --release --features cuda-backtest-kernel
```

## Installer / Bundle Build

From `ta_desktop_demo/`:

```powershell
cd ta_desktop_demo
cargo tauri build
```

GPU variants:

```powershell
cd ta_desktop_demo
cargo tauri build --features cuda
```

```powershell
cd ta_desktop_demo
$env:VECTORBT_USE_PREBUILT_PTX="1"
cargo tauri build --features cuda-backtest-kernel
```

## Backtest Kernel PTX

The GPU backtest kernel (`double_crossover`) can be built either from:

- **Prebuilt PTX** (default fallback): `ta_desktop_demo/crates/app_core/src/kernels/double_crossover.prebuilt.ptx`
- **Local nvcc build** (developer mode)

Environment toggles:

- `VECTORBT_USE_PREBUILT_PTX=1` forces using the prebuilt PTX (recommended for CI/packaging).
- `VECTORBT_REQUIRE_NVCC=1` fails the build if `nvcc` is missing or compilation fails.

Those environment variable names are legacy internal names kept for compatibility; they do not change the product branding, which is `VectorGrid`.

## Vue + Vite Migration Notes (2026-02-26)

This release line includes the completed frontend migration from the legacy static bundle to Vue + Vite while preserving Tauri command/event contracts and storage compatibility.

### Known deltas

- Frontend source/build now lives under `web/` and is built into `web/dist`.
- Tauri config points to `web/dist` (`src-tauri/tauri.conf.json` and root `tauri.conf.json`).
- UI runtime is now Vue-owned; legacy frontend runtime files under `web/src/legacy/*` have been retired.
- Task feedback has been expanded with a global task center (progress, cancellation, timeline, completion/failure toasts).
- Large result surfaces use lazy mounting and row-window virtualization to reduce render overhead.

### Migration validation gates

- `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run lint`
- `npm --prefix web run parity`
- `npm --prefix web run parity:outputs`
- `powershell -ExecutionPolicy Bypass -File scripts/run_workflow_gate.ps1 -Mode candidate`

### Compatibility notes

- No Tauri command contract changes (68 commands parity-checked).
- Existing storage keys remain backward compatible (schema migration adapter in `web/src/utils/storage.ts`).
- Existing artifact/report structures are parity-checked with representative baseline/candidate pairs.
