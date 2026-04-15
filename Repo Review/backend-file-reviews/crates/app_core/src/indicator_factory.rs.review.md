# Review Packet: `crates/app_core/src/indicator_factory.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/indicator_factory.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Usable but not fully public-review ready because the public matrix DTO exposes invariants that can be bypassed through direct construction/deserialization, the surface presents itself more generically than it really is, and wrapper-local test coverage is thin compared with the underlying MA implementation it delegates to`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Low-runtime-risk`, `Semantics-sensitive`

## What This File Owns

`indicator_factory.rs` is a small CPU-only moving-average batch facade, not a general indicator factory.

It owns:

- `IndicatorMatrixF64`, a row-major `(rows, cols, values)` DTO with `try_new()` and `row()`:
  - `crates/app_core/src/indicator_factory.rs:7-31`
- request/response DTOs for MA-batch execution:
  - `MovingAverageBatchRequest`: `34-39`
  - `MovingAverageBatchResponse`: `42-50`
- arithmetic-grid validation:
  - `validate_uniform_step`: `53-71`
- the wrapper entrypoint:
  - `compute_moving_average_batch_cpu`: `75-116`
- two module-local tests:
  - `121-164`

This file does not own MA math. It delegates real work to:

- `crate::double_ma::resolve_alma_params_for_side`: `double_ma.rs:10098`
- `crate::double_ma::ma_matrix_cpu_f64`: `double_ma.rs:10559`

## Classification

- Public surface: yes
  - the module is public from `crates/app_core/src/lib.rs:27`
- Hot path: no
  - the expensive work lives in `double_ma::ma_matrix_cpu_f64`
- CUDA/runtime-critical: no
- Strategy-local: no
  - generic MA wrapper over shared `double_ma` helpers
- Low-risk support code: yes
  - small wrapper with limited blast radius

## Direct Interactions

### Backend interaction map

- exported by:
  - `crates/app_core/src/lib.rs:27`
- uses candle input type:
  - `vector_ta::utilities::data_loader::Candles`
- delegates ALMA parameter resolution:
  - `crates/app_core/src/double_ma.rs:10098`
- delegates matrix construction:
  - `crates/app_core/src/double_ma.rs:10559`
- local-only usage today:
  - I did not find production callers for `compute_moving_average_batch_cpu`, `MovingAverageBatchRequest`, `MovingAverageBatchResponse`, or `IndicatorMatrixF64`; current in-repo usage is limited to the module-local tests

### Downstream verification impact if changed

- wrapper-local DTO or normalization changes would affect:
  - any future external consumer of this public module
- delegated math behavior is owned elsewhere:
  - changes to matrix semantics require verifying `double_ma` tests, not just this wrapper

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API clarity, truthful naming/documentation for narrow wrapper modules, deserialization and invariant ownership, and test posture for thin public facades.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes public DTOs and a public wrapper entrypoint
   - Standard informed:
     - public APIs should preserve invariants and communicate scope honestly

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the current doc comment says “IndicatorFactory-like,” but the real surface is narrower
   - Standard informed:
     - public APIs should describe what they actually do, not a broader aspirational role

3. Rust API Guidelines, Naming
   - Source: <https://rust-lang.github.io/api-guidelines/naming.html>
   - Why it was consulted:
     - the module and type names imply broader scope than the implementation currently provides
   - Standard informed:
     - names should match actual semantics and avoid misleading generality

4. Serde, Implementing Deserialize
   - Source: <https://serde.rs/impl-deserialize.html>
   - Why it was consulted:
     - `IndicatorMatrixF64` derives `Deserialize`, so constructor-only invariants are not actually enforced for all construction paths
   - Standard informed:
     - if a type needs invariant-preserving deserialization, derive alone may be insufficient and a custom path may be warranted

5. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - the wrapper returns raw `String` errors and mostly forwards lower-level failures
   - Standard informed:
     - this is acceptable for a thin boundary, but the behavior should be deliberate and stable

6. Rust Book, Test Organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why it was consulted:
     - thin public wrappers should pin their own boundary behavior rather than relying only on deeper implementation tests
   - Standard informed:
     - add focused wrapper tests instead of larger refactors

## Findings

## Correctness And Safety

### 1. `IndicatorMatrixF64` invariants are advisory, not enforced on all public construction paths

- Evidence:
  - the struct is fully public and derives `Serialize` / `Deserialize`:
    - `crates/app_core/src/indicator_factory.rs:6-10`
  - `try_new()` validates `values.len() == rows * cols`:
    - `13-24`
  - but callers can bypass `try_new()` entirely by:
    - constructing the struct directly
    - deserializing into it
