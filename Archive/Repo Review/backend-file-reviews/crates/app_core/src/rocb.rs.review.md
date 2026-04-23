# Review Packet: `crates/app_core/src/rocb.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/rocb.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Functionally substantial and broadly reused, but not fully review-ready because the ROCB contract relies on stringly dynamic-indicator lookups, several public helper semantics are only implicit, and the CPU/GPU boundary is harder to reason about than the file’s surface suggests`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Composite-strategy-local`, `CPU-GPU-bridge`, `Validation-sensitive`, `Semantics-sensitive`

## What This File Owns

`rocb.rs` owns the ROCB strategy support layer used by composite optimization and validation flows.

It owns:

- strategy config and defaults:
  - `RocbStrategyParams`: `22-35`
  - `Default`: `57-68`
- resolved-period and signal contracts:
  - `RocbResolvedPeriods`: `71-74`
  - `RocbBreakoutSignals`: `77-82`
  - `RocbRiskSignals`: `84-87`
- CUDA-facing device sweep wrappers:
  - `RocbDeviceMatrix`: `104-123`
  - `RocbPeriodSweepDevice`: `126-149`
- request validation and period resolution:
  - `validate_rocb_params`: `152-166`
  - `resolve_rocb_periods`: `168-183`
- dynamic-indicator dispatch adapters and descriptor resolution:
  - `resolve_output_id`: `255-291`
  - `resolve_param_key`: `293-341`
  - `rocb_indicator_ids`: `343-367`
- CPU/GPU signal acquisition:
  - `compute_rocb_donchian_output_sweep_cuda_device`: `437-461`
  - `compute_rocb_atr_sweep_cuda_device`: `463-480`
  - `compute_rocb_breakout_signals`: `533-583`
  - `compute_rocb_risk_signals`: `585-610`
- execution-start and desired-position construction:
  - `compute_rocb_start_t`: `612-631`
  - `build_rocb_desired_positions_into`: `633-758`
  - `build_rocb_desired_positions`: `760-770`
- file-local unit tests:
  - `773-843`

It does not own:

- composite optimization orchestration:
  - `crates/app_core/src/composite_cpu.rs`
- composite validation workflow policy:
  - `crates/app_core/src/composite_validate.rs`
- CUDA-kernel evaluation:
  - `crates/app_core/src/rocb_gpu_kernel.rs`

## Classification

- Public surface: yes
  - `RocbStrategyParams` is re-exported from `crates/app_core/src/lib.rs:126`
  - many helpers are `pub` even when primarily consumed internally
- Hot path: yes, bounded
  - signal construction and desired-position loops sit directly on optimization/validation paths
- CUDA/runtime-critical: partly
  - CPU behavior is central, and CUDA sweep support feeds the kernel path
- Strategy-local: yes
  - ROCB-specific support module
- Low-risk support code: no
  - small changes here alter strategy behavior and validation parity

## Direct Interactions

### Backend interaction map

- Composite optimization request and execution wiring:
  - `crates/app_core/src/composite_cpu.rs:129-138`
  - `862-888`
  - `5770-6167`
  - `6391-6579`
- Composite validation wiring:
  - `crates/app_core/src/composite_validate.rs:96-106`
  - `2138-2293`
  - `3070-3559`
  - `10518-10585`
- ROCB GPU kernel path:
  - `crates/app_core/src/rocb_gpu_kernel.rs:10`
  - `299-532`
- Public config exposure:
  - `crates/app_core/src/lib.rs:126`
- Tauri-side default construction:
  - `src-tauri/src/commands_validation.rs:3019`

### Downstream verification impact if changed

- changing param defaults affects persisted/configured ROCB requests
- changing resolved-period mapping affects how fast/slow MA ranges map into channel and ATR periods
- changing desired-position rules affects optimization scores, validation parity, and GPU/CPU agreement
- changing indicator-descriptor lookup semantics affects both CPU and GPU preparation paths

