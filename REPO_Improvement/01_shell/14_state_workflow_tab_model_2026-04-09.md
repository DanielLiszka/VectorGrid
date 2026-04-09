# State Workflow Tab Model Plan

Last updated: 2026-04-09

## Purpose

Define the current shell workflow/tab model so workspace, workflow, workflow-step, controls-tab, results-tab, and aggregate-tab behavior can be improved without breaking the existing navigation grammar.

## Scope Boundaries

In scope:

- workflow/state model in `workflowModel.ts`
- workflow-state transitions and persistence in `workflowStore.ts`
- mode-specific defaults and allowed-tab rules
- shell subtitles and runtime dataset attributes derived from workflow state

Out of scope:

- deep behavior of individual panels
- options-specific workflow modes
- saved perspectives beyond their use of the workflow model

## Current User-Facing Behavior

The shell already has a structured state model:

- shell mode: `conventional` or `ml-signal`
- workspace tabs: `workflow`, `library`, `aggregate`, `pro`, `settings`
- workflow steps: `data`, `optimize`, `validate`, `analyze`, `finalize`
- controls tabs, results tabs, and aggregate tabs with mode/workflow-specific defaults and allowed sets

Current behavior is already rich:

- mode-specific tab memory is preserved between conventional and ML Signal
- changing workspace or workflow step recalculates allowed tabs intentionally
- shell subtitle and root dataset attributes are derived from current posture
- invalid tabs are ignored
- workflow state is persisted and restored as part of the shell

This is already a navigation contract, not just a store with a few strings.

## Current Code Surface

- `web/src/utils/core/workflowModel.ts`
  - canonical enums, defaults, allowed maps, labels, subtitles, and normalization helpers
- `web/src/utils/core/workflowModel.test.ts`
  - regression coverage for normalization and model helpers
- `web/src/stores/workflowStore.ts`
  - state transitions, runtime sync, persistence, tab memory, and subtitle derivation
- `web/src/stores/workflowStore.test.ts`
  - verifies workspace/workflow-step selection, mode-specific tab memory, invalid input handling, and split-state behavior
- `web/src/components/VueWorkspaceShell.vue`
  - consumes the model to render workspace/mode/step/tab posture

## Current Verification Surface

- `web/src/stores/workflowStore.test.ts`
- `web/src/utils/core/workflowModel.test.ts`
- `web/src/components/VueWorkspaceShell.test.ts`

## Strengths Already Present

- the model is already centralized in `workflowModel.ts`
- defaults and allowed-tab sets are explicit instead of being inferred ad hoc in components
- mode-specific tab memory is already preserved
- root runtime dataset attributes already mirror workflow-store truth
- invalid inputs are already ignored safely

## Weaknesses / Failure Modes

- the workflow grammar is dense and easy to change without understanding all cross-field implications
- workspace/workflow/workflow-step overlap can confuse contributors who read only one layer
- subtitle derivation and runtime root-attribute sync are important but not yet treated as formal contract
- shell posture logic spans helpers, store actions, and shell rendering

## Regressions To Avoid

- breaking mode-specific tab memory between conventional and ML Signal
- allowing tabs that are not valid in the current workflow posture
- changing workspace/workflow-step transitions in ways that contradict the current shell grammar
- breaking subtitle derivation or root dataset sync
- making invalid values mutate state instead of being ignored
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WAI-ARIA tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)
  - relevant because the shell model ultimately drives multiple synchronized tablists and tabpanels
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because navigation grammar should stay centralized in shared state instead of leaking into component-local decisions
- [Pinia actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because workflow transitions are already action-driven and should stay explicit
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because the model’s safety depends on direct store-level regression coverage

## Proposed Improvement Approach

1. Freeze the shell grammar explicitly:
   - modes
   - workspaces
   - workflow steps
   - controls/results/aggregate tab families
2. Keep `workflowModel.ts` as the canonical model definition.
3. Document the transition rules in the store:
   - select mode
   - select workspace
   - select workflow
   - select workflow step
   - select controls/results/aggregate tab
4. Preserve root dataset sync and subtitle derivation as first-class outputs of the model.
5. If more modes or tabs are added later, extend the model centrally before changing shell components.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current shell grammar and normalized value sets.
- [x] Document default and allowed-tab rules for both modes.
- [x] Document mode-specific tab memory and restore behavior.
- [x] Document subtitle derivation and root dataset synchronization.
- [x] Add or tighten tests if new workspaces, steps, or tabs are introduced.
- [x] Keep invalid-value behavior non-mutating.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/stores/workflowStore.test.ts`
- run `npm --prefix web run test -- src/utils/core/workflowModel.test.ts`
- run `npm --prefix web run test -- src/components/VueWorkspaceShell.test.ts`
- manually verify workspace and workflow-step navigation in both conventional and ML Signal modes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the centralized model/store split
- if workflow grammar changes, update model helpers, store tests, and shell tests together
- prefer additive extension over hidden reinterpretation of existing tabs or steps
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/01_shell_app_root_2026-04-08.md`
- `REPO_Improvement/01_shell/02_shell_workspace_grid_2026-04-08.md`
- `REPO_Improvement/01_shell/10_settings_perspectives_2026-04-09.md`
- `REPO_Improvement/01_shell/15_state_artifacts_commands_2026-04-09.md`
