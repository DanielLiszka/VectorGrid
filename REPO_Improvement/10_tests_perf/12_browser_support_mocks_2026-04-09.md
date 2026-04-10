# Browser Support Mocks Plan

Last updated: 2026-04-10

## Purpose

Define the allowed scope and maintenance rules for the mocked-Tauri support layer used by the browser UX harness.

## Scope Boundaries

In scope:

- the browser-harness support files `appHarness.ts` and `mockTauri.ts`
- mocked event, callback, and startup-overlay behavior used only for browser UX smoke
- the boundary between browser smoke support mocks and the broader frontend test-mock infrastructure
- minimal mocked desktop metadata required for browser harness boot

Out of scope:

- the broader Vitest mock layer documented separately in `50_frontend_test_support_mocks`
- real desktop runtime behavior
- options-related mocks
- mock implementations for commands or plugins the browser smoke suite does not actually need

## Current User-Facing Behavior

The browser harness currently injects a narrow mocked Tauri runtime through `addInitScript`, exposing an `invoke` stub, callback registration helpers, a minimal metadata payload, and a lightweight test event emitter. This is enough to boot the shell and drive overlay and navigation smoke tests, but the repo needs a written implementation plan that prevents this support layer from accreting fake desktop behavior simply because it is convenient.

## Current Code Surface

- `web/e2e/browser/support/mockTauri.ts` defines the mocked Tauri runtime injected into each Playwright browser context.
- `web/e2e/browser/support/appHarness.ts` depends on those mocks for deterministic shell boot and overlay helpers.
- `web/playwright.config.ts` ensures the browser harness remains single-worker and deterministic enough for this mock layer.
- `web/src/tauri/invoke.ts` is the real bridge surface whose public assumptions the browser mock must not silently contradict.

## Current Verification Surface

- `npm --prefix web run test:browser:ux`
- targeted browser-harness failures where the mocked runtime no longer boots the shell
- comparison against real desktop smoke when the mock contract is under question

## Strengths Already Present

- the current mock layer is intentionally small and focused on shell boot plus a small event bridge
- context-level injection keeps tests isolated from each other
- the harness already exposes a test-event emitter rather than reimplementing a large swath of backend behavior
- startup-overlay hold/dismiss support lets browser smoke cover boot surfaces deterministically

## Weaknesses / Failure Modes

- the browser mock layer sits close to the real Tauri bridge and can drift if nobody owns the boundary
- a null-returning `invoke` stub can hide missing command expectations if a new shell path begins depending on real data
- helper growth is tempting because it is faster than moving a scenario to desktop smoke
- there is not yet a dedicated plan tying browser mocks back to the real bridge contract

## Regressions To Avoid

- letting browser support mocks become an unofficial second Tauri implementation
- adding command-specific fake data just to make smoke tests pass
- sharing mutable mocked state across specs
- masking a real desktop regression by making the browser mock more permissive
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Playwright browser contexts](https://playwright.dev/docs/browser-contexts) are relevant because the browser harness relies on per-context initialization and isolation.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant because mocks should support focused user-level scenarios, not recreate the whole application backend.
- [Tauri: Calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because the mock layer should stay aligned with the public semantics of the real invoke bridge.
- [Tauri capabilities overview](https://v2.tauri.app/security/capabilities/) is relevant because browser mocks should not encourage assumptions about unrestricted desktop access that the real app does not have.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Document a narrow allowed surface for the browser mock layer: shell boot, event listener plumbing, minimal metadata, and startup overlay control.
2. Require any scenario that needs richer native behavior to move into desktop smoke rather than expanding the browser mock indiscriminately.
3. Keep browser mocks isolated per context and make their state intentionally easy to reason about.
4. Review browser mock changes against the real `invoke` and desktop runtime contracts before accepting them.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Inventory every mocked global and helper exported by the browser support layer.
- [x] Document which mocked behaviors are required for shell boot and which are optional conveniences.
- [x] Define when a new behavior belongs in desktop smoke instead of the browser mock.
- [x] Keep browser support mocks isolated per Playwright context.
- [x] Keep the browser support layer non-options only.
- [x] Cross-check mock assumptions against the real Tauri bridge when shell boot paths change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web run test:browser:ux` after mock-layer changes
- compare suspicious browser-only passes against desktop smoke behavior before broadening mocks
- inspect the real `web/src/tauri/invoke.ts` surface when browser support code changes
- verify the mocked startup-overlay flow still matches the current shell boot expectations
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- prefer subtraction and tightening over growth in the browser mock layer
- if a mock becomes too rich, split it or move the scenario into desktop smoke
- avoid hidden compatibility shims in browser support helpers
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on current-state mock-layer evidence; no browser support mock code change was required for this item.
- Current mocked globals / helpers inventory:
  - `mockTauri.ts`
    - `installMockedTauri(context, options)`
    - `dismissStartupOverlay(page)`
    - injected globals: `window.isTauri`, `window.__TAURI_INTERNALS__`, `window.__TAURI_EVENT_PLUGIN_INTERNALS__`, `window.__TAURI__`, `window.__VTA_TEST_EMIT`, `window.__VTA_TEST_INVOKES`
  - `appHarness.ts`
    - `gotoMockedShell`
    - `waitForShellReady`
    - `openCommandPalette`
    - `closeCommandPaletteWithEscape`
    - `openTaskCenter`
    - `closeTaskCenterWithEscape`
- Required shell-boot behaviors versus optional conveniences:
  - required:
    - per-context `addInitScript` injection
    - minimal `invoke`, event listen/unlisten, callback transform/unregister plumbing
    - minimal `currentWindow/currentWebview` metadata
    - startup overlay hold/dismiss path needed for deterministic shell boot
  - optional convenience:
    - `commandResults` overrides for narrowly scoped command responses
    - lightweight `__VTA_TEST_EMIT` test event emitter
  - protective default:
    - `strictUnknownInvoke` stays `true` by default so new shell boot paths fail loudly instead of silently returning `null`
- Desktop-smoke handoff rule remains explicit:
  - if a scenario needs richer native window/plugin/runtime behavior than the minimal boot bridge above, it belongs in desktop smoke instead of expanding the browser mock
- Isolation/contract posture remains healthy:
  - mocks are injected per Playwright browser context
  - browser support layer stays non-options only
  - recent shell boot tightening did not require broadening the mock surface
- Cross-check posture against the real bridge:
  - keep `web/src/tauri/invoke.ts` as the authority when shell boot paths change
  - do not let browser-only conveniences redefine real invoke expectations
- Cross-doc sync:
  - no `README.md` or `REPO_MAP.md` update was required because the mock contract already matched the current implementation and recent tightening only reinforced the existing boundary
- Acceptance evidence:
  - `npm.cmd --prefix web run test:browser:ux`
  - supporting browser-harness checks from the same tranche:
    - `npm.cmd --prefix web run a11y:smoke`
    - `npm.cmd --prefix web run baseline:ui`

## Related Docs

- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/11_browser_palette_overlays_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/50_frontend_test_support_mocks_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/21_tauri_mock_runtime_smoke_2026-04-08.md`
