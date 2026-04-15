# Second-Pass Review: `crates/app_core/src/kernels/options_expression.cu`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kernels/options_expression.cu`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/kernels/options_expression.cu.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as compact but semantics-sensitive because premium handling and robust-score blending are product behavior, not just CUDA mechanics.

## Stale Check

The live source still owns:

- the per-split tuple aggregation kernel
- the combined search/holdout/stress kernel
- the robust-score blend used for options ranking

The first-pass packet remains current.

## Second-Pass Research Log

1. NVIDIA CUDA C Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why consulted in second pass:
     - ground the kernel-contract and Rust launch-coupling review
   - Additional constraint informed:
     - handwritten kernels this small should keep their assumptions explicit because debugging surface is narrow

2. NVIDIA CUDA C Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - reassess whether there is any safe reason to broaden or merge the current work partitioning
   - Additional constraint informed:
     - existing partitioning should stay unless measurement says otherwise

## Delta vs First Pass

### 1. The two-kernel split remains product contract, not just implementation style

- Why this matters:
  - the Rust runtime, ranking semantics, and exactification timing all already assume the current split-scoring and combined-scoring surfaces
- Practical implication:
  - broad kernel unification would be higher risk than the file’s size suggests

### 2. Premium-divide and robust-score weights remain policy choices that should stay explicit

- Why this matters:
  - they continue to encode ranking semantics, not mere numeric plumbing

## Verification If Changed

- rerun the CUDA options-expression tests in `options_expression_gpu.rs` and `options_expression.rs`
- rerun the options-expression Nsight workflow if launch shape or kernel semantics change

## Regression And Feature-Preservation Constraints

- Preserve kernel names and argument order expected by `options_expression_gpu.rs`
- Preserve current premium handling and robust-score semantics unless intentionally re-baselined
- Keep work partitioning stable unless measurement justifies a deeper change

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that the current two-kernel split is itself part of the product/runtime contract, not just an implementation convenience.
