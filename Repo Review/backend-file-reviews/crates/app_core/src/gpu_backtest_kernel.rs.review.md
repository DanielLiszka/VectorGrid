# Review Packet: `crates/app_core/src/gpu_backtest_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/gpu_backtest_kernel.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Powerful and strategically important shared CUDA runtime, but not fully public-review ready because several public helper APIs expose under-documented preconditions, the dynamic uploaded-batch path under-validates caller-provided geometry, and some runtime/diagnostic state is retained through thread-local side channels without enough explicit contract documentation`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Hot path`, `CUDA/runtime-critical`, `Unsafe-adjacent`, `Benchmark-sensitive`, `Semantics-sensitive`

## What This File Owns

`gpu_backtest_kernel.rs` is the shared CUDA runtime hub for the exact `GpuKernel` Double-MA path and several adjacent GPU-backed helper surfaces.

Major owned responsibilities include:

- runtime creation, module loading, stream ownership, scratch-buffer reuse, and per-device caches:
  - `CudaKernelRuntime`: `crates/app_core/src/gpu_backtest_kernel.rs:1800-1877`
  - thread-local runtime and diagnostic state: `2004-2008`
- launch-shape and kernel-capability heuristics:
  - `detect_device_launch_caps`: `581-608`
  - `detect_kernel_launch_caps`: `611-632`
  - `choose_launch_shape_1d_from_candidates`: `703-786`
- CUDA graph capture / replay helper:
  - `capture_graph_exec`: `849-887`
- shared public config and precomputed-matrix DTOs:
  - `KernelConfig`: `2174-2182`
  - `PrecomputedHostMaMatrix`: `2185-2188`
- device-regime upload/build/download helpers:
  - `upload_host_regime_filter`: `2343-2375`
  - regime builders: `2951-3420`
- uploaded dynamic Double-MA batch upload / execution:
  - `UploadedDoubleMaDynamicProgramBatch`: `2504-2526`
  - `upload_double_ma_dynamic_program_batch`: `2610-2724`
  - `eval_uploaded_double_ma_dynamic_program_gpu_kernel`: `2726-2898`
- exact batch evaluation entrypoints:
  - `eval_double_ma_batch_gpu_kernel*`: `8728-11904`
- optimization entrypoints and timing side-channel capture:
  - `optimize_double_ma_gpu_kernel*`: `11701-11982`
- campaign VRAM staging / resident-market management:
  - `CampaignVramPrefetchOutcome`: `3990-3993`
  - staging helpers: `4135-4320`, `11984-12191`

This file does not merely wrap kernels. It is the owning runtime layer for:

- module loading and context lifetime
- scratch allocation policy
- device/host transfer strategy
- launch heuristics
- staged resident market uploads
- score/shortlist diagnostics side channels
- several public helper APIs re-exported from `ta_app_core`

## Classification

- Public surface: yes
  - the module itself is public from `crates/app_core/src/lib.rs:26`
  - dynamic-batch helpers are re-exported from `crates/app_core/src/lib.rs:292-295`
- Hot path: yes
  - this is the exact `GpuKernel` execution and optimization hub
- CUDA/runtime-critical: yes
  - it owns contexts, streams, module loading, launches, uploads, and runtime caches
- Strategy-local: no
  - shared infrastructure used by `double_ma`, validation, dynamic strategy runtime, and several strategy-specific GPU helpers
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- main optimization and orchestration caller:
  - `crates/app_core/src/double_ma.rs:2291`, `4945`, `7583-7595`, `7774`, `12328`, `13152`, `13428`, `13563`, `13839`
- validation callers:
  - `crates/app_core/src/double_ma_validate.rs:199-216`, `6830-7261`
  - `crates/app_core/src/composite_validate.rs:1107-1129`, `5990`
- shared dynamic GPU runtime:
  - `crates/app_core/src/strategy_gpu_runtime.rs:168`, `284-346`
- strategy-specific GPU helpers built on this runtime:
  - `asr_gpu_kernel.rs`, `carm_gpu_kernel.rs`, `cdmc_gpu_kernel.rs`, `kqsr_gpu_kernel.rs`, `mkcs_gpu_kernel.rs`, `rocb_gpu_kernel.rs`, `vcc_gpu_kernel.rs`, `vcet_gpu_kernel.rs`, `vemr_gpu_kernel.rs`
