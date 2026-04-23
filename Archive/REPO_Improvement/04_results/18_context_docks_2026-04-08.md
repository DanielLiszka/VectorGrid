# Context Docks Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WAI-ARIA Authoring Practices: tabs](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)
  - relevant because dock visibility is closely tied to active workspace/tab posture
- [WAI-ARIA Authoring Practices: grid](https://www.w3.org/WAI/ARIA/apg/patterns/grid/)
  - relevant because summary docks still need predictable keyboard and structural semantics
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because docks should summarize store state rather than become separate authorities
- [Pinia core concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because dock content is already driven by shared store truth
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

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
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the visibility gates for each current dock.
- [x] Document the summary/action responsibilities for each dock separately.
- [x] Document analysis-dock persistence and collapsibility behavior.
- [x] Document validation-dock compactness and quick-action posture.
- [x] Add direct component tests for optimize, validation, and history docks.
- [x] Preserve the “context surface, not duplicate workspace” boundary in future changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/AnalysisContextDock.test.ts`
- add or run dock-specific component tests for optimize, validation, and history
- manually verify dock visibility in each relevant workspace/tab combination
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve existing mount conditions in `App.vue` unless a visibility change is deliberate and tested
- keep dock changes additive and summary-first
- if shared dock primitives are introduced later, keep existing dock actions stable until parity is proven
- do not merge docks into one generic component unless the role differences are still expressible clearly
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the dock family with [AnalysisContextDock.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/AnalysisContextDock.test.ts), [OptimizeContextDock.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeContextDock.test.ts), [ValidationContextDock.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/ValidationContextDock.test.ts), [HistoryContextDock.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryContextDock.test.ts), [shellDockContract.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/utils/shellDockContract.test.ts), and [App.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/App.test.ts).
- No production code change was required in this tranche because the current summary-first dock posture, visibility gating, and analysis-dock persistence behavior are already pinned by direct tests.

## Related Docs

- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/05_validation/22_validation_context_docks_2026-04-08.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
- `REPO_Improvement/01_shell/08_shell_docks_shortcuts_2026-04-08.md`
