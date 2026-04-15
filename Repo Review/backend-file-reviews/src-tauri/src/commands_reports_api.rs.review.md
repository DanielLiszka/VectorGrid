# Review Packet: `src-tauri/src/commands_reports_api.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_api.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally sound, but not yet explicit enough to count as a polished public Tauri command boundary`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Async-runtime-sensitive`, `Binary-transport-sensitive`, `Cross-feature-utility-drift`

## What This File Owns

`commands_reports_api.rs` is the actual Tauri command boundary for report export and related file-opening operations.

This file owns:

- the serialized frontend response DTO:
  - `SaveReportXlsxResponse`: `src-tauri/src/commands_reports_api.rs:6-12`
- public `#[tauri::command]` wrappers for:
  - fallback workbook build: `src-tauri/src/commands_reports_api.rs:14-21`
  - shortlist CSV build: `src-tauri/src/commands_reports_api.rs:23-30`
  - save report from live report payload: `src-tauri/src/commands_reports_api.rs:32-50`
  - save report from persisted artifact: `src-tauri/src/commands_reports_api.rs:53-71`
  - open reports directory: `src-tauri/src/commands_reports_api.rs:74-105`
  - reveal arbitrary path: `src-tauri/src/commands_reports_api.rs:107-112`
  - open arbitrary path: `src-tauri/src/commands_reports_api.rs:114-119`
- the async-to-blocking handoff through `tauri::async_runtime::spawn_blocking`
- the Tauri-specific `AppHandle` seam for locating the app-local reports directory:
  - `src-tauri/src/commands_reports_api.rs:79-99`

This file does not own:

- report generation logic
  - delegated to `src-tauri/src/commands_reports.rs`
- workbook model/layout logic
  - delegated to `src-tauri/src/commands_reports_validation_common.rs`
  - and `src-tauri/src/commands_reports_validation_xlsx.rs`
- path validation/open/reveal implementation details
  - delegated through `src-tauri/src/commands_reports.rs` into `src-tauri/src/commands_reports_shared.rs`

## Classification

- Public surface: yes
  - command functions are re-exported from `src-tauri/src/commands.rs:136-138`
  - and registered in `src-tauri/src/lib.rs:126-133`
- Hot path: no
  - this is a desktop IPC boundary for export and shell-open operations
- Runtime-critical: moderate
  - every desktop invoke for report export passes through this file
- Benchmark-sensitive: low to moderate
  - only the binary fallback command can become memory-heavy, and that path is a fallback rather than the default export route
- CUDA/runtime-critical: no
- Strategy-local: no
- Low-risk support code only: no
  - this is the canonical public boundary for these commands

## Direct Interactions

### Backend owners this file depends on

- blocking report/export implementation:
  - `src-tauri/src/commands_reports.rs:22-250`
- crate command-surface assembly:
  - `src-tauri/src/commands.rs:136-138`
- Tauri invoke registration:
  - `src-tauri/src/lib.rs:126-133`

### Frontend contract anchors

- command allowlist:
  - `web/src/tauri/commands.ts:46-53`
  - `web/src/tauri/commands.test.ts:20-24`
- main validation export orchestration:
  - `web/src/services/validationService.ts:2537-2540`
  - `web/src/services/validationService.ts:2641-2693`
  - `web/src/services/validationService.ts:2721`
  - `web/src/services/validationService.ts:2814`
- settings-side reports-dir open:
  - `web/src/services/settingsService.ts:15-16`

### Cross-feature consumers of the supposedly report-local shell commands

- data service:
  - `web/src/services/dataService.ts:474-478`
- history service:
  - `web/src/services/historyService.ts:1339`
- meta service:
  - `web/src/services/metaService.ts:2399-2413`
  - `web/src/services/metaService.ts:2916-2932`
- ML signal service:
  - `web/src/services/mlSignalService.ts:130-134`
- options expression service:
  - `web/src/services/optionsExpressionService.ts:1426-1430`

### Test anchors

