# Review Packet: `src-tauri/src/commands_reports.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Functionally capable, but too broad and under-hardened to treat as a finished canonical backend export boundary`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Artifact-contract-sensitive`, `Export-surface-sensitive`, `Legacy-code-sensitive`, `Frontend-and-CLI-dependent`

## What This File Owns

`commands_reports.rs` is the backend report-export coordination hub behind the Tauri-facing report commands.

This file owns:

- the crate-local façade over report submodules:
  - module declarations: `src-tauri/src/commands_reports.rs:7-16`
  - shared helper re-exports: `src-tauri/src/commands_reports.rs:18-35`
- blocking/headless report-export entrypoints:
  - shortlist CSV wrapper: `src-tauri/src/commands_reports.rs:38-42`
  - meta workbook bridge: `src-tauri/src/commands_reports.rs:44-49`
  - validation workbook base64 builder: `src-tauri/src/commands_reports.rs:51-59`
  - headless workbook bytes builder: `src-tauri/src/commands_reports.rs:61-71`
  - save-to-disk helpers: `src-tauri/src/commands_reports.rs:73-107`
- artifact-to-report translation for artifact-backed export:
  - dataset-meta projection: `src-tauri/src/commands_reports.rs:109-190`
  - report reconstruction from saved artifacts: `src-tauri/src/commands_reports.rs:192-217`
  - artifact-backed save plus best-effort metadata persistence: `src-tauri/src/commands_reports.rs:219-250`
- retained legacy/reference workbook builders that are no longer on the active product path:
  - `build_report_xlsx_analysis_v2`: `src-tauri/src/commands_reports.rs:252-1767`
  - `build_report_xlsx_bytes`: `src-tauri/src/commands_reports.rs:1769-3137`

This file does not own:

- the actual Tauri command surface
  - that lives in `src-tauri/src/commands_reports_api.rs`
- the active validation workbook layout/model logic
  - that lives in `src-tauri/src/commands_reports_validation_common.rs`
  - and `src-tauri/src/commands_reports_validation_xlsx.rs`
- the actual shortlist CSV text builder
  - that lives in `src-tauri/src/commands_reports_csv.rs`
- artifact path validation and general run-artifact persistence policy
  - that lives in `src-tauri/src/run_artifacts.rs`

## Classification

- Public surface: indirect but real
  - helpers are re-exported from `src-tauri/src/commands.rs:132-138`
  - downstream Tauri commands wrap this file through `src-tauri/src/commands_reports_api.rs:14-119`
  - those commands are registered in `src-tauri/src/lib.rs:126-133`
  - the headless CLI also depends on it through `src-tauri/src/bin/ta_headless_validate.rs:15` and `src-tauri/src/bin/ta_headless_validate.rs:677`
- Hot path: no
  - this is user-triggered report export/orchestration, not optimization or validation execution
- Runtime-critical: moderate
  - it controls artifact-backed export behavior, saved report paths, and backend/frontend export fallback shape
- Benchmark-sensitive: low on the current product path
  - export is interactive and post-run, not a perf gate
- CUDA/runtime-critical: no
- Strategy-local: no
  - this file is shared across validation and meta-report workflows
- Low-risk support code only: no
  - it owns persisted-artifact mutation and report-schema shaping

## Direct Interactions

### Backend owners this file depends on

- Tauri-facing wrapper layer:
  - `src-tauri/src/commands_reports_api.rs:14-119`
- active validation workbook model and rendering:
  - `src-tauri/src/commands_reports_validation_common.rs:2581-2728`
  - `src-tauri/src/commands_reports_validation_xlsx.rs:2293-2435`
- shortlist CSV shaping:
  - `src-tauri/src/commands_reports_csv.rs:12-95`
- meta workbook generation:
  - `src-tauri/src/commands_reports_meta_xlsx.rs:5-779`
- shared path/open/reveal helpers:
  - `src-tauri/src/commands_reports_shared.rs:6-189`
