# Bundle 10 Consolidation — Live Execution Broker Position And Order Reconciliation

Generated: 2026-04-11

This consolidation keeps only the findings that remained supported after re-checking the provided bundle docs and owner files. It stays inside the current narrow live scope: Alpaca broker path, Polygon stocks live-feed path, single active live stream, single active live runner, non-options only, and the current settings-mounted execution surface. Sources for each material claim are named inline by file and section or module.

## 1. Surface understanding

Bundle 10 is supposed to harden one specific operator-truth gap inside the existing live execution slice, not widen the product. The bundle doc frames the work as reconciliation between desired local action, broker position, and open broker orders for the current single-symbol live mode only, while the umbrella productization doc and repo map keep the broader live posture narrow: Alpaca REST, Polygon stocks live streaming, one live stream slot, one live runner slot, and the execution panel still mounted inside the settings workspace rather than a separate live shell. Sources: `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Purpose, Scope Boundaries, Proposed Improvement Approach; `05_live_execution_productization_2026-04-10.md` — Current User-Facing Behavior, Scope Boundaries; `REPO_MAP.md` — Important current-state note under Settings, Live execution.

The current review contract already has stronger truth anchors than the bundle doc fully carries forward. The execution review surface is symbol-scoped rather than portfolio-wide; `Position: -` is explicitly different from confirmed `Position: flat`; optimistic local feedback is distinct from broker-confirmed refresh; and raw JSON preview remains part of the diagnostic surface. Sources: `04_execution_order_position_review_2026-04-09.md` — Current User-Facing Behavior, Regressions To Avoid, Proposed Improvement Approach, Execution Notes; `executionStore.ts` — initial state; `executionService.ts` — `positionMeta`, `optimisticOrderRowFromExecution`, `refreshOrders`.

The main risk is therefore not missing UI ambition. It is that a new reconciliation summary could collapse different provenance layers into fake certainty: blank-symbol account scope, optimistic local rows, broker-confirmed state, stale snapshots, and prior-context leftovers could all be compressed into one overconfident status unless the plan gets much more explicit. Sources: `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Weaknesses / Failure Modes, Regressions To Avoid, Checklist; `04_execution_order_position_review_2026-04-09.md` — Weaknesses / Failure Modes, Regressions To Avoid; `executionService.ts` — `refreshOrders`, `optimisticOrderRowFromExecution`; `executionStore.ts` — `applyActionResult`, `applyLiveExecutionRunnerEvent`, `setInputField`, `setSelectField`; `commands_execution.rs` — `get_live_execution_snapshot`, `execute_strategy_signal_blocking`.

