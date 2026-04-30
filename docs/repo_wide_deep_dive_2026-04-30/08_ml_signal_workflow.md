# ML Signal Workflow Implementation Plan

## Surface

Primary files:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/services/historyService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/components/MlSignalShellPanel.vue`
- `scripts/run_ml_signal_quality_gate.ps1`
- `scripts/run_ml_signal_native_smoke.ps1`
- `scripts/train_ml_signal_catboost.py`
- `configs/ml_signal_catboost_default.json`
- `src-tauri/src/run_artifacts.rs`

## Current Strengths

- ML Signal has a clear workflow for dataset export, model bundle use, scoring, validation, and policy sweep.
- Recent hardening added stronger lineage and frontend response handling.
- A dedicated quality gate exists for this surface.
- The workflow is already framed as research/evidence rather than live automation.

## Improvement Plan

### MLS-01: Strict Score Cache Contract

Risk: Low/medium.

Problem:
Score-cache provenance has been substantially hardened, including manifest and fingerprint support. The remaining improvement is to make release-quality acceptance stricter than research inspection, so older or partial caches can be inspected without accidentally becoming public release evidence.

Implementation:

- Complete a strict mode that rejects score caches missing:
  - input data fingerprint
  - feature schema hash
  - model bundle hash
  - score generation version
  - policy version
- Keep legacy warning-only behavior in research mode.
- Show strict/research mode in `MlSignalWorkspacePanel`, history replay, and artifact summaries.

Files:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`

Verification:

- `cargo test -p ta_app_core --lib ml_signal`
- `cargo test -p ta_desktop_demo_app --lib commands_ml_signal`
- `npm --prefix web run test -- src/services/mlSignalService.test.ts`
- `powershell -File scripts/run_ml_signal_quality_gate.ps1`

### MLS-02: Policy Sweep Full-Vs-Retained Dual Verdict

Risk: Low.

Problem:
A policy can look good on retained trades while hiding what was discarded or what happened to the full candidate set.

Implementation:

- Extend `MlSignalPolicySweepResult`, `MlSignalValidationResult`, and candidate verdict rows so each sweep row reports:
  - full set metrics
  - retained set metrics
  - rejected set metrics
  - retained fraction
  - delta vs full set
  - rejection reason distribution
- Add a dual verdict:
  - retained quality verdict
  - full workflow verdict
- Preserve old summary fields for compatibility.

Files:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`

Verification:

- `cargo test -p ta_app_core --lib ml_signal`
- `cargo test -p ta_desktop_demo_app --lib commands_ml_signal`
- `npm --prefix web run test -- src/services/mlSignalService.test.ts`

### MLS-03: Model Bundle Trust Manifest

Risk: Medium.

Problem:
Model bundles already have a strict v2-style manifest path and the native command layer resolves bundle-local files. The remaining release-readiness work is to enforce the trust manifest consistently across scoring, native smoke, artifact history, and diagnostics.

Implementation:

- Tighten the model bundle manifest with:
  - model file hashes
  - native library hashes if used
  - feature schema hash
  - expected runtime version
  - provider/source label
  - created-at timestamp
- Validate hashes before scoring in strict mode.
- Add redacted diagnostics for mismatches.
- Require the native smoke gate whenever native library or bundle-loading behavior changes.

Files:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `src-tauri/src/redaction.rs`
- `scripts/train_ml_signal_catboost.py`
- `configs/ml_signal_catboost_default.json`
- `scripts/run_ml_signal_native_smoke.ps1`

Verification:

- `cargo test -p ta_app_core --lib ml_signal`
- `cargo test -p ta_desktop_demo_app --lib commands_ml_signal`
- `cargo test -p ta_desktop_demo_app --lib redaction`
- `powershell -File scripts/run_ml_signal_native_smoke.ps1 -CsvPath <dataset.csv> -BundleManifest <bundle\\manifest.json>`

### MLS-04: Calibration And Threshold Diagnostics

Risk: Low.

Problem:
Users need to understand whether a threshold is stable or just overfitted to one slice.

Implementation:

- Add calibration buckets for score ranges.
- Add threshold stability across time windows where source data supports it.
- Add warnings for tiny retained sample sizes.
- Persist diagnostics in run artifacts and expose them through history reload.
- Export diagnostics to report JSON/CSV when report surfaces support ML Signal fields.

Files:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/services/historyService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`

Verification:

- `cargo test -p ta_app_core --lib ml_signal`
- `cargo test -p ta_desktop_demo_app --lib commands_ml_signal`
- `npm --prefix web run test -- src/services/mlSignalService.test.ts`

### MLS-05: Browser Workflow Coverage

Risk: Low.

Problem:
ML Signal can regress at the workflow level even when services pass unit tests.

Implementation:

- Add E2E coverage for:
  - load/export dataset
  - select model bundle or fixture bundle
  - score
  - validate score cache
  - run policy sweep
  - inspect warnings
- Use a fixture model bundle that is safe for CI or mock the command boundary when native runtime is unavailable.
- If this becomes a new spec file, wire it into the same browser QA gate that currently runs workflow coverage.

Files:

- `web/e2e/browser/mlSignalWorkflow.spec.ts` (new, requires gate wiring)
- `web/e2e/browser/qa/workflowMatrix.spec.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/services/mlSignalService.ts`

Verification:

- `powershell -File scripts/run_frontend_e2e_gate.ps1`
- `powershell -File scripts/run_ml_signal_quality_gate.ps1`

### MLS-06: Quality Gate As Release Subgate

Risk: Low.

Problem:
The ML Signal quality gate exists, but public release readiness should state whether it passed, skipped, or is outside public scope.

Implementation:

- Add ML Signal gate status to release candidate summaries.
- If skipped, require a reason:
  - feature disabled
  - native runtime unavailable
  - research-only build
  - missing fixture bundle
- Keep public release docs aligned with actual behavior.
- Include the native smoke status separately from the pure Rust/web quality gate when a native bundle is packaged.

Files:

- `scripts/run_ml_signal_quality_gate.ps1`
- `scripts/run_ml_signal_native_smoke.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/run_public_release_gate.ps1`
- `docs/PUBLIC_RELEASE.md`
- `docs/RELEASE.md`

Verification:

- `powershell -File scripts/run_ml_signal_quality_gate.ps1`
- `powershell -File scripts/run_ml_signal_native_smoke.ps1 -CsvPath <dataset.csv> -BundleManifest <bundle\\manifest.json>`
- `powershell -File scripts/run_release_candidate_gate.ps1 -Variant cpu -SkipWorkspaceTests -SkipPackagedSmoke`

## Acceptance Criteria For This Guide

- Strict score-cache mode rejects missing data/model/feature/policy fingerprints while research mode may inspect legacy caches.
- Policy sweeps report full, retained, and rejected populations separately.
- Model bundle trust checks never print secrets or full local credential values in diagnostics.
- Native model/runtime work remains optional and does not break CPU-only builds.
- `scripts/run_ml_signal_quality_gate.ps1` remains the primary surface gate.

## Deferred Higher-Risk Work

- Automatically using ML Signal output in live execution.
- Adding new model architectures without leakage and reproducibility tests.
- Accepting unsigned or unmanifested native model bundles in strict mode.
