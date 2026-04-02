# Metric Contract Unification Plan

Date: 2026-04-02

## Purpose

This plan defines a phased path to support a canonical optimization metric surface across all strategy families without turning the current Double-MA-only `metric_contract` switch into a cross-strategy debt trap.

The target is:

- a universal `Core17` metric surface for saved optimization artifacts and downstream consumers
- a gated `Trade22` extension for strategies and routes that can prove correct trade-summary metrics

This plan is intentionally separate from the campaign runner plan. The campaign runner is orchestration work. Metric-contract unification is result-contract and metrics-correctness work.

## Canonical Surfaces

### `Core17`

The current exact-kernel 17-row surface is:

1. `pnl`
2. `trades`
3. `max_dd`
4. `mean`
5. `std`
6. `exposure`
7. `net_exposure`
8. `downside_deviation`
9. `ulcer_index`
10. `omega`
11. `downside_frequency`
12. `downside_severity`
13. `long_mean_return`
14. `short_mean_return`
15. `kappa_tau`
16. `underwater_fraction`
17. `max_underwater_duration_bars`

### `Trade22`

`Trade22` extends `Core17` with:

18. `closed_trades`
19. `winning_trades`
20. `losing_trades`
21. `gross_profit`
22. `gross_loss`

## Executive Summary

Universal `Core17` support is feasible, but it is not just a UI toggle.

- The shared `Metrics` type already carries most of the needed fields.
- The explicit `Full17` / `Trade22` switch is still a Double-MA exact-kernel feature today.
- Some non-Double-MA paths still zero-fill parts of the richer metric surface, so they are not yet truly contract-complete.

The recommended implementation shape is:

- introduce a strategy-agnostic metric-surface concept for artifacts and downstream consumers
- make `Core17` the first universal target
- add `Trade22` only for routes with correct and tested trade summaries
- keep the current Double-MA exact-kernel switch as an implementation source, not as the permanent cross-strategy product contract

## Why This Needs Its Own Plan

The current support boundary is uneven:

- the explicit metric-contract switch is route-gated to exact `GpuKernel` paths in the Double-MA family
- validation and composite paths currently suppress the contract fields in their request helpers
- some standalone and composite paths still synthesize missing fields as zeros rather than computing them

That means the work splits into three different concerns:

- canonical contract definition
- per-strategy metrics correctness
- product exposure and artifact/export normalization

Those concerns are adjacent to the campaign runner, but not owned by it.

## Current State

### Reusable foundations

- Shared metrics shape already exists in `crates/strategies/src/double_ma/mod.rs`.
- Optimization artifacts already persist the returned metric payloads.
- Meta and compare stages already consume saved artifact metrics rather than recomputing them from scratch.

### Current product limits

- The explicit `metric_contract` control is still tied to the Double-MA request path.
- Frontend request shaping rejects explicit metric contracts unless the route is the exact `GpuKernel` route.
- Backend request validation repeats the same guard.
- Validation and composite request builders currently force `metric_contract: None` and `include_trade_metrics: false`.

### Current backend correctness gaps

- Some non-Double-MA paths still assign zeros to richer fields such as downside-frequency, long/short mean return, underwater summaries, and trade-distribution metrics.
- That means universal `Core17` is partly contract plumbing and partly backend metrics work.
- Universal `Trade22` is a larger step because trade-summary correctness is stricter than simply widening the row shape.

## Goals

- Define one canonical cross-strategy optimization metric surface.
- Make `Core17` available in saved optimization artifacts for all strategies that already support optimization.
- Preserve CPU-only correctness and fallback behavior.
- Avoid silently claiming `Trade22` support where trade summaries are not actually computed.
- Keep downstream consumers such as validation, compare, meta, and optional ML able to reason over a normalized metric table.

## Non-Goals

- Forcing every strategy to expose a user-facing `metric_contract` selector in the first rollout.
- Pretending zero-filled trade metrics count as real `Trade22` support.
- Rewriting the Double-MA kernel metric layout purely for naming consistency.
- Coupling campaign orchestration rollout to metric-surface unification rollout.

## Recommended Architecture

### Contract layer

- Introduce a generic metric-surface label for optimization artifacts, for example `OptimizationMetricSurface::{Legacy, Core17, Trade22}`.
- Treat `DoubleMaMetricContract` as one producer-side implementation detail rather than the universal product contract.
- Centralize metric-row projection and artifact serialization so row order and field names are not redefined in multiple places.

### Capability layer

- Add per-strategy and per-route capability reporting for `Core17` and `Trade22`.
- Keep `Trade22` capability narrower than `Core17` until correctness is proven.
- Allow product/UI exposure to follow capabilities rather than hardcoded family checks.

### Correctness layer

- Replace zero-filled placeholders with real calculations where feasible.
- If a field still cannot be produced on a route, keep that route out of the corresponding contract tier.
- Require strategy-local tests for every route that claims `Core17` or `Trade22`.

## Likely Primary Touchpoints

### Contract and shared surface

- `crates/strategies/src/double_ma/mod.rs`
  - shared `Metrics` shape and field semantics
- `crates/optimizer/src/lib.rs`
  - optimization result shape persisted into artifacts
- `src-tauri/src/run_artifacts.rs`
  - saved artifact serialization and schema compatibility

