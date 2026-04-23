# Output Overview Decision Surface Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Overview output surface so the leading candidate, execution-risk cues, comparison summary, and next-step actions form a decision-grade review surface instead of a decorative winner card.

## Scope Boundaries

In scope:

- `web/src/components/OverviewWorkspacePanel.vue`
- winner framing, best stat cards, comparison summary, and execution-risk cues
- copy/export actions and handoff into Top Results
- analysis-store fields that directly support overview decisions

Out of scope:

- dense shortlist triage mechanics in Top Results
- workbook/report presentation
- options-related overview surfaces

## Current User-Facing Behavior

Overview already tries to answer the core question “is this winner worth deeper review?” It presents the leading candidate, supporting stats, best-summary details, optional execution-risk notice, baseline-comparison controls, and export/copy actions. It also includes a strong handoff back to Top Results so the user can move from decision framing into shortlist triage when needed.

The current panel is therefore not just a landing page. It is already the first decision surface after an optimization run or artifact load, and its quality directly affects whether users trust the candidate before moving into drilldown, validation, or execution review.

## Current Code Surface

- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/historyStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`

## Current Verification Surface

- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/services/analysisService.test.ts`
- result-shell tests that cover tab and workflow handoff

## Strengths Already Present

- the panel already keeps winner framing separate from shortlist triage
- execution-risk cues already connect optimization output to downstream execution decisions
- baseline review already lives here rather than cluttering Top Results
- export and copy actions are already placed near the winner decision surface
- the current copy already encourages deliberate movement into Top Results rather than pretending the overview is sufficient by itself

## Weaknesses / Failure Modes

- the winner card can look more decisive than it really is if caveats and comparison context weaken
- execution-risk language can drift away from the actual loaded run configuration
- best-summary and best-stat presentation can become too terse for decision support or too dense for quick review
- the presence of export buttons can imply readiness even when comparison or stale-state context is weak
- there is no dedicated implementation doc freezing what Overview must expose before a user should trust the winner framing

## Regressions To Avoid

- reducing the visibility of execution-risk cues when they still apply
- turning Overview into either a generic splash panel or a duplicate of Top Results
- weakening baseline review or compare summary visibility
- presenting a winner without enough supporting evidence or without clear next actions
- detaching export/copy actions from the actual current winner state
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tabs Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because Overview must keep a distinct decision-framing role within the results tab system.
- [ARIA: status role | MDN](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Roles/status_role) is relevant because winner readiness, pending states, and execution-risk guidance are dynamic advisory information.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the overview decision surface should stay driven by shared analysis state instead of panel-local reconstruction.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the most important Overview behaviors are decision-state transitions and action readiness.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because this panel should ultimately be judged by what a user can see, understand, and act on in one pass.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Freeze Overview as the first-pass decision surface for the current leading candidate.
2. Treat execution-risk notice, baseline review, and best-summary detail as mandatory decision context rather than optional supporting text.
3. Keep Top Results as the canonical next-step handoff when the user needs shortlist context.
4. Add or tighten tests around execution-risk visibility, compare controls, and winner/export action readiness.
5. Prefer additive decision evidence over purely aesthetic winner-card changes.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the minimum evidence Overview must show before the winner feels decision-grade.
- [x] Document execution-risk notice expectations and coupling to run configuration.
- [x] Document baseline review and comparison-summary behavior.
- [x] Document copy/export and Top-results handoff behavior.
- [x] Add or tighten tests for decision-surface readiness and compare/action flows.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/OverviewWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/analysisService.test.ts`
- manually verify empty, pending, and ready Overview states with and without execution-risk cues
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current Top-results handoff even if Overview gets richer
- keep export/copy controls truthful to the currently loaded winner state
- prefer additive decision context over replacing existing winner framing wholesale
- if execution-risk criteria evolve later, update Overview copy and tests in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Expanded [OverviewWorkspacePanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OverviewWorkspacePanel.test.ts) to pin execution-risk visibility and the settings handoff when the current run enables short same-bar execution.
- Revalidated the panel’s empty, pending, ready, baseline-review, copy/export, and Top-handoff behavior through the overview and analysis-service test surfaces.
- No production code change was required in this tranche because the current Overview panel already satisfies the documented decision-surface contract once the missing execution-risk regression coverage was added.

## Related Docs

- `REPO_Improvement/04_results/01_results_overview_2026-04-08.md`
- `REPO_Improvement/04_results/03_results_top_compare_2026-04-09.md`
- `REPO_Improvement/04_results/09_results_analysis_contract_2026-04-09.md`
- `REPO_Improvement/09_live/05_execution_safety_modes_2026-04-09.md`
