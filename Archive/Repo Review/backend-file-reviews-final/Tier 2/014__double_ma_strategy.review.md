# Final Consolidated Review: `crates/app_core/src/double_ma_strategy.rs`

- Review date: `2026-04-15`
- Source file: [double_ma_strategy.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma_strategy.rs:1>)
- First-pass packet: [double_ma_strategy.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/double_ma_strategy.rs.review.md:1>)
- Second-pass packet: [double_ma_strategy.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_strategy.rs.review.md:1>)
- Third-pass packet: [double_ma_strategy.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_strategy.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/014__crates__app_core__src__double_ma_strategy.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://rust-lang.github.io/api-guidelines/naming.html>
  - <https://doc.rust-lang.org/book/ch09-00-error-handling.html>

## Consolidated Judgment

The fourth-pass packet is directionally correct because it reanchors the review on the live validator surface and avoids the stale third-pass framing. The strengthening needed here is that the first-pass accessor-normalization concern is now confirmed against the current `StrategyConfig` owner: accessors such as `kaufman_stop_period()` and `trend_quality_linearreg_period()` clamp raw zero values before this validator checks them.

That means the final guidance is slightly stronger than the fourth pass alone: this file still belongs in `Small surgical refactor`, but the highest-value safe-now work is now clearly test-first raw-field validation hardening on the branches that are currently reading normalized accessors too early. No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Add local negative-path tests for representative raw-zero cases that the validator currently claims to reject.
- Patch only the confirmed branches to validate raw submitted fields before consulting normalizing accessors.
- Prioritize representative fields across the real affected families, such as Kaufman stop, trend-quality, breakout, volume-flow, and PIT/ORC period/count checks.
- Add a short ownership note explaining that this module validates boundary inputs, while `double_ma_validate::repair_params` is an internal repair path for perturbation-style work.
- Leave the identity optimization wrappers alone in the same patch unless they are being documented as compatibility shims.

## Deferred Work

- Do not relocate validation ownership into `ta_strategies` as part of this packet.
- Do not introduce typed errors in the same change as raw-field hardening.
- Do not redesign the pairwise incompatibility matrix or wrapper naming layers in the same pass.
- Do not remove the internal repair layer used by validation perturbation flows.

## Verification If Changed

- `cargo test -p ta_app_core validate_strategy_cfg -- --nocapture`
- Add and run local regression tests in `double_ma_strategy.rs` for representative raw-zero cases.
- Re-run at least one optimization path and one validation path that pass through this validator after tightening the boundary.

## Regression And Feature-Preservation Constraints

- Preserve currently valid strategy combinations and supported profiles.
- Treat error text and first-failure ordering as compatibility-sensitive because these `String` errors surface upward.
- Tighten only malformed raw-input acceptance; do not change the behavior of valid configs or omitted-field defaults.
- Preserve internal repair behavior in `double_ma_validate.rs` for internal stochastic and perturbation flows.

## Final Conclusion

The implementation-facing conclusion is:

- keep `double_ma_strategy.rs` in `Small surgical refactor`
- treat the fourth pass as the right frame, but strengthen it with the now-confirmed accessor-normalization issue
- use test-first, branch-local hardening to validate raw fields before clamping accessors on the affected checks
- defer validator relocation, typed errors, wrapper cleanup, and profile-matrix redesign
