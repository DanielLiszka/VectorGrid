# Final Consolidated Review: `crates/app_core/src/composite_mo.rs`

- Review date: `2026-04-15`
- Source file: [composite_mo.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_mo.rs:1>)
- First-pass packet: [composite_mo.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/composite_mo.rs.review.md:1>)
- Second-pass packet: [composite_mo.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_mo.rs.review.md:1>)
- Third-pass packet: [composite_mo.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_mo.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/008__crates__app_core__src__composite_mo.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with narrowing`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://serde.rs/enum-representations.html>
  - <https://doc.rust-lang.org/std/fmt/trait.Debug.html>

## Consolidated Judgment

All four passes land on the right bucket. The third-pass packet is the weak link because it drifted into cache/hash framing that is not the real live risk in this file. The fourth-pass packet is the corrective one: the real review surface is still the public request and telemetry contract, invalid-path accounting, prescan edge cases, and the deliberate final CPU `Trade22` rewrite.

The fourth-pass packet is valid, but it should be read narrowly. The safe-now work is contract hardening and test coverage, not behavior redesign. No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Make invalid-path rejection accounting internally consistent before adding any new telemetry buckets.
- If the `start_t >= n` or cancellation path is short-circuited, preserve the current `rejected_other_constraints` accounting semantics explicitly.
- For fixed-type requests, reject or explicitly guard `gpu_prescan.ma_pairs` values that exclude the request's fixed base MA pair.
- Document and test that top-level request `mode` is currently ignored instead of silently treating it as active policy.
- Add focused tests for sampler wire tags, prescan fixed-pair exclusion, and the current search-phase-only meaning of `requested_evaluations` / `actual_evaluations`.

## Deferred Work

- Do not derive full `Deserialize` support for the result/telemetry surface yet; several fields are still reporting-oriented strings rather than persistence-grade schema.
- Do not redesign or remove the final CPU `Trade22` Pareto rewrite in this pass.
- Do not broaden `ma_pairs` validation into a full MA-type-domain policy rewrite for optimized MA-type search spaces.
- Do not rename public telemetry fields or redefine evaluation-counter semantics without updating callers and tests together.

## Verification If Changed

- `cargo test -p ta_app_core composite_mo -- --nocapture`
- `cargo test -p ta_app_core composite_mo_fast_objectives_emit_trade22_metric_surface -- --nocapture`
- `cargo test -p ta_app_core composite_mo_gpu_prescan_narrows_large_len_ranges -- --nocapture`
- `cargo test -p ta_app_core composite_mo_gpu_prescan_supports_multiple_ma_pairs -- --nocapture`
- `cargo test -p ta_app_core composite_mo_candidates_validate_consistently -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve the deliberate final CPU `Trade22` metric rewrite.
- Preserve current prescan narrowing semantics unless the search-domain contract is being changed deliberately.
- Preserve current sampler tag spellings unless compatibility aliases or a versioned request-shape change is added.
- Treat `requested_evaluations`, `actual_evaluations`, and exact-grid counters as compatibility-sensitive telemetry fields.

## Final Conclusion

The implementation-facing conclusion is:

- keep `composite_mo.rs` in `Small surgical refactor`
- treat the fourth pass as the authoritative correction to a misfocused third pass
- prioritize invalid-path accounting truth, fixed-type `gpu_prescan.ma_pairs` validation, and explicit ignored-`mode` documentation/tests
- defer persistence-schema work, telemetry renames, and any redesign of the final `Trade22` enrichment flow
