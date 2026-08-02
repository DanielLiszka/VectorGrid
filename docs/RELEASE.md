# Release Notes

This document tracks release-time desktop integration requirements for VectorGrid.

For the full claim-level verification and release authority model, see `docs/verification_benchmark_release_authority.md`. Its canonical machine-readable policy index is `docs/release_authority_matrix.json`; the release doctor validates it with `node scripts/check_release_authority_matrix.mjs` before running the readiness core.

## Release Evidence Authority

Release evidence is layered. Do not treat one release artifact as proof of a different release question.

| Release question | Primary authority | Notes |
|---|---|---|
| Is the local machine/repo ready for this variant? | `powershell -File scripts/run_release_doctor.ps1 -Variant <variant>` and `readiness.json` | The doctor runs Tauri config sync and release-readiness core. It does not build, stage, hash, or smoke a package. |
| Was a candidate locally rehearsed? | `powershell -File scripts/run_release_candidate_gate.ps1 -Variant <variant>` and `summary.json` / `summary.md` | The release-candidate summary is broad rehearsal evidence, not a replacement for manifest, provenance, hashes, smoke, or output parity. |
| What exactly was built? | `provenance.json`, with `provenance.txt` as secondary human-readable evidence | Use this for build identity, toolchain, and environment. |
| Which assets were staged? | `manifest.json` | Current required resources include the CatBoost native DLL and CatBoost license, bundled synthetic sample CSV, `double_crossover.prebuilt.ptx`, and `options_expression.prebuilt.ptx`. The staging manifest may report `installer_signature_eligible`, but deliberately keeps `publishable: false`; only a passing `public-release-summary.json` is final publication authority. |
| What are the asset hashes? | `SHA256SUMS.txt`, interpreted against `manifest.json` | Hashes alone are not variant authority. |
| Did the packaged artifact launch? | `packaged-smoke-summary.json` | Bundle mode verifies the staged installer hash, enforces the expected NSIS payload allowlist and resource hashes, extracts with `7z`/`7za`, and launches only the extracted product executable. Raw release-binary or static-inspection modes are not bundle runtime proof, and extraction is not install/shortcut/registry/uninstall proof. |
| Are artifact/report outputs still compatible? | `node scripts/run_output_parity_check.mjs` | The focused command is also run by default in the release-candidate gate. |
| Are shipped live-execution controls covered? | `scripts/run_execution_safety_gate.ps1`, `live-safety-summary.json`, and `scripts/public_live_safety_evidence_contract.mjs` | All public CPU/GPU desktop variants expose Alpaca live execution. Public evidence requires the frontend/companion safety gate plus a fresh authority-mode offline harness; neither proves broker connectivity or manual disaster recovery. |

Current release-candidate caveats:

- `scripts/run_release_candidate_gate.ps1` always runs command-registry parity
  and deterministic JSON/CSV/XLSX output parity. A deliberately narrow local
  diagnostic can pass `-IncludeOutputParity:$false`; the public-release wrapper
  always enables it.
