# Live Mode Discrete Surface Promotion Plan

Last updated: 2026-04-11

## Purpose

Define a measured plan to promote live execution from a settings-mounted execution panel into a clearer discrete live mode with its own operating surface, while preserving the current narrow broker and strategy scope.

## Scope Boundaries

In scope:

- live mode as a distinct operating mode and navigation surface
- information architecture and route/workspace placement for the existing live feature
- separation between research/backtest workflows and live operation workflows
- operator-state clarity, posture copy, and mode-entry/exit boundaries

Out of scope:

- broad execution strategy expansion
- options live trading
- multi-broker expansion
- redesigning unrelated workflow surfaces

## Current User-Facing Behavior

The current execution feature is real, but it still reads as a settings-adjacent tool:

- the execution panel is mounted in the current execution/settings workspace area
- live, paper, and dry-run actions share one surface with operational and review content mixed together
- the feature can feel both too secondary and too risky at the same time

If live mode is meant to be a discrete mode, the product needs a clearer boundary than the current placement provides.

## Current Code Surface

- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `web/src/stores/workflowStore.ts`
- `web/src/App.vue`
- `web/src/components/VueWorkspaceShell.vue`
- `src-tauri/src/commands_execution.rs`

## Current Verification Surface

- `web/src/components/ExecutionWorkspacePanel.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/services/executionService.test.ts`
- `npm.cmd --prefix web run test:browser:ux`
- `npm.cmd --prefix web run test:desktop:smoke`

## Strengths Already Present

- the live-execution state model already exists
- blocked reasons, status hints, and mode labels already exist
- the backend command surface is already real and bounded
- the scope is narrow enough that surface promotion can stay additive rather than becoming a full trading-platform rewrite

## Weaknesses / Failure Modes

- the current placement understates the seriousness of live actions
- research/backtest mental models and live-operating mental models are currently too close together
- operator posture, session review, and live status can compete for space inside one panel
- a settings-style panel is a poor long-term home if live mode is expected to be discrete

## Regressions To Avoid

- moving the surface in a way that weakens current safety prompts or blocked reasons
- creating a more prominent live mode before its safety and recovery posture is explicit
- hiding existing review/monitoring detail in the name of a cleaner shell
- breaking desktop layout, navigation, or accessibility while promoting the live surface
- allowing this surface to stay behaviorally working while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html)
- [Tauri state management](https://v2.tauri.app/develop/state-management/)
- [Tauri Dialog](https://v2.tauri.app/plugin/dialog/)
- [MDN content-visibility](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/content-visibility)

## Proposed Improvement Approach

1. Treat live execution as a discrete operating mode, not just a heavier variant of research execution settings.
2. Define the mode boundary clearly:
   - how the operator enters live mode
   - how the operator leaves live mode
   - what posture is visible before any broker-facing action
3. Split the surface into clearer sub-areas:
   - current posture and arm/disarm state
   - action controls
   - monitoring and review
   - broker/account context
4. Preserve the current narrow live scope and make that limitation explicit in the new surface.
5. Defer any broad shell redesign outside the execution/live domain.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define the intended discrete live-mode entry point and shell placement.
- [ ] Decide what remains on the existing execution/settings surface versus what moves into live mode.
- [ ] Define the minimum visible posture information required before any live-mode action is available.
- [ ] Define how paper, dry-run, and live broker-armed states are represented inside the promoted surface.
- [ ] Preserve current blocked reasons, confirmations, and review surfaces while reshaping layout.
- [ ] Add or strengthen browser and desktop tests for the promoted live-mode shell.
- [ ] Capture screenshots and route-flow acceptance evidence for the promoted surface.
- [ ] Define rollback conditions if surface promotion increases confusion or weakens safety posture.

## Verification Plan

- rerun targeted execution panel, execution store, and execution service tests near touched files
- rerun `npm.cmd --prefix web run test:browser:ux`
- rerun `npm.cmd --prefix web run test:desktop:smoke`
- rerun `npm.cmd --prefix web run a11y:smoke` if route/layout changes affect the promoted live-mode shell
- run `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`
- capture concrete acceptance evidence such as route screenshots, live-mode shell screenshots, and operator-flow notes

## Final Regression Validation Requirement

- Do not mark this item complete until the finished implementation state has been revalidated at the end of the work, not only per shell/layout slice.
- Final validation must include targeted unit/integration tests for shell state, workspace persistence, bootstrap/restore behavior, and execution-surface mounting, plus the relevant repo quality gates required by the final change set.
- Final validation must include browser, desktop, and accessibility checks proving the promoted live surface is truthful, navigable, and still compatible with the existing settings-mounted dependency posture where required.
- Final validation must include performance evaluation for any touched shell bootstrap, workspace switching, or live-surface mount path so promotion does not degrade startup or navigation responsiveness.
- If the final integrated state introduces correctness, UX, safety, or performance regressions, narrow the slice, add a guard, or revert the offending portion before closure.

## Rollout / Fallback / Compatibility Notes

- start with shell and posture clarification before broad new action expansion
- preserve the existing execution surface as a fallback while the discrete live mode is proven
- if a promoted live shell weakens clarity or safety, keep the current placement until the narrower issues are fixed
- do not widen this plan into a full app-wide navigation redesign

## Related Docs

- `05_live_execution_productization_2026-04-10.md`
- `06_live_execution_operator_safety_controls_2026-04-11.md`
- `REPO_Improvement/09_live/02_execution_workspace_ux_2026-04-09.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`
