# Public Windows Release Runbook

This runbook is the maintainer checklist for publishing VectorGrid as a Windows desktop app.

The default public target is a signed Windows x64 NSIS setup executable. CPU-only is the first recommended public variant because it avoids CUDA driver/runtime ambiguity. GPU variants should be released only with variant-specific evidence and clear system requirements.

Public website and end-user documentation lives under `docs/public/`. Before publishing, verify the release page, installer notes, privacy/disclaimer language, troubleshooting page, and first-run guide match the exact release variant.

The checked-in Vue frontend opens in the Advanced workbench. Public release notes, first-run guidance, screenshots, and clean-machine evidence must describe that actual surface; legacy guided/Simple Mode material must not be published unless that mode is deliberately restored and requalified.

## Release Variant Policy

| Variant | Public label | Required gate |
|---|---|---|
| `cpu` | VectorGrid Windows CPU | `scripts/run_public_release_gate.ps1 -Variant cpu` |
| `gpu-ma` | VectorGrid Windows GPU MA | `scripts/run_public_release_gate.ps1 -Variant gpu-ma -IncludePerf` |
| `gpu-kernel` | VectorGrid Windows GPU Kernel | `scripts/run_public_release_gate.ps1 -Variant gpu-kernel -IncludePerf -IncludeGpuValidationPerf` |

Do not publish artifacts from a dirty worktree. Do not publish unsigned artifacts unless they are explicitly labeled as private tester builds.

The `gpu-ma` authority wiring is complete, but this non-benchmark implementation pass intentionally did not create `scripts/baselines/gpu_ma_perf_baseline.json`. Until a maintainer runs and reviews a serialized `scripts/run_gpu_ma_perf_gate.ps1 -Mode baseline`, both desktop and backend `gpu-ma` performance release rehearsals fail closed before starting their benchmark lanes. The existing GPU-kernel baseline is preserved and is never substituted: it measures `double_ma/gpu_kernel/*` through `Backend::GpuKernel`, whereas the MA authority measures `double_ma/gpu_sweep/*` through `Backend::GpuOnly`, so its historical timing/fingerprint cannot be relabeled safely.

The GPU-kernel row does not automatically claim that the options GPU scorer is promoted. Add `-IncludeOptionsGpuPromotion`, `-OptionsGpuArtifactPath <artifact>`, and exactly one of `-OptionsGpuDatasetId <id>` or `-OptionsGpuImportCsv <csv> -OptionsGpuImportRootSymbol <symbol>` only when making that claim. The public gate then requires the existing options performance baseline plus passing CPU options tests, exact GPU aggregate/ranking parity, promotion checks, and baseline-regression checks. The `gpu-ma` variant cannot opt into this scorer because it does not include `cuda-backtest-kernel`.

## External Requirements

These cannot be completed by repository code alone:

- Windows Authenticode signing through a public-trust route such as Microsoft Store distribution, Azure Artifact Signing / Trusted Signing, or a CA-issued code-signing certificate.
- A stable HTTPS release host for installers, hashes, and release notes.
- Optional updater hosting and updater signing keys if in-app updates are enabled.
- A clean Windows test machine or VM for install, launch, update, and uninstall verification.

Tauri creates Windows installers as MSI or NSIS setup executables. This repo currently configures NSIS because the user-facing artifact is a setup `.exe` and it avoids MSI-specific WiX/VBScript friction for the first release.

## Public Release Command

Run from the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_public_release_gate.ps1 `
  -Variant cpu `
  -ExpectedSignerThumbprint <40-character-certificate-thumbprint>
