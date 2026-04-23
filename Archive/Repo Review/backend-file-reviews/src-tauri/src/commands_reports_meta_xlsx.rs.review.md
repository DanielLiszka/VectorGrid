# Review Packet: `src-tauri/src/commands_reports_meta_xlsx.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_meta_xlsx.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful internal workbook builder, but not yet hardened enough to treat as a stable long-lived export contract without stronger correctness and workbook-level verification`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Semantics-sensitive`, `Workbook-contract-sensitive`, `Support-module`, `Cross-file-meta-export`

## What This File Owns

`commands_reports_meta_xlsx.rs` owns the Stage 3 meta-analysis XLSX workbook builder.

This file owns:

- the internal byte builder used to materialize the workbook:
  - `build_meta_report_xlsx_bytes`: `src-tauri/src/commands_reports_meta_xlsx.rs:4-642`
- workbook structure and sheet layout for:
  - `Meta Analysis`: `src-tauri/src/commands_reports_meta_xlsx.rs:33-284`
  - `By Asset (Train)`: `src-tauri/src/commands_reports_meta_xlsx.rs:287-401`
  - `Asset Matrix`: `src-tauri/src/commands_reports_meta_xlsx.rs:404-483`
  - `Holdout Rows`: `src-tauri/src/commands_reports_meta_xlsx.rs:486-566`
  - `Distributions`: `src-tauri/src/commands_reports_meta_xlsx.rs:569-637`
- chart composition and placement:
  - summary charts: `src-tauri/src/commands_reports_meta_xlsx.rs:255-283`
  - distribution charts via helper: `src-tauri/src/commands_reports_meta_xlsx.rs:634-777`
- local export-side aggregation helpers:
  - `median_train_pnl_pct_by_dataset`: `src-tauri/src/commands_reports_meta_xlsx.rs:645-656`
  - `quantile`: `src-tauri/src/commands_reports_meta_xlsx.rs:659-669`
  - histogram/table helpers: `src-tauri/src/commands_reports_meta_xlsx.rs:671-777`
- direct workbook-zip smoke tests:
  - `src-tauri/src/commands_reports_meta_xlsx.rs:780-983`

This file does not own:

- when meta export runs or where the output path is chosen
  - `src-tauri/src/commands_meta.rs:2284-2318`
- the façade entry point used by neighboring report modules
  - `src-tauri/src/commands_reports.rs:44-48`
- the underlying `MetaSelectionResult` model
  - sourced from `ta_app_core`

## Classification

- Public surface: no
  - this file is an internal report builder behind `src-tauri/src/commands_reports.rs:44-48`
- Hot path: no
  - it runs during export, not during optimization or validation execution loops
- Runtime-critical: moderate
  - Stage 3 meta export depends on it end to end
- CUDA/runtime-critical: no
- Strategy-local: no
  - workbook is generic meta-analysis export infrastructure
- Low-risk support code: yes, but with meaningful downstream output-contract coupling
  - the workbook path is persisted in `src-tauri/src/commands_meta.rs:2298-2313`
  - and frontend flows open or reveal the generated workbook via `web/src/services/metaService.ts:621-631` and `web/src/services/metaService.ts:2909-2932`

## Direct Interactions

### Backend interaction map

- export façade:
  - `src-tauri/src/commands_reports.rs:44-48`
- meta-analysis runtime that calls this builder, writes bytes to disk, and persists the report path:
  - `src-tauri/src/commands_meta.rs:2284-2318`

### Frontend and downstream contract anchors

- meta run response normalization stores `report_xlsx_path`:
  - `web/src/services/metaService.ts:621-631`
- latest-meta-report open/reveal flows rely on that persisted path:
  - `web/src/services/metaService.ts:2909-2932`
- adjacent runtime snapshot tests for latest-report behavior:
  - `web/src/stores/commandActionStore.test.ts:152-243`

### Local test anchors

- direct smoke and zip-contents tests:
  - `src-tauri/src/commands_reports_meta_xlsx.rs:791-983`
- adjacent meta runtime tests:
  - `src-tauri/src/commands_meta.rs` via `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`

## Standards and Research Log

This packet was written after a targeted external review focused on workbook-generation contracts, chart/blank-cell behavior, in-memory XLSX creation, and the expected semantics of “median”.

### Sources consulted

1. `rust_xlsxwriter` workbook module
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/workbook/index.html>
   - Why it was consulted:
     - this file ends by emitting the workbook with `Workbook::save_to_buffer()`
   - Standard informed:
     - `save_to_buffer()` is intended for in-memory/streamed XLSX output, so the current pattern is reasonable for export code, but it necessarily materializes the full workbook in memory

