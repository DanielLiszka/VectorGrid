# Review Packet: `src-tauri/src/commands_reports_validation_xlsx.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_validation_xlsx.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally strong workbook renderer, but still carrying a few contract-truthfulness issues that should be fixed before treating it as a polished canonical export surface`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Workbook-contract-sensitive`, `Semantics-sensitive`, `Renderer-vocabulary-sensitive`, `Support-module`

## What This File Owns

`commands_reports_validation_xlsx.rs` owns the validation XLSX workbook renderer.

This file owns:

- the top-level workbook builder:
  - `build_validation_report_xlsx`: `src-tauri/src/commands_reports_validation_xlsx.rs:2293-2424`
- workbook formatting helpers and display mappings:
  - optional-cell writers: `61-95`
  - display-label helpers: `97-212`
  - format factory: `2217-2289`
- candidate-sheet layout and row rendering:
  - sheet layout: `299-352`
  - header writing: `354-414`
  - row writing: `416-657`
  - generic candidate sheet builder: `659-770`
- specialized sheets:
  - `Regime Audit`: `772-872`
  - `Dashboard`: `874-1974`
  - `Distributions`: `1976-2215`
- workbook-level sheet naming and note text for:
  - `Dashboard`
  - `Approved Candidates`
  - `Shortlist`
  - `Audit Data`
  - `Distributions`
  - `Regime Audit`
- direct unit tests in the module:
  - `2468-2635`

This file does not own:

- the export model or approval/shortlist policy
  - those come from `src-tauri/src/commands_reports_validation_common.rs`
- Tauri command exposure
  - handled by `src-tauri/src/commands_reports_api.rs`

## Classification

- Public surface: no
  - internal workbook renderer behind the report-command layer
- Hot path: no
  - export-time workbook generation only
- Runtime-critical: moderate to high
  - this file defines the user-visible Excel report contract
- CUDA/runtime-critical: no
- Strategy-local: no
  - cross-strategy validation report renderer
- Low-risk support code: no
  - output correctness and sheet semantics are user-visible

## Direct Interactions

### Backend interaction map

- consumes the common export model and run-level rejection helper:
  - `src-tauri/src/commands_reports_validation_xlsx.rs:1-3`
  - `src-tauri/src/commands_reports_validation_xlsx.rs:2297`
- invoked by the reports wrapper:
  - `src-tauri/src/commands_reports.rs:38-59`
- exposed to the frontend through report export commands:
  - `src-tauri/src/commands_reports_api.rs:14-21`
  - `src-tauri/src/commands_reports_api.rs:32-71`

### Frontend contract anchors

- validation report export flow:
  - `web/src/services/validationService.ts:2569-2814`
- command allowlist/tests:
  - `web/src/tauri/commands.test.ts`
- workflow-level frontend coverage:
  - `web/src/services/validationService.test.ts`

### Test anchors

- direct module tests:
  - `src-tauri/src/commands_reports_validation_xlsx.rs:2468-2635`
- dedicated integration suite:
  - `src-tauri/tests/validation_xlsx_report_tests.rs`

## Standards And Research Log

This packet was written after a targeted external review focused on workbook generation, typed-vs-text cell behavior, chart insertion/layout, and internal API contract clarity.

### Sources consulted

1. `rust_xlsxwriter` workbook module
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/workbook/index.html>
   - Why it was consulted:
     - this file assembles the full workbook and returns it as bytes
   - Standard informed:
     - workbook generation is a first-class, in-memory export path; renderer correctness matters more than throughput here

2. `rust_xlsxwriter::worksheet::Worksheet`
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/worksheet/struct.Worksheet.html>
   - Why it was consulted:
     - this file heavily uses merged titles, freeze panes, inserted charts, links, and typed cell writing
   - Standard informed:
     - worksheet APIs support both typed numeric writes and formatted blank cells, so text placeholders in numeric regions are a deliberate output choice rather than a library limitation

3. `rust_xlsxwriter` chart APIs
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/>
   - Why it was consulted:
     - this file owns dashboard and distribution charts
   - Standard informed:
     - chart ranges and sheet contracts are part of the workbook interface and deserve direct verification

4. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this is a shared internal export surface with a durable output contract
   - Standard informed:
     - internal renderers still need explicit semantics, representative tests, and clear failure handling

## Findings

## Correctness And Safety

### 1. Renderer vocabularies for regime labels are out of sync with the common model

- Evidence:
  - regime fragility display only recognizes:
    - `stable`, `selective`, `fragile`
    - `src-tauri/src/commands_reports_validation_xlsx.rs:160-166`
  - regime support display only recognizes:
    - `broad`, `trend_only`, `range_only`, `vol_selective`, `weak`
    - `src-tauri/src/commands_reports_validation_xlsx.rs:169-177`
  - the common model can derive:
    - fragility labels `broad`, `mixed`, `narrow`, `fragile`
    - `src-tauri/src/commands_reports_validation_common.rs:265-284`
    - support labels `trend_ok`, `mixed`, `range_only`, `weak`
    - `src-tauri/src/commands_reports_validation_common.rs:289-301`
