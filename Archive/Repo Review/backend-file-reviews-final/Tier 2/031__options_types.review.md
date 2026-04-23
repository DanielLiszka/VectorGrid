# Final Consolidated Review: `crates/app_core/src/options_types.rs`

- Review date: `2026-04-16`
- Source file: [options_types.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/options_types.rs:1>)
- First-pass packet: [options_types.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/options_types.rs.review.md:1>)
- Second-pass packet: [options_types.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_types.rs.review.md:1>)
- Third-pass packet: [options_types.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/options_types.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/031__crates__app_core__src__options_types.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://serde.rs/field-attrs.html>
  - <https://doc.rust-lang.org/cargo/reference/semver.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>

## Consolidated Judgment

The first three passes were right to keep this file in `Small surgical refactor`, but they stayed too abstract. The fourth pass is the best review because it isolates the one live, behavior-bearing omission/default edge case that is worth treating as the primary implementation target: `filters` omission shape is not semantically uniform.

No wrong `review_fourth_pass.md` is present in this folder. The fourth-pass packet matches the live file and should govern the next pass.

## Safe-Now Priority

- Add serde/default regression coverage for `OptionsExpressionRunRequest.filters`.
  - Omitting the entire `filters` field applies `OptionsStaticFilterBundle::default()`, which sets `max_entry_lag_minutes` to `Some(5)`.
  - A present but empty `filters` object deserializes each field through field-level `#[serde(default)]`, which leaves `max_entry_lag_minutes` as `None`.
  - This difference is behaviorally active because `max_entry_lag_minutes` influences candidate invalidation and structural cache identity.
- Do not change the behavior before the contract is pinned in tests. Add at least:
  - omitted `filters`,
  - `"filters": {}`,
  - `"filters": {"max_entry_lag_minutes": null}`.
- Expand invariant coverage around duplicated and derived fields rather than deleting them:
  - `validation_verdict`,
  - `robust_score`,
  - `tuple_count` versus `ranked_results.len()`,
  - `point_count` versus `points.len()`,
  - `sealed_finalist` and inferred sealed-style verdicts.
- Add rustdoc to the highest-blast-radius public types explaining:
  - active defaults,
  - duplicated counts or summaries,
  - derived fields,
  - provenance-bearing string labels that are intentionally persisted.
- If any string hardening lands, keep it limited to clearly local backend labels. Do not broadly convert provenance-bearing strings into enums in this pass.

## Deferred Work

- Do not blanket-add `deny_unknown_fields` across the major DTOs in this file. The file mixes persistence contracts, IPC-adjacent requests/results, and additive telemetry payloads.
- Do not rename persisted fields such as `selection_metrics` without a versioned contract migration.
- Do not remove duplicated fields wholesale in the same patch as new invariant tests.
- Do not overgeneralize local label centralization into a broad enum/newtype migration.

## Verification If Changed

- `cargo test -p ta_app_core options_types::tests:: -- --nocapture`
- `cargo check -p ta_app_core --tests`
- `cargo test -p ta_app_core options_expression::tests::refresh_validation_verdicts_requires_supported_validation_lanes -- --nocapture`
- If request default behavior changes:
  - focused `options_expression` request/load/replay tests
  - saved-artifact and dataset/signal-manifest replay tests that depend on request semantics

## Regression And Feature-Preservation Constraints

- Preserve current omission/default semantics until the `filters` contract is explicitly decided and tested.
- Treat this file as a persisted contract seam first and a cleanup target second.
- Do not casually reinterpret derived fields during load or display without checking producer-side refresh paths.
- Keep additive telemetry/report surfaces tolerant unless there is a versioned compatibility plan.

## Final Conclusion

Keep `options_types.rs` in `Small surgical refactor`, treat the fourth pass as valid, and prioritize the `filters` omission/default contract plus targeted invariants and rustdoc. Broader serde strictness, persisted-field renames, and schema cleanup remain deferred.
