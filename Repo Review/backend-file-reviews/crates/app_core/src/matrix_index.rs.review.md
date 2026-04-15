# Review Packet: `crates/app_core/src/matrix_index.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/matrix_index.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Algorithmically sound and small, but not fully public-review ready because the main index type carries non-trivial invariants while exposing fully public fields and derived deserialization, which allows invalid states to bypass the validated constructor`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Invariant-bearing`, `Serialization-sensitive`, `Support code`

## What This File Owns

`matrix_index.rs` owns the stable index mapping for the common Double-MA `fast < slow` grid.

It owns:

- the serialized ordering enum:
  - `MatrixOrder`: `crates/app_core/src/matrix_index.rs:5-9`
- the serialized index structure:
  - `DoubleMaFastLtSlowIndex`: `19-30`
- validated construction of the index:
  - `new`: `32-68`
- bidirectional mapping helpers:
  - `(fast_len, slow_len) -> col_id`:
    - `70-88`
  - `col_id -> (fast_len, slow_len)`:
    - `90-107`
- strict monotonic input validation:
  - `validate_strictly_increasing_u16`: `110-120`
- file-local tests:
  - `122-159`

This file does not own:

- period-range expansion or grid selection
- optimizer execution
- result assembly or Tauri transport logic

Those downstream consumers use this file as a stable mapping helper.

## Classification

- Public surface: yes
  - the module is public from `crates/app_core/src/lib.rs:41`
- Hot path: no
  - construction and lookup are lightweight support operations, not numeric execution kernels
- CUDA/runtime-critical: no
- Strategy-local: yes
  - scoped to Double-MA grid indexing and result metadata
- Low-risk support code: mostly support code, but not low-risk enough to ignore because serialized/public invariants matter

## Direct Interactions

### Backend interaction map

- crate export:
  - `crates/app_core/src/lib.rs:41`
- Double-MA result assembly:
  - `crates/app_core/src/double_ma_result.rs:13`
  - `67`
  - `150`
  - `231`
- composite optimization result surfaces:
  - `crates/app_core/src/composite_cpu.rs:112`
  - many result-building call sites such as:
    - `2775-2805`
    - `3121-3151`
    - `3603-3633`
    - `4506-4535`
    - `6304-6333`
    - `8193-8223`
    - `9571-9601`
    - `11522-11552`
- Double-MA result type carries the index:
  - `crates/app_core/src/double_ma.rs:595`
- Tauri-side command/test scaffolding references result fields that may include this structure:
  - `src-tauri/src/commands_campaign.rs:4125`

### Downstream verification impact if changed

- serialized field/layout changes affect:
  - result payloads produced by `double_ma_result.rs` and `composite_cpu.rs`
- mapping semantics changes affect:
  - best-column and top-column reporting across Double-MA result surfaces
- visibility or field-access changes affect:
  - any consumer constructing or deserializing the index directly

## Standards And Research Log

This packet was written after targeted web research focused on public API discipline for invariant-bearing data types, sorted-slice search assumptions, serde-driven serialization/deserialization behavior, and test expectations for small mapping helpers.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes a public data type plus public constructor and lookup helpers
   - Standard informed:
     - public API types should preserve their invariants and avoid accidental overexposure

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file is small but public, and its invariants matter more than its size suggests
   - Standard informed:
     - public types and methods should document mapping guarantees and invariants

3. Rust slice documentation
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html>
   - Why it was consulted:
     - the implementation relies on `partition_point` and `binary_search`
   - Standard informed:
     - these lookup helpers are correct only when sorted-slice invariants are maintained

4. Serde attributes / derive documentation
   - Source: <https://serde.rs/attributes.html>
   - Why it was consulted:
     - the main types derive `Serialize` and `Deserialize`
   - Standard informed:
     - derive is convenient, but invariant-bearing public structs may need additional validation strategy

5. Rust Book, Testing
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why it was consulted:
     - this file is a pure mapping helper whose quality bar is heavily test-driven
   - Standard informed:
     - small bidirectional mapping utilities should have focused unit coverage for valid, invalid, and boundary cases

## Findings

## Correctness And Safety

### 1. `DoubleMaFastLtSlowIndex` can be instantiated in invalid states outside `new()`

