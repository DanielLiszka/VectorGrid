# Final Consolidated Review: `crates/app_core/src/sharpe_stats.rs`

- Review date: `2026-04-15`
- Source file: [sharpe_stats.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/sharpe_stats.rs:1>)
- First-pass packet: [sharpe_stats.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/sharpe_stats.rs.review.md:1>)
- Second-pass packet: [sharpe_stats.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/sharpe_stats.rs.review.md:1>)
- Third-pass packet: [sharpe_stats.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/sharpe_stats.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/017__crates__app_core__src__sharpe_stats.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://www.davidhbailey.com/dhbpapers/sharpe-frontier.pdf>
  - <https://stackedboxes.org/2017/05/01/acklams-normal-quantile-function/>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>

## Consolidated Judgment

The fourth pass correctly overturns the third pass’s emphasis. The strongest live issue is not the `0.5` PSR fallback contract. It is the tail-branch sign bug in `normal_inv_cdf`, which distorts `expected_max_sharpe` and therefore `deflated_sharpe_ratio` once `num_trials` gets large enough to enter Acklam’s tail regions.

This still belongs in a small patch. The right change is narrow: fix the inverse-CDF tail signs, add tests that would have caught the bug, and document the most misuse-prone semantics. Do not combine it with a broader redesign of invalid-input handling.

## Safe-Now Priority

- Fix the tail-branch sign handling in `normal_inv_cdf` to match Acklam’s approximation.
- Add regression tests for:
  - lower-tail sign
  - upper-tail sign
  - representative tail and central round-trips
  - monotonic `expected_max_sharpe` as `num_trials` grows
- Add a test that freezes the current PSR fallback behavior if that sentinel contract remains intentional for now.
- Document that `kurtosis` is treated as raw Pearson kurtosis, not excess kurtosis.
- Document that these helpers are Sharpe-family statistical estimators, not generic math utilities, and that `sr_hat` / `benchmark_sr` need matching frequency semantics.

## Deferred Work

- Do not redesign the PSR invalid-input return shape in the same patch.
- Do not change approximation family, switch libraries, or combine the fix with broader formula rewrites.
- Do not alter valid-input PSR / DSR formula behavior unless the validation wrapper layer is being reviewed together.

## Verification If Changed

- `cargo test -p ta_app_core sharpe_stats::tests:: -- --nocapture`
- `cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`
- If present and used by the current wrapper flow:
  - `cargo test -p ta_app_core validation_stats::tests:: -- --nocapture`

## Regression And Feature-Preservation Constraints

- Keep the fix surgical. The intentional output drift should be in DSR-related values for moderate and larger trial counts, not in unrelated helper behavior.
- Do not mix the inverse-CDF correction with PSR sentinel redesign.
- Do not mix it with `T` vs `T - 1` formula changes, dependency swaps, or kurtosis/skewness sanitization changes.
- Expect DSR-adjacent validation gating to move legitimately after the fix.
- `minimum_track_record_length` should not materially move from this patch because it squares the quantile term.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `sharpe_stats.rs` at `Small surgical refactor`
- fix the inverse-CDF tail sign bug first
- add tail-focused regression tests and document kurtosis/sentinel semantics
- defer the broader invalid-input contract discussion until the wrapper-layer validation policy is reviewed directly