Stale-reference check: no stale or nonexistent file reference is confirmed from the supplied packet. The related doc `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md` is referenced by bundle 10 and bundle 05, but it was not included here, so this consolidation cannot validate its contents or current relevance. Sources: `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Related Docs; `05_live_execution_productization_2026-04-10.md` — Related Docs.

## 2. Consolidated must-fix findings

### MF-1 — Blank-symbol reconciliation must be explicitly disabled

- **Severity:** critical  
- **Type:** correctness / operator-truth  
- **Location:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Scope Boundaries, Checklist; `executionService.ts` — `refreshOrders`; `executionStore.ts` — `setAvailability`; `commands_execution.rs` — `symbol_matches`, `get_live_execution_snapshot`; `04_execution_order_position_review_2026-04-09.md` — Weaknesses / Failure Modes
- **Why it matters:** The plan says the bundle is for current single-symbol live mode only, but the current owner files still permit blank-symbol refresh. The store allows refresh with no symbol, the service labels the result as "orders for all symbols," and the backend blank-symbol path treats symbol matching as a wildcard and can still return a position by taking the first matching position from the full positions list. The earlier review doc already flags empty-symbol handling as a less truthful pseudo-portfolio view. A reconciliation summary built on top of that would be able to present mixed-scope account state as if it were one canonical symbol state.
- **Recommended change:** Add a hard disabled-state rule: reconciliation is unavailable until a canonical nonblank symbol is selected. If blank-symbol refresh remains supported for manual review, it must stay table/raw-preview-only and must not drive a canonical position/order/action summary.
- **Sources:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Scope Boundaries, Checklist; `executionService.ts` — `refreshOrders`; `executionStore.ts` — `setAvailability`; `commands_execution.rs` — `symbol_matches`, `get_live_execution_snapshot`; `04_execution_order_position_review_2026-04-09.md` — Weaknesses / Failure Modes.

### MF-2 — The plan needs a canonical typed reconciliation model with explicit freshness ownership

- **Severity:** critical  
- **Type:** correctness / operator-truth / maintainability  
- **Location:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Proposed Improvement Approach, Checklist; `commands_execution.rs` — `GetLiveExecutionSnapshotResponse`, `ExecuteStrategySignalResponse`; `executionService.ts` — `ExecutionSnapshotResponse`, `ExecuteSignalResponse`, `refreshOrders`; `executionStore.ts` — `markVue`, `updatedAtUnixMs`
- **Why it matters:** The backend already returns a broker snapshot timestamp (`fetched_at_unix_ms`) and a richer execution response (`desired_position`, `current_position_qty`, `target_position_qty`, `delta_qty`), but the frontend types discard most of this. The service snapshot type does not include `fetched_at_unix_ms`, and the reduced execution type keeps only `action`, `order_submitted`, `order_id`, `delta_qty`, `last_bar_ts`, and warnings. Meanwhile the store’s `updatedAtUnixMs` is bumped on any UI mutation, not just on broker refresh. Without an explicit data-ownership model, a reconciliation layer can accidentally derive freshness from generic UI activity or derive broker truth from already-formatted strings.
- **Recommended change:** Add a typed reconciliation state in service/store with separate fields for: broker snapshot payload, broker snapshot time, derived open orders, desired action / desired position, runner-event advisory state, optimistic local pending state, and generic UI mutation time. State plainly that broker freshness comes only from `GetLiveExecutionSnapshotResponse.fetched_at_unix_ms` and that formatted strings / rendered rows are not canonical reconciliation inputs.
- **Sources:** `commands_execution.rs` — `GetLiveExecutionSnapshotResponse`, `ExecuteStrategySignalResponse`; `executionService.ts` — `ExecutionSnapshotResponse`, `ExecuteSignalResponse`, `refreshOrders`; `executionStore.ts` — `markVue`, `updatedAtUnixMs`; `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Proposed Improvement Approach, Checklist.

### MF-3 — “Open orders” must be defined against the current `status=all` snapshot contract

