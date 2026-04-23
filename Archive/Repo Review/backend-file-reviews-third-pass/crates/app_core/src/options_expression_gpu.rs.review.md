# Third-Pass Review: `crates/app_core/src/options_expression_gpu.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_expression_gpu.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/options_expression_gpu.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_expression_gpu.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got An Expanded Third Pass

The second-pass packet focused on context debt and correctness-first sync behavior. Another pass was warranted because the stronger remaining issue turned out to be cache behavior, not broader CUDA architecture.

## Third-Pass Research Log

1. Rust `BTreeMap`
   - Source: <https://doc.rust-lang.org/std/collections/struct.BTreeMap.html>
   - Why consulted in third pass:
     - verify what the current runtime-cache eviction policy actually means
   - Third-pass constraint informed:
     - iteration order is key-sorted, not insertion- or recency-based

2. Rust `Mutex`
   - Source: <https://doc.rust-lang.org/std/sync/struct.Mutex.html>
   - Why consulted in third pass:
     - reassess whether panic-on-poison is still just cleanup debt or a stronger operational problem
   - Third-pass constraint informed:
     - poisoning is recoverable, so `expect("... poisoned")` is a deliberate escalation to repeated panic

## What Third Pass Sharpened

### 1. `evict_oldest_if_needed()` is not actually oldest eviction; it is lexical key eviction

- Evidence:
  - the runtime store still uses `BTreeMap`
  - eviction removes the first key returned by the map
- Why this matters:
  - cache hit behavior, VRAM reuse, and exported cache-usage stats are all shaped by a policy that is currently easier to misread than the name implies
- Practical implication:
  - the third-pass packet should emphasize cache-policy truthfulness over broader context redesign

### 2. A single panic can still poison the runtime store and turn later options-expression GPU work into repeat panics

- Evidence:
  - the file still locks global runtime state with `expect(...)`
  - runtime entries are reused process-wide rather than being purely local scratch
- Why this matters:
  - the operational blast radius is larger than the second-pass packet stated
- Practical implication:
  - poison recovery or runtime-store reset is now a higher-value bounded hardening target than generic context cleanup

## Implementation Priority

- Highest-ROI hardening:
  - make cache eviction semantics truthful and recover from poison without repeating panics
- Still deferred:
  - broader cross-subsystem CUDA context ownership changes

## Verification If Changed

- `cargo test -p ta_app_core options_expression_gpu::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core options_expression::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- rerun the options-expression GPU perf workflow if cache/runtime policy changes

## Regression And Feature-Preservation Constraints

- Preserve per-device runtime reuse
- Preserve current `shared_key` / `outcome_key` cache identity behavior
- Preserve exported `OptionsGpuRuntimeCacheUsage` fields
- Do not mix this cleanup with broader context-ownership changes unless it is perf-gated

## Third-Pass Conclusion

`Bucket unchanged, but the stronger issue is cache operability`

`options_expression_gpu.rs` remains a `Small surgical refactor` file, but the third pass shifts emphasis away from general context debt. The higher-value work is now cache-policy truthfulness and poison recovery in the process-wide runtime store.
