# Review Packet: `crates/app_core/src/strategy_dynamic.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_dynamic.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Important semi-public runtime surface, but not fully review-ready because execution-policy rules, fallback semantics, env-controlled shadow behavior, and test-state assumptions are only partially documented`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Semi-public-contract`, `Runtime-policy`, `CPU/GPU-routing`, `Semantics-sensitive`, `Test-stateful`

## What This File Owns

`strategy_dynamic.rs` is the dynamic-strategy execution hub for the Double-MA family and its composite-profile variants. It owns:

- preview / backend resolution for dynamic IR requests:
  - `preview_dynamic_double_ma_strategy_ir`: `crates/app_core/src/strategy_dynamic.rs:427`
  - `prepare_runnable_dynamic_double_ma_plan`: `444`
- lazy CPU fallback compilation and lazy CPU context materialization:
  - `LazyPreparedDynamicCpuContext`: `70`
  - `LazyCompiledCpuFallbackPlan`: `113`
  - `RunnableDynamicDoubleMaPlan`: `150`
- dynamic strategy env flags, shadow controls, and artifact logging:
  - `497-604`
- static-config-to-dynamic-plan compilation helpers:
  - `649-791`
- dynamic indicator slot staging, cache keys, cancellation, and optional CUDA preparation:
  - `835-1295`
- CPU-side dynamic plan evaluation and trade-stat computation:
  - `1615-2454`
- a large file-local test suite:
  - `2456-3779`

It does not just “compile IR.” It also chooses routes, stages data, performs CPU fallback, writes artifacts, and embeds test-only counters.

## Classification

- Public surface: effectively yes
  - exported from `crates/app_core/src/lib.rs:280-285`
  - surfaced to Tauri via `src-tauri/src/commands_dynamic.rs:3-42`
  - indirectly drives frontend preview flows through `web/src/services/optimizeService.ts:1034`
- Hot path: partially
  - preview is setup-time work, but indicator staging and metric evaluation are runtime compute paths
- CUDA/runtime-critical: yes
  - GPU-vs-CPU backend choice and indicator staging are decided here
- Strategy-local: partially
  - dynamic Double-MA is the direct focus, but composite profiles also route through this file
- Test-sensitive: yes
  - file owns global counters and env-driven behavior that leak into test scheduling

## Direct Interactions

### Backend interaction map

- Tauri JSON preview command:
  - `src-tauri/src/commands_dynamic.rs:3-42`
- Double-MA optimization/runtime flow:
  - `crates/app_core/src/double_ma.rs:5433`
  - `11117-11694`
- Composite CPU / MO evaluation:
  - `crates/app_core/src/composite_cpu.rs:12517-12805`
  - `crates/app_core/src/composite_mo.rs:2510`
- GPU parity / runtime tests:
  - `crates/app_core/src/gpu_backtest_kernel.rs:16095-18873`
- Preset compilation coverage:
  - `crates/app_core/src/strategy_presets.rs:113-246`

### Downstream verification impact if changed

- changing backend-resolution rules changes whether previewed IR stays on `GpuUnified` or silently falls back to `Cpu`
- changing slot-staging rules changes both CPU execution and CUDA-prepared dynamic indicator reuse
- changing env/shadow behavior changes observability and experiment gating outside the file itself
- changing lazy fallback semantics changes the request lifecycle assumed by `double_ma.rs` and the Tauri preview command

## Standards And Research Log

This packet was written after targeted web research focused on lazy initialization semantics, internal API documentation standards, and CUDA/runtime contract boundaries.

### Sources consulted

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file exports multiple behavior-heavy helpers whose contracts are not obvious from signatures
   - Standard informed:
     - execution-policy, fallback, and error behavior need explicit local documentation

