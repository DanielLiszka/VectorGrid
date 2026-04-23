# Review Packet: `crates/app_core/src/dynamic_indicator_adapter.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/dynamic_indicator_adapter.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful and mostly sound adapter layer, but not fully public-review ready because a public unchecked CUDA entrypoint bypasses catalog validation, device-path compute failures are silently turned into host-backed CUDA success, and the module’s public docs/tests do not explain the unsafe-adjacent keepalive and fallback contract clearly enough`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `CUDA/runtime-critical`, `Unsafe-adjacent`, `Benchmark-sensitive`, `Semantics-sensitive`

## What This File Owns

`dynamic_indicator_adapter.rs` is the app-core adapter between catalog-owned dynamic-indicator metadata/validation and actual CPU/CUDA indicator dispatch. It also owns prepared CUDA input caching, fallback accounting, device-row slicing for chained indicators, and the validated versus unchecked CUDA entrypoints.

Major owned surfaces include:

- typed adapter error model:
  - `DynamicIndicatorAdapterError`: `crates/app_core/src/dynamic_indicator_adapter.rs:37-97`
- validated CPU path:
  - `compute_dynamic_indicator_cpu`: `183-238`
- prepared CUDA cache and ownership surface:
  - `PreparedCudaIndicatorInputs`: `587-609`
  - constructors and cache access: `612-763`
- device-row chaining helper:
  - `device_slice_ref_from_matrix_row`: `1119-1149`
- validated prepared-input CUDA path:
  - `compute_dynamic_indicator_cuda_with_inputs`: `1220-1301`
- unchecked/raw prepared-input CUDA path:
  - `compute_dynamic_indicator_cuda_direct_with_inputs`: `1304-1369`
- module tests:
  - `1401-2040`

This file does not own:

- descriptor lookup and parameter/output validation rules:
  - `crates/app_core/src/indicator_catalog.rs:326-443`
- higher-level dynamic strategy planning:
  - `crates/app_core/src/strategy_dynamic.rs`
- higher-level regime/strategy logic that sometimes uses the unchecked direct entrypoint:
  - `crates/app_core/src/asr.rs`
  - `crates/app_core/src/cdmc.rs`
  - `crates/app_core/src/double_ma.rs`

## Classification

- Public surface: yes
  - the module is publicly exported from app-core:
    - `crates/app_core/src/lib.rs:28`
- Hot path: moderate to high
  - used by dynamic strategy and several CUDA-backed strategy implementations
- CUDA/runtime-critical: high
  - owns prepared input cache, device dispatch, and fallback policy
- Strategy-local: no
  - shared adapter infrastructure for many strategy modules
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- validation and normalization dependency:
  - `crates/app_core/src/indicator_catalog.rs:326-443`
- dynamic strategy staging and lazy prepared-input cache:
  - `crates/app_core/src/strategy_dynamic.rs:1069-1262`
- composite validation prepared-input use:
  - `crates/app_core/src/composite_validate.rs:190-223`
- ASR unchecked direct CUDA usage:
  - `crates/app_core/src/asr.rs:579-648`
  - `881-902`
- CDMC unchecked direct CUDA usage:
  - `crates/app_core/src/cdmc.rs:389-420`
  - `735+`
- Double-MA regime/device-row chaining:
  - `crates/app_core/src/double_ma.rs:9698-9767`
- fallback-stat consumer:
  - `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs:398-404`

### Downstream verification impact if changed

- validated-path changes affect:
  - dynamic strategy compilation/runtime
  - composite validation GPU helpers
  - direct indicator parity expectations
- unchecked/direct-path changes affect:
  - ASR and CDMC CUDA sweeps
  - any strategy code that intentionally bypasses catalog validation for internal parameter families
- fallback-policy changes affect:
  - benchmark interpretation
  - profiling-bin route diagnostics
  - whether device regressions become visible errors

## Standards And Research Log

This packet was written after targeted web research focused on public API clarity, documentation for hidden ownership/safety assumptions, naming guidance for unchecked entrypoints, unsafe-review posture, test-organization guidance, and CUDA best-practices framing for fallback and route-observability behavior.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes a public adapter surface used by multiple backend subsystems
   - Standard informed:
     - public helpers should make validation and fallback semantics explicit

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - prepared-input ownership and matrix-row keepalive rules are not obvious from signatures alone
   - Standard informed:
     - public contract surfaces should document hidden preconditions and ownership expectations

