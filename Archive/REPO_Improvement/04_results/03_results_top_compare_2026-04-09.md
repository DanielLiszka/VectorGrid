# Results Top Compare Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Top-results compare flow so pinned shortlist candidates, replacement/removal actions, stale cues, and winner-versus-shortlist review remain explicit and decision-useful.

## Scope Boundaries

In scope:

- compare tray behavior inside `TopWorkspacePanel`
- pin, clear, replace, and remove actions for shortlisted candidates
- provenance badges, stale reasons, and selection guardrails tied to compare state
- analysis-store state that feeds the compare tray

Out of scope:

- Overview baseline comparison against saved runs
- drilldown rendering internals
- options-related compare surfaces

## Current User-Facing Behavior

The Top panel already supports more than row selection. Users can pin the currently selected ranked candidate into a compare tray, keep multiple pinned candidates visible, replace a pinned card with the current selection, remove cards individually, or clear the whole compare set.

Pinned cards already carry more than rank and title. They include summary text, badges, and stale reasons such as the candidate being unavailable in the current shortlist or hidden by current filters. The selection panel above the table also tells the user to pin the current candidate if a side-by-side compare stack is needed.

That means compare is already a first-class shortlist workflow, not an incidental extra action.

## Current Code Surface

- `web/src/components/TopWorkspacePanel.vue`
- `web/src/stores/analysisStore.ts`
- `web/src/services/analysisService.ts`
- `web/src/components/AnalysisContextDock.vue`
- `web/src/stores/workflowStore.ts`

## Current Verification Surface

- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/services/analysisService.test.ts`
- `web/src/components/AnalysisContextDock.test.ts`

## Strengths Already Present

- the compare workflow already exists inside the shortlist surface where it is most useful
- pinned candidates already retain context beyond a bare identifier
- stale reasons already explain when a pinned item no longer matches the active shortlist view
- the UI already supports both additive pinning and replacement/removal actions
- compare remains separate from saved-run baseline review, which keeps responsibilities clearer

## Weaknesses / Failure Modes

- compare behavior is split between visible panel state and analysis-service reconciliation logic
- pinned-candidate truth can drift if filters, run changes, or row identities change unexpectedly
- stale reasons are useful but easy to weaken into vague or missing messages
- compare actions can become confusing if the selected candidate and pinned tray are not clearly distinguished
- there is no dedicated implementation doc freezing compare as shortlist comparison rather than generic history comparison

## Regressions To Avoid

- losing stale reasons or provenance badges on pinned candidates
- breaking row-identity based replacement and removal behavior
- collapsing compare into a less explicit “selected row only” model
- letting current shortlist filters silently invalidate pinned cards without explanation
- duplicating Overview baseline-comparison responsibilities inside Top compare
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Grid Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because Top compare depends on an interactive table workflow where selection and adjacent actions remain keyboard-coherent.
- [Table Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/table/) is relevant because pinned compare cards summarize table-derived information and should preserve the semantics of what the shortlist is actually showing.
- [ARIA: status role | MDN](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Roles/status_role) is relevant because stale cues, guardrails, and compare-state updates are advisory live information rather than focus-stealing alerts.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because compare state needs a single authoritative owner instead of per-component shadow state.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because compare behavior is a sequence of deterministic state transitions that should stay covered in focused tests.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Document Top compare as a dedicated shortlist-comparison subsystem, separate from Overview baseline comparison and separate from simple row selection.
2. Freeze the current action vocabulary: pin current, clear all, replace pinned item, and remove pinned item.
3. Treat pinned-card badges and stale reasons as contract data rather than decorative extras.
4. Keep compare identity tied to stable row ids and reconciled shortlist state, not presentation-only row position.
5. Add or tighten tests whenever pinning, stale reconciliation, or compare-card presentation changes.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the exact purpose of Top compare versus Overview baseline comparison.
- [x] Document pin, clear, replace, and remove action behavior.
- [x] Document pinned-card identity, summary, badge, and stale-reason expectations.
- [x] Document how filter changes and shortlist changes affect pinned candidates.
- [x] Add or tighten tests for pinning, replacing, stale-cue reconciliation, and compare-card actions.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/TopWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/AnalysisContextDock.test.ts`
- run `npm --prefix web run test -- src/services/analysisService.test.ts`
- manually verify pin/replace/remove behavior with filtered and unfiltered shortlists
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current pinned-candidate state shape until all compare consumers migrate together
- prefer additive compare metadata over repurposing existing card fields
- if compare grows later, keep shortlist-comparison and saved-baseline comparison as separate concepts
- if row identity logic changes, update compare reconciliation and stale-cue tests in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the Top compare flow as a dedicated shortlist-comparison subsystem rather than a replacement for Overview baseline comparison. `TopWorkspacePanel.test.ts`, `AnalysisContextDock.test.ts`, and `analysisService.test.ts` continue to cover pinning, compare-card state, and stale/provenance-aware compare behavior at the current scope.
- No code change was required in this tranche because the existing compare action vocabulary and stale reconciliation behavior already satisfy the documented contract.

## Related Docs

- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/10_out_overview_decision_surface_2026-04-09.md`
- `REPO_Improvement/04_results/15_dataset_stale_provenance_cues_2026-04-09.md`
- `REPO_Improvement/04_results/20_matrix_table_state_2026-04-08.md`
