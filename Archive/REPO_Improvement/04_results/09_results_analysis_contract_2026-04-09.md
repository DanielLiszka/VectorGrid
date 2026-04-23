# Results Analysis Contract Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the shared analysis contract so result panels, context docks, workflow hydration, and saved-run comparisons all read from a stable, truthful analysis-state surface instead of diverging panel-local interpretations.

## Scope Boundaries

In scope:

- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- analysis-facing result fields such as best summary, top rows, pareto meta, drill status, and heatmap state
- workflow-run hydration and saved-run comparison data that feed analysis state

Out of scope:

- implementation details of each individual panel layout
- validation workbook/report contracts
- options-related analysis surfaces

## Current User-Facing Behavior

The result panels already depend on a broad shared analysis contract. Overview consumes winner framing, best stats, comparison summary, and export readiness. Top consumes columns, rows, filters, selected-candidate details, pinned candidates, stale cues, and guardrails. Pareto, Drilldown, and Heatmap all consume dedicated meta, status, and rendering-state fields from the same analysis store.

The analysis service is also responsible for rehydrating this state from workflow runs and saved artifacts, building summaries, computing comparison text, handling drill/heatmap/pareto refresh, and keeping chart-heavy tabs in sync with the currently selected run and result row.

That makes the analysis contract one of the most important shared truth surfaces in the frontend. If it drifts, every result tab can still render while quietly telling a different story.

## Current Code Surface

- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/AnalysisContextDock.vue`
- `web/src/stores/historyStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`

## Current Verification Surface

- `web/src/services/analysisService.test.ts`
- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/components/AnalysisContextDock.test.ts`
- workflow and hydration tests that feed analysis state

## Strengths Already Present

- the repo already centralizes most result interpretation in one service/store pair
- the same state already feeds multiple result tabs and the analysis dock
- saved-run comparison and live workflow-run hydration already converge into a shared contract
- analysis-service tests already cover drill, heatmap, comparison, and summary behavior
- the store already exposes panel-ready state instead of forcing each component to reconstruct it

## Weaknesses / Failure Modes

- the shared contract is large enough that silent field drift is easy
- result panels can over-assume field semantics if the contract is not explicitly documented
- hydration, comparison, and chart-refresh behavior are coupled tightly enough that regressions may surface far from the changed code
- some fields are summary-level while others are render-state-level, and that boundary is easy to blur
- there is no dedicated implementation doc freezing stable shared analysis semantics

## Regressions To Avoid

- changing analysis-store field meaning without synchronized panel updates
- letting saved-run hydration and live-run hydration produce materially different result narratives
- dropping summary or status fields that downstream panels already rely on
- treating panel-local fallback text as a substitute for fixing broken shared state
- weakening analysis-service tests when changing summary, compare, drill, or heatmap behavior
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because this is a classic shared-state integration surface that should remain centralized.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because the analysis store is a canonical cross-panel store rather than a view-local cache.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the analysis contract is primarily a deterministic state-mapping and action-contract surface.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because many shared analysis fields originate from backend result payloads or replay artifacts crossing the command boundary.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because this contract effectively behaves like a structured frontend schema over hydrated run data.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Freeze the analysis contract as a shared result-state layer with explicit categories: winner framing, shortlist state, compare state, chart state, and status/advisory state.
2. Document which analysis fields are stable panel-consumption fields versus transient render internals.
3. Align live-run hydration and saved-run hydration through the same contract expectations wherever possible.
4. Treat analysis-service tests as contract tests, not just implementation tests.
5. Add or tighten parity checks whenever a result panel or workflow-hydration path changes.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the shared analysis-state categories and their consumers.
- [x] Document stable panel-facing fields versus render-internal state.
- [x] Document live-run versus saved-run hydration expectations.
- [x] Document comparison, drill, pareto, and heatmap state as part of one shared contract.
- [x] Add or tighten tests whenever analysis-store or analysis-service field semantics change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/analysisService.test.ts`
- run `npm --prefix web run test -- src/components/OverviewWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/TopWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/AnalysisContextDock.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current analysis-store field names and meanings until all known consumers migrate together
- prefer additive fields over repurposing existing shared state
- if a panel needs a new summary, add it through the shared analysis contract before introducing panel-local derivations
- update saved-run hydration and live-run hydration together when shared state semantics change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the shared analysis contract through [analysisService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/analysisService.test.ts), [analysisStore.actions.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/analysisStore.actions.test.ts), and the major consuming panel tests.
- The new Drilldown and Heatmap component tests added in this tranche increase coverage around panel consumption of the same shared contract without widening the runtime blast radius.
- The main remaining gap is not panel drift but missing Tauri integration coverage for the analysis commands feeding this contract.

## Related Docs

- `REPO_Improvement/04_results/01_results_overview_2026-04-08.md`
- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/18_context_docks_2026-04-08.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
