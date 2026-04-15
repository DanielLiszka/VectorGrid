# Second-Pass Review: `crates/app_core/src/options_expression_gpu.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_expression_gpu.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/options_expression_gpu.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a hot-path CUDA runtime owner where context policy, cache reuse, and buffer ownership matter more than small structural cleanup.

## Stale Check

The live source still owns:

- per-device runtime and module setup
- device-resident shared/outcome caches
- reusable output buffers and chunk planning

The first-pass packet remains current.

## Second-Pass Research Log

1. CUDA C++ Programming Guide
   - Source: <https://docs.nvidia.com/cuda/archive/12.4.0/pdf/CUDA_C_Programming_Guide.pdf>
   - Why consulted in second pass:
     - verify context concurrency limits and overlap requirements
   - Additional constraint informed:
     - kernels from different CUDA contexts cannot execute concurrently, so separate-context design is real performance debt, not only architectural untidiness

2. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/archive/11.6.2/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - reassess multiple-context cost and reuse rationale
   - Additional constraint informed:
     - current device-resident reuse remains worth preserving unless profiling proves otherwise

3. `cust::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why consulted in second pass:
     - re-check uninitialized device-allocation assumptions
   - Additional constraint informed:
     - current reuse and output handling still depend on fully overwritten buffers before decode

## Delta vs First Pass

### 1. Separate CUDA contexts now look more consequential than the first pass stated

- Why this matters:
  - options-expression and sibling GPU runtimes still carry real cross-runtime performance debt
- Practical implication:
  - preserve the current defer posture around deeper context ownership changes

### 2. Immediate sync/copy is more understandable in light of overlap requirements

- Why this matters:
  - the current path uses ordinary host `Vec`s, not page-locked buffers needed for richer overlap
- Practical implication:
  - keep correctness-first serialization unless profiling justifies a bigger runtime redesign

## Verification If Changed

- `cargo test -p ta_app_core options_expression_gpu::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core options_expression::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
- rerun the options-expression Nsight workflow if launch/copy/context behavior changes

## Regression And Feature-Preservation Constraints

- Preserve current device-resident shared/outcome cache reuse
- Preserve chunk-budget safeguards and correctness-first sync behavior unless profiling proves otherwise
- Do not change context ownership across GPU subsystems without coordinated measurement

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that the separate-context design is real cross-runtime performance debt, while the current immediate sync/copy posture still has a defensible correctness-first rationale.
