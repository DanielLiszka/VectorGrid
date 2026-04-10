# History Search Filter UX Plan

Last updated: 2026-04-09

## Purpose

Define the current saved-run search and filter UX so ranking, selection stability, and library-scoped discoverability can be improved without turning history search into a different product surface.

## Scope Boundaries

In scope:

- saved-run library search behavior
- search debouncing and library-only messaging
- fuzzy ranking and token matching
- selection/detail stability while filtering

Out of scope:

- global command search
- session-history selection UX
- backend artifact indexing changes

## Current User-Facing Behavior

The history panel already ships a real saved-run search experience:

- search is explicitly local to the saved-run library
- input is debounced in the panel
- an empty query preserves original order
- short queries favor exact and prefix-style matches
- longer queries use fuzzy ranking
- multi-token search works across strategy kind, dataset path, artifact path, and tags
- filtered results preserve selected-detail behavior when possible

The UI copy already tells the user this is not global search and points global commands toward `Ctrl+K`.

## Current Code Surface

- `web/src/components/HistoryWorkspacePanel.vue`
  - owns the search input, local explanatory copy, and debounced search updates
- `web/src/stores/historyStore.ts`
  - owns tokenization, search normalization, fuzzy ranking, and visible-list synchronization
- `web/src/stores/historyStore.search.test.ts`
  - verifies empty-query ordering, short-query preference, typo-tolerant fuzzy search, and multi-token search
- `web/src/components/HistoryWorkspacePanel.test.ts`
  - panel-level behavior coverage

## Current Verification Surface

- `web/src/stores/historyStore.search.test.ts`
- `web/src/components/HistoryWorkspacePanel.test.ts`
- `web/src/stores/historyStore.actions.test.ts`

## Strengths Already Present

- search scope is already explicit and not confused with global search
- ranking behavior already distinguishes short-query and fuzzy-query cases
- tags and dataset paths are already part of searchable text
- debouncing is already implemented in the panel instead of forcing every keystroke into full refresh churn
- empty queries intentionally preserve authorial/library order

## Weaknesses / Failure Modes

- the scorer and ranking rules are embedded inside the store rather than isolated as a clearly documented contract
- selection/detail stability rules are important but easy to miss when changing filtering
- matched-field reasoning is not surfaced to the user, which can make fuzzy results feel opaque
- current behavior depends on several interacting rules that are only partly documented through tests
- it is easy for future edits to turn this into “global-ish search” copy or behavior

## Regressions To Avoid

- making saved-run search behave like a global shell search
- changing short-query ranking without intentional review
- losing empty-query order preservation
- clearing or thrashing selected detail on harmless filter changes
- removing tags/dataset/artifact path from the search corpus silently
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WAI-ARIA Authoring Practices: listbox](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/)
  - relevant because the saved-run library behaves like a searchable selection list that must remain predictable
- [WAI-ARIA Authoring Practices: grid](https://www.w3.org/WAI/ARIA/apg/patterns/grid/)
  - relevant because history results and detail interactions depend on stable keyboard/navigation semantics
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because search/filter UX regressions are best caught through realistic UI flows and fixtures
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because the ranking logic already lives in store-driven behavior and should remain directly testable
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because artifacts, reports, manifests, and replay payloads behave like versioned serialized contracts that should stay explicit and reviewable

## Proposed Improvement Approach

1. Freeze the current search contract explicitly:
   - empty-query behavior
   - short-query ranking
   - fuzzy ranking
   - multi-token matching
2. Consider extracting the scorer into a pure helper so ranking fixtures are easier to lock down.
3. Add tests for:
   - selection persistence through filtering
   - no-match UX
   - library-only search messaging
4. Keep the current “saved-run library only” scope visible in copy and behavior.
5. If future matched-field hints are added, keep ranking rules unchanged unless deliberately revised.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the saved-run search corpus and ranking rules.
- [x] Document empty-query ordering as a contract item.
- [x] Document short-query versus fuzzy-query behavior.
- [x] Document multi-token matching across dataset/artifact/tags.
- [x] Add coverage for selection/detail stability and no-match cases.
- [x] Preserve library-only scope in both copy and behavior.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/stores/historyStore.search.test.ts`
- run `npm --prefix web run test -- src/components/HistoryWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/historyStore.actions.test.ts`
- manually verify search, no-match, and selection-stability behavior in the history panel
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- keep search local to the saved-run library
- preserve current ranking unless a new ranking contract is deliberately adopted
- prefer additive hints or better copy over ranking churn
- if scorer extraction happens later, keep output parity fixtures before replacing the current store path
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
- `REPO_Improvement/06_artifacts/26_history_service_store_2026-04-08.md`
- `REPO_Improvement/04_results/18_context_docks_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`

## Execution Notes

- Closed as a saved-run search UX hardening slice without widening the search scope beyond the local run library.
- Added coverage in `web/src/stores/historyStore.actions.test.ts` for selection/detail persistence while filters still match and for clean selection reset when filters yield zero results.
- Added panel coverage in `web/src/components/HistoryWorkspacePanel.test.ts` proving the no-match state stays explicitly library-scoped and continues to direct global search toward `Ctrl+K`.
- Acceptance evidence for closure: `npm.cmd --prefix web run test -- src/components/HistoryWorkspacePanel.test.ts src/stores/historyStore.actions.test.ts src/stores/historyStore.search.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