2. `rust_xlsxwriter::worksheet::Worksheet`
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/worksheet/struct.Worksheet.html>
   - Why it was consulted:
     - this file writes numeric cells, string placeholders, formatted blanks, freeze panes, and inserted charts
   - Standard informed:
     - `write_blank()` exists specifically for formatted blank cells, and chart insertion is a first-class worksheet operation with explicit error behavior

3. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - even though this is an internal support module, it owns a reusable output contract and should still be assessed for documentation, naming clarity, failure handling, and verification posture
   - Standard informed:
     - internal modules with reusable behavior still benefit from clear contracts, representative examples/tests, and explicit failure considerations

4. Microsoft Excel `Median` semantics
   - Source: <https://learn.microsoft.com/en-us/dotnet/api/microsoft.office.interop.excel.worksheetfunction.median?view=excel-pia>
   - Why it was consulted:
     - this file labels the Asset Matrix statistic as “median by dataset”
   - Standard informed:
     - for an even-sized set, median is the average of the two middle values, not a rounded single-index pick

## Findings

## Correctness And Safety

### 1. The file labels a statistic as median but does not compute the median for even-sized samples

- Evidence:
  - sheet title says `Per-asset PnL% matrix (median by dataset)`: `src-tauri/src/commands_reports_meta_xlsx.rs:411-418`
  - `median_train_pnl_pct_by_dataset()` delegates to a local `quantile(&vs, 0.5)`: `src-tauri/src/commands_reports_meta_xlsx.rs:645-656`
  - `quantile()` rounds to a single index instead of averaging the middle two values for even-length inputs: `src-tauri/src/commands_reports_meta_xlsx.rs:659-669`
- Why it matters:
  - for a two-point sample such as `[0.01, 0.99]`, the current implementation returns `0.99`, not the median
  - this is a workbook-correctness issue, not a style issue
- Review judgment:
  - this should be treated as a correctness fix, but because exported values will change, rollout is still `Semantics-sensitive`
- Smallest justified fix:
  - replace the local median logic with a true median implementation or consume a precomputed median from a shared/core helper
  - add a regression test covering even-sized samples

### 2. Missing numeric values are emitted as `"-"` strings inside numeric regions

- Evidence:
  - summary holdout metrics fall back to strings: `src-tauri/src/commands_reports_meta_xlsx.rs:223-240`
  - asset-matrix missing dataset cells also write `"-"`: `src-tauri/src/commands_reports_meta_xlsx.rs:471-478`
- Why it matters:
  - this creates mixed-type columns in workbook regions that otherwise contain numeric data
  - it weakens Excel-side sorting, filtering, formulas, and pivot-table behavior
- Research relevance:
  - `rust_xlsxwriter` explicitly supports blank formatted cells through `write_blank()`, so there is a library-native way to keep a visually formatted cell without turning the column into text
- Review judgment:
  - objectively suboptimal, but contract-sensitive because users may already expect visible `"-"` placeholders
- Smallest justified fix:
  - if product behavior allows it, switch missing numeric cells to formatted blanks or a consistent numeric-null convention
  - verify workbook UX manually before landing

### 3. Pass-rate formatting is inconsistent across workbook surfaces

- Evidence:
  - summary pass-rate cells use `pct_fmt`: `src-tauri/src/commands_reports_meta_xlsx.rs:174-206`
  - distribution “Pass rate” histogram bins are written with `num6_fmt`: `src-tauri/src/commands_reports_meta_xlsx.rs:581-604`
- Why it matters:
  - the same metric appears as percentage-formatted in one sheet and raw decimal fractions in another
- Review judgment:
  - not a compute bug, but it is an output-contract inconsistency that should be resolved deliberately
- Smallest justified fix:
  - pick one presentation rule for pass rate and apply it across summary tables and histogram tables/charts

## API And Naming

### 1. `median_train_pnl_pct_by_dataset` currently over-promises what it returns

- Evidence:
  - function name says `median`, but implementation is rounded-index quantile selection: `src-tauri/src/commands_reports_meta_xlsx.rs:645-669`
- Why it matters:
  - the current name makes the workbook easier to trust than the implementation deserves
- Smallest justified fix:
  - either compute a true median or rename the helper and worksheet label to match the actual statistic

### 2. The workbook contract is implicit rather than documented

