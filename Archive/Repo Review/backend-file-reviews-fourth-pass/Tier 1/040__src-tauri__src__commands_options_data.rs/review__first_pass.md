# Review Packet: `src-tauri/src/commands_options_data.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_options_data.rs`
- Packet status: `Reviewed`
- Public-review readiness: `High-value workflow boundary, but not yet clean enough to accept as-is`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Filesystem-contract-sensitive`, `Network-provider-sensitive`, `Options-workflow-critical`

## What This File Owns

`commands_options_data.rs` is the canonical Tauri/backend boundary for options dataset ingestion and dataset discovery in the current options workflow.

This file owns:

- public Tauri commands for:
  - CSV import into the managed options dataset store: `src-tauri/src/commands_options_data.rs:1076-1087`
  - dataset listing: `src-tauri/src/commands_options_data.rs:1089-1100`
  - single-manifest load: `src-tauri/src/commands_options_data.rs:1102-1123`
  - Polygon/Massive bounded dataset download: `src-tauri/src/commands_options_data.rs:1133-1145`
- public headless helpers for CLI/runtime reuse:
  - `import_options_dataset_blocking_headless`: `src-tauri/src/commands_options_data.rs:1069-1074`
  - `download_polygon_options_dataset_blocking_headless`: `src-tauri/src/commands_options_data.rs:1125-1131`
- public request/response DTOs:
  - `OptionsDatasetImportRequest`: `src-tauri/src/commands_options_data.rs:110-145`
  - `PolygonOptionsDatasetDownloadRequest`: `src-tauri/src/commands_options_data.rs:147-172`
  - `PolygonOptionsDatasetDownloadResponse`: `src-tauri/src/commands_options_data.rs:174-184`
- options dataset store location and dataset-id derivation:
  - `options_datasets_dir`: `src-tauri/src/commands_options_data.rs:349-355`
  - `sanitize_dataset_id`: `src-tauri/src/commands_options_data.rs:294-300`
  - dataset-id derivation helpers: `src-tauri/src/commands_options_data.rs:302-346`
- import-side provider canonicalization defaults and aliases:
  - `src-tauri/src/commands_options_data.rs:15-108`
- the full provider download orchestration path:
  - HTTP client and retry helper: `src-tauri/src/commands_options_data.rs:497-559`
  - contract discovery: `src-tauri/src/commands_options_data.rs:561-618`
  - aggregate fetching: `src-tauri/src/commands_options_data.rs:620-681`
  - normalized CSV writing: `src-tauri/src/commands_options_data.rs:712-779`
  - dataset materialization: `src-tauri/src/commands_options_data.rs:781-977`

This file does not own:

- the core options dataset manifest schema
- CSV-to-dataset parsing and manifest serialization
- options expression execution
- frontend request shaping and dataset selection UX

Those concerns are delegated to:

- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_types.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/services/optionsExpressionService.ts`
- `web/src/stores/optionsExpressionStore.ts`

## Classification

- Public surface: yes
  - commands are re-exported from `src-tauri/src/commands.rs:113-117`
  - commands are registered in `src-tauri/src/lib.rs:101-104`
- Hot path: no
  - this is dataset preparation/discovery, not per-tick runtime logic
- Runtime-critical: moderately
  - later options-expression execution depends directly on the dataset and manifest contracts this file creates
- Benchmark-sensitive: low locally
  - most cost is external I/O, CSV handling, and provider latency rather than internal compute hot loops
- CUDA/runtime-critical: no
- Strategy-local: yes
  - this is a dedicated options workflow boundary
- Low-risk support code only: no
  - it owns filesystem layout, provider integration, and public request/result contracts

## Direct Interactions

### Backend owners this file depends on

- market-data root and store placement:
  - `src-tauri/src/settings.rs:58`
  - `src-tauri/src/commands_options_data.rs:349-355`
- Polygon/Massive cache and API helpers:
  - `src-tauri/src/polygon.rs:172`
  - `src-tauri/src/polygon.rs:368`
  - `src-tauri/src/polygon.rs:1099`
  - consumed in `src-tauri/src/commands_options_data.rs:831-850`
- app-core dataset parsing and manifest persistence:
  - `crates/app_core/src/options_data_store.rs:291`
  - `crates/app_core/src/options_data_store.rs:435-490`

### Backend files that depend on this file’s contract