- Evidence:
  - the struct exposes all invariant-bearing fields publicly:
    - `crates/app_core/src/matrix_index.rs:19-30`
  - it derives `Serialize` and `Deserialize`:
    - `19`
  - `new()` validates sorted input and internally derives coherent offsets/start indices:
    - `32-68`
  - lookup methods assume those invariants hold:
    - `70-107`
  - there are no in-repo direct struct literal constructions outside this file:
    - search results show only the type declaration and impl within `matrix_index.rs`
- Why it matters:
  - a caller can bypass `new()` through direct construction or deserialization and create mismatched `fast_offsets`, `slow_start_idx`, `cols`, or unsorted period arrays
  - the current methods degrade to wrong answers or spurious `None` rather than surfacing that the index object itself is invalid
- Smallest justified fix:
  - make the fields private
  - keep `new()` as the validated constructor
  - if deserialization from external artifacts is required, add a validated deserialization path rather than raw derived `Deserialize`
- Review judgment:
  - high-severity invariant/API issue
  - safe-now if no external consumer depends on public fields

## API And Naming

### 1. `MatrixOrder` is a public single-variant enum with no in-repo consumer outside this file

- Evidence:
  - `MatrixOrder` has one variant:
    - `crates/app_core/src/matrix_index.rs:5-9`
  - search results only show usage inside `matrix_index.rs`
- Why it matters:
  - a public single-variant enum can be a valid forward-compatibility anchor
  - but if that is the intent, it should be documented
  - otherwise it is just extra public API surface
- Smallest justified fix:
  - either document it as a forward-compatibility marker for serialized ordering semantics
  - or narrow it if crate-external use is not intended
- Review judgment:
  - low-medium API issue

### 2. The main struct’s public fields look broader than current consumer usage requires

- Evidence:
  - public fields:
    - `crates/app_core/src/matrix_index.rs:21-29`
  - in-repo consumers appear to treat the type as an opaque payload/helper rather than mutating its internals
- Why it matters:
  - public fields increase compatibility burden and weaken the invariant story without obvious in-repo benefit
- Smallest justified fix:
  - privatize fields and add read-only accessors only if callers truly need them
- Review judgment:
  - medium API-surface issue

## Error Handling

### 1. Constructor validation is explicit, but invalid-state usage after construction degrades silently

- Evidence:
  - `new()` returns descriptive `String` errors for empty/unsorted/overflow cases:
    - `33-68`
    - `110-120`
  - lookup methods return `Option` and cannot distinguish “not found” from “corrupt internal shape”:
    - `70-107`
- Why it matters:
  - that distinction is acceptable when the type can only be created by `new()`
  - it becomes ambiguous once public fields / raw deserialization can violate invariants
- Smallest justified fix:
  - preserve current method signatures
  - strengthen the type invariant boundary instead of rewriting return types
- Review judgment:
  - acceptable if the type is validated at construction
  - weak once raw external construction is possible

## Module Boundaries And Technical Debt

### 1. Serialization convenience currently weakens encapsulation

- Evidence:
  - derives:
    - `crates/app_core/src/matrix_index.rs:19`
  - public fields:
    - `21-29`
  - no custom validation layer around deserialization
- Why it matters:
  - the file is mixing two goals:
    - ergonomic serialization
    - strong internal invariants
  - today, serialization wins and invariants are left exposed
- Smallest justified fix:
  - keep serialization support
  - move validation responsibility back to the type boundary
- Review judgment:
  - medium technical-debt issue

## Docs And Examples

### 1. The file explains the ordering, but not the full invariant contract

- Evidence:
  - the top-level docs describe the mapping and `FastMajor` traversal:
    - `crates/app_core/src/matrix_index.rs:11-18`
  - they do not explicitly document:
    - strict monotonicity requirements
    - the meaning of `slow_start_idx` and `fast_offsets` as derived internals
    - whether zero-column indices are valid
    - whether callers are expected to construct/deserialize this type directly
- Smallest justified fix:
  - extend the type docs to say:
    - construct via `new()`
    - fields are derived invariants, not user-authored payload fields
    - zero-column behavior if intentionally supported

## Tests And Missing Verification