- artifact loading and save-back semantics:
  - `src-tauri/src/run_artifacts.rs:1378-1400`
  - `src-tauri/src/run_artifacts.rs:2415-2419`
  - `src-tauri/src/run_artifacts.rs:2422-2435`
  - `src-tauri/src/run_artifacts.rs:1001-1009`

### Backend modules that already depend on this file

- crate command surface assembly:
  - `src-tauri/src/commands.rs:132-138`
- headless validation CLI:
  - `src-tauri/src/bin/ta_headless_validate.rs:15`
  - `src-tauri/src/bin/ta_headless_validate.rs:677`
- meta analysis export and shared quantile/filename helpers:
  - `src-tauri/src/commands_meta.rs:1-5`
  - `src-tauri/src/commands_meta.rs:2288-2298`
  - `src-tauri/src/commands_meta.rs:2579-2583`
- run-artifact naming and reveal behavior:
  - `src-tauri/src/run_artifacts.rs:13`
  - `src-tauri/src/run_artifacts.rs:508`

### Frontend contract anchors

- Tauri command allowlist:
  - `web/src/tauri/commands.ts:46-53`
- main validation export orchestration and fallback behavior:
  - `web/src/services/validationService.ts:2531-2543`
  - `web/src/services/validationService.ts:2569-2755`
  - `web/src/services/validationService.ts:2813-2814`
- settings-side reports directory open:
  - `web/src/services/settingsService.ts:15-16`
- report/open actions:
  - `web/src/stores/commandActionStore.test.ts:110-243`

### REPO_MAP anchors

- report-export entrypoints:
  - `REPO_MAP.md:312-326`
  - `REPO_MAP.md:1391-1403`
- report output location:
  - `REPO_MAP.md:1366-1367`
  - `REPO_MAP.md:1400-1401`
- command cluster mapping:
  - `REPO_MAP.md:1783`

## Standards and Research Log

This packet was written after a targeted official-source review focused on Tauri command contracts, Rust API validation expectations, file-write semantics, and the XLSX-writer performance/memory modes that matter for this export boundary.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - report export is reached through Tauri commands even though this file sits one layer behind them
   - Standard informed:
     - the backend export contract is effectively public and should be explicit about payload/error semantics

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file shapes a backend surface reused by Tauri commands, CLIs, and other backend modules
   - Standard informed:
     - functions that accept loose input should validate important contract assumptions
     - public behavior should be truthful about what the file actually supports

3. Rust `std::fs::write`
   - Source: <https://doc.rust-lang.org/std/fs/fn.write.html>
   - Why it was consulted:
     - this file persists generated workbooks directly with `std::fs::write`
   - Standard informed:
     - writes replace full file contents and are whole-file operations, which matters when deciding whether save-back and temp-file strategies are needed

4. `rust_xlsxwriter` crate docs
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/>
   - Why it was consulted:
     - the active workbook builders depend on `rust_xlsxwriter`
   - Standard informed:
     - the crate is optimized for XLSX generation, but workbook construction is still a deliberate output-format boundary rather than a trivial helper

5. `rust_xlsxwriter::Workbook` docs
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/workbook/struct.Workbook.html>
   - Why it was consulted:
     - this file currently uses buffered workbook generation, while the crate also offers `save_to_writer`, `save_to_buffer`, and low/constant-memory worksheet modes
   - Standard informed:
     - there are legitimate lower-memory options, but they come with feature/usage constraints and are not a casual safe-now refactor for a chart-heavy export path

## Applicable Current Standards

- Export-boundary standard:
  - artifact-backed export commands should validate that the artifact type matches the mutation they are about to perform
- Truthful-surface standard:
  - names and docs should make it clear that the active workbook path is validation-report export, not a generic arbitrary-report builder
- Coordinator-file standard:
  - active runtime paths should be easy to distinguish from retained legacy/reference code
- Persistence standard:
  - whole-file save behavior and save-back mutation paths should be intentionally bounded and testable
- Performance standard:
  - memory-reduction changes to workbook generation belong in the benchmark-sensitive/defer bucket unless the current path is proven problematic

## Findings

### Correctness / Safety

