# Second-Pass Review: `src-tauri/src/options_parent_adapters.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/options_parent_adapters.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/options_parent_adapters.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a replay-provenance-sensitive adapter layer whose supported family set and replay defaults should remain explicit.

## Stale Check

The live source still owns:

- parent-artifact replay adaptation into options expression input
- supported strategy-family filtering
- replay identity defaults like `artifact_replay` and fixed bin count

The first-pass packet remains current.

## Second-Pass Research Log

1. `serde_json::Value`
   - Source: <https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html>
   - Why consulted in second pass:
     - verify fallback extraction behavior from saved artifacts
   - Additional constraint informed:
     - replay compatibility still depends on permissive JSON extraction order

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in second pass:
     - confirm partial-deserialization expectations from parent artifacts
   - Additional constraint informed:
     - the supported-family contract is stricter than "similar shape"

3. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - confirm why failures here are still desktop-facing contract defects
   - Additional constraint informed:
     - replay-adapter behavior is part of public workflow semantics

## Delta vs First Pass

### 1. Strategy-kind resolution here is still exact-string only, unlike other normalizers in the repo

- Why this matters:
  - legacy aliases or spelling drift can make structurally compatible artifacts unreplayable
- Practical implication:
  - if replay support grows, alias policy should be handled intentionally rather than by accident

### 2. `artifact_replay` and `bins: Some(256)` remain replay identity/shape contracts, not incidental defaults

- Practical implication:
  - preserve them unless options-expression replay policy intentionally changes

## Verification If Changed

- `cargo test -p ta_desktop_demo_app options_parent_strategy_kind_support_matches_current_double_like_scope -- --nocapture`
- `cargo test -p ta_desktop_demo_app saved_artifact_replay_provenance -- --nocapture`
- `cargo test -p ta_app_core manifest_projection_uses_previous_bar_for_decision_when_trade_is_next_bar -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_options_expression -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve the supported family set until the replay contract intentionally expands
- Preserve candidate-selection order and `trade_on_next_bar` semantics
- Preserve replay identity defaults unless a migration plan exists

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that alias handling and replay-default values here are real compatibility policy, not generic adapter cleanup.
