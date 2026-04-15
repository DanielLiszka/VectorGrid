# Third-Pass Review: `crates/app_core/src/cuda_device.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/cuda_device.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/cuda_device.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cuda_device.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the process-lifetime CUDA device selection owner still looked like the right framing under fresh research. The third pass sharpened the operational cache analysis because preferred-device results are sticky for process life, including early failures or stale snapshots.

## Third-Pass Research Log

1. Rust OnceLock docs
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in third pass:
     - reconfirm process-lifetime initialization semantics for the preferred-device caches
   - Third-pass constraint informed:
     - once-initialized device selection state can stay sticky until restart even if circumstances change

2. Rust Mutex docs
   - Source: <https://doc.rust-lang.org/std/sync/struct.Mutex.html>
   - Why consulted in third pass:
     - reassess how failures and cached metadata should be surfaced operationally
   - Third-pass constraint informed:
     - device-selection state should remain honest about sticky initialization and failure behavior

3. CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in third pass:
     - ground the device-selection discussion in a runtime-sensitive context
   - Third-pass constraint informed:
     - device-choice policy is operationally sensitive and should not pretend to be more dynamic than it is

## What Third Pass Sharpened

### 1. Preferred-device results are sticky for process life, including bad early state
- Evidence:
  - Both the preferred ID and preferred metadata are cached in separate OnceLock values.
- Why this matters:
  - A transient enumeration failure or early-device snapshot can remain authoritative until restart.
- Practical implication:
  - Keep the existing bucket, but describe the sticky cache semantics explicitly and add tests around early-failure behavior.

### 2. Operational truthfulness matters as much as heuristic choice
- Why this matters:
  - The sharper issue is not merely how the heuristic picks a device, but how long the choice persists once made.
- Practical implication:
  - If hardened, start with clearer reset and diagnostic behavior before revisiting the heuristic itself.

## Implementation Priority

- Make sticky preferred-device semantics explicit in diagnostics and tests.
- Only after that should heuristic device-choice policy be reconsidered.

## Verification If Changed

- cargo test -p ta_app_core cuda_device -- --nocapture
- representative tests for early failure, repeated lookup, and restart-required cache semantics

## Regression And Feature-Preservation Constraints

- Preserve current CPU-only fallback behavior.
- Do not imply live re-selection when the cache is process-lifetime sticky.
- Keep device-selection diagnostics honest if reset or retry behavior is added later.

## Third-Pass Conclusion

`Bucket unchanged, but sticky cache semantics sharpened`

cuda_device.rs stays in Small surgical refactor. The third pass mainly tightens the operational point that preferred-device state is sticky for process life, so transient early failures or stale snapshots can persist until restart.
