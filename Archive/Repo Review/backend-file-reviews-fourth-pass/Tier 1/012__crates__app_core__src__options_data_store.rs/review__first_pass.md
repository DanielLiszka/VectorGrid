# Review Packet: `crates/app_core/src/options_data_store.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_data_store.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally useful and locally covered, but not fully public-review ready because explicit root-symbol overrides can hide mixed-root CSVs, duplicate per-contract timestamps are silently dropped while manifest row_count preserves the pre-dedup count, duplicate normalized headers silently shadow earlier columns, and manifest scanning fails closed on the first malformed dataset`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Artifact-contract`, `Options-workflow`, `Semantics-sensitive`, `Low-runtime-risk`

## What This File Owns

`options_data_store.rs` owns normalized options-dataset CSV import, manifest read/write/scan helpers, and the conversion from flat normalized rows into grouped in-memory option-contract series.

It owns:

- public import request DTO:
  - `ImportedOptionsDatasetCsvBuildRequest`: `crates/app_core/src/options_data_store.rs:11-28`
- CSV normalization helpers:
  - `NormalizedCsvRow`: `31-40`
  - `HeaderIndex`: `43-66`
  - header normalization and lookup:
    - `68-107`
  - cell parse helpers:
    - `109-156`
  - root symbol, option type, and expiration derivation helpers:
    - `153-171`
  - `parse_row`: `173-289`
- primary normalized CSV ingest path:
  - `load_imported_options_dataset_from_csv`: `291-433`
- manifest read/write/list helpers:
  - `read_options_dataset_manifest`: `435-448`
  - `write_options_dataset_manifest`: `450-465`
  - `scan_options_dataset_manifests`: `467-485`
  - `dataset_manifest_path`: `488-489`
- file-local tests:
  - `492-681`

This file does not own:

- the manifest and row/container type definitions themselves:
  - `crates/app_core/src/options_types.rs`
- Tauri-side dataset import/download orchestration:
  - `src-tauri/src/commands_options_data.rs`
- options-expression runtime evaluation over imported datasets:
  - `crates/app_core/src/options_expression.rs`
  - `src-tauri/src/commands_options_expression.rs`

## Classification

- Public surface: yes
  - the module is public at `crates/app_core/src/lib.rs:49`
  - core entry points are re-exported at `crates/app_core/src/lib.rs:215-218`
- Hot path: no
  - this is ingest and manifest support, not runtime execution
- CUDA/runtime-critical: no
- Strategy-local: no
  - it is options-workflow support infrastructure rather than a single-strategy engine
- Low-risk support code: not entirely
  - it is support code, but it defines persisted dataset contract and import semantics that downstream workflows depend on

## Direct Interactions

### Backend interaction map

- crate export surface:
  - `crates/app_core/src/lib.rs:49`
  - `215-218`
- core type dependency:
  - `crates/app_core/src/options_types.rs:314-390`
- Tauri dataset import/list/load flows:
  - imports:
    - `src-tauri/src/commands_options_data.rs:9-11`
  - manifest persistence and listing:
    - `959-960`
    - `1058-1059`
    - `1096`
    - `1111-1119`
  - public import path:
    - `1017-1057`
    - `1069-1080`
- Tauri options-expression replay path:
  - imports:
    - `src-tauri/src/commands_options_expression.rs:22-25`
  - manifest validation and dataset replay loading:
    - `1595-1674`
- options-expression engine consumes `ImportedOptionsDataset`:
  - `crates/app_core/src/options_expression.rs:376`
  - `430-437`
  - `2505`
  - `3070-3182`
  - `3461`
  - `5225`
  - `5264`
- CLI/headless consumers:
  - `src-tauri/src/bin/ta_options_dataset_import.rs`
  - `src-tauri/src/bin/ta_options_dataset_download.rs`
  - `src-tauri/src/bin/ta_options_expression_feasibility.rs:1572`

### Downstream verification impact if changed

- changing normalized CSV parsing or grouping semantics affects:
  - options dataset import
  - options-expression replay and feasibility workflows
- changing manifest read/write/scan behavior affects:
  - Tauri dataset listing and load flows
  - persisted dataset directories already written to disk
- changing request-level invariant enforcement affects:
  - `commands_options_data.rs`
  - any direct crate consumer of `load_imported_options_dataset_from_csv`

## Standards And Research Log

This packet was written after targeted web research focused on public API documentation, Serde compatibility, CSV parser behavior, dedup semantics, timestamp conversion behavior, and direct file-write contracts.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exports a public import request and manifest I/O helpers
   - Standard informed:
     - public interfaces should keep their wire and persistence contracts explicit and intentional

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the public import and manifest helpers currently have no rustdoc contract guidance
   - Standard informed:
     - public functions should document examples and `Errors` semantics, especially for file and parse workflows

3. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - the public request type is serialized/deserialized and uses many required metadata fields
   - Standard informed:
     - persisted or transported request fields are compatibility contract, not incidental plumbing

4. `csv::ReaderBuilder` documentation
   - Source: <https://docs.rs/csv/latest/csv/struct.ReaderBuilder.html>
   - Why it was consulted:
     - the importer enables `flexible(true)` on a supposedly normalized CSV format
   - Standard informed:
     - `flexible(true)` explicitly disables record-width checking, so enabling it is a deliberate format-tolerance decision

5. `Vec::dedup_by` documentation
   - Source: <https://doc.rust-lang.org/std/vec/struct.Vec.html#method.dedup_by>
   - Why it was consulted:
     - the importer deduplicates per-contract rows by timestamp after sorting
   - Standard informed:
     - dedup keeps the first item of each equal run, so duplicate-row policy is a real semantics choice

6. `std::fs::write`
   - Source: <https://doc.rust-lang.org/std/fs/fn.write.html>
   - Why it was consulted:
     - manifest writes are done directly with `std::fs::write`
   - Standard informed:
     - direct writes replace the file contents in place; if more atomic publication is desired, staging and rename should be considered explicitly

7. `chrono::DateTime::from_timestamp_millis`
   - Source: <https://docs.rs/chrono/latest/chrono/struct.DateTime.html#method.from_timestamp_millis>
   - Why it was consulted:
     - the file derives expiration dates from millisecond timestamps
   - Standard informed:
     - invalid millisecond timestamps must be handled explicitly and are not guaranteed to decode

## Findings

## Correctness And Safety

### 1. An explicit `root_symbol` override can hide mixed-root CSV content instead of validating it

- Evidence:
  - `parse_row` uses the explicit root symbol directly when present:
    - `crates/app_core/src/options_data_store.rs:178-180`
  - when an override is provided, the CSV `root_symbol` column is not read or checked:
    - `181-187` only run in the `else` branch
  - the dataset-level root-symbol consistency check then compares against the already-overridden value:
    - `313-317`
    - `326-339`
- Why it matters:
  - a caller can pass `root_symbol = "SPY"` and the importer will accept rows whose CSV root symbol is something else, because the row-level value has already been replaced
  - that weakens the integrity of a persisted dataset manifest
- Smallest justified fix:
  - if the CSV contains a `root_symbol` column and the request also supplies an explicit root symbol, validate that non-empty row values agree with the override
  - keep the override behavior only for datasets that truly lack the column
- Review judgment:
  - medium-high correctness / artifact-integrity issue

### 2. Duplicate per-contract timestamps are silently dropped, but manifest `row_count` still reports the pre-dedup total

- Evidence:
  - `row_count` is incremented for every parsed row:
    - `crates/app_core/src/options_data_store.rs:353`
  - per-contract rows are later sorted and deduplicated by timestamp:
    - `395-402`
  - manifest `row_count` is then persisted from the original counter, not the retained rows:
    - `405-427`
- Why it matters:
  - the in-memory dataset and the persisted manifest can disagree about how many rows are actually retained after ingest
  - duplicate-row policy is currently invisible to downstream consumers
- Smallest justified fix:
  - track duplicate rows dropped during ingest
  - either make manifest `row_count` reflect retained rows or add a second field such as `import_row_count` / `deduplicated_rows`
  - do not silently change duplicate policy without documenting which row wins
- Review judgment:
  - high artifact-contract issue

## API And Naming

### 1. Duplicate normalized headers silently overwrite earlier columns

- Evidence:
  - `header_index` normalizes every header name and inserts it into a `BTreeMap`:
    - `crates/app_core/src/options_data_store.rs:74-78`
  - later duplicates of the same normalized name replace the earlier index with no error
- Why it matters:
  - headers like `root-symbol`, `root_symbol`, and `Root Symbol` all normalize to the same key
  - if more than one appears, the importer will silently bind to the last one, which is a real schema-resolution choice
- Smallest justified fix:
  - reject duplicate normalized header names up front
  - keep the current normalization, but do not silently shadow earlier columns
- Review judgment:
  - medium API / schema-contract issue

## Error Handling

### 1. Dataset manifest scanning aborts on the first malformed manifest

