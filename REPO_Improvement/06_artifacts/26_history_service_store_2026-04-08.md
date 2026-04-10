# History Service Store Plan

Last updated: 2026-04-09

## Purpose

Define the current ownership split between the history store and history service so refresh, detail, replay, and artifact utility flows can be improved without breaking the current session/saved-run model.

## Scope Boundaries

In scope:

- session-history versus saved-run separation
- history store state ownership
- history service orchestration for refresh/load/apply/rerun/open/reveal/delete/tag-save
- detail caching and runtime gating

Out of scope:

- redesigning artifact formats
- options-specific history behavior
- replacing the saved-run library with a different storage backend

## Current User-Facing Behavior

The history workspace already supports two different surfaces:

- session history
- saved runs

Current behavior is richer than a flat log:

- saved-run detail is loaded lazily by path and cached
- saved-run refresh prunes stale detail cache entries
- mutating actions are disabled while runtime activity is active
- saved-run actions include load, apply, rerun, open report, reveal file, delete artifact, and tag save
- ML-signal-specific replay branches already exist
- report opening already falls back from direct open to reveal behavior where needed

So the current history system is already an orchestration layer, not just a static list.

## Current Code Surface

- `web/src/stores/historyStore.ts`
  - owns state, visible lists, search sync, selected-detail view, cache pruning, and action dispatch
- `web/src/services/historyService.ts`
  - owns refresh/load/apply/rerun/open/reveal/delete/tag-save flows and cross-store orchestration
- `web/src/stores/workflowArtifactsStore.ts`
  - receives hydrated artifact state when history actions load or apply runs
- `web/src/services/historyService.test.ts`
  - verifies service behavior, artifact hydration, tags, detail normalization, and replay branches
- `web/src/stores/historyStore.actions.test.ts`
  - verifies store-to-service dispatch and state behavior

## Current Verification Surface

- `web/src/services/historyService.test.ts`
- `web/src/stores/historyStore.actions.test.ts`
- `web/src/stores/historyStore.search.test.ts`
- `web/src/components/HistoryWorkspacePanel.test.ts`

## Strengths Already Present

- the store/service ownership split already exists and is practical
- detail loading is lazy and cached rather than eager and expensive
- runtime gating already prevents risky mutating actions during active work
- ML-signal-specific replay handling is already accounted for
- artifact JSON paths and XLSX report paths are already treated as different concepts

## Weaknesses / Failure Modes

- the service owns several distinct responsibility classes that are not documented separately
- detail cache behavior can be easy to regress subtly during refresh/delete changes
- replay/apply/load differences are meaningful but currently implicit
- there is no one document that explains which layer owns listing, detail normalization, and utility actions
- store/service typing around admission and metric-surface detail is still easy to drift

## Regressions To Avoid

- breaking load, apply, or rerun semantics while refactoring shared code
- causing unnecessary detail-fetch churn
- losing ML-signal replay branching
- conflating artifact JSON paths with report XLSX paths
- weakening runtime gating that currently protects mutating actions
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the current store/service split depends on one clear state authority
- [Pinia actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because user actions should remain store-driven even when orchestration lives in services
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because store/service seams should remain directly testable with mocked service calls
- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because many history actions ultimately bridge into Tauri commands and file operations
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because artifacts, reports, manifests, and replay payloads behave like versioned serialized contracts that should stay explicit and reviewable

## Proposed Improvement Approach

1. Document the four responsibility layers explicitly:
   - list state
   - detail state
   - replay/load/apply orchestration
   - file/report utility actions
2. Preserve the current session-history versus saved-run separation.
3. Tighten contracts for cached saved-run detail, especially around:
   - metric surface
   - validation admission
   - ML-signal detail
4. Add focused regression coverage for:
   - cache pruning on refresh/delete
   - runtime gating
   - ML replay branching
   - report-path versus artifact-path behavior
5. Keep the store as the visible-state owner and the service as the orchestration owner.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document store ownership versus service ownership clearly.
- [x] Document saved-run detail caching and pruning behavior.
- [x] Document load/apply/rerun/open/reveal/delete/tag-save semantics separately.
- [x] Document ML-signal-specific replay branches and report fallback behavior.
- [x] Add or tighten cache-hit, cache-prune, and runtime-gating regressions.
- [x] Preserve path-type distinctions between artifacts and reports.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run `npm --prefix web run test -- src/stores/historyStore.actions.test.ts`
- run `npm --prefix web run test -- src/stores/historyStore.search.test.ts`
- manually verify refresh, detail load, load/apply/rerun, open report, reveal, delete, and tag-save flows
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- keep lazy detail loading unless a proven better caching strategy exists
- preserve current runtime gating for mutating actions
- prefer explicit responsibility extraction over merging service logic into the store
- if types are tightened later, keep legacy artifact detail normalization until saved-run compatibility is proven
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
- `REPO_Improvement/06_artifacts/10_history_search_filter_ux_2026-04-08.md`
- `REPO_Improvement/06_artifacts/09_workflow_artifact_store_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`

## Execution Notes

- Closed as a store/service ownership hardening slice without merging orchestration back into the history store.
- Added explicit store regressions in `web/src/stores/historyStore.actions.test.ts` for runtime-running gating and stale-detail-cache pruning, and added service-level report fallback coverage in `web/src/services/historyService.test.ts`.
- The current split remains: visible state, selection, filtering, and gating in `historyStore`; refresh/load/apply/rerun/open/reveal/delete/tag-save orchestration in `historyService`.
- Acceptance evidence for closure: `npm.cmd --prefix web run test -- src/stores/historyStore.actions.test.ts src/services/historyService.test.ts src/stores/historyStore.search.test.ts src/components/HistoryWorkspacePanel.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

