# Final Consolidated Review: `crates/app_core/src/stats_common.rs`

- Review date: `2026-04-16`
- Source file: [stats_common.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/stats_common.rs:1>)
- First-pass packet: [stats_common.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/stats_common.rs.review.md:1>)
- Second-pass packet: [stats_common.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/stats_common.rs.review.md:1>)
- Third-pass packet: [stats_common.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/stats_common.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 3/004__crates__app_core__src__stats_common.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Safe-now hardening`
- External research anchors:
  - <https://doc.rust-lang.org/std/primitive.f64.html#method.clamp>
  - <https://doc.rust-lang.org/std/primitive.f64.html#method.round>
  - <https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast>
  - <https://doc.rust-lang.org/std/primitive.slice.html#method.select_nth_unstable>

## Consolidated Judgment

The earlier packets correctly resisted algorithm churn, but the fourth pass is valid because it identifies the real gap: the current file exposes public numeric helpers without fully documenting or pinning the live conventions in [quantile](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/stats_common.rs:1>), [median](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/stats_common.rs:13>), [worst_min](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/stats_common.rs:17>), [worst_max](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/stats_common.rs:29>), [median_u32](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/stats_common.rs:41>), and [worst_min_u32](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/stats_common.rs:50>).

`Document only` is too passive here. The file still does not justify semantic redesign, but it does justify a small hardening pass that makes the actual behavior explicit and test-backed, especially around sorted-input expectations, clamping, even-length medians, and non-finite extreme handling.

## Safe-Now Priority

- Add rustdoc stating the real contracts:
  - `quantile` expects already-sorted input;
  - `q` is clamped into `[0, 1]`;
  - the helper is rounded-index, not interpolated;
  - empty-input fallbacks remain `0.0` / `0`;
  - even-length medians resolve to the upper middle element;
  - `worst_min` / `worst_max` return `0.0` when the selected extreme is non-finite.
- Add tests for:
  - `q < 0` / `q > 1` clamping;
  - even-length `median` and `median_u32`;
  - infinity behavior in `worst_min` / `worst_max`.

## Deferred Work

- Do not change the quantile convention.
- Do not change median semantics.
- Do not replace forgiving helpers with `Option` / `Result`.
- Do not rewrite the helpers around `select_nth_unstable` without evidence that they are hot.
- Do not sort internally inside `quantile` unless you are intentionally changing contract semantics.

## Verification If Changed

- `cargo test -p ta_app_core stats_common::tests:: -- --nocapture`
- Expand verification to validation/statistics consumers only if behavior changes rather than docs/tests.

## Regression And Feature-Preservation Constraints

- Preserve current rounded-index quantile behavior.
- Preserve upper-middle median behavior.
- Preserve current empty-input defaults.
- Do not silently change infinity or broader non-finite handling.
- Do not accidentally ossify `q = NaN` into a stronger contract unless that is intentional.

## Final Conclusion

Change the final bucket to `Safe-now hardening`. The fourth pass is valid, no wrong fourth-pass file is present, and the right next move is narrow documentation and unit-test hardening without changing the helper algorithms.
