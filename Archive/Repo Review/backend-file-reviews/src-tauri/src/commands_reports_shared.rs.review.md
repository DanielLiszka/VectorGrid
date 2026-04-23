# Review Packet: `src-tauri/src/commands_reports_shared.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_shared.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Solid shared support code, but too under-specified for a module that influences persisted filenames, shell-open UX, and meta statistics`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Semantics-sensitive`, `Path-contract-sensitive`, `Shell-integration-sensitive`, `Support-module`

## What This File Owns

`commands_reports_shared.rs` is the shared helper layer behind the report/export surface.

This file owns:

- the shared sorted-slice statistic helper:
  - `quantile`: `src-tauri/src/commands_reports_shared.rs:6-15`
- pre-shell path validation:
  - `existing_path`: `src-tauri/src/commands_reports_shared.rs:18-29`
- blocking OS integration for reveal/open:
  - `reveal_path_blocking`: `src-tauri/src/commands_reports_shared.rs:32-88`
  - `open_path_blocking`: `src-tauri/src/commands_reports_shared.rs:91-127`
- filename sanitization for report/artifact-style names:
  - `sanitize_filename_component`: `src-tauri/src/commands_reports_shared.rs:130-144`
- app-local reports-dir creation and filename collision handling:
  - `reports_dir_for_base`: `src-tauri/src/commands_reports_shared.rs:147-150`
  - `suggest_report_path_for_base`: `src-tauri/src/commands_reports_shared.rs:153-155`
  - `suggest_report_path_in_dir`: `src-tauri/src/commands_reports_shared.rs:167-189`
- direct unit tests for path and collision behavior:
  - `src-tauri/src/commands_reports_shared.rs:191-268`

This file does not own:

- the public Tauri command surface
  - that is `src-tauri/src/commands_reports_api.rs:74-119`
- full report-generation logic
  - that is `src-tauri/src/commands_reports.rs`
- Stage 3 meta export orchestration
  - that is `src-tauri/src/commands_meta.rs`

## Classification

- Public surface: no
  - this file is wrapped by `src-tauri/src/commands_reports.rs:16-35`
- Hot path: no
  - it runs during save/open/reveal operations and meta-summary aggregation, not in optimization loops
- Runtime-critical: moderate
  - failures here directly affect report path creation and file-opening UX
- CUDA/runtime-critical: no
- Strategy-local: no
- Low-risk support code: yes, but with user-visible contract weight
  - filename rules and open/reveal behavior propagate into persisted paths and frontend UX

## Direct Interactions

### Backend interaction map

- immediate wrapper/re-export layer:
  - `src-tauri/src/commands_reports.rs:16-35`
  - save path selection uses this file at `src-tauri/src/commands_reports.rs:83-91`
- public async Tauri commands depend on these helpers through the wrapper:
  - `src-tauri/src/commands_reports_api.rs:74-119`
- crate command-surface re-export path:
  - `src-tauri/src/commands.rs:132-138`
- meta-analysis runtime uses shared quantile and filename sanitization:
  - filename sanitization in `src-tauri/src/commands_meta.rs:290-292`
  - report/model naming and suffix probing in `src-tauri/src/commands_meta.rs:1955-1972`
  - quantile use in `src-tauri/src/commands_meta.rs:2578-2583`
  - additional quantile use in `src-tauri/src/commands_meta.rs:3055-3060`
- artifact runtime also reuses filename sanitization and parallel path suggestion logic:
  - `src-tauri/src/run_artifacts.rs:8-14`
  - `src-tauri/src/run_artifacts.rs:501-515`

### Frontend contract anchors

- shared open-with-reveal-fallback helper:
  - `web/src/utils/openPath.ts:12-31`
  - `web/src/utils/openPath.test.ts:16-53`
- representative backend consumers:
  - validation flow: `web/src/services/validationService.ts:2638-2728`
  - meta flow: `web/src/services/metaService.ts:2389-2413`
  - latest-meta-report flow: `web/src/services/metaService.ts:2909-2932`
  - data flow: `web/src/services/dataService.ts:468-478`
  - ML signal flow: `web/src/services/mlSignalService.ts:127-134`
  - options-expression flow: `web/src/services/optionsExpressionService.ts:1422-1430`

### Test anchors

- direct unit tests:
  - `src-tauri/src/commands_reports_shared.rs:207-268`
- adjacent Rust coverage:
  - `src-tauri/src/commands_reports.rs:3398-3433`
  - `src-tauri/src/commands_reports_api.rs:358-371`
