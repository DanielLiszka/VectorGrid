# Review Packet: `crates/app_core/src/validation_stats.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/validation_stats.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Small helper module, but not fully review-ready because its statistical assumptions and gate semantics are only implicit`
- Recommended action bucket: `Document only`
- Sensitivity tags: `Statistics`, `Validation-gates`, `Low-compute`, `Semantics-sensitive`

## What This File Owns

`validation_stats.rs` owns a narrow validation-probability helper surface:

- probability-gate sanitization:
  - `sanitize_probability_gate`: `crates/app_core/src/validation_stats.rs:1-9`
- PSR/DSR wrapper:
  - `compute_psr_dsr`: `11-34`
- gate evaluation:
  - `passes_probability_gates`: `36-45`
- file-local tests:
  - `48-95`

## Classification

- Public surface: yes
  - used by `double_ma.rs`, `composite_cpu.rs`, `gpu_backtest_kernel.rs`, and `validation_pipeline.rs`
- Hot path: no
- Contract-critical: yes
  - these helpers affect approval gates and validation warnings
- Support code: yes

## Direct Interactions

### Backend interaction map

- consumed directly by:
  - `crates/app_core/src/composite_cpu.rs:1411`
  - `crates/app_core/src/double_ma.rs:24258-24265`
  - `crates/app_core/src/gpu_backtest_kernel.rs:5092-5157`
  - `crates/app_core/src/validation_pipeline.rs:268-298`

### Downstream verification impact if changed

- changing PSR/DSR assumptions changes candidate approval and robust-score downstream decisions
- changing gate semantics changes whether missing or out-of-range probability inputs are tolerated

## Standards And Research Log

This packet was written after targeted web research focused on the underlying Sharpe-ratio methodology.

### Sources consulted

1. David H. Bailey and Marcos Lopez de Prado, *The Deflated Sharpe Ratio*
   - Source: <https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf>
   - Why it was consulted:
     - this file wraps PSR/DSR computations for validation gating
   - Standard informed:
     - probability-based Sharpe metrics are methodology-sensitive and should keep their current assumptions explicit

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the helper signatures hide several policy choices
   - Standard informed:
     - gate behavior and fallback semantics should be documented locally

## Findings

## Statistical Assumptions

### 1. `compute_psr_dsr` hardcodes statistical-reference assumptions without documenting them

- Evidence:
  - PSR call uses reference sharpe `0.0`, skew `0.0`, kurtosis `3.0`
  - DSR call uses the same distribution assumptions plus trial/peer inputs: `11-22`
- Why this matters:
  - these are methodology choices, not mechanical pass-throughs
  - future edits could change them casually without recognizing that approval behavior would drift
- Smallest justified fix:
  - add rustdoc stating the current reference assumptions and why they are intentionally delegated through `sharpe_stats`

## Gate Semantics

### 2. Missing probability stats are treated as failures when thresholds are present, and that policy is correct but under-documented

- Evidence:
  - `passes_probability_gates` uses `unwrap_or(0.0)` when a threshold is set: `36-45`
  - tests lock this in: `90-95`
- Why this matters:
  - this is conservative and sensible, but it is a policy choice callers should know
- Smallest justified fix:
  - document that missing PSR/DSR behaves like failure only when the corresponding threshold is requested

## Input Sanitization

### 3. Invalid gate values are clamped or dropped rather than rejected

- Evidence:
  - `sanitize_probability_gate` clamps finite inputs to `[0, 1]` and drops non-finite values: `1-9`
- Why this matters:
  - callers may assume invalid thresholds error loudly, but today they normalize quietly
- Smallest justified fix:
  - document the clamping behavior so threshold handling is explicit

## Tests And Missing Verification

### 4. The tests are appropriately narrow and already capture the core semantics

- Evidence:
  - clamping, threshold logic, bounded output, and missing-stat behavior are all covered: `48-95`
- Safe-now conclusion:
  - no structural change is justified here

## Code-Quality Improvement Register

- Safe now:
  - document PSR/DSR assumptions
  - document missing-stat gate behavior
  - document clamping/drop rules
- Defer:
  - any methodology change to PSR/DSR inputs

## Technical-Debt Reduction Register

- Safe now:
  - rustdoc only
- Defer:
  - changing probability methodology
- Do not casually remove:
  - current conservative missing-stat gate behavior

## Performance Opportunity Register

- Safe structural work:
  - none needed
- Benchmark-sensitive:
  - none

## Recommended Action

`Document only`

The implementation is already tight. The important improvement is to document its approval semantics so later changes do not inadvertently relax or reinterpret the current validation policy.

## Exact Local Verification Required If Changed

1. File-local tests:
   - `cargo test -p ta_app_core validation_stats::tests:: -- --nocapture`
2. Representative caller:
   - `cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core validation_stats::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change the hardcoded PSR/DSR reference assumptions without coordinated review of validation outputs
- Do not relax missing-stat gate behavior casually; it is currently conservative by design
