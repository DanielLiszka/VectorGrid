# Review Packet: `crates/app_core/src/cuda_device.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/cuda_device.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Small but important runtime helper that is conceptually clear, but not public-review ready as-is because its cached error behavior is process-lifetime, its convenience helper silently drops resolver failures, and its selection policy is mostly implicit rather than documented`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `GPU-runtime`, `Error-handling-sensitive`, `Benchmark-sensitive`, `Startup-diagnostics-sensitive`

## What This File Owns

`cuda_device.rs` owns preferred CUDA device discovery for `ta_app_core`. It defines the public device-metadata DTO, resolves the preferred device ID under the `cuda` feature, exposes public accessors for both error-preserving and lossy access patterns, and provides CPU-build stubs that return `Ok(None)`.

This file owns:

- public device metadata DTO:
  - `PreferredCudaDevice`: `crates/app_core/src/cuda_device.rs:4-9`
- CUDA-feature-gated preferred-device ID resolution and caching:
  - `resolve_preferred_cuda_device_id`: `12-41`
- public error-preserving accessor for preferred device ID:
  - `preferred_cuda_device_id_result`: `44-46`
- public preferred-device metadata accessor:
  - `preferred_cuda_device`: `49-69`
- non-CUDA stubs:
  - `preferred_cuda_device`: `71-74`
  - `preferred_cuda_device_id_result`: `76-79`
- lossy convenience helper:
  - `preferred_cuda_device_id`: `81-83`

Current selection policy:

- no devices => `None`: `20-23`
- one device => `Some(0)`: `24-26`
- multiple devices => choose the device with the largest `total_memory()`, keeping the first device encountered on ties:
  - `28-38`

## Classification

- Public surface: yes
  - re-exported from `crates/app_core/src/lib.rs:95-98`
- Hot path: no
  - one-time or infrequent runtime/device-selection helper
- CUDA/runtime-critical: yes
  - it influences automatic GPU device selection, startup prewarm, diagnostics, and optional GPU routing
- Strategy-local: no
  - shared runtime helper
- Low-risk support code: mixed
  - tiny file, but high leverage because it shapes whether GPU paths are considered available and how failures are surfaced

## Direct Interactions

### Backend interaction map

- crate-root re-export:
  - `crates/app_core/src/lib.rs:95-98`
- backend normalization that preserves errors:
  - `crates/app_core/src/double_ma.rs:125-149`
  - representative uses:
    - `crates/app_core/src/double_ma.rs:4989`
    - `crates/app_core/src/composite_cpu.rs:2262`
    - `crates/app_core/src/composite_mo.rs:1229-1234`
- options-expression runtime path that uses the lossy helper:
  - `crates/app_core/src/options_expression.rs:87-99`
  - `3790-3805`
  - `5271-5274`
- benchmark/tooling surfaces that use the lossy helper:
  - `crates/app_core/benches/options_expression_gpu_score.rs:19-27`
  - `crates/app_core/src/options_expression.rs:7185-7186`
  - `7339-7340`

### Tauri/runtime interaction map

- startup prewarm path uses the error-preserving accessor:
  - `src-tauri/src/lib.rs:58-76`
- system diagnostics surface uses preferred-device metadata and error reporting:
  - `src-tauri/src/commands_system.rs:27-40`

### Downstream verification impact if changed

- changing cached-failure behavior affects:
  - startup prewarm logging and recovery
  - GPU diagnostics reported to the app
  - any later runtime path that relies on repeated device discovery
- changing the lossy helper affects:
  - options-expression GPU routing
  - benchmark skip behavior
  - any caller that currently interprets `None` as “no GPU available”
- changing device-selection policy affects:
  - which GPU gets auto-selected on multi-GPU systems
  - user-facing diagnostics in `get_gpu_status`

## Standards And Research Log

This packet was written after targeted web research focused on Rust one-time initialization semantics, public API documentation guidance, and CUDA multi-device selection context.

### Sources consulted