- frontend fallback coverage:
  - `web/src/utils/openPath.test.ts:16-53`
  - `web/src/services/validationService.test.ts:2008-3458`
  - `web/src/services/metaService.test.ts:1111-1169`
  - `web/src/services/dataService.test.ts:281-348`
  - `web/src/services/mlSignalService.test.ts:745-809`
  - `web/src/stores/commandActionStore.test.ts:228-254`

## Standards And Research Log

This packet was written after a targeted external review focused on filesystem existence checks, process spawning semantics, shared helper API clarity, and quantile-definition ambiguity.

### Sources consulted

1. Rust `std::path::Path`
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html>
   - Why it was consulted:
     - this file uses `Path::exists()` to validate user-facing shell-open targets
   - Standard informed:
     - `exists()` is documented as error-prone, can hide permission/broken-link errors by returning `false`, and has TOCTOU risk; `try_exists()` is the better API when the caller wants error visibility

2. Rust `std::process::Command`
   - Source: <https://doc.rust-lang.org/std/process/struct.Command.html>
   - Why it was consulted:
     - this file uses `Command::spawn()` for OS launch behavior
   - Standard informed:
     - `spawn()` only confirms the child process was started and returns a handle; it does not prove the OS actually opened the target successfully

3. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this is shared backend utility code with non-obvious semantics and user-visible behavior
   - Standard informed:
     - function contracts, failure considerations, and examples/tests should make semantics explicit rather than forcing readers to infer them from implementation

4. NIST Dataplot quantile reference
   - Source: <https://www.itl.nist.gov/div898/software/dataplot/refman2/auxillar/quantile.htm>
   - Why it was consulted:
     - this file exposes a helper named `quantile`, but its implementation is a specific rounded-index selector
   - Standard informed:
     - quantile definitions vary across systems, 0.5 quantile is the median, and multiple methods are common; the exact method should be documented if the helper is reused for product-facing statistics

## Findings

## Correctness And Safety

### 1. `existing_path` uses `Path::exists()`, which hides some real errors and is explicitly discouraged for precise checks

- Evidence:
  - implementation: `src-tauri/src/commands_reports_shared.rs:18-29`
- Research relevance:
  - Rust documents `exists()` as error-prone, notes TOCTOU risk, and states that inaccessible metadata returns `false`
- Why it matters:
  - permission failures, broken symlinks, and other metadata problems are collapsed into “path does not exist”
  - that weakens error specificity on a user-visible path-opening boundary
- Smallest justified fix:
  - replace `exists()` with `try_exists()` and surface metadata-access errors explicitly
- Sensitivity:
  - `Safe now`

### 2. Unsupported targets currently return success instead of an error

- Evidence:
  - fallback `Ok(())` after platform-specific blocks in:
    - `src-tauri/src/commands_reports_shared.rs:87-88`
    - `src-tauri/src/commands_reports_shared.rs:126-127`
- Why it matters:
  - on unsupported targets, callers would receive success even though no shell action occurred
- Smallest justified fix:
  - replace the fallback success with an explicit unsupported-platform error
- Sensitivity:
  - `Safe now`

### 3. The shared `quantile` helper is semantics-sensitive and currently under-specified

- Evidence:
  - implementation rounds to a single sorted index:
    - `src-tauri/src/commands_reports_shared.rs:6-15`
  - product-facing meta summaries depend on it:
    - `src-tauri/src/commands_meta.rs:2578-2583`
    - `src-tauri/src/commands_meta.rs:3055-3060`
- Research relevance:
  - NIST explicitly notes that quantile definitions vary across packages and methods
- Why it matters:
  - downstream readers can easily assume interpolation, Excel-style percentile behavior, or true median semantics when the function currently provides none of those guarantees
- Review judgment:
  - this is not automatically wrong, but it is too ambiguous for a shared helper with output-facing consequences
- Smallest justified fix:
  - at minimum, document the exact method
  - only change the implementation with coordinated verification, because output values in `commands_meta.rs` will change
- Sensitivity:
  - `Semantics-sensitive`

## API And Naming

### 1. `quantile` is named too broadly for its actual contract

- Evidence:
  - name and body: `src-tauri/src/commands_reports_shared.rs:6-15`
- Why it matters:
  - the current name suggests a generally accepted quantile implementation, but the function is really a specific rounded-index selector
- Smallest justified fix:
  - either document the method precisely or rename to something like `rounded_index_quantile` if the repo wants the implementation choice to be unmistakable

### 2. The module name is narrower than the responsibilities it now owns

