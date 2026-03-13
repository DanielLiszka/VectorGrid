# Validation XLSX Report Modernization Plan

Date: 2026-03-13
Status: Planning artifact; implementation not started
Scope: `ta_desktop_demo` validation-stage XLSX output for all implemented strategies

Purpose: define a purpose-built plan to bring XLSX analysis/output up to the current validation model, replace outdated shortlist logic, and provide a progress checklist that can be updated during implementation.

This document is intentionally about the Rust/Tauri app only. The external Python `SERENITY` workbook and legacy `__init__.py` are inspiration for information design, not implementation dependencies.

## Goal

Produce an XLSX report that communicates validation outcomes instantly and accurately for all implemented strategies.

The target is not "export a spreadsheet". The target is:

- reflect the actual end-of-validation approval semantics
- summarize the best evidence without hiding raw detail
- surface current validation-stage signals across all supported strategies
- preserve CPU fallback behavior and current artifact persistence flows
- make the report useful both interactively and in headless/automated runs

## Why This Work Is Needed

Current XLSX export exists, but it is behind the current state of validation.

Current issues confirmed in source:

- report filtering still keys primarily off `robust_pareto`, `cross_ticker_pareto`, and optional CSCV thresholds
- report builders do not currently use the newer final approval semantics such as candidate-level `validation_verdict` and run-level `run_verdict`
- the validation UI exposes XLSX export, but the "auto-export after validation" toggle appears not to be wired into `runValidation()`
- current workbook structure is still largely generic and MA-shaped rather than truly aligned to the current strategy-agnostic validation model
- XLSX test coverage is smoke-level only and does not validate workbook semantics deeply

## Product Direction

The right direction is:

- keep the "instant-read" clarity of the old `SERENITY` workbook
- do not copy the old giant hard-coded single-sheet implementation model
- use the current Rust builder as the technical foundation
- redesign the workbook around current validation semantics and strategy-agnostic result fields

The report should answer these questions immediately:

1. Did the run approve any candidate?
2. What is the final recommendation?
3. How many candidates were approved and why?
4. Which candidate is currently best and what are its parameters?
5. How did it perform across walk-forward, CSCV/PBO, PSR/DSR, cross-ticker, bootstrap, perturbation, and any other active validation layers?
6. What does the broader shortlist look like?
7. What raw evidence is available for deeper inspection?

## Current Code Surfaces

Primary backend export surfaces:

- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_shared.rs`

Primary validation/result surfaces:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/run_artifacts.rs`

Primary frontend surfaces:

