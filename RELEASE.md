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

## Public Release Operations

### Supported public release scope

- Platform: Windows x64 only
- Variants:
  - CPU-only
  - GPU (MA sweep)
  - GPU (kernel)
- Public support posture is intentionally narrow:
  - CPU-only is the broadest compatibility lane
  - GPU variants require a working NVIDIA driver at runtime
  - this repo does not currently promise macOS or Linux release artifacts
  - only the latest tagged release and any immediate hotfix on top of it should be treated as current

### Public release checklist

Before publishing a public release or prerelease:

1. confirm which variant(s) are being published and name them explicitly
2. run the repo quality lane and the relevant build verification for those variants
3. confirm `web/dist` handoff, command parity, and Tauri config sync are still green
4. confirm release assets use the normalized naming convention below
5. draft release notes using the minimum section set below
6. keep benchmark evidence linked as supporting material rather than as the release note body

### Release asset naming

When manual release artifacts are attached to a GitHub release, normalize the attachment names to:

- `vectorgrid-<version>-windows-x64-cpu.<ext>`
- `vectorgrid-<version>-windows-x64-gpu-ma.<ext>`
- `vectorgrid-<version>-windows-x64-gpu-kernel.<ext>`

Use the same variant labels in release notes, perf artifacts, and support discussions. Do not mix internal experiment names into public release attachment names.

### Release notes minimum sections

Every public release or prerelease note should include:

1. summary of what changed
2. supported platform and published variant(s)
3. verification summary
4. GPU/driver requirements and known constraints
5. compatibility or migration notes
6. known issues or deferred items

Benchmark or faceoff evidence may be linked, but should stay subordinate to operational release guidance.

### Post-release validation and rollback posture

- After publishing, rerun at least one smoke check against the released artifact or an equivalent candidate build.
- Treat the first `72` hours after publication as the high-attention triage window for launch, packaging, and compatibility regressions.
- Prefer a hotfix or release withdrawal when a public release has any of these problems:
  - app fails to launch
  - wrong frontend bundle or stale command inventory is shipped
  - variant labeling or GPU requirements are misleading
  - core shell/report-opening flows are broken
  - the published asset does not match the documented variant
- Use `scripts/post_release_telemetry_review.mjs` as supplemental follow-up evidence, not as the sole release gate.
- Keep retained local benchmark/profiling evidence under `output/`; do not rely on `target/` for anything you want to survive routine build-cache cleanup.

## Vue + Vite Migration Notes (2026-02-26)

This release line includes the completed frontend migration from the legacy static bundle to Vue + Vite while preserving Tauri command/event contracts and storage compatibility.

### Known deltas

- Frontend source/build now lives under `web/` and is built into `web/dist`.
- Tauri config points to `web/dist`, with `src-tauri/tauri.conf.json` using `../web/dist` and the root `tauri.conf.json` mirror using `./web/dist`.
- Windows packaging now also uses `src-tauri/tauri.windows.conf.json` and the root `tauri.windows.conf.json` mirror for app-shipped native resources such as the CPU CatBoost `MlSignal` DLL.
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

For high-risk non-options UI changes, pair the automated accessibility smoke with the representative manual review slices in `web/e2e/accessibility-review-matrix.md`. Browser review is the default; add desktop review when the change affects native titlebar, geometry, OS notifications, or open/reveal path behavior.

### Compatibility notes

- No Tauri command contract changes (68 commands parity-checked).
- Existing storage keys remain backward compatible (schema migration adapter in `web/src/utils/storage.ts`).
- Existing artifact/report structures are parity-checked with representative baseline/candidate pairs.

### Scoped migration freeze helper

- `node scripts/check_migration_freeze_scope.mjs` is a narrow local guard for explicit migration or stabilization windows only.
- It is not a general repo-quality gate and should not be treated as an always-on repo policy.
- When it is used, keep the allowlist in that script aligned with the approved migration note for the current window.
- The current allowlist is intentionally small: `web/`, `scripts/baselines/`, and a short set of top-level docs/scripts that belonged to the Vue + Vite migration lane.
