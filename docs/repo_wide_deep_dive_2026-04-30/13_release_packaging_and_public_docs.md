# Release Packaging And Public Documentation Implementation Plan

## Surface

Primary files:

- `README.md`
- `docs/PUBLIC_RELEASE.md`
- `docs/RELEASE.md`
- `SECURITY.md`
- `PRIVACY.md`
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.windows.conf.json`
- `scripts/run_public_release_gate.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/generate_release_manifest.ps1`
- `scripts/write_clean_machine_smoke_checklist.ps1`

## Current Strengths

- Public release planning already exists.
- There are dedicated release gate scripts.
- Manifest generation is part of the release workflow.
- The app has clear CPU and CUDA build variants.

## Improvement Plan

### REL-01: Public Release Baseline Bootstrap

Risk: Low.

Problem:
Some release gates depend on ignored local baseline/evidence files. A clean contributor or release machine should know exactly what is missing.

Implementation:

- Add a bootstrap script or preflight mode that reports required local baselines and how to generate them.
- Do not silently skip required public evidence.
- Add release docs explaining baseline ownership and storage.

Files:

- `scripts/run_public_release_gate.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `docs/PUBLIC_RELEASE.md`
- `docs/RELEASE.md`

Verification:

- `powershell -File scripts/run_release_candidate_gate.ps1 -Variant cpu -SkipPackagedSmoke -SkipWorkspaceTests`
- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta -SkipWorkspaceTests`

### REL-02: Output Parity Required For Public Release

Risk: Low.

Problem:
Release candidate docs indicate output parity can be skipped. For public release, parity should be present or explicitly waived.

Implementation:

- Add a strict public release mode requiring output parity evidence.
- Include output parity summary in generated release manifest.
- Document the bypass only for internal builds.

Files:

- `scripts/run_public_release_gate.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/run_output_parity_check.mjs`
- `scripts/generate_release_manifest.ps1`
- `docs/PUBLIC_RELEASE.md`

Verification:

- `node scripts/run_output_parity_check.mjs`
- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta -SkipWorkspaceTests`

### REL-03: Node And NPM Reproducibility Check

Risk: Low.

Problem:
Rust lockfiles are visible, but frontend reproducibility needs equally explicit release checks.
`web/package.json` already declares `packageManager: npm@11.6.4` and Node `engines`; release scripts need to enforce and capture them.

Implementation:

- Require `npm ci` for release builds.
- Check Node version against `.nvmrc`, `.node-version`, or `web/package.json` `engines` if present.
- Capture Node and npm versions in release summary/manifest.
- Fail release gate on lockfile drift.

Files:

- `web/package.json`
- `web/package-lock.json`
- `scripts/run_public_release_gate.ps1`
- `scripts/run_release_candidate_gate.ps1`

Verification:

- `npm --prefix web ci`
- `npm --prefix web run typecheck`
- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta -SkipWorkspaceTests`

### REL-04: Unsigned Beta Must Be Mechanically Labeled

Risk: Low/medium.

Problem:
Unsigned beta builds are useful, but they should be impossible to confuse with signed public release artifacts.
Current `AllowUnsignedBeta` relaxes signature enforcement and records a boolean summary, but it does not rename artifacts or mark manifest entries non-publishable.

Implementation:

- If `AllowUnsignedBeta` is used, force:
  - artifact filename contains `unsigned-beta`
  - manifest marks it non-publishable
  - manifest entries include `publishable: false` and `unsigned_beta: true`
  - docs call out expected OS warnings
  - summary includes unsigned status
- Keep signed release path unchanged.

Files:

- `scripts/run_public_release_gate.ps1`
- `scripts/generate_release_manifest.ps1`
- `docs/PUBLIC_RELEASE.md`

Verification:

- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta -SkipWorkspaceTests`
- Manifest inspection test once added.

### REL-05: Bundled Resource And Sample Data Evidence

Risk: Low.

Problem:
Release readiness should prove that public first-run resources are inside the installer.
The current manifest records required resources, including CatBoost, sample CSV, `double_crossover.prebuilt.ptx`, and `options_expression.prebuilt.ptx`, but hashes only staged artifacts. Tauri Windows config bundles CatBoost and sample CSV resources; PTX inclusion is a separate GPU asset/package concern.

Implementation:

- Include sample data, PTX files for GPU builds, model fixtures if any, and docs in the release manifest.
- Record per-resource existence, SHA-256, and bundle-inclusion status.
- Add a clean-machine checklist item that opens Simple Mode and runs the sample workflow.

Files:

- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.windows.conf.json`
- `scripts/generate_release_manifest.ps1`
- `scripts/write_clean_machine_smoke_checklist.ps1`
- `docs/PUBLIC_RELEASE.md`

Verification:

- `powershell -File scripts/generate_release_manifest.ps1`
- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta -SkipWorkspaceTests`

### REL-06: Signing And Updater Rehearsal

Risk: Medium.

Problem:
Public desktop releases need rehearsed signing and, if enabled, updater metadata.

Implementation:

- Add release docs for:
  - Windows code-signing prerequisites
  - certificate storage
  - timestamp server
  - updater key handling if enabled
  - rollback and revoked release process
- Add dry-run checks that verify required env vars are present without printing secrets.

Files:

- `docs/PUBLIC_RELEASE.md`
- `docs/RELEASE.md`
- `scripts/run_public_release_gate.ps1`
- `src-tauri/tauri.conf.json`

Verification:

- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta -SkipWorkspaceTests`
- Manual signing rehearsal on a release machine.

### REL-07: Documentation Drift Sweep

Risk: Low.

Problem:
Release docs can drift behind CUDA, options, ML Signal, and Simple Mode behavior.

Implementation:

- Add a release-doc checklist covering:
  - CPU vs GPU variants
  - CUDA/PTX packaging
  - Simple Mode public path
  - Polygon data limits
  - options research caveats
  - ML Signal research caveats
  - local storage/privacy
  - support and troubleshooting
- Link current gates and commands.

Files:

- `README.md`
- `docs/PUBLIC_RELEASE.md`
- `docs/RELEASE.md`
- `PRIVACY.md`
- `SECURITY.md`

Verification:

- Markdown link check if available.
- Manual doc review before public release.

## Acceptance Criteria For This Guide

- Public release gates report pass, fail, skipped, or waived status for every required evidence lane.
- Unsigned beta artifacts are mechanically labeled and marked non-publishable in manifest output.
- `npm ci` and Node/npm version checks run before frontend release builds.
- Bundled resources and GPU/PTX assets record existence, SHA-256, and bundle-inclusion status in release manifests when included.
- New public docs describe CPU/GPU variants, Polygon limitations, local storage, signing status, and support boundaries accurately.

## Deferred Higher-Risk Work

- Auto-updater enablement without a signed release rehearsal.
- Public distribution of experimental native model bundles.
- Publishing GPU builds without a GPU clean-machine smoke run.