3. Rust API Guidelines, Naming
   - Source: <https://rust-lang.github.io/api-guidelines/naming.html>
   - Why it was consulted:
     - the unchecked direct CUDA entrypoint is public on a public module
   - Standard informed:
     - names should signal when a function bypasses validation or invariants

4. Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why it was consulted:
     - this module contains unsafe-adjacent device-slice row projection and keepalive-sensitive ownership
   - Standard informed:
     - bounds and lifetime assumptions next to unsafe conversions must stay explicit and reviewable

5. Rust Book, Test Organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why it was consulted:
     - the validated path is tested, but the negative/fallback contracts are not
   - Standard informed:
     - public boundary errors and fallback semantics need focused tests, not just parity smoke tests

6. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - silent fallback from device-path execution to host-backed CUDA affects route fidelity and benchmark interpretation
   - Standard informed:
     - fallback behavior should be observable enough that performance regressions are not mistaken for successful device-path runs

## Findings

## Correctness And Safety

### 1. Device-path compute failures are silently converted into host-backed CUDA success for most callers

- Evidence:
  - validated prepared-input CUDA path:
    - `crates/app_core/src/dynamic_indicator_adapter.rs:1220-1301`
  - it attempts device dispatch and then falls back on any error returned by:
    - `cuda_device_data_ref_for_input_kind(...).and_then(...)`: `1252-1266`
  - on error it records a fallback and reruns host-backed CUDA:
    - `1271-1285`
  - unchecked direct path is even looser:
    - `1304-1353`
  - the unchecked path swallows the device-path error and falls back without recording the fallback counters:
    - `1325-1353`
  - fallback counters are only consumed by the profiling bin:
    - `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs:398-404`
- Why it matters:
  - route regressions can turn into “successful but slower” runs instead of visible errors
  - benchmark or profiling conclusions can be contaminated if normal call sites do not know the device path failed
- Smallest justified fix:
  - safe-now:
    - expose fallback observability to ordinary callers, not only the profiling bin
  - semantics-sensitive:
    - narrow fallback so only expected compatibility/preparation failures reroute to host-backed CUDA
    - let genuine `ComputeFailure` surface as an error
- Review judgment:
  - medium-severity correctness/debuggability issue
  - benchmark-sensitive and semantics-sensitive

### 2. No memory-safety bug was proven in `device_slice_ref_from_matrix_row`, but its keepalive contract should be explicit

- Evidence:
  - row bounds and byte-offset checks happen before the unsafe conversion:
    - `crates/app_core/src/dynamic_indicator_adapter.rs:1119-1149`
  - in-repo consumers keep the owning matrices alive in scope:
    - `crates/app_core/src/double_ma.rs:9730`
    - `9767`
- Why it matters:
  - this helper is only safe if the owning `DeviceMatrixF32` outlives the returned slice ref
  - the current code appears to honor that, but the contract is mostly carried by convention and nearby implementation knowledge
- Smallest justified fix:
  - document the keepalive expectation at the helper boundary
- Review judgment:
  - docs/safety-clarity issue, not a proven bug

## API And Naming

### 1. `compute_dynamic_indicator_cuda_direct_with_inputs` is public but intentionally bypasses the validated adapter contract

- Evidence:
  - module is public:
    - `crates/app_core/src/lib.rs:28`
  - the direct entrypoint is public:
    - `crates/app_core/src/dynamic_indicator_adapter.rs:1304-1369`
  - unlike the validated path, it does not:
    - consult `find_dynamic_indicator_descriptor`
    - validate params against the catalog
    - validate output refs
  - it only trims the indicator ID and injects `first_valid`:
    - `1315-1324`
  - internal strategy code uses it for legitimate unchecked parameter families:
    - `crates/app_core/src/asr.rs:640-648`
    - `crates/app_core/src/cdmc.rs:402-420`
- Why it matters:
  - the function is useful internally, but not public-review ready as a generic public API
  - its name does not clearly signal “unchecked” or “bypasses catalog validation”
