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
  return String(value ?? '').trim() === 'GpuKernel'
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
      supportsShortlistConfidenceMode: true,
      supportsShortlistUnionMode: true,
      supportsAdvancedExactKernelControls: true,
      outputSummary: `Top-K, heatmap, include-all, export, and advanced exact-kernel shortlist/score/trade-metric controls apply to ${kind} through the current kernel-targeted double-MA route.`,
    }
  }

  return {
    supportsExportAll: false,
    supportsExtraShortlistObjectives: false,
    supportsTradeMetrics: false,
    supportsShortlistConfidenceMode: false,
    supportsShortlistUnionMode: false,
    supportsAdvancedExactKernelControls: false,
    outputSummary:
      route.targetBackend === 'CpuOnly'
        ? `Top-K, heatmap, and include-all controls apply to ${kind} through the current CPU double-MA route. Export-all and advanced exact-kernel shortlist/score/trade-metric controls are disabled on this route.`
        : `Top-K, heatmap, and include-all controls apply to ${kind} through the current hybrid double-MA route. Export-all and advanced exact-kernel shortlist/score/trade-metric controls are disabled on this route.`,
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
          supportsTradeMetrics:
            kind === 'DynamicDoubleMa'
              ? requestedGlobalBackendIsGpuKernel(inputs.requestedBackend)
              : supportsRequestedExactKernelControls(kind, inputs),
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
                + ' Trade metrics follow that same exact-compatible dynamic GpuKernel path.'
                + ' Shortlist confidence mode follows that same exact-compatible dynamic GpuKernel path.'
                + ' Shortlist union mode follows that same exact-compatible dynamic GpuKernel path once extra shortlist heads are present.'
                + ' Shortlist slice and neighborhood diagnostics follow that same exact-compatible dynamic GpuKernel path.'
              : kind === 'Vfct'
                ? 'Top-K, heatmap, include-all, and export controls apply to VFCT runs through the double-MA backend.'
                : kind === 'DoubleMa'
                  ? 'Top-K, heatmap, include-all, and export controls all apply to Double MA runs.'
                  : `Top-K, heatmap, include-all, and export controls apply to ${kind} through the double-MA backend.`,
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
      supportsTradeMetrics: false,
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
        ? `${kind} supports Top-K and include-all native candidate returns. On the requested standalone GpuKernel route it also supports host-side extra shortlist heads, union reranking, and retained-shortlist confidence reranking, but it does not emit the Double MA heatmap or per-pair CSV export.`
        : `${kind} supports Top-K and include-all native candidate returns, but does not emit the Double MA heatmap or per-pair CSV export.`,
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
      supportsTradeMetrics: false,
      supportsShortlistConfidenceMode: false,
      supportsShortlistUnionMode: false,
      supportsAdvancedExactKernelControls: false,
      executionSummary:
        'CompositeCpu uses the composite CPU optimization path.',
      modeSummary: `${kind} always runs in Grid mode.`,
      outputSummary: `${kind} supports ranked Top-K and include-all native candidate returns, but does not emit the Double MA heatmap or CSV export.`,
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
      supportsTradeMetrics: false,
      supportsShortlistConfidenceMode: false,
      supportsShortlistUnionMode: false,
      supportsAdvancedExactKernelControls: false,
      executionSummary:
        'Composite MO manages execution through its own sampler and optional GPU prescan settings.',
      modeSummary:
        'Composite MO does not use the global optimization-mode selector.',
      outputSummary:
        'Composite MO uses its own search-space and prescan settings instead of Top-K, heatmap, include-all, or export-all shaping.',
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
    supportsShortlistConfidenceMode: false,
    supportsShortlistUnionMode: false,
    supportsAdvancedExactKernelControls: false,
    executionSummary: 'This strategy uses a fixed execution path.',
    modeSummary: 'This strategy does not expose the global mode selector.',
    outputSummary: 'Only strategy-supported result shaping controls are shown.',
  }
}