- Evidence:
  - the file owns report-path helpers, generic shell-open helpers, filename sanitization, and a shared statistic helper, and those are reused outside the report builder itself
  - consumers include `commands_meta.rs` and `run_artifacts.rs`
- Why it matters:
  - the file has become a mixed utility bucket rather than a purely report-local module
- Review judgment:
  - this is real cohesion drift, but renaming/splitting the module is not the smallest justified move right now
- Smallest justified fix:
  - document the actual ownership scope now
  - defer any structural split until a broader runtime-utilities pass

## Error Handling

### 1. Successful `spawn()` does not prove the target actually opened

- Evidence:
  - all shell integrations use `Command::spawn()`:
    - `src-tauri/src/commands_reports_shared.rs:40-47`
    - `src-tauri/src/commands_reports_shared.rs:57-66`
    - `src-tauri/src/commands_reports_shared.rs:75-82`
    - `src-tauri/src/commands_reports_shared.rs:98-105`
    - `src-tauri/src/commands_reports_shared.rs:110-117`
    - `src-tauri/src/commands_reports_shared.rs:119-124`
- Research relevance:
  - Rust documents `spawn()` as starting a child process and returning a handle
- Why it matters:
  - a launch helper can report success even if the OS command later fails to open the resource
- Review judgment:
  - this is a limitation of the chosen integration style, not necessarily a bug
- Smallest justified fix:
  - document this as best-effort launch behavior
  - keep frontend fallback behavior intact

### 2. All failures are flattened into `String`

- Evidence:
  - every exported helper returns `Result<_, String>`
- Why it matters:
  - stringly typed errors are harder to classify or test precisely
- Review judgment:
  - acceptable at the current Tauri boundary style, but still a maintainability tradeoff
- Smallest justified fix:
  - keep string transport if compatibility matters, but normalize important messages and test them more directly

## Module Boundaries And Technical Debt

### 1. Quantile logic is already duplicated in a neighboring workbook module

- Evidence:
  - this file defines `quantile`: `src-tauri/src/commands_reports_shared.rs:6-15`
  - `commands_reports_meta_xlsx.rs` carries a second local copy: `src-tauri/src/commands_reports_meta_xlsx.rs:659-669`
- Why it matters:
  - duplicated statistic helpers are exactly where silent semantic drift starts
- Smallest justified fix:
  - centralize one documented implementation if the repo intends to keep this quantile method

### 2. Filename/path collision probing is parallel to other modules instead of fully shared

- Evidence:
  - this file probes report filename suffixes: `src-tauri/src/commands_reports_shared.rs:167-189`
  - `commands_meta.rs` reimplements a similar timestamp-suffix loop: `src-tauri/src/commands_meta.rs:1955-1972`
  - `run_artifacts.rs` has a parallel helper: `src-tauri/src/run_artifacts.rs:501-515`
- Why it matters:
  - collision policies can drift independently across backend artifact families
- Review judgment:
  - valid debt, but not worth abstracting immediately unless the repo wants a single canonical timestamped-path policy

## Docs And Examples

### 1. Core helper contracts are not documented

- Missing contract notes:
  - quantile method semantics
  - 96-character filename truncation
  - reserved-character replacement rules
  - collision suffix cap of `999`
- Evidence:
  - `src-tauri/src/commands_reports_shared.rs:4`
  - `src-tauri/src/commands_reports_shared.rs:130-144`
  - `src-tauri/src/commands_reports_shared.rs:167-189`
- Why it matters:
  - these are not incidental details; they affect filenames, persisted paths, and meta summaries
- Smallest justified fix:
  - add short contract comments next to these helpers

## Tests And Missing Verification

### 1. Direct coverage is decent for collisions and missing-path rejection, but important behavior is still unpinned

- Direct tests currently cover:
  - reports-dir creation
  - missing-path rejection for both shell helpers
  - first collision suffix
  - suffix exhaustion
  - `reports/` blocked by a file
  - refs: `src-tauri/src/commands_reports_shared.rs:207-268`
- Adjacent coverage currently covers:
  - wrapper-layer collision behavior in `src-tauri/src/commands_reports.rs:3398-3433`
  - async missing-path surfacing in `src-tauri/src/commands_reports_api.rs:358-371`
  - frontend open-with-reveal-fallback behavior in the web test suite
- Missing direct coverage:
  - `quantile` edge semantics
  - `sanitize_filename_component` colon replacement, truncation, and empty-input behavior
  - empty-string and whitespace-only path rejection
  - unsupported-platform behavior
  - real successful shell-launch flows
