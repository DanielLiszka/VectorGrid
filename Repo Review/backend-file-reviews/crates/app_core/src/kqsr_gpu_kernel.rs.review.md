# Review Packet: `crates/app_core/src/kqsr_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kqsr_gpu_kernel.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally solid and backed by parity coverage, but not fully public-review ready because raw device-pointer transports are exposed through a safe public API, caller obligations around CUDA context and pointer lifetime are undocumented, and the current host-terminal decode path is expensive for repeated validation windows`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Hot path`, `CUDA/runtime-critical`, `Unsafe-adjacent`, `Benchmark-sensitive`, `Semantics-sensitive`

## What This File Owns

`kqsr_gpu_kernel.rs` is the strategy-local CUDA execution adapter for KQSR metric evaluation.

It owns:

- the raw input carrier structs for precomputed KQSR device matrices and combo batches:
  - `KqsrGpuDeviceSignalMatrices`: `crates/app_core/src/kqsr_gpu_kernel.rs:33-49`
  - `KqsrGpuComboBatch`: `51-56`
- reusable device and host scratch state:
  - `KqsrKernelScratch`: `59-87`
- a per-thread, per-device CUDA runtime cache:
  - `KqsrCudaRuntime`: `97-142`
  - thread-local runtime registry: `144-146`
- host-side validation of signal geometry and combo windows:
  - `validate_inputs`: `159-255`
- device log-return generation:
  - `launch_compute_log_returns_f32`: `257-300`
- resident close/log-return cache population:
  - `ensure_resident_close_lr`: `303-334`
- strategy-flag packing and metric decode:
  - `kqsr_kernel_flags`: `336-349`
  - `decode_metrics_row`: `351-353`
- the public GPU metric evaluation entrypoint:
  - `eval_kqsr_metrics_gpu_kernel_device_rows`: `356-565`
- file-local validation tests:
  - `569-648`

This file does not own:

- KQSR signal generation or parameter validation:
  - `crates/app_core/src/kqsr.rs`
- generic GPU runtime helpers used across strategy-local wrappers:
  - `crates/app_core/src/standalone_gpu_runtime.rs`
- metric-row decode semantics:
  - `crates/app_core/src/composite_gpu_metric_decode.rs`
- kernel implementation details:
  - `crates/app_core/src/kernels/double_crossover.cu`

## Classification

- Public surface: yes
  - the module is public from `crates/app_core/src/lib.rs:36`
  - `KqsrGpuDeviceSignalMatrices`, `KqsrGpuComboBatch`, and `eval_kqsr_metrics_gpu_kernel_device_rows` are public through that module
- Hot path: yes
  - used in optimization and validation GPU workflows
- CUDA/runtime-critical: yes
  - owns CUDA context/module/stream setup, launch-shape selection, device scratch reuse, and kernel launches
- Strategy-local: yes
  - KQSR-specific, not a generic GPU evaluation layer
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- public module export:
  - `crates/app_core/src/lib.rs:36`
- optimization caller:
  - `crates/app_core/src/composite_cpu.rs:8083-8115`
- validation callers:
  - fixed-candidate GPU validation:
    - `crates/app_core/src/composite_validate.rs:6541-6569`
  - walk-forward batch loop:
    - `crates/app_core/src/composite_validate.rs:6187-6241`
  - reopt loop:
    - `crates/app_core/src/composite_validate.rs:6882-6959`
- strategy config dependency:
  - `crates/app_core/src/kqsr.rs`
- shared standalone runtime helpers:
  - `crates/app_core/src/standalone_gpu_runtime.rs`
- metric decode contract:
  - `crates/app_core/src/composite_gpu_metric_decode.rs:1-5`
- kernel entrypoints:
  - `crates/app_core/src/kernels/double_crossover.cu:2489`
  - `crates/app_core/src/kernels/double_crossover.cu:3824`

### Downstream verification impact if changed

- transport or visibility changes affect:
  - `composite_cpu.rs`
  - `composite_validate.rs`
  - any out-of-repo user of the public module
- runtime-cache changes affect:
  - all repeated KQSR GPU evaluations on the same thread/device
- sync/copy/decode changes affect:
  - optimization throughput
  - walk-forward / reopt validation throughput

