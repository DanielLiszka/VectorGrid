# Review Packet: `crates/app_core/src/composite_gpu_metric_decode.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_gpu_metric_decode.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Small but important shared decoder; not accept-as-is because the row-layout contract is implicit, shortened rows can partially zero-fill instead of failing clearly, and kernel/host parity depends on undocumented duplicated indexing`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Support-code`, `Shared-GPU-boundary`, `Kernel-ABI-sensitive`, `Parity-sensitive`

## What This File Owns

`composite_gpu_metric_decode.rs` owns the shared host-side decoder for non-double-MA GPU kernel metric rows. It translates a raw `[f32]` row returned by several strategy-specific CUDA wrappers into the shared `ta_strategies::double_ma::Metrics` struct, including clamping/sanitization of extended “core17” risk fields.

This file owns:

- the shared stride constant:
  - `NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE`: `crates/app_core/src/composite_gpu_metric_decode.rs:5`
- scalar sanitization helpers:
  - `clamp_non_negative`: `8-14`
  - `derive_martin_ratio`: `16-29`
- the shared row decoder:
  - `decode_non_double_ma_gpu_metrics_row`: `31-96`

This file does not own:

- the `Metrics` struct definition
  - `crates/strategies/src/double_ma/mod.rs:60-106`
- the CUDA kernel-side row layout
  - `crates/app_core/src/kernels/double_crossover.cu`
- the strategy-local GPU wrappers that allocate output buffers and slice rows
  - `*_gpu_kernel.rs` consumers listed below

## Classification

- Public surface: no
  - all items are `pub(crate)` or private
- Hot path: moderate
  - the decoder runs once per returned GPU combo row, but only does scalar conversion and clamping
- Runtime-critical: yes
  - multiple strategy GPU wrappers depend on it for host-visible metrics
- CUDA/runtime-critical: yes
  - it is a shared host/kernel ABI boundary
- Strategy-local: no
  - it is shared infrastructure for many strategy-local GPU wrappers
- Low-risk support code: not fully
  - the implementation is small, but ABI mistakes here propagate broadly

## Direct Interactions

### Backend interaction map

- shared `Metrics` target type:
  - `crates/strategies/src/double_ma/mod.rs:60-106`
- mirrored CPU-side risk-metric conventions:
  - `crates/strategies/src/double_ma/mod.rs:142-316`
  - especially shared `EPS_DENOM` / `CLAMP_MAX` / Martin-ratio behavior at:
    - `182-183`
    - `268-280`
- CUDA kernel row layout for non-double-MA kernels:
  - `crates/app_core/src/kernels/double_crossover.cu:11010-11033`
  - `crates/app_core/src/kernels/double_crossover.cu:11458-11481`
- distinct Double-MA kernel row layout that includes trade-distribution fields:
  - `crates/app_core/src/kernels/double_crossover.cu:4978-5044`
- direct wrapper consumers:
  - `crates/app_core/src/asr_gpu_kernel.rs`
  - `crates/app_core/src/vcc_gpu_kernel.rs`
  - `crates/app_core/src/vemr_gpu_kernel.rs`
  - `crates/app_core/src/rocb_gpu_kernel.rs`
  - `crates/app_core/src/vcet_gpu_kernel.rs`
  - `crates/app_core/src/mkcs_gpu_kernel.rs`
  - `crates/app_core/src/kqsr_gpu_kernel.rs`
  - `crates/app_core/src/kvdf_gpu_kernel.rs`
  - `crates/app_core/src/carm_gpu_kernel.rs`
  - `crates/app_core/src/cdmc_gpu_kernel.rs`

Representative usage:

- VCC wrapper allocates rows using this stride constant:
  - `crates/app_core/src/vcc_gpu_kernel.rs:489-492`
- VCC wrapper slices exact rows and decodes them through this helper:
  - `crates/app_core/src/vcc_gpu_kernel.rs:930-933`

### Downstream verification impact if changed

- any change to row indexing or sanitization affects all non-double-MA GPU wrappers listed above
- any change to Martin-ratio derivation or clamping affects CPU/GPU parity assertions in composite optimization tests
- any change to short-row behavior affects how kernel ABI drift presents:
  - panic
  - explicit error
  - or silent zero-filled metrics

