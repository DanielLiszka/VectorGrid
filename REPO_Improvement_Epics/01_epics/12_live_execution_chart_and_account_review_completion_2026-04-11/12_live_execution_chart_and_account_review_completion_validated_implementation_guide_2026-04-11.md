# Live Execution Chart And Account Review Completion Validated Implementation Guide

Last updated: 2026-04-11

Companion to:
- `12_live_execution_chart_and_account_review_completion_2026-04-11.md`

## Purpose

Freeze the validated review-truth contracts for chart, account, order, and position review before any visual completion work lands.

## Review Data Contract

The execution review surface must move from string-heavy status fields to structured review state.

Minimum structured review domains:
- symbol scope
- account review
- broker snapshot freshness
- position review
- order review
- chart review
- raw preview provenance

## Symbol Scope Rules

- chart review requires one explicit symbol
- position review requires one explicit symbol
- all-symbol order review may exist only as non-chart, no-position manual review
- no-symbol refresh must not populate symbol-level chart or position summaries
- the surface must visibly label `symbol-scoped` versus `all-symbol` review

## Account Review Source

The first authoritative account-review source is an extended broker snapshot contract, not free-form output text.

Account test remains:
- a connectivity/verification aid
- a source of supplemental account identity details

It is not the sole canonical account-review model for the completed surface.

## Freshness And Provenance Rules

Broker review freshness comes from backend fetch timestamps, not generic UI mutation time.

Required review freshness fields:
- source command
- authoritative fetch timestamp
- visible `as of` rule
- stale threshold

Provenance classes:
- broker-confirmed
- optimistic local
- local runtime advisory
- unavailable / suppressed

Configured live mode must stay separate from verified account state.

## Chart State Taxonomy

The chart surface must explicitly handle:
- no dataset loaded
- no symbol selected
- symbol mismatch
- broker review loaded but chart unavailable
- chart loaded with no broker markers
- chart loaded with broker-confirmed markers
- stale broker data
- marker suppression because provenance rules are not satisfied

The chart is never the sole carrier of execution truth.

## Raw Preview Contract

Raw preview remains an intentional troubleshooting surface.

It must always show:
- which command produced it
- what scope it covers
- when it was generated

The preview must not silently impersonate the current chart/order/account state.

## Owner Surface

- frontend:
  - `web/src/components/ExecutionWorkspacePanel.vue`
  - `web/src/services/executionService.ts`
  - `web/src/stores/executionStore.ts`
  - minimal chart helper reuse only from `web/src/services/analysisService.ts`
- backend/runtime:
  - `src-tauri/src/commands_execution.rs`
  - `src-tauri/src/state.rs`
  - `src-tauri/src/lib.rs`

Execution review stays settings-mounted in this slice; discrete surface promotion is separate work.

## Preconditions

Before code changes start, freeze these choices in the parent bundle doc:

- no chart without explicit symbol scope
- broker freshness must come from backend fetch timestamps only
- account review uses a structured typed source, not raw preview text
- chart completion may ship later than account/order/position review if provenance is not ready

## Ordered Implementation Slices

1. Structured review-state slice
   - define typed review models for account, scope, freshness, position, orders, and preview provenance
2. Scope and freshness slice
   - enforce symbol-scope rules
   - replace generic UI timestamps with broker-backed freshness
3. Account/order/position review slice
   - build the richer review surface before chart markers
4. Provenance slice
   - separate optimistic, broker-confirmed, advisory, and suppressed states visibly
5. Chart slice
   - add chart wiring and markers only after scope/freshness/provenance are explicit

## File-Level Change Plan

- `src-tauri/src/commands_execution.rs`
  - preserve or minimally extend typed snapshot/account fields needed for structured review
- `src-tauri/src/state.rs`
  - remain the runtime owner for dataset/live-bar readiness when review truth depends on loaded data
- `web/src/services/executionService.ts`
  - widen snapshot/account types and normalize structured review data
- `web/src/stores/executionStore.ts`
  - own structured review state and provenance, not just flattened strings
- `web/src/components/ExecutionWorkspacePanel.vue`
  - render structured account/order/position review first, then chart states
- `web/src/services/analysisService.ts`
  - reuse only minimal chart helper pieces if needed; do not import broader analysis ownership

Do not couple execution review to broader analysis-service state.

## Acceptance Evidence

Required evidence:

- no-symbol state with chart/position suppressed
- no-dataset state with truthful review copy
- symbol mismatch state
- stale broker snapshot state
- configured live mode vs verified account state shown distinctly
- optimistic-vs-broker review distinction visible in both summary and table
- chart suppressed or downgraded when provenance rules are not satisfied
- completed chart/account review screenshot once truthful state is ready

## Closure Conditions

Do not mark the bundle complete until:

- symbol scope is explicit and truthful
- broker freshness is backend-driven, not UI-mutation driven
- account/order/position review is structured and provenance-aware
- raw preview is still available with explicit source labeling
- chart completion never overclaims beyond the validated provenance rules

## Verification Requirements

Explicit scenario coverage is required for:
- no symbol selected
- no dataset loaded
- dataset/symbol mismatch
- stale broker snapshot freshness
- optimistic rows and counters
- configured live mode vs verified account state
- raw preview provenance

Run:
- targeted frontend tests in `ExecutionWorkspacePanel.test.ts`, `executionService.test.ts`, and `executionStore.actions.test.ts`
- targeted backend tests near execution snapshot/account payload changes
- `npm.cmd --prefix web run test:browser:ux`
- `npm.cmd --prefix web run a11y:smoke` when review layout changes materially

Screenshot evidence must include non-happy paths, not only the final polished state.

## Rollback Rule

If richer account/order/position review can be made truthful but chart provenance cannot, ship the richer review surface first and keep the chart in an explicitly suppressed or `sync pending` honesty state.