- direct wrapper tests:
  - `src-tauri/src/commands_reports_api.rs:121-372`
- frontend orchestration:
  - `web/src/services/validationService.test.ts:2008-3458`
- adjacent latest-report open flows:
  - `web/src/stores/commandActionStore.test.ts:138-254`

## Standards and Research Log

This packet was written after a targeted official-source review focused on Tauri command exposure, argument/return contracts, binary return guidance, and Tokio blocking-task semantics for public IPC boundaries.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file is a pure Tauri command surface
   - Standard informed:
     - separate-module commands should be `pub`
     - command names must stay unique
     - arguments should use serializable/deserializable contract types
     - returned values are part of the frontend contract

2. Tauri returning array buffers guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - `build_report_xlsx` currently returns a large binary payload as base64 text
   - Standard informed:
     - for large binary responses, `tauri::ipc::Response` is the optimized transport instead of JSON-serialized text

3. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why it was consulted:
     - every public command in this file delegates through `spawn_blocking`
   - Standard informed:
     - `spawn_blocking` is for bounded blocking work
     - once a blocking task has started, it cannot be aborted like async work
     - excessive long-lived use can queue/saturate the blocking pool

4. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file defines a public backend contract
   - Standard informed:
     - boundary functions should validate arguments and make behavior legible

5. Tauri state management
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why it was consulted:
     - this file uses `AppHandle` and app-local path resolution at the command boundary
   - Standard informed:
     - command-layer use of app-managed runtime context is expected, but should stay intentional and well-bounded

## Applicable Current Standards

- Public-command standard:
  - command payloads and return types are frontend contracts, not private implementation details
- Binary-transport standard:
  - large binary fallback returns should distinguish between convenient JSON transport and optimized byte transport
- Blocking-work standard:
  - `spawn_blocking` is appropriate for bounded blocking work and shell operations, but it is not a substitute for cancellable async execution
- Module-boundary standard:
  - a report-specific command module should not silently become the app-wide home for unrelated shell-open utilities without that being documented

## Findings

### Correctness / Safety

#### 1. `save_report_xlsx_from_artifact` exposes the same artifact-kind contract hole at the public command boundary

Objective issue:

- the public command wrapper accepts an arbitrary path string:
  - `src-tauri/src/commands_reports_api.rs:53-71`
- it delegates into the artifact-backed export path that uses `load_artifact_value_checked_blocking` rather than the kind-checking loader:
  - `src-tauri/src/commands_reports.rs:219-250`
  - `src-tauri/src/run_artifacts.rs:2415-2419`
- a neighboring loader already proves the repo has a stricter run-artifact-kind path available:
  - `src-tauri/src/run_artifacts.rs:2422-2435`

Impact:

- the public command contract currently allows any JSON under `runs/` with a compatible validation shape to be treated as exportable and then mutated with `report_xlsx_path`
- that is a public-boundary hardening gap, not just an internal implementation detail

Smallest justified fix:

- ensure artifact-backed export validates a recognized run-artifact `kind` before proceeding
- add a negative wrapper test proving unsupported artifact kinds are rejected

Sensitivity:

- safe-now
- public-surface-sensitive

#### 2. No direct `spawn_blocking` correctness bug is evident, and `spawn_blocking` is the right primitive here

Objective observation:

- all public commands delegate blocking work through `spawn_blocking`:
  - `src-tauri/src/commands_reports_api.rs:16-20`
  - `src-tauri/src/commands_reports_api.rs:25-29`
  - `src-tauri/src/commands_reports_api.rs:46-50`
  - `src-tauri/src/commands_reports_api.rs:67-71`
  - `src-tauri/src/commands_reports_api.rs:82-84`
  - `src-tauri/src/commands_reports_api.rs:109-111`
  - `src-tauri/src/commands_reports_api.rs:116-118`

Impact:

- this is correct for report generation, file I/O, and OS shell-open operations
- no immediate correctness fix is justified here

Recommended action:

- accept the concurrency primitive as-is
- keep the missing join-failure coverage noted under tests