## Standards And Research Log

This packet was written after targeted external review focused on slice-length handling, recoverable-error expectations for shared decode helpers, and host/kernel contract clarity for CUDA-side row layouts.

### Sources consulted

1. Rust slice docs
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html>
   - Why it was consulted:
     - this file mixes direct indexing with `get(...).unwrap_or(0.0)` against raw metric rows
   - Standard informed:
     - slice indexing and optional access have materially different failure behavior and should be used deliberately

2. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this helper currently has no recoverable error path even though it sits on a shared decode boundary
   - Standard informed:
     - unexpected-but-recoverable shape problems are better surfaced explicitly than hidden as plausible defaults

3. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - even crate-local shared helpers should make invariants obvious when many modules rely on them
   - Standard informed:
     - shared abstractions should be dependable and unsurprising

4. NVIDIA CUDA C++ Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why it was consulted:
     - this file is part of a host/kernel interface where buffer layout and row interpretation must stay aligned
   - Standard informed:
     - host-side interpretation of device output must be treated as an explicit contract, not an informal convention

5. Rust Reference, type-cast expressions
   - Source: <https://doc.rust-lang.org/reference/expressions/operator-expr.html#type-cast-expressions>
   - Why it was consulted:
     - this file casts `f32` metric slots into `u32` counters and durations
   - Standard informed:
     - float-to-int casts are defined, but they do not validate “this should have been an integer” as a contract

## Findings

## Correctness And Safety

### 1. Shortened rows can partially degrade into plausible zero-filled metrics instead of failing clearly

- Evidence:
  - the decoder hard-indexes the first 12 fields:
    - `crates/app_core/src/composite_gpu_metric_decode.rs:32-43`
  - but the extended fields use `get(...).unwrap_or(0.0)`:
    - `44-50`
  - all current wrapper consumers allocate and slice rows using the exact shared stride constant `19`:
    - `crates/app_core/src/vcc_gpu_kernel.rs:489-492`
    - `crates/app_core/src/vcc_gpu_kernel.rs:930-933`
    - analogous use sites exist in the other wrapper modules returned by repo search
  - the kernel-side non-double-MA layout also writes a full 19-field row when the stride is 19:
    - `crates/app_core/src/kernels/double_crossover.cu:11010-11033`
    - `crates/app_core/src/kernels/double_crossover.cu:11458-11481`
- Why it matters:
  - under the current contract, a row shorter than 19 is not a valid alternate format; it is a bug or ABI drift
  - the current decoder can therefore turn a layout mismatch into apparently valid metrics with zeros in the tail fields
- Smallest justified fix:
  - add an explicit length contract at the top of the decoder:
    - `debug_assert_eq!(row.len(), NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE);`
    - and/or promote the decoder to `Result<Metrics, String>` with a clear error on wrong row length
- Review judgment:
  - real correctness/diagnostic issue; safe-now hardening

### 2. Length handling is inconsistent: some row mismatches panic immediately, others are silently tolerated

- Evidence:
  - indices `0..=11` will panic on a too-short row:
    - `32-43`
  - indices `12..=18` silently default to zero:
    - `44-50`
- Why it matters:
  - the caller does not get one coherent failure mode
  - debugging an ABI mismatch becomes harder because behavior depends on which field fell off the row
- Smallest justified fix:
  - use one policy consistently:
    - either explicit length validation up front
    - or an array/chunk-based decode that guarantees the expected layout before field extraction
- Review judgment:
  - safe-now local cleanup with real diagnostic value

### 3. Integer-valued metric slots are cast without checking that the GPU actually returned integral values

- Evidence:
  - `trades_f` is cast with `as u32` after only finite-and-nonnegative checks:
    - `crates/app_core/src/composite_gpu_metric_decode.rs:39`
    - `73-76`
  - `max_underwater_duration_bars_f` is handled the same way:
    - `50`
    - `100-105`
- Why it matters:
  - a malformed kernel row containing `12.5` or similar still decodes into a plausible integer
  - that weakens the host/device ABI boundary because corrupted rows can look valid enough to continue
- Smallest justified fix:
  - keep the casts, but add `debug_assert!(value.fract() == 0.0)` or equivalent checked-path validation in tests/debug builds
