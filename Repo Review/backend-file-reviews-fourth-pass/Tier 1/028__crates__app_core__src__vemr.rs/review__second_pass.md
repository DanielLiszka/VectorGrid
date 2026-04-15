# Second-Pass Review: `crates/app_core/src/vemr.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vemr.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/vemr.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a many-responsibilities strategy owner where late-bound selector values, diagnostics, and staged CPU/CUDA execution make safe local cleanup difficult.

## Stale Check

The live source still:

- exposes late-bound `z_ma_type` and `z_devtype`
- uses thread-local route diagnostics and forced fallback hooks
- combines multiple responsibilities under one strategy-local surface

The first-pass packet remains current.

## Second-Pass Research Log

1. Serde default-field docs
   - Source: <https://serde.rs/attr-default.html>
   - Why consulted in second pass:
     - reassess omission-driven selector semantics
   - Additional constraint informed:
     - selector values remain fragile public config

2. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in second pass:
     - verify diagnostic-scope expectations
   - Additional constraint informed:
     - route diagnostics and fallback hooks remain thread-local observability/test controls

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep staged hybrid execution caution explicit
   - Additional constraint informed:
     - safe immediate work still favors documentation and contract clarity

## Delta vs First Pass

### 1. `VemrStrategyParams` remains more fragile public selector surface than the first pass emphasized

- Why this matters:
  - `z_ma_type` and `z_devtype` materially affect CPU vs CUDA eligibility

### 2. Thread-local diagnostics and fallback hooks are observability/test controls, not generic runtime state

- Practical implication:
  - preserve the defer bucket

## Verification If Changed

- `cargo test -p ta_app_core vemr::tests:: -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`
- representative CUDA-facing VEMR coverage if route behavior changes

## Regression And Feature-Preservation Constraints

- Preserve default period mapping and z-score selector semantics unless callers move together
- Preserve route diagnostics and fallback hooks
- Preserve current desired-position behavior unless CPU/GPU parity is reverified

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass mainly clarifies that late-bound selector values and thread-local diagnostics remain stronger product contract than they first appeared.
