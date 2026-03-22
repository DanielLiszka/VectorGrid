
# Composite route policy and request-shaping optimization plan

## Scope

This plan is scoped to the review-map surface **“## 2. Composite route policy and request shaping”** and the attached files:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `crates/app_core/src/composite_profile_route.rs`

It stays within the review-map posture: optimization-first, parity-preserving, and benchmark-aware. It does **not** propose new strategies or broad UI expansion.

## Goals

1. Eliminate unnecessary request-shaping allocations and repeated parsing.
2. Remove frontend/backend route-policy drift for `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`.
3. Make guardrails depend on the **effective** route, not just the requested backend string.
4. Shrink frontend warm-path work before the request ever reaches Rust.
5. Improve route-summary fidelity so benchmarking and user-facing summaries can describe the real route.
6. Add regression coverage for the exact route/guardrail combinations most likely to drift.

## Current implementation map

### 1) `composite_profile_route.rs`

Current Rust policy is simple and cheap:

- `composite_profile_from_strategy` derives profile identity from `StrategyConfig`.
- `composite_profile_gpu_target` maps:
  - `CompositeTrend` -> `GpuKernel` unless ADX is on
  - `BreakoutMomentum` -> `GpuKernel` unless ADX is on
  - `RegimeHybrid` -> `GpuHybridKernel`
  - fallback -> `CpuOnly`
- `composite_profile_runtime_label` still returns labels like `"CompositeTrendCpu"` even when the GPU target is `GpuKernel` or `GpuHybridKernel`.

This file is already allocation-free. The main issue is **label fidelity**, not raw throughput.

### 2) `compositeProfileGpuPolicy.ts`

This file encodes the same ADX-sensitive route policy in TypeScript:

- `CompositeTrend`: ADX off -> `GpuKernel`, ADX on -> `GpuHybridKernel`
- `BreakoutMomentum`: ADX off -> `GpuKernel`, ADX on -> `GpuHybridKernel`
- `RegimeHybrid`: always `GpuHybridKernel`

That is the right place to centralize frontend route preview logic, but today it only returns a GPU target and a static summary string. It does not model requested backend vs effective backend.

### 3) `strategyOptimizationCapabilities.ts`

Capabilities are currently **kind-only**. For composite kinds, capability exposure is static:

- `supportsExportAll: true`
- `supportsHeatmap: true`
- `supportsTopK: true`
- `supportsIncludeAll: true`

That means the capability layer cannot represent the actual route for:

- `CompositeTrend` with ADX on
- `BreakoutMomentum` with ADX on
- `RegimeHybrid` always

The most important drift is `supportsExportAll`: the file treats composite kinds as uniformly export-capable, but the request builder only supports `exportAllCsvPath` on the GPU-kernel path.

### 4) `optimizeRequestBuilder.ts`

This file contains the biggest performance and correctness surface.

#### Hot-path issues already visible in the current code

1. **Array expansion just to count pairs**
   - `buildDoubleMaOptimizationRequest` expands fast and slow ranges, then counts `fast < slow`.
   - `buildCompositeCpuOptimizationRequest` does the same.
   - `buildCompositeMoOptimizationRequest` does the same.

   That means extra arrays, extra iteration, and more GC pressure just to compute a pair count.

2. **Full MA support map copy**
   - `normalizeMovingAverageSupportMap` clones the entire support map into a new object.
   - Each builder only needs two lookups: fast MA and slow MA.
   - That is unnecessary work and an avoidable memory copy.

3. **Repeated parsing of the same inputs**
   - `buildDoubleMaOptimizationRequest` parses costs, cooldown, persistence, exit mode, starting equity, etc.
   - `buildStrategyConfigFromInputs` parses many of the same fields again.
   - `buildStrategyConfigFromInputs` also parses many fields for strategies that are not active on the current request.

4. **Guardrails use the requested backend string instead of the effective route**
   - Kernel-only restrictions are checked using `backend === 'GpuKernel'`.
   - `exportAllCsvPath` also checks the requested backend string rather than the resolved route.
   - This creates drift for composite profiles where `GpuKernel` may resolve to `GpuHybridKernel`, and for `Auto`, which may still serialize to a GPU-family route.

5. **Unknown backend silently falls back to CPU**
   - `normalizeBackend` maps unknown values to `CpuOnly`.
   - That creates a silent performance cliff and hides stale vocabulary bugs.

6. **Static numeric guardrails are magic numbers**
   - `heatmapBins <= 512`
   - `includeAll` combinations <= `2_000_000`
   - No named constants, no inline rationale, and no frontend preflight for known kernel limits like `top_k <= 256` when `include_all=false` on the kernel path.

### 5) `optimizeRequestBuilder.test.ts`

The test file already covers:

- composite ADX route selection
- explicit `CpuOnly` fallback
- exact-grid forcing for heatmap/include-all
- legacy `GpuHybridKernel` normalization
- some Double-MA mode normalization

The missing coverage is exactly where drift is likely:

- `Auto` + composite effective route
- `exportAllCsvPath` under effective `GpuKernel` vs effective `GpuHybridKernel`
- requested `GpuKernel` resolving to hybrid and incorrectly tripping kernel-only restrictions
- route-aware capability exposure
- unknown backend vocabulary rejection
- known backend-limit preflight such as `top_k > 256` on kernel path

---
## Current hotspot anchors (for implementation)

These are the highest-value current locations to edit first.

| File | Current location | Why it matters |
| --- | --- | --- |
| `optimizeRequestBuilder.ts` | `normalizeBackend` (~496-503) | Unknown/stale backend vocabulary silently becomes `CpuOnly`. |
| `optimizeRequestBuilder.ts` | `resolveDoubleMaBackendPayload` (~520-535) | Effective route is resolved late and only after the strategy object is built. |
| `optimizeRequestBuilder.ts` | `buildDoubleMaOptimizationRequest` (~1116-1118) | Fast/slow ranges are expanded into arrays just to count pairs. |
| `optimizeRequestBuilder.ts` | `buildDoubleMaOptimizationRequest` (~1266-1283) | Kernel-only restrictions and export-all rules use the requested backend string, not the effective route. |
| `optimizeRequestBuilder.ts` | `buildStrategyConfigFromInputs` (~613-834) | Common fields and many inactive strategy-specific fields are reparsed on every call. |
| `optimizeRequestBuilder.ts` | `buildCompositeCpuOptimizationRequest` (~1387-1390) | Same range-array allocation pattern appears again. |
| `optimizeRequestBuilder.ts` | `buildCompositeMoOptimizationRequest` (~1869-1871, ~1941-1951) | Same range-array allocation pattern, plus fast/slow domain metadata is recomputed a second time. |
| `strategyOptimizationCapabilities.ts` | composite branch (~64-117) | Capability exposure is kind-only and cannot represent ADX-sensitive effective routes. |
| `compositeProfileGpuPolicy.ts` | entire file (~1-37) | Correct route policy exists here, but it is too small to act as the shared route-preview authority. |
| `composite_profile_route.rs` | `composite_profile_runtime_label` (~24-31) | Labels describe profile identity poorly once GPU execution enters the picture. |

---


## Priority order

### P0 – must do first

1. **Introduce a single effective-route preview in the frontend**
2. **Replace pair-count array expansion with arithmetic counting**
3. **Stop cloning the MA support map**
4. **Split `buildStrategyConfigFromInputs` into parse-once common state + kind-specific state**
5. **Make capabilities route-aware for the composite profiles**

### P1 – do immediately after P0

6. Add named guardrail constants and route-aware guardrail helpers
7. Reject unknown backend vocabulary instead of silently forcing CPU
8. Split Rust profile labels from execution labels
9. Add table-driven parity tests across the route matrix

### P2 – only after benchmarks

10. Consider further micro-optimizations only if app-path measurements still show frontend request shaping as material
11. Do **not** add unsafe/uninitialized memory in these files unless a benchmark proves the need

---

## Proposed target architecture

## A. Central frontend route preview

Make `compositeProfileGpuPolicy.ts` the canonical source for composite profile route resolution. Extend it from “GPU target only” to a proper route preview.

### Proposed API

```ts
export type CompositeProfileRequestedBackend = 'Auto' | 'CpuOnly' | 'GpuKernel'

export interface CompositeProfileGpuInputs {
  trendUseAdx?: boolean
  breakoutUseAdx?: boolean
}

export interface CompositeProfileRoutePreview {
  kind: CompositeProfileKind
  requestedBackend: CompositeProfileRequestedBackend
  effectiveBackend: CompositeProfileGpuTarget
  usesHybrid: boolean
  supportsExportAll: boolean
  supportsHeatmap: boolean
  supportsTopK: boolean
  supportsIncludeAll: boolean
}

export function normalizeCompositeRequestedBackend(
  value: unknown,
): CompositeProfileRequestedBackend

export function resolveCompositeProfileRoute(
  kind: CompositeProfileKind,
  requestedBackend: CompositeProfileRequestedBackend,
  inputs: CompositeProfileGpuInputs = {},
): CompositeProfileRoutePreview
```

### Why this matters

- One canonical place for ADX-sensitive composite routing.
- `strategyOptimizationCapabilities.ts` can read the same logic as the request builder.
- The request builder can decide `exportAllCsvPath`, kernel-only restrictions, and summaries from the same effective route.

### Concrete edit for `compositeProfileGpuPolicy.ts`

