# Review Packet: `crates/app_core/src/mkcs_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/mkcs_gpu_kernel.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally useful and locally parity-tested, but not fully public-review ready because the safe public API accepts raw device-pointer transports with invariants the type system does not encode, the resident close/log-return cache is keyed by pointer identity rather than content, and the public evaluator contract around host-upload fallback and unsupported strategy overlays is not documented`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `CUDA-runtime-critical`, `Benchmark-sensitive`, `Semantics-sensitive`, `Strategy-local`

## What This File Owns

`mkcs_gpu_kernel.rs` owns the MKCS strategy-local CUDA runtime wrapper and the MKCS GPU-kernel evaluation path that consumes precomputed device-resident signal matrices.

It owns:

- the CUDA-only compilation boundary and MKCS-specific kernel flags:
  - `crates/app_core/src/mkcs_gpu_kernel.rs:1-31`
- public transport structs for the MKCS GPU path:
  - `MkcsGpuDeviceSignalMatrices`: `33-44`
  - `MkcsGpuComboBatch`: `46-51`
- reusable device scratch/state for combo uploads and host metric staging:
  - `MkcsKernelScratch`: `54-87`
- the per-device CUDA runtime wrapper:
  - `MkcsCudaRuntime`: `95-141`
  - thread-local runtime map:
    - `139-141`
- upload deduplication helpers for combo vectors:
  - `ensure_uploaded_slice`: `143-161`
- host-side shape, bounds, and window validation:
  - `validate_inputs`: `163-245`
- the raw CUDA launch wrapper for log-return generation:
  - `launch_compute_log_returns_f32`: `247-290`
- resident close/log-return caching:
  - `ensure_resident_close_lr`: `292-326`
- strategy-flag mapping and metric-row decode:
  - `mkcs_kernel_flags`: `328-340`
  - `decode_metrics_row`: `343-345`
- the main public evaluator:
  - `eval_mkcs_metrics_gpu_kernel_device_rows`: `347-549`
- file-local validation tests:
  - `552-630`

This file does not own:

- the MKCS config/signal-building logic:
  - `crates/app_core/src/mkcs.rs`
- shared module-loading and launch-shape helpers:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- shared scratch/cache helpers:
  - `crates/app_core/src/standalone_gpu_runtime.rs`
- the underlying CUDA kernel implementations:
  - `crates/app_core/src/kernels/double_crossover.cu`

## Classification

- Public surface: yes
  - the module is public at `crates/app_core/src/lib.rs:45`
- Hot path: yes
  - this evaluator is used on MKCS optimization and repeated-window validation GPU routes
- CUDA/runtime-critical: yes
  - it owns context/module/stream lifetime, launches, device buffers, and host-device synchronization
- Strategy-local: yes
  - it is MKCS-specific rather than generic GPU infrastructure
- Low-risk support code: no
  - this is strategy-local runtime code with a high correctness and performance blast radius

## Direct Interactions

### Backend interaction map

- crate export surface:
  - `crates/app_core/src/lib.rs:45`
- shared launch/module helpers:
  - `crates/app_core/src/gpu_backtest_kernel.rs:581`
  - `611`
  - `703`
  - `1879`
- shared cache/allocation helpers:
  - `crates/app_core/src/standalone_gpu_runtime.rs:10-27`
  - `34-45`
  - `54-89`
- metrics-row decoder:
  - `crates/app_core/src/composite_gpu_metric_decode.rs:5`
  - `31`
- MKCS config and signal-matrix inputs:
  - `crates/app_core/src/mkcs.rs:22`
  - `113-176`
  - `434-517`
  - `650-777`
- optimization route usage:
  - `crates/app_core/src/composite_cpu.rs:7664`
  - `18976`
- validation route usage:
  - `crates/app_core/src/composite_validate.rs:2513`
  - `2715`
  - `2965`
  - `3052`
  - parity tests at `25457` and `25556`
- desktop-level cross-ticker smoke:
  - `src-tauri/src/commands_validation.rs:2785`
- underlying kernel entrypoints:
  - `crates/app_core/src/kernels/double_crossover.cu:1832`
  - `3824`

### Downstream verification impact if changed

- transport-type or pointer-contract changes affect:
  - `mkcs.rs`
  - `composite_cpu.rs`
  - `composite_validate.rs`
- launch argument, buffer-stride, or sync changes affect:
  - MKCS GPU optimization parity
  - MKCS GPU validation parity
  - cross-ticker validation smoke
