# Portfolio And Live Execution Expansion Plan

Date: 2026-04-30

## Goal

Expand portfolio feasibility and live execution only as evidence-first, safety-first Advanced Mode features. The plan strengthens replay evidence, local paper behavior, Alpaca reconciliation, risk-policy clarity, and feed/runner lifecycle handling.

## Current Owner Files

Portfolio:

- `crates/app_core/src/portfolio_feasibility.rs`
- `src-tauri/src/commands_portfolio.rs`
- `web/src/services/portfolioService.ts`
- `web/src/utils/portfolioWorkspaceModel.ts`
- `web/src/components/PortfolioFeasibilityWorkspacePanel.vue`

Live execution:

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/broker_paper.rs`
- `src-tauri/src/execution_policy.rs`
- `src-tauri/src/execution_quantity.rs`
- `src-tauri/src/execution_mocks.rs`
- `web/src/services/executionService.ts`
- `web/src/services/liveExecutionLifecycleMachine.ts`
- `web/src/stores/executionStore.ts`
- `web/src/utils/executionRisk.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `scripts/run_execution_safety_gate.ps1`
- `scripts/run_live_execution_safety_harness.ps1`

## Research Basis

- Alpaca orders have explicit lifecycle states and support client-provided order IDs; streaming updates are recommended for open-order state maintenance: <https://docs.alpaca.markets/docs/trading/orders/>
- Alpaca's order docs distinguish client-supplied order IDs and order lifecycle states; reconciliation should prefer stable IDs over UI-only labels: <https://docs.alpaca.markets/docs/orders-at-alpaca>
- Alpaca trade-update examples show streaming order updates and lookup by `client_order_id`, which supports an idempotent submission/reconciliation timeline: <https://docs.alpaca.markets/docs/working-with-orders>
- Alpaca bracket orders are parent/exit-order chains where take-profit and stop-loss children are conditional after fill: <https://docs.alpaca.markets/docs/trading/orders/>
- Alpaca market-data WebSocket messages may be batched, and slow clients can be disconnected if their outgoing buffer fills: <https://docs.alpaca.markets/docs/streaming-market-data>
- FINRA SR-FINRA-2025-017 proposes replacing current day-trading margin provisions with modern intraday margin standards; as of 2026-04-30, implementation must verify the actual effective date before any margin-related product copy or logic changes: <https://www.finra.org/rules-guidance/rule-filings/sr-finra-2025-017>
- Tauri security docs treat frontend-to-Rust commands as a trust boundary; live execution commands must remain explicitly scoped and risk-gated: <https://v2.tauri.app/security/>

## Safety Position

No feature in this plan should be presented as production trading advice or production trading readiness. Live execution remains Advanced Mode. Emergency controls must not be rate-limited, hidden behind analytics, or blocked by non-critical refresh pacing.

## Expansion Track A: Portfolio Replay Evidence Matrix

### Problem

Portfolio feasibility work depends on source artifacts and validation evidence. Users need to see which candidates are admissible and why.

### Implementation

Current owners:

- `crates/app_core/src/portfolio_feasibility.rs`
- `src-tauri/src/commands_portfolio.rs`
- `web/src/services/portfolioService.ts`
- `web/src/utils/portfolioWorkspaceModel.ts`
- `web/src/components/PortfolioFeasibilityWorkspacePanel.vue`

Add:

- per-candidate evidence status;
- missing evidence reasons;
- validation replay class;
- dataset/ticker/source artifact identity;
- portfolio source role;
- rejected-candidate table;
- exportable feasibility summary.

### Tests

- legacy artifacts without portfolio fields remain reviewable;
- candidate with missing validation evidence is rejected with reason;
- frontend model sorts rejected reasons deterministically.

## Expansion Track B: Portfolio Export Without Orders

### Problem

Portfolio outputs are useful for research, but generating orders would cross a risk boundary.

### Implementation

Add export-only outputs:

- JSON feasibility summary;
- CSV candidate/evidence rows;
- Markdown summary;
- optional XLSX only after report-shape plan and parity are added.

Do not output broker orders, order tickets, or live allocation instructions.

### Acceptance Criteria

- Export states "research feasibility only".
- No Alpaca API calls are made by portfolio export.
- Report/export tests cover path and content shape.

## Expansion Track C: Local Paper Bracket Preview

### Problem

The local paper broker supports bracket-like parent/child behavior. Users need a non-mutating preview before execution.

### Implementation

Current owners:

- `src-tauri/src/broker_paper.rs`
- `src-tauri/src/commands_execution.rs`
- `web/src/services/executionService.ts`
- `web/src/utils/executionRisk.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`

Add preview command/model:

- request position intent;
- risk policy;
- quantity calculation;
- stop-loss/take-profit presence;
- parent/child order group preview;
- same-bar tie-break disclosure for local paper;
- no broker mutation.

### Tests

- preview does not change broker state;
- bracket-required policy blocks missing child orders;
- stop-loss-first same-bar tie break is documented in preview;
- frontend emergency controls remain separate.

## Expansion Track D: Alpaca Reconciliation And Event Evidence

### Problem

