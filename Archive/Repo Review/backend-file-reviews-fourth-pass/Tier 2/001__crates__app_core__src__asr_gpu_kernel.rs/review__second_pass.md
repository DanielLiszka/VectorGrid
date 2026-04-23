# Second-Pass Review: `crates/app_core/src/asr_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/asr_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/asr_gpu_kernel.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a safe-looking API wrapped around raw device-pointer contracts and flagged row-pointer validity and cache assumptions as the main risks.

## Stale Check

The live source still owns:

- sparse-row compaction
- resident close/LR cache reuse
- raw device-pointer handoff into CUDA execution

The first-pass packet remains current.

## Second-Pass Research Log

1. `cust::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why consulted in second pass:
     - re-check device-buffer reuse and allocation assumptions
   - Additional constraint informed:
     - runtime reuse remains sound only if row/input invariants hold

2. `cust::Stream`
   - Source: <https://docs.rs/cust/latest/cust/stream/struct.Stream.html>
   - Why consulted in second pass:
     - verify the host-read safety boundary
   - Additional constraint informed:
     - the explicit stream sync remains the key decode boundary

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/archive/12.6.2/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - reassess cache/reuse posture and transfer avoidance
   - Additional constraint informed:
     - runtime reuse remains worth preserving if correctness contracts are made clearer

## Delta vs First Pass

### 1. Resident close/LR cache reuse still assumes immutability of the pointed-at series, not merely pointer validity

- Why this matters:
  - in-place mutation with the same identity can reuse stale returns
- Practical implication:
  - this remains a documentation and contract-hardening gap if the API stays public

### 2. Fast/slow row-pointer validity deserves the same explicit treatment already applied to Kaufman rows

- Practical implication:
  - still the highest-value small hardening if this wrapper is touched

## Verification If Changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel asr_gpu_kernel -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve sparse-row compaction and current runtime reuse
- Preserve packed-row upload skipping
- Do not retune compaction heuristics without GPU measurement

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that cache correctness depends on immutable input meaning, not only non-null pointers, and that fast/slow row validation is still the cleanest bounded hardening target.