- Why it matters:
  - the type visually suggests that `try_new()` is the invariant gate, but the actual public API does not require callers to use it
- Smallest justified fix:
  - safest low-churn options:
    - document that direct construction must preserve the invariant
    - or add a custom `Deserialize` / `TryFrom`-style validation path if stronger guarantees are wanted
  - more invasive option:
    - make fields private and require constructor use
- Review judgment:
  - medium public-API correctness issue
  - low current risk because no production callers were found

### 2. `row()` treats zero-width matrices as “no row,” which makes valid-empty and invalid-access look the same

- Evidence:
  - `try_new()` accepts `rows * cols == 0` when `values` is empty:
    - `13-24`
  - `row()` returns `None` when `self.cols == 0` even if `row < self.rows`:
    - `26-31`
- Why it matters:
  - a zero-column matrix is representable, but its row semantics are ambiguous
  - consumers cannot distinguish:
    - “requested row exists but is empty”
    - “row is out of bounds”
- Smallest justified fix:
  - decide whether zero-column matrices are legitimate
  - then either:
    - reject them in `try_new()`
    - or make `row()` return `Some(&[])` for in-range zero-width rows
- Review judgment:
  - low-to-medium contract issue
  - semantics-sensitive if behavior changes

## API And Naming

### 1. The module presents itself more broadly than it really is

- Evidence:
  - doc comment:
    - `crates/app_core/src/indicator_factory.rs:73-74`
  - actual public entrypoint is:
    - CPU-only
    - moving-average-only
    - uniform-step-grid-only
    - `75-116`
- Why it matters:
  - “indicator factory” implies broader multi-indicator responsibility than the file currently owns
  - the implementation is a narrow wrapper over `double_ma`
- Smallest justified fix:
  - safest option:
    - keep the API stable but rewrite the doc comment/module framing to say explicitly that this is a CPU moving-average batch shim
  - defer-later:
    - rename only if the project decides this should remain a long-lived public surface
- Review judgment:
  - medium naming/documentation issue
  - safe-now as documentation

### 2. Response normalization is slightly asymmetric

- Evidence:
  - `ma_type` is trimmed and lowercased:
    - `85`
  - `ma_source` is trimmed but not lowercased:
    - `86`
  - the response returns those normalized values:
    - `110-114`
- Why it matters:
  - this is small, but it is observable API behavior
  - asymmetry is not necessarily wrong; it is just undocumented
- Smallest justified fix:
  - document the normalization policy or align it intentionally
- Review judgment:
  - low-severity API issue

## Error Handling

### 1. The string-based error surface is acceptable for a thin wrapper, but it mostly forwards lower-level details

- Evidence:
  - `try_new()`, `validate_uniform_step()`, and `compute_moving_average_batch_cpu()` all return `Result<_, String>`:
    - `13-24`
    - `53-71`
    - `81-116`
  - wrapper forwards ALMA and MA-matrix errors from `double_ma`:
    - `96-107`
- Why it matters:
  - this is fine today because the file is only a thin boundary
  - if this surface becomes a real supported external API, typed errors or a more intentionally documented error vocabulary would be stronger
- Smallest justified fix:
  - do not force a typed-error rewrite now
  - keep current messages stable and document the wrapper role
- Review judgment:
  - acceptable as-is
  - document-only

## Module Boundaries And Technical Debt

### 1. This is effectively a facade over `double_ma`, not an owner of indicator behavior

- Evidence:
  - ALMA parameter resolution:
    - `crates/app_core/src/double_ma.rs:10098`
  - MA matrix generation:
    - `crates/app_core/src/double_ma.rs:10559`
  - wrapper-local logic is limited to:
    - string trimming
    - uniform-step validation
    - DTO assembly
- Why it matters:
  - the narrow wrapper is fine, but the public surface should not imply standalone ownership of indicator logic
  - absence of in-repo production callers suggests this may be an anticipatory or lightly-used API
- Smallest justified fix:
  - keep the wrapper if it is useful for future consumers
  - document that real behavior is delegated to `double_ma`
- Review judgment:
  - moderate boundary-clarity issue
  - no large refactor justified

## Docs And Examples

### 1. The public DTOs and wrapper need a more honest contract description

- Evidence:
  - no rustdoc explains:
    - whether `IndicatorMatrixF64` direct construction is supported
    - whether zero-width matrices are valid
    - that only moving averages are supported
    - that periods must be strictly increasing with uniform step
