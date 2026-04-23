# ML Shell Handoff Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the ML Signal shell handoff so the application switches cleanly between the conventional optimization shell and the dedicated ML Signal workspace without losing context, mislabeled actions, or stale runtime affordances.

## Scope Boundaries

In scope:

- `web/src/components/MlSignalShellPanel.vue`
- `web/src/components/MlSignalShellPanel.test.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/services/historyService.ts`
- `web/src/services/mlSignalService.ts`
- workflow/shell mode state that decides when ML Signal owns the optimize surface

Out of scope:

- ML model training/scoring internals
- options-related shell behavior
- redesigning the entire workspace system

## Current User-Facing Behavior

ML Signal already takes over the optimize shell in a dedicated way:

- `MlSignalShellPanel` teleports the ML workspace into the optimize host mount
- the titlebar switches to ML-aware labels such as “Build score cache”
- history/apply flows can switch the workflow into ML Signal mode
- ML Signal actions operate through their own workspace panel instead of sharing the conventional optimization form

The visible promise is strong: when the app is in ML Signal mode, the shell should feel intentionally different, not half-conventional and half-ML.

## Current Code Surface

- `web/src/components/MlSignalShellPanel.vue`
- `web/src/components/MlSignalShellPanel.test.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/services/historyService.ts`
- `web/src/services/mlSignalService.ts`
- `web/src/stores/workflowStore.ts`

## Current Verification Surface

- `web/src/components/MlSignalShellPanel.test.ts`
- `web/src/components/MlSignalWorkspacePanel.test.ts`
- `web/src/components/DesktopTitlebar.test.ts`
- `web/src/services/historyService.test.ts`

## Strengths Already Present

- ML Signal already has a dedicated shell panel instead of being bolted awkwardly onto the conventional optimize panel
- shell mounting is explicit through `useShellMount`
- mode-aware titlebar labels already exist
- history/apply flows already know how to switch shell mode for ML Signal runs
- tests already cover basic mount and label behavior

## Weaknesses / Failure Modes

- shell ownership is split across multiple components and services
- mode switching can become stale if one surface updates later than another
- titlebar/action text can drift from the actual ML workspace capabilities
- the optimize host mount is shared infrastructure, so ML-specific shell behavior can regress indirectly through generic shell changes

## Regressions To Avoid

- showing conventional optimization actions while ML Signal mode is active
- mounting the ML workspace without fully suppressing conflicting conventional surfaces
- losing shell-mode restoration when loading or applying ML Signal runs
- breaking the dedicated optimize-host mount behavior while refactoring the shell
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue Teleport](https://vuejs.org/guide/built-ins/teleport) is relevant because ML Signal shell ownership depends on correctly mounting the dedicated panel into the shared optimize host.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because shell mode and workspace ownership are shared application concerns rather than local component state.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because shell handoff behavior spans several reactive stores and should remain test-driven.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because history/apply flows and ML actions eventually cross the Rust boundary while preserving shell-mode expectations.
- [WAI-ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/) are relevant because mode-switching shells must keep action naming and focus expectations understandable.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Treat ML Signal shell handoff as a mode-ownership contract.
2. Document the authoritative mode transition triggers:
   - user navigation into ML Signal
   - loading/applying ML Signal artifacts
   - returning to conventional strategy work
3. Audit all visible shell affordances in ML mode:
   - titlebar copy
   - footer actions
   - optimize-host panel content
   - workflow mode state
4. Add tests that verify shell consistency rather than only single-component rendering.
5. Keep ML Signal shell ownership additive and isolated instead of entangling it more deeply with the conventional optimize layout.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document ML Signal shell-mode entry and exit triggers.
- [x] Audit titlebar/footer/host-mount behavior while ML Signal mode is active.
- [x] Add or tighten tests for shell-mode restoration after loading/applying ML Signal runs.
- [x] Add or tighten tests ensuring conventional optimize content is suppressed when ML shell owns the mount.
- [x] Verify action text and shell labels match the actual ML workspace capabilities.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `MlSignalShellPanel.test.ts`
- run `MlSignalWorkspacePanel.test.ts`
- run `DesktopTitlebar.test.ts`
- run relevant `historyService.test.ts` cases covering ML run loading/applying
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- keep the dedicated ML shell model rather than merging it back into the conventional optimize panel
- prefer clearer mode ownership and tests over clever shared-shell abstractions
- if a shell cleanup makes ML mode ambiguous, restore the more explicit dedicated handoff
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/03_ml_score_cache_bundles_2026-04-08.md`
- `REPO_Improvement/08_ml/11_mlsignal_dataset_export_2026-04-08.md`
- `REPO_Improvement/01_shell/01_shell_app_root_2026-04-08.md`
- `REPO_Improvement/01_shell/34_frontend_tauri_bridge_2026-04-08.md`

## Execution Notes

- Closed by making ML shell ownership explicit instead of mode-only.
- Shared ML primary actions are now derived from the workflow step:
  - `data` -> Export dataset
  - `optimize` -> Build score cache
  - `validate` -> Run ML validation
  - `analyze` -> Run ML policy sweep
  - `finalize` -> inspect-only with no shared dispatch
- `historyService.ts` now restores the ML workflow step during load/apply/rerun using persisted `ml_signal.stage` when available and a request-shape fallback otherwise.
- `DesktopTitlebar.vue`, `RunFooterPanel.vue`, `optimizeStore.ts`, and `workflowModel.ts` now share the same ML step/action contract.
- Shell consistency coverage now includes titlebar/footer labels, disabled finalize behavior, saved-run ML step restoration, and adjacent ML shell panels.
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/DesktopTitlebar.test.ts src/components/RunFooterPanel.test.ts src/stores/optimizeStore.actions.test.ts src/services/historyService.test.ts src/utils/core/workflowModel.test.ts`
  - `npm.cmd --prefix web run test -- src/components/MlSignalWorkspacePanel.test.ts src/components/MlSignalShellPanel.test.ts src/components/VueWorkspaceShell.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