- cache-key changes affect:
  - repeated-window validation runs
  - optimization loops that reuse resident close/log-return state

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API contracts around unsafe-adjacent types, thread-local runtime caching, interior mutability, CUDA runtime/launch behavior, host-device transfer guidance, and the `cust` crate’s public runtime surface.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes public transport structs and a public evaluator over unsafe CUDA internals
   - Standard informed:
     - safe public surfaces should make invariants explicit and avoid accidental overexposure

2. Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why it was consulted:
     - the file bridges safe Rust callers to raw-pointer CUDA launches
   - Standard informed:
     - sound wrappers over unsafe internals must clearly define which invariants remain the caller’s responsibility

3. `thread_local!` documentation
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why it was consulted:
     - the file stores per-thread, per-device runtimes in thread-local state
   - Standard informed:
     - thread confinement is a real part of the runtime contract and should be documented when correctness/perf depend on it

4. `RefCell` documentation
   - Source: <https://doc.rust-lang.org/std/cell/struct.RefCell.html>
   - Why it was consulted:
     - runtime launch-cap caches and thread-local maps use interior mutability
   - Standard informed:
     - interior mutability is acceptable here, but aliasing and mutation assumptions should stay narrow and well scoped

5. NVIDIA CUDA C Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why it was consulted:
     - the file launches kernels via positional argument arrays and owns stream/context interaction
   - Standard informed:
     - launch ABI, stream synchronization, and device-memory lifetime are correctness-sensitive, not just implementation detail

6. NVIDIA CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - the evaluator caches device-resident close/log returns and stages combo uploads/output downloads
   - Standard informed:
     - resident-buffer reuse is good, but synchronization and host-device transfers are benchmark-sensitive and should be changed only with measurement

7. `cust` crate documentation
   - Source: <https://docs.rs/cust/latest/cust/>
   - Why it was consulted:
     - this file is a thin orchestration layer over `cust` contexts, modules, streams, functions, and device buffers
   - Standard informed:
     - context/module/stream semantics and buffer ownership are part of the wrapper contract and should stay explicit

## Findings

## Correctness And Safety

### 1. The safe public API accepts raw device-pointer transports with invariants it cannot actually enforce

- Evidence:
  - public transport structs expose raw pointers as `u64`:
    - `crates/app_core/src/mkcs_gpu_kernel.rs:33-44`
    - `46-51`
  - the public evaluator is safe to call:
    - `347-549`
  - launch wrappers then pass raw pointers into CUDA kernels via `*mut c_void` argument arrays:
    - `272-287`
    - `495-529`
  - `validate_inputs` checks shapes/bounds/nullness, but cannot prove:
    - pointer lifetime
    - device affinity
    - row-major layout validity
    - backing allocation ownership
    - `163-245`
- Why it matters:
  - callers can satisfy the type checker while still violating the actual runtime safety contract
  - this is the primary public-review-readiness problem in the file
- Smallest justified fix:
  - if no crate-external caller needs this surface, narrow the transport types and evaluator to `pub(crate)`
  - if public exposure must remain, add rustdoc documenting the exact pointer, lifetime, device, and layout invariants
  - do not pretend validation can fully make this safe
- Review judgment:
  - high safety/API issue

### 2. The resident close/log-return cache is keyed by pointer identity, so stale reuse is possible if a caller mutates data in place

- Evidence:
  - the cache key is derived from host/device identity:
    - `CloseSeriesKey`: `crates/app_core/src/standalone_gpu_runtime.rs:10-18`
  - the evaluator reuses cached resident returns when the key matches:
    - `crates/app_core/src/mkcs_gpu_kernel.rs:297-325`
  - host-upload fallback and device-pointer reuse both flow through that same key path:
    - `304-320`
- Why it matters:
  - if the same host slice or device allocation is mutated in place while preserving pointer identity, cached log returns can become stale with no detection
  - that is a correctness risk, not just a performance tradeoff
- Smallest justified fix:
  - if callers are required to treat those buffers as immutable for the cache lifetime, document that requirement explicitly
  - do not switch to content hashing casually, because that is benchmark-sensitive
  - consider adding an opt-in invalidation/reset path if stale reuse becomes a practical maintenance problem
- Review judgment:
  - high correctness/cache-contract issue

## API And Naming

### 1. The public evaluator contract does not document important behavior: host-upload fallback and unsupported overlays/SL/TP

- Evidence:
  - if `close_device_ptr == 0`, the evaluator uploads `close_f32` to device memory:
    - `crates/app_core/src/mkcs_gpu_kernel.rs:305-310`
  - strategy overlays, stop-loss, and take-profit are explicitly rejected:
    - `359-365`
  - these are significant contract rules, but there is no rustdoc on the public structs or evaluator describing them
- Why it matters:
  - callers cannot tell from the public API which parts of the transport are optional versus mandatory
  - they also cannot tell which strategy features invalidate the GPU kernel route
