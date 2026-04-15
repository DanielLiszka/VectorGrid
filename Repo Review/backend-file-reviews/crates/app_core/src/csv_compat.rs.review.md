# Review Packet: `crates/app_core/src/csv_compat.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/csv_compat.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful and already fairly well-tested ingestion helper, but not public-review ready as-is because two parser acceptance rules can silently alter or propagate malformed numeric data, and the supported input contract is broader in practice than the rustdoc says`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Data-ingestion`, `Semantics-sensitive`, `Low-throughput-risk`

## What This File Owns

`csv_compat.rs` owns one public CSV compatibility loader for building `Candles` from desktop-side CSV inputs. It owns header normalization, alias-based column mapping, permissive positional fallback, flexible timestamp parsing, optional-number parsing, CSV read error formatting, and the final `CandleFieldFlags` that downstream runtime code uses to decide whether optional arrays are available.

This file owns:

- header normalization and alias lookup:
  - `normalize_header`: `crates/app_core/src/csv_compat.rs:35-41`
  - `find_idx`: `43-52`
- column inference and compatibility mapping:
  - `infer_column_mapping`: `54-119`
- timestamp and optional numeric parsing:
  - `parse_i64_flexible`: `121-140`
  - `parse_f64_or_nan`: `142-151`
- CSV row emptiness, line-number extraction, and read-error formatting:
  - `csv_record_is_empty`: `153-155`
  - `csv_record_line_num`: `157-163`
  - `format_csv_read_error`: `165-175`
- public ingestion entrypoint:
  - `read_candles_from_csv_compat`: `182-303`
- file-local compatibility tests:
  - `323-417`

This file does not own:

- cache persistence or cache selection
  - `src-tauri/src/data_cache.rs`
- timestamp normalization to milliseconds or timeframe inference
  - `src-tauri/src/state.rs:1523-1543`
- downstream strategy/runtime behavior that relies on `candles.fields`
  - callers in strategy/runtime modules consume those flags after load

## Classification

- Public surface: yes
  - exported from `crates/app_core/src/lib.rs:94`
- Hot path: moderate
  - not per-bar compute hot path, but it is the first runtime ingestion step for user CSV input
- CUDA/runtime-critical: indirect only
  - it does not launch GPU work, but its output gates downstream runtime surfaces and optional-column availability
- Strategy-local: no
  - shared loader
- Low-risk support code: mixed
  - simple implementation, but semantics matter because bad ingest can taint the entire backend pipeline

## Direct Interactions

### Backend interaction map

- public re-export:
  - `crates/app_core/src/lib.rs:94`
- primary desktop runtime ingestion path:
  - `src-tauri/src/data_cache.rs:304-316`
- direct test/helper loader in validation command tests:
  - `src-tauri/src/commands_validation.rs:1096-1098`
- timestamp normalization and timeframe inference after load:
  - `src-tauri/src/state.rs:1523-1543`
- metadata derived from `Candles.fields`:
  - `src-tauri/src/state.rs:809-821`
- downstream runtime consumers that gate behavior on optional-field availability:
  - `crates/app_core/src/double_ma.rs:2553-2554`
  - `crates/app_core/src/composite_cpu.rs:11090`
  - `crates/app_core/src/gpu_backtest_kernel.rs:3730`

### Other direct callers

- command-line and audit tools also use this loader directly:
  - `crates/app_core/src/bin/ta_cli.rs:190`
  - `crates/app_core/src/bin/real_data_optimizer_eval.rs:124`
  - `crates/app_core/src/bin/real_data_fidelity_audit.rs:292`
  - `src-tauri/src/bin/ta_headless_validate.rs:393`
  - `src-tauri/src/bin/ta_profile_route_matrix.rs:192`
  - `src-tauri/src/bin/ta_options_parent_artifact.rs:302`
  - `src-tauri/src/bin/ta_refactor_snapshots.rs:136`

### Downstream verification impact if changed

- timestamp parsing changes affect:
  - timestamp normalization to milliseconds
  - timeframe inference
  - cache contents and any downstream resampling