## Standards And Research Log

This packet was written after targeted web research focused on parameter-validation posture, serde default-field semantics for persisted request structs, and one-time shared descriptor initialization.

### Sources consulted

1. Rust API Guidelines dependability guidance, `C-VALIDATE`
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html#functions-validate-their-arguments-c-validate>
   - Why it was consulted:
     - the module exposes a request/config struct plus `validate_rocb_params`
   - Standard informed:
     - request-driven strategy helpers should make invalid states fail explicitly

2. Rust API Guidelines documentation guidance, `C-FAILURE`
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html#function-docs-include-error-panic-and-safety-considerations-c-failure>
   - Why it was consulted:
     - multiple public helpers return `Result<String>` or encode implicit strategy behavior
   - Standard informed:
     - public strategy helpers should document behaviorally important invariants and failure modes

3. Serde field defaults
   - Source: <https://serde.rs/attr-default.html>
   - Why it was consulted:
     - `RocbStrategyParams` uses many `#[serde(default = ...)]` fields
   - Standard informed:
     - persisted/default-compatible config surfaces should treat default-value evolution as compatibility-sensitive

4. `std::sync::OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why it was consulted:
     - the indicator-descriptor cache is initialized through `OnceLock`
   - Standard informed:
     - one-time shared initialization is appropriate here, but behavior should stay deterministic and well-scoped

## Findings

## Correctness And Safety

### 1. The desired-position builder silently returns on shape mismatch

- Evidence:
  - `build_rocb_desired_positions_into` returns early if breakout/risk lengths do not match `close.len()`: `649-651`
- Why this matters:
  - this avoids panics, but it also turns a real upstream wiring failure into a quiet zero/partial result path
  - because the function is `pub`, callers do not get a signal that their inputs were malformed
- Smallest justified fix:
  - keep the no-panic posture, but either document the silent-return contract or add a fallible variant for upstream callers that need diagnostics

### 2. “Expected-or-scan” first-valid logic is correct-looking but under-explained

- Evidence:
  - `first_finite_idx_expected_or_scan`: `195-201`
  - `first_joint_finite_idx_expected_or_scan`: `203-219`
  - these are used to derive start points for both breakout and ATR series
- Why this matters:
  - the fallback from expected index to scan is important for resilience against indicator warmup differences
  - without comments, it is easy to “simplify” this into a stricter but more fragile assumption
- Smallest justified fix:
  - document why the expected index is only an optimization hint and not a hard invariant

## API And Naming

### 3. The dynamic-indicator boundary is stringly typed in the critical places

- Evidence:
  - indicator IDs and output IDs are resolved by string: `255-341`
  - signal acquisition calls indicators by string name such as `"donchian"` and `"atr"`: `533-610`
- Why this matters:
  - the approach is flexible and consistent with the dynamic-indicator system, but it weakens local readability and compile-time guarantees
  - typos or catalog drift are caught only at runtime
- Smallest justified fix:
  - keep the dynamic model, but add comments documenting which catalog outputs/param keys ROCB requires and why the descriptor cache exists

## Error Handling

### 4. The public helper surface mixes explicit `Err` paths and silent fallback behavior

- Evidence:
  - invalid config values are rejected in `validate_rocb_params`: `152-166`
  - all-non-finite indicator outputs are rejected: `579-583` and `606-610`
  - shape mismatch inside desired-position construction is silently ignored: `649-651`
- Why this matters:
  - different parts of the module apply different failure policies
  - that can be fine, but it is not obvious to callers which helpers are strict and which are lenient
- Smallest justified fix:
  - document the contract split and avoid broadening silent fallbacks further

## Module Boundaries And Technical Debt

### 5. The file owns both strategy semantics and GPU-preparation support, which raises maintenance cost

- Evidence:
  - strategy semantics live in the desired-position loop: `633-758`
  - GPU-preparation/device sweep wrappers live in the same file: `104-149`, `437-480`
- Why this matters:
  - the code is still coherent today, but it combines business logic, indicator-adapter glue, and CUDA-facing matrix wrappers in one module
  - future edits risk mixing semantics changes with runtime plumbing changes
- Smallest justified fix:
  - keep the file whole for now
  - document the sub-roles explicitly and defer any split until ROCB changes become frequent enough to justify a strategy-local submodule layout

## Docs And Examples

### 6. ROCB behavior is mostly encoded in code, not in local documentation

- Evidence:
  - the file has no rustdoc on the config struct or major public helpers
- Why this matters:
  - the module is strategy-local, but it also exposes a persisted request/config surface and reusable public helpers
  - missing docs raise regression risk for later cleanup passes
- Smallest justified fix:
  - add rustdoc for `RocbStrategyParams`, `resolve_rocb_periods`, `compute_rocb_start_t`, and the desired-position builders

## Tests And Missing Verification

### 7. CPU unit coverage is useful, but key contract edges are still missing

- Evidence:
  - current tests cover defaults, resolution, entry/exit behavior, and buffer reuse: `773-843`
- Missing coverage:
  - `long_only = false` short-entry behavior
  - `allow_flip = true` versus `false`
  - silent length-mismatch behavior in `build_rocb_desired_positions_into`
  - fallback behavior when indicator warmup does not match the expected period-derived start
- Smallest justified fix:
  - add small tests around short-side and flip behavior before changing the trading loop

## Performance, Allocation, And CUDA Concerns

### 8. The desired-position path is performance-relevant, but this is not the place for speculative speed work

- Evidence:
  - `build_rocb_desired_positions_into` intentionally reuses a caller-owned buffer: `642-646`
  - the loop is single-pass and allocation-light after buffer sizing: `665-758`
- Safe-now conclusion:
  - preserve the current buffer-reuse pattern
- Benchmark-sensitive conclusion:
  - do not rewrite the trading loop for style or iterator aesthetics without parity verification, because the real risk is semantic drift, not raw CPU throughput

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc
  - document silent shape-mismatch handling
  - add short-side and flip-logic tests
- Defer:
  - splitting GPU wrappers out of the file
  - replacing dynamic-indicator string contracts with a broader typed abstraction

## Technical-Debt Reduction Register

- Safe now:
  - clarify first-valid fallback semantics
  - make desired-position preconditions explicit
- Defer:
  - larger module decomposition
- Do not casually remove:
  - the `OnceLock` descriptor cache
  - the buffer-reuse API
  - the period-mapping defaults that connect ROCB to fast/slow optimization ranges

## Performance Opportunity Register

- Safe structural work:
  - none beyond preserving current buffer reuse and single-pass behavior
- Benchmark-sensitive:
  - any behavioral rewrite of the desired-position loop
  - any attempt to fuse or reorder indicator acquisition without parity checks

## Recommended Action

`Small surgical refactor`

The useful improvements here are contract clarity and test coverage, not strategy rewrites. ROCB already has the right broad structure; it mostly needs its failure policy, warmup semantics, and public config behavior stated more explicitly.

## Exact Local Verification Required If Changed

1. File-local ROCB tests:
   - `cargo test -p ta_app_core rocb::tests:: -- --nocapture`
2. Composite ROCB optimization path:
   - `cargo test -p ta_app_core composite_cpu -- --nocapture`
3. Composite ROCB validation path:
   - `cargo test -p ta_app_core composite_validate -- --nocapture`
4. If CUDA sweep support changes:
   - run the matching ROCB GPU validation / workflow coverage under CUDA-enabled builds before accepting the change

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core rocb::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change period-mapping defaults casually; they are part of ROCB’s optimizer contract
- Do not change entry/exit or flip semantics without verifying CPU optimization and validation parity together
- Do not remove the silent-resilience logic around first-valid scanning unless the indicator stack is proven to guarantee stricter warmup alignment