- Smallest justified fix:
  - add rustdoc for the transport structs and evaluator
  - state clearly:
    - `close_device_ptr == 0` triggers host upload
    - signal row pointers must already be device-resident
    - overlays, stop-loss, and take-profit are unsupported on this route
- Review judgment:
  - medium API/documentation issue

### 2. The module is public, but apparent consumers are internal GPU-route code only

- Evidence:
  - the module is publicly exported:
    - `crates/app_core/src/lib.rs:45`
  - in-repo call sites are internal:
    - `crates/app_core/src/composite_cpu.rs:7664`
    - `crates/app_core/src/composite_validate.rs:2513`
    - `2715`
    - `2965`
    - `3052`
- Why it matters:
  - a public surface with raw device handles creates compatibility burden without obvious external benefit
- Smallest justified fix:
  - audit whether any crate-external caller uses this module directly
  - if not, narrow it to `pub(crate)` and keep the public contract at higher-level orchestration surfaces
- Review judgment:
  - medium public-surface issue

## Error Handling

### 1. Runtime errors are surfaced as `String`, but pointer-validity failures cannot be distinguished from generic launch/runtime failure

- Evidence:
  - most failures are funneled through `map_err(|e| e.to_string())` across context/module/stream/launch/copy operations:
    - examples:
      - `100-109`
      - `259-260`
      - `285-286`
      - `308`
      - `313`
      - `528`
      - `531`
      - `540-541`
  - invalid raw pointer/device-affinity issues are not modeled directly by the API and therefore can only show up as downstream CUDA/runtime errors
- Why it matters:
  - this is acceptable for an internal GPU wrapper, but weak for a broad public API
  - it reinforces that the pointer contract should be narrowed or documented rather than relying on stringified runtime failures
- Smallest justified fix:
  - keep `String` if consistency with the surrounding crate matters
  - improve the public contract rather than over-engineering the error enum here
- Review judgment:
  - low-medium error-handling issue

## Module Boundaries And Technical Debt

### 1. The launch ABI is tightly coupled to `double_crossover.cu` entrypoints and should be treated as coordinated multi-file state

- Evidence:
  - this file launches:
    - `compute_log_returns_f32`: `257-287`
    - `mkcs_backtest_metrics_f32`: `429-529`
  - the corresponding kernel entrypoints live in:
    - `crates/app_core/src/kernels/double_crossover.cu:3824`
    - `1832`
- Why it matters:
  - argument order, integer widths, and stride semantics are coordinated across Rust and CUDA files
  - “small cleanup” edits on one side can produce high-regression launch bugs on the other
- Smallest justified fix:
  - keep this coupling explicit in docs/comments
  - do not refactor launch argument order without simultaneously reviewing the CUDA kernel signatures
- Review judgment:
  - medium cross-file technical-debt issue

## Docs And Examples

### 1. The public transport types and evaluator have no rustdoc explaining invariants, cache semantics, or thread-local runtime behavior

- Evidence:
  - public items include:
    - `MkcsGpuDeviceSignalMatrices`: `crates/app_core/src/mkcs_gpu_kernel.rs:33`
    - `MkcsGpuComboBatch`: `46`
    - `eval_mkcs_metrics_gpu_kernel_device_rows`: `347`
  - there is no rustdoc describing:
    - pointer validity requirements
    - cache immutability assumptions
    - thread-local runtime behavior
    - unsupported strategy features
    - host-upload fallback behavior
- Smallest justified fix:
  - add rustdoc to the public structs and evaluator
  - explicitly call out which invariants remain caller obligations

## Tests And Missing Verification

### 1. File-local validation coverage is good, and the main parity checks pass, but important cache/fallback paths remain untested

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core mkcs_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
      - passed
      - included:
        - `validate_inputs_rejects_out_of_range_combo_rows`
        - `checked_i32_rejects_overflow`
        - `validate_inputs_rejects_stride_smaller_than_cols`
        - `validate_inputs_rejects_invalid_windows`
  - optimization GPU-route verification run:
    - `cargo test -p ta_app_core mkcs_gpu_kernel_request_runs_when_cuda_available --features cuda,cuda-backtest-kernel -- --nocapture`
      - passed
  - validation parity verification runs:
    - `cargo test -p ta_app_core mkcs_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
      - passed
    - `cargo test -p ta_app_core mkcs_validation_gpu_kernel_walk_forward_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
      - passed
  - current tests do not directly cover:
    - host-upload fallback when `close_device_ptr == 0`
    - resident-cache refresh behavior when the same pointer identity is reused with different contents
    - overlay/stop-loss/take-profit rejection
    - misuse of nonzero-but-invalid raw pointers