#### 1. Artifact-backed export does not verify artifact kind before mutating the artifact file

Objective issue:

- `save_report_xlsx_from_artifact_blocking` loads a JSON value through `load_artifact_value_checked_blocking`, reconstructs a report, writes an XLSX, and then writes `report_xlsx_path` back into the source artifact:
  - `src-tauri/src/commands_reports.rs:219-250`
- `build_validation_report_from_artifact` only requires `/validation/res` to exist:
  - `src-tauri/src/commands_reports.rs:192-217`
- the loader used here only enforces `runs/` containment plus JSON parsing:
  - `src-tauri/src/run_artifacts.rs:1378-1400`
  - `src-tauri/src/run_artifacts.rs:2415-2419`
- a neighboring artifact loader already performs an explicit run-artifact-kind check:
  - `src-tauri/src/run_artifacts.rs:2422-2435`

Impact:

- any JSON file under the runs directory with a compatible validation shape can be treated as exportable and then mutated as though it were a canonical run artifact
- that is avoidable contract looseness on a persisted backend surface

Smallest justified fix:

- require a recognized run-artifact `kind` before artifact-backed export proceeds
- add a negative test proving non-run or malformed artifact kinds are rejected

Sensitivity:

- safe-now
- artifact-contract-sensitive

### API and Naming

#### 2. The active implementation is validation-report specific, but the main names still read as generic “report” builders

Objective issue:

- all active workbook/export entrypoints in this file delegate directly to the validation workbook builder:
  - `src-tauri/src/commands_reports.rs:51-59`
  - `src-tauri/src/commands_reports.rs:61-71`
  - `src-tauri/src/commands_reports.rs:83-107`
- the file itself documents that the product path now goes through the validation workbook builder:
  - `src-tauri/src/commands_reports.rs:61-65`
- the live model builder also assumes a validation-report shape:
  - `src-tauri/src/commands_reports_validation_common.rs:2581-2728`
- the frontend fallback branch calls `build_report_xlsx` specifically as a validation export fallback:
  - `web/src/services/validationService.ts:2666-2693`

Impact:

- the current surface works, but the naming is broader than the live contract
- that makes future backend review and future refactors easier to misread than they should be

Smallest justified fix:

- document clearly in code and packet notes that the active product path is validation-report export
- keep renaming or command-churn deferred unless the frontend contract is intentionally revised

Sensitivity:

- semantics-sensitive
- frontend-contract-sensitive

### Error Handling

#### 3. Best-effort artifact save-back is acceptable, but its warning branches are not explicitly covered

Objective issue:

- artifact save-back failure is downgraded into `artifact_update_warning`:
  - `src-tauri/src/commands_reports.rs:234-246`
- current automated coverage is strong on positive cases:
  - `src-tauri/src/commands_reports.rs:3437-3721`
  - `src-tauri/src/commands_reports_api.rs:277-318`
- but there is no explicit test for:
  - non-object artifact payload warning
  - save-back persistence failure warning

Impact:

- users can still receive a saved XLSX successfully, but artifact metadata may silently remain stale except for a warning string
- that behavior is reasonable, but it deserves direct test coverage

Smallest justified fix:

- add negative-path tests that assert the warning string for both branches

Sensitivity:

- safe-now

### Module Boundaries / Tech Debt

#### 4. The active coordinator file still contains two large dead legacy workbook builders

Objective issue:

- the active path is now a thin delegation layer:
  - `src-tauri/src/commands_reports.rs:51-107`
- but the same file still carries two retained legacy/reference builders:
  - `src-tauri/src/commands_reports.rs:252-1767`
  - `src-tauri/src/commands_reports.rs:1769-3137`
- the file is therefore still `3843` lines long even though the live ownership has narrowed substantially

Impact:

- active and legacy concerns are interleaved in the same module
- this increases review noise, makes live ownership harder to see, and leaves multiple mentally competing report-selection implementations in one place

Recommended action:

- do not delete legacy builders casually
- isolate them into a dedicated legacy/reference module once the team is ready to preserve or retire them intentionally

Sensitivity:

- safe-now to isolate
- semantics-sensitive to remove

#### 5. Dataset-meta projection is hand-maintained and duplicated across `data.meta` and `data.ctx`

Objective issue:

- `build_report_data_from_dataset_meta` maintains separate allowlists for string, numeric, and boolean fields, and deliberately duplicates some keys into both subtrees:
  - `src-tauri/src/commands_reports.rs:109-190`

Impact:

- this is workable today and reasonably tested
- however, it creates drift risk whenever dataset metadata grows or downstream report consumers change expectations

Smallest justified fix:

- keep behavior as-is for now
- if this area changes, centralize the field descriptor table rather than extending multiple ad hoc lists

Sensitivity:

- safe-now if touched locally
- semantics-sensitive if the projected schema changes

### Docs / Examples

#### 6. There is no module-level documentation that cleanly separates the active path from retained historical builders

Objective issue:

- the file contains a short comment near the active headless builder:
  - `src-tauri/src/commands_reports.rs:61-65`
- but the module still reads primarily as a giant workbook file, not as a coordination hub with retained historical code

Impact:

- contributors can easily spend time reasoning about inactive builders as though they were still product-critical

Smallest justified fix:

- add module-level documentation stating:
  - which path is active
  - which builders are legacy/reference only
  - which neighboring modules own the live workbook model/layout path

Sensitivity:

- safe-now

### Tests / Missing Verification

#### 7. Coverage is strong, but the missing cases are specific and important

Current strong coverage:

- direct file tests:
  - `src-tauri/src/commands_reports.rs:3262-3839`
- async wrapper tests:
  - `src-tauri/src/commands_reports_api.rs:225-372`
- active workbook contract suite:
  - `src-tauri/tests/validation_xlsx_report_tests.rs:323-915`
- frontend export orchestration:
  - `web/src/services/validationService.test.ts:2008-3458`

Missing verification worth calling out:

- no negative-path test that proves artifact-backed export rejects a non-run artifact kind
- no direct coverage for the warning branches in `save_report_xlsx_from_artifact_blocking`
- no explicit boundary tests for `max_rows = None` versus `0`
- no successful OS-shell integration test for `open_path` / `reveal_path`
- no full frontend -> Tauri -> real Rust end-to-end report export test

Recommended action:

- add the negative-path backend tests now if this file is changed
- keep the true end-to-end desktop flow as manual verification unless the repo later adds a desktop integration harness for report export

### Perf / Allocation / Export Concerns

#### 8. The fallback build path duplicates workbook bytes in memory, but this is not currently a benchmark-critical problem

Objective observation:

- `build_report_xlsx_blocking` builds workbook bytes in memory and then base64-encodes them:
  - `src-tauri/src/commands_reports.rs:51-59`
- the frontend only uses that branch after save-to-disk export fails:
  - `web/src/services/validationService.ts:2666-2693`
- `rust_xlsxwriter` offers lower-memory worksheet modes and writer-oriented save APIs:
  - <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/workbook/struct.Workbook.html>

Impact:

- large reports will transiently allocate both workbook bytes and base64 output
- however, this is a fallback/export path, not a hot optimization or validation lane

Recommended action:

- do not optimize this blindly
- if large-report export becomes a real issue, evaluate low/constant-memory modes or a different fallback transport only with measurement and workbook-feature validation

Sensitivity:

- benchmark-sensitive
- do not touch yet

## Recommended Action

- Overall recommendation: `Small surgical refactor`
- Accept as-is:
  - the active validation-report workbook path itself is well covered and should not be broadly rewritten
- Safe-now changes:
  - validate artifact kind before artifact-backed export mutates a saved file
  - add negative tests for malformed/non-run artifact kinds and save-back warning branches
  - add module-level docs clarifying active path versus retained legacy builders
- Defer:
  - legacy builder extraction/removal
  - naming churn on the public command surface
  - workbook memory-mode changes

## Exact Local Verification Required If Changed

1. Run `cargo test -p ta_desktop_demo_app commands_reports -- --nocapture`.
   - This is the direct Rust lane for the report coordination cluster.
