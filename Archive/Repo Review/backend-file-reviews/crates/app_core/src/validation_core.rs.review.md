# Review Packet: `crates/app_core/src/validation_core.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/validation_core.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Small but contract-heavy orchestrator; not fully review-ready because tuple semantics and shared normalization decisions are only implicit`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Validation-orchestration`, `Generic-contract`, `Low-compute`, `Semantics-sensitive`

## What This File Owns

`validation_core.rs` owns a thin orchestration wrapper around validation candidate collection and finalization:

- generic validation runner:
  - `run_validation_candidates`: `crates/app_core/src/validation_core.rs:7-51`
- file-local tests:
  - `62-321`

Its job is small, but it bridges caller-provided candidate processing with the shared validation pipeline and CSCV finalization.

## Classification

- Public surface: yes
  - used from `double_ma_validate.rs` and `composite_validate.rs`
- Hot path: no
- Contract-critical: yes
  - it decides common sample normalization before downstream gates/statistics run
- Low-risk support code: only partially
  - small file, but a subtle semantic bridge

## Direct Interactions

### Backend interaction map

- callers:
  - `crates/app_core/src/double_ma_validate.rs:3507`
  - `crates/app_core/src/composite_validate.rs:12365`
- downstream pipeline hooks:
  - `crates/app_core/src/validation_pipeline.rs:581`
  - `1367`

### Downstream verification impact if changed

- changing candidate normalization changes the observation counts and shared start index passed into later PSR/DSR and CSCV logic
- changing tuple semantics changes every caller-provided `process_one` closure contract

## Standards And Research Log

This packet was written after targeted web research focused on documentation and argument-validation guidance for small generic APIs.

### Sources consulted

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the function is small, but its generic closure contract is not obvious from the signature
   - Standard informed:
     - important tuple/closure semantics should be documented explicitly

2. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why it was consulted:
     - this file normalizes observation counts and shared start points before downstream statistics run
   - Standard informed:
     - contract-heavy helpers should make normalization rules explicit rather than hidden

## Findings

## Contract Clarity

### 1. The `process_one` result tuple is semantically important, but it is weakly typed and undocumented

- Evidence:
  - `process_one` returns `Result<(C, usize, usize), String>`: `16-23`
  - the returned `usize` values are interpreted as candidate `start_t` and sample observations: `33-41`
- Why this matters:
  - callers must know tuple ordering from implementation, not from the type system
  - future mistakes here would silently distort later probability and CSCV calculations
- Smallest justified fix:
  - document the tuple contract, or replace it with a tiny named struct in a later pass

## Normalization Rules

### 2. Shared normalization of `common_start_t` and `sample_obs` is currently implicit

- Evidence:
  - `common_start_t` becomes the max start across candidates: `35-39`
  - each `sample_obs` is floored to at least `2`: `39`
- Why this matters:
  - these are policy choices, not incidental implementation details
  - they affect downstream PSR/DSR and CSCV behavior
- Smallest justified fix:
  - add rustdoc explaining why the common start is maxed and why sample observations are floored at `2`

## API Shape

### 3. Phase names remain stringly typed

- Evidence:
  - `validate_phase` and `cscv_prep_phase` are passed as `&'static str`: `12-13`
- Why this matters:
  - the current design is workable, but callers can still drift on phase naming conventions
- Smallest justified fix:
  - if string phases remain, document expected naming/use; defer introducing a stronger phase enum unless the surface grows

## Tests And Missing Verification

### 4. The tests cover sorting, cancelation, and progress monotonicity well

- Evidence:
  - three focused tests exist and passed: `185-321`
- Residual gap:
  - no test locks down the exact tuple semantics in a named way
- Smallest justified fix:
  - add one comment or test note clarifying `(candidate, start_t, sample_obs)`

## Code-Quality Improvement Register

- Safe now:
  - document tuple semantics
  - document `common_start_t` and `sample_obs` normalization
  - document expected phase-name usage
- Defer:
  - replacing phase strings with a typed enum
  - larger API reshaping

## Technical-Debt Reduction Register

- Safe now:
  - local documentation
- Defer:
  - stronger typed wrapper structs
- Do not casually remove:
  - current `sample_obs.max(2)` floor
  - current `common_start_t.max(...)` behavior

## Performance Opportunity Register

- Safe structural work:
  - none needed
- Benchmark-sensitive:
  - none

## Recommended Action

`Small surgical refactor`

The implementation is fine. The useful work is to make the generic tuple contract and normalization policy explicit so future validation changes do not accidentally distort downstream stats.

## Exact Local Verification Required If Changed

1. File-local tests:
   - `cargo test -p ta_app_core validation_core::tests:: -- --nocapture`
2. Representative validation callers:
   - `cargo test -p ta_app_core double_ma_validate -- --nocapture`
   - `cargo test -p ta_app_core composite_validate -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core validation_core::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not reorder or reinterpret the `process_one` tuple without updating all callers together
- Do not change `common_start_t` or observation-floor rules casually; downstream validation math depends on them
