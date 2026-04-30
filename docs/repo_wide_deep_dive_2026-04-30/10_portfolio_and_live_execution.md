# Portfolio And Live Execution Implementation Plan

## Surface

Primary files:

- `crates/app_core/src/portfolio_feasibility.rs`
- `src-tauri/src/commands_portfolio.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/broker_paper.rs`
- `src-tauri/src/execution_policy.rs`
- `web/src/services/portfolioService.ts`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `web/src/components/PortfolioFeasibilityWorkspacePanel.vue`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/e2e/browser/execution-review-surface.spec.ts`
- `scripts/run_execution_safety_gate.ps1`

## Current Strengths

- Portfolio feasibility exists as a research/evidence lane rather than pretending to be a full allocator.
- Local paper execution is safety-focused and distinct from live brokerage paths.
- Execution policy code already centralizes important risk checks.
- Credentials and provider secrets are not treated as ordinary frontend state.

## Improvement Plan

### PORT-01: Harden Existing Portfolio Replay Matrix Contract

Risk: Medium.

Problem:
Portfolio feasibility already detects replay-matrix evidence through `PortfolioReplayEvidenceStatus` and `require_replay_matrix`. The improvement is not to invent a second concept, but to make the existing contract visible, stricter, and reportable.

Implementation:

- Promote the current replay-matrix detection into a named documented contract.
- Require any admitted "allocation-ready" candidate to expose:
  - source artifact IDs
  - source artifact hashes
  - ticker/data fingerprints
  - strategy family
  - date overlap
  - allocation assumption
  - covariance/correlation matrix schema and sample window
  - fees/slippage assumption
  - replay exactness class
- Preserve the existing `PortfolioFeasibilityRequestV1.require_replay_matrix` behavior.
- Show matrix status in Portfolio UI and reports.
- Block exact replay claims when any input lacks content identity.

Files:

- `crates/app_core/src/portfolio_feasibility.rs`
- `src-tauri/src/commands_portfolio.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/portfolioService.ts`
- `web/src/components/PortfolioFeasibilityWorkspacePanel.vue`

Verification:

- `cargo test -p ta_app_core --lib portfolio_feasibility`
- `cargo test -p ta_desktop_demo_app --lib commands_portfolio`
- `cargo test -p ta_desktop_demo_app --lib run_artifacts`
- `npm --prefix web run test -- src/services/portfolioService.test.ts`

Acceptance criteria:

- Existing tests around `PortfolioReplayEvidenceStatus` continue to pass.
- No target weights or allocation-frontier payload is emitted until co-timed replay evidence exists.
- The UI shows whether the result is missing replay evidence, replay-required, replay-present, or not applicable.
- Legacy artifacts remain inspectable but are not promoted to allocation-ready.

### PORT-02: Allocation Diagnostics Without Auto-Allocation

Risk: Low/medium.

Problem:
Useful portfolio work can be added without pretending the app is ready to allocate user capital automatically.

Implementation:

- Add diagnostics for:
  - correlation/covariance quality
  - matrix rank/conditioning where cheap
  - date overlap
  - missing ticker windows
  - concentration
  - drawdown contribution
  - exposure by strategy family
- Keep allocation suggestions labeled as diagnostics/research.
- Do not turn diagnostics into live execution orders.

Files:

- `crates/app_core/src/portfolio_feasibility.rs`
- `src-tauri/src/commands_portfolio.rs`
- `web/src/services/portfolioService.ts`
- `web/src/components/PortfolioFeasibilityWorkspacePanel.vue`

Verification:

- `cargo test -p ta_app_core --lib portfolio_feasibility`
- `cargo test -p ta_desktop_demo_app --lib commands_portfolio`
- `npm --prefix web run test -- src/services/portfolioService.test.ts`

### LIVE-01: Execution Risk Policy V2

Risk: Medium.

Problem:
Execution risk policy currently covers max order notional, max position notional, shorting permission, and bracket requirements. That is the right center of gravity, but public release readiness needs a versioned policy object with more complete audit and replay context.

Implementation:

- Add execution policy versioning.
- Persist:
  - max order value
  - max position value
  - max daily loss
  - max open orders
  - allowed symbols
  - allowed order types
  - live/paper mode
  - provider
  - kill-switch status
  - same-bar bracket behavior
- Include policy hash in execution artifacts.

Files:

- `src-tauri/src/execution_policy.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/broker_paper.rs`
- `web/src/services/executionService.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib execution_policy`
- `cargo test -p ta_desktop_demo_app --lib commands_execution`
- `npm --prefix web run test -- src/services/executionService.test.ts`
- `powershell -File scripts/run_execution_safety_gate.ps1`

### LIVE-02: Local Paper Fill Realism And Session Evidence Completeness

Risk: Low/medium.

Problem:
Local paper execution already has bracket handling and live-session artifact/event capture. The improvement is to make paper fills and session evidence more complete so results cannot look more precise than their assumptions justify.

Implementation:

- Add or formalize settings for:
  - fee model
  - spread/slippage model
  - partial fill mode
  - market-on-close handling
  - same-bar stop/target priority
  - rejected order behavior
- Store settings in run/session artifacts.
- Append structured session events for submitted, rejected, filled, cancelled, and reconciled local-paper actions.
- Show a clear "local simulation" label in the UI.

Files:

- `src-tauri/src/broker_paper.rs`
- `src-tauri/src/execution_policy.rs`
- `src-tauri/src/commands_execution.rs`
- `web/src/services/executionService.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib broker_paper`
- `cargo test -p ta_desktop_demo_app --lib commands_execution`
- `npm --prefix web run test -- src/services/executionService.test.ts`
- `powershell -File scripts/run_execution_safety_gate.ps1`

### LIVE-03: Split Execution Command Hubs

Risk: Medium.

Problem:
`commands_execution.rs` is a large boundary surface. Splitting by responsibility would lower future regression risk.

Implementation:

- Split into modules such as:
  - command DTOs
  - risk policy
  - paper broker commands
  - provider credential commands
  - session/reconciliation commands
- Keep public Tauri command names stable.
- Move tests with the modules.
- After any split, run command-binding and risk-policy checks so the frontend command registry, Tauri handler list, Specta bindings, and risk policy remain in sync.

Files:

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_execution_policy.rs` (new)
- `src-tauri/src/commands_execution_paper.rs` (new)
- `src-tauri/src/commands_execution_provider.rs` (new)