### Existing Double-MA exact-kernel contract path

- `crates/app_core/src/double_ma.rs`
  - request validation and route gating for explicit metric contracts
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - existing 17/22 metric-count layout and fixed-contract dispatch
- `web/src/services/optimizeRequestBuilder.ts`
  - current frontend metric-contract normalization and guards

### Non-Double-MA routes that need auditing

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/asr.rs`
- `crates/app_core/src/composite_gpu_metric_decode.rs`

### Product capability and exposure layer

- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeService.ts`
- compare/export readers that consume saved optimization artifacts

## Proposed First Implementation Slice

The safest first slice is not universal product exposure. It is artifact normalization.

- [ ] Add generic metric-surface metadata to new optimization artifacts.
- [ ] Add centralized `Core17` projection helpers.
- [ ] Use those helpers in one non-Double-MA route that already comes close to `Core17`.
- [ ] Prove compare/export/meta can consume the normalized surface without caring which strategy produced it.

Exit criteria:

- at least one non-Double-MA route writes a canonical `Core17` surface
- downstream readers consume the canonical surface without route-specific branching

## Phase Plan

### Phase 0: contract freeze and audit

- [ ] Freeze the canonical `Core17` field list and order.
- [ ] Freeze the canonical `Trade22` field list and order.
- [ ] Build a route-by-route support table: native, derivable, missing, zero-filled.
- [ ] Map each field to its current computation source or placeholder source.
- [ ] Decide whether artifact readers should version on surface name, schema version, or both.

Exit criteria:

- one canonical field order is documented
- every optimization route is classified against `Core17` and `Trade22`

### Phase 1: generic artifact surface

- [ ] Add a strategy-agnostic artifact field for metric surface metadata.
- [ ] Add centralized helpers to project `Metrics` into canonical `Core17` and `Trade22` rows.
- [ ] Normalize compare/export/meta readers to prefer the new surface metadata when present.
- [ ] Keep legacy artifacts readable without migration.

Exit criteria:

- new artifacts can declare which canonical metric surface they contain
- downstream readers no longer depend on ad hoc row-shape assumptions

### Phase 2: universal `Core17` correctness

- [ ] Audit every optimizing strategy path for fields that are still zero-filled or omitted.
- [ ] Implement missing `Core17` computations route by route, starting with standalone and composite GPU/CPU paths.
- [ ] Add route-local tests proving the non-zero-capable fields are computed from actual path state rather than placeholders.
- [ ] Update capability summaries so product code can detect `Core17` availability directly.

Exit criteria:

- every strategy that supports optimization can emit a correct `Core17` surface
- tests exist for each route family that claims `Core17`

### Phase 3: `Trade22` correctness expansion

- [ ] Identify which strategies already have correct trade-level reconstruction and summary data.
- [ ] Implement missing trade-summary computations where practical.
- [ ] Keep `Trade22` disabled for any route that still relies on placeholders.
- [ ] Add strict tests for `closed_trades`, `winning_trades`, `losing_trades`, `gross_profit`, and `gross_loss`.

Exit criteria:

- every route that advertises `Trade22` computes all five trade-extension fields correctly
- unsupported routes remain explicitly out of scope rather than silently degraded

### Phase 4: product exposure

- [ ] Decide whether the user-facing control becomes a universal selector or remains strategy-specific.
- [ ] Update frontend capability summaries and request builders to use the generic surface capability flags.
- [ ] Update saved-run compare/export flows to surface the canonical metric surface cleanly.
- [ ] Document how validation, meta, and optional ML should consume canonical surfaces.

Exit criteria:

- product exposure follows real route capability rather than family-specific hardcoding
- users can tell which strategies support which metric surface

## Testing Checklist

- [ ] Add unit tests for centralized `Core17` / `Trade22` row projection.
- [ ] Add route-level tests for at least one standalone strategy currently zero-filling fields.
- [ ] Add route-level tests for at least one composite route.
- [ ] Add artifact round-trip tests covering legacy and new metric-surface metadata.
- [ ] Add compare/export tests proving the canonical row order is stable.
- [ ] Add negative tests proving routes that lack trade-summary correctness do not claim `Trade22`.

## Risks

- The current Double-MA exact-kernel contract may tempt us to over-generalize a backend-specific implementation detail.
- Some strategies may need non-trivial trade reconstruction to support true `Trade22`.
- If we expose a universal product toggle too early, UI promises can outrun backend correctness.
- Saved artifact compatibility can get messy if row shape and schema metadata drift separately.

## Open Questions

- Should `Core17` become the default saved artifact surface everywhere, even when the user never picks a metric-surface override?
- Should `Trade22` ever be auto-promoted, or only requested explicitly?
- Do we want one universal product label, or separate labels for artifact surface and optimization backend contract?
- Should optional ML treat `Core17` as a hard prerequisite for campaign-wide feature extraction?

## Recommendation

Proceed in this order:

1. define the canonical `Core17` / `Trade22` surfaces and artifact metadata
2. make `Core17` truly correct across all optimizing strategies
3. expand `Trade22` only where trade summaries are proven and tested
4. then decide how much of that should become user-facing product control

That keeps the work honest. It gives the campaign runner and later ML stages a normalized artifact surface without pretending that every route already supports the full Double-MA kernel contract today.