2. Run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture`.
   - This is the workbook contract suite for the active validation XLSX product path.
3. Run `npm --prefix ./web run test -- src/services/validationService.test.ts`.
   - This verifies frontend export branching, fallback build behavior, and report open/reveal handling.
4. Run `npm --prefix ./web run test -- src/stores/commandActionStore.test.ts src/tauri/commands.test.ts`.
   - This verifies command registration plus adjacent latest-report open flows.
5. If `open_path`, `reveal_path`, or reports-dir behavior changed, do one manual desktop spot check on the host OS.
6. If artifact-backed export changed, do one manual persisted-run export and confirm both:
   - the XLSX is written successfully
   - `report_xlsx_path` is updated in the saved artifact

Drafting spot-check status:

- `cargo test -p ta_desktop_demo_app commands_reports -- --nocapture` passed with `41` tests during packet preparation
- sub-review verification also reported passes for:
  - `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture`
  - `npm --prefix ./web run test -- src/services/validationService.test.ts`
  - `npm --prefix ./web run test -- src/stores/commandActionStore.test.ts src/tauri/commands.test.ts`

## Cross-File Coordination

- `src-tauri/src/commands_reports_api.rs`
  - wraps these blocking functions as Tauri commands
- `src-tauri/src/commands_reports_validation_common.rs`
  - owns the live validation-report model/selection semantics
- `src-tauri/src/commands_reports_validation_xlsx.rs`
  - owns the live workbook layout/output path
- `src-tauri/src/commands_reports_shared.rs`
  - owns path/open/reveal/sanitize helpers used here
- `src-tauri/src/run_artifacts.rs`
  - owns path validation and artifact save-back behavior
- `src-tauri/src/commands_meta.rs`
  - depends on shared helpers and meta workbook generation
- `src-tauri/src/bin/ta_headless_validate.rs`
  - depends on the headless workbook export helper
- `web/src/services/validationService.ts`
  - decides when this file is used for save-to-disk export versus fallback base64 build

## Code-Quality and Debt Register

- active runtime ownership and retained historical builders are mixed in one large file
- `commands_reports.rs` is no longer the actual Tauri command layer even though the name still suggests that ownership
- shared helpers such as `quantile` and `sanitize_filename_component` have drifted into broader backend utility roles
- test fixtures and CSV helpers are partially duplicated across the report modules

## Performance Opportunity Register

- Possible future opportunity:
  - evaluate `rust_xlsxwriter` low/constant-memory modes if report size becomes a real memory issue
- Constraints:
  - the active path is chart-heavy and currently verified through workbook-content tests
  - changing workbook construction mode is not a safe-now cleanup item
- Not recommended now:
  - transport or writer-path rewrites without measurement

## Implementation Staging

1. Hardening stage
   - validate artifact kind in artifact-backed export
   - add negative tests for malformed artifacts and warning branches
2. Clarity stage
   - add module docs clarifying active builder path versus retained legacy builders
3. Deferred structural stage
   - isolate or archive legacy workbook builders once the team intentionally decides how to preserve comparison history

## Change Blast Radius

- Backend:
  - report export wrappers
  - artifact-backed export persistence
  - headless validation XLSX export
  - meta report helper reuse
- Frontend:
  - validation export flows
  - report open/reveal behavior
  - settings-side reports-directory open
- Stored artifacts:
  - `report_xlsx_path` persistence on saved run artifacts

## Observability Gaps

- there is no explicit metric/log surface for artifact save-back warnings
- there is no automated proof of successful OS shell open/reveal behavior
- there is no end-to-end desktop export test spanning frontend -> Tauri -> real Rust workbook output

## Open Questions and Assumptions

- Assumption:
  - retained legacy builders are being kept intentionally for workbook-history comparison, not by accident
- Assumption:
  - artifact-backed export is intended only for canonical run artifacts under `runs/`
- Open question:
  - whether the team wants the current generic “report” naming to remain for compatibility, or eventually wants explicit “validation report” naming once contract churn is acceptable