Verification:

- `cargo test -p ta_desktop_demo_app --lib commands_execution`
- `cargo test -p ta_desktop_demo_app --lib execution_policy`
- `npm --prefix web run check:commands`
- `node web/scripts/check-tauri-command-risk-policy.mjs`
- `powershell -File scripts/run_execution_safety_gate.ps1`

### LIVE-04: Session Recovery And Reconciliation Evidence

Risk: Medium/high.

Problem:
Any live or paper execution session needs explicit recovery and reconciliation semantics.

Implementation:

- Add session ledger entries:
  - command accepted
  - order submitted
  - order rejected
  - order filled/partial/cancelled
  - broker state snapshot
  - local state snapshot
  - reconciliation result
- Add recovery behavior after restart for paper sessions.
- Keep live provider reconciliation read-only unless the user explicitly enables live trading.
- Store enough event ordering information to distinguish command acceptance, broker submission, broker acknowledgement, local artifact append, and reconciliation completion.

Files:

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/broker_paper.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/executionService.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib broker_paper`
- `cargo test -p ta_desktop_demo_app --lib commands_execution`
- `npm --prefix web run test -- src/services/executionService.test.ts`
- `powershell -File scripts/run_execution_safety_gate.ps1`

### LIVE-05: Keep Live Execution Advanced-Only

Risk: Low.

Problem:
Simple Mode should remain focused on backtest optimization and optional validation. Live execution would materially raise user risk.

Implementation:

- Ensure Simple Mode never starts live broker flows.
- If execution artifacts appear in history, show read-only summaries in Simple Mode.
- Keep all credential and live mode settings in Advanced Mode.

Files:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `web/e2e/browser/execution-review-surface.spec.ts`

Verification:

- `npm --prefix web run test -- src/components/GuidedWorkspacePanel.test.ts src/components/ExecutionWorkspacePanel.test.ts`
- `powershell -File scripts/run_frontend_e2e_gate.ps1`
- `powershell -File scripts/run_execution_safety_gate.ps1`

## Acceptance Criteria For This Guide

- Portfolio work remains evidence/diagnostic-only until co-timed replay evidence is present.
- Execution policy changes are versioned and serialized into session/order evidence.
- Paper fills disclose local simulation assumptions for fees, spread, slippage, partial fills, and bracket ordering.
- Live broker controls remain Advanced-only and keep existing credential/risk gates.
- Splitting `commands_execution.rs` preserves public Tauri command names.

## Deferred Higher-Risk Work

- Broker order placement from Simple Mode.
- Real capital allocation suggestions.
- Multi-broker abstraction before paper and Alpaca semantics are fully audited.
