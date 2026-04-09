# Runtime State Bridge Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the frontend runtime-state bridge between Tauri commands/events and Pinia stores so command lifecycle, progress events, live-runner events, persistence, and teardown behavior remain coordinated and leak-free.

## Scope Boundaries

In scope:

- `web/src/stores/taskStore.ts`
- `web/src/tauri/instrumentation.ts`
- `web/src/tauri/events.ts`
- `web/src/tauri/invoke.ts`
- runtime event-to-store bridging
- task history/timeline/event persistence

Out of scope:

- per-feature service logic that merely consumes the stores
- backend command implementations
- options runtime surfaces

## Current User-Facing Behavior

The shell already has a general-purpose runtime bridge. It tracks command lifecycle and subscribes to shared Tauri events for:

- `double_ma_progress`
- `research_campaign_progress`
- `live_execution_runner_event`
- `polygon_live_status`
- `polygon_live_bar`

The task store currently exposes:

- active tasks
- task history
- event feed
- timeline
- toasts
- overall progress

Runtime truth is not limited to `taskStore`. The repo also uses:

- `runtimeStore` for boot state, desktop logs, window-state posture, and notification posture
- `runtimePinia` so background services can resolve the active Pinia instance
- `desktopProductization` to bridge Tauri logging, notifications, and window-state behavior into frontend runtime state

Task history, events, and timeline are persisted into browser storage. Event ingestion is throttled for noisy channels, and shared event listening ensures multiple consumers do not attach redundant native listeners for the same event channel.

## Current Code Surface

- `web/src/stores/taskStore.ts`
- `web/src/stores/runtimeStore.ts`
- `web/src/stores/runtimePinia.ts`
- `web/src/tauri/instrumentation.ts`
- `web/src/tauri/events.ts`
- `web/src/tauri/invoke.ts`
- `web/src/services/desktopProductization.ts`
- `web/src/services/executionService.ts`
- `web/src/main.ts`

## Current Verification Surface

- `web/src/tauri/events.test.ts`
- `web/src/tauri/instrumentation.test.ts`
- `web/src/stores/taskStoreCampaignProgress.test.ts`
- `web/src/stores/taskStorePerformance.test.ts`
- `web/src/main.test.ts`
- `web/src/services/desktopProductization.test.ts`
- `web/src/services/executionService.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`
- service tests that depend on runtime events, including execution-service tests

## Strengths Already Present

- command invoke lifecycle is already wrapped and instrumented centrally
- shared event-channel multiplexing already prevents redundant native listeners
- noisy event streams already have throttling
- task history/timeline/event persistence is already bounded
- reset helpers for tests already exist
- background services already have a route to the active Pinia instance via `runtimePinia`
- desktop logs are already capped rather than unbounded

## Weaknesses / Failure Modes

- tracked-command lists exist in more than one place, which risks drift
- event payload normalization and command lifecycle wiring are split across several files
- persistence and runtime subscription teardown are subtle enough to regress silently
- non-Tauri contexts rely on graceful no-op behavior that could be broken by direct API usage
- hot-reload/dispose logic and retry logic are easy to miss in changes
- runtime truth is distributed across `taskStore`, `runtimeStore`, and service bridges, so ownership boundaries can blur

## Regressions To Avoid

- leaking event listeners across installs/resets
- duplicating native listeners per event channel
- desynchronizing tracked-command allowlists
- persisting unbounded event/timeline growth
- breaking non-Tauri runtime behavior in tests or browser-only contexts
- bypassing the invoke/listen wrappers with raw API calls
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/)
  - relevant because runtime progress and live status updates arrive through frontend event subscriptions
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because command lifecycle tracking begins at the invoke wrapper
- [Window.localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)
  - relevant because task history/timeline persistence depends on local browser storage semantics
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because the bridge terminates in shared stores rather than component-local state
- [Tauri Logging plugin](https://v2.tauri.app/plugin/logging/)
  - relevant because desktop diagnostics already flow through Tauri-backed logging paths
- [Tauri Notification plugin](https://v2.tauri.app/plugin/notification/)
  - relevant because runtime bridge behavior includes notification posture and routing
- [Tauri Window State plugin](https://v2.tauri.app/plugin/window-state/)
  - relevant because saved/restored window-state availability is part of runtime truth surfaced to the shell
- [Alpaca Orders](https://docs.alpaca.markets/docs/orders-at-alpaca)
  - relevant because live execution, monitoring, and order-state handling sit behind provider contracts that must remain explicit in a public-facing trading app

## Proposed Improvement Approach

1. Document the bridge as a layered pipeline:
   - invoke wrapper
   - instrumentation hooks
   - shared event listener
   - payload normalization
   - Pinia store mutation
   - persistence
2. Define ownership for each bridge concern:
   - command start/finish/error
   - progress-event normalization
   - listener multiplexing
   - persistence bounds
   - test reset
   - runtime Pinia resolution for background services
   - desktop log, notification, and window-state bridging
3. Eliminate silent duplication risk by documenting a single authority for tracked commands and event channels.
4. Treat non-Tauri behavior as a first-class contract:
   - safe no-op listeners
   - safe invoke availability checks
5. Expand tests around retry, teardown, reset, and event-noise throttling.
6. Treat live/runtime code quality as part of the implementation scope: keep provider contracts, cancellation, observability, redaction, and operator safety semantics explicit, conservative, and testable.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the runtime bridge layers and owners.
- [ ] Document tracked-command and event-channel authority.
- [ ] Document event payload normalization rules.
- [ ] Document persistence bounds and storage keys.
- [ ] Document non-Tauri fallback behavior.
- [ ] Document `runtimeStore` versus `taskStore` ownership boundaries.
- [ ] Document `runtimePinia` requirements for background services.
- [ ] Document desktop logging, notification, and window-state bridge behavior.
- [ ] Add tests for listener teardown, retry, and reset behavior.
- [ ] Add tests for shared-channel multiplexing and no-duplicate-native-listener behavior.
- [ ] Add tests for persisted task history/timeline bounds.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Document the failure source, user-visible signal, persisted evidence, and redaction or secret-handling expectations for this live/runtime surface.

## Verification Plan

- run `npm test -- src/tauri/events.test.ts`
- run `npm test -- src/tauri/instrumentation.test.ts`
- run `npm test -- src/stores/taskStoreCampaignProgress.test.ts`
- run `npm test -- src/stores/taskStorePerformance.test.ts`
- run `npm test -- src/main.test.ts`
- run `npm test -- src/services/desktopProductization.test.ts`
- run `npm test -- src/components/SettingsWorkspacePanel.test.ts`
- manually verify one optimize run and one live-status event flow in the desktop shell
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend, Rust, and desktop quality gates for touched live surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any required desktop smoke or paper-trading/operator validation path
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve best-effort behavior for event subscriptions and invoke hooks
- keep the bridge wrapper-based even if internal files are reorganized later
- if storage persistence changes later, provide a compatibility path for existing task-center history
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/28_runtime_store_status_2026-04-08.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
