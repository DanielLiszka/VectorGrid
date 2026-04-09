# Live Candle Stream Data Plan

Last updated: 2026-04-09

## Purpose

Document the current Polygon live candle stream data path so command semantics, runtime state, event payloads, and symbol/data handling remain grounded in current repo behavior.

## Scope Boundaries

In scope:

- Polygon live stream start/stop/status commands
- live stream slot ownership and replacement
- Polygon websocket auth/subscribe flow
- live status and live bar event payloads
- symbol normalization and delayed/live selection
- frontend live-status parsing for the execution workspace

Out of scope:

- historical Polygon download/repair flows
- options or non-Polygon live data providers
- redesigning the execution workspace

## Current User-Facing Behavior

Users can start live minute candles for one stock symbol at a time, stop the stream, and inspect whether a stream is active. The frontend currently treats delayed streaming as the default. While the stream is running, the backend emits live status events as the websocket progresses through start/connect/subscribe/close/error states and emits normalized minute-bar events for matching `AM.<symbol>` aggregates. Starting a stream for a different symbol replaces the prior stream and also stops the live runner.

## Current Code Surface

- `src-tauri/src/commands_data.rs`
  - owns `polygon_start_live_candle_stream`, `polygon_stop_live_candle_stream`, `polygon_live_candle_stream_status`, websocket task startup, status-event emission, and minute-bar normalization
- `src-tauri/src/state.rs`
  - owns the single live stream slot, replacement/abort behavior, and lazy cleanup of finished websocket tasks
- `src-tauri/src/polygon.rs`
  - owns Polygon API key persistence and lookup, including keyring, legacy file, and environment fallback behavior
- `src-tauri/src/commands_execution.rs`
  - consumes normalized live bars through `on_live_execution_bar`, so live stream behavior directly feeds the runner lifecycle
- `web/src/services/executionService.ts`
  - starts/stops the stream, refreshes live status, parses `polygon_live_status`, and translates backend metadata into operator-facing status hints
- `web/src/stores/executionStore.ts`
  - applies live status events into `liveActive`, `liveSymbol`, `liveDelayed`, blocked reasons, and UI hints
- `web/src/components/ExecutionWorkspacePanel.vue`
  - presents live feed controls and blocked-reason text
- `src-tauri/src/commands_data.rs` tests
  - cover symbol punctuation handling for live-bar normalization
- `src-tauri/src/state.rs` tests
  - cover live-bar upsert behavior and invalidation of derived datasets after base updates
- `web/src/services/executionService.test.ts`
  - covers live-status parsing and runtime-event subscription behavior

## Current Stream Contract

- only one live candle stream slot exists at a time
- `market=stocks` is the only currently supported live market
- symbol normalization uppercases and strips spaces but preserves meaningful punctuation such as `.`
- `delayed` defaults to `true` if the caller omits it
- live stream start requires a Polygon API key; the current lookup order is keyring, legacy settings file, then environment fallback
- `polygon_live_candle_stream_status` returns only `active` plus optional stream metadata, not the full websocket phase
- backend emits `polygon_live_status` for lifecycle updates and `polygon_live_bar` for normalized minute bars
- current backend live status strings include:
  - `started`
  - `connecting`
  - `subscribed`
  - `error`
  - `closed`
  - `stopped`
- only `AM` minute aggregate websocket events are normalized into `polygon_live_bar`
- starting live candles for a different symbol stops the runner; stopping live candles also stops the runner

## Current Verification Surface