```ts
export type CompositeProfileRequestedBackend = 'Auto' | 'CpuOnly' | 'GpuKernel'

export interface CompositeProfileRoutePreview {
  kind: CompositeProfileKind
  requestedBackend: CompositeProfileRequestedBackend
  effectiveBackend: CompositeProfileGpuTarget
  usesHybrid: boolean
  supportsExportAll: boolean
  supportsHeatmap: boolean
  supportsTopK: boolean
  supportsIncludeAll: boolean
}

export function normalizeCompositeRequestedBackend(
  value: unknown,
): CompositeProfileRequestedBackend {
  const text = String(value ?? '').trim()
  if (!text) return 'CpuOnly'
  if (text === 'CpuOnly') return 'CpuOnly'
  if (text === 'Auto') return 'Auto'
  if (text === 'GpuOnly' || text === 'GpuKernel' || text === 'GpuHybridKernel') {
    return 'GpuKernel'
  }
  throw new Error(`Unknown backend '${text}'.`)
}

export function resolveCompositeProfileRoute(
  kind: CompositeProfileKind,
  requestedBackend: CompositeProfileRequestedBackend,
  inputs: CompositeProfileGpuInputs = {},
): CompositeProfileRoutePreview {
  if (requestedBackend === 'CpuOnly') {
    return {
      kind,
      requestedBackend,
      effectiveBackend: 'CpuOnly',
      usesHybrid: false,
      supportsExportAll: false,
      supportsHeatmap: false,
      supportsTopK: true,
      supportsIncludeAll: true,
    }
  }

  const effectiveBackend = resolveCompositeProfileGpuTarget(kind, inputs)
  const usesHybrid = effectiveBackend === 'GpuHybridKernel'

  return {
    kind,
    requestedBackend,
    effectiveBackend,
    usesHybrid,
    supportsExportAll: effectiveBackend === 'GpuKernel',
    supportsHeatmap: true,
    supportsTopK: true,
    supportsIncludeAll: true,
  }
}

export function describeCompositeProfileGpuPolicy(
  route: CompositeProfileRoutePreview,
): string {
  if (route.effectiveBackend === 'CpuOnly') {
    return `${route.kind} is pinned to CpuOnly because CPU fallback was explicitly requested.`
  }
  if (route.effectiveBackend === 'GpuHybridKernel') {
    return `${route.kind} resolves to GpuHybridKernel on the current inputs.`
  }
  return `${route.kind} resolves to GpuKernel on the current inputs.`
}
```

### Notes

- Preserve the existing empty/unspecified default if the product today implicitly treats missing backend as CPU. If the intended product default is actually `Auto`, make that change explicitly in the UI and tests rather than sneaking it into the builder refactor.
- The route preview stays internal to frontend shaping. Do **not** add extra serialized fields to the backend request unless the backend is confirmed to ignore unknown fields.

---

## B. Make capabilities input-aware

`strategyOptimizationCapabilities.ts` should stop being kind-only for the composite profiles.

### Current problem

These composite profiles always report:

```ts
supportsExportAll: true
```

That is wrong for:

- `RegimeHybrid`
- `CompositeTrend` with ADX enabled
- `BreakoutMomentum` with ADX enabled
- explicit `CpuOnly` requests

### Recommended edit

Keep the existing public function, but overload it to accept optional route inputs.

```ts
export interface StrategyOptimizationCapabilityInputs {
  kind: unknown
  requestedBackend?: unknown
  trendUseAdx?: unknown
  breakoutUseAdx?: unknown
}

export function describeStrategyOptimizationCapabilities(
  value: unknown,
  routeInputs: Partial<StrategyOptimizationCapabilityInputs> = {},
): StrategyOptimizationCapabilities
```

### Example implementation

```ts
import {
  describeCompositeProfileGpuPolicy,
  normalizeCompositeRequestedBackend,
  resolveCompositeProfileRoute,
} from '../strategies/compositeProfileGpuPolicy'

export function describeStrategyOptimizationCapabilities(
  value: unknown,
  routeInputs: Partial<StrategyOptimizationCapabilityInputs> = {},
): StrategyOptimizationCapabilities {
  const kind = normalizeKind(value)

  if (
    kind === 'CompositeTrend' ||
    kind === 'RegimeHybrid' ||
    kind === 'BreakoutMomentum'
  ) {
    const route = resolveCompositeProfileRoute(
      kind,
      normalizeCompositeRequestedBackend(routeInputs.requestedBackend),
      {
        trendUseAdx: !!routeInputs.trendUseAdx,
        breakoutUseAdx: !!routeInputs.breakoutUseAdx,
      },
    )

    return {
      kind,
      usesGlobalBackend: true,
      usesGlobalMode: true,
      supportsHeatmap: route.supportsHeatmap,
      supportsExportAll: route.supportsExportAll,
      supportsTopK: route.supportsTopK,
      supportsIncludeAll: route.supportsIncludeAll,
      executionSummary: describeCompositeProfileGpuPolicy(route),
      modeSummary: `${kind} uses the Double-MA mode selector. Exact-grid outputs still require Grid.`,
      outputSummary:
        route.supportsExportAll
          ? `${kind} can use Top-K, heatmap, include-all, and export-all on the current route.`
          : `${kind} can use Top-K, heatmap, and include-all on the current route, but export-all is disabled because the effective route is not GpuKernel.`,
    }
  }

  // existing branches unchanged...
}
```

### Performance impact

The raw CPU cost here is small. The bigger gain is indirect:

- fewer invalid requests emitted by the UI
- fewer user retries
- less route-policy drift to debug later
- better support-matrix fidelity

---

## C. Refactor `optimizeRequestBuilder.ts`

This is the biggest patch.

## C1. Replace range expansion with arithmetic pair counting

### Current anti-pattern

Current code expands arrays:

```ts
const fastPeriods = expandRangeU16Like(fast[0], fast[1], fast[2])
const slowPeriods = expandRangeU16Like(slow[0], slow[1], slow[2])
const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods)
```

That allocates memory and copies values only to compute a count.

### Replacement

Add a parsed range helper and an arithmetic pair counter.

```ts
interface ParsedU16Range {
  raw: [number, number, number]
  low: number
  high: number
  effectiveStep: number
  count: number
}

function parseU16Range(
  startRaw: unknown,
  endRaw: unknown,
  stepRaw: unknown,
  label: string,
): ParsedU16Range {
  const start = Math.floor(Number(startRaw))
  const stepInput = Math.floor(Number(stepRaw))
  const endInput = Math.floor(Number(endRaw))

  if (!Number.isFinite(start) || start < 1 || start > 65535) {
    throw new Error(`${label} start must be in [1, 65535].`)
  }
  if (!Number.isFinite(stepInput) || stepInput < 0 || stepInput > 65535) {
    throw new Error(`${label} step must be in [0, 65535].`)
  }

  const end = stepInput === 0 ? start : endInput
  if (!Number.isFinite(end) || end < start || end > 65535) {
    throw new Error(`${label} end must be in [${start}, 65535].`)
  }

  const effectiveStep = stepInput === 0 ? 1 : stepInput
  const count = Math.floor((end - start) / effectiveStep) + 1

  return {
    raw: [start, end, stepInput],
    low: start,
    high: end,
    effectiveStep,
    count,
  }
}

function firstStrictlyGreaterIndex(range: ParsedU16Range, value: number): number {
  if (value < range.low) return 0
  if (value >= range.high) return range.count
  return Math.floor((value - range.low) / range.effectiveStep) + 1
}

function countFastLtSlowPairs(
  fast: ParsedU16Range,
  slow: ParsedU16Range,
  cap: number = Number.POSITIVE_INFINITY,
): number {
  let total = 0
  for (let i = 0, fastValue = fast.low; i < fast.count; i += 1, fastValue += fast.effectiveStep) {
    const firstAllowedSlow = firstStrictlyGreaterIndex(slow, fastValue)
    const remaining = slow.count - firstAllowedSlow
    if (remaining <= 0) {
      if (fastValue >= slow.high) break
      continue
    }
    total += remaining
    if (total > cap) return total
  }
  return total
}
```

### Why this is better

- O(1) extra memory instead of allocating two arrays.
- No element copies.
- Easy early-out for guardrail caps.
- Reusable across all three builders (`DoubleMa`, `CompositeCpu`, `CompositeMo`).

### Where to apply it

- `buildDoubleMaOptimizationRequest`
- `buildCompositeCpuOptimizationRequest`
- `buildCompositeMoOptimizationRequest`

### Important note

This is the **single best allocation reduction** in the attached frontend files.

---

## C2. Stop cloning the MA support map

### Current anti-pattern

```ts
function normalizeMovingAverageSupportMap(raw: unknown): Record<string, MovingAverageSupportEntry> {
  if (!raw || typeof raw !== 'object') return {}
  const out: Record<string, MovingAverageSupportEntry> = {}
  for (const [key, value] of Object.entries(raw as Record<string, unknown>)) {
    const id = normalizeText(key)
    if (!id || !value || typeof value !== 'object') continue
    out[id] = value as MovingAverageSupportEntry
  }
  return out
}
```

That copies the entire support map every time, even though each request only checks two MA ids.

### Replacement

Use an on-demand lookup with a lazy normalized fallback index.

```ts
interface MovingAverageSupportLookup {
  hasData: boolean
  supportsCpu(id: string): boolean
  supportsCuda(id: string): boolean
  supportsKernel(id: string): boolean
}

const EMPTY_MA_SUPPORT_LOOKUP: MovingAverageSupportLookup = {
  hasData: false,
  supportsCpu: () => false,
  supportsCuda: () => false,
  supportsKernel: () => false,
}

function createMovingAverageSupportLookup(raw: unknown): MovingAverageSupportLookup {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return EMPTY_MA_SUPPORT_LOOKUP

  const source = raw as Record<string, unknown>
  let normalizedIndex: Map<string, MovingAverageSupportEntry> | null = null

  const getEntry = (id: string): MovingAverageSupportEntry | null => {
    const direct = source[id]
    if (direct && typeof direct === 'object' && !Array.isArray(direct)) {
      return direct as MovingAverageSupportEntry
    }

    if (normalizedIndex == null) {
      normalizedIndex = new Map()
      for (const [key, value] of Object.entries(source)) {
        if (!value || typeof value !== 'object' || Array.isArray(value)) continue
        const normalized = normalizeText(key)
        if (normalized) normalizedIndex.set(normalized, value as MovingAverageSupportEntry)
      }
    }

    return normalizedIndex.get(id) ?? null
  }

  return {
    hasData: true,
    supportsCpu(id: string) {
      return !!getEntry(id)?.supports_cpu
    },
    supportsCuda(id: string) {
      return !!getEntry(id)?.supports_cuda
    },
    supportsKernel(id: string) {
      return !!getEntry(id)?.supports_cuda_kernel
    },
  }
}
```

