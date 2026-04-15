# Final Consolidated Review: `crates/app_core/src/composite_profile_route.rs`

- Review date: `2026-04-14`
- Source file: [composite_profile_route.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_profile_route.rs:1>)
- First-pass packet: [composite_profile_route.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/composite_profile_route.rs.review.md:1>)
- Second-pass packet: [review__second_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_profile_route.rs.review.md:1>)
- Third-pass packet: [review__third_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_profile_route.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/003__crates__app_core__src__composite_profile_route.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Document only`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://rust-lang.github.io/api-guidelines/naming.html>

## Consolidated Judgment

The second pass had the right bucket. This file is a small, coherent policy helper centered on [composite_profile_from_strategy](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_profile_route.rs:11>), [composite_profile_runtime_label](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_profile_route.rs:24>), and [composite_profile_gpu_target](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_profile_route.rs:42>). The third pass correctly warned that label churn is compatibility-sensitive, but over-escalated by treating that concern as proof the file itself needed code refactoring. The fourth pass is the authoritative correction: the safe-now work is documentation and ownership-boundary clarification, not structural change.

## Safe-Now Priority

- Add short rustdoc explaining that this module classifies composite-profile policy and intended GPU target, not realized per-stage backend execution.
- Document the two label axes explicitly:
  - [composite_profile_runtime_label](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_profile_route.rs:24>) returns the stable runtime-family label
  - [composite_profile_gpu_target](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_profile_route.rs:42>) and [composite_profile_gpu_target_label](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_profile_route.rs:34>) describe intended target policy
- Document precedence explicitly: `BreakoutMomentum > RegimeHybrid > CompositeTrend > CompositeCpu`
- Preserve the current strings, precedence, and total match arms exactly as they are, including the contract asserted by tests starting at [187](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_profile_route.rs:187>)

## Deferred Work

- Track any deduplication with `strategy_dynamic.rs` or frontend mirrors as a separate cross-file task.
- If the repo ever wants cleaner naming, do it as a coordinated route-vocabulary migration rather than an isolated rename here.

## Verification If Changed

- `cargo test -p ta_app_core composite_profile_route -- --nocapture`
- If strings or behavior change, widen verification:
  - `cargo test -p ta_app_core composite_validate -- --nocapture`
  - `cargo test -p ta_app_core double_ma_validate -- --nocapture`
  - frontend parity verification for the route-preview mirror

## Regression And Feature-Preservation Constraints

- Preserve emitted strings exactly unless a broader compatibility audit is in scope.
- Preserve current precedence, because downstream validation gating also depends on the classifier.
- Do not let this file drift into claiming ownership of realized backend-route semantics.
- Do not remove defensive total-match arms as cosmetic cleanup.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `composite_profile_route.rs` at `Document only`
- add boundary and label-axis documentation
- preserve the live strings and mapping behavior exactly