- unit tests in `src-tauri/src/commands_data.rs`
- state tests in `src-tauri/src/state.rs`
- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/components/ExecutionWorkspacePanel.test.ts`

## Strengths Already Present

- runtime state explicitly models one live stream slot, making replacement behavior deterministic
- finished websocket handles are cleaned up when status is queried, preventing stale `active=true` reports from lingering forever
- the backend already emits both coarse lifecycle events and normalized bar events
- symbol punctuation handling is already protected by tests, which matters for symbols like `BRK.B`

## Weaknesses / Failure Modes

- `polygon_live_candle_stream_status` only returns active/off plus stream metadata, while richer phase information exists only on the event stream, so UI docs can drift if they treat status polling as authoritative for connection phase
- live stream replacement is global and destructive; any future UI copy that implies multiple simultaneous streams would be wrong
- the current websocket normalizer only accepts `AM` events and exact symbol matches via the execution symbol matcher, so vendor payload or channel-shape changes can silently narrow what reaches the app
- delayed mode defaults to `true`; that is easy to overlook if docs only mention the websocket URL selection
- live bars reach the runner path, but the dataset mutation side effect happens only when the runner is active; a live-stream-only session does not currently update loaded datasets by itself
- live stream task cleanup is lazy through `live_candle_stream_info()`, so a finished task is cleared on status inspection rather than at the exact moment of completion
- API key storage behavior spans keyring, disk fallback, and env fallback, which is easy to misstate in user docs

## Regressions To Avoid

- weakening the current one-stream-slot semantics or documenting them ambiguously
- changing delayed-default behavior without updating both backend defaults and frontend/operator copy
- broadening accepted websocket payload types without updating normalization docs and tests
- forgetting that runner stop behavior is coupled to stream replacement and stop flows
- misdocumenting Polygon API key source order or current `stocks`-only support
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Polygon WebSocket quickstart](https://polygon.io/docs/websocket/quickstart?auth=signup)
- [Tauri state management](https://v2.tauri.app/es/develop/state-management/)
- [Tauri frontend event listening](https://v2.tauri.app/es/develop/_sections/frontend-listen/)
- [Tauri calling Rust from the frontend](https://v2.tauri.app/es/develop/calling-rust/)
- [Alpaca Orders](https://docs.alpaca.markets/docs/orders-at-alpaca)
  - relevant because live execution, monitoring, and order-state handling sit behind provider contracts that must remain explicit in a public-facing trading app

## Proposed Improvement Approach

1. Document the current one-stream slot lifecycle from request through replacement, websocket progress, and stop.
2. Separate the polling contract from the event-stream contract so status and event consumers are described accurately.
3. Make symbol normalization, delayed defaulting, and `stocks`-only support explicit.
4. Tie live stream behavior to the runner only where the current code actually does so.
5. Preserve current repo behavior in docs before considering any broader live-data abstraction.
6. Treat live/runtime code quality as part of the implementation scope: keep provider contracts, cancellation, observability, redaction, and operator safety semantics explicit, conservative, and testable.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current start/stop/status command payloads and responses.
- [ ] Document the single live-stream slot and replacement semantics.
- [ ] Document symbol normalization and punctuation-preserving matching.
- [ ] Document the current delayed default and websocket URL selection.
- [ ] Document Polygon API key source order and current failure messages.
- [ ] Document the `polygon_live_status` status vocabulary and `polygon_live_bar` payload role.
- [ ] Document that only `AM` minute aggregates currently feed the live-bar path.
- [ ] Document when stream changes stop the runner.
- [ ] Add or tighten tests if live status/event parsing changes.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Document the failure source, user-visible signal, persisted evidence, and redaction or secret-handling expectations for this live/runtime surface.

## Verification Plan

- run targeted unit tests in `src-tauri/src/commands_data.rs`
- run targeted state tests in `src-tauri/src/state.rs`
- run `npm --prefix web run test -- src/services/executionService.test.ts`
- run `npm --prefix web run test -- src/stores/executionStore.actions.test.ts`
- run `npm --prefix web run test -- src/components/ExecutionWorkspacePanel.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend, Rust, and desktop quality gates for touched live surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any required desktop smoke or paper-trading/operator validation path
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current `stocks`-only live-stream contract unless an expansion is explicitly implemented
- preserve delayed-default behavior unless both frontend copy and backend defaults are updated together
- if future work broadens the websocket payload types, update normalization docs and tests in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/17_polygon_download_load_2026-04-08.md`
- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
- `REPO_Improvement/09_live/03_execution_runner_lifecycle_2026-04-08.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