2. `std::sync::OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why it was consulted:
     - this file relies on lazy fallback compilation and lazy CPU context construction through `OnceLock`
   - Standard informed:
     - lazy initialization is appropriate here, but the one-time initialization contract should be narrow and documented

3. NVIDIA CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - the file mixes CPU fallback and CUDA-prepared indicator inputs in a semantics-sensitive staging path
   - Standard informed:
     - host/device staging behavior should be explicit, and benchmark-sensitive route decisions should not be “cleaned up” casually

## Findings

## Correctness And Safety

### 1. The file concentrates too many unrelated responsibilities behind one module boundary

- Evidence:
  - preview and route resolution: `352-495`
  - env parsing and artifact I/O: `497-604`
  - static-config compilation helpers: `649-791`
  - indicator staging and cache logic: `835-1295`
  - CPU metrics engine: `1615-2454`
- Why this matters:
  - this is not just a “dynamic strategy helper”; it is the control plane, staging layer, and CPU evaluator
  - semantic changes in one section can silently alter preview behavior, runtime behavior, or experiment telemetry elsewhere
- Smallest justified fix:
  - do not rewrite this file wholesale now
  - instead, document the ownership split and plan a bounded future extraction into:
    - preview / route policy
    - artifact / env controls
    - staging/cache helpers
    - evaluation engine

### 2. CPU staging still enforces a validator explicitly named for the GPU v1 contract

- Evidence:
  - `prepare_dynamic_indicator_slot_series_cpu_with_controls` calls `validate_dynamic_indicator_output_for_gpu_v1`: `963-1037`
  - the shared CUDA/non-CUDA staging entry point also validates through the same helper: `1165-1269`
  - the validator name and error strings are explicitly GPU-v1-specific: `260-350`
- Why this matters:
  - the current behavior may be intentional, but the contract boundary is unclear
  - a maintainer reading the CPU path could reasonably assume CPU-only dynamic execution allows shapes that the GPU path forbids
  - accidental “broadening” or “simplifying” here could create cross-backend drift
- Smallest justified fix:
  - either rename the validator to reflect that it is the current dynamic-strategy product contract for all routes, or split CPU validation from GPU-v1 validation and make the difference explicit

## API And Behavior Contracts

### 3. Env-controlled shadow and artifact behavior is embedded in core execution code and under-documented

- Evidence:
  - env readers: `497-533`
  - artifact record struct and append path: `535-604`
- Why this matters:
  - these flags affect runtime observability and experiment behavior, but their semantics are discoverable only by reading implementation
  - tests must guard environment access explicitly, which is a sign the contract is global-process state rather than local request state
- Smallest justified fix:
  - add rustdoc describing:
    - which env vars are read
    - what defaults apply
    - whether artifact failures are best-effort or fatal
  - defer moving this behavior into a dedicated shadow/telemetry module until after the contract is written down

## Tests And Missing Verification

### 4. The file’s default-threaded test suite is currently scheduling-sensitive

- Evidence:
  - failed:
    - `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture`
  - observed failure:
    - `prepare_runnable_dynamic_double_ma_plan_defers_cpu_context_until_cpu_execution`
      asserted `compile_dynamic_strategy_plan_call_count() == 1`, but saw `2`
  - cascading follow-on failures:
    - `preview_dynamic_double_ma_strategy_ir_falls_back_to_cpu_for_halftrend_shape`
    - `preview_dynamic_double_ma_strategy_ir_falls_back_to_cpu_for_non_cross_exit_mode`
    - both then hit `test env lock poisoned`
  - passed when serialized:
    - `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`
- Why this matters:
  - the production code may be fine, but the test harness is not robust against default Rust test parallelism
  - global counters and poisoned env locks hide real regressions behind scheduler-dependent noise
- Smallest justified fix:
  - serialize or isolate the affected tests
  - move call-count assertions behind a dedicated test guard
  - stop allowing one panic to poison later env-sensitive tests

## Module Boundaries And Technical Debt

### 5. The dynamic-indicator cache key is intentionally heuristic, but that intent is not documented

- Evidence:
  - `dataset_cache_signature` uses dataset length, first/last timestamps, first/last close/source bits, and source hash: `907-919`
  - normalized strings and param fingerprints use `DefaultHasher`: `856-889`
- Why this matters:
  - the cache is clearly optimized for lightweight reuse rather than full dataset fingerprinting
  - that can be perfectly valid for in-call staging reuse, but future maintainers could mistake it for a stronger identity guarantee
- Smallest justified fix:
  - document the cache-key strength explicitly as a local staging optimization, not a durable dataset identity mechanism

## Performance, Allocation, And CUDA Concerns

### 6. The runtime-sensitive portions should not be refactored for style without route-parity verification

- Evidence:
  - GPU input preparation is lazy and route-aware: `1092-1155`, `1165-1269`
  - CPU context preparation intentionally defers work for GPU-resolved plans until CPU fallback is requested: `150-172`, `444-495`
- Why this matters:
  - these are the right kinds of optimizations to preserve unless measurements or correctness evidence justify change
  - a stylistic “simplification” could eagerly build CPU fallback state or broaden host-side staging work
- Safe-now conclusion:
  - preserve the current lazy structure
- Smallest justified fix:
  - harden tests and docs first; defer structural cleanup

## Code-Quality Improvement Register

- Safe now:
  - document backend-resolution and env/shadow contracts
  - rename or document the GPU-v1 validator as the current cross-route contract
  - isolate global test counters and env-sensitive tests so the suite passes under default threading
  - document the cache-key strength and intended scope
- Defer:
  - breaking the file into smaller modules
  - changing route heuristics or indicator-staging behavior
  - broad cleanup of CPU/GPU branching

## Technical-Debt Reduction Register

- Safe now:
  - documentation hardening
  - test-isolation fixes
  - naming clarification around validation and shadow behavior
- Defer:
  - module extraction
  - artifact/telemetry subsystem separation
- Do not casually remove:
  - lazy CPU fallback construction
  - current preview fallback rules
  - current indicator-staging cache reuse

## Performance Opportunity Register

- Safe structural work:
  - none until route-parity tests remain green under default threading
- Benchmark-sensitive:
  - any change to CPU/GPU staging, prepared CUDA input timing, or fallback eagerness

## Recommended Action

`Larger refactor but defer`

This file contains too much logic, but the risk is semantic drift, not obvious algorithmic failure. The right immediate work is contract documentation and test isolation. Structural decomposition should wait until those contracts are explicit and route parity is protected.

## Exact Local Verification Required If Changed

1. Dynamic strategy suite, default threading:
   - `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture`
2. Dynamic strategy suite, serialized:
   - `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`
3. Tauri preview surface:
   - `cargo test -p ta_desktop_demo_app commands_dynamic -- --nocapture`
4. Representative dynamic-runtime parity surface:
   - `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`

### Verification run for this packet

- Failed:
  - `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture`
- Passed:
  - `cargo test -p ta_app_core strategy_dynamic::tests::prepare_runnable_dynamic_double_ma_plan_defers_cpu_context_until_cpu_execution -- --nocapture`
  - `cargo test -p ta_app_core strategy_dynamic::tests::preview_dynamic_double_ma_strategy_ir_falls_back_to_cpu_for_halftrend_shape -- --nocapture`
  - `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`

## Regression Risk And Feature-Preservation Constraints

- Do not broaden or narrow the accepted dynamic-indicator output contract without coordinating both preview and execution paths
- Do not eagerly compute CPU fallback state for GPU-resolved plans without measuring route-level impact
- Do not move env/shadow logic out of this file until the current behavior is documented and covered by tests
- Treat the current default-threaded test failure as a real quality issue; do not “fix” it by weakening assertions that protect lazy fallback behavior
