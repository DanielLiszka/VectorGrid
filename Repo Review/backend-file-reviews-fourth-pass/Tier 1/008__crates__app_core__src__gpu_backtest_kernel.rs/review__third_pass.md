# Third-Pass Review: `crates/app_core/src/gpu_backtest_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/gpu_backtest_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/gpu_backtest_kernel.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/gpu_backtest_kernel.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Third Pass

This file remained a high-ROI target because it is one of the hottest and most correctness-sensitive runtime owners in the repo. The third pass focused on whether there was a concrete small fix with better return than broad runtime cleanup.

## Third-Pass Research Log

1. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in third pass:
     - re-check whether oversized transfer/output paths are worth prioritizing ahead of broader runtime API cleanup
   - Third-pass constraint informed:
     - dense host/device transfers and oversized retained buffers are a real performance and residency concern

2. Rust `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in third pass:
     - reassess the public probe-runtime helper and scratch retention semantics
   - Third-pass constraint informed:
     - same-thread runtime ownership remains a sharp API surface if a raw handle escapes

3. Rust `MaybeUninit`
   - Source: <https://doc.rust-lang.org/std/mem/union.MaybeUninit.html>
   - Why consulted in third pass:
     - confirm that the existing low-level allocation patterns are still not the best immediate target
   - Third-pass constraint informed:
     - the more valuable third-pass fix is output-shape compaction, not re-litigating already-guarded uninitialized allocation usage

## What Third Pass Sharpened

### 1. The uploaded dynamic exact path still leaves compaction disabled and pays for dense output it does not need

- Evidence:
  - the uploaded exact path sizes output to the dense stride product
  - compaction is disabled on that path
  - dense device output is synchronized and copied back before only the valid pair rows are decoded
  - retained scratch is keyed by `(device_id, out_len)`, so oversized allocations persist in thread-local storage
- Why this matters:
  - this is a concrete performance and VRAM-retention issue with a narrower blast radius than general runtime reuse redesign
- Practical implication:
  - this is the highest-ROI small fix in the file:
    - compact the uploaded dynamic exact path first
    - perf-gate it before touching anything broader

### 2. `prepare_cuda_kernel_runtime_for_probe()` is still sharper than it needs to be, but it is a secondary target

- Evidence:
  - the helper still returns a raw `usize` handle from a thread-local runtime map
  - live use remains internal rather than broad public consumption
- Why this matters:
  - visibility and type opacity can be tightened without disturbing core runtime reuse
- Practical implication:
  - narrow visibility or replace the raw `usize` with a crate-local opaque handle after the uploaded-path compaction fix

## Implementation Priority

- Highest-ROI hardening:
  - compact the uploaded dynamic exact path
- Secondary cleanup:
  - narrow the probe-runtime helper surface

## Verification If Changed

- rerun existing CUDA exact-kernel, uploaded-batch, and staged-market tests
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

Also verify:

- exact metric decode ordering remains unchanged
- pair-map semantics remain unchanged
- retained scratch does not grow unnecessarily after uploaded exact runs

## Regression And Feature-Preservation Constraints

- Preserve runtime reuse, stream topology, and launch heuristics unless measurements justify change
- Preserve exact metric decode ordering and uploaded-batch replay parity
- Do not weaken diagnostics or fallback observability while tightening the probe helper

## Third-Pass Conclusion

`Bucket unchanged, but the next small fix is now concrete`

`gpu_backtest_kernel.rs` remains a `Small surgical refactor` file, and the third pass identifies the best target more precisely: the uploaded dynamic exact path should stop paying for dense output and retained VRAM it does not actually need.
