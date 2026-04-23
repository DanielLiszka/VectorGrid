# Review Packet: `crates/app_core/src/options_expression_gpu.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_expression_gpu.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally useful and locally verified under CUDA features, but not fully public-review ready because key runtime assumptions remain implicit at the Rust/CUDA boundary, poisoned runtime-store locks still panic instead of returning recoverable errors, and cache/context policy is simple rather than explicitly tuned for long-lived multi-runtime GPU use`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `CUDA-runtime-critical`, `Unsafe-adjacent`, `Hot-path`, `Benchmark-sensitive`, `Cross-runtime-sensitive`

## What This File Owns

`options_expression_gpu.rs` owns the CUDA-side runtime for the options-expression lane. It handles module loading, per-device runtime/context storage, tuple-chunk planning, device upload caches for shared and outcome tensors, reusable device output buffers, and both the per-split and combined kernel launch paths.

It owns:

- CUDA module/runtime setup:
  - module names and chunk-plan constants: `crates/app_core/src/options_expression_gpu.rs:20-27`
  - compute-capability probe and module load:
    - `29-60`
- GPU-side result/cache DTOs:
  - `OptionsGpuTupleAggregate`: `64-71`
  - `OptionsGpuRuntimeCacheUsage`: `74-95`
  - tensor-run containers: `98-111`
- uploaded device-input and reusable-output ownership:
  - `UploadedSharedInputs`: `114-121`
  - `UploadedOutcomeInputs`: `123-127`
  - `ReusableOutputBuffers`: `129-152`
  - `CombinedReusableOutputBuffers`: `144-152`
  - upload/allocation/copy helpers:
    - `393-560`
- chunk planning:
  - budget helpers:
    - `188-245`
  - `OptionsGpuChunkPlan` and constructors:
    - `171-186`
    - `247-390`
- per-device runtime:
  - `OptionsExpressionGpuRuntime`: `562-573`
  - runtime creation, kernel-cap cache, shared/outcome cache, reusable buffer reuse:
    - `575-706`
- active scoring paths:
  - range scorer: `707-820`
  - combined search/holdout/stress scorer: `822-1016`
  - plan inspection helpers: `1018-1090`
- process-wide runtime store and exported prewarm hook:
  - runtime store / `with_runtime`: `1092-1137`
  - `prewarm_options_expression_gpu_runtime`: `1139-1145`
- launch wrappers:
  - `launch_score_range`: `1148-1183`
  - `launch_score_search_holdout_stress_into_buffers`: `1185-1255`
  - `launch_score_search_holdout_stress`: `1257-1302`
- crate-internal entry points consumed by the CPU-side engine:
  - `score_options_expression_tensors_gpu_range`: `1304-1311`
  - `score_options_expression_tensors_gpu_multi_range`: `1313-1325`
  - `score_options_expression_tensors_gpu_search_holdout_stress`: `1327-1344`
  - plan inspection helpers: `1346-1363`
- file-local CUDA-feature tests:
  - `1366-1503`

This file does not own:

- the higher-level backend decision, exactify, or reporting flow:
  - `crates/app_core/src/options_expression.rs`
- the kernel source itself:
  - `crates/app_core/src/kernels/options_expression.cu`
- shared launch-cap detection primitives:
  - `crates/app_core/src/gpu_backtest_kernel.rs`

## Classification

- Public surface: partially
  - only `prewarm_options_expression_gpu_runtime` is publicly re-exported at `crates/app_core/src/lib.rs:100`
  - the actual scoring and inspection helpers are crate-internal
- Hot path: yes
  - this file drives kernel launches and host/device transfer cadence
- CUDA/runtime-critical: yes
  - this is the runtime side of the options-expression GPU path
- Strategy-local: yes
  - it is specific to the options-expression workflow
- Low-risk support code: no
  - device memory ownership, launch parameters, and runtime context policy make this high-sensitivity backend code

## Direct Interactions

### Backend interaction map

- input contract source:
  - `crates/app_core/src/options_expression.rs:196-207`
  - GPU entry-point imports: `30-34`
