# Settings, Privacy, Security, And Local State Implementation Plan

## Surface

Primary files:

- `src-tauri/src/settings.rs`
- `src-tauri/src/redaction.rs`
- `src-tauri/src/local_path_policy.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/commands_reports_api.rs`
- `web/src/services/settingsService.ts`
- `web/src/stores/settingsStore.ts`
- `web/src/utils/storage.ts`
- `web/src/utils/core/storageKeys.ts`
- `web/src/components/SettingsWorkspacePanel.vue`
- `SECURITY.md`
- `PRIVACY.md`

## Current Strengths

- The app is local-first and does not require a remote service for core workflows.
- Polygon API keys use keyring first with a fallback path.
- Redaction utilities exist.
- Settings and data roots are already modeled as explicit local concepts.

## Improvement Plan

### SEC-01: Release Secret Hygiene Gate

Risk: Low.

Problem:
Public release readiness needs a mechanical check for accidental local secrets, ignored credential files, signing material, and diagnostic leakage.
Current release readiness warns about Polygon fallback secrets, but it does not scan tracked plus ignored repo-local files for `.env`, PFX/private keys, updater keys, or diagnostic leaks.

Implementation:

- Add a release preflight that scans allowed project paths for:
  - `.env*`
  - provider API keys
  - signing certificates
  - private keys
  - unredacted diagnostic files
  - local config files with credentials
- Scan tracked and ignored files that are inside the repo.
- Fail public release mode unless findings are explicitly waived outside the artifact bundle.
- Rotate any local provider key discovered during release preparation before publication.

Files:

- `scripts/run_release_doctor.ps1`
- a new focused secret-scan/preflight script if this stays separate from release doctor
- `scripts/run_public_release_gate.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `src-tauri/src/redaction.rs`
- `docs/PUBLIC_RELEASE.md`

Verification:

- `powershell -File scripts/run_release_doctor.ps1 -Variant cpu`
- Secret-scan preflight command once added.
- `cargo test -p ta_desktop_demo_app --lib redaction`

### SEC-02: Desktop Trust Boundary Gate

Risk: Low/medium.

Problem:
The desktop command boundary should be part of release evidence, especially where paths, files, credentials, and process launches are involved.

Implementation:

- Add a gate that exercises:
  - local path allow/deny rules
  - reveal/open report commands
  - market-data root validation
  - credential redaction
  - export path restrictions
  - denied traversal/symlink cases
- Include pass/fail status in release candidate summary.

Files:

- `src-tauri/src/local_path_policy.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_data.rs`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/run_public_release_gate.ps1`

Verification:

- `cargo test -p ta_desktop_demo_app --lib local_path_policy`
- `cargo test -p ta_desktop_demo_app --lib commands_reports_api`
- `powershell -File scripts/run_release_doctor.ps1 -Variant cpu`

### SEC-03: Complete Command Surface Risk Registry

Risk: Low.

Problem:
Tauri commands differ in risk. The registry already exists at `src-tauri/command-risk-policy.json` and is checked by `web/scripts/check-tauri-command-risk-policy.mjs`, but it must stay complete as commands are added.

Implementation:

- Harden the existing command risk registry with:
  - command name
  - owner module
  - risk class
  - input validation summary
  - output redaction status
  - test file
- Add a static check for high-risk commands missing registry entries.
- Address the current known missing entry for `save_simple_summary_markdown`.
- Include generated binding coverage status so commands not represented in Specta bindings are intentionally classified.
- Link registry from developer docs.

Files:

- `src-tauri/src/lib.rs`
- `src-tauri/command-risk-policy.json`
- `src-tauri/src/command_bindings.rs`
- `src-tauri/src/commands_*.rs`
- `src-tauri/tests/specta_bindings_export.rs`
- `web/src/tauri/generatedBindings.ts`
- `SECURITY.md`
- `scripts/*`

Verification:

- `cargo test -p ta_desktop_demo_app --lib`
- `cargo test -p ta_desktop_demo_app --test specta_bindings_export`
- `npm --prefix web run check:bindings`
- `node web/scripts/check-tauri-command-risk-policy.mjs`

### PRIV-01: Local Data Inventory For Users

Risk: Low.

Problem:
Users should understand what the app stores locally and how to remove it.

Implementation:

- Add Settings view inventory for:
  - CSV binary cache
  - Polygon cache
  - options data
  - run artifacts
  - reports
  - model bundles/cache
  - credentials location status, without showing secrets
- Add dry-run and confirmed cleanup actions.
- Add public documentation for local storage.

Files:

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/services/settingsService.ts`
- `web/src/components/SettingsWorkspacePanel.vue`
- `PRIVACY.md`

Verification:

- `cargo test -p ta_desktop_demo_app --lib data_cache`
- `cargo test -p ta_desktop_demo_app --test settings_tests`
- `npm --prefix web run test -- src/services/settingsService.test.ts src/components/SettingsWorkspacePanel.test.ts`

### PRIV-02: Legacy Browser Credential Purge

Risk: Low/medium.

Problem:
Old browser localStorage aliases for execution or provider credentials can remain after newer secure storage flows are adopted.

Implementation:

- Add a frontend storage migration that purges legacy credential keys.
- If migration to secure backend storage is possible, do it explicitly and delete browser copies.
- Clarify current behavior: `storage.ts` migrates legacy execution keys forward; purge should happen only after secure storage succeeds or credentials are explicitly cleared.
- Add diagnostics that report presence/absence of legacy keys without exposing values.
- Redact all credential-like keys in exported diagnostics.

Files:

- `web/src/utils/core/storageKeys.ts`
- `web/src/utils/storage.ts`
- `web/src/stores/dataStore.ts`
- `web/src/services/executionService.ts`
- `web/src/services/settingsService.ts`
- `src-tauri/src/redaction.rs`

Verification:

- `npm --prefix web run test -- src/utils/storage.test.ts src/stores/dataStore.actions.test.ts src/services/executionService.test.ts`
- `cargo test -p ta_desktop_demo_app --lib redaction`

### PATH-01: Canonical Path Persistence

Risk: Medium.

Problem:
Paths accepted during setup can become unsafe or ambiguous later if symlinks, drive roots, or directory moves are not rechecked.
Current settings persist the configured path string; shell open/reveal containment is handled separately in `local_path_policy.rs`.

Implementation:

- Store canonical path plus display path for user-selected roots.
- Revalidate root boundaries before every sensitive file operation.
- Reject drive roots, home roots, traversal, and symlink escapes.
- Add clear user-facing errors for invalid or moved paths.

Files:

- `src-tauri/src/settings.rs`
- `src-tauri/src/local_path_policy.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/settingsService.ts`

Verification:

- `cargo test -p ta_desktop_demo_app --lib local_path_policy`
- `cargo test -p ta_desktop_demo_app --test settings_tests`
- `npm --prefix web run test -- src/services/settingsService.test.ts`

## Acceptance Criteria For This Guide

- Release secret scanning covers tracked and ignored repo-local files without printing discovered secret values.
- Credential migration purges legacy browser storage aliases after secure backend storage succeeds or reports a clear non-destructive warning.
- Path policy rejects drive roots, home roots, traversal, and symlink escapes while keeping app-local report/artifact reveals working.
- Tauri command risk registry classifies high-risk commands and generated-binding coverage explicitly.
- Root `SECURITY.md` and `PRIVACY.md` must match actual local storage and credential behavior.

## Deferred Higher-Risk Work

- Network sandboxing or OS-level containment beyond Tauri's current desktop model.
- Changing credential backend semantics without migration tests.
- Automatically deleting user-selected source data.