### Why this is better

- Zero full-map copy on the common path.
- Still safe if inbound keys need whitespace normalization.
- Replaces `Object.keys(map).length` and a full clone with two lookup calls.

### Apply to

- `buildDoubleMaOptimizationRequest`
- `buildCompositeCpuOptimizationRequest`
- `buildCompositeMoOptimizationRequest`

---

## C3. Add a strict backend normalizer and effective-route preview

### Current issue

`normalizeBackend` silently returns `CpuOnly` for unknown values.

That is bad for both correctness and performance because stale or mistyped backend vocabulary becomes a hidden CPU downgrade.

### Replacement

Use strict normalization and separate **requested** backend from **effective** backend.

```ts
type RequestedBackend = 'Auto' | 'CpuOnly' | 'GpuKernel'
type EffectiveBackend = 'CpuOnly' | 'GpuKernel' | 'GpuHybridKernel'

function normalizeRequestedBackend(value: unknown): RequestedBackend {
  const backend = normalizeText(value)
  if (!backend) return 'CpuOnly'
  if (backend === 'CpuOnly') return 'CpuOnly'
  if (backend === 'Auto') return 'Auto'
  if (backend === 'GpuOnly' || backend === 'GpuKernel' || backend === 'GpuHybridKernel') {
    return 'GpuKernel'
  }
  throw new Error(`Unknown backend '${backend}'.`)
}
```

Then add a route preview for the Double-MA family.

```ts
interface DoubleMaRoutePreview {
  requestedBackend: RequestedBackend
  effectiveBackend: EffectiveBackend
  backendPayload: Record<string, unknown>
  usesHybrid: boolean
  isGpuFamily: boolean
  isKernelOnlyRoute: boolean
}

function resolveDoubleMaRoutePreview(
  kind: string,
  requestedBackend: RequestedBackend,
  inputs: DoubleMaOptimizationBuildInputs,
): DoubleMaRoutePreview {
  if (requestedBackend === 'CpuOnly') {
    return {
      requestedBackend,
      effectiveBackend: 'CpuOnly',
      backendPayload: { backend: 'CpuOnly' },
      usesHybrid: false,
      isGpuFamily: false,
      isKernelOnlyRoute: false,
    }
  }

  if (
    kind === 'CompositeTrend' ||
    kind === 'RegimeHybrid' ||
    kind === 'BreakoutMomentum'
  ) {
    const compositeRoute = resolveCompositeProfileRoute(
      kind,
      requestedBackend,
      {
        trendUseAdx: normalizeBoolean(inputs.trendUseAdx),
        breakoutUseAdx: normalizeBoolean(inputs.breakoutUseAdx),
      },
    )

    return {
      requestedBackend,
      effectiveBackend: compositeRoute.effectiveBackend,
      backendPayload: {
        backend: compositeRoute.effectiveBackend,
        device_id: 0,
      },
      usesHybrid: compositeRoute.usesHybrid,
      isGpuFamily: compositeRoute.effectiveBackend !== 'CpuOnly',
      isKernelOnlyRoute: compositeRoute.effectiveBackend === 'GpuKernel',
    }
  }

  return {
    requestedBackend,
    effectiveBackend: 'GpuKernel',
    backendPayload: { backend: 'GpuKernel', device_id: 0 },
    usesHybrid: false,
    isGpuFamily: true,
    isKernelOnlyRoute: true,
  }
}
```

### Important behavior change

This does **not** add new serialized request fields. It only changes which route the builder uses internally for guardrails.

---

## C4. Use effective-route guardrails, not requested-backend guardrails

### Current problem

This block currently uses the requested backend string:

```ts
if (backend === 'GpuKernel') {
  if (exitMode !== 'cross') ...
  if (hasExecutionOverlays) ...
}
```

That is wrong for composite requests where:

- requested backend is `GpuKernel`
- effective backend becomes `GpuHybridKernel`

Under the current code, hybrid-routed requests still get kernel-only restrictions.

### Correct replacement

```ts
if (route.isKernelOnlyRoute) {
  if (common.exitMode !== 'cross') {
    throw new Error('GpuKernel currently supports exit mode = cross only.')
  }
  if (common.hasExecutionOverlays) {
    throw new Error('GpuKernel does not support cooldown/persistence overlays.')
  }
}

if (exportAllCsvPath) {
  if (includeAll) {
    throw new Error('exportAllCsvPath requires includeAll=false (stream export avoids huge RAM usage).')
  }
  if (mode !== 'Grid') {
    throw new Error('exportAllCsvPath requires mode=Grid.')
  }
  if (route.effectiveBackend !== 'GpuKernel') {
    throw new Error('exportAllCsvPath is currently supported only on the effective GpuKernel route.')
  }
  if ((fastVariantCount * slowVariantCount) > 1) {
    throw new Error('exportAllCsvPath is not supported when optimizing MA parameters. Disable MA param sweeps or export.')
  }
}
```

### Also add route-aware preflight for known kernel limits

The adjacent GPU kernel path already has a real `top_k` limit for `include_all=false`.

Add a named constant and preflight it only on the effective kernel route:

```ts
const MAX_GPU_KERNEL_TOP_K = 256

if (route.effectiveBackend === 'GpuKernel' && !includeAll && topK > MAX_GPU_KERNEL_TOP_K) {
  throw new Error(`topK must be <= ${MAX_GPU_KERNEL_TOP_K} on the GpuKernel route when includeAll=false.`)
}
```

### Result

This removes route drift for:

- `Auto`
- composite ADX-on paths
- explicit `GpuKernel` requests that resolve to hybrid
- `exportAllCsvPath`

---

## C5. Parse common strategy state once

### Current problem

`buildDoubleMaOptimizationRequest` and `buildStrategyConfigFromInputs` both parse:

- starting equity
- fees/spread/slippage
- cooldown
- signal persistence
- exit mode
- Kaufman stop settings

And `buildStrategyConfigFromInputs` parses many unrelated strategy-specific inputs even when they are not needed for the current request.

### Recommended split

Introduce two layers:

1. `parseCommonStrategyState(inputs)`
2. `buildKindSpecificStrategyState(inputs, kind)`

Then compose them.

### Example types

```ts
interface CommonStrategyState {
  longOnly: boolean
  allowFlip: boolean
  tradeOnNextBar: boolean
  costPreset: string
  startingEquity: number
  feeBps: number
  spreadBps: number
  slippageBps: number
  commission: number
  epsRel: number
  stopLossPct: number | null
  takeProfitPct: number | null
  cooldownBars: number
  signalPersistenceBars: number
  exitMode: 'cross' | 'kaufman_stop'
  kaufmanStopPeriod: number
  kaufmanStopMult: number
  kaufmanStopMaType: string
  hasExecutionOverlays: boolean
}

interface StrategyPreviewBits {
  trendUseAdx: boolean
  breakoutUseAdx: boolean
}

interface BuiltStrategyConfig {
  strategy: Record<string, unknown>
  preview: StrategyPreviewBits
  common: CommonStrategyState
}
```

### Example shape

```ts
function parseCommonStrategyState(inputs: StrategyConfigBuildInputs): CommonStrategyState {
  const feeBps = Math.max(0, num(inputs.feeBps, 0))
  const spreadBps = Math.max(0, num(inputs.spreadBps, 0))
  const slippageBps = Math.max(0, num(inputs.slippageBps, 0))
  const commission = (feeBps + slippageBps + (spreadBps / 2)) / 10_000
  const cooldownBars = Math.max(0, Math.floor(num(inputs.cooldownBars, 0)))
  const signalPersistenceBars = Math.max(1, Math.floor(num(inputs.signalPersistenceBars, 1)))
  const exitMode = (normalizeText(inputs.exitMode || 'cross') || 'cross') as 'cross' | 'kaufman_stop'
  const kaufmanStopPeriod = Math.max(1, Math.floor(num(inputs.kaufmanStopPeriod, 22)))
  const kaufmanStopMult = Math.max(0.0001, num(inputs.kaufmanStopMult, 2.0))

  if (!Number.isFinite(commission) || commission < 0 || commission >= 1) {
    throw new Error('total cost must be in [0, 1).')
  }
  if (!Number.isFinite(cooldownBars) || cooldownBars < 0 || cooldownBars > 65535) {
    throw new Error('cooldown bars must be in [0, 65535].')
  }
  if (!Number.isFinite(signalPersistenceBars) || signalPersistenceBars < 1 || signalPersistenceBars > 65535) {
    throw new Error('signal persistence bars must be in [1, 65535].')
  }
  if (!(exitMode === 'cross' || exitMode === 'kaufman_stop')) {
    throw new Error("exit mode must be 'cross' or 'kaufman_stop'.")
  }
  if (!Number.isFinite(kaufmanStopMult) || kaufmanStopMult <= 0) {
    throw new Error('kaufman-stop multiplier must be > 0.')
  }

  return {
    longOnly: normalizeBoolean(inputs.longOnly),
    allowFlip: normalizeBoolean(inputs.allowFlip),
    tradeOnNextBar: true,
    costPreset: normalizeText(inputs.costPreset || 'custom') || 'custom',
    startingEquity: normalizeStartingEquity(inputs.startingEquity),
    feeBps,
    spreadBps,
    slippageBps,
    commission,
    epsRel: num(inputs.epsRel, 0),
    stopLossPct: parseRiskPct(inputs.stopLossPct),
    takeProfitPct: parseRiskPct(inputs.takeProfitPct),
    cooldownBars,
    signalPersistenceBars,
    exitMode,
    kaufmanStopPeriod,
    kaufmanStopMult,
    kaufmanStopMaType: normalizeText(inputs.kaufmanStopMaType || 'sma') || 'sma',
    hasExecutionOverlays: cooldownBars > 0 || signalPersistenceBars > 1,
  }
}
```

