# Final Consolidated Review: `crates/app_core/src/validation_stats.rs`

- Review date: `2026-04-15`
- Source file: [validation_stats.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/validation_stats.rs:1>)
- First-pass packet: [validation_stats.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/validation_stats.rs.review.md:1>)
- Second-pass packet: [validation_stats.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_stats.rs.review.md:1>)
- Third-pass packet: [validation_stats.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/validation_stats.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/023__crates__app_core__src__validation_stats.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Partially valid; correct bucket, but rationale overcorrects`
- Canonical final action bucket: `Document only`
- External research anchors:
  - <https://doc.rust-lang.org/std/option/enum.Option.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://www.davidhbailey.com/dhbpapers/sharpe-frontier.pdf>
  - <https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf>

## Consolidated Judgment

The fourth-pass packet belongs to the correct item and lands on the right implementation outcome for this file, but its rationale needs correction. The final bucket for `validation_stats.rs` itself should stay `Document only`, because this module is not the best first place to change behavior. The live validation hard-gate path is duplicated downstream in `validation_pipeline.rs`, and `compute_psr_dsr()` is also reused outside validation in shortlist-confidence flows.

What the fourth pass underweighted is that the third-pass concern was not purely imaginary. `sharpe_stats::probabilistic_sharpe_ratio()` currently preserves a `0.5` invalid-input fallback, so wrapper-level `Some(...)` values can still reflect upstream sentinel-style policy rather than true statistical availability. That means the correct final read is: there is a real cross-file contract issue, but `validation_stats.rs` is the wrong first layer for a behavior-first fix.

## Safe-Now Priority

- Keep this file behaviorally unchanged unless the surrounding `sharpe_stats.rs` and validation consumers are being reviewed at the same time.
- Add rustdoc that states the exact clamp and fallback semantics: finite outputs are bounded into `[0.0, 1.0]`, non-finite outputs become `None`, and missing PSR/DSR is compared as `0.0` by the gate helper.
- Add one narrow edge-case test for the `0.0`-threshold path so the current "missing compared as `0.0`" behavior is pinned precisely.
- Tighten wording in this review chain so it no longer claims that missing stats universally fail whenever a threshold exists.

## Deferred Work

- Do not redesign the `Option` availability contract in this file alone.
- If the repo wants `Option` to mean true statistical unavailability rather than current wrapper semantics, start in `sharpe_stats.rs` and audit all callers together.
- Do not change `passes_probability_gates()` in isolation and assume validation hard-gate behavior is fixed. The live validation pipeline duplicates the gating logic inline.

## Verification If Changed

- `cargo test -p ta_app_core validation_stats::tests:: -- --nocapture`
- `cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`
- If behavior changes anyway, also rerun nearby shortlist-confidence coverage, especially the probability-ordering tests that consume the shared PSR/DSR helpers indirectly.

## Regression And Feature-Preservation Constraints

- Preserve current clamp semantics for finite PSR/DSR outputs unless all downstream consumers are reviewed together.
- Preserve current "missing compared as `0.0`" behavior unless the validation pipeline and confidence-shortlist consumers are updated in the same patch.
- Do not treat this file as validation-only. It participates in shortlist reranking and therefore affects more than run verdicts.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `validation_stats.rs` at `Document only`
- preserve the fourth pass's implementation outcome, but not its full rationale
- document the current clamp and fallback semantics more precisely
- defer any true availability-model redesign to a coordinated pass across `sharpe_stats.rs`, `validation_stats.rs`, `validation_pipeline.rs`, and shortlist-confidence consumers