1. Rust `std::sync::OnceLock` documentation
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why it was consulted:
     - this file caches `Result<Option<u32>, String>` and `Result<Option<PreferredCudaDevice>, String>` inside `OnceLock`
   - Standard informed:
     - `get_or_init` caches the returned value permanently once initialization succeeds, which means wrapping `Result` inside it makes first-call errors sticky for the rest of the process

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes multiple public helpers with materially different error semantics
   - Standard informed:
     - public helpers should make the distinction between lossy and error-preserving behavior obvious

3. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the public DTO and helper trio currently rely on call-site knowledge rather than rustdoc
   - Standard informed:
     - public runtime helpers should document selection policy, feature-gated behavior, and error semantics

4. NVIDIA CUDA C++ Programming Guide, Multi-Device System / Device Selection context
   - Source: <https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-programming-guide/index.html>
   - Why it was consulted:
     - this file implements automatic device enumeration and selection in a multi-GPU context
   - Standard informed:
     - device selection is a runtime policy decision and should be explicit about what attribute it optimizes for

## Findings

## Correctness And Safety

### 1. No direct memory-safety issue was identified in this file

- Evidence:
  - the implementation is small, feature-gated, and uses `cust`/`Device` APIs rather than raw FFI
- Review judgment:
  - the main risk surface is runtime semantics and diagnostics, not unsafe behavior

## API And Naming

### 1. `preferred_cuda_device_id()` is a lossy helper with a deceptively plain name

- Evidence:
  - the helper drops errors by calling:
    - `.ok().flatten()`: `crates/app_core/src/cuda_device.rs:81-83`
  - there is already an error-preserving companion:
    - `preferred_cuda_device_id_result`: `44-46`
  - options-expression uses the lossy path via:
    - `crates/app_core/src/options_expression.rs:87-99`
    - `3790-3805`
    - `5271-5274`
- Why it matters:
  - a caller reading only the name can easily assume it is the normal accessor rather than a convenience wrapper that intentionally erases the reason discovery failed
  - this is especially misleading because other core backends use the error-preserving variant
- Smallest justified fix:
  - document `preferred_cuda_device_id()` explicitly as a lossy convenience helper
  - if future cleanup is acceptable, add a more explicit convenience name and reserve `_result` for the production diagnostic path
- Review judgment:
  - safe-now documentation change
  - renaming is broader and should be compatibility-reviewed

### 2. The auto-selection policy is public in practice but undocumented

- Evidence:
  - multi-device resolution chooses the device with the largest total memory:
    - `crates/app_core/src/cuda_device.rs:28-38`
  - ties are implicitly broken by first enumeration order because the condition is strict `>`:
    - `33-35`
  - Tauri exposes the chosen device’s name and memory in system diagnostics:
    - `src-tauri/src/commands_system.rs:27-40`
- Why it matters:
  - contributors and users can observe the chosen device, but the rationale is not described anywhere in the public surface
  - if the selection policy ever changes, route outcomes on multi-GPU systems can change silently
- Smallest justified fix:
  - add rustdoc describing the current selection rule and CPU-build stub behavior
- Review judgment:
  - documentation gap, not an immediate behavior bug

## Error Handling

### 1. First-call discovery failures are cached for the lifetime of the process

- Evidence:
  - `resolve_preferred_cuda_device_id` stores `Result<Option<u32>, String>` inside a `OnceLock`:
    - `crates/app_core/src/cuda_device.rs:12-18`
    - `13-40`
  - `preferred_cuda_device` stores `Result<Option<PreferredCudaDevice>, String>` the same way:
    - `49-69`
  - because the cached value itself is a `Result`, any first-call `Err(...)` becomes the permanent cached state
  - this feeds:
    - startup prewarm: `src-tauri/src/lib.rs:58-76`
    - diagnostics: `src-tauri/src/commands_system.rs:27-40`
