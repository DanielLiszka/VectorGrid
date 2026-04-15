# Second-Pass Review: `crates/app_core/src/vcc.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vcc.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/vcc.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a many-responsibilities strategy owner and deferred larger cleanup because parameter mapping, diagnostics, and hybrid staging all interact.

## Stale Check

The live source still:

- uses omission-driven `map_*` toggles in `VccStrategyParams`
- relies on thread-local diagnostics and fallback controls
- owns multiple route and desired-position responsibilities at once

The first-pass packet remains current.

## Second-Pass Research Log

1. Serde default-field docs
   - Source: <https://serde.rs/attr-default.html>
   - Why consulted in second pass:
     - verify omission-driven parameter behavior
   - Additional constraint informed:
     - the `map_*` toggles materially define product behavior rather than convenience defaults

2. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in second pass:
     - reassess route-diagnostic scope
   - Additional constraint informed:
     - observability and forced-fallback controls are narrower in scope than process-global readers may assume

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep hybrid CPU/GPU staging caution explicit
   - Additional constraint informed:
     - safe immediate work still favors documentation over behavior churn

## Delta vs First Pass

### 1. The omission defaults and `map_*` toggles are stronger product contract than the first pass emphasized

### 2. Route diagnostics and forced fallback remain thread-local observability hooks, not generic runtime state

- Practical implication:
  - preserve the defer bucket

## Verification If Changed

- `cargo test -p ta_app_core vcc::tests:: -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve current default mapping behavior
- Preserve thread-local route diagnostics and test fallback hooks
- Do not alter desired-position semantics without parity review

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass mainly clarifies that omission-driven mapping defaults and thread-local diagnostics are stronger product contract than they first appeared.