## Standards And Research Log

This packet was written after targeted web research focused on public API documentation for fallible and unsafe-adjacent behavior, unsafe abstraction expectations in Rust, thread-local runtime implications, and NVIDIA CUDA guidance for runtime/context and transfer behavior.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes public helper types and a public GPU-evaluation function
   - Standard informed:
     - public APIs should validate inputs and keep sharp implementation details private unless they are intentionally supported contract

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the public entrypoint has important failure conditions and caller preconditions
   - Standard informed:
     - function docs should include error conditions and safety-relevant constraints

3. Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why it was consulted:
     - this file wraps unsafe launches, uninitialized device allocations, and raw pointer transport across FFI boundaries
   - Standard informed:
     - safe abstractions over unsafe code must make invariants and caller obligations explicit

4. `std::thread_local!` documentation
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why it was consulted:
     - the runtime cache is thread-local and internally mutable
   - Standard informed:
     - thread-local state implies thread affinity and interior-mutable access patterns that should be reflected in API expectations

5. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file manages context/module reuse, resident-device buffers, uploads, and repeated launches
   - Standard informed:
     - preserve justified reuse/caching behavior and treat synchronization/copy topology changes as benchmark-sensitive

6. NVIDIA CUDA C++ Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why it was consulted:
     - this file launches CUDA kernels against raw device pointers and uses device/host memory with explicit synchronization
   - Standard informed:
     - kernel-launch contracts and memory-ownership assumptions should be explicit when the wrapper is safe Rust

## Findings

## Correctness And Safety

### 1. The file exposes raw device-pointer transports through a safe public API

- Evidence:
  - public transport structs contain raw device pointers as `u64`:
    - `crates/app_core/src/kqsr_gpu_kernel.rs:33-49`
    - `51-56`
  - the public evaluation function is a safe `pub fn`:
    - `356-565`
  - the implementation feeds those raw values into unsafe CUDA launches:
    - `282-295`
    - `503-541`
  - validation checks shape/null/bounds, but it cannot prove pointer provenance, context ownership, or allocation lifetime:
    - `159-255`
- Why it matters:
  - a safe caller can hand this function stale, foreign-context, or otherwise invalid device pointers and trigger undefined CUDA behavior or runtime faults
  - that makes the current safe signature stronger than the actual abstraction can honestly guarantee
- Smallest justified fix:
  - strongest low-churn option:
    - narrow the module/types/function to `pub(crate)` if they are only intended for internal app-core use
  - if public exposure must remain:
    - document the pointer/context/lifetime contract explicitly
    - strongly consider an `unsafe fn` or an opaque transport type instead of raw `u64`
- Review judgment:
  - high-severity public-surface safety/design issue
  - semantics-sensitive if any external caller already depends on the current safe/public shape

## API And Naming

### 1. Visibility is broader than the repo-local usage justifies

- Evidence:
  - the module is public:
    - `crates/app_core/src/lib.rs:36`
  - in-repo callers are internal:
    - `crates/app_core/src/composite_cpu.rs:8083-8115`
    - `crates/app_core/src/composite_validate.rs:6541-6569`
    - `6187-6241`
    - `6882-6959`
- Why it matters:
  - the current visibility turns a strategy-local GPU wrapper into a wider compatibility promise than the repo appears to need
  - that is especially costly when the public surface carries raw device pointers
- Smallest justified fix:
  - make the module or at least the raw transport types / evaluator `pub(crate)` unless an external consumer is known
- Review judgment:
  - medium API-surface issue
  - safe-now if there is no external dependency

### 2. The public entrypoint does not document unsupported strategy features

- Evidence:
  - the evaluator rejects overlays, stop-loss, and take-profit:
    - `crates/app_core/src/kqsr_gpu_kernel.rs:365-373`
  - there is no public rustdoc explaining that limitation
- Why it matters:
  - the current function signature looks like a general KQSR evaluator, but it only supports a reduced execution subset
- Smallest justified fix:
  - add rustdoc with `Errors` notes covering unsupported strategy features and pointer/context requirements
- Review judgment:
  - medium documentation/API issue
  - safe-now

## Error Handling

### 1. Error handling is mostly explicit, but the public API still needs stronger contract documentation