```

The public gate wraps the release-candidate gate and then enforces:

- clean git worktree;
- generated bundle artifacts rather than only `target/release/ta_desktop_demo_app.exe`;
- passing packaged desktop smoke;
- one passing `scripts/run_execution_safety_gate.ps1` step and one fresh
  authority-mode `scripts/run_live_execution_safety_harness.ps1` run for the
  current candidate; all public compute variants ship the Alpaca-live surface,
  so these are unconditional even though ordinary candidate rehearsals keep
  explicit opt-in switches;
- staged installer artifacts in the release manifest;
- valid Authenticode signatures on both the staged installer and its extracted
  embedded application;
- trusted timestamps and the configured expected publisher thumbprint on both
  signatures;
- exact extracted-payload allowlisting and manifest hash checks for bundled
  resources;
- all allowlisted native release-proof journeys, each bound to the current
  run id, release variant, and extracted application hash, followed by
  independent evidence-manifest revalidation in the public gate;
- a final staged-artifact re-hash against `manifest.json` and
  `SHA256SUMS.txt`, plus a fresh hash and Authenticode check of the extracted
  application.
- when explicitly requested for `gpu-kernel`, a passing options GPU promotion summary with fixed workload identity, CPU options-test evidence, exact CPU/GPU parity evidence, promotion checks, and candidate-baseline checks.

Bundle smoke requires `7z` or `7za`. It verifies the staged installer hash,
extracts the NSIS payload into the release evidence directory, rejects
anything outside the current 10-path payload allowlist, verifies the three
declared resources (reviewed CatBoost DLL, CatBoost license, and sample-data
CSV), and targets that embedded application. Static
inspection and raw-release-binary
smoke cannot satisfy the public gate. This extraction-based smoke does not
prove install UI, shortcuts, registry state, upgrade, or uninstall; the clean-VM
checklist remains mandatory.

The desktop preflight records the selected archive tool and EdgeDriver versions
and fails before launch unless the EdgeDriver major matches the installed
WebView2 runtime major (or Microsoft Edge when no WebView2 runtime is
discoverable). A release runner may set
`VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER` to a compatible executable; when set,
that path is authoritative and an invalid override is not replaced by a
repo/PATH fallback.

For a non-publishable local rehearsal:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta
```

The unsigned rehearsal may prove packaging mechanics, but it is not
public-release complete. It records `rehearsal-passed` and
`public_release_eligible: false`; `-AllowDirtyWorktree` is rejected unless
`-AllowUnsignedBeta` is also present.

To create a clean-machine smoke checklist from a staged release evidence folder:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/write_clean_machine_smoke_checklist.ps1 `
  -Variant cpu `
  -EvidenceDir output/public_release_candidates/<timestamp>-cpu
