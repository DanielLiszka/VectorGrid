# Review Packet: `crates/app_core/src/strategy_gpu_runtime.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_gpu_runtime.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful runtime surface, but not fully review-ready because cache-scope semantics, reset behavior, and thread-local upload lifetime are only implicit`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `GPU-runtime`, `Cache-state`, `Thread-local`, `Semantics-sensitive`

## What This File Owns

`strategy_gpu_runtime.rs` owns the GPU-facing runtime handle and caching layer for compiled strategy programs:

- capability report and `StrategyHandle`:
  - `18-67`
- compiled bundle cache keys and executable cache keys:
  - `80-120`
- GPU executable metadata:
  - `122-152`
- process-global caches and counters:
  - `154-166`
- per-thread uploaded executable cache:
  - `167-176`
- reset / stats / get-or-build entry points:
  - `188-354`
- file-local tests:
  - `355-637`

## Classification

- Public surface: effectively yes
  - exported through `crates/app_core/src/lib.rs`
- Hot path: runtime-setup only
- CUDA/runtime-critical: yes
- Global-stateful: yes
  - `OnceLock<Mutex<HashMap<...>>>` plus `thread_local!`
- Benchmark-sensitive: yes
  - cache behavior affects repeated dynamic GPU execution

## Direct Interactions

### Backend interaction map

- compiled GPU program source:
  - `crates/app_core/src/strategy_gpu_program.rs`
- GPU upload path:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- dynamic plan compilation used in tests:
  - `crates/app_core/src/strategy_dynamic.rs:658`

### Downstream verification impact if changed

- changing handle construction affects whether GPU programs are materialized for `GpuUnified` plans
- changing cache key semantics affects compiled-bundle reuse and uploaded-program reuse
- changing reset behavior affects test reliability and any runtime path expecting a fresh cache

## Standards And Research Log

This packet was written after targeted web research focused on lazy globals, thread-local state, and CUDA upload safety expectations.

### Sources consulted

1. `std::sync::OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why it was consulted:
     - the module uses process-global lazy caches
   - Standard informed:
     - lazy global initialization is fine, but lifecycle and reset scope must be made explicit

2. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why it was consulted:
     - uploaded executables are cached per thread
   - Standard informed:
     - thread-local state is isolated per thread, so global reset semantics need careful wording

3. `cust::memory::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why it was consulted:
     - the upload path ultimately manages device-resident buffers through the GPU runtime
   - Standard informed:
     - uploaded device resources are lifetime-sensitive and should not be invalidated by unclear cache rules

## Findings

## Correctness And Runtime Semantics

### 1. `reset_gpu_strategy_executable_cache` cannot actually clear other threads’ uploaded-program caches

- Evidence:
  - process-global caches are cleared through `Mutex<HashMap<...>>`: `223-241`
  - the per-thread upload cache is cleared with `GPU_STRATEGY_EXECUTABLE_UPLOAD_CACHE.with(...)`: `232-235`
  - uploaded programs live in `thread_local!` state: `172-176`
- Why this matters:
  - the function name reads like a full reset, but under CUDA it only clears uploads for the current thread
  - a maintainer or test author could assume a stronger reset guarantee than the implementation can provide
- Smallest justified fix:
  - document reset scope explicitly
  - consider renaming or adding a comment that uploaded-thread entries are only cleared for the current thread

## API And Behavior Contracts

### 2. `get_or_build_gpu_strategy_executable` has hidden upload side effects that are not obvious from the return type

- Evidence:
  - cache hit path still performs per-thread upload under CUDA: `287-294`
  - miss path uploads before inserting metadata into the process-global cache: `297-312`
- Why this matters:
  - the function returns a small metadata struct, but it can also mutate thread-local uploaded state and touch device upload code
  - callers and reviewers need that side effect to be explicit
- Smallest justified fix:
  - add rustdoc that this call may upload the ABI program for the current thread/device under CUDA builds

## Cache Design

### 3. The runtime caches are intentionally sticky, but the lifetime and growth policy are undocumented

- Evidence:
  - both process-global caches and thread-local upload cache are unbounded maps: `154-176`
  - no eviction or size cap exists in `188-354`
- Why this matters:
  - long-lived sessions generating many distinct plan/executable fingerprints can grow resident cache state
  - that may be acceptable, but it should be a conscious tradeoff
- Smallest justified fix:
  - document the caches as intentionally unbounded for now
  - defer eviction policy until runtime telemetry shows it is needed

## Tests And Missing Verification

### 4. The tests cover reuse well, but not the current-thread-only reset limitation

- Evidence:
  - tests validate bundle reuse, executable reuse, reset counters, and shape-sensitive keys: `355-637`
- Missing coverage:
  - no test or comment documents that thread-local upload cache reset is limited to the current thread
- Smallest justified fix:
  - add a targeted test or doc note guarding the reset-scope assumption

## Code-Quality Improvement Register

- Safe now:
  - document reset scope
  - document upload side effects on `get_or_build_gpu_strategy_executable`
  - document cache growth policy
- Defer:
  - cache eviction
  - redesigning the split between process-global metadata and per-thread uploads

## Technical-Debt Reduction Register

- Safe now:
  - contract documentation
  - reset-scope clarification
- Defer:
  - multi-thread upload-cache invalidation strategy
  - eviction / lifecycle management
- Do not casually remove:
  - current fingerprint-based reuse behavior
  - current per-thread upload ownership

## Performance Opportunity Register

- Safe structural work:
  - none beyond documentation and tests
- Benchmark-sensitive:
  - any change to cache key shape, upload timing, or reset semantics

## Recommended Action

`Small surgical refactor`

This runtime layer is basically sound. The highest-value work is clarifying what “reset” means, what this API uploads implicitly, and what cache lifetime the module currently promises.

## Exact Local Verification Required If Changed

1. File-local runtime-cache tests:
   - `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`
2. GPU-program compiler tests:
   - `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`
3. Representative dynamic preview/eval surface:
   - `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not broaden the meaning of cache reset in docs or code unless other-thread upload invalidation is actually implemented
- Do not add eviction heuristics without measuring their effect on repeated GPU dynamic execution
- Do not separate metadata caching from upload caching further unless the device-lifetime contract is reviewed end to end
