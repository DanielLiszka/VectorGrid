# Metric Contract Support Matrix

Date: 2026-04-02

## Purpose

This document answers a narrow product question:

- which strategies currently support the explicit 17-row / 22-row Double-MA metric-contract override
- under what route conditions that support is available
- where the current product-layer answer differs from the lower-level backend

## Executive Summary

The explicit 17-row / 22-row switch is no longer exact-Double-MA-only, but it is still not universal.

- The contract itself is defined as `DoubleMaMetricContract::{Full17, Trade22}` in `crates/app_core/src/double_ma.rs`.
- Product-facing support is now available on audited non-exact Double-MA-family routes, standalone CPU / requested-`GpuKernel` routes, and `CompositeCpu`.
- Advanced exact-kernel shortlist, slice, and export controls are still a separate capability bucket. Metric-contract support no longer implies those exact-only controls.
- The remaining product gaps for the explicit override surface are `CompositeMo`, `MlSignal`, and conservative stale-artifact inference for older runs without runtime-authored surface hints.
- This matrix tracks explicit metric-contract override support only. Separate artifact-surface classification for stale legacy artifacts is intentionally more conservative.

## Matrix

| Strategy | Product support for explicit `full_17` / `trade_22` | Route condition | Notes |
| --- | --- | --- | --- |
| `DoubleMa` | Yes | Current audited CPU / GPU routes | Canonical implementation path. |
| `DynamicDoubleMa` | Conditional | Exact-compatible `dynamic_strategy_v1` `GpuKernel` shapes, plus audited dynamic CPU fallback surfaces (plain `DoubleMa` and constrained composite/profile plans) | Exact-only shortlist/slice/export controls remain tied to the exact dynamic `GpuKernel` path. |
| `Pit` | Yes | Current audited CPU / GPU routes | Uses the Double-MA optimizer family path. |
| `Set` | Yes | Current audited CPU / GPU routes | Uses the Double-MA optimizer family path. |
| `Mts` | Yes | Current audited CPU / GPU routes | Uses the Double-MA optimizer family path. |
| `Orc` | Yes | Current audited CPU / GPU routes | Uses the Double-MA optimizer family path. |
| `Marrt` | Yes | Current audited CPU / GPU routes | Uses the Double-MA optimizer family path. |
| `Vfct` | Yes | Current audited CPU / GPU routes | Uses the Double-MA optimizer family path. |
| `CompositeTrend` | Conditional | Current audited non-dynamic routes (`GpuKernel`, `GpuHybridKernel`, `CpuOnly`) | Explicit metric-contract support is available broadly; advanced exact-kernel shortlist/slice/export controls still require the exact `GpuKernel` route. |
| `RegimeHybrid` | Conditional | Current audited non-dynamic routes (`GpuKernel` where exact-compatible, `GpuHybridKernel`, `CpuOnly`) | Product layer now exposes explicit metric contracts on the audited non-exact route family as well. |
| `BreakoutMomentum` | Conditional | Current audited non-dynamic routes (`GpuKernel`, `GpuHybridKernel`, `CpuOnly`) | Explicit metric-contract support is available broadly; advanced exact-kernel shortlist/slice/export controls still require the exact `GpuKernel` route. |
| `Asr` | Conditional | CPU-backed route or requested standalone `GpuKernel` preview | Explicit metric-contract controls are now available. `Trade22` on standalone `GpuKernel` uses bounded host-side enrichment after GPU search. |
| `Vcc` | Conditional | CPU-backed route or requested standalone `GpuKernel` preview | Explicit metric-contract controls are now available. |
| `Vemr` | Conditional | CPU-backed route or requested standalone `GpuKernel` preview | Explicit metric-contract controls are now available. |
| `Rocb` | Conditional | CPU-backed route or requested standalone `GpuKernel` preview | Explicit metric-contract controls are now available. |
| `Vcet` | Conditional | CPU-backed route or requested standalone `GpuKernel` preview | Explicit metric-contract controls are now available. |
| `Kqsr` | Conditional | CPU-backed route or requested standalone `GpuKernel` preview | Explicit metric-contract controls are now available. |
| `Carm` | Conditional | CPU-backed route or requested standalone `GpuKernel` preview | Explicit metric-contract controls are now available. |
| `Kvdf` | Conditional | CPU-backed route or requested standalone `GpuKernel` preview | Explicit metric-contract controls are now available. |
| `Mkcs` | Conditional | CPU-backed route or requested standalone `GpuKernel` preview | Explicit metric-contract controls are now available. |
| `Cdmc` | Conditional | CPU-backed route or requested standalone `GpuKernel` preview | Explicit metric-contract controls are now available. |
| `CompositeCpu` | Yes | `CompositeCpu` route only | Separate request type and optimizer path, but explicit `Full17` / `Trade22` requests are now supported. |
| `CompositeMo` | No | N/A | Separate request type and optimizer path. Saved Pareto artifacts now emit a canonical `Trade22` surface by default, but there is still no explicit metric-contract override on this workflow. |
| `MlSignal` | No | N/A | Separate workflow; does not use optimizer metric-contract controls. |

