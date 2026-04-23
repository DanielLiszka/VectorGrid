# Review Packet: `crates/app_core/src/standalone_gpu_runtime.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/standalone_gpu_runtime.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful CUDA support module, but not fully review-ready because it contains unsafe allocation assumptions, stream-ordered async free behavior, and host/device reuse contracts that are only implicit`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Internal-surface`, `CUDA-runtime-critical`, `Unsafe`, `Shared-support`, `Benchmark-sensitive`

## What This File Owns

`standalone_gpu_runtime.rs` owns small shared CUDA-runtime helpers reused by multiple strategy-local GPU adapters.

It owns:

- close-series identity key:
  - `CloseSeriesKey`: `9-24`
- resident close/log-return cache payload:
  - `ResidentCloseReturns`: `26-31`
- upload-signature helper:
  - `UploadSignature`: `34-37`
  - `slice_signature`: `45-52`
- integer range guard:
  - `checked_i32`: `40-43`
- device-buffer sizing and prefix-copy helpers:
  - `ensure_device_buffer_len`: `54-70`
  - `copy_host_to_device_prefix`: `73-79`
  - `copy_device_to_host_prefix`: `81-87`
- host scratch sizing:
  - `ensure_host_f32_len`: `89-94`

It does not own:

- strategy-specific runtime caches
- kernel launch behavior
- high-level GPU workflow orchestration

## Classification

- Public surface: no
  - all exported items are `pub(crate)`
- Hot path: indirectly yes
  - reused by GPU runtime adapters on evaluation paths
- CUDA/runtime-critical: yes
- Strategy-local: no
  - shared support module
- Low-risk support code: no
  - small code, but unsafe allocation assumptions make it high-sensitivity

## Direct Interactions

### Backend interaction map

- ROCB GPU adapter:
  - `crates/app_core/src/rocb_gpu_kernel.rs:11-13`
- sibling GPU strategy adapters:
  - `crates/app_core/src/vcc_gpu_kernel.rs`
  - `crates/app_core/src/vcet_gpu_kernel.rs`
  - `crates/app_core/src/vemr_gpu_kernel.rs`
- allocation helpers:
  - `crates/app_core/src/alloc_policy.rs`

### Downstream verification impact if changed

- changing buffer-sizing or async-drop behavior affects multiple CUDA strategy adapters
- changing `CloseSeriesKey` semantics affects cache reuse and refresh behavior
- changing `ensure_host_f32_len` affects host scratch correctness and allocation cost

## Standards And Research Log

This packet was written after targeted web research focused on `DeviceBuffer` safety requirements and the constraints around device-copyable types.

### Sources consulted

