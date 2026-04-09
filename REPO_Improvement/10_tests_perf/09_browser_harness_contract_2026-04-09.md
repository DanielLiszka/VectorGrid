# Browser Harness Contract Plan

Last updated: 2026-04-09

## Purpose

Define the role, limits, and support contract for the deterministic mocked-Tauri browser UX harness under `web/e2e/browser/`.

## Scope Boundaries

In scope:

- the Playwright browser harness in mocked-Tauri mode
- the harness bootstrap helpers in `web/e2e/browser/support/`
- the boundary between browser UX smoke and both lower-layer Vitest tests and higher-layer desktop smoke
- script, config, and artifact expectations for `test:browser:ux`

Out of scope:

- options-related browser flows
- real desktop shell proof, which belongs to WebdriverIO desktop smoke
- broad visual baseline capture, which belongs to `baseline:ui`
- component-level interaction minutiae that are already cheaper and more deterministic in Vitest

## Current User-Facing Behavior

The browser UX harness already builds the frontend, serves it through a local preview host, injects a mocked Tauri runtime via `addInitScript`, and runs a small set of Playwright specs against the built shell. It is intentionally lighter than the desktop harness and intentionally broader than individual component tests, but its ownership boundaries are currently implied by code and README text rather than pinned in a single implementation plan.

## Current Code Surface

- `web/e2e/browser/README.md` states the harness purpose and layer boundary.
- `web/playwright.config.ts` defines the browser-harness runtime, web server, artifacts, and failure-trace behavior.
- `web/e2e/browser/support/appHarness.ts` owns mocked-shell boot, shell-ready checks, and overlay helpers.
- `web/e2e/browser/support/mockTauri.ts` injects the minimal mocked-Tauri surface used by browser smoke.
- `web/package.json` exposes `test:browser:ux` and therefore the public execution contract for this harness.

## Current Verification Surface

- `npm --prefix web run test:browser:ux`
- failure artifacts under `output/playwright/browser-ux-artifacts`
- Playwright HTML report output in CI when enabled by config

## Strengths Already Present

- the harness already runs against the built app, which avoids many dev-server-only false positives
- the mocked runtime keeps the suite deterministic enough for keyboard and overlay journeys
- the current spec set is intentionally small and therefore easier to keep meaningful
- the harness already has explicit support helpers instead of embedding setup logic in every spec

## Weaknesses / Failure Modes

- the mocked runtime can drift away from real Tauri behavior if its contract is not explicitly limited
- browser smoke can easily become redundant with component tests if ownership is not maintained
- contributors may overestimate the meaning of browser smoke because it exercises the full shell in Chromium
- without a written contract, support helpers can gradually accrete more fake behaviors than the suite truly needs

## Regressions To Avoid

- moving real-desktop responsibilities into the mocked browser harness
- adding broad visual assertions or large snapshot inventories to what should stay a fast smoke layer
- letting the mock runtime silently grow unsupported commands or desktop-only assumptions
- duplicating Vitest coverage instead of targeting shell-level journeys that benefit from a real browser context
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant because the browser harness should validate user-visible flows, not internal implementation details.
- [Playwright isolation](https://playwright.dev/docs/browser-contexts) is relevant because each browser smoke spec should stay independent and not leak mocked runtime state across tests.
- [Playwright Trace Viewer](https://playwright.dev/docs/trace-viewer) is relevant because retained failure traces are a better debugging surface than ad hoc logging alone.
- [Tauri: Calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because the browser harness deliberately stands in for a very small subset of the Tauri bridge rather than the whole native runtime.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Define browser smoke as the deterministic app-level mocked-Tauri layer between Vitest and desktop smoke.
2. Keep the support mocks intentionally minimal and explicitly document which desktop/runtime behaviors are out of scope.
3. Require every browser smoke spec to justify why it belongs at shell level rather than component/service level.
4. Treat retained Playwright artifacts as debugging aids and review evidence, not as broad golden outputs.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the exact responsibilities of browser UX smoke.
- [ ] Document the explicit non-responsibilities that remain with Vitest or desktop smoke.
- [ ] Inventory every helper exported from `appHarness.ts` and `mockTauri.ts`.
- [ ] Keep the browser suite focused on deterministic shell journeys and overlay behavior.
- [ ] Keep the suite single-worker and deterministic unless there is strong evidence parallelism is safe.
- [ ] Review browser smoke additions for mock sprawl before accepting them as new coverage.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web run test:browser:ux` after browser-harness changes
- review failures using retained traces and screenshots rather than adding permanent sleeps or debug noise
- check that the harness still starts from a fresh built app and mocked runtime for each spec
- verify the harness remains non-options only
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current small suite while clarifying its ownership contract
- prefer tightening helper boundaries before expanding the number of specs
- if a test needs real Tauri behavior, move it to desktop smoke instead of enriching the browser mock
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/10_browser_shell_ux_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/12_browser_support_mocks_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
