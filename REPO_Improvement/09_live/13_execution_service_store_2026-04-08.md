# Execution Service and Store Plan

Last updated: 2026-04-08

## Purpose

Plan improvements to the non-options live-execution service/store split so command invocation, event handling, optimistic state, and user-facing availability logic have a clear source of truth without regressing current broker/runtime safety.

## Scope Boundaries

In scope:

- `executionService` and `executionStore` responsibilities
- runtime event subscription ownership
- optimistic order/position state
- availability and blocked-reason semantics
- persisted execution credentials/settings behavior

Out of scope:

- options execution flows
- adding new broker integrations
- redesigning the entire execution workspace UI

## Current User-Facing Behavior

The execution workspace can test connectivity, refresh broker state, subscribe to live status events, start/stop the live runner, and place or simulate execution actions. The frontend combines persisted settings, optimistic updates, event-stream updates, and explicit refresh calls to keep the UI current.

## Current Code Surface

- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `web/src/utils/storage.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/state.rs`
- `web/src/components/ExecutionWorkspacePanel.vue`

## Current Verification Surface

- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/components/ExecutionWorkspacePanel.test.ts`
- `web/src/stores/domainStores.test.ts`
- Rust execution command tests

## Strengths Already Present

- live execution already has a dedicated service and a dedicated store
- runtime events are already bridged into the frontend
- the store already exposes blocked reasons and availability hints
- there is existing test coverage around service/store behavior

## Weaknesses / Failure Modes

- validation and availability rules are duplicated across store, service, and Rust boundaries
- live truth is split across optimistic rows, event streams, and explicit refreshes
- only one live Polygon stream slot and one runner slot exist, which creates replacement semantics that must stay explicit
- persisted execution keys live in browser storage rather than OS secret storage
- one store currently owns too many concerns: form state, runtime state, subscription lifecycle, and broker snapshots

## Regressions To Avoid

- weakening current broker/runtime safety checks
- making pending/blocked status less truthful
- breaking one-runner replacement behavior
- losing optimistic-state feedback that users currently rely on
- changing credential persistence behavior without a compatibility path

## External Research Summary

- [Pinia introduction](https://pinia.vuejs.org/introduction.html) is relevant for store responsibility boundaries.
- [Pinia actions](https://pinia.vuejs.org/core-concepts/actions.html) is relevant because the current store mixes action orchestration with long-lived runtime semantics.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces that backend commands remain the authority for execution actions.
- [Tauri core permissions reference](https://v2.tauri.app/reference/acl/core-permissions/) is relevant for command/event boundary hygiene.
- [Alpaca orders](https://docs.alpaca.markets/docs/orders-at-alpaca) and [working with positions](https://docs.alpaca.markets/v1.3/docs/working-with-positions) are relevant for clarifying broker-confirmed vs optimistic frontend state.

## Proposed Improvement Approach

1. Assign source-of-truth ownership for each execution concept:
   - availability
   - pending state
   - blocked reasons
   - optimistic order rows
   - broker-confirmed positions/orders
   - live runner status
2. Document the lifecycle of runtime subscriptions, including replacement, teardown, and stale-event handling.
3. Make manual/best/selected parameter-source behavior explicit instead of letting it emerge from store heuristics.
4. Clarify credential persistence and clearing semantics now, even if secure storage remains a later improvement item.
5. Expand tests for event lifetime, one-runner replacement, mismatched live symbol, and refresh-after-optimistic-order flows.

## Checklist

- [ ] Define source-of-truth ownership for availability, pending, blocked, optimistic, and broker-confirmed state.
- [ ] Document runtime subscription lifecycle and stale-event handling.
- [ ] Document manual/best/selected parameter-source semantics.
- [ ] Document live vs paper vs dry-run state expectations.
- [ ] Clarify credential persistence and clearing semantics.
- [ ] Add tests for runner replacement and event-subscription cleanup.
- [ ] Add tests for optimistic-order refresh reconciliation.
- [ ] Preserve current backend safety checks and currently supported strategy limitations.

## Verification Plan

- run `npm test -- src/services/executionService.test.ts`
- run `npm test -- src/stores/executionStore.actions.test.ts`
- run `npm test -- src/components/ExecutionWorkspacePanel.test.ts`
- run targeted Rust execution command tests
- run a desktop/live-runtime smoke flow if event wiring changes

## Rollout / Fallback / Compatibility Notes

- preserve current command authority in Rust
- treat clearer state ownership as additive hardening, not as permission to remove optimistic feedback
- if credential storage changes later, provide a migration path from current browser storage semantics

## Related Docs

- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
- `REPO_Improvement/09_live/03_execution_runner_lifecycle_DATE.md`
- `REPO_Improvement/09_live/11_settings_secret_storage_DATE.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_DATE.md`
- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_DATE.md`