- higher-level CPU/GPU orchestration:
  - `crates/app_core/src/options_expression.rs:3394-3513`
  - `3783-3839`
  - `5459-5539`
- shared launch-cap helpers and parallel-runtime sibling:
  - `crates/app_core/src/gpu_backtest_kernel.rs:1-6`
  - context creation in sibling runtime: `1840-1866`
- kernel module dependency:
  - `crates/app_core/src/kernels/options_expression.cu`
- Tauri/CLI runtime prewarm callers:
  - `src-tauri/src/lib.rs:61-67`
  - `src-tauri/src/bin/ta_options_expression_feasibility.rs:12`
  - `329`
- exported prewarm surface:
  - `crates/app_core/src/lib.rs:100`

### Downstream verification impact if changed

- changing chunk planning or kernel launch cadence affects:
  - measured GPU-score throughput
  - feasibility probes and backend auto decisions in `options_expression.rs`
- changing runtime/context handling affects:
  - `prewarm_options_expression_gpu_runtime`
  - any workflow that shares a GPU with the Double-MA GPU runtime
- changing cache behavior affects:
  - repeated feasibility/run warm behavior
  - VRAM pressure and device-memory residency

## Standards And Research Log

This packet was written after targeted web research focused on CUDA context behavior, asynchronous kernel-launch semantics, transfer overlap requirements, Rust synchronization semantics, `cust` device-buffer safety expectations, and public Rust documentation expectations.

### Sources consulted

1. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file exists to turn host-prepared inputs into efficient GPU execution
   - Standard informed:
     - keep reusable data on-device when practical, measure before changing performance policy, and treat transfer/launch choreography as performance-critical

2. NVIDIA CUDA C++ Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why it was consulted:
     - the file launches kernels directly and manages its own CUDA context
   - Standard informed:
     - kernel launches are asynchronous by default, and kernels from different contexts cannot execute concurrently on the same GPU