- optional-column parsing changes affect:
  - `Candles.fields`
  - validation/runtime paths that require `high`, `low`, `open`, or `volume`
- support-contract changes affect:
  - any user CSVs that rely on permissive compatibility behavior

## Standards And Research Log

This packet was written after targeted web research focused on CSV parser semantics, supported-header behavior, and public Rust API documentation expectations for a compatibility loader whose risk is silent data-shape drift.

### Sources consulted

1. `csv` crate `ReaderBuilder` documentation
   - Source: <https://docs.rs/csv/latest/csv/struct.ReaderBuilder.html>
   - Why it was consulted:
     - this file relies on `has_headers(false)`, `flexible(true)`, and `trim(Trim::All)`
   - Standard informed:
     - `has_headers(false)` means the CSV crate does not treat the first row specially, so this file’s manual first-record header handling is entirely repo-owned behavior
     - `flexible(true)` deliberately disables unequal-length record checking

2. RFC 4180, Common Format and MIME Type for CSV Files
   - Source: <https://datatracker.ietf.org/doc/html/rfc4180>
   - Why it was consulted:
     - this file is intentionally a compatibility reader rather than a strict RFC parser
   - Standard informed:
     - header rows and data rows normally have the same number of fields, but this file explicitly opts into more permissive row-length handling

3. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this is a public ingestion helper whose contract needs to be predictable on malformed or ambiguous input
   - Standard informed:
     - public behavior should be unsurprising and documented where heuristics are involved

4. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the current doc comment is short relative to the loader’s real compatibility heuristics
   - Standard informed:
     - public functions should document supported formats, ambiguities, and caveats

5. Rust Book, How to Write Tests
   - Source: <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>
   - Why it was consulted:
     - this file’s main risks are small input-shape edge cases that should be pinned with focused tests
   - Standard informed:
     - parser edge behavior should be asserted explicitly

## Findings

## Correctness And Safety

### 1. Fractional timestamps are silently rounded instead of being rejected

- Evidence:
  - `parse_i64_flexible` accepts `f64` after integer parsing fails:
    - `crates/app_core/src/csv_compat.rs:126-133`
  - it then rounds the float with `f.round()`:
    - `135`
  - every row uses that helper for the timestamp column:
    - `237-245`
- Why it matters:
  - an input timestamp like `1712345678.6` is accepted and shifted to the next integer rather than rejected
  - timestamps then feed normalization and timeframe inference in the Tauri runtime:
    - `src-tauri/src/state.rs:1523-1543`
  - this is silent data mutation at the ingestion boundary
- Smallest justified fix:
  - accept float-form timestamps only when they are exactly integral after parsing, such as `123.0`
  - reject non-integral fractional timestamps with a line-numbered error
- Review judgment:
  - real semantics-sensitive correctness issue
  - not benchmark-sensitive

### 2. Optional non-empty numeric columns accept non-finite values even though `close` rejects them

- Evidence:
  - `close` is parsed and explicitly rejected if non-finite:
    - `crates/app_core/src/csv_compat.rs:247-256`
  - optional values use `parse_f64_or_nan`, which accepts any non-empty `f64` parse result:
    - `142-151`
  - optional columns are loaded from that helper:
    - `258-265`
  - field availability is then inferred from whether any row is finite:
    - `267-278`
    - `292-298`
- Why it matters:
  - values such as `NaN`, `inf`, or `-inf` can enter `open`, `high`, `low`, or `volume`
  - downstream runtime code often treats `candles.fields.*` as the main validity gate before consuming those arrays
  - this can leave a column marked present while still containing pathological non-finite entries in some rows
- Smallest justified fix:
  - keep blank or missing cells as `NaN` sentinels for “absent”
  - but reject non-empty non-finite values when a user explicitly supplied them
- Review judgment:
  - real ingestion-hardening gap
  - semantics-sensitive because previously accepted malformed files would start failing fast

## API And Naming

### 1. The public support contract is under-documented for positional fallback and header handling

