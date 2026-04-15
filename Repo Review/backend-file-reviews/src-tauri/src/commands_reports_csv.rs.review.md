# Review Packet: `src-tauri/src/commands_reports_csv.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_csv.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful and narrowly scoped, but not yet robust enough to treat as a fully hardened export-format boundary`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Export-format-sensitive`, `Spreadsheet-consumer-sensitive`, `Manual-serializer-sensitive`, `Validation-contract-dependent`

## What This File Owns

`commands_reports_csv.rs` owns the validation-shortlist CSV text builder.

This file owns:

- manual CSV field escaping:
  - `csv_escape`: `src-tauri/src/commands_reports_csv.rs:4-10`
- the blocking backend wrapper used by the report façade:
  - `build_validation_shortlist_csv_blocking`: `src-tauri/src/commands_reports_csv.rs:12-16`
- the actual in-memory serializer from validation model to fixed 28-column CSV text:
  - `build_validation_shortlist_csv_text`: `src-tauri/src/commands_reports_csv.rs:18-95`

This file does not own:

- shortlist semantics, candidate ordering, or fallback row construction
  - delegated to `src-tauri/src/commands_reports_validation_common.rs:2637-2728`
- the public Tauri command surface
  - delegated to `src-tauri/src/commands_reports_api.rs:23-30`
- file download behavior in the browser
  - delegated to `web/src/services/validationService.ts:2531-2543`

## Classification

- Public surface: indirect only
  - this file is a private submodule behind `src-tauri/src/commands_reports.rs:7`
- Hot path: no
  - it runs only on shortlist CSV export requests
- Runtime-critical: low to moderate
  - the output is user-visible and part of the validation review workflow
- Benchmark-sensitive: low
  - this is one-shot string serialization, not a runtime compute lane
- CUDA/runtime-critical: no
- Strategy-local: no
- Low-risk support code only: mostly yes, but it is still an output-format boundary and therefore deserves standards review

## Direct Interactions

### Backend owners this file depends on

- validation export model construction:
  - `src-tauri/src/commands_reports_validation_common.rs:2637-2728`
- reports façade:
  - `src-tauri/src/commands_reports.rs:38-41`
- Tauri wrapper:
  - `src-tauri/src/commands_reports_api.rs:23-30`

### Backend tests that already depend on this file

- direct module tests:
  - `src-tauri/src/commands_reports_csv.rs:98-244`
- parent-module smoke:
  - `src-tauri/src/commands_reports.rs:3262-3290`

### Frontend contract anchors

- shortlist export command invocation:
  - `web/src/services/validationService.ts:2531-2543`
- validation service tests:
  - `web/src/services/validationService.test.ts:2925-2941`
  - `web/src/services/validationService.test.ts:3076-3090`
- store dispatch coverage:
  - `web/src/stores/validationStore.actions.test.ts:150-158`

## Standards and Research Log

This packet was written after a targeted standards review focused on CSV-format correctness, spreadsheet-consumer safety, and whether a manual serializer remains justified for this backend export boundary.

### Sources consulted

1. RFC 4180: Common Format and MIME Type for CSV Files
   - Source: <https://datatracker.ietf.org/doc/html/rfc4180>
   - Why it was consulted:
     - this file is a hand-rolled CSV serializer
   - Standard informed:
     - CSV escaping and record terminators should be assessed against the common CSV baseline rather than repo habit

2. OWASP CSV Injection
   - Source: <https://owasp.org/www-community/attacks/CSV_Injection>
   - Why it was consulted:
     - this exporter produces a user-downloaded `.csv` intended for spreadsheet tools
   - Standard informed:
     - CSV syntax escaping is not the same thing as spreadsheet-safety

3. Rust `csv` crate docs
   - Source: <https://docs.rs/csv/latest/csv/>
   - Why it was consulted:
     - the file currently uses a manual serializer instead of a library writer
   - Standard informed:
     - Rust has a mature standard CSV reader/writer surface, so manual serialization should justify itself by simplicity and stability rather than inertia

## Applicable Current Standards

- CSV-conformance standard:
  - field quoting must remain correct even when records contain commas, quotes, or embedded newlines
- Spreadsheet-safety standard:
  - spreadsheet-facing CSV exports should distinguish between valid CSV syntax and safe spreadsheet interpretation
- Manual-serializer standard:
  - if serialization is hand-rolled, tests must be strong enough to validate the exact cases the serializer claims to support
- Contract-clarity standard:
  - shortlist CSV output is a fixed-schema export boundary and should not rely on fragile duplicated test parsing

## Findings

### Correctness / Safety

#### 1. The current tests do not actually validate full-record correctness for newline-bearing fields

Objective issue:

- the serializer explicitly supports embedded newline characters by quoting such fields:
  - `src-tauri/src/commands_reports_csv.rs:4-10`