Sensitivity:

- accept as-is

### API and Naming

#### 3. `build_report_xlsx` exposes an opaque base64 transport contract that the name and type do not reveal

Objective issue:

- the command returns `Result<String, String>`:
  - `src-tauri/src/commands_reports_api.rs:15-21`
- the delegated implementation returns base64-encoded workbook bytes rather than a filesystem path or typed binary wrapper:
  - `src-tauri/src/commands_reports.rs:51-59`
- the frontend treats that command specifically as a fallback download path:
  - `web/src/services/validationService.ts:2666-2693`
- Tauri’s own guidance points to `tauri::ipc::Response` for optimized binary returns

Impact:

- the contract works today, but it is only obvious if the caller already knows the implementation detail
- callers see `String` and `build_report_xlsx`, not “base64 fallback transport for XLSX bytes”

Smallest justified fix:

- add command-level docs/comments that explicitly state:
  - this is a fallback transport
  - the returned `String` is base64-encoded XLSX content
- consider `tauri::ipc::Response` only if this fallback path becomes operationally important

Sensitivity:

- safe-now to document
- semantics-sensitive / transport-sensitive to change

#### 4. The API layer uses raw `serde_json::Value` payloads for public commands, which keeps the contract looser than necessary

Objective issue:

- public command inputs are untyped JSON values for report and validation payloads:
  - `src-tauri/src/commands_reports_api.rs:15`
  - `src-tauri/src/commands_reports_api.rs:24`
  - `src-tauri/src/commands_reports_api.rs:34`
  - `src-tauri/src/commands_reports_api.rs:55`
- Tauri command arguments can be any `serde::Deserialize` type

Impact:

- boundary validation is deferred deeper into the stack
- the frontend/backend contract is harder to discover and harder to statically evolve

Recommended action:

- do not force a DTO refactor just for style
- if this surface changes materially, prefer typed request DTOs at the API layer rather than expanding `Value`-shaped contracts further

Sensitivity:

- larger refactor but defer

#### 5. `open_path` and `reveal_path` are now generic app-wide shell commands living in a report-specific module

Objective issue:

- the commands are exposed from this report API file:
  - `src-tauri/src/commands_reports_api.rs:107-119`
- but unrelated frontend domains use them as general utilities:
  - `web/src/services/dataService.ts:474-478`
  - `web/src/services/historyService.ts:1339`
  - `web/src/services/metaService.ts:2399-2413`
  - `web/src/services/mlSignalService.ts:130-134`
  - `web/src/services/optionsExpressionService.ts:1426-1430`

Impact:

- module ownership has drifted beyond report export
- future contributors looking for generic shell-open behavior would not naturally start in a report API module

Smallest justified fix:

- document this drift now
- move these generic commands into a non-report boundary only when the team is ready to touch the public command surface deliberately

Sensitivity:

- safe-now to document
- semantics-sensitive to relocate

### Error Handling

#### 6. Join failures and downstream failures are intentionally flattened to `String`, but the command layer has thin diagnostic shape

Objective issue:

- every wrapper maps `JoinError` to `String` through the same pattern:
  - `src-tauri/src/commands_reports_api.rs:19-20`
  - `src-tauri/src/commands_reports_api.rs:28-29`
  - `src-tauri/src/commands_reports_api.rs:49-50`
  - `src-tauri/src/commands_reports_api.rs:70-71`
  - `src-tauri/src/commands_reports_api.rs:83-84`
  - `src-tauri/src/commands_reports_api.rs:110-111`
  - `src-tauri/src/commands_reports_api.rs:117-118`

Impact:

- this matches common Tauri command practice and is acceptable
- however, the surface preserves little structured context if a blocking task panics or the join fails

Recommended action:

- do not redesign the error type just for this
- if future callers need richer diagnostics, add explicit context strings per command rather than a broad custom error hierarchy

Sensitivity:

- document only

### Module Boundaries / Tech Debt

