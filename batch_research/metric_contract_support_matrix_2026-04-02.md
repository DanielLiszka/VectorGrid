# Metric Contract Support Matrix

Date: 2026-04-02

## Purpose

This document answers a narrow product question:

- which strategies currently support the explicit 17-row / 22-row Double-MA metric-contract override
- under what route conditions that support is available
- where the current product-layer answer differs from the lower-level backend

## Executive Summary

The explicit 17-row / 22-row switch is a Double-MA-family feature, not a universal strategy feature.

- The contract itself is defined as `DoubleMaMetricContract::{Full17, Trade22}` in `crates/app_core/src/double_ma.rs`.
- The frontend only serializes `metric_contract` on the Double-MA request path in `web/src/services/optimizeRequestBuilder.ts`.
- Product-facing support is available for the Double-MA family when the request resolves to the exact `GpuKernel` route.
- Product-facing support is not available for the standalone composite/advanced strategy family, `CompositeCpu`, `CompositeMo`, or `MlSignal`.
- `RegimeHybrid` is the main nuance: the current product capability layer says "no" because it resolves to `GpuHybridKernel`, but a backend test shows the engine can exercise `Trade22` for that profile in a direct backend path.

## Matrix

| Strategy | Product support for explicit `full_17` / `trade_22` | Route condition | Notes |
| --- | --- | --- | --- |
| `DoubleMa` | Yes | Exact `GpuKernel` route only | Canonical implementation path. |
| `DynamicDoubleMa` | Conditional | Requested `GpuKernel`, and only for exact-compatible `dynamic_strategy_v1` shapes | Product layer exposes it only on the exact dynamic kernel path. |
| `Pit` | Yes | Exact `GpuKernel` route only | Uses the Double-MA optimizer family path. |
| `Set` | Yes | Exact `GpuKernel` route only | Uses the Double-MA optimizer family path. |
| `Mts` | Yes | Exact `GpuKernel` route only | Uses the Double-MA optimizer family path. |
| `Orc` | Yes | Exact `GpuKernel` route only | Uses the Double-MA optimizer family path. |
| `Marrt` | Yes | Exact `GpuKernel` route only | Uses the Double-MA optimizer family path. |
| `Vfct` | Yes | Exact `GpuKernel` route only | Uses the Double-MA optimizer family path. |
| `CompositeTrend` | Conditional | Supported when the resolved route is `GpuKernel`; not supported on `GpuHybridKernel` or `CpuOnly` | Current policy is `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on. |
| `RegimeHybrid` | No in current product layer | Current route preview resolves to `GpuHybridKernel` or `CpuOnly`, not exact `GpuKernel` | Backend has broader latent support than the product layer currently exposes. |
| `BreakoutMomentum` | Conditional | Supported when the resolved route is `GpuKernel`; not supported on `GpuHybridKernel` or `CpuOnly` | Current policy is `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on. |
| `Asr` | No | N/A | Supports some shortlist features on standalone GPU previews, but not metric-contract controls. |
| `Vcc` | No | N/A | Same limitation as `Asr`. |
| `Vemr` | No | N/A | Same limitation as `Asr`. |
| `Rocb` | No | N/A | Same limitation as `Asr`. |
| `Vcet` | No | N/A | Same limitation as `Asr`. |
| `Kqsr` | No | N/A | Same limitation as `Asr`. |
| `Carm` | No | N/A | Same limitation as `Asr`. |
| `Kvdf` | No | N/A | Same limitation as `Asr`. |
| `Mkcs` | No | N/A | Same limitation as `Asr`. |
| `Cdmc` | No | N/A | Same limitation as `Asr`. |
| `CompositeCpu` | No | N/A | Separate request type and optimizer path. |
| `CompositeMo` | No | N/A | Separate request type and optimizer path. |
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

For this family, explicit metric contracts are still route-gated:

- `metricContract` is normalized in `web/src/services/optimizeRequestBuilder.ts`
- the request builder rejects explicit metric contracts unless the route is the exact `GpuKernel` route
- the backend repeats that guard in `crates/app_core/src/double_ma.rs`

### Composite profile nuances

The current composite-profile route policy is:

- `CompositeTrend`: `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on, `CpuOnly` fallback
- `BreakoutMomentum`: `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on, `CpuOnly` fallback
- `RegimeHybrid`: `GpuHybridKernel` in the current form, `CpuOnly` fallback

Because product support for explicit metric contracts follows the exact-kernel route preview, this means:

- `CompositeTrend`: yes only on the non-ADX kernel path
- `BreakoutMomentum`: yes only on the non-ADX kernel path
- `RegimeHybrid`: no in the current product layer

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

does not expose `supportsAdvancedExactKernelControls`, even when the requested backend is GPU-capable. Those strategies can expose shortlist extras on requested standalone GPU previews, but not the 17-row / 22-row metric-contract override.

### Separate workflows

The following strategies do not use the Double-MA metric-contract surface at all:

- `CompositeCpu`
- `CompositeMo`
- `MlSignal`

Their request types and capability summaries do not include `metric_contract`.

## Backend Nuance

The current product-layer answer for `RegimeHybrid` is "no", but that is not the whole backend story.

A direct backend test in `crates/app_core/src/double_ma.rs` exercises `RegimeHybrid` with `Some(DoubleMaMetricContract::Trade22)`. That means the engine has at least some direct support beyond what the current frontend capability layer advertises.

The practical reading is:

- current product/UI support matrix: `RegimeHybrid = no`
- lower-level backend capability: broader than the current product exposure

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
- Separate request types with no `metric_contract` field:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_mo.rs`
