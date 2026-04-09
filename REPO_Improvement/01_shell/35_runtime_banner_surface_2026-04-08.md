# Runtime Banner Surface Plan

Last updated: 2026-04-09

## Purpose

Plan additive improvements to the runtime banner so it becomes a stable, accessible, truth-preserving shell status surface for boot, degraded startup, and fatal runtime errors without slowing shell mount or reintroducing a blocking splash flow.

## Scope Boundaries

In scope:

- the visible runtime banner surface
- mapping from runtime store state to banner presentation
- boot, degraded-startup, and fatal-error messaging
- accessibility semantics, diagnostics affordances, and overlay precedence

Out of scope:

- re-enabling the startup animation overlay by default
- redesigning Task Center or Command Palette behavior
- adding a new notification system separate from the shell runtime banner

## Current User-Facing Behavior

The app mounts immediately, shows the runtime banner only while the boot phase is `booting` or `error`, and hides it once the runtime reaches `ready`. Boot stage text is driven from the runtime store and status-presentation helpers. Desktop startup issues can be downgraded to warning logs, which means the user may still reach `ready` even when some startup work degraded.

## Current Code Surface

- `web/src/main.ts`
- `web/src/stores/runtimeStore.ts`
- `web/src/components/AppRuntimeBanner.vue`
- `web/src/utils/statusPresentation.ts`
- `web/src/App.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/CommandPaletteOverlay.vue`

## Current Verification Surface

- `web/src/main.test.ts`
- `web/src/components/StartupAnimationOverlay.test.ts`
- shell component tests that mount `App.vue`
- browser/desktop shell smoke coverage

## Strengths Already Present

- boot sequencing is centralized in `startVectorGridApp`
- runtime status already has a dedicated store with phase, stage, timestamps, logs, and error details
- banner rendering is already isolated from the main workspace shell
- startup overlay is already disabled, so the banner is the only visible startup-status surface

## Weaknesses / Failure Modes

- boot phase and stage labels are stringly typed and easy to drift
- there is no explicit degraded-startup presentation contract separate from `ready`
- the banner lacks explicit ARIA live-region semantics
- overlay precedence is mostly implicit through render order and z-index
- users do not have a clear diagnostics escape hatch from the banner itself
- elapsed-time copy is derived but not treated as a first-class status field with explicit expectations

## Regressions To Avoid

- reintroducing a blocking or full-screen startup gate
- suppressing fatal diagnostics in the name of cleaner visuals
- creating noisy live-region behavior that spams assistive technologies
- allowing task-center or command-palette layering to obscure a critical runtime error
- marking the app `ready` earlier than the actual shell contract allows
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [MDN: `status` role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role) supports non-interruptive live updates for background status changes.
- [MDN: `alert` role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/alert_role) supports assertive announcement only for urgent, attention-demanding failures.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) reinforces keeping the state-to-presentation contract centralized instead of duplicating boot logic across components.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) supports verifying user-visible status behavior and layering through realistic shell flows rather than implementation-only tests.
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Define a stable runtime-banner state catalog that maps `booting`, degraded startup, and fatal error conditions to one presentation model.
2. Make the degraded-startup case explicit instead of relying on warning logs that disappear once the shell reaches `ready`.
3. Decide phase-specific accessibility semantics:
   - `role="status"` for normal boot progress and recoverable degraded states
   - `role="alert"` only for fatal startup/runtime failures
4. Document banner precedence against the dormant startup overlay, Task Center, and Command Palette so critical runtime status cannot be visually buried.
5. Add an intentional diagnostics affordance, such as a link or action that routes the user to the most relevant runtime diagnostics surface without forcing a redesign of the shell.
6. Tighten tests around successful boot, degraded startup, fatal startup, and overlay stacking.
7. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Lock a canonical runtime-banner state catalog and phase-to-copy mapping.
- [ ] Define whether degraded desktop initialization stays visible after `ready`.
- [ ] Document exact visibility rules for booting, degraded, and error states.
- [ ] Choose and document ARIA behavior for non-fatal vs fatal states.
- [ ] Define whether the banner exposes a direct diagnostics action and where it routes.
- [ ] Document stacking expectations against Task Center, Command Palette, and any future overlays.
- [ ] Add tests for boot success, degraded startup, and fatal startup presentation.
- [ ] Add smoke coverage that confirms the banner is visible when expected and absent when not expected.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/main.test.ts`
- run targeted banner/shell component tests
- run browser shell smoke that covers boot success and boot failure presentation
- run desktop smoke if window boot sequencing or overlay layering changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current instant-mount posture
- keep the startup overlay component dormant unless a separate decision reopens it
- treat degraded-startup messaging as additive evidence, not as a reason to remove current warning logs
- preserve existing runtime-store fields unless replacement fields are introduced with compatibility shims
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/07_shell_runtime_boot_2026-04-08.md`
- `REPO_Improvement/01_shell/27_settings_service_bootstrap_2026-04-08.md`
- `REPO_Improvement/01_shell/28_runtime_store_status_2026-04-08.md`
- `REPO_Improvement/01_shell/30_status_presentation_format_2026-04-08.md`
- `REPO_Improvement/00_meta/09_regression_safety_rules_2026-04-08.md`
