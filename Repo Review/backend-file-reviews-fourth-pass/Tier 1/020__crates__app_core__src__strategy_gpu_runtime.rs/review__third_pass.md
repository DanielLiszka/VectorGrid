# Third-Pass Review: `crates/app_core/src/strategy_gpu_runtime.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_gpu_runtime.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/strategy_gpu_runtime.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_gpu_runtime.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the GPU strategy executable cache owner still looked like the right framing under fresh research. The third pass kept the bucket but sharpened the operational truthfulness problem: reset and stats helpers can make poisoned cache state look clean even though later execution treats poisoned locks as hard errors.

## Third-Pass Research Log

1. Rust Mutex docs
   - Source: <https://doc.rust-lang.org/std/sync/struct.Mutex.html>
   - Why consulted in third pass:
     - recheck poisoned-lock behavior in the cache owner
   - Third-pass constraint informed:
     - cache diagnostics should stay honest about poisoned state instead of collapsing it to an empty or clean view

2. Rust OnceLock docs
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in third pass:
     - ground the review of global cache lifetime and reset semantics
   - Third-pass constraint informed:
     - process-lifetime cache helpers should not overstate the effect of a local reset path

3. Rust thread_local docs
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in third pass:
     - reconfirm the scope of the upload cache being reported and cleared
   - Third-pass constraint informed:
     - thread-local upload state should not be summarized as if it were fully global runtime truth

## What Third Pass Sharpened

### 1. Reset and stats helpers can hide poisoned cache state
- Evidence:
  - reset_gpu_strategy_executable_cache clears counters even when poisoned caches could not actually be cleared.
  - gpu_strategy_executable_cache_stats collapses poisoned cache state to 0 entries via lock().ok().
- Why this matters:
  - Ops and debug tooling can report a clean-looking reset or empty cache while later build paths still hard-fail on poisoned locks.
- Practical implication:
  - Keep the existing bucket, but make diagnostic truthfulness the first safe-now hardening target.

### 2. Cache scope is narrower than the current reporting can imply
- Why this matters:
  - This file mixes process-global caches, per-thread upload caches, and reset helpers in a way that can read more authoritative than it is.
- Practical implication:
  - If changed, keep cache-scope reporting explicit instead of collapsing everything into one global cache story.

## Implementation Priority

- First hardening step: report poisoned or uncleared cache state honestly rather than treating it as empty.
- Second hardening step: tighten stats and reset wording or APIs so thread-local versus global scope stays visible.

## Verification If Changed

- cargo test -p ta_app_core strategy_gpu_runtime -- --nocapture
- representative cache reset and stat tests that exercise poisoned and thread-local cache paths

## Regression And Feature-Preservation Constraints

- Preserve current executable-cache keys and upload behavior unless the paired runtime path changes with it.
- Do not report poisoned cache state as a successful empty reset.
- Keep thread-local upload cache scope explicit if new diagnostics are added.

## Third-Pass Conclusion

`Bucket unchanged, but diagnostic truthfulness sharpened`

strategy_gpu_runtime.rs stays in Small surgical refactor. The third pass mainly sharpens an operational truthfulness bug: reset and stats helpers can make poisoned cache state look clean even though the execution path later treats poisoned locks as hard failures.