- but the direct tests inspect generated CSV using `.lines()`:
  - `src-tauri/src/commands_reports_csv.rs:169-174`
  - `src-tauri/src/commands_reports_csv.rs:207-208`
- the parent-module smoke test does the same:
  - `src-tauri/src/commands_reports.rs:3278-3283`

Impact:

- a valid quoted multiline field would break the test harness even if the CSV output were correct
- current tests therefore do not protect the real contract for newline-bearing `approval_reason`, `warning_flags`, `fail_reasons`, or future fields

Smallest justified fix:

- replace `.lines()`-based assertions with record-aware parsing in tests
- add at least one explicit multiline-field case that validates complete-record integrity

Sensitivity:

- safe-now

### API and Naming

#### 2. No API/naming bug is evident; the file name matches the owned behavior

Objective observation:

- the file is narrowly and truthfully scoped to shortlist CSV serialization
- the public Tauri command name `build_validation_shortlist_csv` accurately reflects the behavior exposed through this module chain

Recommended action:

- accept as-is

### Error Handling

#### 3. Negative-path coverage for invalid payloads and empty-row cases is missing

Objective issue:

- `build_validation_shortlist_csv_text` can fail when:
  - model construction rejects the payload: `src-tauri/src/commands_reports_csv.rs:21`
  - row selection yields no rows: `src-tauri/src/commands_reports_csv.rs:28-30`
- existing direct tests cover only happy-path behavior:
  - `src-tauri/src/commands_reports_csv.rs:132-244`

Impact:

- the failure contract exists, but it is not directly protected
- a regression in error wording or edge-case handling could land unnoticed

Smallest justified fix:

- add one invalid-payload test
- add one empty-row/no-candidate test

Sensitivity:

- safe-now

### Module Boundaries / Tech Debt

#### 4. The CSV boundary is hand-rolled and duplicated instead of centralized

Objective issue:

- runtime escaping is custom:
  - `src-tauri/src/commands_reports_csv.rs:4-10`
  - `src-tauri/src/commands_reports_csv.rs:35-92`
- CSV test parsing is also custom in this module:
  - `src-tauri/src/commands_reports_csv.rs:103-130`
- a near-duplicate parser exists in the parent-module tests:
  - `src-tauri/src/commands_reports.rs:3145-3174`
- the repo already uses the `csv` crate elsewhere:
  - `crates/app_core/src/csv_compat.rs:3`
  - `crates/app_core/src/options_data_store.rs:6`
  - `crates/app_core/src/ml_signal.rs:12`

Impact:

- escaping rules, record handling, and test parsing can drift independently
- the manual path is still small enough to understand, but it now carries avoidable boundary debt

Recommended action:

- do not rewrite blindly
- if this file is changed, either:
  - centralize CSV parsing/writing helpers locally, or
  - migrate to `csv::Writer`/record-aware parsing deliberately with output-contract verification

Sensitivity:

- safe-now if touched locally
- output-format-sensitive if migrating to a library writer

### Docs / Examples

#### 5. The fixed-schema export contract is tested, but not documented

Objective issue:

- the 28-column header contract is asserted in tests:
  - `src-tauri/src/commands_reports_csv.rs:190-242`
- but there is no code comment explaining:
  - why these columns exist
  - why shortlist rows are preferred over all rows
  - why this exporter is intentionally manual

Impact:

- future contributors must reverse-engineer intent from tests and downstream UI behavior

Smallest justified fix:

- add a short comment above `build_validation_shortlist_csv_text` describing:
  - the fixed column contract
  - the shortlist-vs-all-row fallback
  - any intentional compatibility constraint

Sensitivity:

- safe-now

### Tests / Missing Verification

#### 6. The shortlist selection contract is only partially covered

Objective issue:

- the direct tests prove fallback from `shortlist_rows` to `all_rows`:
  - `src-tauri/src/commands_reports_csv.rs:140-188`
- but there is no direct test proving a non-empty `shortlist_rows` is preferred:
  - relevant choice point: `src-tauri/src/commands_reports_csv.rs:21-26`
- there is also no direct CSV-focused coverage for the fixed `4096` model cap request:
  - `src-tauri/src/commands_reports_csv.rs:21`
  - `src-tauri/src/commands_reports_validation_common.rs:2656`

Impact:

- the visible shortlist CSV semantics depend on upstream model behavior plus this file’s row-selection choice, but only half of that contract is directly tested

Smallest justified fix:

- add one test where `shortlist_rows` is non-empty and clearly differs from `all_rows`
- add a cap-boundary test only if the export limit becomes user-visible or likely to change

Sensitivity:

- safe-now for shortlist-preference coverage
- defer cap tests unless the limit is touched

### Perf / Allocation / Spreadsheet Concerns

#### 7. The exporter is not spreadsheet-safe against formula injection, but changing that behavior is contract-sensitive

Objective issue:

- `csv_escape` only handles CSV syntax characters:
  - `src-tauri/src/commands_reports_csv.rs:4-10`