### 1. Local mapping coverage is good, but invariant-boundary behavior is not tested

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core matrix_index::tests:: -- --nocapture`
      - passed
      - included:
        - `double_ma_index_maps_both_directions`
        - `double_ma_index_rejects_unsorted_axes`
        - `double_ma_index_rejects_invalid_pairs_and_out_of_bounds_cols`
  - current tests cover constructor validation and basic bidirectional mapping:
    - `crates/app_core/src/matrix_index.rs:122-159`
- Missing tests I would want before changing this file:
  - a serialization round-trip test if this type is intended to be persisted/transmitted
  - a test for degenerate zero-column grids if that behavior is intentionally allowed
  - a test that invalid deserialization or invalid internal shape is rejected, if validation is added

## Performance, Allocation, And CUDA Concerns

### 1. The core algorithm is already appropriate for the problem size and usage pattern

- Evidence:
  - construction uses `partition_point` once per fast period:
    - `crates/app_core/src/matrix_index.rs:46-58`
  - lookups use binary search / partition-point-style access:
    - `74-82`
    - `94-100`
- Review judgment:
  - no meaningful performance issue
  - this is the right complexity class for a stable lookup helper

### 2. Saturating arithmetic is acceptable only because `new()` establishes invariants

- Evidence:
  - `col_id` uses `saturating_add`:
    - `crates/app_core/src/matrix_index.rs:82`
  - `params_for_col` uses `saturating_sub` and `saturating_add`:
    - `95`
    - `99-100`
- Why it matters:
  - under valid constructed state, this is harmless defensive arithmetic
  - under externally corrupted state, it can hide invariant breakage rather than exposing it
- Review judgment:
  - do not change this first
  - fix the invariant boundary instead

## Code-Quality Improvement Register

- Safe now:
  - privatize invariant-bearing fields
  - add rustdoc clarifying construction/serialization expectations
  - add one degenerate-grid test if zero-column indices are meant to be supported
- Coordination required:
  - changing serialized/public field visibility if external consumers depend on literal field access
  - adding validated custom deserialization
- Defer until later pass:
  - any broader serialization schema redesign

## Technical-Debt Reduction Register

- Safe to reduce now:
  - public-field overexposure
  - undocumented invariant contract
- Coordination required:
  - serialized shape changes
  - public enum/type narrowing
- Useful debt that should not be “cleaned up” casually:
  - the stable explicit mapping model itself, which is useful for result reproducibility and UI correlation

## Performance Opportunity Register

- Safe structural improvements:
  - none urgent
- Benchmark-sensitive candidates:
  - none
- Not the right place to chase performance without evidence:
  - replacing clear binary-search logic with more complex indexing tricks

## Observability And Diagnostics Gaps

- the type does not document whether invalid serialized payloads are possible or supported
- the type does not document whether zero-column grids are valid
- the type does not document whether callers should treat fields as stable serialized contract or as derived internals

## Implementation Staging Guidance

- Safe now:
  - docs
  - field privacy tightening if no external caller depends on raw field access
  - additive tests
- Coordination required:
  - custom validated deserialization
  - serialized-shape compatibility decisions
- Defer until later pass:
  - broader response-schema redesign

## Change Blast Radius

- Local only:
  - docs
  - tests
- Same subsystem:
  - field visibility or serialization changes affect `double_ma_result.rs`, `double_ma.rs`, and `composite_cpu.rs` result payloads
- Cross-backend/runtime:
  - low; this is a result/index helper, not execution routing

## Open Questions Or Assumptions

- assumption:
  - in-repo callers are expected to create this type through `new()` only
- open question:
  - whether any external consumer depends on public field access or direct deserialization
- open question:
  - whether zero-column indices are intentionally supported or just tolerated

## Recommended Action

`Small surgical refactor`

The right next pass is:

- tighten the invariant boundary around `DoubleMaFastLtSlowIndex`
- document the serialization/constructor contract
- keep the lookup algorithm itself unchanged

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core matrix_index::tests:: -- --nocapture`
- `cargo test -p ta_app_core`
- if serialization or public field visibility changes:
  - `cargo test --workspace`

## Regression Risk And Feature-Preservation Constraints

- do not change the column ordering semantics without deliberate migration planning
- do not silently change serialized field names or layout if any external artifact or UI depends on them
- do not weaken the bidirectional mapping guarantee in exchange for API cleanup