```

The checklist script writes JSON and Markdown under `output/clean_machine_smoke_checklists/`. It does not replace the actual clean-machine install test; it turns the manual verification into repeatable release evidence and prechecks that the expected release manifest, hashes, staged artifacts, and bundled synthetic sample evidence are present.

## Release Evidence

Each run writes evidence under `output/public_release_candidates/<timestamp>-<variant>/`:

- `public-release-summary.json` and `.md`: public gate result;
- `summary.json` and `.md`: release-candidate gate result;
- `readiness.json`: local readiness doctor;
- `provenance.json` and `.txt`: toolchain, git, environment, and binary identity;
- `manifest.json`: staged artifacts, required resources, signatures, release
  paths, and installer-signature eligibility. It is staging evidence and always
  keeps `publishable: false`; final publication authority is
  `public-release-summary.json` with `status: passed` and
  `public_release_eligible: true`;
- `SHA256SUMS.txt`: hashes for staged artifacts;
- `live-safety-summary.json`, `live-safety-cargo.stdout.log`, and
  `live-safety-cargo.stderr.log`: the current candidate's schema-2,
  authority-eligible locked/offline Cargo evidence. The public gate also
  requires the companion execution-safety step and independently validates the
  harness run mode, timestamps, exact Cargo binary/arguments, credential and
  proxy isolation, 128 pinned passing tests, current source hashes, and all
  three ordinary evidence files with
  `scripts/public_live_safety_evidence_contract.mjs`;
- `options-gpu-promotion-summary.json`: present only when `-IncludeOptionsGpuPromotion` is requested; records fixed workload identity, correctness evidence, promotion checks, and baseline comparisons;
- `packaged-smoke-summary.json`: packaged desktop smoke status. Public release
  requires `desktop_suite: release-proof`,
  `exact_package_release_proof: true`, bundle-mode runtime execution, the
  exact executable extracted from the staged installer, and passing
  `profile_isolation` evidence with `real_profile_unchanged: true` and
  `effective_isolation_proven: true`, a matching schema-2 application
  attestation, and `teardown: completed`. The release-proof native harness
  enables the packaged application's QA-only app-data override only after it
  creates a sentinel-owned root and an exact matching ownership marker. The
  application then verifies every Tauri config/data/local/cache/log resolver,
  keeps WebView data and logs beneath that root, and writes a create-new
  attestation before creating the main window. Partial handshakes, marker or
  run-id mismatches, reparse points, path escapes, missing attestations, changes
  to the real product profile, and unsafe teardown all fail closed;
- `desktop-smoke/logs/release-proof/release-proof-evidence-manifest.json` and
  its schema-2 journey evidence files. The required journey IDs and filenames
  come from `scripts/release_proof_journeys.json`; every file must report
  `passed` for the packaged-smoke run, current variant, and exact extracted
  binary identity. The packaged-smoke wrapper records the copied manifest hash,
  and the public gate rejects missing, extra, stale, foreign, failed, symlinked,
  or mutated proof evidence. The current 13-journey set covers clean start,
  titlebar controls, resize/restore geometry, cold-launch bounds, native data
  flow, local-paper cold restart, options lifecycle, ML native catalog,
  campaign lifecycle, cross-ticker validation, route selection, export, and
  recovery;
- `artifacts/`: publishable staged installers and related update/signature files.

The QA app-data override is runtime-dormant unless the complete harness
handshake is present. It isolates application-owned persistence for executable
smoke and recovery tests; it does not provision a new Windows account and does
not prove installer registry, shortcut, Start-menu, uninstall, or other
machine-wide behavior. Keep the clean-machine/VM release rehearsal as separate
evidence. The QA override also omits the single-instance plugin because its
Windows mutex/window-class identifier cannot be an absolute filesystem path;
ordinary non-isolated desktop smoke remains the authority for single-instance
focus behavior. Other desktop plugins remain registered.

Publish only the contents of `artifacts/`, `SHA256SUMS.txt`, release notes, and any updater metadata required for the chosen distribution path. Keep raw local logs private unless intentionally redacted.

## Signing

Public Windows downloads need Authenticode signing. Unsigned or self-signed installers are acceptable only for local testing or tightly controlled private testers.

Recommended approaches:

- Microsoft Store MSIX for the cleanest SmartScreen experience where feasible.
- Azure Artifact Signing / Trusted Signing for non-Store distribution.
- CA-issued OV/EV-style signing through the certificate issuer's current tooling where Azure signing is not suitable.

Do not commit signing keys, PFX files, passwords, or updater private keys. Signing should be injected by the release environment.

The final gate also requires the expected publisher certificate thumbprint.
Pass `-ExpectedSignerThumbprint` or set
`VECTORGRID_EXPECTED_SIGNER_THUMBPRINT`. This value is a public certificate
identifier, not a private key. The installer and its extracted application must
both match it and both must carry timestamp-certificate evidence.

The preferred publishable signing lane is Tauri's Windows signing configuration
(`bundle.windows.signCommand` or the supported certificate settings), which can
sign the application during the bundle pipeline. See:
<https://v2.tauri.app/distribute/sign/windows/>.

The release-candidate gate also supports a post-build signing hook:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_public_release_gate.ps1 `
  -Variant cpu `
  -PostBuildSignScript C:\secure-release-tools\sign-vectorgrid-artifacts.ps1
```

