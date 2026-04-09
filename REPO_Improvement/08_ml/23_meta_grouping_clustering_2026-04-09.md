# Meta Grouping And Clustering Plan

Last updated: 2026-04-09

## Purpose

Define the current Stage 3 grouping and clustering contract so exact-versus-clustered behavior, bucket defaults, representative selection, saved counts, and workbook/report expectations remain explicit and stable.

## Scope Boundaries

In scope:

- Stage 3 grouping mode semantics
- cluster bucket configuration and representative selection
- frontend request defaults and clamps
- backend grouping behavior and saved result counts
- report/artifact implications of grouping changes

Out of scope:

- Stage 3 scoring weights beyond how grouped candidates are fed into scoring
- Stage 4 / Stage 5 ML blending
- batch campaign execution beyond how grouped outputs are saved and displayed
- non-meta grouping logic elsewhere in the repo

## Current User-Facing Behavior

The Meta workspace exposes grouping controls alongside holdout controls. Users can choose `Clustered (recommended)` or `Exact (strict)`, tune integer and float bucket sizes, and choose the number of holdout representatives per cluster. Current behavior includes:

- clustered mode is the default
- exact mode preserves stricter per-parameter grouping boundaries
- representative count is presented as a bounded small choice rather than an open-ended field
- grouping directly affects how many candidate groups are surfaced, scored, and saved
- grouping counts and downstream summaries are reflected in meta results, saved artifacts, and reports

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - current grouping controls, labels, and user-facing copy
- `web/src/stores/analysisStore.ts`
  - persisted grouping inputs and defaults
- `web/src/services/metaRequestBuilder.ts`
  - request shaping for `grouping_mode`, `cluster_config`, and `cluster_representatives`
- `web/src/services/metaService.ts`
  - Stage 3 orchestration and summary propagation
- `src-tauri/src/commands_meta.rs`
  - Stage 3 orchestration and saved result-count surfaces
- `crates/app_core/src/meta_select.rs`
  - `MetaGroupingMode`
  - `MetaClusterConfig`
  - `pick_representatives()`
  - grouped key construction and representative clamping
- `src-tauri/src/run_artifacts.rs`
  - saved artifact counts and grouped-result payload persistence
- `src-tauri/src/commands_reports_meta_xlsx.rs`
  - workbook/report rendering of grouped meta outputs

## Current Verification Surface

- `web/src/services/metaRequestBuilder.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/stores/domainStores.test.ts`
- tests in `crates/app_core/src/meta_select.rs`
  - clustered versus exact grouping behavior
  - representative-signature reuse
- Stage 3 tests in `src-tauri/src/commands_meta.rs`
- report/export tests in `src-tauri/src/commands_reports_meta_xlsx.rs`

## Strengths Already Present

- grouping behavior is already owned by one backend module instead of being half-implemented in the frontend
- frontend defaults align with the current backend defaults
- grouped counts are already carried through saved results rather than being transient UI-only state
- representative count is deliberately clamped, which keeps grouped validation bounded
- tests already cover clustered collapse and representative behavior

## Weaknesses / Failure Modes

- grouping looks like a simple UX preference, but it materially changes saved counts and downstream ranking inputs
- frontend copy and backend behavior can drift even when the visible controls look unchanged
- invalid or weak bucket inputs can silently change grouping aggressiveness if their semantics are not documented precisely
- report and artifact consumers can misread grouped counts if grouping-mode context is not preserved alongside results
- Stage 3 grouping can be confused with later Stage 4 / Stage 5 selection behavior if docs do not distinguish them clearly

## Regressions To Avoid

- changing default grouping mode without updating builder defaults, store defaults, tests, and UI copy together
- widening representative counts beyond the current clamp without measuring downstream cost and report impact
- silently altering bucket quantization semantics
- making saved grouped-count fields ambiguous or inconsistent across service, artifact, and report layers
- documenting grouping as a frontend-only concern when the real behavior is backend-owned

## External Research Summary

- [scikit-learn Clustering User Guide](https://scikit-learn.org/stable/modules/clustering.html)
  - relevant because clustered grouping is fundamentally a coarse partitioning policy, and the documentation reinforces that clustering choices materially affect downstream structure
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because the Stage 3 request and saved grouped outputs behave like contract-bearing JSON payloads
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because grouping controls are frontend-owned inputs whose authoritative behavior is implemented in Rust
- [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because grouped meta outputs and artifacts are persisted as JSON and must remain shape-stable across replay/report paths

## Proposed Improvement Approach

1. Document exact and clustered grouping as distinct backend semantics rather than superficial UI labels.
2. Record the current defaults and clamps:
   - clustered by default
   - `int_bucket = 5`
   - `float_bucket = 0.25`
   - representatives clamped to `1..=3`
3. Explain how representative selection works and why the small clamp exists.
4. Tie grouping choices to concrete downstream effects:
   - groups considered
   - groups scored
   - groups selected
   - artifact/report counts
5. Keep frontend/store defaults and backend defaults synchronized in one documented contract.
6. Make the artifact/report implications explicit so grouping changes are treated as behavior changes, not copy changes.

## Checklist

- [ ] Document `Exact` versus `Clustered` semantics and current default mode.
- [ ] Document `MetaClusterConfig` defaults and normalization rules.
- [ ] Document the current representative-count clamp and its rationale.
- [ ] Document where grouped counts are emitted and persisted.
- [ ] Document how grouping affects workbook/report interpretation.
- [ ] Document frontend/store defaults versus backend defaults.
- [ ] Document current UX labels and why they should stay aligned with backend behavior.
- [ ] Add or maintain tests for exact-vs-clustered collapse behavior.
- [ ] Add or maintain tests for representative selection and persisted group counts.
- [ ] Preserve compatibility for older saved artifacts that already depend on the current grouped-output shape.

## Verification Plan

- run `npm --prefix web test -- src/services/metaRequestBuilder.test.ts`
- run `npm --prefix web test -- src/services/metaService.test.ts`
- run `npm --prefix web test -- src/stores/domainStores.test.ts`
- run targeted grouping tests in `crates/app_core/src/meta_select.rs`
- run targeted Stage 3/report tests in `src-tauri/src/commands_meta.rs` and `src-tauri/src/commands_reports_meta_xlsx.rs`
- manually compare one exact-mode and one clustered-mode run on the same artifact set

## Rollout / Fallback / Compatibility Notes

- preserve current defaults and clamp ranges until a deliberate product decision changes them
- if grouping semantics change, update report wording and saved-count documentation in the same change
- keep older grouped artifacts replayable even if future grouping metadata becomes richer
- prefer additive metadata over replacing current grouped-count fields

## Related Docs

- `REPO_Improvement/08_ml/06_meta_panel_ia_2026-04-09.md`
- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
- `REPO_Improvement/08_ml/24_meta_scoring_weights_2026-04-09.md`
- `REPO_Improvement/08_ml/25_meta_evidence_flags_2026-04-09.md`
- `REPO_Improvement/06_artifacts/17_report_meta_workbook_DATE.md`