- command hub and live invoke surface:
  - `src-tauri/src/commands.rs:113-117`
  - `src-tauri/src/lib.rs:101-104`
- options-expression runtime depends on the same dataset store and manifest contract:
  - `src-tauri/src/commands_options_expression.rs:652-658`
  - `src-tauri/src/commands_options_expression.rs:720-726`
  - `src-tauri/src/commands_options_expression.rs:1593-1599`
- headless CLI surfaces depend on the blocking helpers:
  - `src-tauri/src/bin/ta_options_dataset_import.rs`
  - `src-tauri/src/bin/ta_options_dataset_download.rs`
  - `src-tauri/src/bin/ta_options_expression_feasibility.rs`

### Frontend contract anchors

- typed command allowlist:
  - `web/src/tauri/commands.ts:21-24`
- service wrappers and normalization:
  - `web/src/services/optionsExpressionService.ts:613-651`
  - `web/src/services/optionsExpressionService.ts:1312-1353`
- store-level consumers:
  - `web/src/stores/optionsExpressionStore.ts:472-608`
  - `web/src/stores/optionsExpressionStore.ts:687-706`
- UI consumer:
  - `web/src/components/OptionsExpressionPanel.vue:3384-3389`

### REPO_MAP anchors

- module inventory:
  - `REPO_MAP.md:101`

## Standards and Research Log

This packet was written after a targeted official-source review focused on Tauri command boundaries, blocking HTTP-client usage under async callers, HTTP `Retry-After` semantics, and provider-facing options API expectations.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file exposes four public Tauri commands and two blocking headless entrypoints
   - Standard informed:
     - command request/response DTOs are public IPC contracts

2. reqwest blocking module docs
   - Source: <https://docs.rs/reqwest/latest/reqwest/blocking/index.html>
   - Why it was consulted:
     - the file uses `reqwest::blocking::Client` inside async command wrappers
   - Standard informed:
     - blocking reqwest belongs on a blocking thread boundary, not directly inside async runtime execution

3. reqwest blocking `ClientBuilder`
   - Source: <https://docs.rs/reqwest/latest/reqwest/blocking/struct.ClientBuilder.html>
   - Why it was consulted:
     - the file explicitly sets user-agent and timeout defaults for provider requests
   - Standard informed:
     - timeout configuration is part of the client’s I/O contract and should be intentional

4. HTTP Semantics `Retry-After`
   - Source: <https://www.ietf.org/archive/id/draft-ietf-httpbis-semantics-19.html>
   - Why it was consulted:
     - the retry helper parses `Retry-After`
   - Standard informed:
     - `Retry-After` can be either an `HTTP-date` or delay-seconds, not only an integer

5. Massive / Polygon Options REST API overview
   - Source: <https://massive.com/docs/rest/options/overview>
   - Why it was consulted:
     - this file is explicitly provider-coupled and assumes Polygon/Massive-derived options data semantics
   - Standard informed:
     - provider-specific assumptions should remain explicit rather than implicit

6. Massive / Polygon options contract overview
   - Source: <https://massive.com/docs/rest/options/contracts/contract-overview>
   - Why it was consulted:
     - the file performs contract discovery and filtering before aggregate download
   - Standard informed:
     - contract discovery rules and contract metadata assumptions are provider-facing behavior, not purely internal implementation

7. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this module is a public boundary with multiple DTOs and filesystem/network side effects
   - Standard informed:
     - public behavior should be predictable, explicit, and testable

## Applicable Current Standards

- Tauri boundary standard:
  - blocking provider/file work should stay behind `spawn_blocking`
  - request and response fields are stable frontend contracts
- Filesystem-boundary standard:
  - read-only commands should not mutate disk unless the mutation is deliberate and documented
  - dataset-id validation should happen before path construction
- Provider-request standard:
  - obvious request validation failures should be rejected before remote calls
  - retry behavior should follow HTTP semantics and remain diagnosable
- Data-integrity standard:
  - persisted manifest metadata should be normalized and validated consistently with the runtime behavior used to create it
- Product-contract standard:
  - options dataset storage layout and manifest schema must remain compatible with `commands_options_expression.rs` and the frontend options-expression workflow

## Findings

### Correctness / Safety

#### 1. The download path does not validate an explicit dataset id after sanitization

Objective issue:

- `sanitize_dataset_id` exists and rejects empty post-sanitization IDs:
  - `src-tauri/src/commands_options_data.rs:294-300`
- `derive_polygon_download_dataset_id` uses the weaker `sanitize_id_component` path instead:
  - `src-tauri/src/commands_options_data.rs:334-346`
- the download path joins the derived value directly into a path:
  - `src-tauri/src/commands_options_data.rs:820-828`

Impact:

- a punctuation-only explicit dataset id can collapse to an empty string
- the caller then receives a misleading “already exists” error against the datasets root instead of a clear invalid-id error

Smallest justified fix:

- run the derived download dataset id through `sanitize_dataset_id` before path construction

Sensitivity:

- safe-now

#### 2. `contract_reference_as_of_utc` is validated and persisted inconsistently

Objective issue:

- the download path only parses the first 10 characters of the input for runtime use:
  - `src-tauri/src/commands_options_data.rs:852-860`
- `format_contract_reference_as_of_utc` preserves the original explicit string:
  - `src-tauri/src/commands_options_data.rs:399-407`
- the import path persists `contract_reference_as_of_utc` without validation:
  - `src-tauri/src/commands_options_data.rs:1038-1055`

Impact:

- a value like `2026-04-05junk` can drive a valid `as_of` lookup while storing invalid or non-normalized manifest metadata
- runtime behavior and persisted provenance can drift

Smallest justified fix:

- validate and normalize `contract_reference_as_of_utc` once up front
- reuse that normalized value both for provider lookups and manifest persistence

Sensitivity:

- safe-now

#### 3. Invalid strike ranges are not rejected before network work starts

Objective issue:

- the request parser accepts `strike_min` and `strike_max` independently:
  - `src-tauri/src/commands_options_data.rs:147-172`
- no cross-validation occurs before contract discovery:
  - `src-tauri/src/commands_options_data.rs:786-870`
- strike checks happen only during per-contract filtering:
  - `src-tauri/src/commands_options_data.rs:684-709`

Impact:

- `strike_min > strike_max` becomes a late “no contracts matching the requested filters” failure
- the user pays provider request cost for a request that should have been rejected immediately

Smallest justified fix:

- reject inverted strike ranges before any provider calls begin

Sensitivity:

- safe-now

### Error Handling

#### 4. Read-only dataset discovery commands create directories as a side effect

Objective issue:

- `options_datasets_dir` always calls `create_dir_all`:
  - `src-tauri/src/commands_options_data.rs:349-355`
- both read-only commands route through it:
  - `list_options_datasets`: `src-tauri/src/commands_options_data.rs:1093-1096`
  - `load_options_dataset_manifest`: `src-tauri/src/commands_options_data.rs:1107-1119`
- app-core manifest scanning already tolerates a missing directory:
  - `crates/app_core/src/options_data_store.rs:467-470`

Impact:

- listing/loading options datasets becomes a write operation
- these commands can fail in read-only environments for reasons unrelated to the requested read

Smallest justified fix:

- split the helper into:
  - a read-only directory resolver
  - and a create-if-needed writer path used only by import/download

Sensitivity:

- safe-now

#### 5. One malformed dataset manifest can break listing of all healthy datasets

Objective issue:

- `list_options_datasets` directly delegates to `scan_options_dataset_manifests`:
  - `src-tauri/src/commands_options_data.rs:1093-1096`
- the app-core scan loop propagates the first bad manifest read/parse error:
  - `crates/app_core/src/options_data_store.rs:467-485`

Impact:

- one corrupt `manifest.json` prevents the UI from seeing every other healthy dataset
- the dataset library view is more brittle than necessary

Smallest justified fix:

- degrade to best-effort listing plus warning/logging for bad manifests

Sensitivity:

- safe-now
- coordination required with app-core

#### 6. The retry helper only recognizes integer `Retry-After` values

Objective issue:

- `send_json_with_retry` parses `Retry-After` only as `u64` seconds:
  - `src-tauri/src/commands_options_data.rs:535-540`
- HTTP semantics allow either an `HTTP-date` or delay-seconds
  - external standard consulted: IETF HTTP Semantics

Impact:

- valid `Retry-After` date-form values are ignored
- backoff remains functional, but it can diverge from provider intent

Smallest justified fix:

- support both integer-seconds and date-form `Retry-After` values, or explicitly document the fallback behavior

Sensitivity:

- safe-now

