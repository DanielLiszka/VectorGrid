# Validation, GPU Residency, And Evidence Pipeline Implementation Plan

## Surface

Primary files:

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`

## Current Strengths

- Validation has walk-forward, reoptimization, CSCV/PBO, SPA, perturbation, bootstrap, Monte Carlo, cross-ticker, and route diagnostics.
- GPU validation route and CPU-tail metadata exist for important paths.
- Frontend already separates route summary, route detail, category, aux CPU, warnings, and validation verdict.
- Reports preserve many validation result fields.

## Improvement Plan

### VAL-01: Explicit Acceleration Status Field

Risk: Low.

Problem:
Users do not need a full residency breakdown in Simple Mode, but they do need a plain status such as "GPU accelerated with CPU tail" or "CPU fallback".

Implementation:

- Add frontend-derived `validationExecutionStatusLine` from existing result metadata.
- Render in Validation workspace and Simple validation result.
- Keep detailed route metadata in Advanced view.

Files:

- `web/src/services/validationService.ts`
- `web/src/stores/validationStore.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/GuidedWorkspacePanel.vue`

Verification:

- `npm --prefix web run test -- src/services/validationService.test.ts src/components/ValidationWorkspacePanel.test.ts src/components/GuidedWorkspacePanel.test.ts`

### VAL-02: Cross-Ticker Failure Accounting

Risk: Medium.

Problem:
Cross-ticker aggregation can under-emphasize failed datasets when only successful candidate validations enter accumulated metrics.
Current behavior is closer to fail-fast: dataset load and per-dataset validation errors abort through `?` before a partial `CrossTickerValidationSummary` can be emitted. The plan must decide whether fail-fast remains the product policy or whether partial accounting is desired.

Implementation:

- If fail-fast remains intended, make the fail-fast status explicit in UI, artifacts, and reports.
- If partial accounting is desired, wrap per-dataset load and validation in `match` rather than `?`.
- Add explicit counters for evaluated datasets, passed datasets, failed datasets, skipped datasets, and errored datasets.
- Add per-dataset status, error code, and error message fields to cross-ticker dataset metadata.
- Ensure failed cross datasets affect pass-rate style evidence and/or create warning flags.
- Preserve old output fields while adding new explicit fields.

Files:

- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_validation.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `web/src/services/validationService.ts`

Verification:

- `cargo test -p ta_desktop_demo_app --lib validation_common`
- `cargo test -p ta_desktop_demo_app --lib commands_validation`
- `npm --prefix web run test -- src/services/validationService.test.ts`

### VAL-03: SPA Cancellation Semantics

Risk: Medium.

Problem:
Long-running statistical submodules should not silently downgrade to missing evidence on cancellation.

Implementation:

- Propagate cancellation from SPA/bootstrap phases as a structured cancelled status or run-level warning.
- If the user explicitly cancels, report cancellation rather than completed validation.
- If only a module is skipped by cancellation while the run continues, label it clearly.

Files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `web/src/services/validationService.ts`

Verification:

- `cargo test -p ta_app_core validation_pipeline`
- `cargo test -p ta_app_core double_ma_validate`
- `npm --prefix web run test -- src/services/validationService.test.ts`

### VAL-04: Validation Report Route Metadata Expansion

Risk: Low.

Problem:
XLSX/JSON reports should preserve the same route and evidence clarity that the UI shows.

Implementation:

- Add a route/evidence table to validation XLSX with requested backend, backend used, route, fallback reason, residency tier, CPU-tail milliseconds, module summaries, and skipped/disabled modules.
- Backfill fields from saved artifacts when present.

Files:

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`

Verification:

- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- `node scripts/run_output_parity_check.mjs`

### VAL-05: Validation Request Capability Preview

Risk: Low.

Problem:
Users should know before running validation which modules will run on GPU, CPU, hybrid, or be skipped.

Implementation:

- Extend existing preview text into a structured preview model.
- Display expected route, module availability, dataset requirements, and estimated cost class.
- Reuse in Simple Mode optional validation.

Files:

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/GuidedWorkspacePanel.vue`

Verification:

- `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts`

### VAL-06: Strict GPU Validation Residency Policy

Risk: Medium.

Problem:
GPU validation metadata is useful, but strict runs need a clear pass/fail policy for whether the validation stage actually executed on the expected device path.
Do not enforce strictness by parsing display route strings; current route/residency derivation has string-derived pieces that are useful for UI but too brittle for policy enforcement.

Implementation:

- Add a validation residency policy field:
  - best effort
  - require GPU kernel
  - require GPU/hybrid kernel
  - allow CPU tail
  - CPU only
- Add typed observed residency/result fields before enforcement.
- Fail or warn when the observed route does not meet the requested policy.
- Include the policy and observed route in artifacts and reports.
- Keep Simple Mode on best-effort status text unless the user explicitly selects Advanced strict validation.

Files:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`

Verification:

- `cargo test -p ta_app_core double_ma_validate`
- `cargo test -p ta_app_core composite_validate`
- `cargo test -p ta_desktop_demo_app --lib commands_validation`
- `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts`

## Acceptance Criteria For This Guide

- Validation results distinguish requested route, observed route, fallback reason, and CPU-tail work.
- Simple Mode receives a short status line without losing Advanced route detail.
- Strict GPU residency policy is opt-in and cannot silently downgrade to CPU while reporting success.
- Cross-ticker failure/skip/error counts are preserved in UI, artifacts, and reports.
- Report exports include the same route/evidence fields used by the UI.

## Deferred Higher-Risk Work

- Full dual-verdict validation for every multi-stage validation family.
- GPU implementation of currently CPU-preferred auxiliary tails where benchmarks do not support it.
- Changing default hard gates or approval semantics.