- **Severity:** high  
- **Type:** correctness / operator-truth  
- **Location:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Purpose, Proposed Improvement Approach, Checklist; `commands_execution.rs` — `fetch_alpaca_orders`, `get_live_execution_snapshot`; `executionService.ts` — `buildOrderRows`, `refreshOrders`
- **Why it matters:** The bundle repeatedly frames the problem as reconciling against open orders, but the backend snapshot path currently fetches `/v2/orders?status=all`, not open-only orders. The frontend then reduces whatever comes back into display rows. Without an explicit rule for which statuses count as “open,” the new summary could accidentally treat filled, canceled, or otherwise historical orders as active open-order state.
- **Recommended change:** Add a short canonical rule for open-order derivation. The doc should say that reconciliation derives open-order state from typed broker snapshot data using an explicit status set, not from the rendered order table and not from the total count of returned orders.
- **Sources:** `commands_execution.rs` — `fetch_alpaca_orders`, `get_live_execution_snapshot`; `executionService.ts` — `buildOrderRows`, `refreshOrders`; `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Purpose, Proposed Improvement Approach, Checklist.

### MF-4 — State taxonomy, precedence, and context invalidation rules are still too vague

- **Severity:** high  
- **Type:** correctness / operator-truth / migration  
- **Location:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Proposed Improvement Approach, Checklist, Rollout / Fallback / Compatibility Notes; `04_execution_order_position_review_2026-04-09.md` — Current User-Facing Behavior, Regressions To Avoid, Proposed Improvement Approach; `executionService.ts` — `positionMeta`, `optimisticOrderRowFromExecution`; `executionStore.ts` — initial state, `applyActionResult`, `applyLiveExecutionRunnerEvent`, `setInputField`, `setSelectField`
- **Why it matters:** The existing review contract distinguishes unknown/unfetched state from confirmed flat state, but the current code path is still easy to blur: store initialization uses `Position: -`, service refresh maps missing position to `Position: flat`, optimistic rows are marked `submitted*`, and symbol/mode edits do not invalidate prior review state. Runner events can also prepend advisory optimistic rows at any time. Without explicit precedence and reset rules, the summary could mix prior-symbol review state, optimistic local state, and broker-confirmed state into one overconfident answer.
- **Recommended change:** Add an explicit state taxonomy and precedence matrix. Minimum states: `disabled/unsupported`, `unknown/unfetched`, `broker-confirmed flat`, `broker-confirmed open`, `stale broker snapshot`, and `optimistic local pending`. Minimum precedence: broker-confirmed snapshot > optimistic local event > placeholder unknown. Add explicit invalidation/reset rules for symbol change, mode change, refresh completion, runner stop, and feed stop. Also state that optimistic rows never contribute to canonical open-order counts, stale-age calculations, or block/warn decisions.
- **Sources:** `04_execution_order_position_review_2026-04-09.md` — Current User-Facing Behavior, Regressions To Avoid, Proposed Improvement Approach, Execution Notes; `executionService.ts` — `positionMeta`, `optimisticOrderRowFromExecution`; `executionStore.ts` — initial state, `applyActionResult`, `applyLiveExecutionRunnerEvent`, `setInputField`, `setSelectField`; `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Proposed Improvement Approach, Checklist, Rollout / Fallback / Compatibility Notes.

### MF-5 — The doc must freeze current delta-submission semantics for this bundle