- Review judgment:
  - low-cost contract hardening

## API And Naming

### 1. The decoder name does not communicate that trade-distribution fields are intentionally unavailable

- Evidence:
  - the returned `Metrics` struct includes:
    - `closed_trades`, `winning_trades`, `losing_trades`, `gross_profit`, `gross_loss`
    - `crates/strategies/src/double_ma/mod.rs:86-100`
  - this decoder always sets those fields to zero:
    - `crates/app_core/src/composite_gpu_metric_decode.rs:83-88`
  - the non-double-MA kernel row layout only writes through `max_underwater_duration_bars` at slot `18`:
    - `crates/app_core/src/kernels/double_crossover.cu:11026-11033`
    - `11474-11481`
  - the generic Double-MA kernel layout does have later slots for closed-trade stats and gross profit/loss:
    - `crates/app_core/src/kernels/double_crossover.cu:4988-4992`
    - `5040-5044`
- Why it matters:
  - zero in these fields means “not emitted by this row format,” not “the strategy definitely had zero closed trades or zero gross profit/loss”
  - that distinction is not obvious from the function name alone
- Smallest justified fix:
  - add a short doc comment stating that this decoder targets the 19-field non-double-MA GPU layout and intentionally leaves exact trade-distribution fields at default values
- Review judgment:
  - document-only improvement with high clarity value

## Error Handling

### 1. This shared decode boundary currently cannot report malformed rows as recoverable errors

- Evidence:
  - `decode_non_double_ma_gpu_metrics_row` returns `Metrics` directly:
    - `crates/app_core/src/composite_gpu_metric_decode.rs:31`
  - malformed row shape therefore becomes:
    - a panic for early fields
    - silent defaulting for later fields
- Why it matters:
  - this file is shared across many wrappers, so a clearer error path would localize ABI issues quickly
- Smallest justified fix:
  - if changing the signature is acceptable, return `Result<Metrics, String>`
  - if not, at minimum add a strict debug assertion and document the invariant
- Review judgment:
  - safe-now hardening, but signature changes touch many callers

## Module Boundaries And Technical Debt

### 1. Kernel/host row layout is duplicated and implicit rather than anchored by named slot constants

- Evidence:
  - the decoder uses hard-coded slot numbers:
    - `32-50`
  - the CUDA kernels use their own hard-coded slot writes:
    - `11010-11033`
    - `11458-11481`
- Why it matters:
  - the contract currently lives in two places with no local comment tying them together
  - a future kernel row change could break every shared consumer at once
- Smallest justified fix:
  - define named slot constants or add a contract comment directly above the decoder referencing the kernel row layout it mirrors
- Review judgment:
  - safe-now documentation and maintainability fix

### 2. Martin-ratio and clamping logic intentionally mirrors shared CPU risk-metric behavior, but that parity is undocumented

- Evidence:
  - local constants and derivation:
    - `crates/app_core/src/composite_gpu_metric_decode.rs:5-29`
  - similar GPU decode-side Martin-ratio derivation already exists elsewhere:
    - `crates/app_core/src/gpu_backtest_kernel.rs:956-969`
  - shared CPU-side constants and Martin-ratio derivation:
    - `crates/strategies/src/double_ma/mod.rs:182-183`
    - `crates/strategies/src/double_ma/mod.rs:268-280`
- Why it matters:
  - this duplication is understandable because the shared helper is not directly reusable here
  - but without a comment or a parity test, these formulas can drift quietly
- Smallest justified fix:
  - add a short comment that the decoder intentionally mirrors `finalize_risk_metrics` clamping behavior
  - ideally add a unit test here that locks the expected sanitization and Martin-ratio conventions
- Review judgment:
  - low-cost debt reduction with good parity payoff

## Docs And Examples

### 1. The file has no documentation despite being a shared ABI boundary

- Missing documentation topics:
  - expected row length
  - slot ordering
  - which `Metrics` fields are populated from the 19-field non-double-MA row format
  - which `Metrics` fields remain default placeholders
- Smallest justified fix:
  - add module- or function-level documentation describing the 19-slot layout and the intentionally absent trade-summary fields

## Tests And Missing Verification

### 1. This file has no dedicated tests even though it is reused across many GPU wrappers

