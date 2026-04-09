# Desktop Native Titlebar Plan

Last updated: 2026-04-09

## Purpose

Define the stable desktop-smoke contract around native titlebar controls and post-maximize shell responsiveness.

## Scope Boundaries

In scope:

- the `desktop-titlebar-native-controls.spec.ts` smoke contract
- real-shell availability of minimize, maximize, restore, and close controls exposed by the custom titlebar
- post-maximize/post-restore shell responsiveness checks
- the `DesktopTitlebar.vue` component as the primary shell surface under test

Out of scope:

- broad desktop window-management benchmarking
- close-guard policy details outside what the titlebar surface exposes
- options-related titlebar affordances
- full geometry persistence, which has its own doc

## Current User-Facing Behavior

The current desktop titlebar smoke spec launches the real shell, checks that the custom titlebar controls are enabled, clicks maximize, waits briefly, and either restores or at least confirms the shell remains responsive. On the implementation side, `DesktopTitlebar.vue` integrates with `@tauri-apps/api/window`, tracks maximize/focus state, and guards close requests when live work or protected tasks are still active. That makes the titlebar a meaningful desktop-only verification surface that browser smoke cannot replace.

## Current Code Surface

- `web/e2e/desktop/specs/desktop-titlebar-native-controls.spec.ts` holds the current real-shell titlebar assertions.
- `web/src/components/DesktopTitlebar.vue` owns titlebar actions, window listeners, and close-guard behavior.
- `web/e2e/desktop/support/selectors.ts` defines the titlebar control selectors used in smoke.
- `web/e2e/desktop/support/waits.ts` and `desktop-shell-readiness.spec.ts` provide the readiness foundation that this spec depends on.

## Current Verification Surface

- `npm --prefix web run test:desktop:smoke:spec -- .\e2e\desktop\specs\desktop-titlebar-native-controls.spec.ts`
- the default desktop smoke suite
- desktop artifact screenshots and logs when native controls fail or disappear

## Strengths Already Present

- the spec covers a genuinely desktop-only surface that browser smoke cannot prove
- the implementation already has meaningful close/maximize logic rather than decorative buttons
- the smoke path verifies both button availability and post-action shell health
- selectors are stable and user-facing via accessible labels

## Weaknesses / Failure Modes

- native window behavior can vary more across environments than ordinary browser DOM behavior
- the current maximize/restore flow uses a short pause before checking restore state, which is functional but not ideal
- titlebar behavior mixes shell UX with runtime/task guard logic, which can make failures harder to triage
- the custom titlebar is coupled to Tauri window APIs that are unavailable in browser smoke

## Regressions To Avoid

- allowing the titlebar spec to become a full close-guard or runtime-status integration suite
- removing meaningful post-action shell assertions so the test only checks that buttons exist
- using unstable styling selectors instead of accessible labels
- treating a flaky maximize check as harmless without understanding whether the bug is in the harness, the shell, or the desktop API bridge
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WebdriverIO best practices](https://webdriver.io/docs/bestpractices/) are relevant because native titlebar tests need resilient selectors and automatic waiting rather than timing guesses.
- [Tauri window API reference](https://v2.tauri.app/reference/javascript/api/namespacewindow/) is relevant because `DesktopTitlebar.vue` is driven by real window operations such as maximize, minimize, focus, and close handling.
- [Tauri window-state plugin docs](https://v2.tauri.app/plugin/window-state/) are relevant because maximize/restore and persisted window state are connected concerns in the desktop runtime.
- [WAI-ARIA tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant indirectly because titlebar actions coexist with shell navigation and must not break orientation after window actions.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep the titlebar smoke contract focused on control availability, one maximize/restore interaction, and shell responsiveness afterward.
2. Use the component implementation as a guide for which titlebar behaviors are contract-level versus incidental.
3. Strengthen waits where possible before accepting more timing-based logic.
4. Treat close-guard behavior as a related but separate verification concern unless a specific user-visible regression requires a dedicated desktop test.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the required titlebar controls and their accessible labels.
- [ ] Document the minimal maximize/restore responsiveness journey.
- [ ] Keep titlebar selectors aligned with the actual component accessibility contract.
- [ ] Review any titlebar implementation change against both browser shell assumptions and desktop smoke.
- [ ] Keep the titlebar smoke spec non-options only.
- [ ] Avoid turning titlebar smoke into a broader task-lifecycle integration suite.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the targeted titlebar spec after native-window or titlebar changes
- review logs and screenshots when maximize/restore behavior changes unexpectedly
- confirm the shell remains interactive after the window action sequence
- cross-check titlebar smoke against `DesktopTitlebar.vue` if selectors or behavior drift
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- keep the titlebar spec small and stable before adding any new native-window assertions
- if maximize/restore remains noisy, improve waits and triage before widening the assertions
- preserve close-guard functionality in the component even if the smoke contract stays narrower
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/14_desktop_shell_readiness_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/18_desktop_geometry_behavior_2026-04-09.md`
- `REPO_Improvement/09_live/08_system_gpu_window_surface_2026-04-09.md`