REST order submission alone is not enough evidence for live state. Event reconciliation should be explicit.

### Implementation

Current owners:

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/execution_mocks.rs`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`

Add:

- stable `client_order_id` lookup evidence;
- post-submit broker reconciliation events;
- order lifecycle timeline;
- partial fill handling summary;
- cancel/replace/reject reason fields;
- credential-free intent records.

### Tests

- mock broker partial fills;
- reject after submit;
- timeout then idempotent lookup;
- cancel failure;
- duplicate/out-of-order feed events.

## Expansion Track E: Live Feed Backpressure And Runner Lifecycle

### Problem

Live execution depends on an active Polygon live stream and one active runner slot. Bursty status/events should not obscure safety state.

### Implementation

Current owners:

- `web/src/services/liveExecutionLifecycleMachine.ts`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `web/src/stores/taskStore.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/polygon.rs`

Add:

- explicit feed freshness model;
- runner disarmed reason;
- queued latest signal status;
- event ingestion counters;
- non-critical refresh pacing only for idempotent status reads.

Never pace:

- emergency halt;
- flatten position;
- cancel open orders;
- stop runner;
- future real order placement.

### Tests

- feed interruption disarms runner;
- emergency halt bypasses pacing;
- runner start requires matching active stream symbol;
- status spam does not hide terminal errors.

## Second-Pass Expansion: Execution Risk Policy V2

### Problem

The current safety position is correct, but live expansion needs a stricter policy object that can be reviewed independently from UI state.

### Current Anchors

- `src-tauri/src/execution_policy.rs`
- `src-tauri/src/execution_quantity.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/command-risk-policy.json`
- `web/src/utils/executionRisk.ts`
- `web/src/services/executionService.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`

### Policy Fields

Define an additive `ExecutionRiskPolicyV2`:

- account mode: local paper, broker paper, broker live;
- maximum notional per order;
- maximum open notional;
- maximum daily loss;
- maximum position count;
- bracket-required flag;
- market-hours policy;
- symbol allowlist;
- emergency action permissions;
- margin feature status: disabled, informational-only, externally verified;
- policy hash shown on order preview.

FINRA intraday-margin changes should be treated as an external regulatory input. Do not implement margin computation or product claims until the effective date and broker API fields are re-verified against current official sources.

### Acceptance Criteria

- local paper preview and broker submission both show the policy hash;
- policy failures block mutation before broker calls;
- emergency halt/cancel/flatten does not depend on policy refresh;
- policy serialization is redacted and artifact-safe.

## Second-Pass Expansion: Split Execution Command Hubs

### Problem

Broker, local paper, risk preview, and runner lifecycle operations have different blast radii. One broad command surface makes capability review harder.

### Implementation

Review and, if useful, split command ownership by risk:

- `execution_preview`: read-only preview and policy validation;
- `execution_local_paper`: local paper broker state mutation;
- `execution_broker_paper`: broker paper API calls;
- `execution_runner`: live runner start/stop/status;
- `execution_emergency`: halt/cancel/flatten commands.

Tauri capability policy:

- local development can expose diagnostic commands under `src-tauri/capabilities/dev.json`;
- public release capability should expose only the commands required by the selected variant;
- every command must appear in `src-tauri/command-risk-policy.json`.

### Tests

- command-risk policy covers each command;
- public capability excludes debug/internal execution commands;
- emergency commands remain available when status polling is degraded.

## Second-Pass Expansion: Session Recovery And Broker Watcher Boundary

### Problem

The product can support reconciliation evidence without committing to a long-lived broker watcher.

### Implementation

Phase 1: session recovery on app start or user action.

- list recent local intent records;
- lookup broker orders by `client_order_id`;
- reconcile terminal status, partial fill, cancel, reject, and unknown states;
- surface unresolved intents in Advanced execution diagnostics;
- never auto-submit replacement orders during recovery.

Phase 2: optional watcher design, deferred.

- separate background watcher lifecycle from the one-slot live strategy runner;
- define reconnect/backoff policy;
- define credential lifetime and revocation behavior;
- define OS sleep/resume handling;
- add a support bundle redaction path before enabling.

### Acceptance Criteria

- recovery is user-visible and non-mutating except for local evidence records;
- unknown broker state blocks any "all clear" copy;
- long-lived watcher remains out of scope until explicitly planned.

## Non-Goals

- No live brokerage in Simple Mode.
- No real-money default.
- No automatic order generation from portfolio feasibility.
- No multi-runner execution without a separate lifecycle redesign.
- No hidden credential storage in frontend localStorage.

## Verification

Targeted:

- `npm --prefix web run test -- src/services/executionService.test.ts src/services/liveExecutionLifecycleMachine.test.ts src/stores/executionStore.actions.test.ts src/components/ExecutionWorkspacePanel.test.ts src/services/portfolioService.test.ts src/utils/portfolioWorkspaceModel.test.ts`
- Rust execution/paper/portfolio tests as added.

Safety:

- `powershell -File scripts/run_execution_safety_gate.ps1`

Broad:

- `powershell -File scripts/run_repo_quality_gate.ps1`
