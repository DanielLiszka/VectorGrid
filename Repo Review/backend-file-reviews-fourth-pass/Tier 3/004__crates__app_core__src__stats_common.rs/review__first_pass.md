# Review Packet: `crates/app_core/src/stats_common.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/stats_common.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful utility file, but not fully review-ready because the quantile convention is implicit, non-finite fallback semantics are only partly tested, and the file does not state whether these are mathematically exact or intentionally coarse helpers`
- Recommended action bucket: `Document only`
- Sensitivity tags: `Public-surface`, `Stats-helper`, `Low-compute`, `Semantics-sensitive`

## What This File Owns

`stats_common.rs` owns a few tiny shared statistical helpers:

- rounded-index quantile:
  - `quantile`: `1-11`
- median wrappers:
  - `median`: `13-15`
  - `median_u32`: `41-48`
- worst-case extrema helpers:
  - `worst_min`: `17-27`
  - `worst_max`: `29-39`
  - `worst_min_u32`: `50-52`
- file-local tests:
  - `54-77`

It does not own:

- walk-forward quantile selection logic in `validation_pipeline.rs`
- Sharpe or probability-gate statistics

## Classification

- Public surface: yes
  - all helpers are `pub`
- Hot path: no
- CUDA/runtime-critical: no
- Strategy-local: no
- Low-risk support code: mostly yes
  - but semantics are shared, so documentation still matters

## Direct Interactions

### Backend interaction map

- broader validation/statistics surfaces:
  - `crates/app_core/src/validation_pipeline.rs`
  - `crates/app_core/src/validation_shared.rs`
  - any shared reporting or ranking code that needs cheap extrema summaries

### Downstream verification impact if changed

- changing quantile convention changes percentile-like summaries
- changing non-finite fallback semantics changes worst-case summary behavior for sparse or dirty data

## Standards And Research Log

This packet was written after targeted web research focused on Rust slice-selection/sorting guarantees because the module’s statistical semantics depend more on algorithmic convention than on heavy computation.

### Sources consulted

1. Rust API Guidelines documentation guidance, `C-FAILURE`
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html#function-docs-include-error-panic-and-safety-considerations-c-failure>
   - Why it was consulted:
     - the file exposes public numerical helpers with implicit fallback semantics
   - Standard informed:
     - public numerical helpers should explain approximation or sentinel policy

2. `slice::sort_unstable`
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html#method.sort_unstable>
   - Why it was consulted:
     - `median_u32` sorts a copied vector before selecting the midpoint
   - Standard informed:
     - unstable in-place sorting is a good default for simple numeric order statistics when stability is irrelevant

3. `slice::select_nth_unstable`
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html#method.select_nth_unstable>
   - Why it was consulted:
     - related selection semantics matter when deciding whether this file should stay simple or adopt more optimized order-statistic selection
   - Standard informed:
     - the module can remain simple until there is a demonstrated need for in-place nth-element optimization

## Findings

## Correctness And Safety

### 1. `quantile` uses a rounded-index convention that should be treated as an explicit policy choice

- Evidence:
  - index is computed by rounding `q * (len - 1)`: `1-11`
- Why this matters:
  - there are many valid quantile conventions
  - without docs, callers may assume interpolation or a different percentile definition
- Smallest justified fix:
  - document that this is a coarse rounded-index quantile helper, not an interpolated statistical estimator

## API And Naming

### 2. The helper names are fine, but their sentinel behavior is under-explained

- Evidence:
  - empty `quantile` returns `0.0`: `2-4`
  - `worst_min` / `worst_max` collapse non-finite-only inputs to `0.0`: `17-39`
  - `worst_min_u32` collapses empty input to `0`: `50-52`
- Why this matters:
  - these defaults are practical for summary code, but they are contract choices
- Smallest justified fix:
  - document the empty-input fallback policy

## Error Handling

### 3. The file intentionally does not use `Result`, which is fine as long as callers know these are forgiving helpers

- Evidence:
  - all functions return plain numeric values
- Why this matters:
  - this is appropriate for lightweight summary helpers
  - but the difference between actual zero and empty/non-finite fallback zero should not stay implicit
- Smallest justified fix:
  - document it rather than introduce fallible APIs

## Module Boundaries And Technical Debt

### 4. This file should stay small and mathematically humble

- Evidence:
  - the functions are deliberately tiny and do not attempt sophisticated statistics
- Why this matters:
  - it would be easy to over-expand this file into a grab-bag of mixed-quality statistical utilities
- Recommended posture:
  - keep it limited to coarse shared helpers

## Docs And Examples

### 5. The module needs a brief contract note, not a redesign

- Evidence:
  - no rustdoc is present on any function
- Smallest justified fix:
  - add short rustdoc describing quantile convention and empty/non-finite fallbacks

## Tests And Missing Verification

### 6. The tests are good for current behavior, but there is no explicit lock on the rounded-index quantile convention for nontrivial `q`

- Evidence:
  - tests currently cover `q = 0.0`, `0.5`, and `1.0`: `58-63`
- Missing coverage:
  - nontrivial quantiles such as `0.25` and `0.75`
  - `median_u32` on even-length inputs
- Smallest justified fix:
  - add a couple of convention-locking tests if this file is edited

## Performance, Allocation, And CUDA Concerns

### 7. Performance is intentionally secondary here

- Evidence:
  - only `median_u32` allocates a copied vector: `41-48`
- Safe-now conclusion:
  - leave the simple implementation alone unless profiling proves this helper matters

## Code-Quality Improvement Register

- Safe now:
  - document quantile semantics
  - add a couple more convention-locking tests
- Defer:
  - swapping in nth-element selection or richer quantile logic without a demonstrated need

## Technical-Debt Reduction Register

- Safe now:
  - documentation
  - a few extra tests
- Defer:
  - any statistical API expansion
- Do not casually remove:
  - non-finite-tolerant worst-case helpers

## Performance Opportunity Register

- Safe structural work:
  - none required
- Benchmark-sensitive:
  - only relevant if this file becomes part of a measured bottleneck

## Recommended Action

`Document only`

The file is fine as-is. The only high-value work is clarifying the quantile and fallback conventions so callers do not project stronger statistical meaning onto these helpers than they actually provide.

## Exact Local Verification Required If Changed

1. File-local stats-common tests:
   - `cargo test -p ta_app_core stats_common::tests:: -- --nocapture`
2. If quantile convention changes:
   - rerun the validation/statistics surfaces that consume percentile-like summaries

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core stats_common::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not silently change the quantile convention
- Do not change empty/non-finite fallback outputs without auditing downstream summary consumers
- Do not optimize this file unless profiling proves it matters
