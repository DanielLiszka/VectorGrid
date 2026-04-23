# Second-Pass Review: `crates/app_core/src/composite_profile_route.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_profile_route.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/composite_profile_route.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Document only`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as small policy logic with minor cleanup opportunities around labels and profile routing.

## Stale Check

The live source still functions as:

- static product routing policy for supported composite families
- UI-facing label and summary helper surface

The second pass found less code risk here than the first pass assumed.

## Second-Pass Research Log

1. NVIDIA multi-GPU guide
   - Source: <https://docs.nvidia.com/cuda/cuda-programming-guide/03-advanced/multi-gpu-systems.html>
   - Why consulted in second pass:
     - separate static target labeling from actual runtime capability and device selection
   - Additional constraint informed:
     - these helpers are policy labels, not runtime capability probes

2. CUDA Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-programming-guide/index.html>
   - Why consulted in second pass:
     - reinforce the distinction between static profile naming and real device/context management
   - Additional constraint informed:
     - accidental label churn is higher risk than missed execution logic in this file

## Delta vs First Pass

### 1. This file reads more like product policy and documentation surface than refactor-needing runtime logic

- Why this matters:
  - the main practical risk is misleading or drifting labels, not broken execution
- Practical implication:
  - the bucket should move down to `Document only`

## Verification If Changed

- rerun existing precedence and label tests
- add one assertion that runtime labels remain policy labels rather than capability probes

## Regression And Feature-Preservation Constraints

- Preserve strategy-to-profile mapping precedence for `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`
- Preserve current UI-facing label strings unless the frontend moves in lockstep
- Do not reinterpret these helpers as runtime capability detection

## Second-Pass Conclusion

`First-pass bucket changed`

The second pass moves this file from `Small surgical refactor` to `Document only`. The core risk here is label and policy drift, not missing execution logic, so documentation clarity is a better fit than structural cleanup.