- standalone profiling / tooling:
  - `crates/app_core/src/bin/cupti_sass_probe.rs:1203`
  - `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
  - `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`

### Downstream verification impact if changed

- launch-shape or stream/runtime changes affect:
  - exact GPU optimization
  - exact GPU validation
  - dynamic Double-MA uploaded-batch execution
  - strategy GPU runtime and probe tooling
- resident/staged market cache changes affect:
  - campaign prefetch
  - reuse of market uploads across repeated runs
- public helper contract changes affect:
  - `ta_app_core` re-export consumers
  - bins and external callers using dynamic-batch or probe helpers

## Standards And Research Log

This packet was written after targeted web research focused on public API contract clarity, documentation of fallible and unsafe-adjacent behavior, thread-local runtime implications, CUDA stream / pinned-memory guidance, CUDA graph capture behavior, and multi-context performance constraints.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this module exposes public helper types and functions in a public crate module
   - Standard informed:
     - public APIs should validate arguments, keep implementation details private where possible, and make failure modes explicit

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - several public helper APIs here have sharp preconditions and error behavior but little or no rustdoc
   - Standard informed:
     - public items should document errors, panics, and safety-relevant constraints

3. The Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why it was consulted:
     - this file contains extensive unsafe and FFI-adjacent code paths, raw CUDA handles, uninitialized allocations, and pointer-based kernel launches
   - Standard informed:
     - unsafe-backed abstractions should keep caller obligations and invariants explicit

4. `std::thread_local!` documentation
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why it was consulted:
     - this runtime stores contexts, diagnostics, and scratch buffers in thread-local `RefCell` state
   - Standard informed:
     - thread-local state implies thread affinity and interior-mutable access patterns that should be made clear at API boundaries

5. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file manages pinned-memory staging, asynchronous transfers, stream overlap, launch heuristics, and context reuse
   - Standard informed:
     - preserve justified pinned-memory / multi-stream overlap behavior, avoid casual multi-context proliferation, and treat memory-retention changes as benchmark-sensitive

6. NVIDIA CUDA Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-programming-guide/index.html>
   - Why it was consulted:
     - this file captures and replays CUDA graphs via stream capture
   - Standard informed:
     - graph-capture helpers need explicit stream and error-handling behavior because stream capture has stateful rules and failure modes

## Findings

## Correctness And Safety

### 1. The public uploaded dynamic-batch API under-validates caller-provided geometry and `first_valid`

- Evidence:
  - public upload entrypoint validates row counts and basic limits but does not validate `first_valid <= lr.len()` or `stride == lr.len()`:
    - `crates/app_core/src/gpu_backtest_kernel.rs:2610-2656`
  - later execution uses `uploaded_batch.d_lr.len()` as `t_i` and forwards `uploaded_batch.first_valid` directly into kernel arguments:
    - `2778-2799`
  - sibling exact-evaluation paths in the same module do validate precomputed matrix width against the series length and validate `first_valid_override` against `n`:
    - `3442-3468`
    - `12424-12502`
  - a nearby upload helper also validates `first_valid <= len`:
    - `2343-2355`
- Why it matters:
  - this public surface can accept internally inconsistent uploaded batches even though the downstream kernel assumes consistent lengths and valid `first_valid`
  - even if the current in-repo callers provide well-formed inputs, the API contract is weaker than the exact-eval path it parallels
- Smallest justified fix:
  - safe-now:
    - reject `first_valid > lr.len()` or `first_valid >= lr.len()` according to the kernel’s actual semantic requirement
    - reject `stride != lr.len()` if the uploaded matrix contract is “one MA value per return sample”
  - docs:
    - document the expected relationship among `lr`, `stride`, matrix column count, and `first_valid`
- Review judgment:
  - high-severity public correctness issue
  - safe-now to validate, semantics-preserving if the contract already assumed those invariants

### 2. `prepare_cuda_kernel_runtime_for_probe` exposes a raw CUDA context as `usize` across a public module boundary

- Evidence:
  - helper definition:
    - `crates/app_core/src/gpu_backtest_kernel.rs:1907-1918`
  - the module is public:
    - `crates/app_core/src/lib.rs:26`
  - the in-repo consumer immediately casts the returned integer back to a CUDA context handle:
    - `crates/app_core/src/bin/cupti_sass_probe.rs:1203`
- Why it matters:
  - this is a type-erased raw FFI handle with no lifetime or thread-affinity guard
  - the helper depends on thread-local runtime ownership and the returned handle is only meaningful relative to that runtime/context state
- Smallest justified fix:
  - if external/public use is not intended:
    - reduce visibility to `pub(crate)`
  - otherwise:
    - wrap the handle in a dedicated opaque type and document thread-affinity / validity expectations
- Review judgment:
  - medium-to-high public-surface safety/design issue
  - semantics-sensitive if any external consumer already depends on the raw integer contract

## API And Naming

### 1. `CampaignVramPrefetchOutcome::AlreadyResident` conflates “already active” with “already staged”

- Evidence:
  - public-ish enum used by the staging entrypoint:
    - `crates/app_core/src/gpu_backtest_kernel.rs:3990-3993`
    - `11984-12005`
  - the staging helper returns `AlreadyResident` both when the requested dataset is active in `resident_market` and when it only exists in `staged_market`:
    - `4269-4282`
  - that outcome is forwarded into the higher-level public app-core result enum unchanged:
    - `crates/app_core/src/double_ma.rs:414-417`
    - `4945-4960`
- Why it matters:
  - “already resident” implies active immediate availability, but the staged case still requires later promotion into the active resident cache
  - the current enum is operationally useful but semantically blurry
- Smallest justified fix:
  - document the staged-versus-active meaning explicitly
  - optional later refinement:
    - split into `AlreadyResident` and `AlreadyStaged`
- Review judgment:
  - medium API/observability issue
  - safe-now as documentation; semantics-sensitive if the enum shape changes

### 2. Several public helper APIs expose advanced surfaces without rustdoc examples or contract notes

- Evidence:
  - public items without file-local public-contract docs include:
    - `KernelConfig`: `2174-2182`
    - `PrecomputedHostMaMatrix`: `2185-2188`
    - `UploadedDoubleMaDynamicProgramBatch`: `2504-2526`
    - `upload_double_ma_dynamic_program_batch`: `2610-2622`
    - `eval_uploaded_double_ma_dynamic_program_gpu_kernel`: `2726-2730`
    - `prepare_cuda_kernel_runtime_for_probe`: `1907-1918`
    - `campaign_stage_plain_double_ma_market`: `11984-12005`
- Why it matters:
  - these are not self-evident helper surfaces; they carry geometry, lifetime, and thread-local assumptions
- Smallest justified fix:
  - add rustdoc explaining:
    - input-shape invariants
    - thread-local/runtime affinity
    - whether the type/function is intended for external consumption or only tooling / adjacent runtime modules

## Error Handling

### 1. The kernel-module loader drops the earlier PTX/JIT failure context

- Evidence:
  - `load_double_crossover_module` tries cubin, then PTX with O2 JIT options, then PTX with fewer options, then bare PTX, but only the final failure is returned:
    - `crates/app_core/src/gpu_backtest_kernel.rs:1879-1904`
- Why it matters:
  - CUDA bring-up failures are among the hardest issues to root-cause in this subsystem
  - discarding the earlier loader errors weakens diagnostics for every consumer that depends on the shared module loader
- Smallest justified fix:
  - keep the first and second PTX-load failures and include them in the final error message
  - preserve the current fallback order
- Review judgment:
  - medium error-handling/diagnostics issue
  - safe-now, low regression risk

## Module Boundaries And Technical Debt

### 1. The module combines too many responsibilities to be casually refactored, but that breadth is now a real maintenance burden

- Evidence:
  - one file owns runtime creation, launch heuristics, graph capture, dynamic-program upload/eval, regime builders, exact batch eval, optimization, resident-market staging, side-channel diagnostics, and test harnesses
  - representative ownership points:
    - runtime: `1800-1877`
    - dynamic uploaded-batch path: `2504-2927`
    - regime builders: `2951-3420`
    - optimization pipeline: `11701-15494`
- Why it matters:
  - the code is strategically central, but the review burden is disproportionately high because unrelated concerns live together
  - a broad rewrite here would be dangerous; the right outcome is boundary clarification and smaller targeted hardening first
- Smallest justified fix:
  - safe-now:
    - document the major owned surfaces in rustdoc/module docs
  - defer-later:
    - consider splitting public DTO/helper surfaces from the exact optimization engine and from campaign staging helpers
- Review judgment:
  - real tech-debt issue
  - defer structural changes unless there is a bounded migration plan

### 2. There is objective low-risk cleanup debt from compiler-reported `unused_unsafe` warnings

- Evidence:
  - local verification with CUDA features enabled emitted `unused_unsafe` warnings at representative lines including:
    - `2148`
    - `6042`
    - `6605`
    - `7824`
    - `13399`
- Why it matters:
  - in a file already dense with real unsafe / FFI work, unnecessary `unsafe` blocks add review noise and make the genuinely dangerous areas harder to audit quickly
- Smallest justified fix:
  - remove `unsafe` blocks that the compiler already proves unnecessary, but only as a bounded cleanup pass
- Review judgment:
  - low-severity code-quality issue
  - safe-now if isolated from functional changes

## Docs And Examples

### 1. Public helper surfaces need contract documentation, not just signature exposure

- Evidence:
  - current public types/functions do not explain:
    - dynamic-batch geometry invariants
    - whether `prepare_cuda_kernel_runtime_for_probe` is same-thread only
    - whether uploaded batches are reusable across handles/devices
    - what `AlreadyResident` means when staging already occurred but promotion has not
- Smallest justified fix:
  - add rustdoc with `Errors` and, where relevant, `Safety`-style contract notes even for safe wrappers around unsafe-backed behavior

## Tests And Missing Verification

### 1. Coverage is strong on several targeted surfaces, but the sharp public helpers still lack direct negative tests

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel period_upload_signature_tracks_full_period_contents -- --nocapture`
      - passed
    - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel uploaded_dynamic_double_ma_batch_rejects_mismatched_handle_executable_shape -- --nocapture`
      - passed
    - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel staged_market_promotes_into_active_resident_cache -- --nocapture`
      - passed
  - useful existing tests include:
    - `period_upload_signature_tracks_full_period_contents`: `16169+`
    - `staged_market_promotes_into_active_resident_cache`: `16198+`
    - dynamic GPU parity and relaunch coverage: `18575+`, `18740+`
    - mismatched uploaded-handle rejection: `18841+`