- Evidence:
  - no local `#[cfg(test)]` module exists in `composite_gpu_metric_decode.rs`
- Current verification observed during this review:
  - `cargo test -p ta_app_core asr_cpu_optimization_emits_non_placeholder_core17_fields -- --nocapture`
    - passed
  - direct shared-math anchor exists in:
    - `crates/app_core/src/gpu_backtest_kernel.rs:18419`
      - `decode_full_metric_rows_restore_omega_and_derive_martin_ratio`
  - attempted representative GPU-parity filters in the default lane:
    - `cargo test -p ta_app_core asr_optimization_gpu_kernel_matches_cpu_small_grid_with_ema_zscore -- --nocapture`
    - `cargo test -p ta_app_core vcc_optimization_gpu_kernel_sanity_matches_cpu_small_grid -- --nocapture`
    - both matched `0` tests in the current default feature/test configuration
- Missing dedicated coverage:
  - row-length contract test
  - sanitization/clamping test for NaN, negative drawdown-risk fields, and Martin-ratio edge cases
  - explicit test that trade-summary fields remain default under the 19-slot layout

## Performance, Allocation, And CUDA Concerns

### 1. No meaningful performance rewrite is justified here

- Evidence:
  - the file performs scalar decode only and allocates nothing
- Review judgment:
  - avoid churn
  - the right work here is contract clarity, not micro-optimization

## Code-Quality Improvement Register

- Safe now:
  - add an explicit row-length contract
  - document the slot layout and absent trade-summary fields
  - anchor the Martin-ratio/clamp duplication to the shared CPU logic with a comment or test
  - add narrow unit tests for malformed rows and sanitization
- Defer:
  - broad refactors to unify all GPU metric layouts unless more than one decoder starts drifting

## Technical-Debt Reduction Register

- Safe-now debt to reduce:
  - implicit slot numbering
  - mixed panic/default behavior on short rows
  - undocumented placeholder fields
- Defer-later debt:
  - extracting a shared row-schema abstraction if more GPU layouts converge on the same pattern

## Performance Opportunity Register

- None worth pursuing now
- treat this file as a correctness and maintainability boundary, not a throughput bottleneck

## Recommended Action

- `Small surgical refactor`

Reasoning:

- the file is small enough that contract hardening is cheap
- the useful work is limited and local:
  - make row length explicit
  - document the exact layout
  - add tiny tests
- no broad rewrite is justified

## Exact Local Verification Required If Changed

If this decoder changes:

- add and run dedicated unit tests in `composite_gpu_metric_decode.rs` for:
  - exact-length row decode
  - short-row handling
  - NaN/negative sanitization
  - Martin-ratio edge cases
- run at least one CPU-side core17-field anchor:
  - `cargo test -p ta_app_core asr_cpu_optimization_emits_non_placeholder_core17_fields -- --nocapture`

If row layout or field semantics change and CUDA-enabled tests are available in the environment:

- run representative GPU parity tests from consumer wrappers, for example:
  - `cargo test -p ta_app_core asr_optimization_gpu_kernel_matches_cpu_small_grid_with_ema_zscore -- --nocapture`
  - `cargo test -p ta_app_core vcc_optimization_gpu_kernel_sanity_matches_cpu_small_grid -- --nocapture`
  - `cargo test -p ta_app_core vcet_optimization_gpu_kernel_sanity_matches_cpu_small_grid -- --nocapture`
  - `cargo test -p ta_app_core cdmc_optimization_gpu_kernel_sanity_matches_cpu_small_grid -- --nocapture`
  - `cargo test -p ta_app_core vemr_validation_gpu_kernel_matches_cpu_metrics -- --nocapture`
  - `cargo test -p ta_app_core vcc_validation_gpu_kernel_matches_cpu_metrics -- --nocapture`
  - `cargo test -p ta_app_core cdmc_validation_gpu_kernel_matches_cpu_metrics -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- do not change the slot layout interpretation without coordinating with every wrapper that slices by the shared stride
- do not start populating trade-summary fields from guessed values; they are either explicitly emitted by the row format or they are not
- do not “simplify” the decoder by silently broadening backward-compatibility rules unless that compatibility requirement is explicitly documented
