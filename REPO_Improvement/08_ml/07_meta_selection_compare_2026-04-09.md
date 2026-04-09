# Meta Selection Compare Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the Meta Selection and Compare surfaces so grouped results, compare targets, and saved-artifact comparisons remain explicit, stable, and easy to reason about.

## Scope Boundaries

In scope:

- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/metaService.ts`
- `web/src/stores/analysisStore.ts`
- compare selections, compare summaries, meta table rendering, saved-meta selection, and compare-target loading

Out of scope:

- changes to the underlying meta-selection algorithm
- stage5/policy-eval execution internals beyond how compare surfaces present them
- options-related compare flows

## Current User-Facing Behavior

The meta workspace already supports comparison-oriented review:

- Meta Selection renders grouped rows and summary tables
- compare controls allow a selected compare target
- saved meta and saved stage5 artifacts can be loaded for deeper review
- compare summaries coexist with the main meta outputs rather than living in a separate tool

This is powerful, but comparison behavior can feel implicit if the user does not know which artifact family or row selection is currently driving the summary.

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/metaService.ts`
- `web/src/stores/analysisStore.ts`
- related tests:
  - `web/src/components/MetaWorkspacePanel.test.ts`
  - `web/src/services/metaService.test.ts`

## Current Verification Surface

- `MetaWorkspacePanel.test.ts`
- `metaService.test.ts`
- `metaServiceCampaignResume.test.ts`
- `metaServiceCampaignStrategy.test.ts`

## Strengths Already Present

- compare behavior is already part of the main meta workspace instead of being hidden in a side tool
- meta table rendering and compare summaries are already structured outputs
- saved artifact families already provide concrete comparison sources
- compare state is routed through the analysis/meta service layer rather than only local component state

## Weaknesses / Failure Modes

- compare intent can be obscured when selection context is not obvious
- saved meta, saved stage5, and live meta outputs can overlap conceptually without strong labeling
- compare summaries can become detached from the row or artifact source that produced them
- the user can end up reading a dense summary without clear provenance

## Regressions To Avoid

- changing selection/compare behavior in ways that detach summaries from their source
- making compare controls less discoverable while adding more saved-artifact complexity
- letting compare outputs reuse labels that sound similar but refer to different artifact families
- weakening table/summary parity
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because compare state is shared across the workspace and cannot be treated as a purely local widget concern.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because compare behavior crosses component, store, and service boundaries.
- [WAI-ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/) are relevant because compare flows depend on clear stateful selection in tabs, tables, and menus.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because compare summaries are reading structured artifact families and should keep source/provenance fields legible.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because much of the meta compare state originates from structured JSON payloads emitted by Rust commands.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Treat compare as a provenance-sensitive review workflow.
2. Document every compare source:
   - live meta output
   - saved meta
   - saved stage5
   - saved policy evaluation
3. Ensure the UI always makes the current compare source obvious.
4. Tighten parity between selected source, summary labels, and rendered compare output.
5. Add tests that follow end-to-end compare flows rather than only checking isolated state fields.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document compare-source provenance for live and saved meta artifact families.
- [ ] Audit compare labels and summaries for source clarity.
- [ ] Add or tighten tests for compare selection changes and summary refresh behavior.
- [ ] Add or tighten tests for loading saved artifact families into compare workflows.
- [ ] Ensure compare outputs never lose the label/path/source context that produced them.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `MetaWorkspacePanel.test.ts`
- run `metaService.test.ts`
- run relevant saved-artifact compare tests in `metaServiceCampaignResume.test.ts` and `metaServiceCampaignStrategy.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve existing compare capability while improving source clarity
- prefer clearer provenance and labels over more compare modes
- if a compare cleanup obscures source context, revert and keep the more explicit current state
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/06_meta_panel_ia_2026-04-09.md`
- `REPO_Improvement/08_ml/09_meta_saved_artifact_families_2026-04-09.md`
- `REPO_Improvement/06_artifacts/17_report_meta_workbook_2026-04-09.md`
- `REPO_Improvement/04_results/03_results_top_compare_2026-04-09.md`