- Evidence:
  - the function docs say it supports:
    - VectorTA positional CSV
    - Polygon raw aggregates CSV
    - `crates/app_core/src/csv_compat.rs:177-181`
  - the implementation always consumes the first non-empty record as a header record:
    - `191-202`
  - `ReaderBuilder` is explicitly configured with `has_headers(false)`:
    - `184-188`
  - positional fallback is chosen whenever required named aliases are not both present:
    - `62-65`
    - `90-116`
- Why it matters:
  - a reader of the rustdoc can reasonably assume headerless positional CSV is supported, but the implementation would instead treat the first data row as a header and drop it from the candle data
  - similarly, positional fallback is only safe when columns remain in the expected VectorTA order
- Smallest justified fix:
  - document explicitly that the current loader expects a header row, even when it later falls back to positional order
  - document that positional fallback assumes VectorTA column order
  - if headerless CSVs are intentionally supported, that should become an explicit code path with tests rather than an inference
- Review judgment:
  - contract/documentation gap
  - likely safe to address with docs first

## Error Handling

### 1. File-open errors omit the path that failed

- Evidence:
  - open failure message is currently:
    - `failed to open CSV: {e}` at `crates/app_core/src/csv_compat.rs:183`
- Why it matters:
  - this function is used by cache loaders, validation helpers, CLI paths, and several backend tools
  - including the path would make ingestion failures easier to diagnose when multiple sources are in play
- Smallest justified fix:
  - include `file_path` in the open error message
- Review judgment:
  - low-severity observability improvement

### 2. Record-level read errors are otherwise handled well

- Evidence:
  - read errors include context and line numbers when available:
    - `165-175`
    - `199`
    - `222`
- Review judgment:
  - no broader error-handling rewrite is needed

## Module Boundaries And Technical Debt

### 1. This loader owns important compatibility heuristics that are not described anywhere central

- Evidence:
  - header aliases, positional fallback, flexible row lengths, timestamp coercion, and blank-to-`NaN` semantics all live only in this file
- Why it matters:
  - future ingest changes can accidentally break desktop data-loading behavior even if the rest of the backend stays untouched
  - because this helper is re-exported, its support contract is a backend API in practice
- Smallest justified fix:
  - keep the heuristics local, but document them fully here and reference them from any later ingestion architecture notes
- Review judgment:
  - documentation debt more than structural debt

## Docs And Examples

### 1. The doc comment should describe the actual compatibility policy, not just two headline formats

- Evidence:
  - current docs mention only two supported shapes:
    - `crates/app_core/src/csv_compat.rs:177-181`
  - they do not mention:
    - header-row expectation
    - accepted aliases
    - positional fallback assumptions
    - blank optional cells becoming `NaN`
    - non-empty optional numeric acceptance rules
- Why it matters:
  - this is a public loader used by both app runtime and offline tooling
  - users and future maintainers need a concrete contract
- Smallest justified fix:
  - expand rustdoc with examples and caveats for:
    - expected header usage
    - supported aliases
    - how optional columns are represented when blank
- Review judgment:
  - safe-now docs improvement

## Tests And Missing Verification

### 1. Existing coverage is good for baseline compatibility paths, but it misses the two semantics-sensitive numeric edge cases

- Evidence:
  - current tests cover:
    - Polygon raw columns: `323-343`
    - named aliases with blank optional fields: `345-367`
    - positional fallback: `369-389`
    - invalid timestamp strings: `391-399`
    - quoted headers and ignored comma-bearing fields: `401-417`
  - current tests do not cover:
    - fractional timestamps
    - non-finite optional numeric values
- Smallest justified fix:
  - add a unit test for fractional timestamp input such as `1350460800000.6`
  - add unit tests for `NaN`, `inf`, and `-inf` in optional columns

### 2. There is no test clarifying whether headerless positional CSV is intentionally supported

- Evidence:
  - first non-empty row is always treated as the header record:
    - `crates/app_core/src/csv_compat.rs:191-202`
  - no current test locks headerless behavior one way or the other