#### 7. `SaveReportXlsxResponse` is defined in the Tauri wrapper layer, but the lower-level report implementation depends on it

Objective issue:

- the response DTO is defined in this wrapper module:
  - `src-tauri/src/commands_reports_api.rs:6-12`
- but the lower-level report implementation imports it from `super` and returns it from blocking functions:
  - `src-tauri/src/commands_reports.rs:1`
  - `src-tauri/src/commands_reports.rs:83-107`
  - `src-tauri/src/commands_reports.rs:219-250`

Impact:

- the dependency direction is inverted
- the implementation layer depends on a type owned by the Tauri wrapper file, which makes the boundary less clean than it should be

Smallest justified fix:

- if this export surface is touched, move `SaveReportXlsxResponse` into a neutral shared reports module or crate command-types module
- do not churn it purely for aesthetics

Sensitivity:

- safe-now if already editing this boundary
- module-boundary-sensitive

#### 8. The file contains a mix of public commands and private test seams, which is justified, but it makes ownership easy to underread

Objective observation:

- private helpers such as `save_report_xlsx_with_app`, `save_report_xlsx_from_artifact_with_app`, and `open_reports_dir_blocking_with_revealer` exist primarily to keep the command layer testable:
  - `src-tauri/src/commands_reports_api.rs:41-50`
  - `src-tauri/src/commands_reports_api.rs:62-71`
  - `src-tauri/src/commands_reports_api.rs:87-105`
- the tests rely on those seams directly:
  - `src-tauri/src/commands_reports_api.rs:248-318`
  - `src-tauri/src/commands_reports_api.rs:321-372`

Impact:

- this is good testability, not a bug
- but it reinforces that the file owns the runtime wrapper boundary, not the report logic itself

Recommended action:

- accept as-is
- clarify ownership in code comments if this module is touched

### Docs / Examples

#### 9. The command-level contracts are effectively undocumented

Objective issue:

- there are no rustdoc comments on the public command functions
- the frontend must infer behavior from implementation and tests

Impact:

- callers cannot tell from the Rust surface alone:
  - which command returns a saved path versus base64 content
  - which command is validation-export-specific
  - that `open_reports_dir` reveals/opens the reports directory via shell behavior

Smallest justified fix:

- add short rustdoc comments on each public command and on `SaveReportXlsxResponse`

Sensitivity:

- safe-now

### Tests / Missing Verification

#### 10. Coverage is solid for wrapper parity and happy-path orchestration, but not full runtime integration

Current strong coverage:

- direct async wrapper tests:
  - `src-tauri/src/commands_reports_api.rs:225-372`
- downstream workbook contract tests:
  - `src-tauri/tests/validation_xlsx_report_tests.rs:324-915`
- frontend command-routing/orchestration tests:
  - `web/src/services/validationService.test.ts:2008-3458`
  - `web/src/stores/commandActionStore.test.ts:138-254`
  - `web/src/tauri/commands.test.ts:20-24`

Important gaps:

- no end-to-end frontend invoke -> live Tauri handler -> real Rust integration test
- no join-failure test for the `spawn_blocking` wrappers
- no automated proof of successful OS shell-open behavior
- artifact-backed async tests only cover the clean happy path
- no negative wrapper test for unsupported artifact kinds at the public command layer

Recommended action:

- keep the current test stack
- if this file changes, add only targeted boundary tests rather than broad harness churn

### Perf / Allocation / Binary Transport Concerns

#### 11. The fallback binary command pays JSON/base64 overhead, but that overhead is not on the primary export path

Objective observation:

- `build_report_xlsx` returns stringified binary content
- Tauri documents a more direct binary return path
- the frontend reaches this only after save-to-disk export fails:
  - `web/src/services/validationService.ts:2666-2693`

Impact:

- the current design is acceptable as a compatibility fallback
- it is not the right area for speculative performance work without evidence that users actually hit the fallback often or with large reports

Recommended action:

- benchmark-sensitive; do not touch yet

## Recommended Action

