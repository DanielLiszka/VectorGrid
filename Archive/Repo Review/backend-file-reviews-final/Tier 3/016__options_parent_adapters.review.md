# Final Consolidated Review: `src-tauri/src/options_parent_adapters.rs`

- Review date: `2026-04-16`
- Source file: [options_parent_adapters.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/options_parent_adapters.rs:1>)
- First-pass packet: [options_parent_adapters.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/options_parent_adapters.rs.review.md:1>)
- Second-pass packet: [options_parent_adapters.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/options_parent_adapters.rs.review.md:1>)
- Third-pass packet: [options_parent_adapters.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/options_parent_adapters.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 3/016__src-tauri__src__options_parent_adapters.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://serde.rs/container-attrs.html>
  - <https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html>

## Consolidated Judgment

The fourth-pass packet is valid and correctly recenters the file. [options_parent_adapters.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/options_parent_adapters.rs:1>) is not a public command seam. It is a persisted-artifact replay adapter that:

- classifies supported parent families;
- reconstructs double-like replay context;
- builds options signal manifests from saved artifacts.

The strongest missed risk from earlier packets is schema coupling:

- [build_double_like_parent_replay](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/options_parent_adapters.rs:79>) deserializes full shared DTOs (`DoubleMaRequest`, `DoubleMaValidationParams`, `DoubleMaParamsResolved`) even though it only consumes a subset of their fields.
- That means future additive required fields in those shared structs can make older saved artifacts unreplayable here even when the new fields are irrelevant to this adapter.

The other strong fourth-pass correction is diagnostic clarity:

- missing strategy kind and unsupported strategy kind currently collapse into the same generic failure path;
- `supported_options_parent_family_from_artifact` is a kind-allowlist prefilter, not a full replay-compatibility proof;
- explicit `parent_candidate_index` handling is correctly fail-closed and should stay that way.

## Safe-Now Priority

- Keep the bucket at `Small surgical refactor`.
- Add targeted adapter tests for:
  - missing strategy kind vs unsupported strategy kind;
  - explicit nonzero `parent_candidate_index`;
  - explicit missing candidate failing closed;
  - implicit candidate-0 absence falling back to `run.res.best_params`;
  - candidate payload present but malformed, with no silent fallback.
- Improve diagnostics so “artifact missing usable strategy-kind metadata” is distinct from “artifact strategy kind unsupported”.
- Add short comments clarifying:
  - kind allowlist classification is not replay-proof;
  - explicit candidate index is fail-closed;
  - malformed candidate payload does not fall back.

## Deferred Work

- Defer any narrower local replay DTO until the source Serde contracts are audited.
- Defer any bigger redesign that unifies replay identity instead of passing `family`, `trade_on_next_bar`, and derived request state separately.
- Do not loosen exact strategy-kind matching without an explicit artifact migration plan.

## Verification If Changed

- Keep the focused adapter/replay tests in scope:
  - `cargo test -p ta_desktop_demo_app options_parent_strategy_kind_support_matches_current_double_like_scope -- --nocapture`
  - `cargo test -p ta_desktop_demo_app saved_artifact_replay_provenance -- --nocapture`
- If replay request shape changes, also keep the downstream drilldown/options-expression smoke tests in scope.

## Regression And Feature-Preservation Constraints

- Preserve the supported parent-family set unless product scope changes deliberately.
- Preserve exact-match strategy-kind handling for persisted internal metadata.
- Preserve fail-closed explicit candidate selection.
- Preserve `artifact_replay` and `bins: Some(256)` unless downstream consumers migrate intentionally.

## Final Conclusion

The fourth-pass packet is valid, no wrong fourth-pass file is present, and the final bucket remains `Small surgical refactor`. The safe implementation target is tighter diagnostics plus focused replay-contract tests; the more interesting schema-coupling redesign remains deferred.