- Why it matters:
  - one transient `cust::init`, enumeration, or device-query failure can permanently disable later recovery within the same process
  - that is a meaningful runtime behavior choice, not just an implementation detail
- Smallest justified fix:
  - cache only successful `Ok(Some(...))` and `Ok(None)` outcomes
  - let `Err(...)` retry on later calls instead of becoming process-lifetime state
- Review judgment:
  - highest-priority issue in this file
  - semantics-sensitive, not throughput-sensitive

### 2. The lossy helper collapses resolver failures into “no device”

- Evidence:
  - `preferred_cuda_device_id()` erases errors:
    - `crates/app_core/src/cuda_device.rs:81-83`
  - options-expression consumes that path:
    - `crates/app_core/src/options_expression.rs:87-99`
    - `3790-3805`
    - `5271-5274`
  - required GPU mode then emits a generic “device unavailable” error instead of the real discovery failure:
    - `3469-3481`
  - benchmark/tests also treat `None` as skip:
    - `crates/app_core/benches/options_expression_gpu_score.rs:19-27`
    - `crates/app_core/src/options_expression.rs:7185-7186`
    - `7339-7340`
- Why it matters:
  - resolver errors and genuine “no CUDA device present” are operationally different states
  - collapsing them hides useful diagnostics
  - in benchmark code it can convert infrastructure failure into a misleading skip classification
- Smallest justified fix:
  - keep the lossy helper only for truly optional paths where diagnostics do not matter
  - route required-GPU, benchmark, and diagnostic surfaces through `preferred_cuda_device_id_result()`
- Review judgment:
  - real API/error-handling issue
  - semantics-sensitive and benchmark-sensitive

## Module Boundaries And Technical Debt

### 1. The file lacks a test seam for discovery-policy behavior

- Evidence:
  - current implementation calls `cust::init` and `Device::*` directly inside cached closures:
    - `crates/app_core/src/cuda_device.rs:15-38`
    - `53-66`
  - downstream tests only cover propagated error behavior in another module:
    - `crates/app_core/src/double_ma.rs:14745-14749`
  - Tauri only tests diagnostics shape:
    - `src-tauri/src/commands_system.rs:121-147`
- Why it matters:
  - the highest-risk semantics in this file are not directly locked:
    - retry-vs-sticky failure behavior
    - lossy helper collapse
    - current selection heuristic
- Smallest justified fix:
  - add a small injection seam or file-local helper indirection that can be tested without a real CUDA environment
  - if code extraction is too much for now, at least add targeted tests around the non-CUDA helper semantics and documented policy
- Review judgment:
  - bounded debt worth paying down

## Docs And Examples

### 1. Public DTO and helpers need rustdoc

- Evidence:
  - no rustdoc on:
    - `PreferredCudaDevice`: `crates/app_core/src/cuda_device.rs:4`
    - `preferred_cuda_device_id_result`: `44`
    - `preferred_cuda_device`: `49`
    - `preferred_cuda_device_id`: `81`
- Why it matters:
  - callers need to know:
    - CPU builds return `Ok(None)`
    - multi-GPU selection is memory-based
    - `preferred_cuda_device_id()` is lossy
    - first-call discovery is cached today
- Smallest justified fix:
  - add rustdoc that makes those semantics explicit
- Review judgment:
  - safe-now docs improvement

## Tests And Missing Verification

### 1. There is no direct coverage of sticky-failure or lossy-collapse behavior

- Evidence:
  - direct test coverage in this file: none
  - downstream-only coverage:
    - `crates/app_core/src/double_ma.rs:14745-14749`
    - `src-tauri/src/commands_system.rs:121-147`
- Why it matters:
  - the two most important behaviors in this file can change silently without compile-time signal
- Smallest justified fix:
  - add tests that prove whether:
    - resolver failures should be retryable
    - lossy helper behavior is intentional
    - selection policy remains “largest total VRAM wins”

### 2. Current verification should include both core and Tauri surfaces

- Evidence:
  - core backends and Tauri diagnostics both depend on this module