1. `cust::memory::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why it was consulted:
     - the module uses `DeviceBuffer::uninitialized` and async destruction
   - Standard informed:
     - uninitialized device memory is only safe when fully written before any read

2. `cust::memory::DeviceCopy`
   - Source: <https://docs.rs/cust/latest/cust/memory/trait.DeviceCopy.html>
   - Why it was consulted:
     - helper functions are generic over `T: DeviceCopy`
   - Standard informed:
     - generic GPU buffer helpers should stay limited to trivially device-copyable types

3. NVIDIA CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this module exists to preserve low-overhead buffer reuse for strategy-local GPU adapters
   - Standard informed:
     - allocation and transfer reuse should be preferred over repeated churn on hot GPU paths

## Findings

## Correctness And Safety

### 1. `ensure_device_buffer_len` depends on an important unsafe invariant that is not documented locally

- Evidence:
  - new device buffers are allocated with `unsafe { DeviceBuffer::<T>::uninitialized(required) }`: `60-63`
- Why this matters:
  - the code is reasonable because callers are expected to write into the full used prefix before reading
  - however, the safety assumption is nowhere stated in this file
- Smallest justified fix:
  - add a safety comment documenting that this helper is only correct for call sites that fully initialize the reused prefix before any read

### 2. `CloseSeriesKey` uses pointer identity, which is fast but contract-sensitive

- Evidence:
  - key fields are `len`, `host_ptr`, and `device_ptr`: `10-13`
  - `from_host_and_device` derives identity directly from the current slice pointer and optional device pointer: `16-23`
- Why this matters:
  - this is a pragmatic cache key, but it assumes pointer identity is the right proxy for content identity in the runtime adapters
  - future maintainers should understand that this is an optimization contract, not a semantic equality check
- Smallest justified fix:
  - add a comment explaining that the key is intentionally identity-based for cache invalidation speed

## API And Naming

### 3. `UploadSignature` is cheap and useful, but the hash policy is opaque

- Evidence:
  - `slice_signature` hashes values with `DefaultHasher`: `45-52`
- Why this matters:
  - collision risk is low enough for a cache-skipping hint, but the helper name alone does not explain that it is a best-effort signature, not a canonical digest
- Smallest justified fix:
  - document that `UploadSignature` is only a reuse heuristic

## Error Handling

### 4. Error transport is appropriately simple, but async-free behavior should be stated

- Evidence:
  - old device buffers are released with `drop_async(stream)`: `58-61`
- Why this matters:
  - this is the right low-overhead choice, but correctness depends on stream ordering and caller discipline
- Smallest justified fix:
  - add a short comment documenting that callers rely on stream ordering rather than immediate destruction

## Module Boundaries And Technical Debt

### 5. This file is a good shared-support seam and should stay narrow

- Evidence:
  - the helpers are generic and reusable, but intentionally limited to allocation, copying, and cache keys
- Why this matters:
  - it would be easy to turn this into a generic GPU runtime dumping ground
  - that would likely hide the invariants that strategy-local adapters need to keep visible
- Recommended posture:
  - keep the module small and utility-shaped

## Docs And Examples

### 6. The module needs safety comments more than structural changes

- Evidence:
  - there is no rustdoc or inline explanation despite unsafe allocation and async destruction
- Smallest justified fix:
  - add concise comments on `ensure_device_buffer_len`, `CloseSeriesKey`, and `UploadSignature`

## Tests And Missing Verification

### 7. There are no file-local tests for a shared CUDA helper

- Evidence:
  - no test module exists in this file
- Why this matters:
  - this is acceptable for a narrow support layer, but it means strategy-local GPU tests are implicitly carrying the contract
- Smallest justified fix:
  - only add direct tests if the helper logic changes
  - otherwise rely on higher-level CUDA adapter tests

## Performance, Allocation, And CUDA Concerns

### 8. The current allocation policy is probably the main reason this file exists and should not be abstracted away

- Evidence:
  - conditional reallocation avoids repeated alloc/free churn: `54-70`
  - host scratch growth reuses `alloc_policy` helpers: `89-94`
- Why this matters:
  - any refactor that replaces this with always-fresh allocations is likely a throughput regression
- Recommended posture:
  - benchmark-sensitive; keep the current explicit reuse policy

## Code-Quality Improvement Register

- Safe now:
  - add safety comments
  - document identity-based caching and best-effort upload signatures
- Defer:
  - any broader runtime abstraction or trait-heavy wrapper

## Technical-Debt Reduction Register

- Safe now:
  - local documentation hardening
- Defer:
  - deeper helper consolidation across all GPU adapters
- Do not casually remove:
  - async drop behavior
  - pointer-identity cache keys
  - allocation-reuse logic

## Performance Opportunity Register

- Safe structural work:
  - documentation only
- Benchmark-sensitive:
  - any change to allocation, free ordering, or cache-key policy

## Recommended Action

`Small surgical refactor`

The implementation is fine. The missing piece is explicit documentation of the unsafe and performance-sensitive invariants so this shared CUDA helper stays safe to reuse without becoming opaque.

## Exact Local Verification Required If Changed

1. ROCB GPU helper coverage:
   - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel rocb_gpu_kernel::tests:: -- --nocapture`
2. VCC GPU helper coverage:
   - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel vcc_gpu_kernel::tests:: -- --nocapture`
3. VCET GPU helper coverage:
   - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel vcet_gpu_kernel::tests:: -- --nocapture`
4. VEMR GPU helper coverage:
   - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel vemr_gpu_kernel::tests:: -- --nocapture`

### Verification run for this packet

- Not run in this pass:
  - no direct CUDA-featured verification was executed for this shared helper during the documentation-only review pass

## Regression Risk And Feature-Preservation Constraints

- Do not change allocation or async-free behavior without verifying all dependent GPU adapters
- Do not replace identity-based cache keys with content hashing casually; that trades correctness assumptions for throughput cost
- Do not broaden this helper into a generic runtime abstraction that hides stream-ordering and initialization invariants
