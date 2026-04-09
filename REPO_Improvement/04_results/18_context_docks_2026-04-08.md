# Context Docks Plan

Last updated: 2026-04-08

## Purpose

Define the current context-dock contract so quick context, provenance, and action affordances can be improved without turning the docks into duplicate workspaces or breaking current visibility gates.

## Scope Boundaries

In scope:

- optimize, analysis, validation, and history context docks
- App-level gating and mount conditions
- dock persistence where it already exists
- summary/action role of docks

Out of scope:

- redesigning the underlying workspaces
- introducing large new dock-only workflows
- options-specific panels

## Current User-Facing Behavior

The app already ships four different dock surfaces:

- `OptimizeContextDock`
- `AnalysisContextDock`
- `ValidationContextDock`
- `HistoryContextDock`

They are not interchangeable. Current behavior is intentionally scoped:

- docks are mounted only when the relevant workspace/tab context is active
- they act as quick summaries plus jump-off actions
- analysis dock is currently collapsible and persisted
- validation dock is intentionally compact and summary-first
- optimize and history docks expose context and lightweight actions without replacing the main panels

So the current product model is “context surface, not mini-workspace.”

## Current Code Surface

- `web/src/App.vue`
  - gates which docks are mounted in which shell/workflow contexts
- `web/src/components/OptimizeContextDock.vue`
  - optimize summary and action surface
- `web/src/components/AnalysisContextDock.vue`
  - collapsible analysis summary dock
- `web/src/components/ValidationContextDock.vue`
  - compact validation status/meta/preview dock
- `web/src/components/HistoryContextDock.vue`
  - history/library summary surface
- `web/src/components/AnalysisContextDock.test.ts`
  - current dock-specific regression coverage

## Current Verification Surface

- `web/src/components/AnalysisContextDock.test.ts`
- App-level shell tests through `web/src/App.vue` ownership
- related store/service tests for the underlying data that docks summarize

## Strengths Already Present

- docks already respect workspace/workflow routing instead of floating everywhere
- summary-first posture keeps them lightweight
- analysis dock persistence already proves the shell can support dock memory safely
- validation dock is intentionally compact and avoids duplicating the full validation panel
- the docks already help with provenance and action discovery near the working surface

## Weaknesses / Failure Modes

- dock differences are real, but not documented as an explicit contract
- optimize/validation/history docks have less direct component-test coverage than analysis dock
- shared visual/state patterns are implicit rather than declared
- it is easy for future edits to expand dock content until it duplicates the main workspace
- visibility and persistence rules are not yet written down in one place

## Regressions To Avoid

- showing a dock in the wrong workspace or workflow
- turning a dock into a second full-featured workspace
- regressing the current compact quick-action role
- losing persisted analysis-dock state or other existing visibility behavior
- duplicating or contradicting the authoritative content in the main workspace panels

## External Research Summary

- [WAI-ARIA Authoring Practices: tabs](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)
  - relevant because dock visibility is closely tied to active workspace/tab posture
- [WAI-ARIA Authoring Practices: grid](https://www.w3.org/WAI/ARIA/apg/patterns/grid/)
  - relevant because summary docks still need predictable keyboard and structural semantics
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because docks should summarize store state rather than become separate authorities
- [Pinia core concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because dock content is already driven by shared store truth

## Proposed Improvement Approach

1. Freeze the role of each dock explicitly:
   - what it summarizes
   - what actions it owns
   - where it may appear
2. Define shared dock rules:
   - summary-first
   - provenance/helpful status chips
   - quick actions only
   - no duplication of main workflows
3. Preserve intentionally distinct behavior:
   - analysis collapsibility
   - validation compactness
   - workspace-specific gating
4. Add direct tests for optimize, validation, and history docks similar to the existing analysis-dock coverage.
5. If shared styling or chips are extracted later, keep the individual dock responsibilities distinct.

## Checklist

- [ ] Document the visibility gates for each current dock.
- [ ] Document the summary/action responsibilities for each dock separately.
- [ ] Document analysis-dock persistence and collapsibility behavior.
- [ ] Document validation-dock compactness and quick-action posture.
- [ ] Add direct component tests for optimize, validation, and history docks.
- [ ] Preserve the “context surface, not duplicate workspace” boundary in future changes.

## Verification Plan

- run `npm --prefix web test -- src/components/AnalysisContextDock.test.ts`
- add or run dock-specific component tests for optimize, validation, and history
- manually verify dock visibility in each relevant workspace/tab combination

## Rollout / Fallback / Compatibility Notes

- preserve existing mount conditions in `App.vue` unless a visibility change is deliberate and tested
- keep dock changes additive and summary-first
- if shared dock primitives are introduced later, keep existing dock actions stable until parity is proven
- do not merge docks into one generic component unless the role differences are still expressible clearly

## Related Docs

- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/05_validation/22_validation_context_docks_2026-04-08.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
- `REPO_Improvement/01_shell/08_shell_docks_shortcuts_2026-04-08.md`