- Review judgment:
  - any behavior change here must be verified from both surfaces

## Performance, Allocation, And CUDA Concerns

### 1. Successful caching is appropriate; failure caching is the real problem

- Evidence:
  - device enumeration is not performance-critical enough to justify repeated successful discovery
  - the helper is intentionally one-time for normal cases
- Why it matters:
  - this should not turn into a “remove caching” recommendation
  - the real issue is caching failures, not caching itself
- Review judgment:
  - keep success caching
  - avoid speculative perf work

### 2. Benchmark sensitivity exists through skip classification, not through runtime cost

- Evidence:
  - options-expression benchmark skips on `None`:
    - `crates/app_core/benches/options_expression_gpu_score.rs:19-27`
- Why it matters:
  - lossy error collapse can alter benchmark metadata and hide infrastructure failures as “no device”
- Review judgment:
  - benchmark-sensitive in tooling semantics, not in performance throughput

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc for the public helpers and DTO
  - document the selection policy and lossy helper semantics
  - add targeted tests
- Coordination required:
  - stop caching `Err(...)` permanently
  - remove lossy helper use from required-GPU or benchmark paths
- Defer until later pass:
  - any broader redesign of auto-device selection heuristics

## Technical-Debt Reduction Register

- Safe to reduce now:
  - undocumented public behavior
  - missing direct tests
- Coordination required:
  - sticky-failure caching semantics
  - lossy helper usage in important call sites
- Useful debt that should not be “cleaned up” casually:
  - successful one-time caching of discovery results

## Performance Opportunity Register

- Safe structural improvements:
  - none needed
- Benchmark-sensitive candidates:
  - ensuring benchmark paths use error-preserving resolution when skip/report accuracy matters
- Not the right place to chase performance:
  - the file is tiny and not throughput-bound

## Observability And Diagnostics Gaps

- lossy helper erases root-cause diagnostics
- auto-selection policy is not documented in the public surface
- direct tests do not currently lock retry-vs-sticky failure semantics

## Implementation Staging Guidance

- Safe now:
  - rustdoc additions
  - new focused tests
- Coordination required:
  - retryable error behavior instead of sticky cached failures
  - swapping benchmark/required-GPU paths to the error-preserving accessor
- Defer until later pass:
  - more sophisticated multi-GPU selection heuristics

## Change Blast Radius

- Local only:
  - rustdoc
  - new direct tests
- Same subsystem:
  - cached-failure behavior changes
  - lossy-helper call-site changes
- Cross-runtime:
  - startup prewarm behavior
  - system diagnostics
  - options-expression GPU availability decisions

## Recommended Action

`Small surgical refactor`

The strongest next pass is:

- document the public behavior
- add direct tests
- stop treating transient discovery failures as permanent process state
- keep `preferred_cuda_device_id_result()` as the authoritative path anywhere accurate diagnostics matter

## Exact Local Verification Required If Changed

If only rustdoc or tests change:

- `cargo test -p ta_app_core preferred_cuda_device_resolution_error_is_not_masked -- --nocapture`

If cached-failure or lossy-helper semantics change:

- `cargo test -p ta_app_core preferred_cuda_device_resolution_error_is_not_masked -- --nocapture`
- `cargo test -p ta_desktop_demo_app get_gpu_status_reports_consistent_diagnostics_shape -- --nocapture`
- targeted options-expression GPU-path smoke on a CUDA-enabled machine, especially for required-GPU error messaging and benchmark skip/report behavior

## Regression Risk And Feature-Preservation Constraints

The intended outcome is better runtime diagnostics and recovery behavior, not removal of CUDA auto-selection.

- do not remove CPU-build `Ok(None)` fallback behavior
- do not remove successful one-time caching
- if error caching is relaxed, verify that startup diagnostics and prewarm behavior still remain stable on healthy systems
- if lossy helper usage is narrowed, preserve optional-path ergonomics where silent `None` is actually acceptable
