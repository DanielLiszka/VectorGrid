# Live Execution Session Audit Trail Validated Implementation Guide

Last updated: 2026-04-11

Companion to:
- `07_live_execution_session_audit_trail_2026-04-11.md`

## Purpose

Convert the validated review findings for bundle `07` into a concrete implementation contract for a compact, local-first live-session evidence surface.

## Frozen Decisions

- Session evidence remains inside the current narrow live scope:
  - non-options only
  - current Alpaca broker path only
  - current Polygon live-feed path only
  - single active live stream and single active runner model only
- Reopened live-session artifacts are read-only review artifacts.
- A live-session artifact must not hydrate through `currentRun` / `currentValidation`.
- A live-session artifact must not mutate `latestWorkflowRunSnapshot`.
- If shared artifact plumbing is reused, it is reused only for local storage/list/detail/tag primitives, not for workflow hydration semantics.

## Owner Surface

- Persistence/detail owner:
  - `src-tauri/src/run_artifacts.rs`
  - `src-tauri/src/commands_artifacts.rs`
- Live-session event sources:
  - `src-tauri/src/commands_execution.rs`
  - `src-tauri/src/commands_data.rs`
  - `src-tauri/src/state.rs`
- Frontend review and projection:
  - `web/src/services/executionService.ts`
  - `web/src/stores/executionStore.ts`
  - `web/src/components/ExecutionWorkspacePanel.vue`
- Explicitly not a workflow hydration owner:
  - `web/src/stores/workflowArtifactsStore.ts`

## Artifact Contract

The first shipped artifact shape is a compact JSON summary with a bounded event list.

Required top-level fields:
- session id
- created/finalized timestamps
- current mode
- symbol
- qty
- `dry_run`
- `strategy_kind`
- `data_id`
- parameter source
- source run provenance when present:
  - `artifactPath`
  - `configFingerprint`
  - `routeProvenance`
- compact event list
- compact broker snapshot summaries
- tags only if the shared artifact owner is reused

## Redaction Contract

Never persist:
- `alpaca_api_key`
- `alpaca_api_secret`
- remembered-key JSON
- `apiKey`
- `apiSecret`
- `persistedApiKey`
- `persistedApiSecret`
- raw `ExecutionActionOptions`
- raw `outputPreview`

Account and broker payload policy:
- do not persist account id or account number
- do not persist raw balances or portfolio values in v1
- persist only compact status/provenance summaries needed for review

## Provenance Taxonomy

Every persisted event must declare one provenance class:

- `local_control`
  - operator-triggered start/stop/test/refresh actions
- `runner_lifecycle`
  - `started`, `triggered`, `queued_latest`, `stopped`, `error`
- `signal_outcome`
  - strategy processed with no broker submission
- `optimistic_submission`
  - local `submitted*` advisory state only
- `submission_attempt`
  - backend reported `order_submitted`
- `broker_snapshot`
  - snapshot pulled from Alpaca review commands
- `interruption`
  - feed close/error/end and related recovery evidence

The artifact must never treat `optimistic_submission` or `submission_attempt` as broker-confirmed fill or position truth.

## Write Lifecycle

The v1 write model is:
- create one session file when a live session is first armed
- update the same file in place with bounded event/state summaries during the session
- finalize it on explicit stop or detected terminal interruption

Prefix and placement:
- keep the artifact local-first
- if the shared artifact owner is reused, use a distinct live-session filename prefix and path-constrained load/list rules

## Reopen Contract

Reopened live-session artifacts:
- are review-only
- do not restore live feed state
- do not restore runner state
- do not repopulate workflow artifact hydration state
- do not silently re-arm broker-facing behavior

## Mode Capture Policy

The default capture policy for v1 is:
- live broker mode: default-on
- paper mode: optional and clearly labeled
- dry-run mode: off by default and always labeled simulation-only

## Review Surface

The first review surface stays lightweight:
- a saved session can be opened locally
- review focuses on compact summaries plus bounded events
- no broad session-history browser is introduced in this slice

## Preconditions

Before code changes start, freeze these choices in the parent bundle doc and keep them unchanged during implementation:

- shared artifact owner is reused only for local storage/list/detail primitives
- no workflow hydration reuse is allowed
- reconnect/feed evidence remains in scope for the session artifact
- live-session evidence stays local-first and compact

## Ordered Implementation Slices

1. Schema and provenance slice
   - define the v1 session schema
   - define the provenance taxonomy
   - define redaction-safe broker summary fields
2. Persistence contract slice
   - add the durable write/update/finalize/read-only detail path
   - keep file prefix, confinement, and reopen semantics explicit
3. Event capture slice
   - wire runner lifecycle and submission-attempt evidence
   - wire interruption/feed evidence if it stays in scope
4. Frontend review slice
   - add a lightweight read-only review path
   - keep it separate from `currentRun/currentValidation`
5. Hardening slice
   - add redaction, reopen, and no-snapshot-mutation regression coverage

## File-Level Change Plan

- `src-tauri/src/commands_execution.rs`
  - emit compact session events for runner lifecycle, signal outcomes, submission attempts, and errors
- `src-tauri/src/commands_data.rs`
  - emit compact feed/interruption evidence if reconnect evidence stays in scope
- `src-tauri/src/run_artifacts.rs`
  - add live-session artifact schema, persistence, detail/read-only load, and any distinct filename-prefix handling
- `src-tauri/src/commands_artifacts.rs`
  - expose list/detail/open helpers for the live-session artifact family if shared artifact plumbing is reused
- `web/src/services/executionService.ts`
  - add a read-only fetch/load path for saved live-session evidence
- `web/src/stores/executionStore.ts`
  - hold lightweight review state only; do not write back into workflow artifact state
- `web/src/stores/workflowArtifactsStore.ts`
  - no behavior changes to hydration ownership other than explicit regression protection
- `web/src/components/ExecutionWorkspacePanel.vue`
  - render compact saved-session review only if it can stay clearly separate from current live state

## Acceptance Evidence

Closure evidence must include:

- one sample session artifact from a dry-run or paper/live-safe test path
- one redaction proof showing secrets and forbidden account identifiers are absent
- one reopen proof showing read-only review only
- one proof that `latestWorkflowRunSnapshot` is unchanged by live-session artifact load
- one screenshot or UI capture of the saved-session review path if a review surface is added

## Closure Conditions

Do not mark the bundle complete until all of the following are true:

- live-session artifacts remain compact and readable
- no workflow hydration path is reused
- no forbidden secret/account fields are persisted
- reopen is read-only and non-arming
- review state does not mutate `currentRun/currentValidation`
- the shared artifact owner, if reused, still passes its no-regression checks

## Verification Requirements

- redaction tests for all forbidden secret fields
- no-regression tests proving no mutation of `latestWorkflowRunSnapshot`
- reopen tests proving read-only behavior only
- path-confinement tests if shared artifact plumbing is reused
- targeted Rust tests in:
  - `commands_execution.rs`
  - `commands_data.rs`
  - `run_artifacts.rs`
  - `commands_artifacts.rs`
- targeted frontend tests in:
  - `executionService.test.ts`
  - `executionStore.actions.test.ts`
- `ExecutionWorkspacePanel.test.ts`
- `npm.cmd --prefix web run parity:outputs` if artifact/output shape changes

## Final Regression Validation Requirement

- Do not close this guide’s implementation work until the finished integrated state has been revalidated at the end of the work, not only during intermediate slices.
- Final validation must rerun the targeted backend/frontend tests against the completed session-audit shape, then run the relevant repo quality gates required by the final change set.
- Final validation must include any required browser, desktop, and accessibility checks for added review UI, plus a paper/live-mode manual smoke path proving session capture and reopen remain read-only.
- Final validation must include performance evaluation for touched write, reopen, and review flows so the final state does not regress persistence cost or history responsiveness.
- If the final integrated state introduces correctness, UX, safety, or performance regressions, narrow the slice, add a guard, or revert the offending portion before closure.

## Rollback Rule

If the session artifact cannot remain compact, redaction-safe, and clearly more trustworthy than transient status plus screenshots, rollback to the current transient review model and keep this bundle open.