Then:

```ts
function buildStrategyConfigFromInputs(
  inputs: StrategyConfigBuildInputs,
  kindRaw: unknown,
): BuiltStrategyConfig {
  const kind = normalizeStrategyKindRaw(kindRaw)
  const common = parseCommonStrategyState(inputs)

  switch (kind) {
    case 'CompositeTrend':
      return buildCompositeTrendStrategy(inputs, common)
    case 'RegimeHybrid':
      return buildRegimeHybridStrategy(inputs, common)
    case 'BreakoutMomentum':
      return buildBreakoutMomentumStrategy(inputs, common)
    case 'Pit':
      return buildPitStrategy(inputs, common)
    case 'Vfct':
      return buildVfctStrategy(inputs, common)
    default:
      return buildDefaultDoubleMaStrategy(inputs, kind, common)
  }
}
```

### Why this matters

This is the second biggest warm-path improvement after range-count allocation removal.

It reduces:

- repeated `Number(...)`
- repeated `Math.floor(...)`
- repeated `normalizeText(...)`
- unnecessary parsing of inactive strategy-specific fields

### Practical guidance

Do **not** try to refactor every composite CPU sub-strategy in one giant patch. Split this in stages:

1. common strategy state
2. double-MA/composite-profile specific strategy state
3. only then the composite CPU / CompositeMo callers

That keeps risk manageable.

---

## C6. Normalize data meta once

The builders repeatedly do this:

```ts
const meta = inputs.context?.current_data_meta
const record = meta && typeof meta === 'object' ? meta as Record<string, unknown> : null
if (!record?.has_high || !record?.has_low) ...
```

Replace it with a single helper:

```ts
interface NormalizedDataMeta {
  hasOpen: boolean
  hasHigh: boolean
  hasLow: boolean
  hasClose: boolean
  hasVolume: boolean
}

function normalizeDataMeta(raw: unknown): NormalizedDataMeta {
  const meta = raw && typeof raw === 'object' ? raw as Record<string, unknown> : {}
  return {
    hasOpen: !!meta.has_open,
    hasHigh: !!meta.has_high,
    hasLow: !!meta.has_low,
    hasClose: !!meta.has_close,
    hasVolume: !!meta.has_volume,
  }
}
```

Then every builder and strategy-specific validator reads from that one structure.

### Benefit

Small direct CPU win, but more importantly:

- clearer validation logic
- fewer repeated object casts
- fewer chances for route-specific field checks to drift

---

## C7. Make `normalizeMode` allocation-free

Today `normalizeMode` takes an object:

```ts
normalizeMode(inputs.mode, { includeAll, heatmapBins, exportAllCsvPath })
```

That creates a tiny throwaway object each call. It is not the biggest win, but it is free to fix:

```ts
function normalizeMode(
  value: unknown,
  includeAll: boolean,
  heatmapBins: number,
  exportAllCsvPath: string,
): 'Grid' | 'CoarseToFine' | 'GpuVdps' | 'Auto'
```

This is a micro-optimization only. Do it while touching the surrounding code, but do not prioritize it over the bigger wins.

---

## C8. Add named constants for guardrails

Near the top of `optimizeRequestBuilder.ts`, add:

```ts
const MAX_HEATMAP_BINS = 512
const MAX_INCLUDE_ALL_COMBOS = 2_000_000
const MAX_GPU_KERNEL_TOP_K = 256
```

Then use them everywhere.

### Benefit

- easier perf-gate review
- easier benchmark-backed tuning later
- avoids magic-number drift across the file and tests

### Important note

Do not change the numeric values in the same patch unless you have benchmark evidence.

---

## C9. Refactor `buildDoubleMaOptimizationRequest`

### Suggested skeleton

```ts
export function buildDoubleMaOptimizationRequest(
  inputs: DoubleMaOptimizationBuildInputs,
): Record<string, unknown> | null {
  const kind = normalizeStrategyKindRaw(inputs.context?.strategy_kind)
  if (!isDoubleMaOverlayKind(kind)) return null

  const dataId = normalizeText(inputs.context?.data_id)
  if (!dataId) throw new Error('Load CSV first (missing data id).')

  const dataMeta = normalizeDataMeta(inputs.context?.current_data_meta)

  const fastRange = parseU16Range(inputs.fastStart, inputs.fastEnd, inputs.fastStep, 'fast')
  const slowRange = parseU16Range(inputs.slowStart, inputs.slowEnd, inputs.slowStep, 'slow')

  const fastMaType = normalizeText(inputs.fastMaType)
  const slowMaType = normalizeText(inputs.slowMaType)
  if (!fastMaType) throw new Error('Select a fast MA type.')
  if (!slowMaType) throw new Error('Select a slow MA type.')

  const maSource = normalizeText(inputs.maSource || 'close') || 'close'
  validateCandleFieldRequirements(inputs.context?.current_data_meta ?? null, fastMaType, slowMaType, maSource)
  validateRouteSpecificFields(kind, dataMeta, inputs)

  const requestedBackend = normalizeRequestedBackend(inputs.backend)
  const route = resolveDoubleMaRoutePreview(kind, requestedBackend, inputs)

  const maSupport = createMovingAverageSupportLookup(inputs.maSupportMap)
  validateMaSupport(maSupport, requestedBackend, fastMaType, slowMaType)

  const topK = Math.max(0, Math.floor(num(inputs.topK, 0)))
  const includeAll = normalizeIncludeAll(inputs.includeAll)
  const heatmapBins = Math.max(0, Math.floor(num(inputs.heatmapBins, 0)))
  if (heatmapBins > MAX_HEATMAP_BINS) {
    throw new Error(`heatmapBins must be <= ${MAX_HEATMAP_BINS}.`)
  }

  const exportAllCsvPath = normalizeText(inputs.exportAllCsvPath)
  const mode = normalizeMode(inputs.mode, includeAll, heatmapBins, exportAllCsvPath)

  const fastMaParams = normalizeMaParams(inputs.fastMaParams)
  const slowMaParams = normalizeMaParams(inputs.slowMaParams)
  const fastMaParamSweeps = normalizeMaParamSweeps(inputs.fastMaParamSweeps)
  const slowMaParamSweeps = normalizeMaParamSweeps(inputs.slowMaParamSweeps)

  const fastVariantCount = maSweepVariantCount(fastMaParamSweeps)
  const slowVariantCount = maSweepVariantCount(slowMaParamSweeps)
  const variantMultiplier = fastVariantCount * slowVariantCount

  const comboCap = includeAll
    ? Math.floor(MAX_INCLUDE_ALL_COMBOS / Math.max(1, variantMultiplier)) + 1
    : Number.POSITIVE_INFINITY

  const pairs = countFastLtSlowPairs(fastRange, slowRange, comboCap)
  if (!pairs) throw new Error('No valid parameter combinations (fast must be < slow).')

  const combos = pairs * variantMultiplier
  if (includeAll && combos > MAX_INCLUDE_ALL_COMBOS) {
    throw new Error(`includeAll=true would return ${combos} rows; disable it or reduce ranges/sweeps.`)
  }

  const built = buildStrategyConfigFromInputs(inputs, kind)
  const gpuExecution = resolveDoubleMaGpuExecutionPolicy(route.effectiveBackend, mode)

  if (route.isKernelOnlyRoute) {
    if (built.common.exitMode !== 'cross') {
      throw new Error('GpuKernel currently supports exit mode = cross only.')
    }
    if (built.common.hasExecutionOverlays) {
      throw new Error('GpuKernel does not support cooldown/persistence overlays.')
    }
  }

  if (route.effectiveBackend === 'GpuKernel' && !includeAll && topK > MAX_GPU_KERNEL_TOP_K) {
    throw new Error(`topK must be <= ${MAX_GPU_KERNEL_TOP_K} on the GpuKernel route when includeAll=false.`)
  }

  if (exportAllCsvPath) {
    validateExportAll(route, mode, includeAll, variantMultiplier)
  }

  const dynamicStrategyIr =
    kind === 'DynamicDoubleMa' &&
    inputs.dynamicStrategyIr &&
    typeof inputs.dynamicStrategyIr === 'object' &&
    !Array.isArray(inputs.dynamicStrategyIr)
      ? inputs.dynamicStrategyIr as Record<string, unknown>
      : null

  return {
    strategy_kind: kind,
    backend: route.backendPayload,
    data_id: dataId,
    starting_equity: built.common.startingEquity,
    fast_range: fastRange.raw,
    slow_range: slowRange.raw,
    fast_ma_types: [fastMaType],
    slow_ma_types: [slowMaType],
    ma_source: maSource,
    export_all_csv_path: exportAllCsvPath || null,
    fast_ma_params: fastMaParams,
    slow_ma_params: slowMaParams,
    fast_ma_param_sweeps: fastMaParamSweeps,
    slow_ma_param_sweeps: slowMaParamSweeps,
    strategy: built.strategy,
    objective: normalizeText(inputs.objective || 'Sharpe') || 'Sharpe',
    mode,
    vdps: null,
    top_k: topK,
    include_all: includeAll,
    heatmap_bins: heatmapBins,
    constraints: buildDoubleMaConstraints(inputs),
    dynamic_strategy_ir: dynamicStrategyIr,
    gpu_execution: gpuExecution,
    persist_followup_cache: true,
  }
}
```

