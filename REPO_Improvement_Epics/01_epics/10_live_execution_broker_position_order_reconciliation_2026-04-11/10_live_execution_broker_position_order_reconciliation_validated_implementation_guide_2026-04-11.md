# Live Execution Broker Position And Order Reconciliation Validated Implementation Guide

Last updated: 2026-04-11

Companion to:
- `10_live_execution_broker_position_order_reconciliation_2026-04-11.md`

## Purpose

Freeze the validated reconciliation rules for broker-backed review inside the current execution surface before introducing a new summary layer.

## Scope Decision

The typed reconciliation model applies to broker-backed modes only:
- paper
- live

Dry-run is excluded from canonical broker reconciliation.

## Disabled-State Contract

Canonical reconciliation is unavailable until:
- one canonical nonblank symbol is selected
- the current mode is broker-backed
- a broker snapshot has been fetched for that same symbol/mode context

Blank-symbol refresh may remain available for manual review, but it must stay:
- table/raw-preview only
- non-chartable
- non-canonical

## Canonical Inputs

Canonical reconciliation inputs are typed data only:
- `get_live_execution_snapshot` payload
- backend freshness field `fetched_at_unix_ms`
- explicit desired-action / desired-position advisory state

Not canonical:
- rendered order rows
- formatted summary strings
- chart text
- raw preview
- optimistic `submitted*` rows
- generic store `updatedAtUnixMs`

## Freshness Contract

Broker freshness comes only from:
- `GetLiveExecutionSnapshotResponse.fetched_at_unix_ms`

Visible reconciliation states:
- `disabled`
- `unknown`
- `broker_confirmed_flat`
- `broker_confirmed_open`
- `broker_stale`
- `optimistic_local_pending`

## Open-Order Derivation

Snapshot review starts from `/v2/orders?status=all`.

The reconciliation layer must derive `open orders` from an explicit tested broker-status allowlist owned next to service normalization logic.

It must never:
- use total returned row count
- treat rendered table rows as canonical
- include optimistic rows in canonical open-order counts

## Precedence And Invalidation

Precedence:
- broker-confirmed snapshot
- optimistic local advisory state
- placeholder unknown state

Reconciliation state must invalidate on:
- symbol change
- mode change
- fresh snapshot completion
- runner stop
- feed stop when that changes the current review posture

Optimistic rows never drive:
- canonical open-order counts
- stale-age calculations
- hard broker-confirmed summary claims

## Submission Semantics Freeze

This bundle does not change backend submission math.

Specifically:
- `execute_strategy_signal_blocking` delta-to-target behavior remains unchanged
- this bundle owns clarity, warnings, and typed review state only
- any hard order-aware blocking belongs to a narrower backend-authoritative follow-up

## Owner Surface

- backend:
  - `src-tauri/src/commands_execution.rs`
- frontend service/store/panel:
  - `web/src/services/executionService.ts`
  - `web/src/stores/executionStore.ts`
  - `web/src/components/ExecutionWorkspacePanel.vue`
- compatibility review anchor:
- `REPO_Improvement/09_live/04_execution_order_position_review_2026-04-09.md`

## Preconditions

Before code changes start, freeze these choices in the parent bundle doc:

- canonical reconciliation is symbol-scoped only
- blank-symbol refresh cannot drive canonical summary state
- broker freshness comes only from backend snapshot timestamps
- this bundle is clarity/review work only, not OMS behavior

## Ordered Implementation Slices

1. Type widening slice
   - preserve typed backend snapshot fields in frontend service types
2. Canonical-state slice
   - add structured reconciliation state in the store
   - separate broker-confirmed, optimistic, and unknown/stale states
3. Derivation slice
   - derive canonical open-order state from the typed snapshot and explicit status allowlist
4. UI synthesis slice
   - render the new summary/warning layer while preserving review table and raw preview
5. Regression hardening slice
   - lock scenario coverage for scope, freshness, and invalidation

## File-Level Change Plan

- `src-tauri/src/commands_execution.rs`
  - preserve and, if needed, minimally extend typed snapshot fields without changing delta-submission behavior
- `web/src/services/executionService.ts`
  - widen snapshot/result types
  - normalize canonical reconciliation inputs and freshness fields
- `web/src/stores/executionStore.ts`
  - own the structured reconciliation state, precedence, and invalidation logic
- `web/src/components/ExecutionWorkspacePanel.vue`
  - render canonical summary/warnings with explicit provenance and scope
- `REPO_Improvement/09_live/04_execution_order_position_review_2026-04-09.md`
  - remains the compatibility anchor for preserving table/raw-preview truth

## Acceptance Evidence

Required evidence:

- blank-symbol canonical summary disabled while manual review remains possible
- unknown vs broker-confirmed flat rendered distinctly
- stale snapshot rendered distinctly from fresh broker state
- explicit proof that optimistic rows do not affect canonical open-order counts
- symbol/mode change invalidates stale reconciliation state
- no backend delta-submission behavior change

## Closure Conditions

Do not mark the bundle complete until:

- the canonical summary is more truthful than the current manual interpretation burden
- review table and raw preview remain available as fallback truth surfaces
- blank-symbol ambiguity is removed from canonical state
- freshness and provenance are typed, not string-derived
- no hidden OMS-style submission behavior has been introduced

## Verification Requirements

Acceptance matrix must cover:
- blank-symbol disabled canonical summary
- unknown vs broker-confirmed flat
- stale broker snapshot handling
- open-order derivation from `status=all`
- optimistic-vs-broker separation
- symbol/mode invalidation
- no hidden delta-submission change

Run:
- targeted Rust tests in `commands_execution.rs`
- targeted frontend tests in `executionService.test.ts`, `executionStore.actions.test.ts`, and `ExecutionWorkspacePanel.test.ts`
- `npm.cmd --prefix web run test:browser:ux` for any visible reconciliation summary or warning change

Raw preview stays protected as fallback evidence and must not be removed by this slice.

## Rollback Rule

If the new summary layer is less truthful than the existing review table plus raw preview, hide the summary layer and keep the manual-review surface authoritative.