- Smallest justified fix:
  - add narrow unit tests for `quantile`, filename sanitization, and blank-path rejection
  - keep successful shell launch as manual verification unless the repo introduces a spawn seam/mock

## Performance, Allocation, And CUDA Concerns

### 1. There is no meaningful hot-path performance problem here

- Evidence:
  - allocations are tiny and bounded to path/name handling
  - suffix probing is linear up to `999`: `src-tauri/src/commands_reports_shared.rs:178-183`
- Review judgment:
  - acceptable for occasional report-save flows
  - not a candidate for benchmark work before correctness and contract clarity

### 2. No CUDA or benchmark-sensitive concerns stand out

- This file is pure support/runtime glue, not computational backend code.

## Cross-File Interaction Map

### Most likely coordination points if this file changes

- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/utils/openPath.ts`

### Downstream consumers needing verification

- report save path creation
- report-directory open flow
- file open and reveal fallback flow
- meta-summary quantile outputs
- artifact naming stability where `sanitize_filename_component` is reused

## Code-Quality Improvement Register

### Safe now

- replace `exists()` with `try_exists()` and preserve underlying metadata errors
- return explicit unsupported-platform errors instead of silent success
- add unit tests for `quantile`, `sanitize_filename_component`, and blank-path rejection
- document quantile semantics, truncation, and collision-cap behavior

### Coordination required

- changing quantile semantics
- changing filename sanitization rules
- changing suffix policy or collision cap
- changing open/reveal behavior that frontend fallback flows implicitly assume

### Defer until later pass

- splitting this module into separate stats/path/shell utility modules
- consolidating all timestamped filename-suggestion logic across reports, meta artifacts, and run artifacts

## Technical-Debt Reduction Register

### Debt safe to reduce now

- imprecise `exists()`-based validation
- undocumented helper contracts
- missing edge-case tests

### Debt that spans multiple files

- quantile semantics and whether they should match spreadsheet/statistical-tool expectations
- canonical timestamped filename policy across report and artifact subsystems
- whether shell-open helpers belong in a more generic runtime utility layer

### Debt that should not be “cleaned up” casually

- fallback behavior from `open_path` to `reveal_path`
- persisted filename stability
- current collision suffix scheme unless consumers are migrated deliberately

## Performance Opportunity Register

- Safe structural opportunities:
  - none worth prioritizing
- Benchmark-sensitive opportunities:
  - none
- Not the right place to chase performance:
  - yes

## Observability And Diagnostics Gaps

- callers cannot distinguish missing paths from inaccessible paths today because `exists()` flattens the distinction
- successful launch responses only confirm process spawn, not completed OS open behavior

## Open Questions And Assumptions

- assumption:
  - Windows remains the primary product target, which is why unsupported-platform behavior has not been stressed in tests
- open question:
  - should the repo standardize on a documented quantile method across all backend summary/report code, or is the current rounded-index behavior intentionally product-specific
- open question:
  - should filename sanitization/truncation become a repo-wide artifact utility instead of remaining report-local infrastructure

## Regression Risk And Feature-Preservation Constraints

- Do not change `quantile` semantics casually; `commands_meta.rs` consumes this helper for user-visible summary statistics.
- Do not change filename sanitization or collision policy casually; they influence persisted paths and artifact discoverability.
- Preserve the current frontend contract that tries `open_path` first and falls back to `reveal_path` on error.
- If shell-launch behavior changes, verify on a real desktop environment; current automated tests do not prove successful OS launch behavior.

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_reports_shared -- --nocapture`
2. `cargo test -p ta_desktop_demo_app commands_reports_api -- --nocapture`
3. `cargo test -p ta_desktop_demo_app commands_reports -- --nocapture`
4. `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`
5. `npm --prefix ./web run test -- src/utils/openPath.test.ts src/services/validationService.test.ts src/services/metaService.test.ts src/services/dataService.test.ts src/services/mlSignalService.test.ts src/services/optionsExpressionService.test.ts src/stores/commandActionStore.test.ts`
6. Manual desktop verification if open/reveal helpers change:
   - open an existing report file
   - reveal an existing report file
   - open the app-local `reports/` directory

## Recommended Action

`Small surgical refactor`

The right move is a bounded hardening pass, not a rewrite:

- fix the path-validation API choice
- return real unsupported-platform errors
- document the semantics that already matter
- add the missing edge-case tests

Anything beyond that, especially quantile semantics or filename-policy changes, needs coordinated review because those behaviors already leak into user-visible backend outputs.
