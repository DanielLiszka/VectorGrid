# Live Mode Discrete Surface Promotion Validated Implementation Guide

Last updated: 2026-04-11

Companion to:
- `09_live_mode_discrete_surface_promotion_2026-04-11.md`

## Purpose

Turn the validated shell-promotion findings into a concrete first-slice implementation shape that fits the repo’s current workspace-shell model.

## Promotion Shape Decision

The first slice creates a new top-level `live` workspace.

It does not:
- create a new shell `mode`
- introduce router-based navigation
- preserve execution as a co-equal settings-mounted primary surface

This keeps live discrete without colliding with the existing `mode` concept used for `conventional` and `ml-signal`.

## Current Shell Owner Map

- high-level panel mount owner:
  - `web/src/App.vue`
- shell layout and visible slot owner:
  - `web/src/components/VueWorkspaceShell.vue`
- shell-state persistence owner:
  - `web/src/stores/workflowStore.ts`
- allowed shell vocabulary owner:
  - `web/src/utils/core/workflowModel.ts`
- restore/bootstrap owner:
  - `web/src/services/workflowBootstrap.ts`
- current settings/live surfaces:
  - `web/src/components/SettingsWorkspacePanel.vue`
  - `web/src/components/ExecutionWorkspacePanel.vue`

## First-Slice Scope

The first slice is frontend-shell work first.

Primary goals:
- create a truthful `live` workspace entry
- remove the current settings/live shell contradiction
- keep the live support envelope visible
- preserve current confirmations, blocked reasons, and monitoring detail

Backend/Tauri execution behavior stays unchanged unless a small shell wiring adjustment is unavoidable.

## Support And Dependency Contract

The promoted surface remains narrow and dependent on existing context:
- supported strategies remain `DoubleMa` and `CompositeCpu`
- broker remains Alpaca
- live feed remains Polygon stocks only
- runner still requires an active same-symbol live stream
- loaded dataset / strategy candle context is still required
- parameter source rules remain `selected`, `best`, and `manual`
- single-stream and single-runner limits remain explicit

This slice clarifies placement and posture. It does not create a fully standalone trading cockpit.

## Shell Truth Contract

When the `live` workspace is active, the outer shell must no longer read like settings.

The first slice must update together:
- workspace label
- shell subtitle
- context chips / summary cues that would otherwise imply settings/preferences posture
- full-shell screenshot evidence, not panel-only evidence

## Persistence And Restore Rules

- new persisted `workspace='live'` is supported explicitly
- existing persisted `workspace='settings'` stays settings after migration
- once `live` ships, the settings-mounted execution surface is no longer the primary interactive live entry
- rollback maps persisted `live` back to `settings`

During rollout:
- the `live` workspace is authoritative
- any remaining execution surface in settings is fallback-only or removed

## Dependency Sequencing

This slice may ship before broader live hardening only as shell/posture work.

It must not imply that these sibling bundles are already complete:
- operator safety controls
- secure credential storage
- chart/account review completion

If those sibling bundles remain open, the promoted shell must keep the narrow support envelope and current limitations visible at point of use.

## Preconditions

Before code changes start, freeze these choices in the parent bundle doc:

- first slice is a new `live` workspace, not a new shell `mode`
- execution remains narrow and context-dependent
- settings-mounted execution is no longer the primary interactive live entry after promotion
- shell promotion does not imply sibling bundle completion

## Ordered Implementation Slices

1. Shell vocabulary slice
   - add the `live` workspace to shell vocabulary and persistence
2. Mount topology slice
   - move the primary execution mount out of the current settings special case
3. Shell-truth slice
   - update outer-shell label, subtitle, and context cues together
4. Restore/fallback slice
   - support persisted `workspace='live'`
   - define rollback mapping back to `settings`
5. Regression hardening slice
   - verify shell state, restore, accessibility, and current live guardrails

## File-Level Change Plan

- `web/src/utils/core/workflowModel.ts`
  - add and validate the `live` workspace vocabulary and allowed combinations
- `web/src/stores/workflowStore.ts`
  - persist/select/restore the new `live` workspace and its shell subtitle
- `web/src/services/workflowBootstrap.ts`
  - restore persisted live workspace state safely
- `web/src/App.vue`
  - remove the current execution special-case from the settings-only mount path
  - mount the primary execution surface through the chosen `live` workspace path
- `web/src/components/VueWorkspaceShell.vue`
  - add the `live` workspace entry and truthful outer-shell copy
- `web/src/components/ExecutionWorkspacePanel.vue`
  - adapt mount behavior to the promoted shell target without changing backend live semantics

## Acceptance Evidence

Closure evidence must include:

- one full-shell screenshot of the old settings-mounted reading
- one full-shell screenshot of the promoted live reading
- one persisted-state restore proof into `live`
- one rollback mapping proof from persisted `live` back to `settings`
- one proof that blocked reasons, confirmations, and monitoring detail remain visible

## Closure Conditions

Do not mark the bundle complete until:

- the outer shell no longer reads like settings when the live surface is active
- `mode` semantics remain untouched for `conventional` and `ml-signal`
- persisted shell state and restore behavior are defined and tested
- current live support-envelope limitations remain visible
- backend/Tauri execution behavior is unchanged except for any minimal shell wiring needed to surface the new workspace

## Verification Requirements

- targeted shell-state tests in `workflowStore.ts`
- mount and visibility checks in `App.vue` and `VueWorkspaceShell.vue`
- restore checks through `workflowBootstrap.ts`
- execution-surface regression checks in `ExecutionWorkspacePanel.test.ts`
- `npm.cmd --prefix web run test:browser:ux`
- `npm.cmd --prefix web run a11y:smoke` if tab/selection semantics or shell labeling changes
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` when persisted shell-state behavior changes
- full-shell before/after screenshots

## Final Regression Validation Requirement

- Do not close this guide’s implementation work until the finished integrated state has been revalidated at the end of the work, not only during intermediate shell slices.
- Final validation must rerun the targeted shell/bootstrap/frontend tests against the completed promoted surface, then run the relevant repo quality gates required by the final change set.
- Final validation must include browser, desktop, and accessibility checks proving the final shell state is truthful and navigable.
- Final validation must include performance evaluation for touched bootstrap, workspace switch, and live-surface mount flows so the final state does not regress startup or navigation responsiveness.
- If the final integrated state introduces correctness, UX, safety, or performance regressions, narrow the slice, add a guard, or revert the offending portion before closure.

## Rollback Rule

If the promoted `live` workspace increases shell confusion or weakens current safety posture, remove the `live` workspace entry and map the feature back to the prior settings placement until the narrower shell-truth issues are resolved.