- Smallest justified fix:
  - add rustdoc to:
    - `IndicatorMatrixF64`
    - `MovingAverageBatchRequest`
    - `MovingAverageBatchResponse`
    - `compute_moving_average_batch_cpu`

## Tests And Missing Verification

### 1. Wrapper-local coverage is thin, but the delegated `double_ma` surface has stronger supporting tests

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core indicator_factory::tests:: -- --nocapture`
      - passed (`2` tests)
    - `cargo test -p ta_app_core cpu_ma_params_gaussian_poles_changes_output -- --nocapture`
      - passed
    - `cargo test -p ta_app_core cpu_ma_matrix_rowwise_scalar_fallback_fills_skipped_rows_with_nan -- --nocapture`
      - passed
  - file-local tests cover:
    - smoke behavior
    - non-uniform-period rejection
    - `137-163`
- Missing wrapper-local tests I would want before changing it:
  - empty `ma_type`
  - empty `ma_source`
  - empty `periods`
  - single-period success
  - invalid ALMA params
  - `IndicatorMatrixF64::row()` semantics for zero-width rows
  - direct invariant violation handling if deserialization or public construction remains supported

## Performance, Allocation, And Runtime Concerns

### 1. This file is not the place to chase performance

- Evidence:
  - wrapper cost is negligible relative to `ma_matrix_cpu_f64`
  - there are no allocations or loops here large enough to matter outside the delegated matrix build
- Review judgment:
  - performance changes should target `double_ma`, not this facade

## Code-Quality Improvement Register

- Safe now:
  - clarify the module/doc wording
  - document or harden `IndicatorMatrixF64` invariants
  - add wrapper-local tests for empty/invalid inputs
  - decide and document zero-width matrix row semantics
- Coordination required:
  - any field-privacy change on `IndicatorMatrixF64`
  - any rename if external callers may already exist outside the repo
- Defer until later pass:
  - typed errors
  - larger redesign of this public surface

## Technical-Debt Reduction Register

- Safe to reduce now:
  - misleading “factory-like” framing
  - missing docs on DTO invariants
  - missing wrapper-local tests
- Coordination required:
  - private-field / custom-deserialize changes
- Useful debt that should not be “cleaned up” casually:
  - delegation to `double_ma`; this is the correct place for real MA behavior today

## Performance Opportunity Register

- Safe structural improvements:
  - none meaningful beyond clearer docs and tighter invariants
- Benchmark-sensitive candidates:
  - none in this file
- Not the right place to chase performance:
  - wrapper-level micro-optimizations

## Observability And Diagnostics Gaps

- the file does not say whether direct DTO construction is supported
- the file does not explain zero-width matrix semantics
- the doc comment does not make the CPU-only, MA-only scope explicit enough

## Implementation Staging Guidance

- Safe now:
  - docs
  - tests
  - zero-width semantics clarification
- Coordination required:
  - field privacy or custom deserialization changes
- Defer until later pass:
  - larger public-API redesign

## Change Blast Radius

- Local only:
  - docs
  - tests
  - zero-width semantics if no external callers depend on current behavior
- Same subsystem:
  - constructor/deserialization invariant hardening
- Cross-backend / frontend-backend contract:
  - DTO field visibility or serialized-shape changes

## Open Questions Or Assumptions

- assumption:
  - the current absence of in-repo production callers means this surface is low-risk to document more honestly now
- open question:
  - whether any external consumer already depends on direct deserialization into `IndicatorMatrixF64`

## Recommended Action

`Small surgical refactor`

The right next pass is:

- make the scope honest in docs
- tighten or explicitly document DTO invariants
- add a few wrapper-local tests

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core indicator_factory::tests:: -- --nocapture`
- `cargo test -p ta_app_core cpu_ma_params_gaussian_poles_changes_output -- --nocapture`
- `cargo test -p ta_app_core cpu_ma_matrix_rowwise_scalar_fallback_fills_skipped_rows_with_nan -- --nocapture`
- add and run focused tests for:
  - empty `ma_type`
  - empty `ma_source`
  - zero-width row semantics
  - invariant preservation if `Deserialize` changes

## Regression Risk And Feature-Preservation Constraints

- do not move real MA semantics out of `double_ma` just to make this file “self-contained”
- avoid DTO shape changes unless there is a clear compatibility plan
- if invariants are tightened, treat that as a public-contract change and verify whether any external consumer depends on direct construction/deserialization
