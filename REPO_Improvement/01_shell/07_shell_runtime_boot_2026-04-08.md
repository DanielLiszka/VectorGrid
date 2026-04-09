# Shell Runtime Boot Plan

Last updated: 2026-04-08

## Purpose

Plan improvements to the frontend boot sequence, runtime-stage reporting, and boot-error presentation without reintroducing startup regressions or a blocking splash experience.

## Scope Boundaries

In scope:

- frontend boot stages
- runtime store semantics
- runtime banner behavior
- storage/settings/workflow bootstrap ordering
- desktop startup service initialization

Out of scope:

- re-enabling the startup animation overlay by default
- options workflows
- unrelated shell redesign

## Current User-Facing Behavior

The app mounts immediately, marks the runtime as booting, runs storage migrations plus settings/workflow bootstrap, initializes desktop productization, finalizes dataset readiness, then marks the frontend runtime ready. Failures are surfaced through the runtime banner and stored boot error fields.

## Current Code Surface

- `web/src/main.ts`
- `web/src/stores/runtimeStore.ts`
- `web/src/components/AppRuntimeBanner.vue`
- `web/src/services/settingsBootstrap.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/services/desktopProductization.ts`
- `web/src/App.vue`

## Current Verification Surface

- `web/src/main.test.ts`
- runtime-related component/store tests
- browser/desktop shell smoke coverage

## Strengths Already Present

- the boot sequence is centralized in `startVectorGridApp`
- the runtime store already records phase, stage, timestamps, and error details
- the banner already distinguishes booting vs error states
- the startup animation is already disabled, which reduces one failure surface

## Weaknesses / Failure Modes

- boot stages are stringly typed rather than cataloged
- the banner and the store can drift if stage naming changes casually
- desktop service startup failures are downgraded to warnings, which may hide useful diagnostic distinctions
- deferred dataset hydration and ready-state semantics need clearer guarantees

## Regressions To Avoid

- delaying shell mount behind a full-screen overlay again
- marking ready before deferred boot work is actually stable
- losing stack traces or desktop log visibility on startup failures
- changing boot text or stage semantics without updating tests and docs

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) supports centralizing shared runtime state instead of scattering boot flags across components.
- [Vue Composables](https://vuejs.org/guide/reusability/composables.html) supports moving shell-readiness and banner logic into explicit reusable boundaries when complexity increases.
- [Pinia State](https://pinia.vuejs.org/core-concepts/state.html) supports a stable initial-state contract for runtime boot fields.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces that startup work should avoid blocking the UI thread.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) supports verifying visible boot/error behavior rather than hidden implementation details.

## Proposed Improvement Approach

1. Define a stable boot-stage catalog and keep `runtimeStore` aligned with it.
2. Make `booting`, `ready`, and `error` transitions explicit and testable.
3. Clarify the difference between recoverable desktop startup warnings and fatal runtime failures.
4. Ensure the runtime banner remains the canonical visible boot surface while the startup overlay stays dormant.
5. Add coverage for success, degraded-startup, and fatal-boot paths.

## Checklist

- [ ] Document the exact stage order from mount to ready.
- [ ] Normalize stage names and ensure they are not duplicated ad hoc.
- [ ] Separate fatal boot errors from recoverable desktop warnings.
- [ ] Define when `markReady()` is allowed to fire.
- [ ] Define how deferred dataset bootstrap affects perceived readiness.
- [ ] Keep the runtime banner aligned with the store contract.
- [ ] Add or tighten tests for boot success, degraded startup, and boot failure.
- [ ] Verify that disabling the startup overlay remains the default behavior.

## Verification Plan

- run `npm test -- src/main.test.ts`
- run targeted runtime/banner/store tests
- run browser shell smoke if boot semantics or visible messaging changes
- run desktop smoke if window bootstrap or desktop productization changes

## Rollout / Fallback / Compatibility Notes

- keep the current instant-mount posture
- keep the startup overlay component available but not mounted by default
- preserve current local-storage bootstrap behavior unless explicitly changed

## Related Docs

- `REPO_Improvement/01_shell/35_runtime_banner_surface_DATE.md`
- `REPO_Improvement/01_shell/27_settings_service_bootstrap_DATE.md`
- `REPO_Improvement/00_meta/09_regression_safety_rules_2026-04-08.md`