### Key improvements in this skeleton

- no array expansion
- no full MA map copy
- parse once
- effective-route guardrails
- named constants
- direct reuse of parsed ranges in payload

---

## C10. Refactor `buildCompositeCpuOptimizationRequest`

This function shares the same range-count allocation issue and most of the same repeated normalization patterns.

### Recommended changes

1. Replace fast/slow array expansion with `parseU16Range` + `countFastLtSlowPairs`
2. Replace `normalizeMovingAverageSupportMap` with `createMovingAverageSupportLookup`
3. Use `normalizeDataMeta`
4. Replace repeated `kind !== ...` chains with a `Set`
5. Use the parse-once common strategy state instead of redoing common parsing

### Suggested helper sets

```ts
const GPU_NATIVE_COMPOSITE_KINDS = new Set([
  'Asr',
  'Vcc',
  'Vemr',
  'Rocb',
  'Vcet',
  'Kqsr',
  'Carm',
  'Kvdf',
  'Mkcs',
  'Cdmc',
] as const)
```

Then:

```ts
const isGpuNativeCompositeKind = GPU_NATIVE_COMPOSITE_KINDS.has(kind as any)
```

### Why this helps

The direct CPU savings are smaller than on the Double-MA builder, but:

- it removes the same unnecessary range arrays
- it removes the same support-map copy
- it makes `CompositeCpu` and GPU-native composite routes easier to keep consistent

### What not to do

Do **not** try to micro-optimize every per-kind object literal in the same patch. The big win is the shared parsing and shared range logic. The per-kind object literals only become worth touching if a benchmark proves the request-builder CPU time is still material after the shared fixes.

---

## C11. Refactor `buildCompositeMoOptimizationRequest`

This path is important because the review map explicitly calls out `CompositeMo` as its own search-efficiency surface.

### Immediate improvements worth making in this file

1. Replace range expansion with arithmetic pair counting
2. Reuse parsed fast/slow ranges to build the `IntRange` domains
3. Replace full MA support-map copy with on-demand lookup
4. Reuse parse-once common strategy state for `baseStrategy`

### Concrete code reuse

Instead of reparsing the range again later:

```ts
const rawFastStep = Math.floor(num(inputs.fastStep, 1))
const rawSlowStep = Math.floor(num(inputs.slowStep, 1))
const fastStep = Math.max(1, rawFastStep)
const slowStep = Math.max(1, rawSlowStep)
const fastLow = Math.floor(fast[0])
const slowLow = Math.floor(slow[0])
const fastHigh = rawFastStep === 0 ? fastLow : Math.floor(fast[1])
const slowHigh = rawSlowStep === 0 ? slowLow : Math.floor(slow[1])
```

reuse the parsed ranges directly:

```ts
const fastDomain = {
  IntRange: {
    low: fastRange.low,
    high: fastRange.high,
    step: fastRange.effectiveStep,
    log: false,
  },
}
const slowDomain = {
  IntRange: {
    low: slowRange.low,
    high: slowRange.high,
    step: slowRange.effectiveStep,
    log: false,
  },
}
```

### Why this matters

This is not only faster; it also reduces drift. Right now the file computes pair validity one way and later reconstructs domain metadata another way.

---

## D. Rust: fix route labels without overengineering

`composite_profile_route.rs` is not the place to chase unsafe memory tricks. It is already basically free from an allocation perspective.

The actual issue is that the label surface mixes identity and route.

### Recommended change

Keep profile identity and execution route separate.

### Proposed code

```rust
use crate::strategy_ir::StrategyProfileKind;
use ta_strategies::double_ma::StrategyConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompositeProfileGpuTarget {
    CpuOnly,
    GpuKernel,
    GpuHybridKernel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CompositeProfileRoute {
    pub profile_kind: StrategyProfileKind,
    pub gpu_target: CompositeProfileGpuTarget,
}

impl CompositeProfileRoute {
    pub(crate) const fn profile_label(self) -> &'static str {
        match self.profile_kind {
            StrategyProfileKind::CompositeCpu => "CompositeCpu",
            StrategyProfileKind::CompositeTrend => "CompositeTrend",
            StrategyProfileKind::RegimeHybrid => "RegimeHybrid",
            StrategyProfileKind::BreakoutMomentum => "BreakoutMomentum",
            StrategyProfileKind::DoubleMa => "DoubleMa",
        }
    }

    pub(crate) const fn execution_label(self) -> &'static str {
        match (self.profile_kind, self.gpu_target) {
            (StrategyProfileKind::CompositeCpu, CompositeProfileGpuTarget::CpuOnly) => "CompositeCpuCpuOnly",
            (StrategyProfileKind::CompositeTrend, CompositeProfileGpuTarget::CpuOnly) => "CompositeTrendCpuOnly",
            (StrategyProfileKind::CompositeTrend, CompositeProfileGpuTarget::GpuKernel) => "CompositeTrendGpuKernel",
            (StrategyProfileKind::CompositeTrend, CompositeProfileGpuTarget::GpuHybridKernel) => "CompositeTrendGpuHybridKernel",
            (StrategyProfileKind::RegimeHybrid, CompositeProfileGpuTarget::CpuOnly) => "RegimeHybridCpuOnly",
            (StrategyProfileKind::RegimeHybrid, CompositeProfileGpuTarget::GpuKernel) => "RegimeHybridGpuKernel",
            (StrategyProfileKind::RegimeHybrid, CompositeProfileGpuTarget::GpuHybridKernel) => "RegimeHybridGpuHybridKernel",
            (StrategyProfileKind::BreakoutMomentum, CompositeProfileGpuTarget::CpuOnly) => "BreakoutMomentumCpuOnly",
            (StrategyProfileKind::BreakoutMomentum, CompositeProfileGpuTarget::GpuKernel) => "BreakoutMomentumGpuKernel",
            (StrategyProfileKind::BreakoutMomentum, CompositeProfileGpuTarget::GpuHybridKernel) => "BreakoutMomentumGpuHybridKernel",
            (StrategyProfileKind::DoubleMa, CompositeProfileGpuTarget::CpuOnly) => "DoubleMaCpuOnly",
            (StrategyProfileKind::DoubleMa, CompositeProfileGpuTarget::GpuKernel) => "DoubleMaGpuKernel",
            (StrategyProfileKind::DoubleMa, CompositeProfileGpuTarget::GpuHybridKernel) => "DoubleMaGpuHybridKernel",
            _ => "UnknownCompositeProfileRoute",
        }
    }
}

#[inline]
pub(crate) fn composite_profile_route(strategy: &StrategyConfig) -> CompositeProfileRoute {
    let profile_kind = composite_profile_from_strategy(strategy);
    let gpu_target = match profile_kind {
        StrategyProfileKind::CompositeTrend => {
            if strategy.trend_quality_use_adx() {
                CompositeProfileGpuTarget::GpuHybridKernel
            } else {
                CompositeProfileGpuTarget::GpuKernel
            }
        }
        StrategyProfileKind::BreakoutMomentum => {
            if strategy.breakout_use_adx() {
                CompositeProfileGpuTarget::GpuHybridKernel
            } else {
                CompositeProfileGpuTarget::GpuKernel
            }
        }
        StrategyProfileKind::RegimeHybrid => CompositeProfileGpuTarget::GpuHybridKernel,
        StrategyProfileKind::CompositeCpu | StrategyProfileKind::DoubleMa => CompositeProfileGpuTarget::CpuOnly,
    };

    CompositeProfileRoute { profile_kind, gpu_target }
}

#[cfg_attr(not(feature = "cuda"), allow(dead_code))]
pub(crate) fn composite_profile_runtime_label(strategy: &StrategyConfig) -> &'static str {
    composite_profile_route(strategy).execution_label()
}

pub(crate) fn composite_profile_profile_label(strategy: &StrategyConfig) -> &'static str {
    composite_profile_route(strategy).profile_label()
}
```

### Why this is the right level of change

- keeps the file allocation-free
- improves route-matrix fidelity
- avoids inventing a larger abstraction than needed
- gives benchmarking and UI summaries a better hook

### Optional micro-optimization

Add `#[inline]` to the tiny functions if they are on a hot observability path, but treat that as optional. The main win here is correctness and route-summary clarity, not CPU time.

---

## E. Test plan for `optimizeRequestBuilder.test.ts`

Add table-driven tests instead of many hand-written variants for the route matrix.

### 1) Composite route resolution matrix

