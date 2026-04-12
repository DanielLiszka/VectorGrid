# Live Execution Chart And Account Review Completion Plan

Last updated: 2026-04-11

## Purpose

Define a measured plan to complete the execution review surface so discrete live mode includes a truthful chart-and-account review experience instead of a partially wired chart host and thin account summary.

## Scope Boundaries

In scope:

- completing the execution chart review surface
- account summary and broker-state review for the current live-execution panel
- order and position overlays or markers when they can be made truthful
- explicit provenance for chart, account, order, and position review fields

Out of scope:

- a full portfolio dashboard
- generalized charting redesign outside the execution/live surface
- options live trading
- broker expansion beyond current Alpaca-backed execution

## Current User-Facing Behavior

The execution panel already exposes a chart host and broker review sections, but the current execution chart is not fully wired. The service still reports `Execution chart sync pending`, and account review is thinner than the rest of the execution status surface.

That means the user can review live/runtime status, order rows, and position text, but not yet a completed chart-and-account review surface that matches the seriousness of discrete live mode.

## Current Code Surface

- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `web/src/services/analysisService.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/lib.rs`

## Current Verification Surface

- `web/src/components/ExecutionWorkspacePanel.test.ts`
- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- browser UX coverage for execution review surfaces
- targeted Rust tests near execution snapshot/account responses

## Strengths Already Present

- the execution panel already has a clear monitor/review section
- position summary and order review already exist
- the app already uses lightweight-charts elsewhere, so there is a charting precedent to reuse
- the broker snapshot command can already return richer account and order/position state

## Weaknesses / Failure Modes

- the execution chart surface is still effectively a placeholder/review shell
- account review is thin relative to what the backend can already expose
- a chart can become misleading if markers, loaded data, and broker state are not clearly separated
- operator trust suffers if the chart host is visible but not actually authoritative enough to review live state

## Regressions To Avoid

- plotting optimistic or stale data as if it were broker-confirmed truth
- overloading the chart with too much execution detail before provenance is explicit
- weakening current order-table or position-text review in favor of a prettier but less trustworthy chart
- coupling execution review too tightly to unrelated analysis charts or analysis state
- allowing this surface to stay behaviorally working while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Lightweight Charts documentation](https://tradingview.github.io/lightweight-charts/)
- [Working with /orders](https://docs.alpaca.markets/docs/working-with-orders)
- [Working with /positions](https://docs.alpaca.markets/v1.3/docs/working-with-positions)

## Proposed Improvement Approach

1. Complete the execution review surface in two layers:
   - broker/account review summary
   - chart review with explicit provenance and marker rules
2. Reuse the existing charting approach already present in drilldown/lightweight chart flows where that improves consistency.
3. Make account review richer before making the chart more decorative.
4. Keep the chart explicitly advisory unless and until every plotted element can be tied to a truthful source.
5. Preserve the detailed order table and position summary as first-class review surfaces even if chart overlays improve.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Record the current execution chart gap and define the minimum review-complete chart behavior.
- [ ] Decide which account fields should become first-class in the discrete live-mode review surface.
- [ ] Define which chart overlays can be treated as broker-confirmed, locally optimistic, or advisory only.
- [ ] Define the minimum loaded-dataset and broker-refresh prerequisites before execution chart markers are shown.
- [ ] Preserve the current order and position review surfaces while chart/account review expands.
- [ ] Add or strengthen frontend tests for chart-meta, account summaries, and review-state provenance.
- [ ] Add or strengthen backend tests if snapshot/account payload shape changes.
- [ ] Capture screenshots and acceptance evidence for the completed execution review surface.
- [ ] Define rollback conditions if the chart becomes more visually impressive but less truthful.

## Verification Plan

- rerun targeted execution panel, execution store, and execution service tests near touched files
- rerun any lightweight-chart review tests that become shared with execution review
- rerun `npm.cmd --prefix web run test:browser:ux`
- rerun `npm.cmd --prefix web run a11y:smoke` if the review surface changes materially
- rerun `npm.cmd --prefix web run lint`
- rerun `npm.cmd --prefix web run typecheck`
- rerun `cargo check --workspace`
- run `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`

## Final Regression Validation Requirement

- Do not mark this item complete until the finished implementation state has been revalidated at the end of the work, not only while the chart/account surface is being filled in.
- Final validation must include targeted unit/integration tests for symbol scoping, freshness/provenance mapping, account-summary truth, and chart-state rendering, plus the relevant repo quality gates required by the final change set.
- Final validation must include browser, desktop, and accessibility checks for the review surface, plus a paper/live-mode manual smoke path proving broker-confirmed versus optimistic state remains visibly distinct.
- Final validation must include performance evaluation for any touched chart sync, account refresh, or review-render path so the final state does not regress operator responsiveness.
- If the final integrated state introduces correctness, UX, safety, or performance regressions, narrow the slice, add a guard, or revert the offending portion before closure.

## Rollout / Fallback / Compatibility Notes

- complete account review first if the chart cannot yet be made truthful enough
- preserve the current `chart sync pending` honesty until real chart review is ready
- keep detailed order/position text available even if the chart becomes the more prominent review surface
- do not widen this plan into a full portfolio or watchlist charting feature

## Related Docs

- `05_live_execution_productization_2026-04-10.md`
- `09_live_mode_discrete_surface_promotion_2026-04-11.md`
- `10_live_execution_broker_position_order_reconciliation_2026-04-11.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`
- `REPO_Improvement/09_live/04_execution_order_position_review_2026-04-09.md`
