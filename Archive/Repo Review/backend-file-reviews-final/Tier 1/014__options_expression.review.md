# Final Consolidated Review: `crates/app_core/src/options_expression.rs`

- Review date: `2026-04-14`
- Source file: [options_expression.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs:1>)
- First-pass packet: [options_expression.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/options_expression.rs.review.md:1>)
- Second-pass packet: [options_expression.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_expression.rs.review.md:1>)
- Third-pass packet: [options_expression.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/options_expression.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/014__crates__app_core__src__options_expression.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid in substance but needs normalization`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/sync/struct.Mutex.html>
  - <https://doc.rust-lang.org/std/sync/struct.Arc.html>

## Consolidated Judgment

The fourth pass is substantially right: this file remains a `Small surgical refactor`, and the highest-ROI fixes are in inspection correctness and boundary documentation, not in ranking or robustness semantics. What needs normalization is scope. The final packet should make clear that the user-visible inspection fixes are still behavior changes, and that the aggregate stress-lane asymmetry is real but explicitly deferred because it would move ranking semantics, not just inspection output.

## Safe-Now Priority

- Fix inspection `entry_lag_minutes` so it measures from `requested_entry_timestamp`, not `earliest_tradable_timestamp`.
- Make the fallback companion-entry pricing in `build_event_detail_row(...)` structure-aware for `OutcomeMissing` cases rather than always using exit-side pricing for the companion leg.
- Make `build_event_path_points(...)` either honor quote-aware pricing semantics or explicitly document and test that path inspection remains raw aggregate-bar visualization.
- Add rustdoc to the public blocking entrypoints clarifying:
  - inspection reruns prepare plus ranking
  - `tuple_index` is current-rank-relative rather than a stable identity
  - inspection is baseline-lane replay
  - split fallback semantics are part of live behavior

## Deferred Work

- Keep aggregate stress-lane buy-side asymmetry deferred. That is ranking and robustness semantics work, not inspection cleanup.
- Do not mix inspection correctness fixes with any broader ranking-model change.

## Verification If Changed

- `cargo test -p ta_app_core options_expression -- --nocapture`
- Rerun inspection-focused tests and any UI-facing screenshot or serialization checks that consume tuple detail rows or event-path outputs
- If path pricing semantics change, compare representative quote-semantics runs before and after the patch

## Regression And Feature-Preservation Constraints

- These fixes are user-visible and can change detail rows and event-path outputs even if ranking stays unchanged.
- Quote-aware path fixes can materially change charts for quote-semantics runs; treat that as an intentional behavior change with verification, not as a hidden cleanup.
- Keep aggregate stress-lane semantics out of scope for this patch.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `options_expression.rs` at `Small surgical refactor`
- prioritize inspection correctness and boundary documentation
- defer any ranking or robustness semantics work to a separate pass