## Why These Rows Resolve This Way

### Double-MA family

The product capability layer classifies this set as the Double-MA family:

- `DoubleMa`
- `DynamicDoubleMa`
- `Pit`
- `Set`
- `Mts`
- `Orc`
- `Marrt`
- `Vfct`
- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`

That family is defined in `web/src/utils/strategyOptimizationCapabilities.ts`, and the same set is routed through the auto Double-MA request builder in `web/src/services/optimizeService.ts`.

For this family, explicit metric contracts are no longer exact-route-only:

- `metricContract` is normalized in `web/src/services/optimizeRequestBuilder.ts`
- the request builder now allows explicit metric contracts across the audited Double-MA-family routes
- the backend mirrors that broader audited support, while still keeping shortlist/slice/export controls exact-route-gated

### Composite profile nuances

The current composite-profile route policy is:

- `CompositeTrend`: `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on, `CpuOnly` fallback
- `BreakoutMomentum`: `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on, `CpuOnly` fallback
- `RegimeHybrid`: `GpuHybridKernel` in the current form, `CpuOnly` fallback

Because product support for explicit metric contracts now follows the audited route family rather than only the exact-kernel preview, this means:

- `CompositeTrend`: explicit `Full17` / `Trade22` support on the current audited CPU / hybrid / exact-kernel routes
- `BreakoutMomentum`: explicit `Full17` / `Trade22` support on the current audited CPU / hybrid / exact-kernel routes
- `RegimeHybrid`: explicit `Full17` / `Trade22` support on the current audited CPU / hybrid / exact-compatible kernel routes

### Standalone strategy family

The standalone family:

- `Asr`
- `Vcc`
- `Vemr`
- `Rocb`
- `Vcet`
- `Kqsr`
- `Carm`
- `Kvdf`
- `Mkcs`
- `Cdmc`

does not expose `supportsAdvancedExactKernelControls`, even when the requested backend is GPU-capable. That no longer blocks metric-contract support. These strategies now expose explicit `Full17` / `Trade22` requests, but they still do not inherit the Double-MA exact-only shortlist/slice/export surface.

### Separate workflows

The following strategies do not expose the explicit Double-MA metric-contract override surface:

- `CompositeMo`
- `MlSignal`

`CompositeCpu` is no longer part of this exclusion. It remains a separate request type, but it now carries explicit metric-contract support in its own route/capability layer. `CompositeMo` also no longer sits outside canonical artifact surfaces entirely; it now emits canonical `Trade22` Pareto-point surfaces by default while keeping override controls out of the UI/request shape.

## Backend Nuance

The current product-layer answer and the backend are much closer than they were at the start of this work.

The remaining nuance is no longer `RegimeHybrid`. The remaining nuance is mostly artifact history: new dynamic `strategy_ir` CPU fallback and exact-compatible `dynamic_strategy_v1` shapes support explicit metric surfaces today, but stale artifacts without runtime-authored surface hints are still inferred conservatively.

The practical reading is:

- current product/UI support matrix: broad audited support across the static strategy families, standalone routes, `CompositeCpu`, and dynamic CPU fallback / exact dynamic `GpuKernel`; `CompositeMo` now emits canonical surfaces by default but still does not expose explicit override controls
- lower-level backend capability: mostly aligned with product exposure, with the remaining nuance concentrated in conservative legacy-artifact inference

## Source Anchors

- Metric-contract enum and request field:
  - `crates/app_core/src/double_ma.rs`
- Frontend metric-contract normalization and request emission:
  - `web/src/services/optimizeRequestBuilder.ts`
- Double-MA-family and product capability flags:
  - `web/src/utils/strategyOptimizationCapabilities.ts`
- Double-MA-family routing in the optimize service:
  - `web/src/services/optimizeService.ts`
- Composite profile route policy:
  - `web/src/strategies/compositeProfileGpuPolicy.ts`
- Product tests covering route-gated advanced controls:
  - `web/src/utils/strategyOptimizationCapabilities.test.ts`
  - `web/src/components/AdvancedWorkspacePanel.test.ts`
- Separate request types that do not currently expose the explicit `metric_contract` field:
  - `crates/app_core/src/composite_mo.rs`
