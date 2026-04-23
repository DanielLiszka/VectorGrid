# Second-Pass Review: `crates/app_core/src/mkcs_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/mkcs_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/mkcs_gpu_kernel.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a safe-looking wrapper over raw device-pointer and layout contracts, with explicit rejection of unsupported overlays and exits as a must-preserve behavior.

## Stale Check

The live source still owns:

- raw pointer and device-affinity assumptions
- resident-series reuse and upload dedup
- explicit rejection of unsupported overlay/SL/TP features

The first-pass packet remains current.

## Second-Pass Research Log

1. The Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why consulted in second pass:
     - re-check unsafe contract expectations
   - Additional constraint informed:
     - the public API still hides a sharp raw-pointer and device-layout contract

2. CUDA Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why consulted in second pass:
     - ground pointer/device execution assumptions
   - Additional constraint informed:
     - device-affinity still deserves clearer documentation if the API remains public

3. `cust` docs
   - Source: <https://docs.rs/cust/latest/cust/>
   - Why consulted in second pass:
     - revisit buffer and stream ownership conventions
   - Additional constraint informed:
     - the current wrapper still depends on caller-supplied ownership discipline

## Delta vs First Pass

### 1. Host-upload fallback when `close_device_ptr == 0` remains part of the public behavior, not an incidental implementation detail

### 2. Explicit rejection of unsupported overlays and exits remains exactly the right kind of honesty for this wrapper

- Practical implication:
  - preserve these explicit failures rather than silently degrading behavior

## Verification If Changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel mkcs_gpu_kernel::tests:: -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel mkcs_validation_gpu_kernel_matches_cpu_metrics -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve resident-series reuse and upload dedup behavior
- Preserve explicit rejection of unsupported overlays and exits
- Do not widen the raw-pointer contract without documentation and parity coverage

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that host-upload fallback and explicit unsupported-feature rejection are both active contract behavior worth preserving.
