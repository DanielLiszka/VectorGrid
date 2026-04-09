# Startup Overlay Disposition Plan

Last updated: 2026-04-09

## Purpose

Define the current startup-overlay disposition so the disabled-by-default shell posture is preserved while the retained overlay component remains available for deliberate future use.

## Scope Boundaries

In scope:

- root-level startup overlay gating in `App.vue`
- `StartupAnimationOverlay.vue` behavior and tests
- current relationship between overlay behavior and runtime banner/error surfaces

Out of scope:

- broader runtime boot sequencing
- unrelated splash or branding work
- options-specific startup flows

## Current User-Facing Behavior

Current shipped behavior is simple:

- the startup overlay component still exists
- `App.vue` still imports it
- `startupAnimationEnabled` is set to `false`
- the normal shell mounts immediately instead of waiting on the animation layer

The retained component is not dead code, though. It still has a real behavior contract:

- minimum visible duration
- signature-duration handoff
- skip/dismiss behavior
- reduced-motion posture
- debug hold flag
- error handoff into the runtime banner

So the current product disposition is: keep the implementation and tests, but do not mount it in normal startup.

## Current Code Surface

- `web/src/App.vue`
  - imports the overlay and keeps it disabled behind `startupAnimationEnabled = false`
- `web/src/components/StartupAnimationOverlay.vue`
  - defines intro/exiting/dismissed behavior, reduced-motion handling, skip behavior, and debug hold flag
- `web/src/components/StartupAnimationOverlay.test.ts`
  - verifies minimum visibility, signature timeout, skip, error exit, reduced motion, and debug hold
- `web/src/components/AppRuntimeBanner.vue`
  - current runtime boot/error fallback surface once the overlay is absent or dismissed

## Current Verification Surface

- `web/src/components/StartupAnimationOverlay.test.ts`
- `web/src/main.test.ts`
- shell-level behavior through `web/src/App.vue`

## Strengths Already Present

- the overlay is disabled in a simple, explicit way
- the component and tests were preserved instead of deleted
- reduced-motion and debug-hold behavior are already accounted for
- runtime banner fallback already exists for booting/error visibility when the overlay is absent

## Weaknesses / Failure Modes

- root disposition and component behavior are currently split across files without one clear contract
- the component can be mistaken for active production behavior because it remains imported
- future contributors could re-enable it casually without reviewing the current product decision
- overlay retention versus shipped posture is not yet documented in one place

## Regressions To Avoid

- re-enabling the overlay unintentionally
- deleting the overlay tests and losing the preserved behavior contract
- breaking reduced-motion or debug-hold behavior if the component is kept for later use
- confusing runtime-banner behavior with overlay behavior

## External Research Summary

- [MDN `prefers-reduced-motion`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion)
  - relevant because the retained overlay already reacts to reduced-motion preference
- [Vue performance best practices](https://vuejs.org/guide/best-practices/performance.html)
  - relevant because keeping startup path lean is one reason to leave the overlay disabled
- [MDN ARIA `status` role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because runtime startup/error messaging should remain available even without the overlay
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because startup-path regressions are better validated through realistic shell startup checks

## Proposed Improvement Approach

1. Freeze the current disposition explicitly:
   - retained component
   - disabled root mount
   - runtime banner as the live startup/error surface
2. Keep overlay behavior documented and test-backed even while disabled.
3. Treat any future re-enable as a deliberate product change, not a casual UI tweak.
4. Preserve reduced-motion and debug-hold behavior if the component remains in the repo.

## Checklist

- [ ] Document the current disabled-by-default root posture.
- [ ] Document the retained overlay behavior and why it still matters.
- [ ] Preserve reduced-motion, debug-hold, and error-handoff behavior in tests.
- [ ] Require deliberate review before re-enabling the overlay in `App.vue`.

## Verification Plan

- run `npm --prefix web test -- src/components/StartupAnimationOverlay.test.ts`
- run `npm --prefix web test -- src/main.test.ts`
- manually verify that normal startup still mounts the shell without showing the overlay

## Rollout / Fallback / Compatibility Notes

- keep the overlay disabled unless there is an explicit decision to ship it again
- if the component is eventually removed, archive or replace its behavior contract rather than silently dropping it
- if it is re-enabled later, validate both reduced-motion and runtime-banner handoff behavior again

## Related Docs

- `REPO_Improvement/01_shell/01_shell_app_root_2026-04-08.md`
- `REPO_Improvement/01_shell/07_shell_runtime_boot_2026-04-08.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
- `REPO_Improvement/01_shell/18_ux_a11y_copy_regression_2026-04-09.md`
