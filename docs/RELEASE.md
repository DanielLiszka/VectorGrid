# Release Notes

This document tracks release-time desktop integration requirements for VectorGrid.

For the full claim-level verification and release authority model, see `docs/verification_benchmark_release_authority.md`.

## Release Evidence Authority

Release evidence is layered. Do not treat one release artifact as proof of a different release question.

| Release question | Primary authority | Notes |
|---|---|---|
| Is the local machine/repo ready for this variant? | `powershell -File scripts/run_release_doctor.ps1 -Variant <variant>` and `readiness.json` | The doctor runs Tauri config sync and release-readiness core. It does not build, stage, hash, or smoke a package. |
| Was a candidate locally rehearsed? | `powershell -File scripts/run_release_candidate_gate.ps1 -Variant <variant>` and `summary.json` / `summary.md` | The release-candidate summary is broad rehearsal evidence, not a replacement for manifest, provenance, hashes, smoke, or output parity. |
| What exactly was built? | `provenance.json`, with `provenance.txt` as secondary human-readable evidence | Use this for build identity, toolchain, and environment. |
| Which assets were staged? | `manifest.json` | Current required resources include the CatBoost native DLL and `double_crossover.prebuilt.ptx`. |
| What are the asset hashes? | `SHA256SUMS.txt`, interpreted against `manifest.json` | Hashes alone are not variant authority. |
| Did the packaged artifact launch? | `packaged-smoke-summary.json` | Check `smoke_mode`; raw release-binary smoke is not bundle-level packaging proof. |
| Are artifact/report outputs still compatible? | `node scripts/run_output_parity_check.mjs` | Output parity is separate authority from the release-candidate gate. |

Current release-candidate caveats:

- `scripts/run_release_candidate_gate.ps1` runs `npm --prefix web run parity`, then skips output-parity sample baselines.
- It only runs packaged desktop smoke when `-IncludeDesktopSmoke` is set and `-SkipPackagedSmoke` is not set.
- It invokes generic GPU perf under `-IncludePerf` for non-CPU variants, but that is not a dedicated GPU MA-sweep authority.
- It invokes dedicated GPU validation perf only when `-IncludeGpuValidationPerf` is set on the `gpu-kernel` variant.
- GPU validation release rehearsal uses the GPU validation gate's default `release-authority` workload profile; `validation-heavy` is a separate CUDA development profile, not the default release profile.
- Benchmark-running release steps inherit the repo-local benchmark mutex from `scripts/benchmark_lock.ps1`; this serializes repo-local gates but does not replace checking unrelated external benchmark/profiler processes.
- It references `scripts/run_live_execution_safety_harness.ps1`; review that exact script before treating the harness as release authority.

## Variant Evidence

| Variant/surface | Required evidence | Notes |
|---|---|---|
| CPU-only | CPU build; release doctor; release candidate; provenance/manifest/hash. | Must not require CUDA Toolkit, NVCC, NVIDIA GPU, or driver. |
| GPU MA-sweep | `--features cuda`; release doctor/candidate for `gpu-ma`; NVIDIA driver/runtime evidence. | Current release-candidate `-IncludePerf` uses the generic GPU perf gate. |
| GPU kernel | `--features cuda-backtest-kernel`; `VECTORBT_USE_PREBUILT_PTX=1` for packaged kernel path; manifest PTX evidence; GPU perf; GPU validation perf for validation/residency claims. | `cuda-backtest-kernel` implies `cuda`; runtime NVIDIA driver is still required. |
| Options GPU scorer | Options promotion gate; CPU/GPU ranking parity; options tests. | Current manifest does not list `options_expression.prebuilt.ptx`; that is a future packaging-evidence gap. |
| ML Signal native CPU | ML quality gate; native smoke; CatBoost DLL resource in manifest. | Manifest generation requires the CatBoost native DLL unless explicitly allowed missing for non-shipping diagnostics. |
| Live execution enabled | Execution safety gate; no live credentials; safety harness only after exact script review. | Public live release requires manual recovery and emergency-halt posture. |

## Withdrawal And Hotfix Triggers

Withdraw, relabel, or hotfix a release when evidence shows:

- app launch failure;
- stale frontend bundle or command inventory;
- misleading variant label or GPU/runtime requirement;
- published asset mismatch against manifest or hashes;
- CPU-only artifact requiring CUDA or NVIDIA runtime;
- GPU-kernel artifact missing required PTX/resource evidence;
- hidden CPU fallback under a GPU claim;
- broken artifact/report provenance that could affect user decisions;
- missing CatBoost native DLL under current manifest policy;
- live execution safety failure or credential leakage.

## Desktop Updater

The updater integration is intentionally dormant in development and local builds.

- Rust plugin: `tauri-plugin-updater`
- Frontend bridge: `web/src/services/desktopUpdater.ts`
- Productization hook: `web/src/services/desktopProductization.ts`
- Current config: `src-tauri/tauri.conf.json` has `plugins.updater.active = false`

Before enabling update checks for a signed release:

1. Generate and store the Tauri updater signing key outside the repository.
2. Publish signed update artifacts and the update manifest to an HTTPS endpoint.
3. Set `plugins.updater.active = true`.
4. Add the production endpoint to `plugins.updater.endpoints`.
5. Set `plugins.updater.pubkey` to the public key matching the signing key.
6. Run `cargo check -p ta_desktop_demo_app`, `npm --prefix web run typecheck`, and the frontend QA gates before packaging.

Do not point local or test builds at a production updater endpoint. Keep update errors non-fatal at startup so a missing endpoint cannot block normal offline use.

## Native Desktop Plugins

Native shell integrations should use Tauri plugins instead of handwritten shell fallbacks:

- File and folder pickers: `tauri-plugin-dialog`
- Clipboard writes: `tauri-plugin-clipboard-manager`, with browser fallbacks for web-mode tests
- Opening or revealing files/directories: `tauri-plugin-opener`
- Single-instance focus handoff: `tauri-plugin-single-instance`
- Restart after an installed update: `tauri-plugin-process`

When adding frontend calls to a plugin, update `src-tauri/capabilities/main.json` and `web/scripts/check-tauri-capabilities.mjs` together.
