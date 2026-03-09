export type StrategyOptimizationKind =
  | 'DoubleMa'
  | 'Vfct'
  | 'Asr'
  | 'Vcc'
  | 'Vemr'
  | 'Rocb'
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
    case 'Vfct':
    case 'Asr':
    case 'Vcc':
    case 'Vemr':
    case 'Rocb':
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
  if (kind === 'DoubleMa' || kind === 'Vfct') {
    return {
      kind,
      usesGlobalBackend: true,
      usesGlobalMode: true,
      supportsHeatmap: true,
      supportsExportAll: true,
      supportsTopK: true,
      supportsIncludeAll: true,
      executionSummary:
        kind === 'Vfct'
          ? 'VFCT reuses the double-MA optimizer with volume-backed confirmation filters and unified GPU routing where supported.'
          : 'Double MA supports CPU, Auto, and unified GPU routing.',
      modeSummary:
        kind === 'Vfct'
          ? 'VFCT supports the same Grid, Coarse-to-Fine, GPU VDPS, and Auto mode selection used by the double-MA backend.'
          : 'Double MA supports Grid, Coarse-to-Fine, GPU VDPS, and Auto mode selection.',
      outputSummary:
        kind === 'Vfct'
          ? 'Top-K, heatmap, include-all, and export controls apply to VFCT runs through the double-MA backend.'
          : 'Top-K, heatmap, include-all, and export controls all apply to Double MA runs.',
    }
  }
  if (kind === 'Asr' || kind === 'Vcc' || kind === 'Vemr' || kind === 'Rocb') {
    return {
      kind,
      usesGlobalBackend: true,
      usesGlobalMode: false,
      supportsHeatmap: false,
      supportsExportAll: false,
      supportsTopK: true,
      supportsIncludeAll: true,
      executionSummary: `${kind} can use CPU or unified GPU execution, but optimization mode is fixed to Grid.`,
      modeSummary: `${kind} always runs in Grid mode.`,
      outputSummary: `${kind} returns best/top rows, but does not emit the Double MA heatmap or per-pair CSV export.`,
    }
  }
  if (
    kind === 'CompositeCpu'
    || kind === 'CompositeTrend'
    || kind === 'RegimeHybrid'
    || kind === 'BreakoutMomentum'
  ) {
    return {
      kind,
      usesGlobalBackend: false,
      usesGlobalMode: false,
      supportsHeatmap: false,
      supportsExportAll: false,
      supportsTopK: true,
      supportsIncludeAll: true,
      executionSummary: `${kind} uses the CPU optimization path.`,
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
