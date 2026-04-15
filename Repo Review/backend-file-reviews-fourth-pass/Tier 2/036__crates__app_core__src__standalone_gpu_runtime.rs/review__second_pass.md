# Second-Pass Review: `crates/app_core/src/standalone_gpu_runtime.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/standalone_gpu_runtime.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/standalone_gpu_runtime.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this helper as small but contract-sensitive because async-free behavior, pointer-identity keys, and upload signatures are all deliberate runtime choices.

## Stale Check

The live source still:

- uses `drop_async(stream)` as part of its buffer lifecycle
- keys close-series caches by identity rather than content
- treats upload signatures as skip-upload hints

The first-pass packet remains current.

## Second-Pass Research Log

1. `cust::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why consulted in second pass:
     - verify `uninitialized` and `drop_async` behavior
   - Additional constraint informed:
     - stream ordering is still part of the safety contract

2. `cust::DeviceCopy`
   - Source: <https://docs.rs/cust/latest/cust/memory/trait.DeviceCopy.html>
   - Why consulted in second pass:
     - re-check generic buffer-helper constraints
   - Additional constraint informed:
     - the current helper layer remains carefully minimal for a reason

3. Rust `OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in second pass:
     - compare with sibling cache patterns using this helper layer
   - Additional constraint informed:
     - short-lived runtime caches here still contrast with broader process-lifetime caches elsewhere

## Delta vs First Pass

### 1. `drop_async(stream)` makes stream ordering an explicit part of the safety contract

- Why this matters:
  - hiding the stream would make the lifetime rules harder to reason about

### 2. `UploadSignature` should keep being treated as a skip-upload hint rather than a content-integrity token

- Practical implication:
  - do not strengthen its meaning casually

## Verification If Changed

- ROCB/VCC/VCET/VEMR CUDA adapter tests under `--features cuda,cuda-backtest-kernel`

## Regression And Feature-Preservation Constraints

- Keep allocation reuse and async-free behavior
- Keep pointer-identity cache keys
- Do not upgrade signatures into stronger hashing/content validation without measured need

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that stream ordering is part of the live safety contract and that upload signatures remain best understood as reuse hints, not integrity proofs.
