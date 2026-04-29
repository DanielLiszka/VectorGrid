# Public Windows Release Runbook

This runbook is the maintainer checklist for publishing VectorGrid as a Windows desktop app.

The default public target is a signed Windows x64 NSIS setup executable. CPU-only is the first recommended public variant because it avoids CUDA driver/runtime ambiguity. GPU variants should be released only with variant-specific evidence and clear system requirements.

## Release Variant Policy

| Variant | Public label | Required gate |
|---|---|---|
| `cpu` | VectorGrid Windows CPU | `scripts/run_public_release_gate.ps1 -Variant cpu` |
| `gpu-ma` | VectorGrid Windows GPU MA | `scripts/run_public_release_gate.ps1 -Variant gpu-ma -IncludePerf` |
| `gpu-kernel` | VectorGrid Windows GPU Kernel | `scripts/run_public_release_gate.ps1 -Variant gpu-kernel -IncludePerf -IncludeGpuValidationPerf` |

Do not publish artifacts from a dirty worktree. Do not publish unsigned artifacts unless they are explicitly labeled as private tester builds.

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
powershell -ExecutionPolicy Bypass -File scripts/run_public_release_gate.ps1 -Variant cpu
```

The public gate wraps the release-candidate gate and then enforces:

- clean git worktree;
- generated bundle artifacts rather than only `target/release/ta_desktop_demo_app.exe`;
- passing packaged desktop smoke;
- staged installer artifacts in the release manifest;
- valid Authenticode signatures unless `-AllowUnsignedBeta` is explicitly used.

For a non-publishable local rehearsal:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta
```

The unsigned rehearsal may prove packaging mechanics, but it is not public-release complete.

## Release Evidence

Each run writes evidence under `output/public_release_candidates/<timestamp>-<variant>/`:

- `public-release-summary.json` and `.md`: public gate result;
- `summary.json` and `.md`: release-candidate gate result;
- `readiness.json`: local readiness doctor;
- `provenance.json` and `.txt`: toolchain, git, environment, and binary identity;
- `manifest.json`: staged artifacts, required resources, signatures, and release paths;
- `SHA256SUMS.txt`: hashes for staged artifacts;
- `packaged-smoke-summary.json`: packaged desktop smoke status;
- `artifacts/`: publishable staged installers and related update/signature files.

Publish only the contents of `artifacts/`, `SHA256SUMS.txt`, release notes, and any updater metadata required for the chosen distribution path. Keep raw local logs private unless intentionally redacted.

## Signing

Public Windows downloads need Authenticode signing. Unsigned or self-signed installers are acceptable only for local testing or tightly controlled private testers.

Recommended approaches:

- Microsoft Store MSIX for the cleanest SmartScreen experience where feasible.
- Azure Artifact Signing / Trusted Signing for non-Store distribution.
- CA-issued OV/EV-style signing through the certificate issuer's current tooling where Azure signing is not suitable.

Do not commit signing keys, PFX files, passwords, or updater private keys. Signing should be injected by the release environment.

The release-candidate gate supports a post-build signing hook:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_public_release_gate.ps1 `
  -Variant cpu `
  -PostBuildSignScript C:\secure-release-tools\sign-vectorgrid-artifacts.ps1
```

The hook is called after `cargo tauri build --bundles nsis` and before provenance, manifest, hashes, and signature evidence are captured. The hook receives `-Variant`, `-OutDir`, `-ReleaseBinaryPath`, and `-BundleRoot` when a bundle root exists. A release environment may also set `VECTORGRID_POST_BUILD_SIGN_SCRIPT` instead of passing the parameter.

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
- run a CPU-only optimization and validation;
- export JSON/CSV/XLSX reports;
- verify local data directories are created under the expected app identifier;
- confirm no credentials are bundled and no private local paths are exposed in public logs;
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
- hashes and publisher/signature identity.
