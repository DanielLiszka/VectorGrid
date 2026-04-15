# Second-Pass Review: `crates/app_core/src/mkcs.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/mkcs.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/mkcs.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this strategy owner as route-sensitive and partially opaque, with registry initialization behavior and stale bench coverage as the sharper maintenance risks.

## Stale Check

The live source still:

- caches indicator-registry init through `OnceLock<Result<...>>`
- resolves output IDs permissively
- has route-sensitive optimization and validation behavior

The first-pass packet remains current.

## Second-Pass Research Log

1. `OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in second pass:
     - confirm cached-failure behavior
   - Additional constraint informed:
     - initialization failures are still process-lifetime sticky

2. The Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why consulted in second pass:
     - reassess contract-heavy public surface review
   - Additional constraint informed:
     - permissive output selection remains more worrying than kernel logic here

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - ground fallback and perf-path review
   - Additional constraint informed:
     - stale bench coverage still matters in a route-sensitive file

## Delta vs First Pass

### 1. `OnceLock<Result<...>>` still makes registry-init failure a process-lifetime condition

- Why this matters:
  - operational recovery differs materially from transient failure semantics
- Practical implication:
  - keep this behavior explicit if surfaced or documented

### 2. The stale `mkcs_opt` bench surface remains a meaningful validation gap

- Practical implication:
  - if performance-sensitive route work changes, bench maintenance should move with it

## Verification If Changed

- `cargo test -p ta_app_core mkcs::tests:: -- --nocapture`
- `cargo test -p ta_app_core mkcs_optimization_smoke_runs -- --nocapture`
- `cargo bench -p ta_app_core --bench mkcs_opt --no-run` if bench code changes

## Regression And Feature-Preservation Constraints

- Preserve current device-resident sweep ownership
- Preserve current CPU fallback behavior unless deliberately reworked
- Preserve desired-position semantics unless invalid-input handling is intentionally redesigned

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly reinforces that sticky registry-init failure and stale bench coverage remain the sharper maintenance risks.
