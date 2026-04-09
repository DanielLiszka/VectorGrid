# Settings Perspectives Plan

Last updated: 2026-04-09

## Purpose

Define the current saved-perspectives contract so workstation snapshots, local perspective search, and snapshot restore behavior can be improved without breaking the shell-state workflow users already have.

## Scope Boundaries

In scope:

- saved perspectives in `SettingsWorkspacePanel.vue`
- perspective snapshot lifecycle in `settingsStore.ts`
- local perspective search and perspective mutation actions

Out of scope:

- global command search
- appearance/layout settings outside perspective snapshot data
- cloud sync or multi-device perspective sharing

## Current User-Facing Behavior

Settings already exposes a meaningful saved-perspectives feature:

- built-in perspectives ship with the app
- users can create a perspective from the current shell posture
- users can overwrite, restore, rename, duplicate, and delete perspectives
- perspective search is local to saved perspectives and explicitly not a global command search
- restoring a perspective reapplies mode, workspace, workflow step, tabs, split fraction, and density

Built-in perspectives are already treated differently from user-created ones:

- they can be restored or duplicated
- they are protected from rename/delete actions

So perspectives already function as a reusable shell-state snapshot system.

## Current Code Surface

- `web/src/components/SettingsWorkspacePanel.vue`
  - perspective UI, local search summary, mutation actions, and operator-facing copy
- `web/src/components/SettingsWorkspacePanel.test.ts`
  - verifies perspective controls and local-search posture
- `web/src/stores/settingsStore.ts`
  - built-in perspective inventory plus create/overwrite/apply/rename/duplicate/delete logic
- `web/src/stores/workflowStore.ts`
  - receives restored snapshot state through `applyLegacySnapshot()` and `commitState()`
- `web/src/utils/perspectiveSearch.ts`
  - perspective-local search behavior

## Current Verification Surface

- `web/src/components/SettingsWorkspacePanel.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`
- `web/src/stores/workflowStore.test.ts`

## Strengths Already Present

- perspective snapshots already capture the real shell state users care about
- built-in versus user-created perspectives are already distinguished
- local search posture is explicit and tested
- restore behavior already routes through the current workflow store rather than bypassing state rules
- duplicate and overwrite flows already exist, so the feature is more complete than a simple bookmark list

## Weaknesses / Failure Modes

- the snapshot schema is real but not documented as contract
- perspective mutation rules for built-in versus user-created entries are implicit unless you read the store
- restore behavior depends on both settings and workflow stores, which makes ownership easy to blur
- search, mutation, and restore rules all live in one surface without a dedicated implementation doc

## Regressions To Avoid

- breaking restore of mode/workspace/tabs/split fraction/density
- letting global-search behavior leak into perspective search
- removing built-in perspective protections accidentally
- mutating workflow state outside the current workflow-store contract during restore
- weakening perspective persistence or selection continuity
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Pinia actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because perspective lifecycle is already store-driven and should stay centralized
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because restoring perspectives crosses settings and workflow state but still needs one predictable authority per concern
- [WAI-ARIA listbox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/)
  - relevant because saved perspectives are selected and filtered through a list-style control surface
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because perspective restore and mutation flows should remain directly testable at the store layer

## Proposed Improvement Approach

1. Freeze the perspective snapshot schema explicitly.
2. Document the action model separately:
   - create
   - overwrite
   - apply
   - rename
   - duplicate
   - delete
3. Preserve the distinction between built-in and user-created perspectives.
4. Keep perspective search local and perspective-specific.
5. Add tighter restore-path coverage if snapshot fields expand later.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current perspective snapshot fields.
- [ ] Document built-in versus user-created perspective rules.
- [ ] Document the full mutation lifecycle and current status-line behavior.
- [ ] Document local perspective-search scope and ranking expectations.
- [ ] Verify restore still routes through workflow-store state transitions.
- [ ] Add or tighten tests if the snapshot schema or mutation rules change.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/SettingsWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/settingsStore.actions.test.ts`
- run `npm --prefix web run test -- src/stores/workflowStore.test.ts`
- manually verify create, restore, duplicate, rename, and delete flows
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current built-in perspective ids and protections unless a migration exists
- keep search local to saved perspectives
- if snapshot fields are expanded later, make the additions additive and backward-compatible with persisted JSON
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/09_settings_appearance_layout_2026-04-09.md`
- `REPO_Improvement/01_shell/14_state_workflow_tab_model_2026-04-09.md`
- `REPO_Improvement/01_shell/29_storage_schema_local_state_2026-04-08.md`
- `REPO_Improvement/00_meta/10_verification_expectations_2026-04-08.md`