- Evidence:
  - early validation rejects empty series, bad stride, bad rows, and invalid windows:
    - `159-255`
  - kernel lookup/launch/setup failures are returned as `String`:
    - `269-270`
    - `435-436`
    - `536-541`
- Why it matters:
  - string-based errors are acceptable for a narrow internal GPU adapter
  - the real gap is that the public docs do not tell callers what classes of error to expect or which misuses are not detectable by shape validation
- Smallest justified fix:
  - keep string errors for now
  - add contract documentation instead of forcing a typed-error rewrite
- Review judgment:
  - acceptable as-is for internal use
  - document-only if the module stays public

## Module Boundaries And Technical Debt

### 1. The resident close/log-return cache assumes immutable close input for a given pointer/signature pair

- Evidence:
  - the runtime stores `resident_series`:
    - `crates/app_core/src/kqsr_gpu_kernel.rs:104-128`
  - cache refresh is keyed via `CloseSeriesKey::from_host_and_device(signals.close_f32, signals.close_device_ptr)`:
    - `306`
  - cached resident series are reused when the key matches:
    - `307-334`
  - the key type lives in shared runtime helpers:
    - `crates/app_core/src/standalone_gpu_runtime.rs:10-24`
- Why it matters:
  - this is a reasonable performance optimization
  - but it implies the close buffer and device pointer identity are treated as immutable signal for content validity
  - if a caller reused the same pointer with changed contents, the cache could go stale
- Smallest justified fix:
  - document the immutability assumption
  - add one targeted regression test if the repo expects mutable reuse to be impossible
- Review judgment:
  - medium boundary/maintenance issue
  - safe-now as documentation

## Docs And Examples

### 1. Public helper types need explicit contract docs

- Evidence:
  - missing docs include:
    - same-thread / same-context expectations
    - raw-pointer lifetime requirements
    - unsupported strategy features
    - whether the runtime cache is thread-local and per-device
    - whether `close_device_ptr == 0` means “upload from host close”
- Smallest justified fix:
  - add rustdoc to:
    - `KqsrGpuDeviceSignalMatrices`
    - `KqsrGpuComboBatch`
    - `eval_kqsr_metrics_gpu_kernel_device_rows`
  - include `Errors` and pointer-contract notes

## Tests And Missing Verification

### 1. File-local coverage is narrow, but higher-level parity coverage is strong

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core validate_inputs_rejects_ --features cuda,cuda-backtest-kernel -- --nocapture`
      - included KQSR file-local validation tests:
        - `kqsr_gpu_kernel::tests::validate_inputs_rejects_invalid_windows`
        - `kqsr_gpu_kernel::tests::validate_inputs_rejects_out_of_range_combo_rows`
        - `kqsr_gpu_kernel::tests::validate_inputs_rejects_stride_smaller_than_cols`
    - `cargo test -p ta_app_core checked_i32_rejects_overflow --features cuda,cuda-backtest-kernel -- --nocapture`
      - included:
        - `kqsr_gpu_kernel::tests::checked_i32_rejects_overflow`
    - `cargo test -p ta_app_core kqsr_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
      - passed
  - file-local tests cover only validation and integer-range checks:
    - `crates/app_core/src/kqsr_gpu_kernel.rs:589-648`
  - stronger repo-level parity coverage exists in:
    - optimization parity:
      - `crates/app_core/src/composite_cpu.rs:16824`
      - `16875`
    - validation parity and route diagnostics:
      - `crates/app_core/src/composite_validate.rs:22072-22521`
- Missing tests I would want before changing this file:
  - visibility/contract tests if the API remains public
  - cache-behavior tests around `ensure_resident_close_lr`
  - a dedicated test for unsupported overlays/stop-loss/take-profit strategy rejection

## Performance, Allocation, And CUDA Concerns

### 1. The current host-terminal decode path is reasonable for optimization batches but costly inside repeated validation windows

- Evidence:
  - each evaluation synchronizes the stream:
    - `crates/app_core/src/kqsr_gpu_kernel.rs:545`
  - copies the full output buffer back to host:
    - `546-556`
  - decodes every row into `Vec<Metrics>`:
    - `558-563`
  - validation walk-forward currently calls the evaluator one-window-at-a-time inside nested loops:
    - `crates/app_core/src/composite_validate.rs:6201-6239`
