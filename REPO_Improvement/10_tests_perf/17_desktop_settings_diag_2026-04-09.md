# Desktop Settings Diagnostics Plan

Last updated: 2026-04-09

## Purpose

Define the desktop-smoke contract for the settings workspace diagnostics that only make sense in the real desktop runtime.

## Scope Boundaries

In scope:

- the `desktop-settings-diagnostics.spec.ts` smoke contract
- desktop notifications, logs, task-center controls, and window-state/status text shown in settings
- the settings workspace diagnostics area as rendered in the real shell
- cross-checks between settings diagnostics and desktop runtime/plugin availability

Out of scope:

- general settings form behavior already covered elsewhere
- options-related settings surfaces
- deep runtime-store correctness beyond the user-visible diagnostics summary
- native notification delivery benchmarking

## Current User-Facing Behavior

The current desktop settings diagnostics smoke spec opens the settings workspace in the real shell and checks for a cluster of desktop-specific diagnostics and controls, including notification posture, logs, task-center affordances, close/minimize messaging, desktop presence copy, and window-state detail. This makes the settings workspace an important real-shell readout of desktop capabilities and runtime posture that mocked browser tests cannot fully validate.

## Current Code Surface

- `web/e2e/desktop/specs/desktop-settings-diagnostics.spec.ts` contains the current diagnostics smoke journey.
- `web/src/components/SettingsWorkspacePanel.vue` renders the desktop diagnostics grid and supporting copy.
- `web/src/services/desktopProductization.ts` provides desktop-notification and native-runtime helpers surfaced in settings.
- `web/src/stores/runtimeStore.ts` and `web/src/stores/settingsStore.ts` feed the diagnostics surface with runtime and persistence state.

## Current Verification Surface

- `npm --prefix web run test:desktop:smoke:spec -- .\e2e\desktop\specs\desktop-settings-diagnostics.spec.ts`
- the default desktop smoke suite
- targeted component/store tests when a diagnostics failure is rooted in frontend state rather than native-shell behavior

## Strengths Already Present

- the spec protects a distinctly desktop-facing surface rather than duplicating browser-only coverage
- the settings diagnostics area aggregates multiple important runtime cues in one visible panel
- the current assertions already look for meaningful desktop concepts rather than only the existence of the settings grid
- the surface is useful for user trust because it explains what the desktop runtime is doing

## Weaknesses / Failure Modes

- the current assertions are still somewhat copy-heavy and could get noisy if phrasing evolves
- desktop diagnostics combine several plugins and runtime signals, which can make failures multi-causal
- browser harnesses cannot fully replicate this surface, so desktop smoke must stay disciplined and stable
- the settings panel is large, so a narrow diagnostics contract is necessary to avoid overcoverage

## Regressions To Avoid

- removing desktop-specific posture cues from the smoke contract and leaving only generic settings-grid checks
- asserting on every line of explanatory copy rather than the stable diagnostics concepts
- treating settings diagnostics as a replacement for lower-layer runtime/store tests
- allowing the diagnostics surface to drift away from actual desktop capabilities or plugin state

## External Research Summary

- [Tauri notification plugin docs](https://v2.tauri.app/plugin/notification/) are relevant because settings diagnostics surface notification posture and constraints in the desktop runtime.
- [Tauri window-state plugin docs](https://v2.tauri.app/plugin/window-state/) are relevant because settings diagnostics expose persisted window-state readiness and file/path messaging.
- [WebdriverIO best practices](https://webdriver.io/docs/bestpractices/) are relevant because diagnostics smoke should assert on stable concepts with resilient selectors and waits.
- [WAI-ARIA dialog (modal) pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) is relevant indirectly because settings diagnostics also reference the task center and other overlay-driven desktop affordances.

## Proposed Improvement Approach

1. Keep desktop settings diagnostics smoke focused on stable desktop-runtime concepts rather than full-copy snapshots.
2. Tie each assertion to a concrete desktop capability or runtime status that users depend on.
3. Use lower-layer tests for store/service correctness and let desktop smoke prove that the real shell surfaces those signals coherently.
4. Review wording churn separately from capability regressions so the smoke contract stays useful.

## Checklist

- [ ] Document the required desktop diagnostics concepts visible in settings.
- [ ] Map those concepts back to the runtime/plugin sources that feed them.
- [ ] Reduce reliance on volatile explanatory text where a stronger semantic signal exists.
- [ ] Keep settings diagnostics smoke non-options only.
- [ ] Use desktop smoke to prove real-shell visibility, not every runtime permutation.
- [ ] Review capability wording whenever plugin behavior or runtime policies change.

## Verification Plan

- run the targeted diagnostics spec after settings or desktop-runtime changes
- run the full desktop smoke suite after broader shell updates
- review whether a failing assertion reflects wording churn or an actual capability regression
- cross-check runtime and settings stores when the desktop shell no longer surfaces expected diagnostics

## Rollout / Fallback / Compatibility Notes

- preserve the current diagnostics smoke intent while gradually tightening assertion quality
- prefer concept-level assertions over removing diagnostics coverage entirely
- if settings diagnostics expand, add contract coverage selectively instead of snapshotting the whole panel

## Related Docs

- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/16_desktop_task_center_nav_2026-04-09.md`
- `REPO_Improvement/09_live/08_system_gpu_window_surface_2026-04-09.md`
- `REPO_Improvement/09_live/11_settings_secret_storage_2026-04-09.md`
