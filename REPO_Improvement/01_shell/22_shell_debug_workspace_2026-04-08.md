# Shell Debug Workspace Plan

Last updated: 2026-04-08

## Purpose

Define the current debug workspace contract so raw JSON troubleshooting surfaces can be improved without leaking them into the normal review flow or breaking the current pro/debug posture.

## Scope Boundaries

In scope:

- debug workspace shell routing
- `DebugWorkspacePanel` behavior and mount posture
- raw analysis payload preview surface
- current status/timestamp/debug-json presentation

Out of scope:

- general analysis result UX
- developer console logging
- options-specific debug panels

## Current User-Facing Behavior

The shell already exposes a debug surface for raw analysis payloads. Current behavior is:

- debug workspace is treated as a specialized shell surface, not part of the main review flow
- `App.vue` mounts the debug panel when the active results tab is `debug`
- `VueWorkspaceShell` routes `pro` workspace posture to the `debug` results surface
- the panel teleports into the debug shell mount target
- the panel shows raw analysis JSON plus status and timestamp

The panel copy already makes its intent clear: this is for troubleshooting and exact payload inspection, not for routine analysis review.

## Current Code Surface

- `web/src/components/DebugWorkspacePanel.vue`
  - teleported debug panel showing raw analysis payload preview
- `web/src/App.vue`
  - mounts the debug panel when `debugPanelActive`
- `web/src/components/VueWorkspaceShell.vue`
  - exposes the shell posture that makes the debug results tab reachable
- `web/src/stores/analysisStore.ts`
  - supplies `debugOutputPreview`, status, and update timestamp consumed by the panel
- `web/src/composables/useShellMount.ts`
  - handles the debug mount target readiness

## Current Verification Surface

- shell-routing coverage through `web/src/components/VueWorkspaceShell.test.ts`
- indirect runtime behavior through `web/src/App.vue`
- currently limited direct component-test coverage for the debug panel itself

## Strengths Already Present

- the debug surface is already clearly separated from the main review flow
- raw JSON remains available when exact payload inspection is needed
- the panel already uses the shared shell mount model
- the panel copy already sets correct expectations about troubleshooting intent
- shell posture already treats debug as a specialized workspace/result surface

## Weaknesses / Failure Modes

- there is little direct test coverage for the debug panel itself
- the debug surface contract is mostly implicit across shell routing and one panel
- raw JSON availability and formatting expectations are not documented in one place
- future UI cleanup could hide or break the debug route because it is more niche than other workspaces
- there is no dedicated doc that states what belongs in this workspace versus the regular analysis flow

## Regressions To Avoid

- leaking debug JSON into the main review surfaces
- making the debug workspace unreachable from the intended shell posture
- breaking raw JSON visibility or status/timestamp context
- removing the debug panel’s troubleshooting-only framing
- letting debug-shell changes bypass the shared shell mount contract

## External Research Summary

- [Vue Teleport](https://vuejs.org/guide/built-ins/teleport)
  - relevant because the debug panel is mounted into a shell slot through teleport
- [RFC 8259 JSON](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because the panel intentionally exposes raw JSON payloads for troubleshooting
- [MDN ARIA `status` role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because the panel already surfaces a live status line alongside the raw payload
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the debug surface should continue consuming centralized analysis state instead of inventing its own payload authority

## Proposed Improvement Approach

1. Freeze the debug workspace as a troubleshooting-only shell surface.
2. Document the routing path that makes it reachable from the shell.
3. Preserve the raw JSON plus status/timestamp contract.
4. Add direct component coverage for `DebugWorkspacePanel` if the panel evolves further.
5. Keep the debug surface separated from user-facing mainstream analysis flows.

## Checklist

- [ ] Document the current shell route into the debug workspace.
- [ ] Document the raw JSON/status/timestamp contract of the debug panel.
- [ ] Add direct component-test coverage for `DebugWorkspacePanel` if it changes materially.
- [ ] Preserve the troubleshooting-only framing of the debug workspace.
- [ ] Keep the debug panel mounted through the shared shell mount contract.

## Verification Plan

- run `npm --prefix web test -- src/components/VueWorkspaceShell.test.ts`
- add or run a direct `DebugWorkspacePanel` component test if the panel changes
- manually verify the debug route, raw JSON visibility, and status line in the shell

## Rollout / Fallback / Compatibility Notes

- preserve current debug-route accessibility unless a deliberate shell simplification is approved
- keep raw JSON available for troubleshooting while the workspace exists
- if the debug surface expands later, keep mainstream analysis surfaces clean and non-duplicative
- update this doc and any new tests whenever the debug workspace contract changes

## Related Docs

- `REPO_Improvement/01_shell/01_shell_app_root_2026-04-08.md`
- `REPO_Improvement/01_shell/02_shell_workspace_grid_2026-04-08.md`
- `REPO_Improvement/01_shell/16_ux_shell_mount_readiness_2026-04-08.md`
- `REPO_Improvement/01_shell/18_ux_a11y_copy_regression_2026-04-08.md`