- Evidence:
  - `scan_options_dataset_manifests` iterates dataset directories:
    - `crates/app_core/src/options_data_store.rs:467-482`
  - each discovered manifest is read with `?`:
    - `482`
  - callers use this for dataset listing:
    - `src-tauri/src/commands_options_data.rs:1096`
    - `src-tauri/src/bin/ta_options_expression_feasibility.rs:1572`
- Why it matters:
  - one corrupt dataset manifest can suppress listing of every other healthy dataset
  - this is poor operator ergonomics for a storage directory intended to hold multiple datasets
- Smallest justified fix:
  - degrade malformed manifests into skipped entries plus a surfaced warning at the caller boundary
  - preserve hard failure for directory-read problems, not individual bad manifests
- Review judgment:
  - medium error-handling / operational resilience issue

### 2. `flexible(true)` weakens the normalized CSV contract by disabling record-width checks

- Evidence:
  - the importer configures the reader with:
    - `.flexible(true)`
    - `crates/app_core/src/options_data_store.rs:302-305`
- Why it matters:
  - the file is described and error-messages itself treat the input as a normalized CSV
  - disabling row-width checking means ragged rows can pass through farther than they otherwise would, especially where trailing optional fields become implicit `None`
- Smallest justified fix:
  - if ragged normalized CSVs are not intentionally supported, remove `flexible(true)`
  - if they are supported for compatibility, document that tolerance explicitly in the public contract
- Review judgment:
  - medium error-handling / format-contract issue

## Module Boundaries And Technical Debt

### 1. Some request invariants are enforced only in `commands_options_data.rs`, not in this public loader

- Evidence:
  - `ImportedOptionsDatasetCsvBuildRequest` is public here:
    - `crates/app_core/src/options_data_store.rs:11-28`
  - this loader persists `aggregate_price_field` directly into the manifest:
    - `427`
  - command-layer import logic enforces a key invariant before calling the loader:
    - `src-tauri/src/commands_options_data.rs:1017-1033`
  - command tests pin that upstream-only behavior:
    - `1288-1316`
- Why it matters:
  - direct callers of `load_imported_options_dataset_from_csv` can bypass command-layer invariant checks and persist semantically inconsistent manifest metadata
  - that weakens the public-review readiness of the loader itself
- Smallest justified fix:
  - move the dataset-contract validation that is intrinsic to the manifest into the loader, or expose a shared validator used by both layers
  - keep command-layer UX validation, but do not rely on it as the only guard for a public backend API
- Review judgment:
  - medium module-boundary / contract-consistency issue

### 2. Manifest writes are direct final-path writes with no staging step

- Evidence:
  - `write_options_dataset_manifest` serializes and writes directly to the final path:
    - `crates/app_core/src/options_data_store.rs:450-464`
- Why it matters:
  - this is low severity, but it means manifest publication is “replace in place,” not staged publication
  - if the repo later wants stronger artifact durability guarantees, this is the obvious seam to improve
- Smallest justified fix:
  - keep the current helper if simplicity is preferred
  - otherwise stage to a temp file and rename once serialization succeeds
- Review judgment:
  - low-medium manifest-publication debt

## Docs And Examples

### 1. The public import and manifest helpers have no rustdoc for format, dedup, or persistence semantics

- Evidence:
  - public items include:
    - `ImportedOptionsDatasetCsvBuildRequest`: `11`
    - `load_imported_options_dataset_from_csv`: `291`
    - `read_options_dataset_manifest`: `435`
    - `write_options_dataset_manifest`: `450`
    - `scan_options_dataset_manifests`: `467`
    - `dataset_manifest_path`: `488`
  - there is no rustdoc explaining:
    - header normalization behavior
    - duplicate header policy
    - duplicate timestamp policy
    - explicit root-symbol override semantics
    - whether ragged rows are intentionally allowed
    - what invariant subset is guaranteed by this file versus `commands_options_data.rs`
- Smallest justified fix:
  - add rustdoc with at least one end-to-end import example
  - document the error and dedup behaviors of the loader and manifest scan helpers

## Tests And Missing Verification