- Missing tests I would want before changing the public helper contracts:
  - invalid `first_valid` rejection for `upload_double_ma_dynamic_program_batch`
  - matrix-width / `stride` mismatch rejection for uploaded dynamic batches
  - `prepare_cuda_kernel_runtime_for_probe` contract or visibility expectations
  - loader fallback diagnostics shape when all PTX/cubin attempts fail
  - explicit staged-versus-resident outcome behavior for campaign prefetch

## Performance, Allocation, And CUDA Concerns

### 1. The current pinned-memory and multi-stream design is directionally aligned with NVIDIA guidance and should not be casually “simplified”

- Evidence:
  - the file uses pinned host buffers and separate non-default streams for transfer/staging work:
    - representative pinned/upload helpers and stream ownership:
      - `1457+`, `1472`, `1736-1756`, `4378-4481`
      - runtime streams: `1803-1806`, `1844-1847`
- Review judgment:
  - keep this posture unless benchmarking says otherwise
  - avoid cleanup that collapses everything back into a single blocking stream/copy path

### 2. The dynamic uploaded-batch scratch cache is unbounded for production flows

- Evidence:
  - thread-local scratch cache:
    - `crates/app_core/src/gpu_backtest_kernel.rs:2004-2006`
  - insertion keyed by `(device_id, out_len)`:
    - `2776`
    - `2816-2824`
  - the only explicit full clear I found is the test-only reset helper:
    - `122-127`
