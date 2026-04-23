# Review Packet: `crates/app_core/src/double_ma_result.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_result.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful result-shaping module with one real observability/contract weakness: internal index/variant mismatches can be silently collapsed into missing params or missing matrix metadata rather than surfacing as invariant failures`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Internal-result-surface`, `Observability-sensitive`, `Low-throughput-risk`

## What This File Owns

This module owns the final transformation from raw optimizer output into the richer `BackendOptimizationResult` surface used by the rest of the backend and UI layers.

It owns:

- result-build context:
  - `BackendResultContext`: `crates/app_core/src/double_ma_result.rs:15-39`
- MA param resolution for result rows:
  - `resolve_params`: `41-65`
- grid matrix metadata construction:
  - `build_grid_matrix_index`: `67-104`
- final result assembly:
  - `build_backend_optimization_result`: `106-255`

## Classification

- Public surface: no
  - `pub(crate)` only
- Hot path: moderate
  - every Double-MA optimization result is shaped through this module
- CUDA/runtime-critical: indirect
  - includes route, residency, and GPU timing metadata but does not execute kernels
- Strategy-local: yes
- Low-risk support code: mostly yes

## Direct Interactions

- consumed from:
  - `crates/app_core/src/double_ma.rs:27`
- depends on DTOs defined in:
  - `crates/app_core/src/double_ma.rs`
- uses matrix index helper from:
  - `crates/app_core/src/matrix_index.rs`
- consumes raw optimizer output from:
  - `ta_optimizer::OptimizationResult`

## Standards And Research Log

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this module is an important contract-shaping boundary even if it is crate-private
   - Standard informed:
     - result shaping should not silently discard important invariant failures

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this module maps many internal fields into user-visible result DTOs
   - Standard informed:
     - boundary modules should make fallback/omission behavior explicit

3. Rust Book, How to Write Tests
   - Source: <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>
   - Why it was consulted:
     - result shapers are best protected by invariant and preservation tests
   - Standard informed:
     - preserve-value tests should accompany any silent-fallback behavior

## Findings

## Correctness And Safety

### 1. `resolve_params` silently drops MA param variants when the raw MA IDs are out of range

- Evidence:
  - fast/slow param lookup uses:
    - `.get(...).cloned().unwrap_or(None)`: `crates/app_core/src/double_ma_result.rs:48-55`
- Why it matters:
  - if an upstream invariant is broken and `fast_ma_id` or `slow_ma_id` falls outside the variant arrays, the result surface quietly loses the MA params instead of signaling an internal mismatch
  - that can make downstream drilldown/follow-up behavior harder to diagnose
- Smallest justified fix:
  - either assert/return an internal error on out-of-range MA IDs
  - or at minimum emit a route/result diagnostic note instead of silently returning `None`
- Review judgment:
  - real observability and contract-hardening gap

## Module Boundaries And Technical Debt

### 1. Grid-matrix metadata omissions are silently collapsed to `None`

- Evidence:
  - if `DoubleMaFastLtSlowIndex::new` fails, the helper returns `(None, None, None)`:
    - `crates/app_core/src/double_ma_result.rs:82-103`
  - if any top param cannot be mapped to a column, `top_cols` is cleared and then omitted:
    - `85-99`
- Why it matters:
  - the result surface loses matrix metadata without any explicit signal that an internal mapping invariant was violated
- Smallest justified fix:
  - keep the `None` surface if desired, but attach a diagnostic note when matrix metadata was suppressed due to an internal mismatch
- Review judgment:
  - low-to-medium observability debt

## Tests And Missing Verification

### 1. Existing preservation tests are good, but they do not cover invariant-failure paths

- Evidence:
  - current tests focus on successful field preservation:
    - `262+`
  - no direct tests cover:
    - out-of-range `fast_ma_id` / `slow_ma_id`
    - matrix-index omission behavior
- Smallest justified fix:
  - add focused tests for silent-omission paths so the behavior is intentional rather than accidental

## Performance, Allocation, And CUDA Concerns

### 1. No meaningful performance issue was identified in this file

- Review judgment:
  - not a candidate for perf work

## Recommended Action

`Small surgical refactor`

The module is structurally fine. The worthwhile improvement is to stop silently hiding internal variant/index mismatches.

## Exact Local Verification Required If Changed

- existing `build_backend_result_*` tests in this module
- new tests for out-of-range MA IDs and matrix-index omission behavior

## Regression Risk And Feature-Preservation Constraints

- preserve current successful result-shaping behavior
- if omission paths are made stricter, ensure callers and tests are updated together