- It only runs packaged desktop smoke when `-IncludeDesktopSmoke` is set and `-SkipPackagedSmoke` is not set. `-DesktopSuite` is validated and defaults to `smoke` for ordinary rehearsals.
- The public-release wrapper explicitly passes `-DesktopSuite release-proof` and rejects evidence unless the candidate summary and packaged-smoke summary both identify that exact-package suite. The suite requires a staged bundle, executes the extracted product, and must report a completed isolated-profile lifecycle with the real profile unchanged. It also requires every journey in `scripts/release_proof_journeys.json` to produce schema-2 passing evidence for the current run, release variant, and extracted-binary hash. `scripts/release_proof_evidence_contract.mjs` hashes those files into a no-overwrite, tamper-evident manifest; the public gate independently rebuilds and verifies that manifest before success. Before launch, the native harness creates a sentinel-owned root and exact ownership marker, then enables the implemented QA-only app-data resolver handshake. The packaged application verifies every Tauri config/data/local/cache/log resolver, keeps WebView data and logs under that root, and writes a create-new attestation before opening the main window. Partial handshakes, path escapes, reparse points, marker/run-id mismatches, missing attestations, or changes to the real profile fail closed; use a disposable Windows user/VM when the complete handshake cannot be proven.
- Under desktop `-IncludePerf` or backend `-IncludeGpuPerf`, `gpu-ma` invokes the dedicated `scripts/run_gpu_ma_perf_gate.ps1` CUDA-only MA-sweep profile and `gpu-kernel` invokes `scripts/run_gpu_perf_gate.ps1` with the kernel profile. Their case registries, baseline paths, and summary paths are separate.
- It invokes dedicated GPU validation perf only when `-IncludeGpuValidationPerf` is set on the `gpu-kernel` variant.
- `-IncludeOptionsGpuPromotion` is a separate `gpu-kernel`-only opt-in. It requires `-OptionsGpuArtifactPath` and exactly one of `-OptionsGpuDatasetId` or `-OptionsGpuImportCsv` (plus `-OptionsGpuImportRootSymbol` for CSV). The options promotion gate runs the CPU options test suite, exact aggregate/ranking CPU-GPU parity, Phase 05 promotion checks, and candidate-baseline comparisons before it can pass.
- `-SkipWorkflowGate` exists only for explicitly labeled `-UnsignedBeta`
  topology/static-package rehearsals when performance work is being deferred.
  It cannot be combined with a performance or options-promotion switch, records an explicit
  skipped step plus `workflow_authority_complete: false`, and is rejected for
  any run that is not unsigned. The public-release gate also rejects that
  incomplete authority; this switch is never public-release evidence.
- GPU validation release rehearsal uses the GPU validation gate's default `release-authority` workload profile; `validation-heavy` is a separate CUDA development profile, not the default release profile.
- Benchmark-running release steps inherit the repo-local benchmark mutex from `scripts/benchmark_lock.ps1`; this serializes repo-local gates but does not replace checking unrelated external benchmark/profiler processes.
- Ordinary candidate rehearsals can opt into `-IncludeExecutionSafetyGate` and `-IncludeLiveSafetyHarness`. Every public `cpu`, `gpu-ma`, and `gpu-kernel` package exposes the Alpaca-live command/UI surface, so the public wrapper enables both authorities unconditionally. It then revalidates the current-run schema-2 harness summary, exact locked/offline Cargo identity, 128 pinned tests, source hashes, isolation controls, and candidate step timestamps with `scripts/public_live_safety_evidence_contract.mjs`.

## Public Windows Release Gate

Use `scripts/run_public_release_gate.ps1` for publishable Windows desktop releases. It wraps the release-candidate gate and then enforces public-release evidence:

- clean git worktree; `-AllowDirtyWorktree` is accepted only together with
  `-AllowUnsignedBeta` and produces `rehearsal-passed`, never publishable
  `passed` evidence;
- staged bundle artifacts, not only the raw `target/release/ta_desktop_demo_app.exe`;
- passing packaged desktop smoke;
- one passing execution-safety gate plus fresh, authority-eligible live-safety
  harness evidence for the current candidate; diagnostic simulation and skipped
  safety lanes are rejected;
- installer artifacts listed in `manifest.json`;
- valid Authenticode signatures on both the staged installer and its extracted
  embedded application, trusted timestamps on both, and a match to the
  explicitly configured publisher thumbprint unless explicitly rehearsing with
  `-AllowUnsignedBeta`;
- a final re-hash of every staged artifact against both `manifest.json` and
  `SHA256SUMS.txt`, plus an independent re-hash/signature check of the extracted
  application, immediately before success.
- when `-IncludeOptionsGpuPromotion` is requested, one passing candidate options-promotion step and summary containing the CPU options suite, exact GPU aggregate/ranking parity, promotion checks, and baseline-regression checks.

Default CPU command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_public_release_gate.ps1 `
  -Variant cpu `
  -ExpectedSignerThumbprint <40-character-certificate-thumbprint>