3. `cust::memory::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why it was consulted:
     - this file uses `DeviceBuffer::uninitialized(...)` for reusable outputs
   - Standard informed:
     - uninitialized device buffers are valid only when later use guarantees a full overwrite before any read/copy

4. `cust::context::CurrentContext`
   - Source: <https://docs.rs/cust/latest/cust/context/struct.CurrentContext.html>
   - Why it was consulted:
     - the runtime explicitly sets the current context per thread before launches
   - Standard informed:
     - context affinity is thread-local and explicit in this layer

5. `std::sync::Mutex`
   - Source: <https://doc.rust-lang.org/std/sync/struct.Mutex.html>
   - Why it was consulted:
     - the process-wide runtime store is `Mutex`-guarded and currently uses `.expect(...)`
   - Standard informed:
     - poisoned locks are advisory and can be handled explicitly rather than always panicking

6. `BTreeMap` in `std::collections`
   - Source: <https://doc.rust-lang.org/std/collections/struct.BTreeMap.html>
   - Why it was consulted:
     - all device-input caches are `BTreeMap`s and eviction removes `keys().next()`
   - Standard informed:
     - current cache eviction is key-ordered, not recency-aware

7. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the exported prewarm hook and the CUDA-bound runtime assumptions are undocumented
   - Standard informed:
     - public functions should document errors, panic-relevant behavior, and usage examples

## Findings

## Correctness And Safety

### 1. Launch-bound event ranges are trusted rather than locally validated against the uploaded tensors

- Evidence:
  - lane-local tensor length equality is checked during outcome upload:
    - `crates/app_core/src/options_expression_gpu.rs:435-451`
  - range scoring converts `event_start` / `event_end` directly to `u32` and forwards them into the kernel:
    - `749-753`
    - `795-803`
  - combined scoring likewise forwards `search_end` / `holdout_end` directly:
    - `839-842`
    - `982-993`
- Why it matters:
  - the file assumes its callers always provide bounds that fit the uploaded outcome tensors
  - if that contract is ever broken upstream, the failure mode moves into the kernel path rather than failing early here
- Smallest justified fix:
  - add explicit Rust-side bound validation against the uploaded outcome tensor length before launch
  - keep kernel-side guards if they exist, but do not rely on them as the first line of defense
- Review judgment:
  - medium correctness / runtime-guardrail issue

### 2. The uninitialized output-buffer strategy is sound only if the kernels fully write every output slot

- Evidence:
  - reusable outputs allocate with `DeviceBuffer::uninitialized(...)`:
    - `crates/app_core/src/options_expression_gpu.rs:456-478`
    - `529-537`
  - the safety comments explicitly rely on full-write behavior from the score kernels
  - host copies then read every element of those device buffers:
    - `488-507`
    - `542-549`
- Why it matters:
  - this is a real Rust/CUDA safety contract spanning two files:
    - `options_expression_gpu.rs`
    - `kernels/options_expression.cu`
  - if a future kernel change leaves any output slot unwritten, the subsequent host copy violates the assumed contract
- Smallest justified fix:
  - keep the current strategy because it is performance-conscious and locally documented
  - but strengthen the cross-file contract:
    - add a short comment tying the assumption to the exact kernel behavior
    - keep parity tests close to both the Rust wrapper and the CUDA kernel
- Review judgment:
  - high safety / CUDA-contract issue, but primarily documentation-and-verification hardening rather than immediate code replacement

### 3. The process-wide runtime store still panics on poisoned locks

- Evidence:
  - runtime store uses `Mutex<BTreeMap<u32, Arc<Mutex<OptionsExpressionGpuRuntime>>>>`:
    - `crates/app_core/src/options_expression_gpu.rs:1101-1108`
  - `with_runtime` uses `.lock().expect(...)` for both the store and the per-device runtime:
    - `1121-1134`
  - the test helper also panics on poison:
    - `1110-1113`
- Why it matters:
  - callers get `Result<_, String>` from the runtime surface
  - a poisoned lock bypasses that and escalates to panic
- Smallest justified fix:
  - convert poisoned-lock handling into a recoverable `Err(...)` or reinitialize-on-poison path
  - reserve panics for genuine invariant violations, not shared-state recovery
- Review judgment:
  - medium correctness / error-handling issue

## API And Naming

### 1. The only public hook, `prewarm_options_expression_gpu_runtime`, exposes meaningful behavior with no rustdoc contract

- Evidence:
  - exported public function:
    - `crates/app_core/src/options_expression_gpu.rs:1139-1145`
  - re-exported at:
    - `crates/app_core/src/lib.rs:100`
  - no rustdoc explains:
    - what prewarm actually initializes
    - that it is per-device and process-wide
    - what failures can occur
    - how it interacts with thread-local current-context setting
- Smallest justified fix:
  - add rustdoc with:
    - purpose
    - per-device/process-wide behavior
    - likely errors
    - a short usage example
- Review judgment:
  - medium public-surface documentation issue

## Error Handling

### 1. Module-load fallback is intentionally forgiving, but it does not surface which path actually won

- Evidence:
  - module load attempts:
    - sm_89 cubin first: `41-47`
    - PTX with multiple JIT option sets next: `49-59`
  - the function only returns the winning module or a final `Err(String)`
- Why it matters:
  - the fallback sequence itself is good for compatibility
  - but if a preferred cubin or optimized JIT path fails, the runtime surface gives no diagnostic note describing the fallback that occurred
- Smallest justified fix:
  - preserve the fallback behavior
  - optionally surface a debug-only note in profiling/diagnostic flows when module loading falls back from cubin to PTX or from higher-JIT to lower-JIT options
- Review judgment:
  - low-medium error-handling / observability issue

## Module Boundaries And Technical Debt

### 1. Cache eviction is simple and bounded, but it is neither recency-aware nor byte-budgeted

- Evidence:
  - cache cap is entry-count based:
    - `GPU_INPUT_CACHE_MAX_ENTRIES = 16`: `crates/app_core/src/options_expression_gpu.rs:22`
  - both shared-input and outcome-input caches are `BTreeMap`s:
    - `568-570`
  - eviction removes `cache.keys().next()`:
    - `1092-1098`
  - large outcome tensors can stay resident as single entries:
    - `617-660`
- Why it matters:
  - the cache name suggests “oldest,” but the actual policy is lexicographically first key
  - VRAM pressure is controlled indirectly by entry count, not by bytes retained
- Smallest justified fix:
  - clarify the current policy in comments/docs now
  - if repeated real workloads show eviction churn or VRAM pressure, switch later to a recency-aware and/or byte-budgeted policy
- Review judgment:
  - medium technical-debt / perf-policy issue

### 2. This runtime is a second CUDA-context owner in the same application

- Evidence:
  - this file creates its own context:
    - `crates/app_core/src/options_expression_gpu.rs:577-583`
  - `gpu_backtest_kernel.rs` also creates its own context for the same CUDA stack:
    - `crates/app_core/src/gpu_backtest_kernel.rs:1840-1866`
- Why it matters:
  - NVIDIA guidance recommends avoiding multiple contexts per GPU within the same CUDA application because contexts are time-sliced and carry switching plus memory overhead
  - this is not an immediate correctness bug, but it is real cross-runtime debt
- Smallest justified fix:
  - do not rush to unify contexts without measurement
  - document the separate-context design and treat any context-sharing refactor as a cross-runtime performance project
- Review judgment:
  - medium technical-debt / cross-runtime performance issue

## Docs And Examples

### 1. The file documents some unsafe assumptions inline, but not the runtime behavior that other contributors actually need

- Evidence:
  - the only substantial comments are safety notes on uninitialized device buffers:
    - `crates/app_core/src/options_expression_gpu.rs:456-478`
    - `535-537`
  - missing documentation includes:
    - chunk-plan budget policy and env override:
      - `217-245`
    - cache eviction policy:
      - `1092-1098`
    - public prewarm semantics:
      - `1139-1145`
    - why the runtime immediately synchronizes after launches:
      - `1185-1302`
- Smallest justified fix:
  - add targeted comments/rustdoc at the public hook and the chunk-plan/cache-policy seams
  - keep the inline unsafe comments; they are useful and should remain

## Tests And Missing Verification

### 1. Current CUDA-feature coverage is solid for chunk planning and runtime reuse, but not for several edge contracts

- Evidence:
  - file-local CUDA-feature verification:
    - `cargo test -p ta_app_core options_expression_gpu::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
      - passed `7` tests
  - feature compile verification:
    - `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
      - passed
  - GPU bench compile verification:
    - `cargo bench -p ta_app_core --bench options_expression_gpu_score --features cuda,cuda-backtest-kernel --no-run`
      - passed
- Missing tests I would want before changing this file:
  - explicit range-bound validation failures
  - a regression test proving cache eviction policy if it is changed
  - a cross-check tying the Rust uninitialized-buffer assumption to kernel full-write behavior
  - broader `options_expression` GPU path tests when changing runtime/context policy

## Performance, Allocation, And CUDA Concerns

### 1. The current launch wrappers serialize each launch with an immediate synchronize-and-copy sequence

- Evidence:
  - plain range path:
    - kernel launch: `1158-1177`
    - immediate `stream.synchronize()`: `1181`
    - immediate host copy: `1182-1183`
  - combined path:
    - launch: `1199-1251`
    - immediate `stream.synchronize()`: `1255`
    - then host copy in `launch_score_search_holdout_stress`: `1271-1298`
- Why it matters:
  - CUDA kernel launches are asynchronous by default
  - current code intentionally converts that into a serialized launch/wait/copy loop
  - because host buffers are ordinary `Vec`s rather than pinned buffers, overlapping transfer/compute would need deliberate redesign anyway
- Review judgment:
  - this is benchmark-sensitive
  - do not “optimize” it casually without Nsight/profile evidence

### 2. The file already follows one important CUDA best-practice: keeping reusable inputs and outputs device-resident across repeated launches

- Evidence:
  - shared and outcome inputs are cached in device memory:
    - `568-570`
    - `617-660`
  - reusable output buffers are retained and resized only when necessary:
    - `664-705`
- Review judgment:
  - preserve this posture unless profiling shows a better measured strategy
  - if transfer overlap is explored later, treat it as a measured extension of the existing residency design, not a rewrite

## Code-Quality Improvement Register

- Safe now:
  - add explicit Rust-side event-bound validation before launch
  - convert poisoned-lock panics into recoverable errors
  - add rustdoc for `prewarm_options_expression_gpu_runtime`
  - document the actual cache eviction policy
  - document the full-write kernel assumption as a Rust/CUDA contract
- Coordination required:
  - any change to synchronization/copy choreography
  - any move toward byte-budgeted or LRU-like device caches
  - any attempt to share CUDA contexts with `gpu_backtest_kernel.rs`
- Defer until later pass:
  - context unification across GPU subsystems
  - async copy/overlap redesign
  - deeper cache-policy redesign

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing docs
  - poisoned-lock panic paths
  - missing explicit range validation
  - ambiguous “oldest” cache-eviction wording
- Coordination required:
  - cache byte-budgeting
  - cross-runtime context ownership
  - any change to host/device synchronization cadence
- Useful debt that should not be “cleaned up” casually:
  - current device-resident cache reuse
  - current chunk-plan headroom policy
  - cubin-to-PTX compatibility fallback

## Performance Opportunity Register

- Safe structural improvements:
  - none urgent beyond better docs and explicit bounds checks
- Benchmark-sensitive candidates:
  - removing per-launch synchronization
  - pinned-host-memory transfer paths
  - byte-budgeted or LRU-like cache policy
  - shared-context architecture across GPU runtimes
- Not the right place to chase performance without evidence:
  - changing chunk budgets
  - changing launch-shape selection
  - rewriting reuse policy for aesthetics

## Observability And Diagnostics Gaps

- module-load fallback path is silent
- cache-eviction policy is implicit
- public prewarm behavior is undocumented
- Rust side does not clearly state that event bounds are caller-validated today

## Implementation Staging Guidance

- Safe now:
  - rustdoc
  - range validation
  - poisoned-lock recovery
  - extra comments around cache policy and kernel full-write assumptions
- Coordination required:
  - cache redesign
  - launch/copy overlap work
  - shared CUDA context work
- Defer until later pass:
  - broad runtime-architecture changes

## Change Blast Radius

- Local only:
  - docs
  - bounds checks
  - poisoned-lock recovery
  - cache-policy comments
- Same subsystem:
  - cache-policy or synchronization changes affect `options_expression.rs` GPU ranking behavior
- Cross-backend / frontend-backend contract:
  - context-sharing or runtime-ownership changes affect other CUDA runtimes and Tauri prewarm flows

## Open Questions Or Assumptions

- assumption:
  - the CUDA kernels do fully write every output slot that the Rust side later copies back
- open question:
  - whether separate CUDA contexts for options-expression and Double-MA are materially hurting measured throughput in real mixed workflows
- open question:
  - whether byte-budgeted eviction is necessary for real provider-backed datasets or only for future scale

## Recommended Action

`Small surgical refactor`

The right next pass is:

- add explicit launch-bound validation
- harden poisoned-lock handling
- document the runtime/cache/unsafe contracts
- defer any synchronization, cache-policy, or context-sharing redesign until it is measured

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core options_expression_gpu::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core options_expression::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
- `cargo bench -p ta_app_core --bench options_expression_gpu_score --features cuda,cuda-backtest-kernel --no-run`
- if launch/synchronization/cache behavior changes:
  - `powershell -File scripts/profile_options_expression_nsys.ps1 -CaptureMode nvtx`

## Regression Risk And Feature-Preservation Constraints

- do not replace the current device-resident reuse strategy casually
- do not change synchronization or chunk-planning behavior without profile evidence
- do not alter context ownership across GPU subsystems unless the coordination plan includes `gpu_backtest_kernel.rs` and the relevant profiling workflow
