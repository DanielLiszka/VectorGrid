# Third-Pass Review: `crates/strategies/src/records.rs`

- Review date: `2026-04-14`
- Source file: `crates/strategies/src/records.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/strategies/src/records.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/strategies/src/records.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the strategy records structure-of-arrays owner still looked like the right framing under fresh research. The third pass kept the bucket but sharpened the blast radius of malformed state: length mismatches can panic before validate is called, and append failure is not transactional.

## Third-Pass Research Log

1. Rust Book guidance on panic vs Result
   - Source: <https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html>
   - Why consulted in third pass:
     - recheck whether malformed record state is still being surfaced honestly
   - Third-pass constraint informed:
     - shared record helpers should not rely on callers remembering a separate validation step before every access

2. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether this still fits a small bounded hardening bucket
   - Third-pass constraint informed:
     - public record accessors should keep malformed-state behavior explicit and test-backed

## What Third Pass Sharpened

### 1. Malformed SoA state can panic before validate is ever called
- Evidence:
  - get, to_trades_for_col, and trade_stats_for_col assume matching vector lengths.
- Why this matters:
  - Length mismatch is therefore not merely latent until a dedicated validation call; accessors can panic if callers skip it.
- Practical implication:
  - Keep the existing bucket, but treat pre-validation access behavior as the sharper hardening target.

### 2. try_extend_trades is not transactional on failure
- Evidence:
  - The helper can partially append records before returning an overflow error on a later trade.
- Why this matters:
  - That broadens the effect of malformed or oversized input beyond one rejected append attempt.
- Practical implication:
  - If hardened, make append failure atomic or explicitly documented and tested as partial mutation.

## Implementation Priority

- First hardening step: decide whether accessor paths should require or enforce validated state.
- Second hardening step: make append failure atomic or explicitly partial-by-contract.

## Verification If Changed

- cargo test -p ta_strategies records -- --nocapture
- representative record-state tests for mismatched lengths and partial append failure

## Regression And Feature-Preservation Constraints

- Preserve current record layout and performance characteristics unless consumer crates move together.
- Do not hide malformed-state access behind silent truncation.
- Keep transactional versus partial-append behavior explicit and test-backed if changed.

## Third-Pass Conclusion

`Bucket unchanged, but malformed-state blast radius sharpened`

records.rs stays in Small surgical refactor. The third pass mainly sharpens two bounded issues: malformed structure-of-arrays state can panic before explicit validation runs, and append failure is still not transactional.
