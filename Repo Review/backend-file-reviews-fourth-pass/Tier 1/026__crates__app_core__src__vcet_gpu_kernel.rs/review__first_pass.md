# Review Packet: `crates/app_core/src/vcet_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vcet_gpu_kernel.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful CUDA companion module, but not fully review-ready because owned-device lifetime rules, scratch-cache reuse, and kernel-launch assumptions are mostly implicit`
- Recommended action bucket: `Benchmark-sensitive; do not touch yet`
- Sensitivity tags: `CUDA-runtime-critical`, `Benchmark-sensitive`, `Unsafe-adjacent`, `Semantics-sensitive`

## What This File Owns

`vcet_gpu_kernel.rs` owns the CUDA-side VCET batch preparation and metric evaluation path:

- device signal-table ownership and pointer table shaping: `36-142`
- prepared batch construction: `144-361`
- scratch/runtime cache types: `363-462`
- input validation and kernel launch path: `464-939`
- file-local validation tests: `940-997`

## Classification

- Public surface: yes
  - exported from `crates/app_core/src/lib.rs`
- Hot path: yes
- CUDA/runtime-critical: yes
- Strategy-local: yes
  - VCET-only GPU companion

## Direct Interactions

### Backend interaction map

- main CPU/GPU orchestration consumers:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_validate.rs`
- strategy-local staging:
  - `crates/app_core/src/vcet.rs`
- shared CUDA/runtime utilities:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
  - `crates/app_core/src/standalone_gpu_runtime.rs`

### Downstream verification impact if changed

- changing owned-row lifetime behavior can break resident device-pointer validity
- changing validation or launch assumptions can break VCET GPU parity or fallback routing
- changing scratch/runtime reuse can change throughput and memory residency characteristics

## Standards And Research Log

This packet was written after targeted web research focused on CUDA launch/storage assumptions and unsafe/uninitialized buffer handling.

### Sources consulted

1. NVIDIA CUDA C Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file owns reusable device buffers, resident tables, and launch-shape choices
   - Standard informed:
     - runtime-cache edits should be treated as measurement-sensitive

2. NVIDIA CUDA C Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why it was consulted:
     - the file launches kernels against pointer-heavy row-major inputs
   - Standard informed:
     - pointer/shape invariants should stay explicit and defensive

3. Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why it was consulted:
     - the file uses uninitialized device buffers and lifetime-sensitive ownership patterns
   - Standard informed:
     - safety assumptions should be written next to unsafe-adjacent allocation paths

## Findings

## Correctness And Safety

### 1. Owned signal-table lifetime protection is real behavior, but the contract is only implied

- Evidence:
  - keepalive owners are stored on `VcetGpuOwnedSignalTables`: `54-112`
  - `as_signal_matrices` keeps them live via a no-op binding: `114-142`
- Why this matters:
  - the row-pointer arrays are only valid because the owning device matrices remain alive
- Smallest justified fix:
  - document that `vidya_rows_keepalive`, `ce_long_rows_keepalive`, and `ce_short_rows_keepalive` are semantic ownership fields, not dead storage

### 2. Scratch-buffer reuse depends on uninitialized device allocations whose overwrite assumptions are not documented

- Evidence:
  - uninitialized device buffers are created and reused in the evaluation path: `573`, `611`, `886`
- Why this matters:
  - this is acceptable only if every subsequently read element is always overwritten before use
- Smallest justified fix:
  - add narrow safety comments beside the uninitialized allocation sites instead of changing allocation strategy

## Module Boundaries And Technical Debt

### 3. The file mixes host-side period-resolution staging, device-table ownership, runtime caching, and kernel execution

- Evidence:
  - batch preparation: `154-361`
  - runtime cache definitions: `363-462`
  - kernel execution: `631-939`
- Why this matters:
  - the module is still navigable, but the responsibility split is hidden and discourages safe localized maintenance
- Smallest justified fix:
  - document the ownership split now; defer any extraction until after parity and perf gates are ready

## Tests And Missing Verification

### 4. File-local tests validate shape/range guards only; they do not exercise live CUDA execution in this review pass

- Evidence:
  - file-local tests cover `validate_inputs`: `940-997`
  - CUDA orchestration/parity tests live in `composite_cpu.rs`, not here
- Why this matters:
  - changes that look local can still affect parity, shortlist behavior, or runtime reuse
- Smallest justified fix:
  - preserve the current split and require CUDA parity plus bench coverage for any non-doc change

## Performance And CUDA Concerns

### 5. Runtime-cache and launch-shape behavior are benchmark-sensitive and should not be “cleaned up” by taste

- Evidence:
  - kernel caps and thread-local runtime cache: `418-462`
  - launch path and decode loop: `631-939`
  - Criterion bench coverage exists under `crates/app_core/benches/vcet_opt.rs`
- Why this matters:
  - small structural edits here can shift allocation frequency, launch sizing, and host/device copy volume
- Smallest justified fix:
  - keep behavioral edits behind explicit CUDA perf verification

## Code-Quality Improvement Register

- Safe now:
  - document keepalive ownership semantics
  - document overwrite assumptions near uninitialized device buffers
  - document the batch-prep/runtime/eval split
- Coordination required:
  - extracting runtime cache helpers
  - changing validation or launch-shape logic
- Defer:
  - redesigning the module into generic GPU infrastructure

## Technical-Debt Reduction Register

- Safe now:
  - comments and rustdoc
- Defer:
  - structural splitting
  - cache-policy changes
- Do not casually remove:
  - keepalive storage fields
  - thread-local runtime cache
  - current defensive input validation

## Performance Opportunity Register

- Safe structural work:
  - comments only
- Benchmark-sensitive:
  - launch-shape tuning
  - scratch-buffer policy changes
  - table ownership and upload reuse changes

## Recommended Action

`Benchmark-sensitive; do not touch yet`

The practical value here is documenting the lifetime and allocation contracts. The runtime path is too performance-sensitive to refactor casually during a review-only pass.

## Exact Local Verification Required If Changed

1. File-local guard tests:
   - `cargo test -p ta_app_core vcet_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
2. Representative VCET parity/orchestration tests:
   - `cargo test -p ta_app_core composite_cpu::tests::vcet_gpu_kernel_request_runs_when_cuda_available --features cuda,cuda-backtest-kernel -- --nocapture`
   - `cargo test -p ta_app_core composite_cpu::tests::vcet_gpu_kernel_major_parity_vs_cpu --features cuda,cuda-backtest-kernel -- --nocapture`
3. Representative bench coverage:
   - `cargo bench -p ta_app_core --bench vcet_opt --features cuda,cuda-backtest-kernel`

### Verification run for this packet

- Not run in this review pass:
  - CUDA-gated `vcet_gpu_kernel` tests
  - CUDA parity tests and benches
- Related default-build verification already passed:
  - `cargo test -p ta_app_core vcet::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Preserve the keepalive-backed row-pointer ownership model unless the replacement is parity-tested
- Do not switch allocation strategy or cache lifetime policy without CUDA measurement
- Preserve current validation/fallback behavior so VCET GPU requests do not silently drift into incorrect or partially-evaluated routes
