# Live Execution Session Audit Trail Plan

Last updated: 2026-04-12

## Purpose

Define a measured plan to add durable session evidence for the current live-execution implementation so the live mode can be reviewed after the fact instead of relying on transient status text and optimistic UI rows alone.

## Scope Boundaries

In scope:

- session-level evidence for the current non-options live mode
- runner lifecycle, signal, broker-action, and error evidence
- saved session artifacts or logs that can be reopened locally
- redaction and secret-handling rules for persisted live-session evidence

Out of scope:

- compliance-grade archival systems
- cloud sync or multi-user audit trails
- options live trading
- broad broker expansion

## Current User-Facing Behavior

The current execution surface exposes live and runner hints, optimistic submitted-order rows, broker snapshots, and output preview. That gives an operator immediate context while the app is open.

What is still missing is durable session evidence. Once the session is gone, there is no first-class artifact that explains:

- when live mode was armed
- what runner events occurred
- which signals resulted in orders
- what broker snapshots or errors were observed

## Current Code Surface

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/state.rs`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/stores/workflowArtifactsStore.ts`

## Current Verification Surface

- `web/src/stores/executionStore.actions.test.ts`
- `web/src/services/executionService.test.ts`
- `web/src/components/ExecutionWorkspacePanel.test.ts`
- targeted Rust tests in `src-tauri/src/commands_execution.rs`
- current artifact tests in `src-tauri/src/run_artifacts.rs`

## Strengths Already Present

- runner events already have a clear emitted vocabulary
- broker snapshots already exist as an explicit command surface
- the repo already has a strong artifact culture for non-live workflows
- there is already a centralized Rust artifact owner to extend rather than inventing a second storage model immediately

## Weaknesses / Failure Modes

- critical live-session context is currently transient
- optimistic rows and status hints are useful in-session but weak as after-the-fact evidence
- there is no discrete persisted boundary between local runtime hints and confirmed broker-state evidence
- without a session artifact, bug triage and operator review depend too heavily on memory or screenshots

## Regressions To Avoid

