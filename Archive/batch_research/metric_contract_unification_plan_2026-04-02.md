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
- The explicit `Full17` / `Trade22` switch originated on the Double-MA exact-kernel path, but audited non-exact routes now expose it selectively across the broader product surface.
- The remaining gaps are no longer the ordinary static strategy matrix; they now sit primarily in the separate `MlSignal` workflow and conservative stale-artifact inference for older artifacts without runtime hints.

The recommended implementation shape is:

- introduce a strategy-agnostic metric-surface concept for artifacts and downstream consumers
- make `Core17` the first universal target
- add `Trade22` only for routes with correct and tested trade summaries
- keep the current Double-MA exact-kernel switch as an implementation source, not as the permanent cross-strategy product contract

## Progress Tracking

- Phase 0: completed
- Phase 1: completed
- Phase 2: completed
- Phase 3: completed
- Phase 4: completed

First verified slice landed on 2026-04-04:

- backend-owned `optimization_metric_surface` metadata is now written into saved optimization artifacts
- centralized `Core17` / `Trade22` projection helpers now live in `src-tauri/src/metric_surface.rs`
- artifact hydration/detail readers now thread the surface metadata through to the frontend
- saved-run history and compare summaries now display the declared metric surface when present
- meta-selection now emits warnings when train/holdout corpora mix `Legacy` vs `Core17` / `Trade22` surfaces and persists a machine-readable metric-surface summary in saved meta artifacts
- metric-surface schema version is now `2`; stale schema metadata is re-derived rather than blindly trusted
- the first conservative non-Double-MA routes are the exact-route `CompositeTrend` and `BreakoutMomentum` paths plus `CompositeCpu`
- `CompositeCpu` now computes the previously zero-filled Core17-only risk fields (`downside_frequency`, `downside_severity`, `kappa_tau`, `long_mean_return`, `short_mean_return`, `underwater_fraction`, `max_underwater_duration_bars`) through the shared composite metrics path
- standalone CPU-route artifacts now classify as `Core17` when the realized optimization route is `CpuOnly`, including `GpuKernel->CpuOnly` fallback artifacts and older CPU-requested artifacts with missing runtime route metadata
- standalone `ASR`, `VCC`, `VEMR`, `ROCB`, `VCET`, `KQSR`, `CARM`, `MKCS`, `KVDF`, and `CDMC` `GpuKernel` routes now emit truthful `Core17` surfaces; their widened GPU decode paths now compute the Core17-only fields directly, and newly saved artifacts promote through a runtime-authored `run.res.metric_surface_kind=core17` marker rather than a broad legacy reclassification heuristic
- audited Double-MA-family optimization artifacts now also write conservative runtime-authored metric-surface hints instead of relying entirely on downstream re-derivation: non-exact full-metric `DoubleMa`-family routes self-declare `Core17` / `Trade22`, exact-kernel routes self-declare only when 17-row or 22-row semantics were explicitly requested, exact audited `CompositeTrend` / `BreakoutMomentum` runs do the same, and exact-compatible `DynamicDoubleMa` runs self-declare only when they explicitly request 17-row / 22-row semantics
- `CompositeCpu` now supports explicit `Trade22` requests through bounded CPU-side trade-stat enrichment of returned rows, and standalone non-Double-MA `GpuKernel` routes now support explicit `Trade22` requests through bounded host-side tail completion after GPU search
- plain `DynamicDoubleMa` CPU fallback surfaces now support explicit `Full17` / `Trade22` requests and self-declare `core17` / `trade22` in new artifacts; unsupported dynamic composite/profile variants remain backend-rejected rather than silently upgraded
- `CompositeMo` Pareto artifacts now self-declare `trade22` and carry canonical metric-surface rows projected from their saved Pareto-point metrics through bounded post-search trade-tail enrichment, while legacy unstamped CompositeMo artifacts remain conservatively `Legacy`
- remaining route gaps now sit primarily in the separate `MlSignal` workflow plus conservative handling of older artifacts that predate runtime-authored surface hints

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