### API and Naming

#### 7. The file exposes a stronger dataset-id validation story on import than on provider download

Objective issue:

- import flow checks for an empty resolved dataset id:
  - `src-tauri/src/commands_options_data.rs:983-987`
- provider download flow does not perform the same post-sanitization validation:
  - `src-tauri/src/commands_options_data.rs:821-828`

Impact:

- the two public ingestion paths fail differently for equivalent invalid explicit IDs
- that makes the public contract harder to reason about

Recommended action:

- unify validation and error wording across both ingestion paths

Sensitivity:

- safe-now

### Module Boundaries / Tech Debt

#### 8. This file mixes four different responsibilities that are all independently non-trivial

Objective issue:

- the same module currently owns:
  - Tauri command wrappers
  - store layout and dataset-id policy
  - provider HTTP/retry/orchestration
  - CSV normalization/output

Impact:

- the module is harder to reason about than necessary
- options dataset store changes, provider API changes, and command-surface changes all land in one file

Recommended action:

- keep the public surface here for now
- consider a later bounded extraction of:
  - provider download client/orchestration
  - dataset store helpers
  - pure request-normalization helpers

Sensitivity:

- larger refactor but defer

### Docs / Examples

#### 9. Provider-specific and store-layout assumptions are carried mostly by code, not by local documentation

Objective issue:

- the file assumes:
  - Polygon/Massive-only provider semantics
  - a fixed dataset store layout
  - aggregate-lane-only provider download normalization
- those assumptions are visible in code but not explained locally

Impact:

- maintainers have to rediscover product assumptions by tracing both frontend and options-expression runtime code

Smallest justified fix:

- add short comments or nearby docs describing:
  - the store layout contract
  - why Polygon/Massive downloads are normalized to minute-aggregate `close`
  - why list/load/import/download are separated the way they are

Sensitivity:

- document-only

### Tests / Missing Verification

#### 10. Current tests are mostly helper-level and mocked-frontend-level, not command-boundary-level

Objective issue:

- in-file Rust tests cover helper logic and one headless import rejection:
  - `src-tauri/src/commands_options_data.rs:1147-1324`
- local verification run:
  - `cargo test -p ta_desktop_demo_app commands_options_data -- --nocapture`
  - passed `6` tests
- there is no `src-tauri/tests` coverage for the public Tauri commands
- frontend tests mock `invokeCommand` rather than exercising Rust:
  - `web/src/services/optionsExpressionService.test.ts`
  - `web/src/stores/optionsExpressionStore.test.ts`
  - `web/src/components/OptionsExpressionPanel.test.ts`

Impact:

- the current coverage does not prove:
  - async command wrappers resolve `app_local_data_dir` correctly
  - successful import/list/load flows work end to end
  - cleanup-on-failure works for partial dataset writes
  - provider request validation errors surface at the right stage

Smallest justified fix:

- add at least one IPC/runtime-backed Rust test per command path
- add focused success/failure tests around:
  - read-only list/load behavior
  - invalid explicit dataset IDs
  - cleanup-on-failure
  - request validation before network work

Sensitivity:

- safe-now

### Perf / Allocation / CUDA Concerns

#### 11. The import path copies large CSVs before validating obvious metadata/semantic failures

Objective issue:

- `std::fs::copy` happens before provider/source/aggregate validation and before dataset parse/build:
  - `src-tauri/src/commands_options_data.rs:1004-1008`
  - validation starts at `src-tauri/src/commands_options_data.rs:1009`
  - parse/build happens at `src-tauri/src/commands_options_data.rs:1038-1057`

Impact:

- obviously invalid import requests still pay full copy cost and temporary disk usage
- this is not a runtime hot path, but it is user-visible wasted I/O on large files

Smallest justified fix:

- move pure metadata validation ahead of the file copy
- keep CSV parsing itself after the copy if the managed-store copy remains part of the product contract

Sensitivity:

- safe-now

#### 12. Download fan-out is sequential and provider-sensitive; do not “optimize” it casually

Observation:

- the provider download loop fetches aggregates contract by contract:
  - `src-tauri/src/commands_options_data.rs:916-940`
- this is potentially expensive, but it is also rate-limit and provider-behavior sensitive

Recommended action:

- do not parallelize provider requests casually in this file
- treat any concurrency/performance change as provider-quota-sensitive and benchmark-sensitive

