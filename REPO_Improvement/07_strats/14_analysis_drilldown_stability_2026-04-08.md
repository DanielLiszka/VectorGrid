# Analysis Drilldown and Stability Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for non-options drilldown and local-stability analysis so current-candidate follow-up surfaces remain trustworthy, strategy coverage stays explicit, and partial implementation gaps are not hidden behind generic UI language.

## Scope Boundaries

In scope:

- drilldown and local-stability analysis surfaces
- current-candidate selection and follow-up analysis behavior
- strategy-family coverage and non-coverage
- analysis command, service, and panel alignment

Out of scope:

- options inspection/event drilldown
- redesigning the full result workspace layout
- inventing new analysis modes beyond current drilldown/stability surfaces

## Current User-Facing Behavior

The results workflow lets users select a candidate and compute drilldown curves or local-stability neighborhoods. Double-MA-family analysis is available through dedicated commands, while composite CPU analysis has more uneven coverage. Some strategies have explicit “not implemented yet” behavior for drilldown or local stability, and the Drill workspace must reflect that honestly.

## Current Code Surface

- `web/src/services/analysisService.ts`
- `web/src/components/DrillWorkspacePanel.vue`
- `src-tauri/src/commands_analysis.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.test.ts`

## Current Verification Surface

- analysis-service tests
- drill/workspace component tests
- tests in `crates/app_core/src/double_ma.rs`
- tests in `crates/app_core/src/composite_cpu.rs`
- manual current-candidate follow-up review through the result workspace

## Strengths Already Present

- drilldown and stability are already exposed as explicit follow-up analysis modes
- backend commands already separate Double-MA and composite CPU analysis entrypoints
- the UI already routes current-candidate actions from Top into follow-up analysis
- some strategy-specific “not implemented” cases are already explicit instead of silently failing

## Weaknesses / Failure Modes

- strategy coverage is uneven and easy to overstate
- drilldown/local-stability truth is spread across commands, analysis service, and UI empty-state text
- live execution reuses drilldown compute paths, which raises the importance of keeping semantics stable
- candidate-selection and analysis-surface readiness can drift if result panels change independently

## Regressions To Avoid

- implying drilldown or stability support for strategies that still do not implement it
- breaking current current-candidate handoff from result tables into analysis
- making partial implementation gaps less visible
- changing analysis data semantics without updating the UI status/empty-state language
- weakening existing tests for strategy-specific “not implemented” behavior

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because analysis requests cross command boundaries and need a stable contract.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because drilldown/stability value is only meaningful if the user-visible follow-up flow is testable end to end.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for keeping analysis request/result payloads explicit and versionable.

## Proposed Improvement Approach

1. Publish a current strategy-coverage matrix for drilldown and local stability.
2. Keep the Top-to-Drill follow-up contract explicit, including what current candidate context must be available.
3. Document which strategy families:
   - fully support drilldown and stability
   - partially support one but not the other
   - intentionally return “not implemented yet”
4. Keep analysis-surface empty-state and status copy aligned with actual backend availability.
5. Preserve analysis semantics that are reused by live execution paths.

## Checklist

- [ ] Document the current drilldown and local-stability coverage matrix by strategy family.
- [ ] Document current-candidate handoff requirements from results into analysis.
- [ ] Keep service/UI copy aligned with backend “not implemented yet” behavior.
- [ ] Verify Double-MA and composite CPU analysis entrypoints remain clearly separated.
- [ ] Identify where live execution depends on drilldown semantics and keep that coupling explicit.
- [ ] Add or maintain tests for unsupported strategies and empty-state behavior.
- [ ] Preserve current table-to-analysis navigation flows while improving clarity.
- [ ] Avoid generic language that overstates analysis support.

## Verification Plan

- run targeted analysis-service tests
- run `npm test -- src/components/DrillWorkspacePanel.test.ts`
- run relevant Top workspace tests that trigger analysis handoff
- run targeted Rust drilldown/local-stability tests in `ta_app_core`

## Rollout / Fallback / Compatibility Notes

- preserve existing supported drilldown/local-stability flows
- keep unsupported strategy cases explicit instead of silently suppressing actions
- if more strategy coverage is added later, update the coverage matrix and UI messaging together

## Related Docs

- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/04_results/20_matrix_table_state_2026-04-08.md`
- `REPO_Improvement/07_strats/12_eng_composite_validation_2026-04-08.md`
- `REPO_Improvement/09_live/03_execution_runner_lifecycle_DATE.md`
