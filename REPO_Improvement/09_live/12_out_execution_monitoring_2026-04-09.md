# Out Execution Monitoring Plan

Last updated: 2026-04-09

## Purpose

Define the current operator-facing execution monitoring surface so live status, runner status, orders, positions, and preview output remain truthful and useful without overpromising broker-confirmed or chart-confirmed state.

## Scope Boundaries

In scope:

- execution workspace status lines and hints
- order table, position summary, and chart-meta monitoring cues
- live-feed and runner monitoring copy
- optimistic versus broker-confirmed execution state
- output preview and operator review surfaces for non-options execution

Out of scope:

- broker credential storage policy
- strategy parameter selection logic
- options-related execution monitoring

## Current User-Facing Behavior

The execution surface already provides a monitoring layer rather than only raw action buttons. Current monitoring includes:

- `statusLine`, `statusHint`, `liveStatusHint`, and `runnerStatusHint`
- an order table that may include optimistic submitted rows before broker refresh
- position summary text sourced from broker snapshot refresh
- chart-meta text that makes it clear the chart is advisory and depends on loaded dataset plus refreshed orders
- output-preview JSON for direct operator inspection

The important truth boundary is that the app currently mixes immediate local feedback with delayed broker-confirmed truth:

- one-shot broker submissions can prepend optimistic `submitted*` rows
- confirmed orders and positions only arrive through broker snapshot refresh
- live-feed and runner hints describe runtime posture, not account state

## Current Code Surface

- `web/src/components/ExecutionWorkspacePanel.vue`
  - renders the operator-facing monitoring surface
- `web/src/stores/executionStore.ts`
  - owns monitoring state such as hints, order rows, position/chart meta, and optimistic-row insertion
- `web/src/services/executionService.ts`
  - parses broker snapshots, runner events, live status events, and builds operator-facing monitoring strings
- `src-tauri/src/commands_execution.rs`
  - owns connectivity, snapshot, one-shot execution, runner status, and emitted runner events
- `src-tauri/src/commands_data.rs`
  - owns live candle stream status/events that feed monitoring hints
- `web/src/stores/workflowArtifactsStore.ts`
  - contributes latest workflow snapshot context that execution can reference

## Current Verification Surface

- `web/src/components/ExecutionWorkspacePanel.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/services/executionService.test.ts`
- `web/src/tauri/events.test.ts`
- targeted execution tests in `src-tauri/src/commands_execution.rs`

## Strengths Already Present

- the monitoring surface already distinguishes live-feed posture, runner posture, and broker snapshot output
- optimistic orders are clearly denoted with `submitted*` semantics instead of masquerading as broker-confirmed fills
- the execution store already derives blocked reasons and monitoring cues together, which helps operator truthfulness
- output preview already gives direct inspection of backend responses without requiring hidden logs

## Weaknesses / Failure Modes

- status text can easily blur optimistic local events with broker-confirmed truth
- chart/status/order/position signals live in different latency domains
- monitoring can sound more authoritative than it really is if the hints are not explicit about data provenance
- live-feed active and runner active do not imply broker connectivity is currently healthy
- monitoring fields are spread across service parsing, store state, and panel rendering

## Regressions To Avoid

- presenting optimistic order rows as broker-confirmed fills
- implying live-feed or runner activity means broker/account health is currently healthy
- removing output-preview detail that operators use to inspect execution responses
- weakening position/order refresh clarity in favor of shorter but less truthful status text
- tying chart overlays to implicit assumptions when the current chart is explicitly advisory
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Alpaca orders documentation](https://docs.alpaca.markets/docs/working-with-orders)
  - relevant because order acceptance, status progression, and fill semantics shape what the monitoring surface can truthfully display
- [Alpaca positions documentation](https://docs.alpaca.markets/v1.3/docs/working-with-positions)
  - relevant because position state is broker-confirmed snapshot data, not frontend inference
- [Tauri state management](https://v2.tauri.app/develop/state-management/)
  - relevant because authoritative runtime and snapshot state originates on the Rust side
- [Vue state management guide](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the monitoring surface is a store-driven operator state model rather than a purely local component view
- [Alpaca Orders](https://docs.alpaca.markets/docs/orders-at-alpaca)
  - relevant because live execution, monitoring, and order-state handling sit behind provider contracts that must remain explicit in a public-facing trading app

## Proposed Improvement Approach

1. Document the monitoring surface in provenance layers:
   - local immediate status
   - live-feed/runtime posture
   - optimistic order submission feedback
   - broker-confirmed snapshots
2. Preserve the current distinction between advisory chart/status cues and confirmed broker state.
3. Keep output-preview inspection available as a first-class debugging/operator tool.
4. Tie panel wording to the actual latency and authority of each field.
5. Expand monitoring only in additive ways that improve truthfulness rather than hiding detail.
6. Treat live/runtime code quality as part of the implementation scope: keep provider contracts, cancellation, observability, redaction, and operator safety semantics explicit, conservative, and testable.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document every current execution monitoring field and what authority it represents.
- [ ] Document optimistic versus broker-confirmed order state.
- [ ] Document position summary provenance and refresh requirements.
- [ ] Document live-feed and runner hints separately from broker/account state.
- [ ] Document chart-meta limitations and current advisory posture.
- [ ] Document output-preview usage as operator-visible diagnostic evidence.
- [ ] Add or tighten tests if monitoring strings or provenance rules change.
- [ ] Keep user-facing copy explicit when state is delayed, optimistic, or advisory.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Document the failure source, user-visible signal, persisted evidence, and redaction or secret-handling expectations for this live/runtime surface.

## Verification Plan

- run `npm --prefix web run test -- src/components/ExecutionWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/executionStore.actions.test.ts`
- run `npm --prefix web run test -- src/services/executionService.test.ts`
- run `npm --prefix web run test -- src/tauri/events.test.ts`
- run targeted tests in `src-tauri/src/commands_execution.rs`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend, Rust, and desktop quality gates for touched live surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any required desktop smoke or paper-trading/operator validation path
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current optimistic-row and snapshot-refresh behavior unless the monitoring contract is deliberately redesigned
- keep output preview and detailed hints available even if the UI later gains denser summary presentation
- prefer additive provenance cues over removing existing operator detail
- maintain the current distinction between live/runtime state and broker-confirmed account state
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/09_live/02_execution_workspace_ux_2026-04-09.md`
- `REPO_Improvement/09_live/04_execution_order_position_review_2026-04-09.md`
- `REPO_Improvement/09_live/05_execution_safety_modes_2026-04-09.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/19_desktop_artifact_capture_2026-04-09.md`
