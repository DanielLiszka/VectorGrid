export type StrategyOptimizationKind =
  | 'DoubleMa'
  | 'DynamicDoubleMa'
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
  executionSummary: string
  modeSummary: string
  outputSummary: string
}

function normalizeKind(value: unknown): StrategyOptimizationKind {
  const kind = String(value || '').trim()
  switch (kind) {
    case 'DoubleMa':
    case 'DynamicDoubleMa':
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

export function describeStrategyOptimizationCapabilities(value: unknown): StrategyOptimizationCapabilities {
  const kind = normalizeKind(value)
  if (
    kind === 'DoubleMa'
    || kind === 'DynamicDoubleMa'
    || kind === 'Vfct'
    || kind === 'CompositeTrend'
    || kind === 'RegimeHybrid'
    || kind === 'BreakoutMomentum'
  ) {
    return {
      kind,
      usesGlobalBackend: true,
      usesGlobalMode: true,
      supportsHeatmap: true,
      supportsExportAll: true,
      supportsTopK: true,
      supportsIncludeAll: true,
      executionSummary:
        kind === 'DynamicDoubleMa'
          ? 'DynamicDoubleMa is the constrained Double-MA builder path. It validates a previewed strategy_ir, previews GPU-unified eligibility and explicit CPU fallback reasons, and then runs through the normal optimizer path.'
          : kind === 'Vfct'
          ? 'VFCT reuses the double-MA optimizer with volume-backed confirmation filters and unified GPU routing where supported.'
          : kind === 'CompositeTrend'
            ? 'CompositeTrend runs through the double-MA optimizer. ADX-off uses the GPU kernel path where supported; ADX-on uses the hybrid GPU route.'
            : kind === 'RegimeHybrid'
              ? 'RegimeHybrid runs through the double-MA optimizer and currently targets the hybrid GPU route because its active regime logic is ADX-bearing.'
              : kind === 'BreakoutMomentum'
                ? 'BreakoutMomentum runs through the double-MA optimizer. ADX-off uses the GPU kernel path where supported; ADX-on uses the hybrid GPU route.'
          : 'Double MA supports CPU, Auto, and unified GPU routing.',
      modeSummary:
        kind === 'DynamicDoubleMa'
          ? 'DynamicDoubleMa uses the normal optimization-mode selector after builder preview. Exact-grid outputs still require Grid.'
          : kind === 'Vfct'
          ? 'VFCT uses the Double-MA mode selector. Exact-grid outputs still require Grid.'
          : kind === 'CompositeTrend' || kind === 'RegimeHybrid' || kind === 'BreakoutMomentum'
            ? `${kind} uses the Double-MA mode selector. Exact-grid outputs still require Grid.`
          : 'Double MA uses the global mode selector. Exact-grid outputs still require Grid.',
      outputSummary:
        kind === 'DynamicDoubleMa'
          ? 'DynamicDoubleMa builds constrained strategy_ir payloads, previews the resolved backend, and runs through the normal optimizer path after validation.'
          : kind === 'Vfct'
          ? 'Top-K, heatmap, include-all, and export controls apply to VFCT runs through the double-MA backend.'
          : kind === 'CompositeTrend' || kind === 'RegimeHybrid' || kind === 'BreakoutMomentum'
            ? `Top-K, heatmap, include-all, and export controls apply to ${kind} through the double-MA backend.`
          : 'Top-K, heatmap, include-all, and export controls all apply to Double MA runs.',
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
    return {
      kind,
      usesGlobalBackend: true,
      usesGlobalMode: false,
      supportsHeatmap: false,
      supportsExportAll: false,
      supportsTopK: true,
      supportsIncludeAll: true,
      executionSummary: kind === 'Mkcs'
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
      outputSummary: `${kind} returns best/top rows, but does not emit the Double MA heatmap or per-pair CSV export.`,
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
      executionSummary: 'CompositeCpu uses the composite CPU optimization path.',
      modeSummary: `${kind} always runs in Grid mode.`,
      outputSummary: `${kind} returns ranked results, but does not emit the Double MA heatmap or CSV export.`,
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
      executionSummary: 'Composite MO manages execution through its own sampler and optional GPU prescan settings.',
      modeSummary: 'Composite MO does not use the global optimization-mode selector.',
      outputSummary: 'Composite MO uses its own search-space and prescan settings instead of Top-K, heatmap, include-all, or export-all shaping.',
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
    executionSummary: 'This strategy uses a fixed execution path.',
    modeSummary: 'This strategy does not expose the global mode selector.',
    outputSummary: 'Only strategy-supported result shaping controls are shown.',
  }
}
