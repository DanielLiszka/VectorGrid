# Review Packet: `crates/app_core/src/sharpe_stats.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/sharpe_stats.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Compact and useful, but not fully review-ready because the statistical assumptions are only lightly documented, the numerical approximation choices are implicit, and edge-case behavior is specified more by code than by contract`
- Recommended action bucket: `Document only`
- Sensitivity tags: `Public-surface`, `Stats-helper`, `Numerical`, `Semantics-sensitive`, `Low-compute`

## What This File Owns

`sharpe_stats.rs` owns lightweight helpers for probabilistic and deflated Sharpe-style calculations used by validation logic.

It owns:

- serialized summary struct:
  - `SharpeStats`: `4-7`
- normal CDF and inverse-CDF approximations:
  - `normal_cdf`: `9-24`
  - `normal_inv_cdf`: `26-76`
- Sharpe-derived metrics:
  - `probabilistic_sharpe_ratio`: `78-98`
  - `mean_std`: `100-121`
  - `expected_max_sharpe`: `123-131`
  - `deflated_sharpe_ratio`: `133-144`
  - `minimum_track_record_length`: `146-178`
- file-local tests:
  - `180-223`

It does not own:

- probability-gate policy:
  - `crates/app_core/src/validation_stats.rs`
- validation pipeline ranking/gating:
  - `crates/app_core/src/validation_pipeline.rs`

## Classification

- Public surface: yes
  - multiple helpers are `pub`
- Hot path: no
  - used in validation/reporting logic, not the innermost trading loops
- CUDA/runtime-critical: no
- Strategy-local: no
  - shared statistical helper
- Low-risk support code: only partly
  - compute cost is low, but numerical semantics matter

## Direct Interactions

### Backend interaction map

- validation probability-gate wrapper:
  - `crates/app_core/src/validation_stats.rs:10-27`
- broader validation workflow and reporting surfaces:
  - `crates/app_core/src/validation_pipeline.rs`
  - `crates/app_core/src/validation_shared.rs`

### Downstream verification impact if changed

- changing approximation formulas or edge-case defaults affects PSR/DSR gating behavior
- changing `minimum_track_record_length` semantics affects any workflow that surfaces confidence thresholds or credibility guidance

## Standards And Research Log

This packet was written after targeted web research focused on the documented literature behind the Sharpe-ratio-efficient-frontier family and on behaviorally important internal documentation for numerical helpers.

### Sources consulted

1. Rust API Guidelines documentation guidance, `C-FAILURE`
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html#function-docs-include-error-panic-and-safety-considerations-c-failure>
   - Why it was consulted:
     - these helpers encode non-trivial numerical assumptions but expose almost no contract commentary
   - Standard informed:
     - numerical helpers should document important behavior, approximations, and edge-case policy

2. “The Sharpe ratio efficient frontier” metadata record
   - Source: <https://www.econbiz.de/Record/the-sharpe-ratio-efficient-frontier-bailey-david/10010061572>
   - Why it was consulted:
     - this file implements probabilistic/deflated Sharpe-family helpers associated with Bailey / Lopez de Prado-style methodology
   - Standard informed:
     - the formulas should be treated as statistically meaningful contracts, not generic utility math

3. `std::result`
   - Source: <https://doc.rust-lang.org/std/result/>
   - Why it was consulted:
     - the file returns values directly rather than surfacing statistical invalidity as fallible results
   - Standard informed:
     - if invalid inputs are intentionally folded into sentinel outputs rather than errors, that should be made explicit

## Findings

## Correctness And Safety

### 1. The edge-case fallback policy is sensible but undocumented

- Evidence:
  - `probabilistic_sharpe_ratio` returns `0.5` when inputs are non-finite or `samples < 2`: `85-88`
  - `minimum_track_record_length` returns `None` for invalid or non-credible input states: `152-166`
- Why this matters:
  - these are reasonable defensive choices, but they are part of the observable contract
  - maintainers should not have to infer policy from code
- Smallest justified fix:
  - add rustdoc explaining the sentinel behavior for invalid or insufficient inputs

## API And Naming

### 2. `SharpeStats` is under-specified relative to the helper surface around it