- **Severity:** high  
- **Type:** correctness / migration / operator-truth  
- **Location:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Weaknesses / Failure Modes, Checklist, Rollout / Fallback / Compatibility Notes; `commands_execution.rs` — `execute_strategy_signal_blocking`; `REPO_MAP.md` — Live execution
- **Why it matters:** The bundle says partial fills and open-order states should “block or warn” before the next action, but the backend order-submission path still computes `delta_qty` strictly from desired position minus current broker position. It does not incorporate open orders into the submission decision. If bundle 10 adds hard blocking behavior only at the summary/review layer, it risks implying a stronger safety contract than the command boundary actually enforces.
- **Recommended change:** Add an explicit submission-semantics freeze statement: bundle 10 improves review and warnings only, and does not change `execute_strategy_signal_blocking` delta math. Any hard blocking or order-aware submission changes should move to a separate backend-authoritative follow-up with dedicated Rust and frontend tests.
- **Sources:** `commands_execution.rs` — `execute_strategy_signal_blocking`; `REPO_MAP.md` — Live execution; `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Weaknesses / Failure Modes, Checklist, Rollout / Fallback / Compatibility Notes.

### MF-6 — Verification and rollback need to be upgraded to match a visible truth-layer change

- **Severity:** high  
- **Type:** verification / testing / rollback / operator-truth  
- **Location:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Current Verification Surface, Verification Plan, Rollout / Fallback / Compatibility Notes; `05_live_execution_productization_2026-04-10.md` — Current Verification Surface, Verification Plan; `REPO_MAP.md` — Important current-state note under Settings; `04_execution_order_position_review_2026-04-09.md` — Regressions To Avoid, Verification Plan
- **Why it matters:** Bundle 10 currently makes browser UX reruns conditional on a future “live shell,” but the execution panel is already a user-visible surface because the settings workspace mounts `ExecutionWorkspacePanel.vue` today. The bundle also protects the review tables as fallback, but it does not explicitly preserve raw preview even though the earlier review contract treats raw preview as a diagnostic truth anchor.
- **Recommended change:** Make browser UX reruns expected for any user-visible reconciliation summary or warning change now, not only after future shell promotion. Add an explicit acceptance matrix covering blank-symbol disabled state, unknown vs confirmed flat, stale snapshot handling, open-order filtering from `status=all`, optimistic-vs-broker separation, symbol/mode invalidation, and no hidden delta change. Add a rollback trigger that hides the summary layer if it diverges from typed snapshot / review table / raw preview validation. Keep raw preview explicitly protected in regressions, verification evidence, and fallback notes.
- **Sources:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Current Verification Surface, Verification Plan, Rollout / Fallback / Compatibility Notes; `05_live_execution_productization_2026-04-10.md` — Current Verification Surface, Verification Plan; `REPO_MAP.md` — Important current-state note under Settings; `04_execution_order_position_review_2026-04-09.md` — Regressions To Avoid, Verification Plan.

## 3. Consolidated should-fix findings

### SF-1 — The doc should explicitly choose whether reconciliation is live-only or applies to paper broker mode too

- **Severity:** medium  
- **Type:** scope / migration  
- **Location:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Scope Boundaries; `05_live_execution_productization_2026-04-10.md` — Current User-Facing Behavior; `REPO_MAP.md` — Live execution
- **Why it matters:** Bundle 10 says “current single-symbol live mode only,” but the current execution surface, credentials flow, and snapshot path all exist across dry-run, paper, and live modes. The doc should say whether reconciliation is intentionally withheld from paper mode or whether the same truth model applies there too.
- **Recommended change:** Add one explicit sentence choosing live-only or paper+live behavior for this bundle and explain why.
- **Sources:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Scope Boundaries; `05_live_execution_productization_2026-04-10.md` — Current User-Facing Behavior; `REPO_MAP.md` — Live execution.

### SF-2 — The desired-action source contract should be named explicitly

- **Severity:** medium  
- **Type:** correctness / operator-truth / maintainability  
- **Location:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Proposed Improvement Approach; `executionService.ts` — `runOnce`, `parseRunnerEvent`; `executionStore.ts` — `applyActionResult`, `applyLiveExecutionRunnerEvent`
- **Why it matters:** The proposed operator-facing summary depends on “desired strategy action,” but the current implementation can surface that from a one-shot execution response, from a runner event, or from neither until a new action occurs. Leaving that implicit invites inconsistent implementations.
- **Recommended change:** Define desired-action provenance in the doc. A safe first rule is: latest manual execution response and latest runner event are advisory desired-action sources unless or until replaced by a fresher broker-confirmed snapshot and typed reconciliation state.
- **Sources:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Proposed Improvement Approach; `executionService.ts` — `runOnce`, `parseRunnerEvent`; `executionStore.ts` — `applyActionResult`, `applyLiveExecutionRunnerEvent`.

### SF-3 — The bundle should explicitly say what it does **not** own relative to adjacent live-mode bundles

- **Severity:** low  
- **Type:** scope / maintainability  
- **Location:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Scope Boundaries, Related Docs; `05_live_execution_productization_2026-04-10.md` — Related Docs; `REPO_MAP.md` — Settings mount note
- **Why it matters:** Bundle 10 references adjacent bundles on operator safety, session audit, and surface promotion. Because reconciliation touches user-facing warnings and status, the doc should clearly say it is not moving the execution surface and not absorbing safety-control or audit-trail ownership.
- **Recommended change:** Add a short ownership note: bundle 10 hardens broker/order/position truth inside the current settings-mounted execution surface and does not move the surface or define safety-control semantics beyond the reconciliation summary/warning layer.
- **Sources:** `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Scope Boundaries, Related Docs; `05_live_execution_productization_2026-04-10.md` — Related Docs; `REPO_MAP.md` — Important current-state note under Settings.

## 4. Optional improvements worth considering

1. **Add a compact visible-state matrix to the doc.**  
   A small table with examples like `disabled`, `unknown`, `broker-confirmed`, `local pending`, and `stale broker snapshot` would make the truth model harder to misread during implementation and review. Sources: `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Proposed Improvement Approach; `04_execution_order_position_review_2026-04-09.md` — Proposed Improvement Approach, Checklist.

2. **Rename “discrete live-mode truth layer” if it risks sounding like a new shell surface.**  
   The current repo still mounts execution inside the settings workspace, so a phrase like “reconciliation summary layer inside the current execution surface” may be less misleading. Sources: `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Proposed Improvement Approach; `REPO_MAP.md` — Important current-state note under Settings.

