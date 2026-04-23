# Final Consolidated Review: `crates/optimizer/src/grid.rs`

- Review date: `2026-04-16`
- Source file: [grid.rs](</C:/Rust Projects/ta_desktop_demo/crates/optimizer/src/grid.rs:1>)
- First-pass packet: [grid.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/optimizer/src/grid.rs.review.md:1>)
- Second-pass packet: [grid.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/grid.rs.review.md:1>)
- Third-pass packet: [grid.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/optimizer/src/grid.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/044__crates__optimizer__src__grid.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/primitive.slice.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The fourth pass is valid and should govern the final review. It correctly upgrades this file from "comments only" to a small but real contract-hardening target.

The main live issue is that [`grid_search()`](</C:/Rust Projects/ta_desktop_demo/crates/optimizer/src/grid.rs:19>) assumes `BacktestEngine::eval_batch()` returns exactly one `Metrics` row per input combo, but neither the trait nor the function enforces that. Under-delivery silently truncates `all`, while over-delivery can panic when `best_params` or `top` index `combos`. The rest of the observable contract still matters too: `best_score` and `top_scores` are normalized ranking scores, risk objectives negate raw metrics, `all_scores` stays `None`, and equal-score ties currently preserve input order because the code sorts an index vector with stable `sort_by`.

## Safe-Now Priority

- Make the `eval_batch()` cardinality contract explicit and enforce it consistently instead of allowing silent truncation on one side and panic risk on the other.
- Add focused tests for:
  - risk-objective sign semantics in `best_score` and `top_scores`,
  - equal-score tie ordering,
  - `top_k == 0` shape,
  - current `all_scores: None` behavior,
  - cardinality mismatch handling.
- Add a short contract comment near `score()` and `grid_search()` clarifying stream-parity expectations and ranking-score semantics.

## Deferred Work

- Do not replace the current top-k ordering path with a heap or unstable selection routine unless tie-order compatibility is intentionally changed and retested.
- Do not extract a shared scorer abstraction until the result-shape contract is better pinned.
- Do not change `OptimizationResult` field population in this pass beyond making the existing contract explicit and safe.

## Verification If Changed

- `cargo test -p ta_optimizer grid -- --nocapture`
- If `OptimizationResult` shaping changes, rerun representative optimizer consumer tests in `ta_app_core`

## Regression And Feature-Preservation Constraints

- Preserve the current serialized/result DTO shape unless downstream consumers move together.
- Preserve descending normalized-score ordering and the current equal-score tie behavior unless intentionally changed with coverage.
- Keep non-finite scores mapped to `NEG_INFINITY` unless stream aggregation moves in lockstep.

## Final Conclusion

Keep `grid.rs` in `Small surgical refactor`. The right next move is to lock down the `eval_batch` cardinality contract and add targeted ordering/score-shape coverage, not to redesign the helper or chase performance changes.

