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

## External Research Summary

- [Vue Teleport](https://vuejs.org/guide/built-ins/teleport) is relevant because ML Signal shell ownership depends on correctly mounting the dedicated panel into the shared optimize host.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because shell mode and workspace ownership are shared application concerns rather than local component state.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because shell handoff behavior spans several reactive stores and should remain test-driven.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because history/apply flows and ML actions eventually cross the Rust boundary while preserving shell-mode expectations.
- [WAI-ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/) are relevant because mode-switching shells must keep action naming and focus expectations understandable.

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

## Checklist

- [ ] Document ML Signal shell-mode entry and exit triggers.
- [ ] Audit titlebar/footer/host-mount behavior while ML Signal mode is active.
- [ ] Add or tighten tests for shell-mode restoration after loading/applying ML Signal runs.
- [ ] Add or tighten tests ensuring conventional optimize content is suppressed when ML shell owns the mount.
- [ ] Verify action text and shell labels match the actual ML workspace capabilities.

## Verification Plan

- run `MlSignalShellPanel.test.ts`
- run `MlSignalWorkspacePanel.test.ts`
- run `DesktopTitlebar.test.ts`
- run relevant `historyService.test.ts` cases covering ML run loading/applying

## Rollout / Fallback / Compatibility Notes

- keep the dedicated ML shell model rather than merging it back into the conventional optimize panel
- prefer clearer mode ownership and tests over clever shared-shell abstractions
- if a shell cleanup makes ML mode ambiguous, restore the more explicit dedicated handoff

## Related Docs

- `REPO_Improvement/08_ml/03_ml_score_cache_bundles_2026-04-08.md`
- `REPO_Improvement/08_ml/11_mlsignal_dataset_export_2026-04-08.md`
- `REPO_Improvement/01_shell/01_shell_app_root_2026-04-08.md`
- `REPO_Improvement/01_shell/34_frontend_tauri_bridge_2026-04-08.md`
