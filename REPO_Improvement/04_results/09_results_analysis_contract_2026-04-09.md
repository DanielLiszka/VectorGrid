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

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because this is a classic shared-state integration surface that should remain centralized.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because the analysis store is a canonical cross-panel store rather than a view-local cache.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the analysis contract is primarily a deterministic state-mapping and action-contract surface.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because many shared analysis fields originate from backend result payloads or replay artifacts crossing the command boundary.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because this contract effectively behaves like a structured frontend schema over hydrated run data.

## Proposed Improvement Approach

1. Freeze the analysis contract as a shared result-state layer with explicit categories: winner framing, shortlist state, compare state, chart state, and status/advisory state.
2. Document which analysis fields are stable panel-consumption fields versus transient render internals.
3. Align live-run hydration and saved-run hydration through the same contract expectations wherever possible.
4. Treat analysis-service tests as contract tests, not just implementation tests.
5. Add or tighten parity checks whenever a result panel or workflow-hydration path changes.

## Checklist

- [ ] Document the shared analysis-state categories and their consumers.
- [ ] Document stable panel-facing fields versus render-internal state.
- [ ] Document live-run versus saved-run hydration expectations.
- [ ] Document comparison, drill, pareto, and heatmap state as part of one shared contract.
- [ ] Add or tighten tests whenever analysis-store or analysis-service field semantics change.

## Verification Plan

- run `npm --prefix web test -- src/services/analysisService.test.ts`
- run `npm --prefix web test -- src/components/OverviewWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/TopWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/AnalysisContextDock.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current analysis-store field names and meanings until all known consumers migrate together
- prefer additive fields over repurposing existing shared state
- if a panel needs a new summary, add it through the shared analysis contract before introducing panel-local derivations
- update saved-run hydration and live-run hydration together when shared state semantics change

## Related Docs

- `REPO_Improvement/04_results/01_results_overview_2026-04-08.md`
- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/18_context_docks_2026-04-08.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
