import {
  type CompositeProfileKind,
  describeCompositeProfileGpuPolicy,
  isCompositeProfileKind,
  normalizeCompositeRequestedBackend,
  resolveCompositeProfileRoute,
} from '../strategies/compositeProfileGpuPolicy'

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

export interface StrategyOptimizationCapabilities {
  kind: StrategyOptimizationKind
  usesGlobalBackend: boolean
  usesGlobalMode: boolean
  supportsHeatmap: boolean
  supportsExportAll: boolean
  supportsTopK: boolean
  supportsIncludeAll: boolean
  supportsExtraShortlistObjectives: boolean
  supportsTradeMetrics: boolean
  supportsCore17MetricSurface: boolean
  supportsMetricContract: boolean
  supportsShortlistConfidenceMode: boolean
  supportsShortlistUnionMode: boolean
  supportsAdvancedExactKernelControls: boolean
  executionSummary: string
  modeSummary: string
  outputSummary: string
}

export interface StrategyOptimizationCapabilityInputs {
  requestedBackend?: unknown
  trendUseAdx?: boolean
  breakoutUseAdx?: boolean
}

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

function normalizeKind(value: unknown): StrategyOptimizationKind {
  const kind = String(value || '').trim()
  switch (kind) {
    case 'DoubleMa':
    case 'DynamicDoubleMa':
    case 'MlSignal':
    case 'Pit':
    case 'Set':
    case 'Mts':
    case 'Orc':
    case 'Marrt':
    case 'Vfct':
    case 'Asr':
    case 'Vcc':
    case 'Vemr':
    case 'Rocb':
    case 'Vcet':
    case 'Kqsr':
    case 'Carm':
    case 'Kvdf':
    case 'Mkcs':
    case 'Cdmc':
    case 'CompositeCpu':
    case 'CompositeTrend':
    case 'RegimeHybrid':
    case 'BreakoutMomentum':
    case 'CompositeMo':
      return kind
    default:
      return 'Unknown'
  }
}

function requestedGlobalBackendIsGpuKernel(value: unknown): boolean {
  const backend = String(value ?? '').trim()
  return (
    backend === 'GpuKernel'
    || backend === 'GpuOnly'
    || backend === 'GpuHybridKernel'
  )
}

function describeCompositeExecutionSummary(
  kind: CompositeProfileKind,
  inputs: StrategyOptimizationCapabilityInputs,
): string {
  const route = resolveCompositeProfileRoute(
    kind,
    normalizeCompositeRequestedBackend(inputs.requestedBackend),
    {
      trendUseAdx: !!inputs.trendUseAdx,
      breakoutUseAdx: !!inputs.breakoutUseAdx,
    },
  )

  if (route.targetBackend === 'CpuOnly') {
    return `${kind} runs through the double-MA optimizer and is currently previewed on the CPU fallback route. ${describeCompositeProfileGpuPolicy(kind)}`
  }
  if (route.usesHybrid) {
    return `${kind} runs through the double-MA optimizer and is currently previewed on the hybrid GPU route. ${describeCompositeProfileGpuPolicy(kind)}`
  }
  return `${kind} runs through the double-MA optimizer and is currently previewed on the GPU kernel route. ${describeCompositeProfileGpuPolicy(kind)}`
}

