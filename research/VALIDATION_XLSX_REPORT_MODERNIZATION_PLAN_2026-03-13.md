# Validation XLSX Report Modernization Plan

Date: 2026-03-13
Status: Implementation in progress; XLSX modernization largely implemented, strategy validation coverage audit still open
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
- normal validation export still has legacy-compatibility concerns because saved artifacts may exist without the latest verdict fields
- `exportReportXlsx()` currently has a Stage 3 meta-report fallback path when no validation artifact exists, and that behavior must remain explicit while this validation-focused work is scoped separately

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

## Row-Cap and Data-Volume Policy

The workbook should distinguish between:

- visible presentation tables
- chart staging data
- raw audit export data

Recommended policy:

- row caps may apply to visible tables such as `Approved Candidates` and `Shortlist`
- `Distributions` should be computed from the full intended candidate population, not only the visible capped rows
- `Audit Data` should preserve the full exported evidence set unless an explicit export-size limit is introduced later
- if row caps are used, the workbook should state that visible rows are capped while aggregate charts may use the full underlying set

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
- legacy artifacts that predate `run_verdict` or candidate `validation_verdict` should either:
  - render under a clearly documented compatibility mode, or
  - fail with an explicit user-facing explanation

Compatibility policy must be chosen intentionally before implementation is marked complete.

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

## Validation Feature Support Audit

Date: 2026-03-13

This section tracks current validation-feature coverage across existing strategies and backend routes so XLSX modernization does not assume broader support than the runtime actually provides.

### Strategy Families and Validation Routes

Current validation command routing is intentionally split into two families:

- `DoubleMa`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct`, `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`
  - route through the double-MA validation path in `src-tauri/src/commands_validation.rs`
- `Asr`, `Vcc`, `Vemr`, `Rocb`, `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, `Cdmc`, `CompositeCpu`, `CompositeMo`
  - route through the composite validation path in `src-tauri/src/commands_validation.rs`

At the request-contract level, both families expose the same major validation modules:

- `walk_forward`
- `reopt_walk_forward`
- `spa`
- `perturb`
- `trade_bootstrap`
- `cross_ticker`
- `cscv_pbo`
- hard gates:
  - `min_psr`
  - `min_dsr`
  - `min_cscv_oos_rank_frac`
  - `min_cscv_oos_metric`
  - `max_pbo`

This means the broad feature model is shared. The real gaps are mostly runtime-route gaps, not request-schema gaps.

### Current Support Summary

Broadly supported across both validation families:

- candidate-level `validation_verdict`
- run-level `run_verdict`
- PSR / DSR computation
- minimum track-record-length checks
- robust score / pareto ranking
- optional walk-forward candidate stats
- optional reopt walk-forward summary
- optional SPA summary
- optional perturbation stats
- optional trade bootstrap stats
- optional CSCV / PBO
- optional cross-ticker validation pass

Important note about approval semantics:

- final candidate approval currently depends most directly on:
  - PSR / DSR gates
  - CSCV OOS rank / metric gates
  - minimum track-record-length checks
- run-level approval then adds:
  - PBO gate logic
  - SPA significance handling
- perturbation and trade bootstrap are implemented and exported, but they are not currently part of the final hard-gate / run-verdict approval rule

That appears to be current design, not necessarily a bug.

### Missing or Partial Support by Route

#### 1. `DoubleMa`-family fixed-candidate GPU validation is not feature-complete

Affected strategies:

- `DoubleMa`
- `Pit`
- `Set`
- `Mts`
- `Orc`
- `Marrt`
- `Vfct`
- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`

Current limitations:

- does not support `trade_bootstrap`
- does not support `cross_ticker`
- requires the vectorized validation engine
- requires uniform MA types / strategy config / halftrend filter / MA params across the fixed-candidate batch

Why this appears intentional:

- the blockers are explicitly encoded in `crates/app_core/src/double_ma_validate.rs`
- the error strings clearly describe current runtime constraints rather than accidental omissions

Assessment:

- intentional current limitation
- still a product gap if the goal is full feature parity across CPU and GPU validation routes

#### 2. Composite fixed-candidate GPU validation is only partially feature-complete

Affected strategies:

- `Asr`
- `Vcc`
- `Vemr`
- `Rocb`
- `Mkcs`
- `Kqsr`
- `Carm`
- `Kvdf`
- `Cdmc`
- `CompositeCpu`
- `CompositeMo`

Current limitations:

- does not support `cross_ticker`
- conditionally blocks some modules by candidate/profile family:
  - `walk_forward`
  - `reopt_walk_forward`
  - `spa`
  - `trade_bootstrap`
  - `cscv_pbo`
- requires the vectorized validation engine

Why this appears intentional:

- the blockers are explicitly encoded in `crates/app_core/src/composite_validate.rs`
- the code checks profile support before allowing those modules on the fixed-candidate GPU route

Assessment:

- intentional current limitation
- still a parity gap versus CPU validation

#### 3. Cross-ticker reruns intentionally disable the rest of the optional validation stack

Affected strategies:

- all strategies that use cross-ticker through `commands_validation.rs`

Current behavior:

- cross-ticker rerun requests are rebuilt with:
  - `walk_forward: None`
  - `reopt_walk_forward: None`
  - `spa: None`
  - `perturb: None`
  - `trade_bootstrap: None`
  - `cscv_pbo: None`
  - hard gates cleared
- this applies to both composite and double-MA cross-ticker rerun builders

Why this appears intentional:

- the command layer constructs lightweight cross-ticker follow-up requests on purpose
- likely rationale:
  - keep cross-ticker runtime bounded
  - avoid recursively nesting expensive validation modules on every extra dataset
  - treat cross-ticker as a separate secondary robustness filter rather than a full revalidation pass

Assessment:

- likely intentional
- important product decision that should be documented explicitly because users can reasonably expect "cross-ticker validation" to preserve more of the original stack

#### 4. Headless validation artifact context is incomplete

Affected path:

- `src-tauri/src/bin/ta_headless_validate.rs`

Current behavior:

- headless validation currently builds requests with `validation_context: None`

Observed consequence:

- exported workbooks can show incomplete context such as `Strategy = Unknown`

Assessment:

- likely an unintentional gap
- should be fixed so headless-generated artifacts carry the same strategy/context metadata as UI-generated artifacts

#### 5. Validation capability surfacing is weaker than backend reality

Affected surface:

- validation UI / request-building / capability messaging

Current behavior:

- the request model is broadly shared across strategies
- route-specific runtime blockers are mostly backend-only knowledge
- users can request a broad validation stack without being clearly warned that a given strategy/backend route will skip, block, or downshift some modules

Assessment:

- likely a product/UX gap rather than a core backend bug
- should be addressed so users can tell in advance which validation modules will actually execute

### Current Cross-Ticker Test Data Already Available

Repo-local datasets already present under `data/` and suitable for cross-ticker validation testing include:

- `AAPL.csv`
- `NVDA.csv`
- `QQQ.csv`
- `SPY.csv`
- `SQQQ.csv`
- `TQQQ.csv`
- `TSLA.csv`
- `UVXY.csv`

These should be preferred for support-validation smoke runs before adding any new fixture data.

## Validation Feature Broadening Checklist

This checklist is separate from the XLSX workbook checklist. Its purpose is to broaden actual validation feature support across existing strategies and routes, except where current limitations remain intentional by product decision.

### Documentation and policy

- [ ] Document the current strategy-family split between double-MA validation and composite validation in product-facing or engineering-facing docs.
- [ ] Document which validation modules are optional evidence surfaces versus true approval gates.
- [ ] Document the current meaning of cross-ticker validation:
  - secondary lightweight rerun filter
  - or full-stack revalidation across additional datasets
- [ ] Decide explicitly whether perturbation and trade bootstrap should remain non-gating or become optional gate inputs later.

### Route-level parity audit

- [ ] Build a strategy-by-strategy parity matrix covering CPU, Auto, GpuKernel, and GpuHybridKernel validation routes.
- [ ] Mark each strategy/route pair for support status of:
  - walk-forward
  - reopt walk-forward
  - SPA
  - perturb
  - trade bootstrap
  - cross-ticker
  - CSCV/PBO
  - PSR/DSR gates
  - min-track-record checks
  - run verdict
- [ ] Mark each unsupported cell as one of:
  - intentional
  - deferred
  - unknown / needs investigation

### Double-MA family gaps

- [ ] Treat the double-MA family as the validation golden standard for the application.
- [ ] Define the parity target explicitly:
  - the same validation-stage feature set should be available for double-MA-family runs on CPU and GPU validation routes
  - CPU and GPU runs may differ internally in execution strategy, but not in user-visible feature availability or final semantics
- [ ] Freeze the double-MA parity scope across:
  - `DoubleMa`
  - `Pit`
  - `Set`
  - `Mts`
  - `Orc`
  - `Marrt`
  - `Vfct`
  - `CompositeTrend`
  - `RegimeHybrid`
  - `BreakoutMomentum`
- [ ] Build a double-MA-family CPU vs GPU validation parity matrix covering:
  - walk-forward
  - reopt walk-forward
  - SPA
  - perturb
  - trade bootstrap
  - cross-ticker
  - CSCV/PBO
  - PSR / DSR gates
  - min-track-record checks
  - candidate `validation_verdict`
  - run `run_verdict`
- [ ] Record, for each double-MA-family route, whether the current limitation is:
  - intentional short-term
  - accidental / incomplete
  - blocked by CUDA/runtime architecture
- [ ] Keep final hard-gate and run-verdict semantics identical across CPU and GPU validation routes, even if the expensive reruns are GPU-executed and the cheap reductions remain CPU-side.
- [ ] Ensure the XLSX/report output for double-MA-family runs is semantically identical across CPU and GPU validation routes:
  - same sections
  - same feature visibility
  - same “not run” behavior
  - same approval/rejection reasoning
- [ ] Add at least one broad-stack double-MA CPU reference artifact that exercises the full intended validation surface.
- [ ] Add at least one broad-stack double-MA GPU reference artifact that exercises the same validation surface with parity expectations.
- [ ] Compare CPU and GPU reference artifacts for:
  - candidate ordering
  - approval counts
  - run recommendation
  - rejection reasons
  - surfaced validation modules
- [ ] Document any residual CPU-only fallback behavior that still exists while parity work is in progress.
- [ ] Decide whether fixed-candidate GPU validation for the double-MA family should gain `trade_bootstrap` support.
- [ ] Decide whether fixed-candidate GPU validation for the double-MA family should gain `cross_ticker` support.
- [ ] If full GPU parity is not planned, document forced CPU fallback behavior clearly in UX/status text.
- [ ] Add GPU parity for `reopt_walk_forward` if any route still falls back or degrades there.
- [ ] Add GPU parity for `SPA` if any route still depends on a CPU-only prep path.
- [ ] Add GPU parity for `perturb` if the current path still requires CPU-side candidate reruns.
- [ ] Add GPU parity for `trade_bootstrap` or explicitly decide that CPU fallback remains acceptable for that module.
- [ ] Decide whether double-MA-family `cross_ticker` should remain a lightweight secondary filter or become a full-stack parity-preserving revalidation pass.
- [ ] Review whether the uniform-candidate constraints on MA types / params / strategy config are acceptable long-term or should be relaxed.
- [ ] Add explicit tests proving fallback behavior when unsupported modules are requested on GPU validation routes.

### Composite-family gaps

- [ ] Build a profile-by-profile support table for:
  - `Asr`
  - `Vcc`
  - `Vemr`
  - `Rocb`
  - `Mkcs`
  - `Kqsr`
  - `Carm`
  - `Kvdf`
  - `Cdmc`
  - `CompositeCpu`
  - `CompositeMo`
- [ ] Decide whether composite fixed-candidate GPU validation should gain `cross_ticker` support.
- [ ] Decide whether currently profile-conditional support for walk-forward, reopt walk-forward, SPA, trade bootstrap, and CSCV/PBO should be broadened.
- [ ] If some profiles will remain CPU-preferred for validation, document that explicitly rather than treating it as implicit backend behavior.

### Cross-ticker broadening

- [ ] Decide whether cross-ticker reruns should preserve any of the original optional validation modules:
  - walk-forward
  - reopt walk-forward
  - SPA
  - perturb
  - trade bootstrap
  - CSCV/PBO
- [ ] If full preservation is too expensive, define an intentional reduced cross-ticker mode and expose that policy in the UI/XLSX.
- [ ] Add smoke tests using existing repo-local datasets such as:
  - `AAPL.csv`
  - `NVDA.csv`
  - `SPY.csv`
  - `QQQ.csv`
- [ ] Verify that cross-ticker filtering semantics are clearly represented in the final workbook for both approved and rejected candidates.

### Context propagation and artifact quality

- [x] Populate `validation_context` in headless validation flows so exported artifacts preserve strategy identity and other context.
- [ ] Audit other artifact-generation paths for missing validation context or strategy metadata.
- [ ] Add regression tests ensuring headless-generated XLSX artifacts do not show `Strategy = Unknown` for known strategies.

### Frontend capability surfacing

- [ ] Add validation-capability summaries to the UI comparable to existing optimization capability messaging.
- [ ] Warn users when a requested validation module will not run on the selected strategy/backend route.
- [ ] Surface when the backend falls back from requested GPU validation to CPU validation.
- [ ] Surface when a validation module was skipped intentionally versus disabled by the user.

### Verification

- [ ] Add end-to-end validation support tests for at least one strategy in each family using the broadest supported stack.
- [ ] Add route-specific tests for unsupported-module behavior on GPU validation paths.
- [ ] Add cross-ticker tests that use multiple repo-local datasets from `data/`.
- [ ] Extend XLSX verification so workbook dashboards clearly indicate which validation layers actually ran versus which were unavailable or disabled.

## Frontend and Workflow Requirements

Frontend behavior should support:

- manual `Export report XLSX`
- working `Auto-export report XLSX after validation`
- persisted `report_xlsx_path` in run artifacts/history
- graceful open/reveal behavior after save
- explicit preserved behavior for the current Stage 3 meta-report fallback when no validation result exists
- clear status messages for:
  - export succeeded
  - export skipped because no validation exists
  - export generated a report with zero approved candidates

## Headless and Automation Requirements

The headless validate-to-XLSX path must remain supported.

Requirements:

- headless and UI-triggered workbooks should use the same core builder path
- `save_report_xlsx` and `build_report_xlsx` fallback output should remain semantically aligned
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

Current preview artifact for manual inspection:

- `research/validation_report_preview.xlsx`

## Verification Evidence

Verification completed on 2026-03-13:

- Rust integration test target `src-tauri/tests/validation_xlsx_report_tests.rs` passes and covers:
  - expected sheet set
  - active Dashboard tab
  - chart presence
  - run-level rejection overriding candidate-level approval
  - legacy compatibility mode
  - strategy-family coverage across `ASR`, `VCC`, `VEMR`, `ROCB`, `MKCS`, `KQSR`, `CARM`, `KVDF`, and `CDMC`
- frontend workflow test target `web/src/services/validationService.test.ts` passes and covers:
  - auto-export after validation
  - persisted `report_xlsx_path`
  - non-fatal auto-export failure handling
  - preserved Stage 3 meta-report fallback
- generated preview workbook inspection confirmed:
  - workbook opens to `Dashboard`
  - workbook contains `Approved Candidates`, `Shortlist`, `Distributions`, `Audit Data`, `Dashboard`
  - workbook contains 7 chart parts across `Dashboard` and `Distributions`
  - dashboard/shared strings include run verdict, CSCV/PBO, SPA, reopt walk-forward, cross-ticker, row-cap notes, and fallback notes
- follow-up readability pass implemented on 2026-03-13:
  - candidate sheets and dashboard now render friendlier approval labels
  - distributions now surface approval-state counts plus top non-approval reasons for mixed approved/rejected runs
  - shortlist note now states the ordering rule: approved -> hard-gate survivors -> remaining context
  - dashboard `Top Visible Rows` now has explicit column headers and fills from approved rows first, then shortlist/context rows
  - preview/example source and Rust test expectations were updated for those surfaces
- refreshed execution verification completed on 2026-03-13:
  - `cargo test -q -p ta_desktop_demo_app --test validation_xlsx_report_tests` passes after the readability pass
  - `cargo run -q -p ta_desktop_demo_app --example generate_validation_report_preview` regenerated `research/validation_report_preview.xlsx`
  - real gated `DoubleMa` ALMA optimize -> validate -> XLSX flow reran against `data/AAPL.csv`
  - regenerated `_write_test/headless_reports/aapl_alma_validation_gated.xlsx` opens to `Dashboard`, contains 7 chart parts, and exposes gate-config, approval-state, top-reason, and dashboard-header surfaces in workbook XML

## Verification Checklist

Shared backend evidence:

- [x] report builder uses final validation approval semantics
- [x] zero-approved-candidate runs produce a valid workbook
- [x] all implemented strategies can export without MA-only assumptions
- [x] headless and UI-triggered exports share the intended builder path
- [x] compatibility behavior for legacy validation artifacts is explicit and tested
- [x] `save_report_xlsx` and `build_report_xlsx` fallback paths remain semantically aligned

Shared frontend evidence:

- [x] manual export works from the validation workspace
- [x] auto-export after validation works when enabled
- [x] `report_xlsx_path` persists for normal validation artifacts
- [x] history/open/reveal flows work for validation-generated XLSX files
- [x] the existing Stage 3 meta-report fallback behavior remains explicit and tested

Shared workbook evidence:

- [x] expected sheet set exists
- [x] expected charts/drawings exist
- [x] dashboard communicates run verdict and recommendation immediately
- [x] optional validation layers render as "not run" rather than zero

Shared regression evidence:

- [x] Rust tests cover workbook structure and selection semantics
- [x] frontend tests cover workflow wiring
- [x] sample artifacts refreshed
- [x] parity checks updated
- [ ] telemetry checks updated

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
11. Compatibility behavior for older validation artifacts is intentional and documented.
12. The current no-validation Stage 3 fallback behavior is either preserved explicitly or deliberately replaced with documented rationale.

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
- choose and implement the compatibility policy for older validation artifacts
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
- keep `build_report_xlsx` fallback and `save_report_xlsx` behavior aligned
- preserve or deliberately replace the current Stage 3 meta-report fallback behavior
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
- [x] Confirm the target workbook architecture and keep it stable during implementation.
- [x] Confirm the final XLSX candidate-selection rule with current validation semantics.
- [x] Confirm the compatibility policy for older validation artifacts.
- [x] Confirm whether the current Stage 3 meta-report fallback remains unchanged.

### Backend: report selection semantics

- [x] Inventory all current validation result fields that the workbook should consume.
- [x] Stop treating `robust_pareto` and related shortlist heuristics as the sole final selection rule.
- [x] Surface run-level approval semantics in the builder.
- [x] Surface candidate-level approval semantics in the builder.
- [x] Add explicit handling for zero-approved-candidate runs.
- [x] Add explicit compatibility handling for artifacts that lack `run_verdict` and/or candidate `validation_verdict`.

### Backend: strategy coverage

- [x] Replace MA-only candidate-summary assumptions with strategy-agnostic summaries.
- [x] Keep strategy-local parameter formatting where required.
- [x] Verify all implemented profile families can produce a workbook without missing required columns.

### Backend: workbook redesign

- [x] Implement `Dashboard` sheet.
- [x] Implement `Approved Candidates` sheet.
- [x] Rework `Shortlist` sheet.
- [x] Implement `Distributions` sheet.
- [x] Implement `Audit Data` sheet.
- [x] Add staged chart-data ranges.
- [x] Apply a reusable semantic color system.
- [x] Surface approval-state and rejection-reason distributions for mixed approved/rejected runs.
- [x] Add explicit dashboard headers for the top visible-row summary table.
- [x] Make row-cap behavior explicit for visible tables versus full-set distributions/audit data.
- [ ] Review whether the legacy fallback builder should remain.

### Frontend and workflow

- [x] Wire `Auto-export report XLSX after validation` into the actual validation flow.
- [x] Preserve `report_xlsx_path` for normal validation artifacts.
- [x] Verify history/open/reveal flows for validation-generated XLSX files.
- [x] Preserve or deliberately replace the current Stage 3 meta-report fallback behavior in `exportReportXlsx()`.
- [x] Ensure status text is accurate for success, failure, and no-validation cases.

### Headless and automation

- [x] Keep headless validate-to-XLSX working through the same core builder path.
- [x] Keep `build_report_xlsx` fallback output semantically aligned with saved-file output.
- [x] Verify row-cap behavior for headless and headless-preview flows.
- [ ] Verify deterministic output-path behavior.

### Tests

- [x] Add Rust tests for workbook sheet names and core parts.
- [x] Add Rust tests for selection semantics.
- [x] Add Rust tests for zero-approved-candidate scenarios.
- [x] Add Rust tests for at least one MA-family and one non-MA-family strategy.
- [x] Add frontend tests for auto-export behavior.
- [x] Add frontend tests for artifact persistence/open flows.
- [x] Strengthen artifact/parity checks beyond basic ZIP-shape validation.

### Samples and docs

- [x] Refresh sample XLSX artifacts after workbook redesign stabilizes.
- [ ] Update any telemetry/parity manifests that track XLSX outputs.
- [x] Update architecture/status docs to match the new XLSX state.

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

Real gated validation artifact used for mixed-run sanity checks:

- `_write_test/headless_reports/aapl_alma_validation_gated.json`
- run recommendation: `single_candidate_supported`
- approved candidates: 2
- rejected candidates: 6
- dominant rejection reason: `dsr_below_min:0.30`
- double-MA headless report now emits:
  - `run.req.strategy_kind = DoubleMa`
  - `validation.req.strategy_kind = DoubleMa`
  - `validation.req.validation_context`
  - hard-gate request fields such as `min_dsr`

Reference inspiration only:

- external `SERENITY` workbook design language
- external Python `__init__.py` formatter

Key design decision to preserve from that inspiration:

- "open once, understand instantly"
- mixed validation runs must show both approved candidates and the dominant rejection reasons immediately

Key implementation decision not to preserve:

- giant single-sheet, hard-coded coordinate logic tied to one dataframe shape