```ts
it.each([
  {
    name: 'CompositeTrend auto + ADX off resolves to kernel',
    kind: 'CompositeTrend',
    backend: 'Auto',
    trendUseAdx: false,
    expectedBackend: { backend: 'GpuKernel', device_id: 0 },
  },
  {
    name: 'CompositeTrend auto + ADX on resolves to hybrid',
    kind: 'CompositeTrend',
    backend: 'Auto',
    trendUseAdx: true,
    expectedBackend: { backend: 'GpuHybridKernel', device_id: 0 },
  },
  {
    name: 'RegimeHybrid auto resolves to hybrid',
    kind: 'RegimeHybrid',
    backend: 'Auto',
    expectedBackend: { backend: 'GpuHybridKernel', device_id: 0 },
  },
  {
    name: 'BreakoutMomentum auto + ADX off resolves to kernel',
    kind: 'BreakoutMomentum',
    backend: 'Auto',
    breakoutUseAdx: false,
    expectedBackend: { backend: 'GpuKernel', device_id: 0 },
  },
  {
    name: 'BreakoutMomentum auto + ADX on resolves to hybrid',
    kind: 'BreakoutMomentum',
    backend: 'Auto',
    breakoutUseAdx: true,
    expectedBackend: { backend: 'GpuHybridKernel', device_id: 0 },
  },
])('$name', (row) => {
  const req = buildDoubleMaOptimizationRequest({
    context: {
      strategy_kind: row.kind,
      data_id: 'data-route',
      current_data_meta: {
        has_open: true,
        has_high: true,
        has_low: true,
        has_close: true,
        has_volume: true,
      },
    },
    backend: row.backend,
    fastStart: '5',
    fastEnd: '15',
    fastStep: '5',
    slowStart: '20',
    slowEnd: '40',
    slowStep: '10',
    fastMaType: 'sma',
    slowMaType: 'ema',
    maSource: 'close',
    trendUseAdx: row.trendUseAdx,
    breakoutUseAdx: row.breakoutUseAdx,
  })

  expect(req?.backend).toEqual(row.expectedBackend)
})
```

### 2) Kernel-only restrictions should follow the effective route

This is the most important correctness regression test.

```ts
it('does not apply kernel-only restrictions when requested GpuKernel resolves to hybrid', () => {
  const req = buildDoubleMaOptimizationRequest({
    context: {
      strategy_kind: 'CompositeTrend',
      data_id: 'data-hybrid-effective',
      current_data_meta: {
        has_open: true,
        has_high: true,
        has_low: true,
        has_close: true,
        has_volume: true,
      },
    },
    backend: 'GpuKernel',
    fastStart: '5',
    fastEnd: '15',
    fastStep: '5',
    slowStart: '20',
    slowEnd: '40',
    slowStep: '10',
    fastMaType: 'sma',
    slowMaType: 'ema',
    maSource: 'close',
    trendUseAdx: true,
    exitMode: 'kaufman_stop',
    cooldownBars: '5',
    signalPersistenceBars: '2',
  })

  expect(req?.backend).toEqual({ backend: 'GpuHybridKernel', device_id: 0 })
})
```

Under the current implementation, this sort of case can be rejected because the guardrail only looks at the requested backend string.

### 3) `exportAllCsvPath` must follow the effective route

```ts
it('allows exportAllCsvPath when Auto resolves to GpuKernel', () => {
  const req = buildDoubleMaOptimizationRequest({
    context: {
      strategy_kind: 'CompositeTrend',
      data_id: 'data-export-kernel',
      current_data_meta: {
        has_open: true,
        has_high: true,
        has_low: true,
        has_close: true,
        has_volume: true,
      },
    },
    backend: 'Auto',
    fastStart: '5',
    fastEnd: '15',
    fastStep: '5',
    slowStart: '20',
    slowEnd: '40',
    slowStep: '10',
    fastMaType: 'sma',
    slowMaType: 'ema',
    maSource: 'close',
    trendUseAdx: false,
    mode: 'Grid',
    topK: '20',
    includeAll: 'false',
    heatmapBins: '0',
    exportAllCsvPath: '/tmp/out.csv',
  })

  expect(req?.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
  expect(req?.export_all_csv_path).toBe('/tmp/out.csv')
})

it('rejects exportAllCsvPath when the effective route is hybrid', () => {
  expect(() => buildDoubleMaOptimizationRequest({
    context: {
      strategy_kind: 'CompositeTrend',
      data_id: 'data-export-hybrid',
      current_data_meta: {
        has_open: true,
        has_high: true,
        has_low: true,
        has_close: true,
        has_volume: true,
      },
    },
    backend: 'Auto',
    fastStart: '5',
    fastEnd: '15',
    fastStep: '5',
    slowStart: '20',
    slowEnd: '40',
    slowStep: '10',
    fastMaType: 'sma',
    slowMaType: 'ema',
    maSource: 'close',
    trendUseAdx: true,
    mode: 'Grid',
    topK: '20',
    includeAll: 'false',
    heatmapBins: '0',
    exportAllCsvPath: '/tmp/out.csv',
  })).toThrow('effective GpuKernel route')
})
```

### 4) Unknown backend vocabulary must throw

```ts
it('rejects unknown backend vocabulary instead of silently forcing CPU', () => {
  expect(() => buildDoubleMaOptimizationRequest({
    context: {
      strategy_kind: 'DoubleMa',
      data_id: 'data-unknown-backend',
      current_data_meta: { has_close: true },
    },
    backend: 'GpuKernal', // typo
    fastStart: '5',
    fastEnd: '15',
    fastStep: '5',
    slowStart: '20',
    slowEnd: '40',
    slowStep: '10',
    fastMaType: 'sma',
    slowMaType: 'ema',
    maSource: 'close',
  })).toThrow("Unknown backend 'GpuKernal'")
})
```

### 5) Kernel top-k preflight

```ts
it('preflights the GpuKernel top-k limit when includeAll=false', () => {
  expect(() => buildDoubleMaOptimizationRequest({
    context: {
      strategy_kind: 'DoubleMa',
      data_id: 'data-topk-limit',
      current_data_meta: { has_close: true },
    },
    backend: 'GpuKernel',
    fastStart: '5',
    fastEnd: '15',
    fastStep: '5',
    slowStart: '20',
    slowEnd: '40',
    slowStep: '10',
    fastMaType: 'sma',
    slowMaType: 'ema',
    maSource: 'close',
    includeAll: 'false',
    topK: '257',
  })).toThrow('topK must be <= 256')
})
```

### 6) Capabilities parity tests

If capability logic becomes input-aware, add tests asserting:

- `CompositeTrend + ADX off + Auto` -> `supportsExportAll === true`
- `CompositeTrend + ADX on + Auto` -> `supportsExportAll === false`
- `RegimeHybrid + Auto` -> `supportsExportAll === false`
- `BreakoutMomentum + ADX off` -> `supportsExportAll === true`
- `BreakoutMomentum + ADX on` -> `supportsExportAll === false`
- explicit `CpuOnly` -> `supportsExportAll === false`

---

## Memory / allocation guidance

## What to do

### 1) Remove full range-array creation
This is the biggest frontend memory win in the attached files.

### 2) Stop cloning the full MA support map
Two MA lookups do not justify copying the entire map.

### 3) Reuse parsed range metadata
Use one parsed structure for:
- validation
- pair counting
- request payload
- CompositeMo domain construction

### 4) Parse common strategy state once
Avoid duplicate `Number`, `Math.floor`, `normalizeText`, and `normalizeBoolean` calls.

### 5) Reuse frozen empty singletons where appropriate
Examples:
- empty MA support lookup
- empty constraints object only if not mutated
- shared static kind sets

### 6) Prefer `switch` / set membership over long repeated `kind !== ...` chains
This is mostly readability/maintainability, but it also avoids repeated string comparisons.

## What **not** to do

### 1) Do not use uninitialized memory in TypeScript/JavaScript request builders
There is no safe, meaningful equivalent here. `Buffer.allocUnsafe`, raw `ArrayBuffer` tricks, or object-shape abuse are not appropriate for this surface.

The builder is dominated by:
- parsing
- branching
- object construction
- string normalization

The right optimization is **copy elimination and parse elimination**, not uninitialized memory.

### 2) Do not add `MaybeUninit` to `composite_profile_route.rs`
That file is already branch-only and allocation-free. Unsafe initialization tricks would only add maintenance and safety cost.

### 3) Do not preallocate giant arrays “just in case”
That would make the warm app path worse, not better.

---

## Notes on uninitialized memory, specifically

Because you asked about using uninitialized memory where possible:

- In this bundle’s TypeScript files, there is no good, safe target for it.
- In `composite_profile_route.rs`, there is no meaningful allocation to remove.
- If you later review bundle 3 (`gpu_backtest_kernel.rs`, residency/runtime), **that** is where unsafe or uninitialized-buffer work is potentially appropriate and already exists in adjacent code. It is not the right tool for the route-policy/request-builder layer.

So the honest recommendation for this bundle is:

- **yes** to eliminating memory copies
- **yes** to eliminating temporary arrays and cloned maps
- **no** to uninitialized-memory tricks in the attached frontend and route-policy files

---

## Benchmark and perf-gate plan

These changes mostly affect the **warm app-path** timing boundary and the **route-matrix / route-summary** boundary.

### Measure before and after

1. Builder-only micro-timing in the frontend/test harness
   - `buildDoubleMaOptimizationRequest`
   - `buildCompositeCpuOptimizationRequest`
   - `buildCompositeMoOptimizationRequest`

2. Route-matrix validation
   - confirm the frontend preview matches Rust route selection for the composite profiles

3. Error-path cost
   - invalid requests with giant ranges
   - invalid `includeAll`
   - invalid `exportAllCsvPath`
   - invalid backend vocabulary

### What to expect

- noticeable drop in transient allocations / GC pressure for large invalid range inputs
- moderate CPU reduction from parse-once normalization
- fewer invalid requests due to route-aware capabilities
- cleaner route summaries for benchmark / diagnostic output

### What not to claim without measurement

- any end-to-end optimizer throughput win beyond the frontend shaping cost
- any GPU runtime improvement from these frontend changes
- any benefit from unsafe memory techniques in these files

---

## Rollout plan

## Phase 1 – low-risk structural fixes

1. Add named constants
2. Add `normalizeDataMeta`
3. Add `parseU16Range` + `countFastLtSlowPairs`
4. Add `createMovingAverageSupportLookup`
5. Add strict backend normalization

These are self-contained and easy to test.

## Phase 2 – route preview unification

