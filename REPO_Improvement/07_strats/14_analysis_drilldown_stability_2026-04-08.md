# Analysis Drilldown and Stability Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because analysis requests cross command boundaries and need a stable contract.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because drilldown/stability value is only meaningful if the user-visible follow-up flow is testable end to end.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for keeping analysis request/result payloads explicit and versionable.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Publish a current strategy-coverage matrix for drilldown and local stability.
2. Keep the Top-to-Drill follow-up contract explicit, including what current candidate context must be available.
3. Document which strategy families:
   - fully support drilldown and stability
   - partially support one but not the other
   - intentionally return “not implemented yet”
4. Keep analysis-surface empty-state and status copy aligned with actual backend availability.
5. Preserve analysis semantics that are reused by live execution paths.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current drilldown and local-stability coverage matrix by strategy family.
- [x] Document current-candidate handoff requirements from results into analysis.
- [x] Keep service/UI copy aligned with backend “not implemented yet” behavior.
- [x] Verify Double-MA and composite CPU analysis entrypoints remain clearly separated.
- [x] Identify where live execution depends on drilldown semantics and keep that coupling explicit.
- [x] Add or maintain tests for unsupported strategies and empty-state behavior.
- [x] Preserve current table-to-analysis navigation flows while improving clarity.
- [x] Avoid generic language that overstates analysis support.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run targeted analysis-service tests
- run `npm test -- src/components/DrillWorkspacePanel.test.ts`
- run relevant Top workspace tests that trigger analysis handoff
- run targeted Rust drilldown/local-stability tests in `ta_app_core`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve existing supported drilldown/local-stability flows
- keep unsupported strategy cases explicit instead of silently suppressing actions
- if more strategy coverage is added later, update the coverage matrix and UI messaging together
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by publishing the current drilldown/local-stability coverage matrix in [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md), including:
  - the Double-MA-family path
  - the `CompositeCpu`/standalone composite-family path
  - the currently supported `Asr` / `Vemr` / `Vcc` / `Rocb` split
  - explicit “not implemented yet” coverage for `Vcet`, `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, and `Cdmc`
  - the explicit non-optimizer state for `MlSignal` artifacts
- Tightened frontend evidence in [analysisService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/analysisService.test.ts) with a regression test that proves unsupported `Vcet` drilldown remains explicit in `drillStatus` and `statusLine` instead of being silently implied as supported.
- Existing flow coverage remains the main truth anchor:
  - current-candidate handoff in `TopWorkspacePanel.test.ts`
  - Drill workspace rendering in `DrillWorkspacePanel.test.ts`
  - request routing and cache behavior in `analysisService.test.ts`
- Stable truth anchors:
  - `web/src/services/analysisService.ts`
  - `src-tauri/src/commands_analysis.rs`
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/composite_cpu.rs`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/services/analysisService.test.ts src/components/TopWorkspacePanel.test.ts src/components/DrillWorkspacePanel.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/04_results/20_matrix_table_state_2026-04-08.md`
- `REPO_Improvement/07_strats/12_eng_composite_validation_2026-04-08.md`
- `REPO_Improvement/09_live/03_execution_runner_lifecycle_2026-04-08.md`