- Why it matters:
  - without a test, the team cannot tell whether dropping the first data row on headerless input is accepted behavior or an accidental gap
- Smallest justified fix:
  - add an explicit test for headerless positional CSV
  - if unsupported, assert the current failure or document it explicitly
  - if supported, add a dedicated detection path

### 3. Consumer-side smoke coverage should be considered if semantics change

- Evidence:
  - cache loader tests use this helper indirectly:
    - `src-tauri/src/data_cache.rs:550-599`
- Review judgment:
  - if parser semantics tighten, at least one desktop-side consumer test should be rerun

## Performance, Allocation, And CUDA Concerns

### 1. No conservative performance issue was identified in this file

- Evidence:
  - CSV load happens once per file, not per-strategy evaluation
  - the implementation is straightforward and already uses the `csv` crate’s buffered reader behavior
- Why it matters:
  - this is not a meaningful target for speculative micro-optimization
- Review judgment:
  - accept current throughput profile as-is

## Code-Quality Improvement Register

- Safe now:
  - expand rustdoc to describe the actual compatibility contract
  - include `file_path` in open-failure errors
  - add focused tests for fractional timestamps and non-finite optional values
- Coordination required:
  - reject non-integral fractional timestamps
  - reject non-empty non-finite optional numeric cells
- Defer until later pass:
  - any broader redesign of supported CSV dialects or auto-detection policy

## Technical-Debt Reduction Register

- Safe to reduce now:
  - documentation ambiguity around header expectations and positional fallback
  - missing tests for parser edge cases
- Defer:
  - broader dialect negotiation or schema-declaration support
- Useful debt that should not be “cleaned up” casually:
  - permissive `flexible(true)` row handling if the team still needs compatibility with imperfect real-world CSVs

## Performance Opportunity Register

- Safe structural improvements:
  - none required
- Benchmark-sensitive or throughput-sensitive candidates:
  - none identified
- Semantics-sensitive candidates:
  - timestamp parsing strictness
  - optional-column finite-value strictness

## Observability And Diagnostics Gaps

- open failure lacks file path context
- rustdoc does not spell out the real compatibility heuristics
- headerless positional CSV support is not currently explicit one way or the other

## Open Questions Or Assumptions

- assumption:
  - the current product likely expects a header row even for positional fallback, because the implementation always consumes the first non-empty row as headers
- open question:
  - should headerless positional CSV be supported deliberately, or should the contract stay header-required and be documented as such?

## Implementation Staging Guidance

- Safe now:
  - docs expansion
  - error-message context improvement
  - new parser edge-case tests
- Coordination required:
  - strict rejection of non-integral timestamps
  - strict rejection of explicit non-finite optional values
- Defer until later pass:
  - larger compatibility-policy redesign

## Change Blast Radius

- Local only:
  - rustdoc and open-error context
- Same subsystem:
  - parser strictness changes that affect cache/input handling
- Cross-backend runtime impact:
  - any change that alters timestamp arrays or `Candles.fields`

## Recommended Action

`Small surgical refactor`

The best next pass is:

- clarify the input contract
- add the missing edge-case tests
- tighten timestamp and optional numeric acceptance only if the team wants malformed data to fail fast instead of being coerced

## Exact Local Verification Required If Changed

If only docs or error-message context changes:

- `cargo test -p ta_app_core csv_compat -- --nocapture`

If timestamp or optional-field parsing semantics change:

- `cargo test -p ta_app_core csv_compat -- --nocapture`
- targeted desktop consumer tests in `src-tauri/src/data_cache.rs`
- any workflow smoke test that loads real CSV input and derives timeframe/metadata

## Regression Risk And Feature-Preservation Constraints

The goal here is to improve ingestion correctness and contract clarity without reducing supported backend capability.

- do not remove Polygon raw CSV support
- do not remove positional fallback casually if real users depend on it
- if timestamp or optional-value parsing is tightened, treat it as an explicit input-validation policy change and verify against current data-loading workflows
- preserve downstream `Candles` shape and `CandleFieldFlags` semantics for valid inputs