3. **Add one short data-flow note.**  
   A sentence or diagram showing `get_live_execution_snapshot` -> typed reconciliation state -> rendered summary/table/raw preview would reinforce that the rendered UI is downstream of typed canonical state rather than the source of it. Sources: `commands_execution.rs` — `get_live_execution_snapshot`; `executionService.ts` — `refreshOrders`; `executionStore.ts` — `applyActionResult`.

## 5. Suggestions to reject as out of scope or unsupported

1. **Reject portfolio OMS / multi-broker / multi-symbol expansion ideas here.**  
   Bundle 10 and the umbrella live doc both keep the scope narrow and explicitly exclude portfolio-level order management, multi-broker reconciliation, and multi-symbol orchestration. Sources: `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Scope Boundaries, Rollout / Fallback / Compatibility Notes; `05_live_execution_productization_2026-04-10.md` — Scope Boundaries.

2. **Reject hard smart-order or block-submit behavior in this bundle without a matching backend contract change.**  
   The current backend still submits delta-to-target based on broker position, so introducing OMS-like blocking only in the summary layer would be misleading. Sources: `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Regressions To Avoid, Checklist; `commands_execution.rs` — `execute_strategy_signal_blocking`; `REPO_MAP.md` — Live execution.

3. **Reject moving the execution surface out of the settings workspace as part of bundle 10.**  
   Surface promotion is owned by a separate live-mode bundle, and the repo map still describes execution as settings-mounted today. Sources: `05_live_execution_productization_2026-04-10.md` — Proposed Improvement Approach, Related Docs; `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Related Docs; `REPO_MAP.md` — Important current-state note under Settings.

4. **Reject implementations that use optimistic rows, formatted summary strings, or rendered table rows as canonical reconciliation inputs.**  
   The current code already shows these are presentation-layer outputs, not trustworthy typed state. Sources: `executionService.ts` — `optimisticOrderRowFromExecution`, `positionMeta`, `refreshOrders`; `executionStore.ts` — `applyActionResult`; `04_execution_order_position_review_2026-04-09.md` — Current User-Facing Behavior, Regressions To Avoid.

## 6. Recommended doc edits

### Add a new subsection: “Truth model and disabled states”

Suggested content:

- Reconciliation is disabled until a canonical nonblank symbol is selected.
- Reconciliation is also disabled when the current symbol/mode context has changed since the last broker snapshot.
- `Position: -` means unknown/unfetched state; `Position: flat` means broker-confirmed no position.
- Optimistic local state remains advisory and is never displayed as broker-confirmed truth.

Sources: `04_execution_order_position_review_2026-04-09.md` — Current User-Facing Behavior, Proposed Improvement Approach; `executionService.ts` — `positionMeta`; `executionStore.ts` — initial state, `setAvailability`; `commands_execution.rs` — `symbol_matches`, `get_live_execution_snapshot`.

### Add a new subsection: “Canonical reconciliation inputs and freshness”

Suggested content:

- Canonical broker source: `get_live_execution_snapshot` typed payload.
- Broker freshness derives only from `fetched_at_unix_ms`.
- Desired-action / desired-position state is stored separately from broker snapshot time.
- Rendered rows, chart text, summary text, and raw preview are outputs, not canonical inputs.

Sources: `commands_execution.rs` — `GetLiveExecutionSnapshotResponse`, `ExecuteStrategySignalResponse`; `executionService.ts` — `ExecutionSnapshotResponse`, `ExecuteSignalResponse`, `refreshOrders`; `executionStore.ts` — `markVue`, `updatedAtUnixMs`.

### Add a new subsection: “Open-order derivation and optimistic-row exclusion”

Suggested content:

- Snapshot returns order history, not only open orders.
- The reconciliation layer derives open-order state from an explicit broker-status set.
- Optimistic rows never contribute to canonical open-order counts, age, or block/warn logic.

Sources: `commands_execution.rs` — `fetch_alpaca_orders`, `get_live_execution_snapshot`; `executionService.ts` — `buildOrderRows`, `optimisticOrderRowFromExecution`.

### Expand the checklist with explicit context and semantics items

Suggested additions:

- Define symbol/mode invalidation and reset rules.
- Define the precedence matrix across broker-confirmed, optimistic local, stale, and unknown states.
- State explicitly that this bundle does not change `execute_strategy_signal_blocking` submission semantics.
- State whether the bundle applies to live only or to paper mode as well.

Sources: `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Checklist; `commands_execution.rs` — `execute_strategy_signal_blocking`; `executionStore.ts` — `setInputField`, `setSelectField`, `applyLiveExecutionRunnerEvent`.

