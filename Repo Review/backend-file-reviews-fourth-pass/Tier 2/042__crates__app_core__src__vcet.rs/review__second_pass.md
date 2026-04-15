# Second-Pass Review: `crates/app_core/src/vcet.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vcet.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/vcet.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as mostly sound but policy-heavy because mapping toggles, diagnostics, and host-stage fallback influence route semantics.

## Stale Check

The live source still:

- uses omission-driven `map_*` defaults
- carries thread-local route diagnostics
- uses `OnceLock` indicator-id caching

The first-pass packet remains current.

## Second-Pass Research Log

1. Serde default-field docs
   - Source: <https://serde.rs/attr-default.html>
   - Why consulted in second pass:
     - verify omission-driven parameter behavior
   - Additional constraint informed:
     - mapping defaults remain live contract, not convenience only

2. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in second pass:
     - reassess route-diagnostic scope
   - Additional constraint informed:
     - diagnostics and host-stage fallback hooks are observability/test controls with narrow scope

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep staging-path caution explicit
   - Additional constraint informed:
     - current CPU/GPU parity still argues for bounded rather than structural change

## Delta vs First Pass

### 1. `VcetStrategyParams` remains more contract-sensitive than the first pass emphasized because `map_*` toggles control whether fast/slow ranges or explicit values win

### 2. `OnceLock` indicator-id cache is intentional anti-overhead policy, not accidental indirection

- Practical implication:
  - keep cleanup small and localized

## Verification If Changed

- `cargo test -p ta_app_core vcet::tests:: -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve current default/mapping behavior
- Preserve route diagnostics and host-stage fallback hooks
- Preserve desired-position semantics unless CPU/GPU parity is revalidated

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that omission-driven mapping defaults and the `OnceLock` cache are both deliberate policy choices.