6. Extend `compositeProfileGpuPolicy.ts` to return a full route preview
7. Refactor `strategyOptimizationCapabilities.ts` to accept route inputs
8. Refactor `buildDoubleMaOptimizationRequest` to use effective-route guardrails

This is the highest-value correctness patch.

## Phase 3 – parse-once strategy state

9. Introduce `parseCommonStrategyState`
10. Refactor `buildStrategyConfigFromInputs`
11. Update `buildCompositeCpuOptimizationRequest`
12. Update `buildCompositeMoOptimizationRequest`

This is the largest frontend CPU cleanup.

## Phase 4 – observability fidelity

13. Refactor `composite_profile_route.rs` to expose profile label vs execution label
14. Update Rust tests accordingly

## Phase 5 – benchmark-backed tuning

15. Re-evaluate `MAX_HEATMAP_BINS`
16. Re-evaluate `MAX_INCLUDE_ALL_COMBOS`
17. Confirm whether other kernel-specific preflight limits belong in the frontend

---

## Risk assessment

## Low risk

- named constants
- no-array pair counting
- normalized data-meta helper
- on-demand MA support lookup
- strict unknown-backend rejection (if the UI already only emits valid values)

## Medium risk

- parse-once strategy-state split
- route-aware capability exposure
- route-aware `exportAllCsvPath` behavior for `Auto`

## Higher risk

- any change to default backend when backend is unspecified
- any attempt to serialize new preview fields to the backend
- any “optimization” involving unsafe memory in this bundle

---

## File-by-file change list

## `web/src/strategies/compositeProfileGpuPolicy.ts`

### Keep
- `CompositeProfileKind`
- `resolveCompositeProfileGpuTarget`

### Change
- add strict requested-backend normalizer
- add route preview object
- make summary function route-aware

### Expected benefit
- central route authority
- less drift with capabilities and request builder

---

## `web/src/utils/strategyOptimizationCapabilities.ts`

### Keep
- non-composite strategy branches as-is initially

### Change
- composite branches become input-aware
- `supportsExportAll` becomes route-aware
- summaries describe the effective route, not just the kind

### Expected benefit
- fewer invalid requests
- lower support-matrix drift

---

## `web/src/services/optimizeRequestBuilder.ts`

### Keep
- request payload shapes
- existing public builder entry points

### Change
- remove range expansion usage
- remove full support-map clone
- add strict backend normalization
- add route preview usage
- split common strategy parsing from kind-specific parsing
- reuse parsed range structures in all three builders
- add named constants and known kernel preflight

### Expected benefit
- the majority of frontend request-shaping savings in this bundle

---

## `web/src/services/optimizeRequestBuilder.test.ts`

### Keep
- existing happy-path coverage

### Change
- add table-driven composite route matrix
- add effective-route guardrail tests
- add capability parity tests
- add unknown-backend rejection tests
- add top-k kernel-limit preflight tests

### Expected benefit
- drift prevention and safer refactor rollout

---

## `crates/app_core/src/composite_profile_route.rs`

### Keep
- core route-policy logic
- ADX-sensitive target selection

### Change
- expose a `CompositeProfileRoute`
- split identity label from execution label
- update tests to assert the new labels

### Expected benefit
- better route-summary fidelity
- no meaningful runtime regression

---

## Final recommendation

If you only do three things, do these first:

1. **Replace range-array expansion with arithmetic pair counting**
2. **Add a single effective-route preview and use it for guardrails**
3. **Split common strategy parsing so the builder stops reparsing everything twice**

That combination gives you the best mix of:

- real frontend performance improvement
- less allocation churn
- fewer unnecessary memory copies
- less route drift
- better regression safety

It is also the right response to your goal of minimizing unnecessary memory movement without introducing unsafe or artificial low-level tricks where they do not belong.


---

## Addendum: additional findings, corrections, and optional changes

This addendum captures several items that were either underemphasized or not called out explicitly enough in the first version.

## 1. Additional confirmed findings

### 1.1 `strategyOptimizationCapabilities.ts` is missing several Double-MA overlay kinds

This is the biggest thing I would add to the first version.

`optimizeRequestBuilder.ts` keeps these kinds on the Double-MA path:

- `Pit`
- `Set`
- `Mts`
- `Orc`
- `Marrt`
- `Vfct`
- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`

But `strategyOptimizationCapabilities.ts` does not include `Pit`, `Set`, `Mts`, `Orc`, or `Marrt` in `StrategyOptimizationKind`, `normalizeKind`, or the Double-MA capability branch.

That means the capability layer can silently classify a real supported builder kind as `Unknown`, which is a support-matrix drift bug and can also expose the wrong UI controls.

### Recommended edit

Expand the capability kind union and normalize through a shared family set.

```ts
export type StrategyOptimizationKind =
  | 'DoubleMa'
  | 'DynamicDoubleMa'
  | 'MlSignal'
  | 'Pit'
  | 'Set'
  | 'Mts'
  | 'Orc'
  | 'Marrt'
  | 'Vfct'
  | 'Asr'
  | 'Vcc'
  | 'Vemr'
  | 'Rocb'
  | 'Vcet'
  | 'Kqsr'
  | 'Carm'
  | 'Kvdf'
  | 'Mkcs'
  | 'Cdmc'
  | 'CompositeCpu'
  | 'CompositeTrend'
  | 'RegimeHybrid'
  | 'BreakoutMomentum'
  | 'CompositeMo'
  | 'Unknown'

const DOUBLE_MA_FAMILY_KINDS = new Set<StrategyOptimizationKind>([
  'DoubleMa',
  'DynamicDoubleMa',
  'Pit',
  'Set',
  'Mts',
  'Orc',
  'Marrt',
  'Vfct',
  'CompositeTrend',
  'RegimeHybrid',
  'BreakoutMomentum',
])
```

Then use `DOUBLE_MA_FAMILY_KINDS.has(kind)` instead of manually repeating a partial subset.

### Why this matters

This is mostly a correctness and product-stability fix, but it also prevents wasted retry cycles and invalid UI states, which indirectly reduces warm app-path churn.

## 1.2 Unknown capabilities currently fail open on `topK` / `includeAll`

Another thing I would explicitly correct: the `Unknown` branch in `strategyOptimizationCapabilities.ts` currently returns:

```ts
supportsTopK: true
supportsIncludeAll: true
```

That is the wrong default for a capability surface.

### Recommended edit

Fail closed for unknown kinds.

```ts
const UNKNOWN_CAPABILITIES: StrategyOptimizationCapabilities = Object.freeze({
  kind: 'Unknown',
  usesGlobalBackend: false,
  usesGlobalMode: false,
  supportsHeatmap: false,
  supportsExportAll: false,
  supportsTopK: false,
  supportsIncludeAll: false,
  executionSummary: 'Unknown strategy kind. No global optimization capabilities should be assumed.',
  modeSummary: 'Unknown strategy kind.',
  outputSummary: 'Only explicitly supported controls should be shown.',
})
```

If the product still needs “best effort” behavior for unknown kinds, split the model into:

- `possibleSupport`
- `currentAvailability`

But do not let the default branch expose shaping controls by accident.

## 1.3 Canonicalize MA IDs once, in one place

This one is easy to miss because the current code works when the caller already uses the expected casing.

Right now:

- whitelist checks sometimes use `.toLowerCase()`
- MA support map normalization preserves the original key spelling
- support lookups use the raw normalized `fastMaType` / `slowMaType`
- the backend GPU path lowercases MA ids before use

That leaves avoidable room for case-drift bugs.

### Recommended edit

Add a dedicated MA-id normalizer and use it consistently for support lookups.

```ts
function normalizeMaId(value: unknown): string {
  return normalizeText(value).toLowerCase()
}

function normalizeMovingAverageSupportMap(
  raw: unknown,
): Record<string, MovingAverageSupportEntry> {
  if (!raw || typeof raw !== 'object') return EMPTY_SUPPORT_MAP

  const out: Record<string, MovingAverageSupportEntry> = Object.create(null)
  let hasAny = false
  for (const key in raw as Record<string, unknown>) {
    const value = (raw as Record<string, unknown>)[key]
    const id = normalizeMaId(key)
    if (!id || !value || typeof value !== 'object') continue
    out[id] = value as MovingAverageSupportEntry
    hasAny = true
  }
  return hasAny ? out : EMPTY_SUPPORT_MAP
}
```

Then in each builder:

```ts
const fastMaType = normalizeMaId(inputs.fastMaType)
const slowMaType = normalizeMaId(inputs.slowMaType)
```

### Why this matters

This is a correctness fix first, but it also reduces repeated per-branch lowercasing and simplifies support checks.

## 1.4 `CompositeMo` only needs pair existence, not a full count

In `buildCompositeMoOptimizationRequest`, the current code expands both ranges and counts all `fast < slow` pairs just to reject the all-invalid case.

That is more work than the function needs.

### Recommended edit

Use a small arithmetic helper that only answers “does any valid pair exist?”.

```ts
function hasAnyFastLtSlowPair(
  fast: readonly [number, number, number],
  slow: readonly [number, number, number],
): boolean {
  const fastCount = countU16RangeValues(fast[0], fast[1], fast[2])
  const slowCount = countU16RangeValues(slow[0], slow[1], slow[2])
  if (fastCount <= 0 || slowCount <= 0) return false

  const fastMin = Math.floor(fast[0])
  const fastMax = Math.floor(fast[1])
  const slowMin = Math.floor(slow[0])
  const slowMax = Math.floor(slow[1])
  return fastMin < slowMax && slowMin > fastMin ? true : fastMax < slowMax
}
```

Or better: reuse the same capped arithmetic counter proposed in the first plan and compare with `> 0`.

### Why this matters

This is a real warm-path allocation win for `CompositeMo`, and it is especially valuable because `CompositeMo` already has enough search-space work without paying for unnecessary range materialization.

## 1.5 Add capped multiplication to avoid numeric overflow / precision drift

The first version discussed arithmetic counting, but I would make the overflow part more explicit.

Current code does:

```ts
const combos = pairs * fastVariantCount * slowVariantCount
```

That is fine for ordinary ranges, but it can drift once counts get large enough for JavaScript number precision to matter.

### Recommended edit

Use a saturating / capped multiply helper.

```ts
function multiplyCapped(limit: number, ...values: number[]): number {
  let acc = 1
  for (const value of values) {
    if (!Number.isFinite(value) || value <= 0) return 0
    if (acc > limit / value) return limit + 1
    acc *= value
  }
  return acc
}

