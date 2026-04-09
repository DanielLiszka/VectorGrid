# State Artifacts Commands Plan

Last updated: 2026-04-09

## Purpose

Define the current quick-action and artifact-command contract so report-open, export, drill-navigation, and artifact-backed latest-path behavior can be improved without breaking the current shell command surface.

## Scope Boundaries

In scope:

- `commandActionStore.ts`
- interaction between command actions and `workflowArtifactsStore`
- latest report/meta-report path resolution
- routed shared shell commands such as exports, drill navigation, and layout toggles

Out of scope:

- command-palette UI itself
- deep implementation of analysis/data/validation services
- options-specific artifact commands

## Current User-Facing Behavior

The shell already exposes a shared action layer that multiple surfaces use:

- run / cancel / validate
- browse / load dataset / open Polygon
- export top CSV / JSON and validation shortlist CSV
- open latest report or latest meta report
- toggle controls panel
- drill clear / previous / next trade navigation

Latest report behavior already has a fallback chain:

- store state
- workflow-artifacts current run
- root runtime dataset attributes

That means artifact-backed quick actions already survive beyond the immediate run surface. This store is a real bridge between shell commands and current artifact state.

## Current Code Surface

- `web/src/stores/commandActionStore.ts`
  - routed command methods, latest-path resolution, artifact fallback, and runtime-running guards
- `web/src/stores/commandActionStore.test.ts`
  - verifies shared action dispatch, report-path fallback, and drill-navigation actions
- `web/src/stores/workflowArtifactsStore.ts`
  - stores current run/validation artifacts, including report paths
- `web/src/stores/workflowArtifactsStore.test.ts`
  - verifies artifact hydration behavior
- `web/src/services/historyService.ts`
  - interacts with artifact-backed report/open flows in library/replay paths
- `web/src/services/validationService.ts`
  - writes artifact-backed validation/report data used by quick actions

## Current Verification Surface

- `web/src/stores/commandActionStore.test.ts`
- `web/src/stores/workflowArtifactsStore.test.ts`
- related history/validation service tests

## Strengths Already Present

- the shell already has a centralized action-routing layer
- report-path fallback already uses artifact state instead of only legacy runtime data attributes
- command actions mostly delegate into existing stores/services instead of reimplementing business logic
- runtime-running guards already prevent unsafe report-open flows
- drill-navigation actions are already normalized into the same shared surface

## Weaknesses / Failure Modes

- latest-path resolution rules are important but easy to miss
- artifact-backed and legacy quick-action state still coexist, which can blur the current source of truth
- command routing spans several stores/services and is therefore easy to regress indirectly
- there is no one doc that explains which actions are shell-shared versus panel-local

## Regressions To Avoid

- breaking latest-report fallback from workflow artifacts
- opening report paths while runtime-running guards should block
- moving shared commands back into multiple UI surfaces
- losing drill-navigation or export behavior during refactors
- confusing legacy quick-action snapshots with the preferred artifact-backed path
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because many shared shell actions eventually bridge into Tauri commands
- [Pinia actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because shared command behavior is already modeled as action dispatch rather than component-side imperative code
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because artifact-backed quick actions depend on centralized state instead of DOM-driven fallbacks
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because shared shell commands often need higher-level interaction verification
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Freeze the shared command surface explicitly.
2. Document current path-resolution precedence for latest report/meta-report actions.
3. Preserve the distinction between:
   - routed shell commands
   - artifact state
   - legacy runtime snapshots
4. Keep command actions as a delegating layer rather than a business-logic layer.
5. Add explicit regressions around report-path fallback and runtime-running guards if quick-action coverage expands.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current shared shell command inventory.
- [x] Document latest report/meta-report path precedence.
- [x] Document workflow-artifact fallback behavior.
- [x] Document runtime-running guard behavior for report-open flows.
- [x] Preserve delegated routing into existing stores/services.
- [x] Add or tighten tests if quick-action sources or precedence rules change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/stores/commandActionStore.test.ts`
- run `npm --prefix web run test -- src/stores/workflowArtifactsStore.test.ts`
- run related history and validation service tests if latest-path or artifact behavior changes
- manually verify report-open behavior after artifact-backed validation runs
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks

## Rollout / Fallback / Compatibility Notes

- preserve artifact-backed fallback as the preferred current path source
- keep legacy runtime snapshots only as long as they are needed for compatibility
- if new shell commands are added, attach them to the current shared action surface rather than inventing a new one
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/04_shell_command_palette_2026-04-09.md`
- `REPO_Improvement/06_artifacts/09_workflow_artifact_store_2026-04-08.md`
- `REPO_Improvement/06_artifacts/28_workflow_artifacts_sync_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