- The explicit `metric_contract` control is no longer exact-route-only: audited Double-MA-family CPU / hybrid / exact-GPU routes, standalone CPU / requested-`GpuKernel` routes, and `CompositeCpu` now serialize and honor explicit `Full17` / `Trade22` requests.
- The remaining exact-route gating is now concentrated in advanced shortlist, slice, confidence, and export-all controls rather than the metric-contract selector itself.
- Dynamic `strategy_ir` now supports explicit metric surfaces across exact-compatible `dynamic_strategy_v1` shapes and audited CPU fallback surfaces, including constrained composite/profile plans that resolve to CPU.
- `CompositeMo` now emits canonical `Trade22` artifact surfaces by default for saved Pareto points, but it still sits outside the explicit metric-contract request surface.
- `MlSignal` remains outside the canonical metric-contract request surface because it is a separate validation/sweep workflow rather than an optimizer result surface.

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

- [x] Add generic metric-surface metadata to new optimization artifacts.
- [x] Add centralized `Core17` projection helpers.
- [x] Use those helpers in one non-Double-MA route that already comes close to `Core17`.
- [x] Prove compare/export/meta can consume the normalized surface without caring which strategy produced it.

Current note:

- history and compare consumers are using the new surface metadata
- export now carries the surface metadata through report payload construction
- meta now persists a surface summary and warns on mixed or inferred train/holdout corpora
- the next real work is Phase 2 route-by-route correctness, not more Phase 1 plumbing

Exit criteria:

- at least one non-Double-MA route writes a canonical `Core17` surface
- downstream readers consume the canonical surface without route-specific branching

## Phase Plan

### Phase 0: contract freeze and audit

- [x] Freeze the canonical `Core17` field list and order.
- [x] Freeze the canonical `Trade22` field list and order.
- [x] Build a route-by-route support table: native, derivable, missing, zero-filled.
- [x] Map each field to its current computation source or placeholder source.
- [x] Decide whether artifact readers should version on surface name, schema version, or both.

Exit criteria:

- one canonical field order is documented
- every optimization route is classified against `Core17` and `Trade22`

Field/source map summary:

- `pnl`, `trades`, `max_dd`, `exposure`, and `net_exposure` come from the realized strategy evaluators on each route family (`double_ma`, shared composite CPU evaluators, or the standalone route-local evaluators).
- `mean` and `std` in the canonical row are projected centrally from saved `Metrics` as `sharpe * volatility` and `volatility` in `src-tauri/src/metric_surface.rs`.
- `downside_deviation`, `ulcer_index`, `omega`, `downside_frequency`, `downside_severity`, `long_mean_return`, `short_mean_return`, `kappa_tau`, `underwater_fraction`, and `max_underwater_duration_bars` now come from real risk-metric computation on the audited route families rather than placeholders.
- `closed_trades`, `winning_trades`, `losing_trades`, `gross_profit`, and `gross_loss` come from truthful trade-summary sources only: exact-kernel 22-row outputs, dynamic generic evaluators, `CompositeCpu` bounded trade-tail enrichment, standalone bounded trade-tail enrichment, and bounded `CompositeMo` Pareto-point trade-tail enrichment.
- Placeholder-driven surfaces now remain only in stale legacy artifacts without runtime-authored hints; newly saved audited optimization artifacts no longer depend on zero-filled trade or risk fields to claim `Core17` / `Trade22`.

### Phase 1: generic artifact surface

- [x] Add a strategy-agnostic artifact field for metric surface metadata.
- [x] Add centralized helpers to project `Metrics` into canonical `Core17` and `Trade22` rows.
- [x] Normalize compare/export/meta readers to prefer the new surface metadata when present.
- [x] Keep legacy artifacts readable without migration.

Exit criteria:

- new artifacts can declare which canonical metric surface they contain
- downstream readers no longer depend on ad hoc row-shape assumptions

### Phase 2: universal `Core17` correctness

- [x] Audit every optimizing strategy path for fields that are still zero-filled or omitted.
- [x] Implement missing `Core17` computations route by route, starting with standalone and composite GPU/CPU paths.
- [x] Add route-local tests proving the non-zero-capable fields are computed from actual path state rather than placeholders.
- [x] Update capability summaries so product code can detect `Core17` availability directly.

Current note:

- `CompositeCpu` is the first non-Double-MA route family with real Phase 2 computation work landed
- the shared `ta_strategies::composite_cpu` metrics path now computes the Core17-only risk fields instead of zero-filling them
- the base `CompositeCpu` optimization result builder now self-declares `metric_surface_kind=core17` instead of relying on downstream inference
- the workflow gate passed after that change, so the broader research flow did not regress
- standalone CPU-route artifacts now up-level to `Core17` when the realized optimization route is CPU-backed, and route-local `ASR` / `VCC` / `VEMR` / `ROCB` / `VCET` optimization tests now prove those richer fields can survive the composite optimizer path on non-degenerate fixtures
- the standalone `ASR`, `VCC`, `VEMR`, `ROCB`, `VCET`, `KQSR`, `CARM`, `MKCS`, `KVDF`, and `CDMC` `GpuKernel` paths now emit truthful Core17-only fields (`downside_frequency`, `downside_severity`, `long_mean_return`, `short_mean_return`, `kappa_tau`, `underwater_fraction`, `max_underwater_duration_bars`) from real CUDA path state, and their direct CPU-vs-GPU parity tests now check those fields
- newly widened standalone GPU artifacts promote through an explicit runtime-authored `metric_surface_kind` marker in `run.res`, so old saved GPU artifacts without that marker remain on `Legacy`
- no additional standalone non-Double-MA `GpuKernel` routes remain pending in the current audited set; `DynamicDoubleMa` plain CPU fallback now also computes truthful Core17-only fields instead of zero-filling them, and request-level tests now cover explicit non-exact `Full17` / `Trade22` requests on that audited shape
- dynamic `strategy_ir` CPU fallback now supports composite/profile plans as well as plain `DoubleMa`, and request-level tests cover explicit `Trade22` / `Full17` requests across the audited dynamic CPU profile set
- `CompositeMo` Pareto artifacts now participate in the canonical surface through runtime-authored `trade22` hints and canonical Pareto-row projection, so the remaining Phase 2 gaps now sit primarily in the separate `MlSignal` workflow and in conservative stale-artifact inference where no runtime-authored surface hint exists
- audited Double-MA-family routes now self-declare their canonical metric surface at result-build time when the realized route is known to have a truthful full-row or trade-tail surface, which removes downstream ambiguity for new artifacts without broadening `RegimeHybrid`, generic `DynamicDoubleMa`, or reduced-row exact-kernel artifacts beyond the currently audited scope
- stale exact-`GpuKernel` artifacts now re-derive `Core17` / `Trade22` only from explicit runtime hints, exact-contract requests, or persisted `gpu_kernel.metrics_count` route diagnostics; reduced-row exact artifacts without that evidence now stay on `Legacy` instead of being upgraded by strategy kind alone
- explicit exact-compatible `DynamicDoubleMa` and `RegimeHybrid` `Full17` / `Trade22` requests now self-declare their surface in `run.res.metric_surface_kind`, and plain `DynamicDoubleMa` CPU fallback requests do the same on audited `DoubleMa`-profile shapes, so new artifacts on those shapes no longer depend on downstream inference
- frontend capability summaries now expose a conservative `supportsCore17MetricSurface` flag, a dedicated `supportsMetricContract` flag, and `DynamicDoubleMa` request shaping now permits explicit metric-surface requests while leaving exact-kernel shortlist/slice controls route-gated

Exit criteria:

- every strategy that supports optimization can emit a correct `Core17` surface
- tests exist for each route family that claims `Core17`

### Phase 3: `Trade22` correctness expansion

- [x] Identify which strategies already have correct trade-level reconstruction and summary data.
- [x] Implement missing trade-summary computations where practical.
- [x] Keep `Trade22` disabled for any route that still relies on placeholders.
- [x] Add strict tests for `closed_trades`, `winning_trades`, `losing_trades`, `gross_profit`, and `gross_loss`.

Exit criteria:

- every route that advertises `Trade22` computes all five trade-extension fields correctly
- unsupported routes remain explicitly out of scope rather than silently degraded

Current note:

- `Trade22` eligibility is now gated separately from generic `Core17` eligibility in `metric_surface.rs`, so unsupported standalone CPU routes, dynamic composite profiles, and unaudited composite GPU routes stay out of `Trade22` even when a request artifact carries `include_trade_metrics` or `metric_contract=trade_22`
- `CompositeCpu` now answers explicit `Trade22` requests truthfully through bounded CPU-side enrichment, standalone non-Double-MA `GpuKernel` routes do the same through host-side tail completion after GPU search, and audited non-exact Double-MA-family CPU / hybrid routes now answer explicit `Trade22` requests through bounded host-side recompute of returned rows
- plain `DynamicDoubleMa` CPU fallback surfaces and exact-compatible `dynamic_strategy_v1` shapes now answer explicit `Trade22` requests on audited `DoubleMa`-profile shapes
- audited dynamic `strategy_ir` CPU fallback now also answers explicit `Trade22` requests on composite/profile plans through bounded trade-tail enrichment layered on top of the existing dynamic metric path
- `CompositeMo` now emits truthful `Trade22` Pareto-point metrics through bounded post-search enrichment, so the remaining Phase 3 nuance is no longer static strategy coverage; it is the explicit decision to keep `MlSignal` out of the optimizer metric-contract surface and to preserve conservative stale-artifact inference

### Phase 4: product exposure

- [x] Decide whether the user-facing control becomes a universal selector or remains strategy-specific.
- [x] Update frontend capability summaries and request builders to use the generic surface capability flags.
- [x] Update saved-run compare/export flows to surface the canonical metric surface cleanly.
- [x] Document how validation, meta, and optional ML should consume canonical surfaces.

Exit criteria:

- product exposure follows real route capability rather than family-specific hardcoding
- users can tell which strategies support which metric surface

Current note:

- frontend capability summaries now expose a dedicated `supportsMetricContract` flag instead of forcing the metric-contract control to piggyback on the broader `supportsAdvancedExactKernelControls` bucket
- the Advanced workspace panel now keys the explicit metric-contract selector off that dedicated capability flag, which keeps current behavior intact while letting future routes graduate `metricContract` separately from shortlist/slice/export controls
- standalone strategy routes now advertise metric-contract support separately from the exact-only shortlist/slice controls, so requested standalone `GpuKernel` previews can expose `Full17` / `Trade22` without inheriting the Double-MA advanced-control surface
- `CompositeCpu` and the audited non-exact Double-MA-family routes now expose explicit metric-contract support in the main product capability layer, while advanced shortlist/slice/export controls remain route-gated
- `DynamicDoubleMa` request shaping now allows explicit `include_trade_metrics` / `metric_contract` requests off the exact-kernel path, and the audited CPU fallback surface now includes constrained composite/profile plans as well as plain `DoubleMa`
- `CompositeMo` now advertises canonical metric-surface availability in the capability layer without exposing a generic metric-contract selector; its saved Pareto artifacts surface `Trade22`, but the product UI still treats explicit contract selection as strategy-specific rather than universal
- canonical-surface consumption policy is now:
  - validation should treat canonical `Core17` / `Trade22` artifact rows as summary/provenance data and still rerun candidates from params rather than trusting saved metrics as a substitute for validation execution
  - meta should prefer uniform `Core17` / `Trade22` corpora, warn on mixed or inferred legacy corpora, and avoid assuming trade-tail fields exist on stale legacy artifacts
  - optional ML should treat `Core17` as the minimum normalized optimizer surface and treat `Trade22` as additive feature richness rather than a hard requirement outside explicitly audited workflows

## Testing Checklist

- [x] Add unit tests for centralized `Core17` / `Trade22` row projection.
- [x] Add route-level tests for at least one standalone strategy currently zero-filling fields.
- [x] Add route-level tests for at least one composite route.
- [x] Add artifact round-trip tests covering legacy and new metric-surface metadata.
- [x] Add compare/export tests proving the canonical row order is stable.
- [x] Add negative tests proving routes that lack trade-summary correctness do not claim `Trade22`.

Current note:

- route-level `Trade22` verification now exists for `CompositeCpu`, standalone CPU routes, standalone requested-`GpuKernel` routes, and audited non-exact Double-MA-family routes such as plain CPU `DoubleMa` and CPU `RegimeHybrid`

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