function describeCompositeOutputSummary(
  kind: CompositeProfileKind,
  inputs: StrategyOptimizationCapabilityInputs,
): {
  supportsExportAll: boolean
  supportsExtraShortlistObjectives: boolean
  supportsTradeMetrics: boolean
  supportsMetricContract: boolean
  supportsShortlistConfidenceMode: boolean
  supportsShortlistUnionMode: boolean
  supportsAdvancedExactKernelControls: boolean
  outputSummary: string
} {
  const route = resolveCompositeProfileRoute(
    kind,
    normalizeCompositeRequestedBackend(inputs.requestedBackend),
    {
      trendUseAdx: !!inputs.trendUseAdx,
      breakoutUseAdx: !!inputs.breakoutUseAdx,
    },
  )

  if (route.supportsExportAll) {
    return {
      supportsExportAll: true,
      supportsExtraShortlistObjectives: true,
      supportsTradeMetrics: true,
      supportsMetricContract: true,
      supportsShortlistConfidenceMode: true,
      supportsShortlistUnionMode: true,
      supportsAdvancedExactKernelControls: true,
      outputSummary: `Top-K, heatmap, include-all, export, and advanced exact-kernel shortlist/score/metric-contract controls apply to ${kind} through the current kernel-targeted double-MA route. Saved optimization artifacts on this audited route emit the canonical Core17 surface. Replay-free exact union/confidence reranks and replay-backed exact slice/neighborhood diagnostics stay route-aligned on this surface.`,
    }
  }

  return {
    supportsExportAll: false,
    supportsExtraShortlistObjectives: false,
    supportsTradeMetrics: true,
    supportsMetricContract: true,
    supportsShortlistConfidenceMode: false,
    supportsShortlistUnionMode: false,
    supportsAdvancedExactKernelControls: false,
    outputSummary:
      route.targetBackend === 'CpuOnly'
        ? `Top-K, heatmap, and include-all controls apply to ${kind} through the current CPU double-MA route. Explicit Full17 and Trade22 metric-contract requests are available on this audited route. Export-all and advanced exact-kernel shortlist/score controls remain disabled on this route.`
        : `Top-K, heatmap, and include-all controls apply to ${kind} through the current hybrid double-MA route. Explicit Full17 and Trade22 metric-contract requests are available on this audited route. Export-all and advanced exact-kernel shortlist/score controls remain disabled on this route.`,
  }
}

function supportsRequestedExactKernelControls(
  kind: StrategyOptimizationKind,
  inputs: StrategyOptimizationCapabilityInputs,
): boolean {
  if (kind === 'DynamicDoubleMa') return false
  if (isCompositeProfileKind(kind)) {
    return resolveCompositeProfileRoute(
      kind,
      normalizeCompositeRequestedBackend(inputs.requestedBackend),
      {
        trendUseAdx: !!inputs.trendUseAdx,
        breakoutUseAdx: !!inputs.breakoutUseAdx,
      },
    ).supportsExportAll
  }
  return requestedGlobalBackendIsGpuKernel(inputs.requestedBackend)
}

function supportsRequestedCore17MetricSurface(
  kind: StrategyOptimizationKind,
  _inputs: StrategyOptimizationCapabilityInputs,
): boolean {
  switch (kind) {
    case 'DoubleMa':
    case 'DynamicDoubleMa':
    case 'Pit':
    case 'Set':
    case 'Mts':
    case 'Orc':
    case 'Marrt':
    case 'Vfct':
    case 'CompositeCpu':
    case 'CompositeTrend':
    case 'RegimeHybrid':
    case 'BreakoutMomentum':
      return true
    case 'Asr':
    case 'Vcc':
    case 'Vemr':
    case 'Rocb':
    case 'Vcet':
    case 'Kqsr':
    case 'Carm':
    case 'Mkcs':
    case 'Kvdf':
    case 'Cdmc':
      return true
    default:
      return false
  }
}

