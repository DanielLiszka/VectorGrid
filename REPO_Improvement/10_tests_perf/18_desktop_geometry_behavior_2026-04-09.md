# Desktop Geometry Behavior Plan

Last updated: 2026-04-09

## Purpose

Define the opt-in desktop-smoke contract for deterministic resize behavior and post-resize shell usability.

## Scope Boundaries

In scope:

- the `desktop-window-geometry.spec.ts` opt-in suite
- real-shell resize, post-resize usability, and restore-to-original-geometry checks
- the boundary between geometry smoke and persisted window-state behavior
- environment tolerance expectations for geometry assertions on Windows

Out of scope:

- default stable desktop-smoke green-path requirements
- full benchmarking of resize performance
- options-related window flows
- maximize/restore titlebar behavior, which belongs to the titlebar doc

## Current User-Facing Behavior

The repo already keeps the geometry spec behind the opt-in `test:desktop:geometry` suite. The spec resizes the window to a deterministic target, waits for the size to land within a tolerance, checks that the core shell remains usable, opens settings, and then restores the original geometry. This is a useful native-runtime check, but it is intentionally not part of the default stable suite because window sizing is more environment-sensitive than ordinary shell smoke.

## Current Code Surface

- `web/e2e/desktop/specs/desktop-window-geometry.spec.ts` contains the current resize/restore journey.
- `web/wdio.desktop.conf.ts` keeps geometry in a separate opt-in suite.
- `web/e2e/desktop/support/waits.ts` and `selectors.ts` provide the shell and settings readiness helpers used after resize.
- `web/src/components/DesktopTitlebar.vue` and Tauri window integration influence the broader shell behavior around desktop window state.

## Current Verification Surface

- `npm --prefix web run test:desktop:geometry`
- `npm --prefix web run test:desktop:smoke:all` when running the full suite intentionally
- desktop artifact logs and screenshots when geometry operations fail

## Strengths Already Present

- the geometry spec is already isolated from the stable default smoke path
- it verifies both the resize itself and post-resize shell usability
- the tolerance-based check already acknowledges desktop-environment variability
- the restore step protects against one-way destructive geometry changes during a run

## Weaknesses / Failure Modes

- window geometry is inherently more environment-sensitive than most other desktop-smoke surfaces
- geometry success can depend on OS/window-manager timing outside the app itself
- the spec covers current usability after resize but not long-term persistence across relaunch
- if contributors misunderstand its role, they may promote it into mandatory green-path coverage too early

## Regressions To Avoid

- moving geometry into the stable default suite without evidence that it is reliable enough
- tightening tolerances unrealistically for desktop environments that do not honor exact pixels
- dropping the restore step and leaving the local environment altered after a run
- confusing geometry smoke with persisted window-state validation

## External Research Summary

- [Tauri window API reference](https://v2.tauri.app/reference/javascript/api/namespacewindow/) is relevant because resize and restore operations depend on real window lifecycle semantics.
- [Tauri window-state plugin docs](https://v2.tauri.app/plugin/window-state/) are relevant because geometry smoke sits adjacent to persisted window-state behavior but is not identical to it.
- [WebdriverIO best practices](https://webdriver.io/docs/bestpractices/) are relevant because geometry assertions need explicit waits and tolerant conditions rather than fixed timing.
- [Microsoft WebView2 user data folder guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder) is relevant because desktop shell environment state can influence repeated local runs.

## Proposed Improvement Approach

1. Keep geometry behavior as an opt-in suite until its stability clearly justifies broader inclusion.
2. Treat the spec as a shell-usability-after-resize check, not as the authority on persisted window-state correctness.
3. Preserve tolerance-based assertions and explicit restore logic.
4. Escalate repeated geometry instability into harness or environment analysis before turning the spec off or making it mandatory.

## Checklist

- [ ] Document the geometry suite as opt-in, not stable-default coverage.
- [ ] Keep resize assertions tolerance-based and environment-aware.
- [ ] Keep restore-to-original-geometry mandatory in the spec.
- [ ] Separate geometry smoke from window-state persistence concerns.
- [ ] Keep the geometry suite non-options only.
- [ ] Capture and review artifact logs whenever geometry behavior becomes flaky.

## Verification Plan

- run `npm --prefix web run test:desktop:geometry` after geometry or window-state related desktop changes
- review logs and screenshots when resize/restore behavior fails
- verify the shell remains usable after the resize step
- avoid treating geometry green-ness as a blocker for unrelated desktop-shell work unless the changed code touches window behavior directly

## Rollout / Fallback / Compatibility Notes

- keep geometry opt-in until stability data supports a wider role
- preserve the current restore behavior even if the assertion set evolves
- if geometry flakes, first improve waits and diagnostics before loosening the contract

## Related Docs

- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/15_desktop_native_titlebar_2026-04-09.md`
- `REPO_Improvement/09_live/08_system_gpu_window_surface_2026-04-09.md`