### Expand the verification and rollback sections

Suggested additions:

- Require browser UX verification for any visible reconciliation status/warning change because the execution panel is already user-visible.
- Require an acceptance matrix covering blank-symbol disablement, unknown vs confirmed flat, stale broker snapshots, open-order filtering, optimistic-vs-broker separation, symbol/mode invalidation, and no hidden delta change.
- Preserve raw preview explicitly as fallback evidence.
- Hide the summary layer if validation shows weaker truthfulness than the current review table/raw preview combination.

Sources: `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Verification Plan, Rollout / Fallback / Compatibility Notes; `05_live_execution_productization_2026-04-10.md` — Current Verification Surface, Verification Plan; `REPO_MAP.md` — Important current-state note under Settings; `04_execution_order_position_review_2026-04-09.md` — Regressions To Avoid, Verification Plan.

## 7. Recommended implementation order

1. **Revise the bundle doc first.**  
   Freeze the disabled-state rules, typed data ownership, freshness rules, state taxonomy, open-order derivation, and the no-hidden-submission-change boundary before implementation begins. Sources: `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Proposed Improvement Approach, Checklist; `04_execution_order_position_review_2026-04-09.md` — Proposed Improvement Approach; `AGENTS.md` — coding conventions.

2. **Preserve backend truth and widen frontend types before adding UI synthesis.**  
   The backend already provides most of the needed fields. The first implementation slice should preserve them in `executionService.ts` and store them in typed reconciliation state rather than relying on presentation strings. Sources: `commands_execution.rs` — `GetLiveExecutionSnapshotResponse`, `ExecuteStrategySignalResponse`; `executionService.ts` — `ExecutionSnapshotResponse`, `ExecuteSignalResponse`; `executionStore.ts` — state and `applyActionResult`.

3. **Implement derived reconciliation state next, including open-order filtering and context invalidation.**  
   Only after typed state exists should the bundle derive broker-confirmed position/open-order status, stale-state cues, and advisory local-pending state. Sources: `commands_execution.rs` — `fetch_alpaca_orders`, `get_live_execution_snapshot`; `executionStore.ts` — `setInputField`, `setSelectField`, `applyLiveExecutionRunnerEvent`; `executionService.ts` — `optimisticOrderRowFromExecution`, `refreshOrders`.

4. **Render the summary/warnings only after canonical/advisory separation is implemented.**  
   Keep the detailed review table and raw preview visible while the higher-level summary is introduced. Sources: `04_execution_order_position_review_2026-04-09.md` — Regressions To Avoid, Execution Notes; `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Rollout / Fallback / Compatibility Notes.

5. **Add targeted tests and browser UX evidence before rollout.**  
   Keep changes surgical and test-adjacent, matching the repo’s local quality expectations. Sources: `AGENTS.md` — coding conventions; `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Current Verification Surface, Verification Plan; `05_live_execution_productization_2026-04-10.md` — Current Verification Surface, Verification Plan.

6. **Defer any hard blocking or OMS-like submission behavior to a later, narrower follow-up.**  
   That keeps bundle 10 aligned with its stated goal: clarity and truthfulness first, not broker-behavior expansion. Sources: `10_live_execution_broker_position_order_reconciliation_2026-04-11.md` — Proposed Improvement Approach, Rollout / Fallback / Compatibility Notes; `commands_execution.rs` — `execute_strategy_signal_blocking`.