- Why it matters:
  - unrecognized labels render as `not run` in the workbook even when the model actually has a derived status
  - this is a cross-file correctness bug, not a style preference
- Smallest justified fix:
  - align the renderer to the same canonical regime vocabularies as the common layer and frontend, or better, collapse all three onto one shared label set

### 2. The shortlist note text no longer matches the actual shortlist ordering policy

- Evidence:
  - workbook note says:
    - `approved -> hard-gate survivors -> remaining context`
    - `src-tauri/src/commands_reports_validation_xlsx.rs:2347-2355`
  - common shortlist policy is actually:
    - approved
    - hard-gate survivors
    - legacy shortlist
    - remaining context
    - `src-tauri/src/commands_reports_validation_common.rs:2548-2579`
- Why it matters:
  - the workbook is describing the policy incorrectly
- Smallest justified fix:
  - update the note text to reflect the real ordering
- Sensitivity:
  - `Safe now`

### 3. Optional numeric cells are rendered as text placeholders inside numeric regions

- Evidence:
  - `write_optional_number` writes `"not run"` for `None`: `61-77`
  - `write_optional_usize` writes `"n/a"` for `None`: `79-95`
- Why it matters:
  - this creates mixed-type columns in areas that otherwise contain numeric data
  - Excel sorting, formulas, filters, and pivots become weaker or less predictable
- Research relevance:
  - worksheet APIs support typed blanks, so this is an output-contract choice
- Review judgment:
  - objectively suboptimal, but changing it is workbook-contract-sensitive
- Smallest justified fix:
  - if product behavior allows it, switch optional numeric cells to formatted blanks or a consistent numeric-null convention

## API And Naming

### 1. The file is renderer-specific, but its display helpers are effectively schema adapters

- Evidence:
  - helpers such as `display_regime_flag`, `display_regime_support`, `display_cross_mode_label`, and `bootstrap_mode_label` encode business vocabulary, not just presentation sugar
- Why it matters:
  - these mappings are easy to treat as harmless UI text, but they are actually part of the workbook contract
- Smallest justified fix:
  - document these helpers as canonical display adapters for the workbook surface

### 2. Some helper names do not describe the workbook behavior they actually implement

- Evidence:
  - `write_optional_number` and `write_optional_usize` inject text placeholders, not only numeric cells: `61-95`
  - `pct_fmt` is named like a percentage format, but in this file it is defined with a decimal format code rather than a `%` format:
    - `src-tauri/src/commands_reports_validation_xlsx.rs:2255`
- Why it matters:
  - misleading names make workbook-contract changes harder to review
- Smallest justified fix:
  - rename helpers or document the rendering policy explicitly so callers do not infer the wrong semantics

## Error Handling

### 1. Unknown labels silently degrade to `not run`

- Evidence:
  - `display_regime_flag`, `display_regime_support`, and other helpers use catch-all fallbacks to `not run` or `Review`
- Why it matters:
  - label drift in upstream model code can become silent workbook misinformation rather than an explicit regression
- Smallest justified fix:
  - use explicit tests for known label vocabularies
  - consider a more visible placeholder for unknown values during development, while preserving user-friendly output in release if needed

### 2. Error transport is stringly typed, but consistent with the surrounding report surface

- Evidence:
  - all `rust_xlsxwriter` errors are flattened to `String`
- Review judgment:
  - acceptable for the surrounding Tauri/report pipeline, though not ideal for deeper diagnostics

### 3. Some layout and navigation failures are silently discarded

- Evidence:
  - several operations use `let _ = ...` instead of propagating `XlsxError`, including:
    - freeze panes and widths around `301`, `799`, `894`, `1991`
    - dashboard internal links at `1491-1494`
- Why it matters:
  - the export can succeed while losing workbook affordances such as freeze panes, widths, or sheet links
- Smallest justified fix:
  - propagate these failures for critical workbook affordances, or at minimum isolate and comment the ones intentionally treated as best-effort

## Module Boundaries And Technical Debt

### 1. The renderer is a large monolith, but the responsibilities are at least renderer-local

- Evidence:
  - `2635` lines containing formatting, sheet writing, chart building, dashboard logic, and tests
- Why it matters:
  - this is substantial maintenance surface, but it is still more cohesive than the common model builder
- Review judgment:
  - a split into sheet-specific builders could help later, but it is not the smallest necessary change today

### 2. The workbook is compensating for vocabulary and policy drift upstream

- Evidence:
  - current tests tolerate multiple label variants:
    - `src-tauri/tests/validation_xlsx_report_tests.rs:413-425`
- Why it matters:
  - a tolerant test can hide real cross-layer drift instead of surfacing it
