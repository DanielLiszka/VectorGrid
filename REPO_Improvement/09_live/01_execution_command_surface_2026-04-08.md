# Execution Command Surface Plan

Last updated: 2026-04-09

## Purpose

Define the current non-options execution command surface so one-shot execution, broker checks, snapshots, live-feed controls, and runner controls remain contract-driven and safety-preserving.

## Scope Boundaries

In scope:

- one-shot execution commands
- broker connectivity and snapshot commands
- live-feed commands that execution depends on
- runner start/stop/status commands
- event payloads and operator-facing semantics exposed through the frontend

Out of scope:

- options execution
- adding new brokers
- redesigning the execution workspace UI

## Current User-Facing Behavior

The execution workspace supports broker connectivity checks, one-shot signal submission, broker snapshot refresh, live-feed start/stop, and auto-runner start/stop. The shipped behavior currently distinguishes:

- `paper` versus `live` broker mode
- `dry_run=true` versus broker-armed submission
- one-shot execution versus long-lived runner automation
- live-feed state versus runner state

The frontend already blocks a number of unsafe starts before invoking Rust, but Rust remains authoritative for execution validation and order submission behavior.

## Current Code Surface

- `src-tauri/src/commands_execution.rs`
  - owns `execute_strategy_signal_once`, `test_alpaca_connection`, `get_live_execution_snapshot`, `start_live_execution_runner`, `stop_live_execution_runner`, `live_execution_runner_status`, and runner event emission
- `src-tauri/src/commands_data.rs`
  - owns live-feed commands and events that execution depends on
- `src-tauri/src/commands.rs`
  - re-exports the live and execution command surface
- `src-tauri/src/lib.rs`
  - registers the execution and live-data commands on the Tauri boundary
- `web/src/services/executionService.ts`
  - invokes all execution-related commands, parses status/event payloads, and enforces several frontend preconditions
- `web/src/stores/executionStore.ts`
  - derives blocked reasons, mode labels, availability flags, and runtime-state summaries
- `web/src/components/ExecutionWorkspacePanel.vue`
  - renders the command surface, operator copy, and confirmation prompts

## Current Verification Surface

- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/components/ExecutionWorkspacePanel.test.ts`
- targeted execution tests in `src-tauri/src/commands_execution.rs`
- live-stream/event bridge tests in `web/src/tauri/events.test.ts` and related instrumentation tests

## Strengths Already Present

- the backend already centralizes execution authority and keeps live-order submission out of the frontend
- current UI copy already distinguishes paper/live posture and dry-run/broker posture
- the store/service layer already exposes operator-facing blocked reasons instead of hiding unsafe preconditions
- execution already has dedicated tests around command invocation, event parsing, and blocked-reason behavior

## Weaknesses / Failure Modes

- the command surface spans one-shot execution, live-feed control, and runner automation, so semantics can be blurred if documented too loosely
- there are overlapping frontend and backend guardrails, which is good for safety but easy to let drift
- broker-confirmed state, optimistic state, and runner/live status are not the same thing, yet user-facing text can make them sound interchangeable
- the current execution surface still relies on browser-stored credentials, which affects how state and safety copy should be documented

## Regressions To Avoid

- weakening paper/live separation or dry-run/broker separation
- allowing invalid quantity, symbol, or credential states past either the frontend or backend guardrails
- making runner or live-feed start behavior less truthful than the current blocked-reason model
- hiding broker-side failure cases behind generic UI errors
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Alpaca orders documentation](https://docs.alpaca.markets/docs/orders-at-alpaca) is relevant because broker-side order acceptance, buying-power checks, and order-state semantics shape what this command surface can promise.
- [Alpaca positions documentation](https://docs.alpaca.markets/v1.3/docs/working-with-positions) is relevant because snapshot and order flows surface broker-confirmed state, not just local optimistic state.
- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because every execution action is invoke-mediated and must stay shape-stable.
- [Tauri state management](https://v2.tauri.app/develop/state-management/) is relevant because single-slot runner/live state is maintained on the Rust side and reflected into the frontend.

## Proposed Improvement Approach

1. Document the command matrix explicitly: connectivity, snapshot, one-shot execution, live-feed control, and runner control.
2. Make the safety posture explicit for each command: what the frontend blocks, what Rust validates, and what the broker can still reject.
3. Distinguish dry-run, paper-broker, and live-broker outcomes in both request and response semantics.
4. Tie command docs to the runtime events and status payloads that the frontend already consumes.
5. Preserve the current execution surface as additive hardening, not as a redesign opportunity.
6. Treat live/runtime code quality as part of the implementation scope: keep provider contracts, cancellation, observability, redaction, and operator safety semantics explicit, conservative, and testable.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document each execution-related command and its current purpose.
- [ ] Document request fields and required preconditions for one-shot execution.
- [ ] Document paper, live, and dry-run semantics separately.
- [ ] Document connectivity and broker snapshot commands and their expected outputs.
- [ ] Document live-feed and runner control commands as dependencies, not interchangeable actions.
- [ ] Document which failures are local validation failures, backend failures, or broker/API failures.
- [ ] Add or tighten tests if command payloads or result fields change.
- [ ] Manually verify paper-mode and dry-run flows before changing live-mode behavior.
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
- run targeted execution tests in `src-tauri/src/commands_execution.rs`
- run targeted event bridge tests if command or event payloads change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend, Rust, and desktop quality gates for touched live surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any required desktop smoke or paper-trading/operator validation path
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve Rust command authority and current frontend guardrails
- keep any clarification work additive; do not remove existing safety prompts or blocked reasons as part of a docs-driven cleanup
- if credential handling or execution modes change later, keep explicit backward-compatible copy until the migration is complete
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/09_live/03_execution_runner_lifecycle_2026-04-08.md`
- `REPO_Improvement/09_live/07_live_candle_stream_data_2026-04-08.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
