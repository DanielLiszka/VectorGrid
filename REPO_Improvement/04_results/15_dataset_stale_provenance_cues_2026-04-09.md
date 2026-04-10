# Dataset Stale And Provenance Cues Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to dataset provenance and stale-result cues so users can see when ranked candidates, pinned comparisons, or result artifacts are no longer aligned with the active shortlist or current dataset context.

## Scope Boundaries

In scope:

- selected-top provenance chips
- pinned-candidate stale cues and shortlist guardrails
- dataset and artifact metadata used by result surfaces
- context-dock cues tied to result freshness and provenance

Out of scope:

- low-level dataset fingerprint computation
- report path persistence outside result-surface implications
- options-related provenance cues

## Current User-Facing Behavior

The current analysis layer already surfaces provenance and staleness in several ways. The selected Top candidate gets provenance chips, pinned compare candidates can carry stale reasons, the Top panel exposes stale cues and selection guardrails, and workflow artifacts carry dataset metadata and artifact identity alongside run results.

This is important because shortlist rows, pinned candidates, and saved artifacts can diverge for legitimate reasons: filters can hide the previously selected row, the current shortlist can change, or a saved artifact can come from a different dataset context than the currently active one.

The repo already has some of the right primitives. What is missing is a dedicated plan that freezes which cues are mandatory and how they should remain understandable.

## Current Code Surface

- `web/src/services/analysisService.ts`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/AnalysisContextDock.vue`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/historyService.ts`

## Current Verification Surface

- `web/src/services/analysisService.test.ts`
- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/stores/workflowArtifactsStore.test.ts`
- `web/src/services/historyService.test.ts`

## Strengths Already Present

- pinned candidates already carry explicit stale reasons
- selected candidates already surface provenance chips rather than only raw metric text
- Top selection guardrails already explain when focus moved because the prior row disappeared
- workflow artifacts already normalize dataset and artifact metadata
- context docks already expose status and current-result context outside the main result panes

## Weaknesses / Failure Modes

- provenance cues are distributed across chips, guardrails, stale-cue lists, and artifact metadata
- some stale conditions are shortlist-local while others are artifact- or dataset-level, and those can blur together
- provenance cues can be treated as optional decoration in UI refactors
- the active current-run context can drift from saved baseline or pinned compare context if cues weaken
- there is no single implementation plan freezing stale/provenance expectations across the results workflow

## Regressions To Avoid

- dropping pinned-candidate stale reasons
- hiding selection guardrails when the active row changed because of filters or shortlist changes
- weakening provenance chips so users lose quick context on what they are reviewing
- losing normalized dataset or artifact metadata that downstream cues depend on
- treating stale-result warnings as cosmetic instead of decision-relevant
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because provenance and stale cues depend on shared workflow and analysis state, not one component’s local view.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because the same stale/provenance facts are consumed by multiple panels and docks.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because stale-cue correctness is mostly deterministic reconciliation behavior.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because dataset meta and artifact payloads feeding these cues behave like structured contracts.
- [WAI-ARIA Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because provenance cues need to survive as users move across result tabs rather than living in only one panel.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Separate shortlist staleness, compare staleness, and artifact/dataset provenance into explicit cue families.
2. Freeze selected-candidate chips, pinned-candidate stale reasons, and Top selection guardrails as mandatory decision cues.
3. Keep dataset and artifact metadata normalized in workflow-artifact state so result surfaces do not infer provenance heuristically.
4. Add or tighten tests for stale-candidate reconciliation and artifact-metadata normalization.
5. Ensure context docks reinforce rather than contradict the main result panes.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document cue families: provenance chips, stale reasons, guardrails, and artifact metadata.
- [x] Document shortlist-filter and selection-loss stale behavior.
- [x] Document saved-run and artifact dataset provenance expectations.
- [x] Document dock and panel consistency expectations for stale/provenance cues.
- [x] Add or tighten tests for reconciliation and metadata normalization.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/analysisService.test.ts`
- run `npm --prefix web run test -- src/components/TopWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/workflowArtifactsStore.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification

## Rollout / Fallback / Compatibility Notes

- preserve current stale-cue semantics until all compare and result consumers migrate together
- prefer additive provenance detail over removing existing cues
- keep artifact metadata normalization backward-safe while UI cues are clarified
- if provenance cues later move into a dedicated summary surface, keep the existing Top and Overview cues during the transition
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated stale/provenance behavior through [analysisService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/analysisService.test.ts), [TopWorkspacePanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/TopWorkspacePanel.test.ts), [workflowArtifactsStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/workflowArtifactsStore.test.ts), and [historyService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/historyService.test.ts).
- The current evidence confirms selected-candidate provenance chips, pinned-candidate stale reasons, shortlist guardrails, and normalized artifact metadata are all still present and mutually consistent.
- No production code change was required in this tranche because the current stale/provenance cue families already satisfy the documented contract at the present scope.

## Related Docs

- `REPO_Improvement/04_results/03_results_top_compare_2026-04-09.md`
- `REPO_Improvement/04_results/11_out_top_results_table_2026-04-09.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/02_data/22_dataset_provenance_2026-04-08.md`
