# History Library Core Plan

Last updated: 2026-04-09

## Purpose

Define the current non-options history-library core so session history, saved-run browsing, saved-run detail, apply/load/rerun flows, and saved-run tags/search behavior can be improved without breaking the run library as a practical reopen-and-reuse surface.

## Scope Boundaries

In scope:

- `web/src/stores/historyStore.ts`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`
- saved-run search, detail, tags, load/apply/rerun, report/reveal/delete flows

Out of scope:

- options expression library behavior
- Rust artifact deletion/listing implementation details
- new artifact families that are not already part of the normal non-options run library

## Current User-Facing Behavior

The run library is already a real workflow surface. It shows session runs, saved runs, filtered saved-run results, detail summaries, report/artifact paths, validation summaries, ML-related detail fields when present, tag editing, load/apply/rerun actions, open/reveal actions, delete, and runs-folder opening. The store also supports ranked saved-run search behavior, compare-run selection, and detail caching so the library is already more than a raw file list. Underneath, the history layer is doing three jobs at once: session history from local storage, saved-run library from Tauri, and replay/apply/rerun/preset orchestration.

## Current Code Surface

- `web/src/components/HistoryWorkspacePanel.vue`
  - run-library UI, search/tags drafts, confirmation flow, and action routing
- `web/src/stores/historyStore.ts`
  - library state, saved-run search, selection, detail state, and lazy service dispatch
- `web/src/stores/historyStore.actions.test.ts`
  - store action behavior, filtering, selection, and tag/update flows
- `web/src/stores/historyStore.search.test.ts`
  - saved-run search ranking behavior
- `web/src/services/historyService.ts`
  - artifact listing/detail/loading/apply/rerun/open/reveal/delete behavior
- `web/src/services/historyService.test.ts`
  - service-level run-library behavior coverage
- `src-tauri/src/run_artifacts.rs`
  - upstream run-artifact summary/detail source

## Current Verification Surface

- `web/src/stores/historyStore.actions.test.ts`
- `web/src/stores/historyStore.search.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/components/HistoryWorkspacePanel.test.ts`

## Strengths Already Present

- the library already separates session history from saved artifact history
- saved-run search already uses structured ranking rather than naive substring only
- detail data is cached and normalized instead of being re-fetched blindly every render
- run-library actions already map back into real workflow state restoration paths
- tags, validation summaries, and ML-related details already make the library more than a filename browser

## Weaknesses / Failure Modes

- history state and service behavior are broad enough that ownership is easy to blur
- the saved-run TypeScript summary is thinner than the richer Rust summary/detail surface
- the library mixes artifact management, workflow rehydration, and preset export/import utilities
- there is real compatibility debt around stage4/stage5 naming and nearby non-run artifact-library flows
- there is a real non-options scope boundary here because nearby UI code also hosts options-related surfaces

## Regressions To Avoid

- breaking saved-run search ranking and token behavior
- losing detail caching or making library actions overly chatty
- breaking load/apply/rerun semantics for normal saved runs
- dropping tag editing or run-directory/report open/reveal flows
- breaking ML-signal replay branching or current runtime-running action gating
- letting options-related concerns distort the non-options history-library contract
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because run-library state is already shared domain state
- [Pinia: Testing](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because current library behavior is heavily protected at store/service level
- [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because saved-run artifact detail is JSON-backed state
- [JSON Schema: What Is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because run-library detail payloads behave like structured contracts, not free-form blobs

## Proposed Improvement Approach

1. Document the library in clear subdomains:
   - session history
   - saved-run listing/search
   - saved-run detail cache
   - load/apply/rerun
   - report/reveal/delete utilities
2. Make the non-options scope explicit even though nearby code may reference broader repo surfaces.
3. Decide whether future work here is strictly run-library-only or true cross-artifact-library core before widening it.
4. Preserve more upstream Rust summary metadata at the first list response only if it does not break current lightweight library ergonomics.
5. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document session-history versus saved-run-library responsibilities.
- [x] Document saved-run search and visible-list behavior.
- [x] Document saved-run detail cache shape and fetch behavior.
- [x] Document load/apply/rerun/open/reveal/delete/tag flows.
- [x] Document current non-options scope boundary for this library surface.
- [x] Add or tighten tests if search, detail, or workflow-restore behavior changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/stores/historyStore.actions.test.ts`
- run `npm --prefix web run test -- src/stores/historyStore.search.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run `npm --prefix web run test -- src/components/HistoryWorkspacePanel.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- keep load/apply/rerun semantics stable before changing library layout or store shape
- if history detail grows, preserve current displayed summaries until replacements exist
- keep non-options run-library improvements separate from any options-library work
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/09_workflow_artifact_store_2026-04-08.md`
- `REPO_Improvement/06_artifacts/28_workflow_artifacts_sync_2026-04-08.md`
- `REPO_Improvement/04_results/01_results_overview_2026-04-08.md`
- `REPO_Improvement/04_results/17_analysis_store_service_2026-04-08.md`

## Execution Notes

- Closed as a history-library core hardening slice without widening the surface beyond the existing non-options run library.
- Added store-level regression coverage in `web/src/stores/historyStore.actions.test.ts` for detail-cache persistence through filtering and for stale-cache pruning during refresh/delete flows, complementing the existing service and panel coverage.
- Re-verified that session history, saved-run listing/search, lazy detail loading, and load/apply/rerun/report/reveal/delete flows remain split between the history store and history service instead of collapsing into ad hoc component logic.
- Acceptance evidence for closure: `npm.cmd --prefix web run test -- src/stores/historyStore.actions.test.ts src/stores/historyStore.search.test.ts src/services/historyService.test.ts src/components/HistoryWorkspacePanel.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