- Why it matters:
  - long-lived threads can retain device buffers for many distinct output shapes with no eviction policy or public reset path
  - this is not automatically a correctness bug, but it is a real retention/perf debt in a desktop process
- Smallest justified fix:
  - safe-now:
    - document the retention behavior
  - coordination required:
    - add a bounded eviction or an explicit reset hook if evidence shows real VRAM pressure from shape churn
- Review judgment:
  - medium performance/memory-management issue
  - benchmark-sensitive; do not redesign casually

### 3. Launch-shape and dispatch heuristics are performance-critical and not the right place for speculative cleanup

- Evidence:
  - capability detection and launch-shape heuristics:
    - `581-786`
  - multiple route/contract heuristics and environment flags:
    - `4510-5800`
    - `6456-8726`
- Review judgment:
  - preserve these heuristics unless profiling or perf-gate evidence says otherwise
  - if changed, require GPU perf validation, not just unit tests

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc for public helper surfaces
  - validate uploaded dynamic-batch `first_valid` and matrix geometry
  - preserve earlier loader failure context in error messages
  - remove compiler-proven unnecessary `unsafe` blocks in isolated follow-up edits
- Coordination required:
  - raw-context helper visibility/type redesign
  - any change to `CampaignVramPrefetchOutcome`
  - any production eviction/reset policy for `DYNAMIC_GPU_METRICS_SCRATCH`