const combos = multiplyCapped(MAX_INCLUDE_ALL_ROWS, pairs, fastVariantCount, slowVariantCount)
if (includeAll && combos > MAX_INCLUDE_ALL_ROWS) {
  throw new Error(
    `includeAll=true would return more than ${MAX_INCLUDE_ALL_ROWS} rows; disable it or reduce ranges/sweeps.`
  )
}
```

### Why this matters

This is both a correctness fix and a micro-optimization because it stops work as soon as the guardrail limit is exceeded.

## 1.6 Remove small but repeated array allocations in normalizers

The first version mentioned the big range-allocation issue. I would add that there are also smaller repeated allocations from `Object.entries(...)`, `Object.keys(...)`, and short-lived arrays in helper code.

### Recommended edit

Prefer `for ... in` and a `hasAny` flag for these helpers:

- `normalizeMaParams`
- `normalizeMaParamSweeps`
- `normalizeMovingAverageSupportMap`
- `maSweepVariantCount`
- `hasMovingAverageSupportData`

Example:

```ts
function normalizeMaParams(value: unknown): Record<string, number> | null {
  if (!value || typeof value !== 'object') return null
  const input = value as Record<string, unknown>
  const out: Record<string, number> = Object.create(null)
  let hasAny = false
  for (const key in input) {
    const name = normalizeText(key)
    if (!name) continue
    const parsed = Number(input[key])
    if (!Number.isFinite(parsed)) continue
    out[name] = parsed
    hasAny = true
  }
  return hasAny ? out : null
}
```

### Why this matters

These are not the top hotspot individually, but together they reduce allocation noise in the request builders.

## 1.7 Capability objects can be made table-driven and mostly allocation-free

This is optional, but still worthwhile.

`describeStrategyOptimizationCapabilities(...)` currently builds a fresh object every call, including long summary strings. If the UI re-asks for capabilities during reactive renders, that creates avoidable garbage.

### Recommended edit

For static kinds, return frozen prebuilt objects from a constant table. For route-aware composite kinds, derive from a prebuilt base object and only patch the few route-dependent fields.

```ts
const STATIC_CAPABILITIES: Record<string, StrategyOptimizationCapabilities> = Object.freeze({
  MlSignal: Object.freeze({ ... }),
  CompositeMo: Object.freeze({ ... }),
  CompositeCpu: Object.freeze({ ... }),
})
```

### Why this matters

This is low risk. It is mainly a frontend churn reduction, not a dramatic throughput change.

## 1.8 `tradeOnNext` is currently a dead input in the builder surface

`StrategyConfigBuildInputs` includes `tradeOnNext`, but `buildStrategyConfigFromInputs` emits `trade_on_next_bar: true` unconditionally.

### Recommended handling

I would treat this as an explicit cleanup decision:

- if the product is intentionally fixed to `true`, remove or deprecate the dead input from the request-builder surface
- if the product expects the toggle to work, parse it once and honor it everywhere

### Safe recommendation

Because your stated goal is “do not break functionality,” the safest immediate move is:

1. keep runtime behavior unchanged
2. document `tradeOnNext` as ignored / fixed
3. add one test that makes the fixed behavior explicit
4. only change it after confirming backend support and intended product semantics

## 2. Additional test coverage I would add

### 2.1 Capability drift tests for missing overlay kinds

Add explicit tests for:

- `Pit`
- `Set`
- `Mts`
- `Orc`
- `Marrt`

Each should assert the capability layer classifies them as Double-MA-family kinds rather than `Unknown`.

### 2.2 Unknown capability fail-closed test

```ts
it('fails closed for unknown strategy kinds in capability exposure', () => {
  const caps = describeStrategyOptimizationCapabilities('DefinitelyNotAKind')
  expect(caps.supportsTopK).toBe(false)
  expect(caps.supportsIncludeAll).toBe(false)
  expect(caps.supportsHeatmap).toBe(false)
  expect(caps.supportsExportAll).toBe(false)
})
```

### 2.3 MA-id case canonicalization tests

```ts
it('treats MA support map ids case-insensitively', () => {
  const req = buildDoubleMaOptimizationRequest({
    ...baseInputs,
    fastMaType: 'EMA',
    slowMaType: 'VWMA',
    maSupportMap: {
      ema: { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
      vwma: { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
    },
  })

  expect(req?.fast_ma_types).toEqual(['ema'])
  expect(req?.slow_ma_types).toEqual(['vwma'])
})
```

### 2.4 Capped-combo arithmetic parity tests

Add table-driven tests comparing the new capped arithmetic counter against the current expand-and-count behavior on a range matrix of small inputs.

That gives you safe proof before swapping out the old logic.

### 2.5 Rust precedence tests

`composite_profile_from_strategy(...)` has an implicit precedence order:

1. breakout
2. regime hybrid
3. trend quality
4. composite CPU

Add tests that make this precedence explicit when multiple flags are set.

Example:

```rust
#[test]
fn breakout_wins_if_multiple_profile_flags_are_set() {
    let mut strategy = StrategyConfig::default();
    strategy.breakout_momentum_enable = true;
    strategy.regime_hybrid_enable = true;
    strategy.trend_quality_enable = true;

    assert_eq!(
        composite_profile_from_strategy(&strategy),
        StrategyProfileKind::BreakoutMomentum
    );
}
```

That is a correctness/observability improvement, not a performance play, but it is cheap and valuable.

## 3. Optional changes that go beyond the original prompt but still fit your goals

These are worthwhile only if you want to push further without broad product redesign.

### 3.1 Emit sparser request payloads only if backend defaults are already proven safe

This is the biggest optional optimization that was not fully spelled out in the first version.

Today `buildStrategyConfigFromInputs(...)` emits a large strategy object with many inactive fields populated even for simple Double-MA cases.

If the backend already deserializes missing fields via struct defaults, you can materially reduce request size and frontend object creation by emitting:

- common strategy fields
- only the active kind-specific fields

### Important warning

This should only be done after confirming deserialize/default behavior in the command boundary and tests.

If that backend proof is missing, keep the current full payload.

### 3.2 Add a request-builder microbenchmark

The repo review map is benchmark-first, but the attached files do not include a direct request-builder measurement harness.

A tiny benchmark for:

- current range expansion path
- arithmetic pair counting path
- current capability shaping
- route-preview path

would make the frontend-side wins measurable rather than anecdotal.

This can be done with a tiny Vitest bench, Node bench, or even a dedicated script that replays representative builder inputs.

### 3.3 Consider a generated route/capability manifest from Rust

This is still beyond the direct prompt, but it stays aligned with correctness and parity.

Longer term, the cleanest way to eliminate frontend/backend route drift is to generate or export a tiny manifest for:

- composite profile route policy
- backend vocabulary
- export-all eligibility
- heatmap eligibility
- mode restrictions

This is more invasive than the other changes, so I would not do it first. But it is the highest-confidence long-term correctness move.

### 3.4 Property-based tests for arithmetic counters

After replacing range-array expansion with arithmetic counting, add randomized parity tests over small bounded ranges.

That gives stronger confidence than only a few hand-picked examples.

### 3.5 Move any “uninitialized memory” work to the actual low-level runtime bundle

Your memory/copy goal is valid, but for this attached surface the right moves are:

- fewer temporary arrays
- fewer temporary objects
- no duplicated parsing
- capped arithmetic instead of full expansion
- fewer repeated string normalizations

If you later want to use `MaybeUninit`, `set_len`, custom buffer pools, pinned host buffers, or uninitialized scratch regions, that belongs in the lower-level runtime / residency bundle, not in these request-building files.

The most relevant future files for that are the ones named in the review map’s GPU runtime bundle, not this current composite route-policy bundle.

## 4. Updated priority order after this addendum

If I were tightening the implementation order now, I would use:

### P0

1. effective-route preview before validation
2. arithmetic pair counting instead of range expansion
3. capability-layer fix for missing kinds + fail-closed unknown behavior

### P1

4. MA-id canonicalization
5. capped combo multiplication
6. stop cloning / reallocating small support maps and sweep maps

### P2

7. mostly allocation-free capability tables
8. CompositeMo zero-check helper
9. Rust precedence tests and route-summary cleanup

### P3 (optional / requires proof)

10. sparse payload emission
11. generated route/capability manifest
12. deeper runtime-level memory work in the GPU bundle

## 5. Bottom line on what I would change in the first version

The first version still has the right core direction.

The most important things I would explicitly add are:

1. **`strategyOptimizationCapabilities.ts` misses real supported kinds**
2. **unknown capability exposure should fail closed**
3. **MA ids should be canonicalized once**
4. **`CompositeMo` should not expand ranges just to check existence**
5. **combo multiplication should be capped / saturating**
6. **small helper allocations (`Object.entries`, `Object.keys`) should be cleaned up after the large range-allocation fix**

Those additions make the plan more complete on correctness, support-matrix drift, and memory/GC pressure while staying fully aligned with your goal: improve performance and correctness without removing features or breaking behavior.