- Why it matters:
  - this design is fine for batched optimization
  - for repeated small validation windows, the forced sync/copy/decode round-trip becomes a real throughput limiter
- Smallest justified fix:
  - do not rewrite now
  - consider a later internal API that returns raw metric rows or accepts reusable host buffers for windowed validation callers
- Review judgment:
  - medium performance issue
  - benchmark-sensitive; do not touch casually

### 2. The thread-local runtime and resident-series cache are directionally correct and should not be “simplified” without evidence

- Evidence:
  - thread-local per-device runtime registry:
    - `crates/app_core/src/kqsr_gpu_kernel.rs:144-146`
  - runtime caches module/stream/scratch/resident series:
    - `97-142`
  - resident close/log-return reuse:
    - `303-334`
- Review judgment:
  - this is the right default posture for avoiding repeated setup/upload work
  - avoid cleanup that would recreate contexts or resident buffers per call

## Code-Quality Improvement Register

- Safe now:
  - narrow visibility if external public use is not intended
  - add contract rustdoc
  - add explicit test for unsupported overlays/stop-loss/take-profit
  - document resident close-cache assumptions
- Coordination required:
  - changing the API from safe to `unsafe`
  - changing transport types away from raw `u64`
  - introducing a lower-level raw-row/reusable-buffer validation path
- Defer until later pass:
  - broader consolidation of strategy-local GPU wrappers into shared infrastructure

## Technical-Debt Reduction Register

- Safe to reduce now:
  - undocumented pointer/context contract
  - undocumented strategy-feature limitations
  - missing cache-behavior tests
- Coordination required:
  - public API narrowing
  - transport redesign
  - alternate validation-oriented output path
- Useful debt that should not be “cleaned up” casually:
  - thread-local runtime caching
  - resident close/log-return reuse
  - explicit launch-shape selection

## Performance Opportunity Register

- Safe structural improvements:
  - none urgent beyond better documentation and targeted tests
- Benchmark-sensitive candidates:
  - reusable host output buffers
  - raw-row output path for repeated validation windows
  - any change to runtime caching or synchronization behavior
- Not the right place to chase performance without evidence:
  - removing synchronization or caching for simplicity alone

## Observability And Diagnostics Gaps

- the public API does not say that raw pointers must belong to the active CUDA context
- the public API does not say that overlays/stop-loss/take-profit are unsupported
- resident close-cache assumptions are implicit rather than documented

## Implementation Staging Guidance

- Safe now:
  - docs
  - unsupported-feature rejection test
  - visibility narrowing if internal-only
- Coordination required:
  - unsafe/public API redesign
  - alternate output-path design for validation loops
- Defer until later pass:
  - broader GPU-wrapper refactors

## Change Blast Radius

- Local only:
  - rustdoc
  - cache documentation
  - local validation tests
- Same subsystem:
  - visibility narrowing
  - API contract changes
  - validation-loop output-path changes
- Cross-backend/runtime:
  - any change to thread-local runtime or resident-series reuse

## Open Questions Or Assumptions

- assumption:
  - this wrapper is intended for internal app-core use even though the module is currently public
- open question:
  - whether any out-of-repo caller currently depends on the safe/public raw-pointer transport
- open question:
  - whether the validation subsystem would materially benefit from a raw-row or reusable-buffer output mode, or whether current costs are acceptable

## Recommended Action

`Small surgical refactor`

The right next pass is:

- narrow or document the raw-pointer public contract
- add missing contract docs and one or two focused tests
- treat validation-throughput improvements as later benchmarked work, not immediate cleanup

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core validate_inputs_rejects_ --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core checked_i32_rejects_overflow --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core kqsr_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
- if transport/runtime behavior changes:
  - `cargo test -p ta_app_core kqsr_optimization_gpu_kernel_sanity_matches_cpu_small_grid --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kqsr_validation_gpu_kernel_reports_route_diagnostics --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kqsr_validation_gpu_kernel_walk_forward_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- do not remove runtime caching or resident-series reuse without measurement
- do not silently change the kernel argument contract or metric decode stride
- if the API is narrowed or made `unsafe`, update all internal callers deliberately and verify no external consumer depends on the previous public safe shape