- Defer until later pass:
  - splitting this file into smaller modules

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing public contract docs
  - missing negative tests for public helper invariants
  - dropped loader diagnostics
- Coordination required:
  - thread-local runtime API exposure
  - staged-versus-resident prefetch status semantics
  - scratch-cache lifetime controls
- Useful debt that should not be “cleaned up” casually:
  - pinned-buffer and multi-stream staging design
  - launch-shape and route-selection heuristics
  - graph-capture helper flow

## Performance Opportunity Register

- Safe structural improvements:
  - none obvious beyond stronger diagnostics and bounded scratch-cache observability
- Benchmark-sensitive candidates:
  - scratch-cache eviction/reset
  - changes to pinned memory use
  - stream topology changes
  - launch-shape heuristic changes
- Not the right place to chase performance without evidence:
  - broad “simplify the kernel runtime” refactors
  - removing transfer overlap or staging logic for readability alone

## Observability And Diagnostics Gaps

- loader fallback loses early PTX/JIT failure context
- public probe helper does not document that it is exposing a raw context from thread-local runtime state
- prefetch result naming does not distinguish staged versus active-resident state
- dynamic uploaded-batch scratch retention is not surfaced to diagnostics or reset tooling

## Implementation Staging Guidance

- Safe now:
  - dynamic uploaded-batch input validation
  - rustdoc and contract notes
  - negative tests
  - richer loader error messages
- Coordination required:
  - probe-helper visibility/type changes
  - prefetch outcome semantics
  - scratch-cache reset/eviction policy
- Defer until later pass:
  - structural decomposition of the module

## Change Blast Radius

- Local only:
  - docs
  - loader diagnostic messages
  - `unused_unsafe` cleanup
- Same subsystem:
  - uploaded dynamic-batch validation
  - prefetch outcome wording
  - scratch-cache management
- Cross-backend/runtime:
  - any runtime, stream, module-loading, launch-shape, or resident-market policy change

## Open Questions Or Assumptions

- assumption:
  - the uploaded dynamic-batch contract is intended to require one MA sample column per `lr` value, even though that is not currently enforced at the upload boundary
- open question:
  - whether `prepare_cuda_kernel_runtime_for_probe` is meant to remain a semi-public tooling hook or should be narrowed to crate-local usage
- open question:
  - whether the app wants to distinguish “already staged for next promotion” from “already active resident” in campaign prefetch UX and telemetry

## Recommended Action

`Small surgical refactor`

The right next pass is not a rewrite. It is:

- harden the public uploaded-batch contract
- add missing contract docs
- improve loader diagnostics
- keep performance-sensitive runtime behavior unchanged unless measurement justifies touching it

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel period_upload_signature_tracks_full_period_contents -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel staged_market_promotes_into_active_resident_cache -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel uploaded_dynamic_double_ma_batch_rejects_mismatched_handle_executable_shape -- --nocapture`
- add and run focused negative tests for:
  - invalid `first_valid`
  - uploaded-batch `stride` / matrix-width mismatch
  - probe-helper contract misuse or narrowed visibility behavior
  - staged-versus-resident prefetch outcomes
- if stream/runtime/launch heuristics change:
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Regression Risk And Feature-Preservation Constraints

- do not casually change launch heuristics, stream topology, or pinned-memory behavior
- do not remove the uploaded dynamic-batch path or the probe helper without replacing the consumer path that depends on them
- do not silently alter campaign-prefetch behavior without verifying downstream progress and artifact consumers
- treat scratch-cache lifetime changes as performance work, not hygiene work
