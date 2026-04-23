# Final Consolidated Review: `crates/app_core/src/ml_confidence.rs`

- Review date: `2026-04-14`
- Source file: [ml_confidence.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/ml_confidence.rs:1>)
- First-pass packet: [ml_confidence.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/ml_confidence.rs.review.md:1>)
- Second-pass packet: [ml_confidence.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/ml_confidence.rs.review.md:1>)
- Third-pass packet: [ml_confidence.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/ml_confidence.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/011__crates__app_core__src__ml_confidence.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`

## Consolidated Judgment

The fourth pass is the right authoritative packet here. The file stays in `Small surgical refactor`, and the safe-now work is public-entrypoint hardening rather than model-behavior redesign. The real issues are scope validation drift across the three training entrypoints, feature-shape permissiveness that can panic or train unusable models, and train-side fallback evidence that can read more authoritative than it is. Those are all bounded fixes if the patch stays away from changing feature order/count or swapping the modeling path itself.

## Safe-Now Priority

- Unify scope normalization and validation across all three public training entrypoints, and reject prepared-row scope mismatches.
- Validate raw and prepared row shape before training. Every row should match the fixed seven-feature schema and contain finite values.
- Make train-side fallback evidence explicit in warnings and metadata so `healthy` and calibration-related outputs do not read as fully CV-backed when they have fallen back to train metrics.
- Document and test the current `Auto` and `meta_artifacts_used` semantics instead of silently reinterpreting them.

## Deferred Work

- Keep feature-count/order changes and CV splitter changes out of this hardening pass.
- Do not mix model-behavior drift with API-safety work.

## Verification If Changed

- `cargo test -p ta_app_core ml_confidence -- --nocapture`
- Add direct tests for mismatched scope, bad feature width, and non-finite feature values
- Compare representative persisted model metadata before and after any warning/metadata clarification

## Regression And Feature-Preservation Constraints

- Tightening entrypoint validation will convert some currently accepted direct-caller inputs into hard errors; treat that as an intentional contract change and cover it in tests.
- Do not silently reinterpret `healthy` or `meta_artifacts_used`, because those semantics can leak into persisted models and meta-command status handling.
- Keep model topology and feature ordering unchanged in this patch.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `ml_confidence.rs` at `Small surgical refactor`
- harden scope and feature-shape validation now
- clarify fallback evidence without changing model behavior in the same patch