```

Unsigned/dirty runs are for local rehearsal only and must not be published as public releases. See `docs/PUBLIC_RELEASE.md` for the full public release runbook.

For a publishable build, configure Windows signing in Tauri's bundle pipeline so
the application executable is signed before it is embedded. Tauri documents
`bundle.windows.signCommand` for custom Windows signing:
<https://v2.tauri.app/distribute/sign/windows/>.

Release environments can additionally sign post-build artifacts between build
and manifest capture with either:

- `-PostBuildSignScript <path>` on `scripts/run_public_release_gate.ps1` or `scripts/run_release_candidate_gate.ps1`;
- `VECTORGRID_POST_BUILD_SIGN_SCRIPT=<path>` in the release environment.

The post-build hook receives `-Variant`, `-OutDir`, `-ReleaseBinaryPath`, and
`-BundleRoot` when available. It runs after NSIS creation, so it can sign the raw
release executable and outer installer but cannot retroactively sign the
application copy already embedded in that installer. The public gate therefore
extracts the staged NSIS payload and verifies the embedded signature
independently; a post-build-only signing arrangement will fail closed unless it
also performs an explicit safe repack.

Bundle-mode packaged smoke requires `7z` or `7za` on the release machine so it
can inspect and execute the exact embedded application without installing or
silently substituting `target/release/ta_desktop_demo_app.exe`. The current
allowlist contains exactly 10 payload paths: the product executable, declared
CatBoost DLL and CatBoost license, sample-data resource, and known Tauri/NSIS
support files. Bundle smoke independently hash-verifies the three declared
resources (DLL, license, and sample CSV); new payloads must be reviewed and
deliberately added.

Native desktop smoke also resolves the installed WebView2 runtime (falling back
to Microsoft Edge only when WebView2 cannot be found) and rejects an
`msedgedriver` whose major version differs. Set
`VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER` to the exact compatible driver when the
repo/PATH copy is stale; an invalid explicit override fails closed instead of
silently falling back. Release-readiness and WDIO environment evidence record
the archive tool, driver, WebView2/Edge paths, versions, selected compatibility
target, and major-version result before launch.

Run `npm --prefix web run test:desktop:prereqs` for the offline discovery,
override-precedence, version-parsing, compatibility, and archive-tool contract
tests.

Run `node --test scripts/release_proof_evidence_contract.test.mjs` for the
release-proof evidence authority contract. It covers the complete allowlisted
success plus missing, failed, stale-run, foreign-binary, non-allowlisted, and
post-smoke mutation failures.

Run `node --test scripts/public_live_safety_evidence_contract.test.mjs scripts/public_live_release_wiring_contract.test.mjs`
for positive/negative public live-safety evidence parsing, authority routing,
PowerShell parsing, and release-matrix contracts. These tests do not run Cargo
or contact a broker.

## Windows Bundle Configuration

The Windows platform config is intentionally explicit:

- `src-tauri/tauri.windows.conf.json` and root `tauri.windows.conf.json` must remain mirrored.
- The configured public bundle target is `nsis`, producing a setup `.exe`.
- `resources/native/catboostmodel.dll` is bundled as `native/catboostmodel.dll` so packaged ML/native workflows can resolve it from Tauri's resource directory.
- `resources/licenses/CatBoost-LICENSE.txt` is bundled as `licenses/CatBoost-LICENSE.txt`.
- `resources/licenses/TradingView-Lightweight-Charts-LICENSE.txt` and `resources/licenses/TradingView-Lightweight-Charts-NOTICE.txt` are bundled under `licenses/` for the pinned `lightweight-charts` `5.1.0` frontend dependency.
- `resources/sample-data/vectorgrid_synthetic_ohlcv.csv` is bundled as `sample-data/vectorgrid_synthetic_ohlcv.csv` so the documented first-run workflow can use a clean-machine sample fast path once the shipped UI and docs are aligned.
- WebView2 install behavior is declared explicitly with `downloadBootstrapper`; offline WebView2 distribution is a future release-profile decision because it materially increases installer size.

`node scripts/check_tauri_config_sync.mjs` validates the mirror, resource source paths, NSIS target, WebView2 install mode, and installer icon path.

`node --test scripts/lightweight_charts_license_contract.test.mjs` pins the
dependency and lockfile version, verifies the exact reviewed upstream NOTICE
text and Apache-2.0 license hash, and checks both Tauri resource maps plus the
release-manifest and extracted-package smoke contracts.

### CatBoost native dependency

The reviewed Windows native model-applier dependency is the official CatBoost
`v1.2.10` x86-64 release asset. Its immutable source URL, tag commit, byte size,
SHA-256, Apache-2.0 license URL, and license SHA-256 are pinned in
`scripts/baselines/catboost-native-v1.2.10.json`.

Use `scripts/acquire_catboost_native.ps1` for clean-clone acquisition. It
downloads to a staging path, verifies the exact size/hash before publication,
and quarantines an existing mismatched DLL rather than deleting it. Release
manifest generation, release readiness, the manifest/toolchain contract, and
extracted-package smoke all fail closed if the DLL or license differs from the
reviewed baseline. `scripts/build_catboost_native_windows.ps1` is a maintainer
alternative only for a clean source checkout at the pinned tag; dirty source
trees are rejected.

The upstream DLL is not Authenticode-signed. Preserve its reviewed upstream
hash and require a signed/timestamped VectorGrid application and installer.
Organizational redistribution/legal approval remains a release-owner decision;
the engineering checks do not claim to provide that approval.

## Variant Evidence

| Variant/surface | Required evidence | Notes |
|---|---|---|
| CPU-only | CPU build; release doctor; release candidate; provenance/manifest/hash. | Must not require CUDA Toolkit, NVCC, NVIDIA GPU, or driver. |
| GPU MA-sweep | `--features cuda`; release doctor/candidate for `gpu-ma`; `scripts/run_gpu_ma_perf_gate.ps1 -Mode candidate`; NVIDIA driver/runtime evidence. | The dedicated profile uses `gpu_ma_perf_baseline.json`; candidate mode fails before benchmark compilation when that reviewed baseline is absent and never falls back to the GPU-kernel baseline. |
| GPU kernel | `--features cuda-backtest-kernel`; forced-prebuilt release doctor; manifest PTX evidence; GPU perf; GPU validation perf for validation/residency claims. | `cuda-backtest-kernel` implies `cuda`. The current embedded application modules use PTX ISA 9.2 and target `sm_89`, so this variant requires compute capability 8.9 or newer plus a driver that successfully JIT-loads both modules. The release doctor forces `VECTORBT_USE_PREBUILT_PTX=1` and verifies that contract; an end-user CUDA Toolkit is not required. The current qualified device is the RTX 4090 (8.9); broader architecture/driver coverage remains open. |
| Options GPU scorer | `gpu-kernel` candidate/public gate with `-IncludeOptionsGpuPromotion`, explicit artifact and dataset/import identity, CPU options tests, exact CPU/GPU parity, promotion checks, and baseline comparison. | The manifest tracks `options_expression.prebuilt.ptx`; do not treat that packaging evidence alone as options GPU promotion authority. |
| ML Signal native CPU | ML quality gate; real native smoke; reviewed CatBoost DLL/license resources in manifest. | Manifest generation requires the exact pinned CatBoost asset and license unless the DLL is explicitly allowed missing for a non-shipping diagnostic manifest. |
| Live execution enabled | Execution safety gate; no live credentials; safety harness only after exact script review. | Public live release requires manual recovery and emergency-halt posture. |

## Withdrawal And Hotfix Triggers

Withdraw, relabel, or hotfix a release when evidence shows:

- app launch failure;
- stale frontend bundle or command inventory;
- misleading variant label or GPU/runtime requirement;
- published asset mismatch against manifest or hashes;
- CPU-only artifact requiring CUDA or NVIDIA runtime;
- GPU-kernel artifact missing required PTX/resource evidence;
- GPU-kernel release doctor unable to JIT-load either embedded application PTX module on the tested driver;
- hidden CPU fallback under a GPU claim;
- broken artifact/report provenance that could affect user decisions;
- missing or mismatched CatBoost native DLL/license/provenance baseline under current manifest policy;
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
