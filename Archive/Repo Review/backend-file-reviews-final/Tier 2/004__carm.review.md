# Final Consolidated Review: `crates/app_core/src/carm.rs`

- Review date: `2026-04-15`
- Source file: [carm.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/carm.rs:1>)
- First-pass packet: [carm.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/carm.rs.review.md:1>)
- Second-pass packet: [carm.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/carm.rs.review.md:1>)
- Third-pass packet: [carm.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/carm.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/004__crates__app_core__src__carm.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://serde.rs/field-attrs.html>
  - <https://vectoralpha.dev/projects/ta/indicators/otto/>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The earlier passes were correct to keep this file in the small bucket and to avoid broad cleanup. `carm.rs` remains a strategy-owner surface with public serde defaults, indicator dispatch, desired-position logic, and CPU/GPU parity implications. That makes compatibility-sensitive local fixes preferable to policy reshaping.

The fourth pass improves the priority order substantially. The strongest live safe-now issue is not broad selector cleanup. It is that [validate_carm_params](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/carm.rs:173>) still accepts negative `otto_threshold`, while OTTO construction and desired-position gating interpret that field differently. The packet is also right that the current “OTTO inclusive” test overstates what it actually proves. No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Reject negative `otto_threshold` in [validate_carm_params](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/carm.rs:173>).
- Add a direct regression test proving negative `otto_threshold` is rejected.
- Repair or replace the current “OTTO inclusive” test so it actually exercises equality-at-threshold behavior rather than only a positive spread with the default threshold.
- Add a short contract note explaining that `otto_threshold` is a public serde-defaulted field whose value currently influences both OTTO construction and desired-position gating.

## Deferred Work

- Defer splitting or reinterpreting `otto_threshold` into multiple fields or semantics.
- Defer deduplicating `hott`/`lott` computation; that likely requires broader adapter or output-surface work.
- Defer converting `resolve_output_id()` to a fail-closed exact-only policy.
- Defer broader selector and fallback-policy cleanup unless it is sourced from the dynamic indicator catalog contract.

## Verification If Changed

- `cargo test -p ta_app_core carm -- --nocapture`
- Add a regression for negative `otto_threshold`.
- Update the OTTO-threshold semantics test so equality and strict/inclusive behavior are explicitly characterized.
- Re-run representative CARM CPU/GPU smoke or parity paths after any parameter-validation change.

## Regression And Feature-Preservation Constraints

- Preserve existing behavior for valid non-negative `otto_threshold` values.
- Do not change OTTO semantics globally while only fixing validation and tests.
- Keep current fallback and indicator-resolution behavior stable.
- Avoid rolling in broader output-id or indicator-surface changes with the numeric validation fix.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `carm.rs` in `Small surgical refactor`
- reject negative `otto_threshold`, add the missing regression, and fix the misleading OTTO-threshold test
- document the current public-field contract rather than redesigning it
- defer selector-policy changes, OTTO recomputation cleanup, and broader compatibility reshaping