- `web/src/services/validationService.ts`
- `web/src/stores/validationStore.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

Related but separate XLSX surface:

- `src-tauri/src/commands_reports_meta_xlsx.rs`

## Scope Rules

In scope:

- validation-stage report XLSX for all currently implemented strategies
- manual export from the validation workspace
- optional auto-export after validation
- headless validate-to-XLSX flow
- workbook semantics, color rules, chart set, and test coverage

Out of scope for this document:

- rewriting meta-analysis XLSX output
- importing code from the external Python/Jesse project
- optimization-stage XLSX export unless it is needed for shared report primitives

## Report Design Principles

1. The first sheet must communicate the run verdict immediately.
2. Candidate selection must reflect current backend approval semantics, not older shortlist heuristics alone.
3. The workbook must remain readable for non-MA strategies.
4. Strategy-specific parameter fields should be shown without making the workbook strategy-specific.
5. Color must encode meaning consistently.
6. The workbook must separate "summary", "ranked shortlist", "distribution view", and "raw audit data".
7. Hidden or secondary staging sheets are acceptable if they make charts and formulas cleaner.

## Target Workbook Architecture

### Sheet 1: `Dashboard`

Purpose: the "open once, understand instantly" page.

Must include:

- run timestamp
- strategy kind
- dataset identity
- validation backend used / route metadata when available
- run-level `run_verdict`
- `final_recommendation`
- `approved_candidate_count`
- first approved rank
- explicit "no candidate approved" state when applicable
- best approved candidate summary
- compact gate summary cards for:
  - walk-forward
  - CSCV/PBO
  - PSR
  - DSR
  - cross-ticker
  - perturbation
  - trade bootstrap
- compact top-approved table with color-coded pass/fail indicators
- charts for the most decision-relevant evidence

### Sheet 2: `Approved Candidates`

Purpose: ranked table of final approved candidates only.

Must include:

- rank
- approval/pass flag
- candidate verdict summary
- parameter summary
- in-sample metrics
- walk-forward metrics
- CSCV/PBO metrics
- PSR / DSR
- cross-ticker metrics
- bootstrap / perturb metrics when present
- route/backend metadata when relevant

Behavior:

- if zero candidates are approved, the sheet should still exist and clearly state that no candidates passed final approval

### Sheet 3: `Shortlist`

Purpose: broader ranked candidate table for context and comparison.

Should include:

- all reportable validation candidates after current shortlist policy is applied
- explicit columns showing why a candidate was not finally approved
- stable sort order and row cap controls

### Sheet 4: `Distributions`

Purpose: chart-oriented and aggregate evidence.

Candidate content:

- distribution of approved vs non-approved candidates
- robust score distribution
- walk-forward Sharpe / drawdown distributions
- CSCV OOS metric distribution
- PSR / DSR distribution
- per-strategy parameter-shape charts where meaningful
- cross-ticker score distribution when present

### Sheet 5: `Audit Data`

Purpose: raw and machine-friendly evidence.

Should include:

- one row per candidate
- stable columns intended for downstream analysis or copy/paste
- exact numeric values used by dashboard and charts

This sheet may be hidden by default if that improves workbook UX.

## Candidate Selection Policy for the XLSX

The report should distinguish clearly between:

- approved candidates
- shortlist candidates
- all validation candidates

Recommended policy:

1. `Dashboard` and `Approved Candidates` should key off the final backend approval semantics.
2. `Shortlist` should show the broader ranked context, including near-miss candidates.
3. `Audit Data` should preserve the full exportable evidence set.

Important rule:

- older shortlist heuristics such as `robust_pareto` and `cross_ticker_pareto` can still be useful as explanatory columns, but they should not remain the sole definition of what the report considers "final".

## Validation Data Contract to Surface

The report should surface, when available:

- run-level verdict fields
- candidate-level verdict fields
- final recommendation
- approved candidate count
- first approved rank
- strategy kind
- backend used / backend route / fallback reason
- parameter payload or normalized parameter summary
- in-sample metrics
- walk-forward metrics
- CSCV/PBO metrics
- PSR / DSR
- cross-ticker metrics
- perturbation metrics
- trade bootstrap metrics
- local stability / related hard-gate evidence when present
- any strategy-family-specific fields already carried in validation artifacts

Missing data policy:

- absent validation layers should render as "not run" rather than zero
- charts should omit unavailable series cleanly
- workbook generation must not fail merely because an optional validation layer was disabled

## Color and Visual System

Color must be semantic and reusable, not ad hoc.

Recommended baseline:

- green: approved / good / above threshold
- red: rejected / failed / below threshold
- amber: caution / borderline / advisory warning
- blue: contextual or descriptive metadata
- neutral gray: not run / not applicable

Formatting policy:

- prefer reusable formats and conditional formatting rules over hundreds of one-off style variants
- use icons/badges sparingly and consistently
- keep chart palette aligned with the table semantics

## Chart Set

Minimum recommended chart set for the first implementation pass:

- approved candidate score by rank
- risk/return scatter for finalists
- walk-forward vs in-sample comparison
- CSCV or PBO summary view
- parameter-shape distribution relevant to the strategy family
- approved vs rejected counts by reason category if feasible

Charts should be driven from clean staging data, not directly from the visible formatted tables.

## Strategy Coverage Requirements

The workbook must support all currently implemented strategies without assuming MA-only parameters.

Implementation guidance:

- keep shared workbook/report plumbing shared
- keep strategy-specific parameter-summary extraction strategy-local where necessary
- avoid hard-coding only `fast_ma_type`, `fast_len`, `slow_ma_type`, `slow_len` as the primary candidate identity

## Frontend and Workflow Requirements

Frontend behavior should support:

- manual `Export report XLSX`
- working `Auto-export report XLSX after validation`
- persisted `report_xlsx_path` in run artifacts/history
- graceful open/reveal behavior after save
- clear status messages for:
  - export succeeded
  - export skipped because no validation exists
  - export generated a report with zero approved candidates

## Headless and Automation Requirements

The headless validate-to-XLSX path must remain supported.

Requirements:

- headless and UI-triggered workbooks should use the same core builder path
- row-cap behavior should be explicit and testable
- output path logic should remain deterministic and sanitized

## Testing and Verification Requirements

Rust/backend:

- add targeted tests for candidate selection semantics used by XLSX
- add workbook-structure assertions beyond "bytes start with PK"
- verify sheet names and expected chart/drawing parts
- add tests for zero-approved-candidate runs
- add tests for multiple strategy families

Frontend:

- verify auto-export is actually invoked when enabled
- verify artifact path persistence after XLSX save
- verify UI status lines for success/failure/no-validation cases

Artifact/parity:

- update or replace shallow XLSX parity checks with workbook-semantic checks
- refresh sample artifacts after the new workbook shape is stable

## Verification Checklist

Shared backend evidence:

- [ ] report builder uses final validation approval semantics
- [ ] zero-approved-candidate runs produce a valid workbook
- [ ] all implemented strategies can export without MA-only assumptions
- [ ] headless and UI-triggered exports share the intended builder path

Shared frontend evidence:

- [ ] manual export works from the validation workspace
- [ ] auto-export after validation works when enabled
- [ ] `report_xlsx_path` persists for normal validation artifacts
- [ ] history/open/reveal flows work for validation-generated XLSX files

Shared workbook evidence:

- [ ] expected sheet set exists
- [ ] expected charts/drawings exist
- [ ] dashboard communicates run verdict and recommendation immediately
- [ ] optional validation layers render as "not run" rather than zero

Shared regression evidence:

- [ ] Rust tests cover workbook structure and selection semantics
- [ ] frontend tests cover workflow wiring
- [ ] sample artifacts refreshed
- [ ] parity/telemetry checks updated

## Acceptance Criteria

The work is complete only when all of the following are true:

1. XLSX export reflects final validation approval semantics rather than older shortlist-only heuristics.
2. The workbook includes a true dashboard-grade first sheet.
3. The workbook supports all implemented strategies without MA-only assumptions.
4. Manual export works from the validation workspace.
5. Auto-export after validation works when enabled.
6. Headless validate-to-XLSX still works.
7. Run artifact/history persistence includes the XLSX path for normal validation exports.
8. Rust tests validate workbook structure and report-selection semantics.
9. Frontend tests cover auto-export wiring and artifact persistence.
10. Sample/baseline artifacts and tracker docs are updated.

## Proposed Implementation Phases

## Phase 0: Planning and Contract Freeze

- confirm target workbook architecture
- define final candidate-selection semantics for XLSX
- define the minimum strategy-agnostic parameter summary contract
- define missing-data rendering policy

## Phase 1: Backend Data Contract Alignment

- audit current validation result fields used by the XLSX builder
- align report builder inputs with final approval semantics
- add helpers to normalize candidate summaries across strategies
- keep legacy fallback behavior only if it is still justified

## Phase 2: Workbook Redesign

- implement `Dashboard`
- implement `Approved Candidates`
- rework `Shortlist`
- add `Distributions`
- add `Audit Data`
- move charts onto staged data ranges

## Phase 3: Workflow Wiring

- wire auto-export after validation
- ensure normal validation exports persist `report_xlsx_path`
- verify reveal/open/history flows
- align headless export with the same builder path

## Phase 4: Tests and Sample Artifacts

- add Rust workbook-structure tests
- add selection-policy tests
- add frontend workflow tests
- refresh sample artifacts
- strengthen parity/telemetry checks

## Implementation Checklist

### Planning

- [x] Create this purpose-built planning document.
- [ ] Confirm the target workbook architecture and keep it stable during implementation.
- [ ] Confirm the final XLSX candidate-selection rule with current validation semantics.

### Backend: report selection semantics

- [ ] Inventory all current validation result fields that the workbook should consume.
- [ ] Stop treating `robust_pareto` and related shortlist heuristics as the sole final selection rule.
- [ ] Surface run-level approval semantics in the builder.
- [ ] Surface candidate-level approval semantics in the builder.
- [ ] Add explicit handling for zero-approved-candidate runs.

### Backend: strategy coverage

- [ ] Replace MA-only candidate-summary assumptions with strategy-agnostic summaries.
- [ ] Keep strategy-local parameter formatting where required.
- [ ] Verify all implemented strategies can produce a workbook without missing required columns.

### Backend: workbook redesign

- [ ] Implement `Dashboard` sheet.
- [ ] Implement `Approved Candidates` sheet.
- [ ] Rework `Shortlist` sheet.
- [ ] Implement `Distributions` sheet.
- [ ] Implement `Audit Data` sheet.
- [ ] Add staged chart-data ranges.
- [ ] Apply a reusable semantic color system.
- [ ] Review whether the legacy fallback builder should remain.

### Frontend and workflow

- [ ] Wire `Auto-export report XLSX after validation` into the actual validation flow.
- [ ] Preserve `report_xlsx_path` for normal validation artifacts.
- [ ] Verify history/open/reveal flows for validation-generated XLSX files.
- [ ] Ensure status text is accurate for success, failure, and no-validation cases.

### Headless and automation

- [ ] Keep headless validate-to-XLSX working through the same core builder path.
- [ ] Verify row-cap behavior for headless and UI flows.
- [ ] Verify deterministic output-path behavior.

### Tests

- [ ] Add Rust tests for workbook sheet names and core parts.
- [ ] Add Rust tests for selection semantics.
- [ ] Add Rust tests for zero-approved-candidate scenarios.
- [ ] Add Rust tests for at least one MA-family and one non-MA-family strategy.
- [ ] Add frontend tests for auto-export behavior.
- [ ] Add frontend tests for artifact persistence/open flows.
- [ ] Strengthen artifact/parity checks beyond basic ZIP-shape validation.

### Samples and docs

- [ ] Refresh sample XLSX artifacts after workbook redesign stabilizes.
- [ ] Update any telemetry/parity manifests that track XLSX outputs.
- [ ] Update architecture/status docs to match the new XLSX state.

## Risks and Guardrails

Risks:

- the workbook regresses into MA-only assumptions and breaks non-MA strategies
- the builder confuses shortlist heuristics with final approval semantics
- optional validation layers cause sparse-data layout failures
- UI and headless flows diverge into separate workbook behaviors
- chart/layout ambition outpaces test coverage and becomes fragile

Guardrails:

- keep candidate-summary normalization explicit and strategy-aware
- separate approved-candidate logic from broader shortlist logic
- treat missing optional validation layers as first-class expected cases
- keep chart data staged and testable
- prefer reusable formats and conditional formatting over brittle one-off styling

## Immediate Execution Order

1. Freeze the XLSX selection semantics around current run-level and candidate-level validation approval fields.
2. Define the strategy-agnostic candidate summary contract and missing-data policy.
3. Rework the backend builder around the target workbook architecture.
4. Wire auto-export and artifact persistence cleanly through the normal validation flow.
5. Add workbook-structure and workflow tests before refreshing sample artifacts.

## Working Notes

Reference inspiration only:

- external `SERENITY` workbook design language
- external Python `__init__.py` formatter

Key design decision to preserve from that inspiration:

- "open once, understand instantly"

Key implementation decision not to preserve:

- giant single-sheet, hard-coded coordinate logic tied to one dataframe shape
