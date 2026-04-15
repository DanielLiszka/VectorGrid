# Review Packet: `crates/app_core/src/kernels/double_crossover.cu`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kernels/double_crossover.cu`
- Packet status: `Reviewed`
- Public-review readiness: `Important handwritten kernel unit, but not fully review-ready because Rust/CUDA ABI contracts, kernel family ownership, and performance-sensitive tuning assumptions are mostly implicit`
- Recommended action bucket: `Benchmark-sensitive; do not touch yet`
- Sensitivity tags: `CUDA-kernel-critical`, `ABI-sensitive`, `Benchmark-sensitive`, `Monolithic-kernel-unit`

## What This File Owns

`double_crossover.cu` is the main handwritten CUDA compilation unit for the app-core GPU path. It currently contains:

- shared ABI structs and strategy-program probe kernels near the top: `31-123`
- dynamic strategy and double-MA backtest kernels starting at `68`, `373`, and `703`
- a large family of strategy/profile kernels through the middle and lower sections: `857-11489`
- CUB-backed top-k selection kernels in the late section: `8963-10352`

This is a monolithic kernel translation unit, not a single-purpose file.

## Classification

- Public surface: indirectly
  - kernel names and ABI layout are consumed by Rust-side launch code
- Hot path: yes
- CUDA/runtime-critical: yes
- Strategy-local: no
  - shared kernel unit used by multiple strategy/runtime paths

## Direct Interactions

### Backend interaction map

- build/compilation surface:
  - `crates/app_core/build.rs`
- Rust launch/runtime consumers:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
  - multiple `*_gpu_kernel.rs` modules under `crates/app_core/src/`
- options-related coupling:
  - `crates/app_core/src/options_expression_gpu.rs`
  - `crates/app_core/src/kernels/options_expression.cu`

### Downstream verification impact if changed

- changing struct layout or kernel names can silently break Rust-side launches
- changing block sizing or shared-memory behavior can alter throughput, occupancy, or shortlist ranking
- changing math or guard behavior can affect parity across many strategy families at once

## Standards And Research Log

This packet was written after targeted web research focused on CUDA launch behavior, shared-memory sorting, and ABI-sensitive kernel maintenance.

### Sources consulted

1. NVIDIA CUDA C Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file owns a large share of the handwritten GPU hot path
   - Standard informed:
     - tuning-oriented edits must be benchmark-backed

2. NVIDIA CUDA C Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why it was consulted:
     - kernel launch contracts, memory access assumptions, and guard behavior live here
   - Standard informed:
     - ABI and launch-shape assumptions should stay explicit

3. `cub::BlockRadixSort` documentation
   - Source: <https://nvidia.github.io/cccl/cub/api/classcub_1_1BlockRadixSort.html>
   - Why it was consulted:
     - this file uses CUB block radix sort in the top-k kernels
   - Standard informed:
     - block/thread/shared-memory assumptions are part of the kernel contract, not decoration

## Findings

## Correctness And Safety

### 1. The Rust/CUDA ABI contract is handwritten and fragile, but the file does not explain which structs must stay layout-locked

- Evidence:
  - ABI structs such as `GpuValueOperandAbi`, `GpuConditionAbi`, `GpuConditionGroupAbi`, `GpuSignalBindingsAbi`, and `GpuIndicatorSlotAbi` are declared at `31-67`
  - Rust-side consumers launch matching kernels from `gpu_backtest_kernel.rs`
- Why this matters:
  - any field-order or type-width drift can become a runtime-only failure
- Smallest justified fix:
  - add comments stating that these structs are cross-language ABI types and must stay synchronized with Rust-side definitions

## Module Boundaries And Technical Debt

### 2. This file has become a monolithic kernel warehouse with a very large blast radius

- Evidence:
  - kernel entry points span from `68` through `11489`
  - the same CU contains dynamic strategy probing, options tuple scoring, strategy backtests, and top-k helpers
- Why this matters:
  - even “localized” edits increase the risk of rebuild, register-pressure, or shared include fallout across unrelated strategies
- Smallest justified fix:
  - document the major kernel families now; defer any compilation-unit split until the team explicitly budgets a CUDA refactor pass

## Performance And CUDA Concerns

### 3. The CUB-backed top-k region is explicitly performance-sensitive and should not be reformatted into a different algorithm without measurement

- Evidence:
  - repeated `cub::BlockRadixSort` use appears at `8963-10352`
  - these kernels allocate shared temp storage and fixed block/thread layouts
- Why this matters:
  - this is the kind of code where readability-driven “simplification” can degrade occupancy or shortlist throughput
- Smallest justified fix:
  - preserve current top-k structure unless benchmark and parity coverage are part of the task

## Tests And Missing Verification

### 4. This file has no kernel-local review harness; verification is spread across Rust launchers, parity tests, and perf gates

- Evidence:
  - the source file itself contains kernels only
  - compilation is wired through `crates/app_core/build.rs`
- Why this matters:
  - safe maintenance depends on testing the Rust launch paths and the strategy families that consume them
- Smallest justified fix:
  - keep recommended changes narrowly documentary unless the caller tests and perf gates are run

## Code-Quality Improvement Register

- Safe now:
  - document cross-language ABI structs
  - document kernel-family ownership at the top of the file
  - document that the top-k region is tuning-sensitive
- Coordination required:
  - splitting the translation unit
  - renaming kernels
  - changing block/thread/shared-memory policy
- Defer:
  - any broad kernel reorganization

## Technical-Debt Reduction Register

- Safe now:
  - comments and section headers
- Defer:
  - file splitting
  - algorithm swaps in top-k or backtest kernels
- Do not casually remove:
  - ABI structs
  - existing kernel entry names
  - CUB-backed top-k helpers

## Performance Opportunity Register

- Safe structural work:
  - comments only
- Benchmark-sensitive:
  - launch geometry
  - shared-memory layout
  - top-k algorithm changes
  - cross-kernel refactoring that might change inlining/register pressure

## Recommended Action

`Benchmark-sensitive; do not touch yet`

This file is too central and too broad to refactor safely during a documentation review pass. The best immediate improvement is to make the ABI and kernel-family boundaries explicit for future maintainers.

## Exact Local Verification Required If Changed

1. Rebuild the CUDA module:
   - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
2. Representative Rust-side launch coverage:
   - `cargo test -p ta_app_core gpu_backtest_kernel --features cuda,cuda-backtest-kernel -- --nocapture`
3. Strategy-family parity coverage for affected callers
4. CUDA performance verification:
   - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

### Verification run for this packet

- Not run in this review pass:
  - CUDA build/tests/perf gate for this translation unit

## Regression Risk And Feature-Preservation Constraints

- Preserve kernel names and ABI layouts unless Rust-side consumers are updated together
- Preserve current top-k and launch-shape behavior unless measurement shows a clear improvement
- Avoid mixed-purpose “cleanup” edits that couple unrelated strategy families in the same compilation unit