- Smallest justified fix:
  - if this surface is not meant as external stable API:
    - reduce it to `pub(crate)` or rename it to something explicitly unchecked/internal
  - otherwise:
    - document the bypassed validation steps and intended internal-only use cases
- Review judgment:
  - medium-severity API-surface issue
  - semantics-sensitive for external/public consumers

## Error Handling

### 1. The module has a typed error model, but many cross-file boundaries immediately erase it into `String`

- Evidence:
  - typed error owner:
    - `crates/app_core/src/dynamic_indicator_adapter.rs:37-97`
  - downstream erasure examples:
    - `crates/app_core/src/strategy_dynamic.rs:1080-1087`
    - `1244-1262`
    - `crates/app_core/src/composite_validate.rs:197-223`
    - `crates/app_core/src/asr.rs:584-592`
- Why it matters:
  - higher layers lose the distinction between:
    - unsupported indicator
    - parameter validation
    - output validation
    - CUDA preparation
    - compute failure
- Smallest justified fix:
  - keep `String` at distant UI/report boundaries if desired
  - but preserve typed branching at internal strategy/runtime call sites where fallback decisions depend on the actual failure class
- Review judgment:
  - low-to-medium error-surface debt

## Module Boundaries And Technical Debt

### 1. One public module currently owns too many policies at once

- Evidence:
  - public error/types: `37-125`
  - CPU adapter path: `183+`
  - prepared CUDA cache: `390+`, `587-763`
  - device-row chaining: `1119-1149`
  - validated CUDA path: `1220-1301`
  - unchecked direct path: `1304-1369`
- Why it matters:
  - the module is functional, but it is carrying:
    - validation-adapter policy
    - cache ownership
    - fallback policy
    - stats
    - device-slice chaining
  - that makes it harder to reason about which part of the API is truly “safe and public” versus internal infrastructure
- Smallest justified fix:
  - safe-now:
    - document the ownership split
  - defer-later:
    - consider separating validated API from unchecked/internal CUDA helpers

## Docs And Examples

### 1. The public surfaces need rustdoc on ownership, validation, and keepalive expectations

- Evidence:
  - key public surfaces include:
    - `DynamicIndicatorAdapterError`: `37`
    - `DynamicIndicatorComputeOutput`: `100`
    - `PreparedCudaIndicatorInputs`: `587`
    - `compute_dynamic_indicator_cuda_with_inputs`: `1220`
    - `compute_dynamic_indicator_cuda_direct_with_inputs`: `1304`
- Why it matters:
  - the difference between “validated path” and “unchecked direct path” is not obvious enough from the current public signatures
  - the prepared-input lifetime and matrix-row keepalive contract should not be left implicit
- Smallest justified fix:
  - add rustdoc describing:
    - validated versus unchecked path
    - prepared-input ownership model
    - device-row keepalive expectation
    - fallback behavior

## Tests And Missing Verification

### 1. Coverage is strong on happy-path parity and selective uploads, but negative-path public contracts are under-tested

- Evidence:
  - targeted CPU-path run succeeded:
    - `cargo test -p ta_app_core cpu_adapter_computes_single_output_ma_like_indicator -- --nocapture`
    - result: `1` test passed
  - existing positive-path tests include:
    - selective upload checks: `crates/app_core/src/dynamic_indicator_adapter.rs:1554-1680`
    - CUDA parity tests: `1776+`
    - decycler/mama prepared-input support: `2027+`
  - I did not find in-file negative tests for:
    - unsupported indicator IDs
    - invalid params/output refs
    - device-path fallback accounting
    - row-out-of-range handling for `device_slice_ref_from_matrix_row`
- Smallest justified fix:
  - add those focused negative tests

## Performance, Allocation, And CUDA Concerns

### 1. The selective-upload prepared-input design is a real strength and should not be disturbed casually

- Evidence:
  - required fields are derived from input kind and source:
    - `crates/app_core/src/dynamic_indicator_adapter.rs:271+`
    - `551+`
    - `628+`
  - tests pin selective upload behavior:
    - `1554-1680`
- Review judgment:
  - keep this design
  - it is a positive performance characteristic

### 2. The prepared-input cache is throughput-oriented and benchmark-sensitive