- the resulting CSV is downloaded directly for user spreadsheet consumption:
  - `web/src/services/validationService.ts:2537-2540`
- OWASP guidance distinguishes CSV syntax correctness from spreadsheet-safety when cells begin with formula-significant prefixes

Impact:

- if any exported string field ever begins with `=`, `+`, `-`, `@`, or similar spreadsheet-significant prefixes, spreadsheet software may interpret it as a formula
- that is a real boundary concern, even though current exported values are mostly low-risk

Recommended action:

- do not change cell contents without product agreement
- if spreadsheet safety becomes a requirement, define and document an escaping policy explicitly and add fixture coverage for it

Sensitivity:

- semantics-sensitive
- do not touch yet without explicit contract review

#### 8. Records are emitted with `\n` only rather than RFC-style `\r\n`

Objective observation:

- the file joins records with `\n`:
  - `src-tauri/src/commands_reports_csv.rs:95`

Impact:

- this is widely tolerated
- but it is not the common RFC 4180 line terminator and therefore should be treated as an output-format choice, not an invisible implementation detail

Recommended action:

- document current behavior or change it deliberately, not casually

Sensitivity:

- output-format-sensitive
- defer unless there is an interoperability reason

## Recommended Action

- Overall recommendation: `Small surgical refactor`
- Safe-now changes:
  - replace `.lines()`-based CSV assertions with record-aware parsing
  - add negative tests for invalid payload / empty-row cases
  - add a direct test proving shortlist rows are preferred when present
  - add a short contract comment above the serializer
- Defer:
  - spreadsheet-formula mitigation
  - line-terminator changes
  - broader migration to `csv::Writer` unless the file is already being touched for a real reason

## Exact Local Verification Required If Changed

1. Run `cargo test -p ta_desktop_demo_app commands_reports_csv -- --nocapture`.
   - Direct leaf-module coverage.
2. Run `cargo test -p ta_desktop_demo_app validation_shortlist_csv_smoke -- --nocapture`.
   - Verifies the parent reports façade still exercises the CSV builder correctly.
3. Run `cargo test -p ta_desktop_demo_app commands_reports_api -- --nocapture`.
   - Verifies the async Tauri wrapper still matches the blocking CSV path.
4. Run `npm --prefix ./web run test -- src/services/validationService.test.ts`.
   - Verifies frontend shortlist export orchestration and status behavior.
5. Run `npm --prefix ./web run test -- src/stores/validationStore.actions.test.ts`.
   - Light wiring coverage for store dispatch.
6. If output content or escaping changes, do one manual desktop shortlist export and inspect the downloaded file in a spreadsheet-aware viewer.

Drafting spot-check status:

- `cargo test -p ta_desktop_demo_app commands_reports_csv -- --nocapture` passed with `3` tests
- `cargo test -p ta_desktop_demo_app validation_shortlist_csv_smoke -- --nocapture` passed with `1` test
- `cargo test -p ta_desktop_demo_app commands_reports_api -- --nocapture` passed with `8` tests
- `npm --prefix ./web run test -- src/stores/validationStore.actions.test.ts` passed with `8` tests
- `npm --prefix ./web run test -- src/services/validationService.test.ts` had already passed earlier in this review run with `48` tests

## Cross-File Coordination

- `src-tauri/src/commands_reports_validation_common.rs`
  - owns row shaping, shortlist semantics, and caps this serializer consumes
- `src-tauri/src/commands_reports.rs`
  - re-exports the blocking builder and uses it in parent-module tests
- `src-tauri/src/commands_reports_api.rs`
  - exposes the async Tauri command surface
- `web/src/services/validationService.ts`
  - converts returned CSV text into a browser download

## Code-Quality and Debt Register

- manual CSV writing and manual CSV test parsing live side by side
- parent-module tests duplicate CSV parsing logic
- the fixed header contract is test-anchored but not documented inline

## Performance Opportunity Register

- there is no meaningful performance problem here today
- if the file is ever reworked, prefer correctness and boundary clarity over micro-optimization

## Implementation Staging

1. Test-hardening stage
   - replace `.lines()` CSV assertions with record-aware parsing
   - add missing negative and shortlist-preference tests
2. Clarity stage
   - document the fixed-schema export contract
3. Deferred output-policy stage
   - decide explicitly whether spreadsheet-safety and CRLF normalization are product requirements

## Change Blast Radius

- backend shortlist CSV generation
- Tauri shortlist export command behavior
- validation export UX and downloaded shortlist files

## Observability Gaps

- frontend tests mock the CSV payload and do not validate the real Rust-generated file contents end-to-end
- there is no automated spreadsheet-consumer check

## Open Questions and Assumptions

- Assumption:
  - the fixed 28-column header is intended as a stable shortlist export contract
- Open question:
  - whether shortlist CSV is expected to be spreadsheet-safe by policy, or only syntactically valid CSV
