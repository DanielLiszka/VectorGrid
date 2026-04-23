# Frontend Tauri Bridge Plan

Last updated: 2026-04-09

## Purpose

Define the frontend Tauri bridge contract so command invocation, event subscription, runtime detection, instrumentation hooks, and desktop-productization integration can be improved without breaking the typed command boundary or allowing raw Tauri drift across the frontend.

## Scope Boundaries

In scope:

- `web/src/tauri/commands.ts`
- `web/src/tauri/invoke.ts`
- `web/src/tauri/events.ts`
- `web/src/tauri/instrumentation.ts`
- `web/src/tauri/runtimeError.ts`
- `web/src/test/tauriMocks.ts`
- `web/src/services/desktopProductization.ts`
- `src-tauri/src/lib.rs` only as the upstream command inventory source

Out of scope:

- Rust-side Tauri command implementations
- options-specific frontend Tauri flows
- WebdriverIO/E2E desktop harness details

## Current User-Facing Behavior

Users do not see this bridge directly, but many visible shell behaviors depend on it. Optimizations, validation, data loading, report open/reveal flows, runtime event progress, task-center instrumentation, desktop logs, and mock-runtime tests all rely on the same frontend Tauri wrappers. The current bridge already enforces typed command names, normalizes invoke failures into `TauriRuntimeError`, shares one native listener per event channel, and instruments tracked commands/events into the task lifecycle store. `desktopProductization.ts` is the parallel plugin bridge for desktop-specific plugins.

## Current Code Surface

- `web/src/tauri/commands.ts`
  - auto-generated registry of allowed Tauri command names
- `web/src/tauri/invoke.ts`
  - typed invoke wrapper, runtime detection, and lifecycle hook support
- `web/src/tauri/events.ts`
  - shared event-listener wrapper with handler fan-out and cleanup
- `web/src/tauri/instrumentation.ts`
  - task lifecycle instrumentation for invoke commands and event channels
- `web/src/tauri/runtimeError.ts`
  - normalized error type used by the bridge
- `web/src/test/tauriMocks.ts`
  - mock IPC harness and legacy bridge helpers for tests
- `web/scripts/generate-tauri-commands.mjs`
  - generates `commands.ts` from `src-tauri/src/lib.rs`
- `web/scripts/check-known-tauri-commands.mjs`
  - checks literal command strings in frontend/scripts code against the generated registry
- `web/src/services/desktopProductization.ts`
  - parallel plugin bridge for log, notification, and window-state plugins

## Current Verification Surface

- `web/src/tauri/commands.test.ts`
- `web/src/tauri/invoke.contract.test.ts`
- `web/src/tauri/invoke.mockIPC.test.ts`
- `web/src/tauri/events.test.ts`
- `web/src/tauri/instrumentation.mockIPC.test.ts`
- `web/src/services/desktopProductization.test.ts`

## Strengths Already Present

- typed command names already exist instead of free-form strings everywhere
- invoke failures are already normalized consistently
- event listeners already share a native subscription per channel
- instrumentation is already centralized instead of duplicated per service
- mock-runtime test support already exercises the same bridge layer the app uses

## Weaknesses / Failure Modes

- raw `@tauri-apps/api/*` usage could still creep in outside the bridge if not documented as a rule
- generated command inventory can drift if regeneration/checking is skipped
- bridge ownership is split across command registry, invoke wrapper, events wrapper, instrumentation, and plugin bridge code
- lifecycle-hook install/reset and event-subscription retry behavior are easy to miss because they are thin infrastructure
- plugin bridge failures currently do not share the same normalized error path as invoke failures

## Regressions To Avoid

- bypassing `invokeCommand()` with raw `invoke()` in normal app code
- allowing unknown literal command names to avoid the registry
- breaking one-native-listener-per-event behavior
- weakening error normalization or typed-runtime detection
- drifting instrumentation channels away from real backend event channels
- desynchronizing `commands.ts` from `src-tauri/src/lib.rs`
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the invoke wrapper is the primary frontend-to-Rust boundary
- [Tauri: Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/)
  - relevant because shared event listeners are the frontend side of backend event delivery
- [Tauri JS Core API](https://v2.tauri.app/reference/javascript/api/namespacecore/)
  - relevant because `invoke()` and runtime detection semantics ultimately come from this API
- [Tauri JS Event API](https://v2.tauri.app/reference/javascript/api/namespaceevent/)
  - relevant because event subscription semantics underpin `events.ts`
- [Tauri JS Mocks API](https://v2.tauri.app/reference/javascript/api/namespacemocks/)
  - relevant because mock-runtime coverage should stay aligned with official mock APIs
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Treat the frontend Tauri layer as a documented boundary with five responsibilities:
   - generated command registry
   - typed invoke wrapper
   - shared event wrapper
   - instrumentation/test harness
   - plugin bridge integration
2. Document a simple rule:
   - application code uses bridge helpers
   - bridge code is the only normal place raw Tauri APIs should appear
3. Keep generated command inventory and checker scripts part of the bridge contract, not optional tooling.
4. Normalize plugin-bridge failures through the same diagnostics path as command invokes when practical.
5. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Execution Notes

- kept the typed frontend Tauri boundary intact and verified it through the existing command-registry, invoke, event, and instrumentation suites
- added direct `invoke.mockIPC` coverage for lifecycle-hook failures so bridge instrumentation remains best-effort instead of being able to break normal invokes
- preserved the current generation/check workflow around `commands.ts` and kept the plugin bridge separate while still validating the shared diagnostics path through desktop-productization tests

## Checklist

- [x] Document the allowed frontend entry points for Tauri command calls and events.
- [x] Document generated command inventory ownership and regeneration/check policy.
- [x] Document invoke error normalization and runtime-detection semantics.
- [x] Document shared event-listener behavior, retry behavior, and teardown expectations.
- [x] Document instrumentation channel ownership and tracked-command policy.
- [x] Add or tighten bridge tests if wrappers or generation rules change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/tauri/commands.test.ts`
- run `npm --prefix web run test -- src/tauri/invoke.contract.test.ts`
- run `npm --prefix web run test -- src/tauri/invoke.mockIPC.test.ts`
- run `npm --prefix web run test -- src/tauri/events.test.ts`
- run `npm --prefix web run test -- src/tauri/instrumentation.mockIPC.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current bridge wrapper layer even if implementation details change internally
- update generation/check scripts in the same change as command registry format changes
- keep mock-runtime helpers aligned with production bridge semantics, not as a separate fake protocol
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/04_fe_tauri_bridge_tests_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/21_tauri_mock_runtime_smoke_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/48_generated_command_inventory_2026-04-08.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`