- Evidence:
  - five sheets, six charts, placeholder rows, and multiple derived tables are all encoded directly in implementation without module-level contract notes: `src-tauri/src/commands_reports_meta_xlsx.rs:4-777`
- Why it matters:
  - this file is effectively a report schema, but future maintainers must infer that schema from code and fragile tests
- Smallest justified fix:
  - add a short module comment documenting sheet inventory, chart inventory, and the intended meaning of the derived tables

## Error Handling

### 1. Parameter-JSON serialization failures are silently collapsed to empty strings

- Evidence:
  - `serde_json::to_string(...).unwrap_or_default()` appears in three workbook regions:
    - `src-tauri/src/commands_reports_meta_xlsx.rs:162`
    - `src-tauri/src/commands_reports_meta_xlsx.rs:341`
    - `src-tauri/src/commands_reports_meta_xlsx.rs:529`
- Why it matters:
  - if serialization ever fails, the workbook silently loses useful diagnostic content instead of surfacing a report-generation error or a visible placeholder
- Review judgment:
  - low-frequency, but still a real diagnostics gap
- Smallest justified fix:
  - convert these to explicit error propagation or at least an intentional placeholder string such as `"<params serialization failed>"`

## Module Boundaries And Technical Debt

### 1. Report-layout code currently owns local statistical logic that likely belongs elsewhere

- Evidence:
  - workbook module contains both formatting/layout code and statistical helpers:
    - `median_train_pnl_pct_by_dataset`: `src-tauri/src/commands_reports_meta_xlsx.rs:645-656`
    - `quantile`: `src-tauri/src/commands_reports_meta_xlsx.rs:659-669`
    - histogram helpers: `src-tauri/src/commands_reports_meta_xlsx.rs:671-777`
- Why it matters:
  - export modules are a poor place for numerical-definition drift
  - the median bug above is a concrete example of why statistics embedded in presentation code become maintenance debt
- Smallest justified fix:
  - keep workbook rendering here, but move reusable statistic helpers into a shared report/meta utility or consume already-computed aggregates from core code

### 2. The Asset Matrix title merge range is fixed while dataset columns are dynamic

- Evidence:
  - title merge range is hard-coded to column `10`: `src-tauri/src/commands_reports_meta_xlsx.rs:411-418`
  - dataset columns are generated dynamically from `datasets.iter()`: `src-tauri/src/commands_reports_meta_xlsx.rs:443-447`
- Why it matters:
  - this is not a correctness bug, but it is a layout-maintenance smell and can leave the title visually mismatched with wide matrices
- Smallest justified fix:
  - compute the merge end column from the dataset count, with an upper/lower bound if needed

## Docs And Examples

### 1. The workbook’s no-data and placeholder branches are not documented

- Evidence:
  - no-train fallback: `src-tauri/src/commands_reports_meta_xlsx.rs:394-400`
  - no-matrix-data fallback: `src-tauri/src/commands_reports_meta_xlsx.rs:428-430`
  - no-holdout fallback: `src-tauri/src/commands_reports_meta_xlsx.rs:561-563`
  - histogram no-data fallback: `src-tauri/src/commands_reports_meta_xlsx.rs:733-735`
- Why it matters:
  - these are user-visible workbook behaviors and part of the maintenance contract
- Smallest justified fix:
  - describe these branches in a short module-level contract note and add tests that assert them explicitly

## Tests And Missing Verification

### 1. Current tests are far too shallow for the workbook contract this file owns

- Evidence:
  - direct tests only cover smoke generation and the presence of workbook sheet names plus `chart1.xml` and `chart2.xml`: `src-tauri/src/commands_reports_meta_xlsx.rs:791-983`
  - the file actually emits:
    - five sheets: `src-tauri/src/commands_reports_meta_xlsx.rs:33-637`
    - two summary charts: `src-tauri/src/commands_reports_meta_xlsx.rs:255-283`
    - four distribution charts: `src-tauri/src/commands_reports_meta_xlsx.rs:634-637`
- Missing coverage:
  - even-sample median correctness in Asset Matrix
  - four distribution chart parts
  - no-data branches for train rows, holdout rows, matrix, and histograms
  - representative cell values in summary, holdout, and matrix sheets
  - pass-rate formatting consistency
- Smallest justified fix:
  - expand workbook tests to assert concrete cell content and all chart parts, not just coarse zip presence

## Performance, Allocation, And CUDA Concerns

### 1. `save_to_buffer()` materializes the full workbook in memory, but this is acceptable for the current role