- Smallest justified fix:
  - once a canonical label set is chosen, tighten the tests rather than accepting multiple incompatible outputs

## Docs And Examples

### 1. Sheet notes and display labels are doing policy explanation work with no shared contract comment

- Examples:
  - compatibility note: `906-924`
  - expected execution summaries: `1406-1460`
  - friction/regime notes: `1684-1860`
  - shortlist notes: `2347-2355`
- Why it matters:
  - future maintainers can easily change workbook language without realizing they are changing policy communication

## Tests And Missing Verification

### 1. Test coverage is decent overall, but a few important truthfulness gaps remain

- Direct module tests:
  - `3` tests in `2468-2635`
- Dedicated integration tests:
  - `15` tests in `src-tauri/tests/validation_xlsx_report_tests.rs`
- Well-covered areas:
  - sheet presence
  - chart presence
  - compatibility mode
  - run-level rejection behavior
  - various route-note/dashboard surfaces
- Remaining gaps:
  - no test pins the renderer’s regime flag/support vocabulary against the common model vocabulary
  - no test catches the shortlist-note mismatch
  - no focused test verifies hyperlink targets, freeze panes, or column-format semantics
  - no test asserts whether optional numeric placeholders should stay as strings versus blanks

## Performance, Allocation, And CUDA Concerns

### 1. This file is not a performance-priority target

- Evidence:
  - it is export-time workbook assembly with sheet writes and chart creation
  - row cloning for `approved_rows.take(visible_cap)` is small relative to the export path: `2314-2323`
- Review judgment:
  - acceptable for current role

### 2. No benchmark-sensitive runtime issue stands out here

- The benchmark-sensitive pieces are the route-note claims coming from the common model, not the workbook renderer itself.

## Cross-File Interaction Map

### Most likely coordination points if this file changes

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `web/src/services/validationService.ts`

### Downstream consumers needing verification

- validation workbook export flow
- report save/open UX from the frontend
- CSV/XLSX consistency for shortlist and approval semantics
- dashboard interpretation of regime and friction blocks

## Code-Quality Improvement Register

### Safe now

- align renderer regime label mappings with the common model/frontend vocabulary
- fix the shortlist note wording
- add direct tests for regime display mappings and shortlist note text
- tighten tolerant integration tests once vocabulary is canonicalized

### Coordination required

- changing optional numeric placeholders from strings to blanks
- changing sheet labels, note text, or dashboard terminology in ways the frontend/docs may depend on

### Defer until later pass

- split the renderer into dashboard/candidate-sheet/distribution/audit sub-builders

## Technical-Debt Reduction Register

### Debt safe to reduce now

- label-vocabulary drift
- inaccurate shortlist note
- missing direct tests for renderer truthfulness

### Debt that spans multiple files

- canonical regime label vocabulary across common model, workbook renderer, frontend summaries, and tests
- policy-note wording ownership

### Debt that should not be “cleaned up” casually

- sheet names
- dashboard layout/links
- compatibility-mode messaging

## Performance Opportunity Register

- Safe structural opportunities:
  - minor decomposition into sheet-specific helpers later
- Benchmark-sensitive opportunities:
  - none
- Not the right place to chase performance:
  - yes

## Observability And Diagnostics Gaps

- unknown label values degrade to `not run` rather than exposing a clear cross-layer mismatch
- tolerant integration assertions currently hide some renderer/model drift

## Open Questions And Assumptions

- assumption:
  - the workbook is intended to share the same regime label vocabulary as the common model and frontend, not invent its own display-only dialect
- open question:
  - should optional numeric workbook cells remain visibly textual for user readability, or should the renderer prefer Excel-native blanks

## Regression Risk And Feature-Preservation Constraints

- Do not rename sheets or dashboard link targets casually.
- Do not change shortlist/approval messaging without verifying the common model and frontend assumptions.
- Treat regime label fixes as cross-file consistency work; the common model, frontend, and tests should agree on one vocabulary.

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_reports_validation_xlsx -- --nocapture`
2. `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture`
3. `cargo test -p ta_desktop_demo_app commands_reports_validation_common -- --nocapture`
4. `cargo test -p ta_desktop_demo_app commands_reports -- --nocapture`
5. `npm --prefix ./web run test -- src/services/validationService.test.ts src/tauri/commands.test.ts`
6. Manual workbook spot-check if labels or note text change:
   - open the generated workbook
   - verify Dashboard regime/friction sections
   - verify Approved/Shortlist sheet notes

## Recommended Action

`Small surgical refactor`

The renderer does not need a rewrite to become more trustworthy. It needs a bounded truthfulness pass:

- fix the regime vocabulary drift
- make the shortlist note accurate
- decide deliberately whether numeric gaps should stay as text or become blanks
- add direct tests for those exact contracts

That improves backend export quality without destabilizing the broader validation workflow.
