# Second-Pass Review: `crates/app_core/src/strategy_presets.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_presets.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/strategy_presets.rs.review.md`
- First-pass action bucket: `Document only`
- Second-pass action bucket: `Document only`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as a curated fixture catalog whose main need is clearer intent documentation, not code changes.

## Stale Check

The live source still provides:

- hard-coded representative fixtures
- grouped strategy subsets like `double_like_kernel_strategy_cases()`
- benchmark and route/parity inputs rather than generated defaults

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground representative-fixture intent documentation
   - Additional constraint informed:
     - the file still reads as curated fixture policy, not implementation risk

2. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in second pass:
     - reassess how much downstream coverage anchors on these presets
   - Additional constraint informed:
     - even simple preset changes remain benchmark- and parity-sensitive

## Delta vs First Pass

### 1. These presets are more clearly curated representative fixtures than the first pass spelled out

- Why this matters:
  - changing the numbers affects route and parity coverage, not just convenience examples
- Practical implication:
  - keep the file in `Document only`

## Verification If Changed

- `cargo test -p ta_app_core strategy_presets::tests:: -- --nocapture`
- a representative `double_ma` route/parity lane

## Regression And Feature-Preservation Constraints

- Preserve grouped-case membership and current parameter choices unless route/parity coverage is re-audited
- Treat preset changes as benchmark-input-sensitive
- Do not repurpose these fixtures into generated defaults

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Document only` bucket remains correct. The second pass mainly clarifies that these presets are curated coverage fixtures, not generic defaults or arbitrary samples.