## Recommended Action

- Overall packet disposition: `Small surgical refactor`

Safe-now changes:

- validate and normalize explicit download dataset IDs
- split read-only vs writer-only dataset directory resolution
- normalize and validate `contract_reference_as_of_utc` consistently
- reject inverted strike ranges before provider requests
- move import metadata validation ahead of file copy
- add Rust-side command-boundary tests

Document-only changes:

- clarify provider/store-layout assumptions locally
- document the aggregate-lane normalization contract for direct Polygon/Massive downloads

Larger refactor but defer:

- split provider orchestration and store helpers into dedicated modules
- redesign manifest-listing behavior to surface partial results plus warnings

Semantics-sensitive; do not touch casually:

- changing malformed-contract handling from hard failure to skip-and-count behavior
- any provider-download concurrency changes

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_options_data -- --nocapture`
2. Add and run targeted Rust tests for:
   - successful import into store plus manifest readability
   - read-only list/load behavior when the datasets root is absent or read-only
   - invalid explicit dataset id on the download path
   - `contract_reference_as_of_utc` normalization
   - inverted strike-range validation
   - cleanup-on-failure for import/download
3. If provider/retry behavior changes, add isolated tests around:
   - `send_json_with_retry`
   - contract pagination
   - aggregate chunking/dedup
4. Rerun frontend consumers covering:
   - `web/src/services/optionsExpressionService.test.ts`
   - `web/src/stores/optionsExpressionStore.test.ts`
   - `web/src/components/OptionsExpressionPanel.test.ts`
5. If live provider behavior changes and credentials are available, run a bounded headless smoke using the options dataset import/download CLI helpers before accepting the packet’s recommendations as implemented

## Cross-File Coordination

- `src-tauri/src/settings.rs`
  - owns market-data-root resolution
- `src-tauri/src/polygon.rs`
  - owns stock-underlier cache download and API-key lookup
- `crates/app_core/src/options_data_store.rs`
  - owns manifest path/read/write/scan and CSV-to-dataset parsing
- `src-tauri/src/commands_options_expression.rs`
  - depends on the exact dataset-store and manifest contract created here
- `web/src/services/optionsExpressionService.ts`
  - normalizes all public command payloads from this file
- `web/src/stores/optionsExpressionStore.ts`
  - is the main active frontend consumer of these commands

## Code-Quality and Debt Register

- High-value debt:
  - read-only commands mutate disk
  - inconsistent `contract_reference_as_of_utc` normalization
  - missing public-command integration coverage
- Medium-value debt:
  - inconsistent dataset-id validation between import and download
  - large module with provider, store, and Tauri concerns mixed together
- Low-value debt:
  - local documentation does not explain the provider/store contract clearly

## Performance Opportunity Register

- Safe-now:
  - validate import metadata before copying large files
- Measurement-first:
  - any change to provider request concurrency
  - any change to retry policy or backoff strategy
- Not worth chasing here:
  - micro-optimizations unrelated to filesystem/network behavior

## Implementation Staging

1. Safe now
   - dataset-id validation hardening
   - read-only helper split
   - `contract_reference_as_of_utc` normalization
   - strike-range prevalidation
   - pre-copy validation for import
   - test coverage improvements
2. Coordination required
   - resilient partial listing of healthy manifests when one manifest is corrupt
3. Defer until later pass
   - module extraction of provider/store concerns
   - semantics changes around malformed contract handling
   - provider request concurrency changes

## Change Blast Radius

- High blast radius:
  - manifest schema or dataset-store-layout changes
  - changes to provider-download response payload fields
- Medium blast radius:
  - dataset-id validation and error wording
  - `contract_reference_as_of_utc` normalization
  - list/load behavior changes
- Low blast radius:
  - pre-copy validation ordering
  - new tests and local documentation

## Observability Gaps

- no structured warning path when one manifest is corrupt but others are healthy
- no explicit diagnostic surface for provider retry decisions
- no command-boundary tests proving cleanup behavior after partial write failures

## Open Questions and Assumptions

- Assumption:
  - the product currently wants direct provider download in this panel to stay aggregate-lane only
- Open question:
  - should corrupt manifests be hidden with warnings, or should the whole list remain hard-fail strict?
- Open question:
  - should `load_options_dataset_manifest` remain public if the frontend continues to rely almost entirely on `list_options_datasets`?