- Evidence:
  - `SharpeStats` only contains `psr` and `dsr`: `4-7`
  - the file also exposes `minimum_track_record_length` and `expected_max_sharpe`, which are related but not represented in the struct
- Why this matters:
  - the struct name reads as broader than it really is
  - consumers may assume it is the canonical payload for all Sharpe-derived outputs
- Smallest justified fix:
  - document that `SharpeStats` is intentionally a narrow summary struct, not the full surface of the module

## Error Handling

### 3. Statistical invalidity is normalized into values instead of surfaced explicitly

- Evidence:
  - non-finite and undersized cases fall back to `0.5`, `0.0`, `sr_mean`, or `None`: `78-178`
- Why this matters:
  - this is appropriate for a utility helper used in gating/reporting
  - however, callers need to know when a neutral or empty output means “not enough information” rather than “true statistical neutrality”
- Smallest justified fix:
  - document the fallback policy and avoid introducing partial `Result` handling inconsistently

## Module Boundaries And Technical Debt

### 4. The normal CDF / inverse CDF approximations are implementation details that should be treated as frozen unless there is a proven numerical issue

- Evidence:
  - `normal_cdf` uses an Abramowitz-and-Stegun-style approximation: `9-24`
  - `normal_inv_cdf` uses an Acklam-style rational approximation: `26-76`
- Why this matters:
  - these approximations are reasonable and common, but changing them can create small numerical drift in downstream thresholds
- Recommended posture:
  - document the approximations and leave them alone unless there is a demonstrated accuracy or stability problem

## Docs And Examples

### 5. The file needs brief statistical context

- Evidence:
  - comments identify the approximations, but there is no rustdoc on the public functions
- Why this matters:
  - maintainers need to know whether `kurtosis` is expected as raw kurtosis or excess-adjusted handling, what `num_trials` means, and why invalid inputs collapse to sentinel outputs
- Smallest justified fix:
  - add short rustdoc clarifying inputs, assumptions, and edge-case outputs

## Tests And Missing Verification

### 6. Current tests are good smoke coverage but light on numerical-boundary checks

- Evidence:
  - tests cover monotonicity, range, determinism, and confidence sensitivity: `184-223`
- Missing coverage:
  - non-finite input behavior
  - near-boundary `confidence_level` handling
  - `samples < 2` sentinel behavior
  - extreme `num_trials` stability
- Smallest justified fix:
  - add a few explicit edge-case tests before changing formulas or fallback policy

## Performance, Allocation, And CUDA Concerns

### 7. Performance is not the priority; numerical contract stability is

- Evidence:
  - the file is compute-light and allocation-free
- Safe-now conclusion:
  - avoid optimizing numerical helpers that are not a measured bottleneck
- Real risk:
  - silent statistical drift from approximation or fallback-policy changes

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc for statistical assumptions and fallback policy
  - add edge-case tests for invalid inputs and near-boundary confidence levels
- Defer:
  - replacing local approximations with external crates unless the repo wants a broader numerical-dependency tradeoff

## Technical-Debt Reduction Register

- Safe now:
  - document neutral/default outputs
  - document the meaning of `kurtosis`, `num_trials`, and `peer_*`
- Defer:
  - numerical-library migration
- Do not casually remove:
  - the current deterministic approximation approach

## Performance Opportunity Register

- Safe structural work:
  - none required
- Benchmark-sensitive:
  - none

## Recommended Action

`Document only`

The implementation is acceptable and already has basic tests. The best improvement is making the statistical contract explicit so later edits do not reinterpret the formulas or their fallback behavior.

## Exact Local Verification Required If Changed

1. File-local Sharpe tests:
   - `cargo test -p ta_app_core sharpe_stats::tests:: -- --nocapture`
2. Probability-gate wrapper coverage:
   - `cargo test -p ta_app_core validation_stats::tests:: -- --nocapture`
3. If formulas or sentinel behavior change:
   - run representative validation workflow coverage before accepting the change

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core sharpe_stats::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change fallback outputs casually; they feed validation gating semantics
- Do not swap approximation formulas without proving there is a numerical or stability benefit
- Do not reinterpret the input statistics silently; document any intended semantic change first