- Overall recommendation: `Small surgical refactor`
- Safe-now changes:
  - validate run artifact kind for `save_report_xlsx_from_artifact`
  - add a negative wrapper test for unsupported artifact kinds
  - add rustdoc/contract comments for the public commands and `SaveReportXlsxResponse`
  - explicitly document the base64 fallback contract of `build_report_xlsx`
  - explicitly document that `open_path` / `reveal_path` are shared shell-open utilities despite living in the report API module
  - if this boundary is already being edited, move `SaveReportXlsxResponse` out of the wrapper module to remove the dependency inversion
- Accept as-is:
  - `spawn_blocking` usage for this wrapper layer
  - current private helper seams that make the module testable
- Defer:
  - moving generic shell-open commands into a non-report module
  - replacing the base64 fallback with `tauri::ipc::Response`
  - introducing typed DTOs for all command payloads

## Exact Local Verification Required If Changed

1. Run `cargo test -p ta_desktop_demo_app commands_reports_api -- --nocapture`.
   - This is the direct wrapper-layer lane.
2. Run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture`.
   - This verifies the downstream workbook contract behind the wrappers.
3. Run `npm --prefix ./web run test -- src/services/validationService.test.ts src/stores/commandActionStore.test.ts src/tauri/commands.test.ts`.
   - This verifies command routing and report/open UX behavior from the frontend side.
4. If `open_reports_dir`, `open_path`, or `reveal_path` change, do one manual desktop shell-open check on the host OS.

Drafting spot-check status:

- `cargo test -p ta_desktop_demo_app commands_reports_api -- --nocapture` passed with `8` tests
- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture` passed with `15` tests
- `npm --prefix ./web run test -- src/services/validationService.test.ts src/stores/commandActionStore.test.ts src/tauri/commands.test.ts` passed with `60` tests

## Cross-File Coordination

- `src-tauri/src/commands_reports.rs`
  - owns the blocking implementation this file exposes
- `src-tauri/src/commands.rs`
  - re-exports this command surface
- `src-tauri/src/lib.rs`
  - registers the Tauri commands
- `web/src/services/validationService.ts`
  - primary consumer of the report-export commands
- `web/src/services/settingsService.ts`
  - consumes `open_reports_dir`
- generic cross-feature consumers of `open_path` / `reveal_path`:
  - `web/src/services/dataService.ts`
  - `web/src/services/metaService.ts`
  - `web/src/services/mlSignalService.ts`
  - `web/src/services/optionsExpressionService.ts`
  - `web/src/services/historyService.ts`

## Code-Quality and Debt Register

- command contracts are mostly discoverable only through tests and frontend usage
- report-specific module naming under-describes the true ownership of generic shell-open commands
- the wrapper layer is intentionally thin, but input contracts remain looser than necessary because of `Value` payloads

## Performance Opportunity Register

- Possible future opportunity:
  - use `tauri::ipc::Response` for binary fallback if fallback export becomes important enough to justify transport churn
- Not recommended now:
  - transport rewrites without evidence that the fallback path is a meaningful operational cost

## Implementation Staging

1. Clarity stage
   - add rustdoc/comments for the public commands and response DTO
2. Boundary stage
   - if the API surface changes, introduce typed DTOs selectively instead of expanding `Value` usage
3. Deferred structural stage
   - relocate generic shell-open commands only if the team deliberately wants to clean up the public command map

## Change Blast Radius

- frontend validation export flows
- settings-side reports-dir behavior
- latest-report open/reveal flows
- all app features that reuse `open_path` / `reveal_path`

## Observability Gaps

- no end-to-end invoke-path test against the live Tauri registration
- no automated proof of successful shell integration on the host OS
- no explicit join-failure coverage for the blocking-task wrappers

## Open Questions and Assumptions

- Assumption:
  - the current base64 fallback is intentionally kept because the primary product path is save-to-disk first
- Open question:
  - whether the team wants generic shell-open commands to remain in this report API module for compatibility, or eventually wants a shared non-report boundary for them
