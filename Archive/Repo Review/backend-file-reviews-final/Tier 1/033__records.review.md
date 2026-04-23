# Final Consolidated Review: `crates/strategies/src/records.rs`

- Review date: `2026-04-15`
- Source file: [records.rs](</C:/Rust Projects/ta_desktop_demo/crates/strategies/src/records.rs:1>)
- First-pass packet: [records.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/strategies/src/records.rs.review.md:1>)
- Second-pass packet: [records.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/strategies/src/records.rs.review.md:1>)
- Third-pass packet: [records.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/strategies/src/records.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/033__crates__strategies__src__records.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The review progression here is mostly about scoping. Earlier passes were right that this file has real invariant and API-shape debt, but the fourth pass is the first one that cleanly separates safe-now work from breaking redesign. Because `TradeRecords` exposes public vectors directly, true invariant enforcement is not a small change. That pushes the correct immediate scope toward documentation, debug-only checks, and targeted statistical edge-case tests.

The fourth pass also sharpens the most important live asymmetry: `len()` is anchored to `col`, so malformed state is not merely "invalid." It is asymmetric. If `col` is longer than a sibling vector, accessors can panic. If `col` is shorter, other tail data becomes silently unreachable. That is exactly the kind of contract that should be documented and tested before any deeper redesign.

No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Add explicit rustdoc for:
  - the equal-length invariant across all columns
  - `col` being the row-count anchor
  - the malformed-state consequences for accessors and scans
- Add targeted `TradeStats` edge-case tests, especially for:
  - flat closed trades
  - all-win / no-loss `profit_factor`
  - parity with the duplicated trade-stat logic used elsewhere
- Add narrow debug-only invariant assertions in places where malformed state would otherwise fail late and opaquely.

## Deferred Work

- Defer any redesign that privatizes the fields or changes serialized shape.
- Defer validated-deserialization changes.
- Defer changing `get()` semantics.
- Defer making `try_extend_trades()` transactional or otherwise behavior-changing.

## Verification If Changed

- `cargo test -p ta_strategies -- --nocapture`
- Add targeted tests around malformed-state behavior and `TradeStats` edge cases.
- Only widen verification scope if serialization semantics or field visibility changes.

## Regression And Feature-Preservation Constraints

- Preserve the current public field layout and serde shape unless a deliberate API change is being made.
- Preserve current `TradeStats` semantics where downstream code already depends on them.
- Keep safe-now work additive and characterization-oriented rather than structural.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `records.rs` in `Small surgical refactor`
- spend the immediate effort on invariant rustdoc, debug-only assertions, and parity-sensitive `TradeStats` tests
- preserve the current public struct shape and access semantics for now
- defer real invariant enforcement and API reshaping until a deliberate breaking-change pass
