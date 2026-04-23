# Second-Pass Review: `crates/app_core/src/cuda_device.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/cuda_device.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/cuda_device.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as small but policy-bearing because deterministic device selection and clean CPU-only fallback matter more than clever heuristics.

## Stale Check

The live source still uses:

- `OnceLock`-based memoization
- largest-total-memory selection policy
- CPU-clean behavior when no CUDA device exists

The first-pass packet remains current.

## Second-Pass Research Log

1. `OnceLock`
   - Source: <https://doc.rust-lang.org/nightly/std/sync/struct.OnceLock.html>
   - Why consulted in second pass:
     - confirm one-time initialization and failure behavior
   - Additional constraint informed:
     - cached ID and cached metadata can still diverge permanently after initialization failures

2. NVIDIA multi-GPU guide
   - Source: <https://docs.nvidia.com/cuda/cuda-programming-guide/03-advanced/multi-gpu-systems.html>
   - Why consulted in second pass:
     - compare the current heuristic to broader multi-GPU selection considerations
   - Additional constraint informed:
     - largest-memory selection is predictable product policy, not a universally optimal scheduler

## Delta vs First Pass

### 1. There are effectively two memoized truths here: chosen device ID and chosen-device metadata

- Why this matters:
  - partial initialization failure can leave them diverged
- Practical implication:
  - still a bounded hardening target if this file is revisited

### 2. The current largest-memory heuristic should stay documented as product policy

- Practical implication:
  - do not market it as an optimal scheduler without broader device-selection work

## Verification If Changed

- add stubbed multi-device tests covering tie cases
- add a divergence case where ID lookup succeeds but metadata lookup fails

## Regression And Feature-Preservation Constraints

- Preserve CPU-only behavior when no CUDA runtime/device exists
- Preserve deterministic selection behavior unless replaced by a richer documented policy
- Keep current heuristic readable and explicit

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that the memoized device ID and memoized metadata can diverge, and that the current largest-memory rule should be treated as explicit product policy.