- Evidence:
  - final output path is `Workbook::save_to_buffer()`: `src-tauri/src/commands_reports_meta_xlsx.rs:640-642`
- Research relevance:
  - `rust_xlsxwriter` documents `save_to_buffer()` as the in-memory/streaming path
- Review judgment:
  - this is not the right place to chase performance first
  - the module is export support code, not a hot runtime path
- Recommendation:
  - keep the current approach unless meta reports become large enough to show real memory pressure in practice

### 2. No benchmark-sensitive or CUDA-sensitive issues stand out in this file

- This file is workbook/export code only.
- Do not treat it as a performance-tuning target ahead of optimization, validation, or GPU runtime modules.

## Cross-File Interaction Map

### Most likely coordination points if this file changes

- `src-tauri/src/commands_reports.rs`
  - internal façade and report-module assembly
- `src-tauri/src/commands_meta.rs`
  - runtime flow that generates, saves, and persists the workbook
- `web/src/services/metaService.ts`
  - downstream UX that opens the report path and summarizes the meta run

### Downstream consumers that need verification if this file changes

- Stage 3 meta run export flow
- latest meta report open flow
- reveal-meta-files flow
- any future tests that inspect workbook zip contents or sheet names

## Code-Quality Improvement Register

### Safe now

- add workbook-content tests for:
  - even-sample median behavior
  - all six charts
  - no-data branches
  - representative summary/matrix/holdout cells
- document the workbook schema at module level
- replace silent `unwrap_or_default()` fallbacks with explicit diagnostics
- compute the Asset Matrix title merge range dynamically

### Coordination required

- switch numeric placeholder strings from `"-"` to formatted blanks or another null convention
- normalize pass-rate presentation across sheets
- fix the median implementation if consumers depend on exact workbook values today

### Defer until later pass

- any broader refactor that moves all report-statistic helpers into shared/core modules
- any redesign of workbook layout, sheet naming, or chart arrangement

## Technical-Debt Reduction Register

### Debt safe to reduce now

- shallow workbook tests
- undocumented sheet/chart contract
- silent diagnostics loss on param serialization
- cosmetic fixed merge range

### Debt that spans multiple files

- where derived export statistics should live
- whether missing numeric values should remain visible strings or become true blanks
- whether report formatting should align with other workbook exporters in the repo

### Debt that should not be “cleaned up” casually

- existing sheet names
- report path persistence behavior
- placeholder/no-data branches that preserve user-visible output rather than failing hard

## Performance Opportunity Register

- Safe structural opportunities:
  - none worth prioritizing before correctness and verification hardening
- Benchmark-sensitive opportunities:
  - none currently justified
- Not the right place to chase performance:
  - yes; this file should be improved for correctness, contract clarity, and test depth first

## Observability And Diagnostics Gaps

- workbook generation currently gives no contextual hint when param serialization falls back to empty strings
- tests do not make workbook regressions legible at the sheet or cell level

## Open Questions And Assumptions

- assumption:
  - workbook consumers care more about readable placeholders than Excel-native blank numeric cells today, since the file currently writes `"-"` explicitly
- assumption:
  - the Asset Matrix label intends a statistical median, not merely a central quantile approximation
- open question:
  - should pass-rate bins be presented as percentages for human readability or left as raw fractions for easier downstream numeric reuse

## Regression Risk And Feature-Preservation Constraints

- Do not change sheet names casually.
- Do not remove placeholder branches that currently guarantee a workbook still renders when data is sparse.
- Treat any change to median semantics, missing-value representation, or pass-rate formatting as workbook-contract-sensitive and verify with both tests and a manual Excel/LibreOffice spot-check.
- Keep Stage 3 meta export behavior intact: this module must continue to return workbook bytes that `commands_meta.rs` can persist without changing the surrounding artifact flow.

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_reports_meta_xlsx -- --nocapture`
2. `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`
3. `npm --prefix ./web run test -- src/services/metaService.test.ts src/stores/commandActionStore.test.ts`
4. Manual workbook spot-check:
   - open a generated Stage 3 meta workbook
   - verify sheet names, chart presence, Asset Matrix values, and sparse-data fallback rows

## Recommended Action

`Small surgical refactor`

The highest-value work here is not a redesign. It is a bounded hardening pass:

- correct the mislabeled median behavior
- decide deliberately how missing numeric cells should be represented
- make pass-rate presentation consistent
- raise the workbook test bar so future report changes stop being guesswork

That keeps the file additive and useful without disturbing the surrounding meta runtime or frontend open-path flows.