### 1. File-local coverage is good for quote semantics, but key ingest-contract edge cases remain unpinned

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core options_data_store::tests:: -- --nocapture`
      - passed `4` tests, including:
        - `load_imported_dataset_groups_contract_rows`
        - `load_quote_dataset_preserves_quote_fields_and_semantics`
        - `load_derived_mid_dataset_preserves_mid_fields_without_quote_fill_broadening`
        - `load_quote_dataset_rejects_mislabeled_rows_without_quote_columns`
  - Tauri-side command verification run:
    - `cargo test -p ta_desktop_demo_app commands_options_data::tests:: -- --nocapture`
      - passed `6` tests, including:
        - `polygon_download_import_request_freezes_aggregate_lane_invariants`
        - `minute_aggregate_import_rejects_non_close_aggregate_price_field`
  - current tests do not directly pin:
    - duplicate normalized header rejection
    - explicit root-symbol override mismatch detection
    - duplicate timestamp policy and manifest row-count semantics
    - malformed manifest scan tolerance
- Missing tests I would want before changing this file:
  - duplicate header normalization conflict should error
  - explicit request root symbol should be checked against CSV root-symbol values when present
  - duplicate per-contract timestamps should produce a pinned retained-row and manifest-count policy
  - manifest scan should define behavior on one malformed dataset among many

## Performance, Allocation, And CUDA Concerns

### 1. No urgent performance issue stands out; this is ingest-path correctness work, not runtime optimization work

- Evidence:
  - main cost is one-off CSV parse and grouping:
    - `crates/app_core/src/options_data_store.rs:291-433`
- Review judgment:
  - this is not the right file to chase micro-optimizations before the ingest contract is clearer

### 2. Sorting and dedup are acceptable here, but any change to duplicate-row semantics is semantics-sensitive rather than performance-driven

- Evidence:
  - per-contract rows are sorted and deduplicated:
    - `395-402`
- Review judgment:
  - keep performance work secondary
  - treat duplicate-row handling as a data-contract change first

## Code-Quality Improvement Register

- Safe now:
  - document duplicate-row and root-symbol-override semantics
  - reject duplicate normalized headers
  - clarify or remove `flexible(true)` if normalized CSVs are intended to be strict
  - harden manifest scanning against one bad dataset
- Coordination required:
  - moving request invariant validation from `commands_options_data.rs` into shared/backend code
  - changing persisted manifest `row_count` meaning
- Defer until later pass:
  - broader refactors across options import, manifest, and replay cache code

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing docs
  - duplicate-header ambiguity
  - manifest scan fail-closed behavior
- Coordination required:
  - request invariant ownership between this file and `commands_options_data.rs`
  - any persisted manifest field semantics change
- Useful debt that should not be “cleaned up” casually:
  - the grouped in-memory representation by contract, which downstream options-expression code relies on

## Performance Opportunity Register

- Safe structural improvements:
  - none urgent
- Benchmark-sensitive candidates:
  - none worth prioritizing before correctness and contract cleanup
- Not the right place to chase performance without evidence:
  - BTreeMap-versus-HashMap churn
  - row-clone micro-optimizations

## Observability And Diagnostics Gaps

- duplicate rows are dropped without a surfaced duplicate count
- explicit root-symbol overrides do not expose whether CSV content actually matched the override
- manifest scanning does not preserve partial success when one dataset is corrupt

## Implementation Staging Guidance

- Safe now:
  - docs
  - additive tests
  - duplicate-header hardening
  - manifest-scan resilience
- Coordination required:
  - aggregate/price-semantic invariant ownership
  - manifest `row_count` semantic change
  - root-symbol override contract tightening
- Defer until later pass:
  - broader options import/persistence refactors

## Change Blast Radius

- Local only:
  - rustdoc
  - additive tests
  - duplicate-header rejection
- Same subsystem:
  - duplicate-row or root-symbol behavior changes affect options dataset import and replay workflows
- Cross-backend / frontend-backend contract:
  - manifest field semantics affect Tauri dataset listing, loading, and options-expression UI assumptions

## Open Questions Or Assumptions

- assumption:
  - a normalized options CSV is intended to have one effective root symbol, not multiple symbols hidden behind an override
- open question:
  - whether `row_count` is intentionally “rows seen during import” rather than “rows retained after dedup”
- open question:
  - whether ragged row tolerance from `flexible(true)` is required for real provider-normalized exports or is just permissive leftover configuration

## Recommended Action

`Small surgical refactor`

The right next pass is:

- make the import contract explicit
- harden the obvious schema and artifact-listing edges
- decide and document the retained-row versus imported-row semantics
- keep broader options-workflow refactors deferred

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core options_data_store::tests:: -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_options_data::tests:: -- --nocapture`
- `cargo test -p ta_desktop_demo_app --no-run`
- if manifest row-count or scan behavior changes:
  - `cargo test --workspace --no-run`

## Regression Risk And Feature-Preservation Constraints

- do not silently change duplicate-row retention policy without documenting which row wins and how the manifest counts it
- do not narrow accepted normalized CSV formats without checking real stored/provider datasets first
- do not move invariant validation across layers in a way that breaks Tauri import UX or persisted options dataset compatibility
