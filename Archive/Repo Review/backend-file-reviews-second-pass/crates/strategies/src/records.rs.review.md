# Second-Pass Review: `crates/strategies/src/records.rs`

- Review date: `2026-04-14`
- Source file: `crates/strategies/src/records.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/strategies/src/records.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as compact and test-covered but contract-sensitive because it exposes serializable trade DTOs and a parallel-vector SoA container.

## Stale Check

The live source still:

- derives `Serialize` / `Deserialize`
- relies on `validate()` for the parallel-vector invariant
- computes per-column statistics through full scans

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground public invariant and semantics documentation expectations
   - Additional constraint informed:
     - flat-trade handling in `trade_stats_for_col()` is observable policy, not just a helper detail

## Delta vs First Pass

### 1. Flat closed trades are excluded from the `win_rate` denominator but still included in `closed_trades` and expectancy

- Why this matters:
  - this is a legitimate policy choice, but it is visible behavior and should not be "corrected" casually

### 2. Derived deserialization still allows malformed parallel-vector lengths to enter until `validate()` runs

- Practical implication:
  - the first-pass invariant-documentation need remains fully valid

## Verification If Changed

- `cargo test -p ta_strategies -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve field layout and overflow-checked `try_from_trade`
- Preserve current flat-trade handling in `TradeStats` unless downstream consumers explicitly re-baseline
- Keep the parallel-vector invariant explicit if docs are improved

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that flat-trade handling in `TradeStats` is active policy and that deserialized invalid lengths remain a real invariant gap until `validate()` is called.
