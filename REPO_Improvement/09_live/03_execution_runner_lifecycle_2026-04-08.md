# Execution Runner Lifecycle Plan

Last updated: 2026-04-09

## Purpose

Document the current live execution runner lifecycle so start/stop/status behavior, live-bar processing, cancellation semantics, and frontend runtime expectations stay aligned with the code that already ships.

## Scope Boundaries

In scope:

- auto-runner start/stop/status commands
- runner slot ownership and replacement
- live-bar trigger flow and coalescing
- cancellation, stale-generation, and error semantics
- frontend event/status consumption for the runner

Out of scope:

- options execution
- adding new broker integrations
- redesigning execution workspace UX

## Current User-Facing Behavior

The execution workspace can start one auto-runner for the current symbol, stop it, and inspect its current status. The runner only reacts to Polygon live bars, and the frontend currently blocks runner start unless live candles are active for the same symbol. Dry-run mode requires no Alpaca credentials; broker-armed mode does. While active, the runner reports lifecycle events and surfaces optimistic submitted-order rows in the UI.

## Current Code Surface

- `src-tauri/src/commands_execution.rs`
  - owns the runner config/status/event payloads, the single runner slot, generation/cancel handling, `start_live_execution_runner`, `stop_live_execution_runner`, `live_execution_runner_status`, and the `on_live_execution_bar` execution pipeline
- `src-tauri/src/state.rs`
  - owns `upsert_live_bar`, which the runner uses to mutate the loaded dataset before recomputing the strategy, and invalidates active derived datasets when the base dataset changes
- `src-tauri/src/lib.rs`
  - registers the execution runner commands on the Tauri surface
- `web/src/services/executionService.ts`
  - enforces frontend preconditions for runner start, parses runner status/event payloads, and subscribes to `live_execution_runner_event`
- `web/src/stores/executionStore.ts`
  - derives blocked reasons, applies runner events into UI state, and prepends optimistic order rows when runner events carry a submitted order
- `web/src/components/ExecutionWorkspacePanel.vue`
  - presents runner actions and user-facing blocked-reason text
- `web/src/services/executionService.test.ts`
  - covers symbol-mismatch rejection, runner event parsing, and runtime-event subscription behavior
- `web/src/stores/executionStore.actions.test.ts`
  - covers blocked reasons, runtime-event reconciliation, and optimistic-order handling

## Current Runner Contract

- only one live execution runner slot exists at a time
- supported live strategy kinds are currently `DoubleMa` and `CompositeCpu`
- start requires a non-empty symbol, finite `qty > 0`, a non-empty strategy `data_id`, and an already loaded candle dataset for that `data_id`
- frontend runner start also requires an active live candle stream for the same symbol
- broker-armed runner start requires Alpaca API key + secret; `dry_run=true` does not
- `stop_live_execution_runner` is idempotent and returns both the prior status snapshot and the post-stop status
- `live_execution_runner_status` exposes `active`, `config`, `inflight`, timestamps, counters, `last_action`, `last_order_id`, and `last_error`
- backend runner events currently use these statuses:
  - `started`
  - `stopped`
  - `triggered`
  - `queued_latest`
  - `signal_processed`
  - `order_submitted`
  - `error`

## Current Verification Surface

- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/components/ExecutionWorkspacePanel.test.ts`
- runner-focused unit tests in `src-tauri/src/commands_execution.rs`

## Strengths Already Present

- backend runner replacement uses a generation gate plus an explicit cancel handle, so stale in-flight tasks cannot mutate a newer runner slot
- the runner keeps only the latest pending live bar while work is in flight instead of queueing an unbounded backlog
- frontend start logic already refuses a runner/live-symbol mismatch before invoking Rust
- status payloads already expose enough counters and timestamps to document a real lifecycle contract

## Weaknesses / Failure Modes

- the frontend parser understands a `cancelled` runner event state, but the current backend cancellation path is intentionally silent and does not emit a `cancelled` event
- runner truth is split across command responses, `live_execution_runner_event`, and explicit status refreshes, so docs and tests can diverge if any layer changes wording or state mapping
- `skip_count` covers both stale bars and coalesced in-flight bars, so operator expectations can drift if that meaning is not documented explicitly
- the runner mutates the loaded base dataset through `upsert_live_bar`; that invalidates active derived datasets and is easy to miss if the lifecycle doc only focuses on broker actions
- start/stop replacement behavior is single-slot and global, so same-session experiments on different symbols are destructive replacements, not parallel runners
- dry-run and broker-armed flows share most of the same pipeline, so credential and order-submission semantics are easy to blur in user-facing docs

## Regressions To Avoid

- weakening the current single-slot generation and cancel-handle safety model
- changing frontend event parsing without updating backend emitters and tests in the same work item
- documenting `skip_count` or cancellation semantics inaccurately enough that operators infer nonexistent guarantees
- forgetting that `upsert_live_bar` mutates the loaded base dataset and invalidates derived datasets
- silently broadening supported live strategy kinds without documenting or testing the new lifecycle expectations
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri state management](https://v2.tauri.app/es/develop/state-management/)
- [Tauri frontend event listening](https://v2.tauri.app/es/develop/_sections/frontend-listen/)
- [Tauri calling Rust from the frontend](https://v2.tauri.app/es/develop/calling-rust/)
- [Alpaca: Orders at Alpaca](https://docs.alpaca.markets/docs/orders-at-alpaca)
- [Alpaca: Working with Positions](https://docs.alpaca.markets/v1.3/docs/working-with-positions)
- [Polygon WebSocket quickstart](https://polygon.io/docs/websocket/quickstart?auth=signup)

## Proposed Improvement Approach

1. Document the current runner slot lifecycle from start through replacement, trigger, completion, and stop.
2. Make the event/status matrix explicit, including which transitions emit events and which cancellation paths are intentionally silent.
3. Document frontend preconditions separately from backend validation so operator-facing behavior matches the actual code path.
4. Tie runner lifecycle notes to the underlying dataset-mutation side effect in `upsert_live_bar`.
5. Preserve the current one-runner model and document it before considering any future expansion.
6. Treat live/runtime code quality as part of the implementation scope: keep provider contracts, cancellation, observability, redaction, and operator safety semantics explicit, conservative, and testable.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current single-slot runner ownership model.
- [ ] Document start preconditions at both frontend and backend boundaries.
- [ ] Document the current runner status payload fields and their meaning.
- [ ] Document the emitted runner event statuses and which code paths produce them.
- [ ] Document generation-gated replacement and explicit cancel behavior.
- [ ] Document in-flight bar coalescing and what `skip_count` currently means.
- [ ] Document the `upsert_live_bar` dependency and derived-dataset invalidation side effect.
- [ ] Document dry-run versus broker-armed credential and order-submission semantics.
- [ ] Add or tighten tests if the event/status mapping changes.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Document the failure source, user-visible signal, persisted evidence, and redaction or secret-handling expectations for this live/runtime surface.

## Verification Plan

- run `npm --prefix web run test -- src/services/executionService.test.ts`
- run `npm --prefix web run test -- src/stores/executionStore.actions.test.ts`
- run `npm --prefix web run test -- src/components/ExecutionWorkspacePanel.test.ts`
- run targeted Rust tests in `src-tauri/src/commands_execution.rs`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend, Rust, and desktop quality gates for touched live surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any required desktop smoke or paper-trading/operator validation path
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current single-runner semantics unless a replacement plan is explicit
- treat clearer lifecycle docs as hardening, not permission to weaken backend cancel/generation safety
- if a future change introduces a new runner event state, update both backend emitters and frontend parsers in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
- `REPO_Improvement/09_live/07_live_candle_stream_data_2026-04-08.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