The hook is called after `cargo tauri build --bundles nsis` and before
provenance, manifest, hashes, and signature evidence are captured. It receives
`-Variant`, `-OutDir`, `-ReleaseBinaryPath`, and `-BundleRoot` when a bundle root
exists. A release environment may also set
`VECTORGRID_POST_BUILD_SIGN_SCRIPT` instead of passing the parameter.

Because the hook runs after NSIS creation, signing the raw executable and outer
installer at this point does not sign the copy already embedded in the
installer. The public gate extracts and checks that inner executable and will
reject a post-build-only signature arrangement. Use Tauri's bundle-time signing
lane, or an explicitly reviewed pre-sign/repack workflow, before treating the
provided post-build SignTool helper as sufficient public-release evidence.

For certificate-store SignTool signing, use the provided hook:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_public_release_gate.ps1 `
  -Variant cpu `
  -PostBuildSignScript .\scripts\sign_windows_artifacts_with_signtool.ps1
```

The hook itself requires either `-CertificateSha1` or `-CertificateSubject`, so a wrapper script or release environment should supply those values without committing certificate material.

Supported environment variables for that hook:

- `VECTORGRID_SIGNTOOL_PATH`
- `VECTORGRID_SIGN_CERT_SHA1`
- `VECTORGRID_SIGN_CERT_SUBJECT`
- `VECTORGRID_SIGN_TIMESTAMP_URL`

## Updater

The updater remains disabled by default in `src-tauri/tauri.conf.json`.

Before enabling it:

1. Generate the Tauri updater signing key outside the repository.
2. Set `bundle.createUpdaterArtifacts = true`.
3. Set `plugins.updater.pubkey` to the public key content.
4. Set `plugins.updater.endpoints` to HTTPS release metadata URLs.
5. Provide `TAURI_SIGNING_PRIVATE_KEY` only in the release environment.
6. Run a full public gate and then a manual update rehearsal from the previous signed version to the new signed version.

Do not enable production updater endpoints in local development builds.

## Manual Clean-Machine Verification

After the public gate passes, verify on a clean Windows 10/11 machine or VM:

- download artifact over the intended public channel;
- verify SHA-256 against `SHA256SUMS.txt`;
- inspect Authenticode publisher and timestamp;
- install without developer tools installed;
- launch from Start menu and desktop shortcut if created;
- confirm the app opens into the documented first-run surface for the release;
- load the bundled synthetic sample dataset, or a known local CSV if the sample is unavailable;
- run a CPU-only first-run optimization and confirm the run summary names the dataset, strategy, candidate, and metrics;
- optionally run validation and confirm the recommendation state is clear;
- copy or save the first-run summary if that surface is promoted by the release docs;
- from the Advanced workbench, export JSON/CSV/XLSX reports;
- verify local data directories are created under the expected app identifier;
- confirm no credentials are bundled and no private local paths are exposed in public logs;
- with paper/sandbox credentials only unless a separately authorized production
  exercise exists, rehearse runner stop, emergency halt, open-order
  cancellation, position flatten/recovery, and restart reconciliation; retain a
  dated operator record because the automated offline harness does not prove
  broker connectivity or manual disaster recovery;
- uninstall and verify the app is removed cleanly.

For GPU variants, repeat with a supported NVIDIA GPU and driver and capture GPU route evidence from the app diagnostics and release perf gates.

## Public User Notes

Every public release should include:

- Windows version and architecture requirements;
- CPU vs GPU variant explanation;
- WebView2 note for older Windows installs;
- provider setup notes for Polygon/Alpaca workflows;
- privacy and local-data handling link;
- `docs/WINDOWS_INSTALL.md` or equivalent release-page install notes;
- no-financial-advice and no-production-trading disclaimer;
- known limitations and supported workflows;
- first-run steps that match the shipped UI, plus the bundled synthetic sample note;
- hashes and publisher/signature identity.
