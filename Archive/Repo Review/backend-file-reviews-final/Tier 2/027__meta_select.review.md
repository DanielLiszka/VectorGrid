# Final Consolidated Review: `crates/app_core/src/meta_select.rs`

- Review date: `2026-04-16`
- Source file: [meta_select.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/meta_select.rs:1>)
- First-pass packet: [meta_select.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/meta_select.rs.review.md:1>)
- Second-pass packet: [meta_select.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/meta_select.rs.review.md:1>)
- Third-pass packet: [meta_select.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/meta_select.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/027__crates__app_core__src__meta_select.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/collections/struct.HashMap.html>
  - <https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by>
  - <https://www.rfc-editor.org/rfc/rfc8785>

## Consolidated Judgment

The fourth-pass packet is the right governing review for this file. The earlier passes were correct to keep the bucket small and to defend tolerant compatibility behavior, but the fourth pass identified the one concrete low-risk defect that should not stay deferred: equal-score ordering is currently nondeterministic because aggregates are built from `HashMap` iteration and then stably sorted only by `meta_score`.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live source and is stronger than the earlier generic “heuristic surface” framing.

## Safe-Now Priority

- Stabilize equal-score ordering in the final aggregate sort:
  - keep `meta_score` as the primary key
  - add a deterministic tie-breaker such as `key`
  - add a regression test proving stable tie ordering and reproducible `top_k` truncation at the boundary
- Add rustdoc on the public contract:
  - `MetaSelectionRequest`: train-stage selector versus holdout-stage augmentation ownership
  - `MetaCandidateAggregate` and `MetaSelectionResult`: `final_score` and `holdout` are stage-2 augmentation outputs, not train-stage outputs from `meta_select_from_artifacts`
  - `meta_select_from_artifacts`: direct callers should not expect populated holdout/final scores
- Clarify the meaning of public counters without renaming them:
  - `artifacts_used` is not identical to “contributed to final groups”
  - `candidates_considered` is post-dedupe within an artifact
- If the team intends to preserve current clustered same-dataset collapse semantics, add a test or comment that says the current rule is effectively “first surviving candidate wins” within a dataset/group key.

## Deferred Work

- Do not change the `pass_rate` denominator in the same patch; it is ranking-policy work.
- Do not redesign clustered representative selection, dataset identity fallback, or canonical key formation in the same pass.
- Do not split request/result types or add public debug counters as part of this small review batch.
- Do not treat the literal `·` separator in a warning string as an objective encoding bug worth implementing around.

## Verification If Changed

- `cargo test -p ta_app_core meta_select::tests:: -- --nocapture`
- `cargo bench -p ta_app_core --bench meta_select_workflow --no-run`
- If ordering or public payload semantics change:
  - `cargo test --workspace --no-run`
  - any ordering-sensitive command/report tests that already exist in the repo

## Regression And Feature-Preservation Constraints

- Preserve current `meta_group_key*` and `canonical_json` semantics in a small pass.
- Do not silently change the meaning of `pass_rate`, `artifacts_used`, or `candidates_considered`.
- Do not rename or repurpose holdout-related request/result fields in a file-local change.
- Treat deterministic tie-breaking as an intentional output stabilization and update any ordering-sensitive tests accordingly.

## Final Conclusion

Keep `meta_select.rs` in `Small surgical refactor`, treat the fourth pass as valid, and implement the deterministic-ordering plus contract-docs lane first. Denominator changes, dataset-identity changes, and request/result reshaping stay deferred.