export function describeStrategyOptimizationCapabilities(
  value: unknown,
  inputs: StrategyOptimizationCapabilityInputs = {},
): StrategyOptimizationCapabilities {
  const kind = normalizeKind(value)
  if (DOUBLE_MA_FAMILY_KINDS.has(kind)) {
    const compositeOutput = isCompositeProfileKind(kind)
      ? describeCompositeOutputSummary(kind, inputs)
      : {
          supportsExportAll: true,
          supportsExtraShortlistObjectives:
            kind === 'DynamicDoubleMa'
              ? requestedGlobalBackendIsGpuKernel(inputs.requestedBackend)
              : supportsRequestedExactKernelControls(kind, inputs),
          supportsTradeMetrics: true,
          supportsMetricContract: true,
          supportsShortlistConfidenceMode:
            kind === 'DynamicDoubleMa'
              ? requestedGlobalBackendIsGpuKernel(inputs.requestedBackend)
              : supportsRequestedExactKernelControls(kind, inputs),
          supportsShortlistUnionMode:
            kind === 'DynamicDoubleMa'
              ? requestedGlobalBackendIsGpuKernel(inputs.requestedBackend)
              : supportsRequestedExactKernelControls(kind, inputs),
          supportsAdvancedExactKernelControls:
            kind === 'DynamicDoubleMa'
              ? requestedGlobalBackendIsGpuKernel(inputs.requestedBackend)
              : supportsRequestedExactKernelControls(kind, inputs),
          outputSummary:
            kind === 'DynamicDoubleMa'
              ? 'DynamicDoubleMa builds constrained strategy_ir payloads, previews the resolved backend, and runs through the normal optimizer path after validation. Include-all stays available broadly; extra shortlist objectives are requestable on the GpuKernel preview path for exact-compatible dynamic_strategy_v1 shapes.'
                + ' Explicit 17-row and 22-row metric-contract overrides are available on exact-compatible dynamic_strategy_v1 GpuKernel shapes and on audited dynamic CPU fallback surfaces, including constrained composite/profile plans that resolve to CPU.'
                + ' Shortlist confidence mode follows that same exact-compatible dynamic GpuKernel path as a replay-free exact rerank.'
                + ' Shortlist union mode follows that same exact-compatible dynamic GpuKernel path as a replay-free exact rerank once extra shortlist heads are present.'
                + ' Shortlist slice and neighborhood diagnostics follow that same exact-compatible dynamic GpuKernel path as replay-backed exact diagnostics.'
                : kind === 'Vfct'
                ? 'Top-K, heatmap, include-all, and export controls apply to VFCT runs through the double-MA backend. Saved optimization artifacts on this audited route emit the canonical Core17 surface. Explicit Full17 and Trade22 metric-contract requests are available on current audited CPU/GPU routes; replay-free exact union/confidence reranks and replay-backed exact slice/neighborhood diagnostics stay exact-route only.'
              : kind === 'DoubleMa'
                  ? 'Top-K, heatmap, include-all, and export controls all apply to Double MA runs. Saved optimization artifacts on current audited Double-MA routes emit the canonical Core17 surface. Explicit Full17 and Trade22 metric-contract requests are available on current audited CPU/GPU routes; replay-free exact union/confidence reranks and replay-backed exact slice/neighborhood diagnostics stay exact-route only.'
                  : `Top-K, heatmap, include-all, and export controls apply to ${kind} through the double-MA backend. Saved optimization artifacts on current audited ${kind} routes emit the canonical Core17 surface. Explicit Full17 and Trade22 metric-contract requests are available on current audited CPU/GPU routes; replay-free exact union/confidence reranks and replay-backed exact slice/neighborhood diagnostics stay exact-route only.`,
        }

    return {
      kind,
      usesGlobalBackend: true,
      usesGlobalMode: true,
      supportsHeatmap: true,
      supportsExportAll: compositeOutput.supportsExportAll,
      supportsTopK: true,
      supportsIncludeAll: true,
      supportsExtraShortlistObjectives:
        compositeOutput.supportsExtraShortlistObjectives,
      supportsTradeMetrics: compositeOutput.supportsTradeMetrics,
      supportsCore17MetricSurface: supportsRequestedCore17MetricSurface(kind, inputs),
      supportsMetricContract: compositeOutput.supportsMetricContract,
      supportsShortlistConfidenceMode:
        compositeOutput.supportsShortlistConfidenceMode,
      supportsShortlistUnionMode: compositeOutput.supportsShortlistUnionMode,
      supportsAdvancedExactKernelControls:
        compositeOutput.supportsAdvancedExactKernelControls,
      executionSummary:
        kind === 'DynamicDoubleMa'
          ? 'DynamicDoubleMa is the constrained Double-MA builder path. It validates a previewed strategy_ir, previews GPU-unified eligibility and explicit CPU fallback reasons, and then runs through the normal optimizer path.'
          : kind === 'Vfct'
            ? 'VFCT reuses the double-MA optimizer with volume-backed confirmation filters and unified GPU routing where supported.'
            : isCompositeProfileKind(kind)
              ? describeCompositeExecutionSummary(kind, inputs)
              : kind === 'DoubleMa'
                ? 'Double MA supports CPU, Auto, and unified GPU routing.'
                : `${kind} uses the Double-MA optimizer with strategy-specific overlays and unified GPU routing where supported.`,
      modeSummary:
        kind === 'DynamicDoubleMa'
          ? 'DynamicDoubleMa uses the normal optimization-mode selector after builder preview. Exact-grid outputs still require Grid.'
          : kind === 'Vfct'
            ? 'VFCT uses the Double-MA mode selector. Exact-grid outputs still require Grid.'
            : kind === 'DoubleMa' || isCompositeProfileKind(kind)
              ? `${kind} uses the Double-MA mode selector. Exact-grid outputs still require Grid.`
              : `${kind} uses the Double-MA mode selector. Exact-grid outputs still require Grid.`,
      outputSummary: compositeOutput.outputSummary,
    }
  }
  if (kind === 'MlSignal') {
    return {
      kind,
      usesGlobalBackend: false,
      usesGlobalMode: false,
      supportsHeatmap: false,
      supportsExportAll: false,
      supportsTopK: false,
      supportsIncludeAll: false,
      supportsExtraShortlistObjectives: false,
      supportsTradeMetrics: false,
      supportsCore17MetricSurface: false,
      supportsMetricContract: false,
      supportsShortlistConfidenceMode: false,
      supportsShortlistUnionMode: false,
      supportsAdvancedExactKernelControls: false,
      executionSummary:
        'ML Signal uses a dedicated CPU research path: export features, import external scores, and validate signal policies against the existing simulator.',
      modeSummary:
        'ML Signal does not use the global optimization-mode selector.',
      outputSummary:
        'ML Signal emits dataset-export, score-cache, and validation artifacts instead of optimizer heatmaps or export-all grids.',
    }
  }
  if (
    kind === 'Asr'
    || kind === 'Vcc'
    || kind === 'Vemr'
    || kind === 'Rocb'
    || kind === 'Vcet'
    || kind === 'Kqsr'
    || kind === 'Carm'
    || kind === 'Kvdf'
    || kind === 'Mkcs'
    || kind === 'Cdmc'
  ) {
    const standaloneGpuKernelPreview = requestedGlobalBackendIsGpuKernel(inputs.requestedBackend)
    return {
      kind,
      usesGlobalBackend: true,
      usesGlobalMode: false,
      supportsHeatmap: false,
      supportsExportAll: false,
      supportsTopK: true,
      supportsIncludeAll: true,
      supportsExtraShortlistObjectives: standaloneGpuKernelPreview,
      supportsTradeMetrics: true,
      supportsCore17MetricSurface: supportsRequestedCore17MetricSurface(kind, inputs),
      supportsMetricContract: true,
      supportsShortlistConfidenceMode: standaloneGpuKernelPreview,
      supportsShortlistUnionMode: standaloneGpuKernelPreview,
      supportsAdvancedExactKernelControls: false,
      executionSummary:
        kind === 'Mkcs'
          ? 'MKCS can use CPU or unified GPU execution, with fast/slow ranges mapped into compression and breakout periods.'
          : kind === 'Kqsr'
            ? 'KQSR can use CPU or unified GPU execution, with GPU scoring fed by staged oscillator and band signal matrices.'
            : kind === 'Vcet'
              ? 'VCET can use CPU or unified GPU execution, with GPU scoring fed by VIDYA trend-state rows and Chandelier Exit stop overlays.'
              : kind === 'Carm'
                ? 'CARM can use CPU or unified GPU execution, with GPU scoring fed by staged Coppock, AO, ROCR, APO, and OTTO HOTT/LOTT signal matrices.'
                : kind === 'Kvdf'
                  ? 'KVDF can use CPU or unified GPU execution, with GPU scoring fed by staged Keltner, VWAP, DTI, DevStop, and FVG trailing-stop signal matrices.'
                  : kind === 'Cdmc'
                    ? 'CDMC can use CPU or unified GPU execution, with GPU scoring fed by staged cycle, MAMA/FAMA, and decycler signal matrices.'
                    : `${kind} can use CPU or unified GPU execution, but optimization mode is fixed to Grid.`,
      modeSummary: `${kind} always runs in Grid mode.`,
        outputSummary: standaloneGpuKernelPreview
        ? `${kind} supports Top-K and include-all native candidate returns. On requested standalone kernel-capable previews it also supports host-side extra shortlist heads plus route-aligned retained-shortlist union and confidence reranking. Extra shortlist heads and retained-shortlist union reranking currently require include_all=true, retained-shortlist confidence reranking requires top_k > 0, include-all is meaningful and not free, and this route does not emit the Double MA heatmap or per-pair CSV export. Explicit Full17 and Trade22 metric-contract requests are available on both CPU-backed and standalone GpuKernel previews; Trade22 on standalone GpuKernel uses bounded host-side enrichment after GPU search.`
        : supportsRequestedCore17MetricSurface(kind, inputs)
          ? `${kind} supports Top-K and include-all native candidate returns, but does not emit the Double MA heatmap or per-pair CSV export. Include-all remains meaningful and not free on this surface. Explicit Full17 and Trade22 metric-contract requests are available on this CPU-backed route.`
          : `${kind} supports Top-K and include-all native candidate returns, but does not emit the Double MA heatmap or per-pair CSV export. Include-all remains meaningful and not free on this surface.`,
    }
  }
  if (kind === 'CompositeCpu') {
    return {
      kind,
      usesGlobalBackend: false,
      usesGlobalMode: false,
      supportsHeatmap: false,
      supportsExportAll: false,
      supportsTopK: true,
      supportsIncludeAll: true,
      supportsExtraShortlistObjectives: false,
      supportsTradeMetrics: true,
      supportsCore17MetricSurface: true,
      supportsMetricContract: true,
      supportsShortlistConfidenceMode: false,
      supportsShortlistUnionMode: false,
      supportsAdvancedExactKernelControls: false,
      executionSummary:
        'CompositeCpu uses the strategy-local composite CPU optimization path, keeps family-specific route diagnostics close to execution, and feeds candidate-detail/local-stability follow-ups from the same request family.',
      modeSummary:
        `${kind} always runs in Grid mode and keeps route-diagnostic, candidate-detail, and local-stability follow-ups on its fixed composite path.`,
      outputSummary:
        `${kind} supports ranked Top-K and include-all native candidate returns plus route-diagnostic-aware candidate detail and local-stability follow-ups for the selected candidate, but does not emit the Double MA heatmap or CSV export. Explicit Full17 and Trade22 metric-contract requests are available on this route.`,
    }
  }
  if (kind === 'CompositeMo') {
    return {
      kind,
      usesGlobalBackend: false,
      usesGlobalMode: false,
      supportsHeatmap: false,
      supportsExportAll: false,
      supportsTopK: false,
      supportsIncludeAll: false,
      supportsExtraShortlistObjectives: false,
      supportsTradeMetrics: true,
      supportsCore17MetricSurface: true,
      supportsMetricContract: false,
      supportsShortlistConfidenceMode: false,
      supportsShortlistUnionMode: false,
      supportsAdvancedExactKernelControls: false,
      executionSummary:
        'Composite MO runs a CPU-side multi-objective evaluator with explicit sampler families (Nsga2, Random, Tpe, and Qmc). An optional GPU prescan can narrow MA domains before the CPU search begins, and tiny discrete spaces can collapse into bounded exact-grid evaluation.',
      modeSummary:
        'Composite MO does not use the global backend selector or the global optimization-mode selector. Prescan mode only affects the stage-0 Double-MA narrowing pass, while sampler choice and search-space shape stay inside the Composite MO contract.',
      outputSummary:
        'Composite MO returns Pareto points from its own search-space, sampler, and prescan settings rather than Double-MA Top-K, heatmap, include-all, or export-all outputs. Saved optimization artifacts on this audited route emit the canonical Trade22 surface for Pareto points, but there is no explicit metric-contract selector on this workflow. Validation currently uses the CPU route; GPU fixed-candidate validation only supports plain CompositeCpu HardGate candidates.',
    }
  }
  return {
    kind,
    usesGlobalBackend: false,
    usesGlobalMode: false,
    supportsHeatmap: false,
    supportsExportAll: false,
    supportsTopK: true,
    supportsIncludeAll: true,
    supportsExtraShortlistObjectives: false,
    supportsTradeMetrics: false,
    supportsCore17MetricSurface: false,
    supportsMetricContract: false,
    supportsShortlistConfidenceMode: false,
    supportsShortlistUnionMode: false,
    supportsAdvancedExactKernelControls: false,
    executionSummary: 'This strategy uses a fixed execution path.',
    modeSummary: 'This strategy does not expose the global mode selector.',
    outputSummary: 'Only strategy-supported result shaping controls are shown.',
  }
}