- persisting secrets or raw credentials in any live-session artifact
- writing so much session detail that the artifact becomes noisy and unusable
- introducing a second artifact truth system that drifts from the current run-artifact owners
- implying broker-confirmed state where the app only observed a local signal or submission attempt
- allowing this surface to stay behaviorally working while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Working with /orders](https://docs.alpaca.markets/docs/working-with-orders)
- [Working with /positions](https://docs.alpaca.markets/v1.3/docs/working-with-positions)
- [Tauri state management](https://v2.tauri.app/develop/state-management/)
- [Tauri path handling](https://v2.tauri.app/develop/filesystem/)

## Proposed Improvement Approach

1. Treat live execution as a discrete mode with its own session evidence contract.
2. Define a compact persisted session artifact that captures:
   - session start/stop posture
   - runner status changes
   - signal-processing outcomes
   - order-submission attempts and broker snapshot summaries
   - error and reconnect evidence
3. Keep provenance explicit inside the artifact:
   - local runtime event
   - optimistic submission event
   - broker-confirmed snapshot data
4. Reuse existing artifact plumbing where it stays readable; only create a separate live-session store if the normal run-artifact path becomes misleading.
5. Make secret redaction a hard requirement, not a best-effort note.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Define the minimum useful live-session artifact schema for the current scope.
- [x] Decide whether live-session evidence should live inside the existing artifact system or beside it.
- [x] Define the provenance vocabulary for local runtime, optimistic action, and broker-confirmed state.
- [x] Define redaction rules for secrets, account identifiers, and any sensitive broker payload fields.
- [x] Define when session artifacts are created, updated, finalized, and reopened.
- [x] Decide whether session evidence is opt-in, default-on, or mode-dependent for live versus paper.
- [x] Add or strengthen tests around artifact persistence, redaction, and reopen behavior.
- [x] Add a lightweight operator-facing review path for the saved live-session evidence.
- [x] Define rollback conditions if the persisted session format becomes too noisy or hard to trust.

## Verification Plan

- rerun targeted execution-store, execution-service, and execution-panel tests near touched files
- rerun targeted Rust tests in `src-tauri/src/commands_execution.rs`, `src-tauri/src/run_artifacts.rs`, and any new live-session owner module
- rerun `node scripts/run_output_parity_check.mjs` if the existing artifact plumbing or payload format is reused
- rerun `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`
- capture concrete acceptance evidence such as sample session artifacts, redaction proofs, and reopened-session review screenshots if a UI review surface is added

## Final Regression Validation Requirement

- Do not mark this item complete until the finished implementation state has been revalidated at the end of the work, not only while the artifact schema is being introduced.
- Final validation must include targeted unit/integration tests for event capture, artifact writing, redaction, reopen, and read-only review behavior, plus the relevant repo quality gates required by the final change set.
- Final validation must include browser, desktop, and accessibility checks for any new review surface, plus a paper/live-mode manual smoke path proving session capture and reopen do not re-arm runtime state.
- Final validation must include performance evaluation for any touched write-path, reopen-path, or history-surface flow so session auditing does not cause unacceptable persistence or review regressions.
- If the final integrated state introduces correctness, UX, safety, or performance regressions, narrow the slice, add a guard, or revert the offending portion before closure.

## Execution Notes

- Live-session evidence is now persisted as a distinct local artifact family under the existing runs directory, not through workflow hydration:
  - `src-tauri/src/run_artifacts.rs` owns create, append, finalize, list, and read-only detail load for `vectorgrid_live_session_v1`
  - `src-tauri/src/commands_artifacts.rs`, `src-tauri/src/commands.rs`, and `src-tauri/src/lib.rs` expose list/detail commands without reusing `currentRun`, `currentValidation`, or `latestWorkflowRunSnapshot`
- The persisted contract is compact and explicitly provenance-scoped:
  - session seed captures mode, symbol, qty, strategy kind, `data_id`, param source, and source-run context when present
  - bounded event and broker-snapshot lists preserve only review-safe summary fields
  - session files are finalized on explicit stop, interruption, supersession, or one-shot completion
- Redaction is enforced by construction rather than best-effort filtering after the fact:
  - raw Alpaca credentials, remembered-key payloads, raw account identifiers, raw balances, and raw output preview do not enter the saved-session artifact shape
  - reopened saved-session evidence remains read-only review data in the execution surface
- The operator review path is intentionally lightweight:
  - `web/src/services/executionService.ts` projects saved-session list/detail responses into typed read-only review models
  - `web/src/stores/executionStore.ts` keeps saved-session state separate from workflow-artifact hydration
  - `web/src/components/ExecutionWorkspacePanel.vue` exposes a manual-refresh “Saved session evidence” section rather than a broader session-history browser
- This epic closed for the current narrow live-mode scope only:
  - no workflow artifact hydration reuse
  - no re-arm on reopen
  - no cloud/archive semantics
  - no widened broker/runtime scope beyond the current Alpaca + Polygon, single-runner, single-stream model
- Acceptance evidence:
  - `cargo test -p ta_desktop_demo_app --lib live_session_`
  - `cargo test -p ta_desktop_demo_app --lib build_live_session_seed_preserves_mode_and_source_run_context`
  - `cargo test -p ta_desktop_demo_app --lib broker_open_order_status_matches_only_open_like_states`
  - `npm.cmd --prefix web run test -- src/services/executionService.test.ts src/stores/executionStore.actions.test.ts src/components/ExecutionWorkspacePanel.test.ts`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`
  - `npm.cmd --prefix web run test:browser:ux`
  - `npm.cmd --prefix web run a11y:smoke`
  - `npm.cmd --prefix web run test:desktop:smoke`

## Rollout / Fallback / Compatibility Notes

- start with compact additive evidence before attempting a broad session-history browser
- preserve the current transient hints and optimistic rows even if durable session evidence is added
- keep session artifacts local-first unless a separate follow-up doc explicitly widens scope
- do not widen this plan into general workflow-artifact redesign unless the integration point proves too awkward

## Related Docs

- `05_live_execution_productization_2026-04-10.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-09.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-09.md`