- Missing tests I would want before changing this file:
  - a host-upload fallback test
  - a cache invalidation/refresh contract test
  - an explicit test that unsupported strategy overlays are rejected

## Performance, Allocation, And CUDA Concerns

### 1. The current cache and upload-dedup design is performance-motivated and should be preserved unless measurement says otherwise

- Evidence:
  - resident close/log returns are cached:
    - `crates/app_core/src/mkcs_gpu_kernel.rs:292-326`
  - combo uploads are deduplicated by upload signature:
    - `143-161`
    - `400-427`
- Why it matters:
  - these are exactly the kinds of structural reuse that keep repeated-window GPU workflows viable
- Review judgment:
  - no broad simplification should remove this reuse without GPU benchmark evidence

### 2. The evaluator always synchronizes and copies the full metrics buffer back to host, which is correct but benchmark-sensitive in repeated validation flows

- Evidence:
  - the stream is synchronized after launch:
    - `crates/app_core/src/mkcs_gpu_kernel.rs:531`
  - the full output slice is copied back to host:
    - `532-541`
  - repeated validation callers include:
    - `crates/app_core/src/composite_validate.rs:2513`
    - `2715`
    - `2965`
    - `3052`
- Why it matters:
  - any attempt to defer sync, change staging, or alter output materialization will change benchmark behavior and potentially route semantics
- Smallest justified fix:
  - do not change this without measurement
  - if optimization is ever needed, validate under the GPU perf gate and the repeated-window parity tests
- Review judgment:
  - medium benchmark-sensitive performance issue

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc for public structs and evaluator
  - document cache immutability assumptions
  - add host-upload fallback and overlay-rejection tests
- Coordination required:
  - narrowing public visibility
  - changing cache-key policy
  - changing launch/signature coupling with the CUDA file
- Defer until later pass:
  - redesigning raw-pointer transport types or sync/copy strategy

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing rustdoc
  - missing explicit tests for fallback/rejection paths
- Coordination required:
  - public raw-handle exposure
  - cross-file launch ABI coupling
  - cache-key semantics
- Useful debt that should not be “cleaned up” casually:
  - resident-series reuse
  - upload-signature deduplication
  - per-thread runtime reuse

## Performance Opportunity Register

- Safe structural improvements:
  - none clearly justified without measurement
- Benchmark-sensitive candidates:
  - cache-key redesign
  - sync/copy changes
  - output staging changes
- Not the right place to chase performance without evidence:
  - replacing current reuse mechanisms with “simpler” code that stages more data through host memory

## Observability And Diagnostics Gaps

- the public contract does not state when host upload versus resident device usage occurs
- cache immutability assumptions are implicit rather than explicit
- overlay/stop-loss/take-profit rejection is only discoverable by runtime error string

## Implementation Staging Guidance

- Safe now:
  - docs
  - additive tests
  - visibility audit
- Coordination required:
  - cache-key policy changes
  - pointer-wrapper redesign
  - launch ABI changes touching the CUDA kernel file
- Defer until later pass:
  - benchmark-driven sync/copy redesign

## Change Blast Radius

- Local only:
  - docs
  - additive tests
- Same subsystem:
  - visibility or contract changes affect MKCS optimization and validation GPU routes
- Cross-backend/runtime:
  - cache/launch/output changes affect performance and parity across GPU workflows

## Open Questions Or Assumptions

- assumption:
  - callers are expected to treat resident close buffers and signal matrices as immutable for the duration of cache reuse
- open question:
  - whether any crate-external caller truly depends on the current public raw-pointer transport types
- open question:
  - whether the host-upload fallback is intended to be a permanent compatibility feature or only a convenience during current integration

## Recommended Action

`Small surgical refactor`

The right next pass is:

- document the actual contract
- add the missing fallback/rejection tests
- narrow the public surface if no external consumer needs it
- defer cache-key or sync/copy changes until a measured GPU perf pass

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core mkcs_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core mkcs_gpu_kernel_request_runs_when_cuda_available --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core mkcs_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core mkcs_validation_gpu_kernel_walk_forward_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
- if cache behavior, sync behavior, or launch arguments change:
  - `cargo test -p ta_desktop_demo_app cross_ticker_mkcs_gpu_kernel_smoke_applies_stats --features cuda,cuda-backtest-kernel -- --nocapture`
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Regression Risk And Feature-Preservation Constraints

- do not remove host-upload fallback or supported route behavior silently
- do not change cache semantics without considering repeated-window validation reuse and benchmark impact
- do not alter launch argument order or widths without coordinated CUDA-kernel review
