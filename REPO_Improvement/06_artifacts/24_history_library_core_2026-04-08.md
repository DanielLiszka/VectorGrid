# History Library Core Plan

Last updated: 2026-04-08

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

## Checklist

- [ ] Document session-history versus saved-run-library responsibilities.
- [ ] Document saved-run search and visible-list behavior.
- [ ] Document saved-run detail cache shape and fetch behavior.
- [ ] Document load/apply/rerun/open/reveal/delete/tag flows.
- [ ] Document current non-options scope boundary for this library surface.
- [ ] Add or tighten tests if search, detail, or workflow-restore behavior changes.

## Verification Plan

- run `npm --prefix web test -- src/stores/historyStore.actions.test.ts`
- run `npm --prefix web test -- src/stores/historyStore.search.test.ts`
- run `npm --prefix web test -- src/services/historyService.test.ts`
- run `npm --prefix web test -- src/components/HistoryWorkspacePanel.test.ts`

## Rollout / Fallback / Compatibility Notes

- keep load/apply/rerun semantics stable before changing library layout or store shape
- if history detail grows, preserve current displayed summaries until replacements exist
- keep non-options run-library improvements separate from any options-library work

## Related Docs

- `REPO_Improvement/06_artifacts/09_workflow_artifact_store_2026-04-08.md`
- `REPO_Improvement/06_artifacts/28_workflow_artifacts_sync_2026-04-08.md`
- `REPO_Improvement/04_results/01_results_overview_2026-04-08.md`
- `REPO_Improvement/04_results/17_analysis_store_service_2026-04-08.md`