- Evidence:
  - per-thread/per-device cache:
    - `390-407`
  - derived-source overrides accumulate until dataset signature changes:
    - `720-736`
- Why it matters:
  - changing cache ownership or invalidation can affect CUDA throughput materially
- Review judgment:
  - benchmark-sensitive
  - do not refactor cache structure casually

### 3. The adapter does not borrow existing VRAM-resident buffers, which is a design tradeoff rather than a safe-now defect

- Evidence:
  - prepared inputs own a `CudaRuntime` and upload from `Candles`:
    - `596`
    - `668-739`
  - strategy code sometimes bypasses this when it already has richer GPU context:
    - `crates/app_core/src/double_ma.rs:9705-9729`
- Review judgment:
  - interesting future optimization topic
  - not a small safe-now patch

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc to public types and entrypoints
  - add focused negative tests
  - remove ambiguity around keepalive and fallback semantics
- Coordination required:
  - public/unchecked entrypoint visibility or renaming decisions
  - exposing fallback observability beyond the profiling bin
- Defer until later pass:
  - splitting the module into validated API, unchecked CUDA helpers, and prepared-input cache layers

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing docs
  - missing negative tests
- Coordination required:
  - typed-error preservation across internal layers
  - visibility/naming of the unchecked direct entrypoint
- Useful debt that should not be “cleaned up” casually:
  - selective prepared-input uploads
  - prepared cache ownership model

## Performance Opportunity Register

- Safe structural improvements:
  - better fallback observability so route regressions are measurable
- Benchmark-sensitive candidates:
  - narrowing fallback from device-path failure to host-backed CUDA
  - cache redesign
  - borrowing existing VRAM buffers
- Not the right place to chase performance:
  - broad cache or runtime redesign without profiling evidence

## Observability And Diagnostics Gaps

- normal call sites do not get strong visibility into device-path fallback
- unchecked direct path does not record fallback stats the way the validated path does
- docs do not explain keepalive expectations for matrix-row slicing

## Implementation Staging Guidance

- Safe now:
  - docs
  - negative tests
- Coordination required:
  - expose fallback state more broadly
  - decide public vs internal status of the unchecked direct entrypoint
- Defer until later pass:
  - module decomposition or VRAM-borrowing redesign

## Change Blast Radius

- Local only:
  - docs and tests
- Same subsystem:
  - indicator adapter call sites
  - profiling-bin fallback stats
- Cross-backend/runtime:
  - dynamic strategy staging
  - composite validation GPU helpers
  - ASR/CDMC/Double-MA CUDA indicator flows

## Open Questions Or Assumptions

- assumption:
  - `compute_dynamic_indicator_cuda_direct_with_inputs` is primarily intended for internal advanced callers, not for general external consumption
- open question:
  - whether the repo wants device-path compute failures to remain soft-fallback behavior or to become visible hard errors outside explicitly compatibility-driven cases

## Recommended Action

`Small surgical refactor`

The validated adapter path is good enough to build on. The right next pass is:

- document the public contract
- add negative tests
- make the unchecked entrypoint obviously unchecked/internal
- improve fallback observability before attempting any larger structural changes

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core cpu_adapter_computes_single_output_ma_like_indicator -- --nocapture`
- if CUDA is available and the relevant features are enabled, rerun representative prepared-input tests such as:
  - `cuda_adapter_parity_matches_cpu_for_single_output_indicator`
  - `selective_prepared_cuda_inputs_only_materialize_requested_close_slice_fields`
- add and run focused negative tests for:
  - unsupported indicator IDs
  - invalid params/output refs
  - `device_slice_ref_from_matrix_row` out-of-range rejection
  - fallback stat accounting
- if fallback policy changes, recheck the profiling-bin fallback counters in:
  - `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

## Regression Risk And Feature-Preservation Constraints

- do not break the validated CPU/CUDA adapter path
- do not remove the internal flexibility needed by ASR/CDMC-style unchecked CUDA sweeps unless a replacement is provided
- if fallback behavior is tightened, treat it as deliberate semantics work and verify that exposed errors are genuinely preferable to silent slower success
- preserve the selective-upload prepared-input behavior and current keepalive correctness
