export type CompositeProfileKind =
  | 'CompositeTrend'
  | 'RegimeHybrid'
  | 'BreakoutMomentum'

export type CompositeProfileGpuTarget =
  | 'CpuOnly'
  | 'GpuKernel'
  | 'GpuHybridKernel'

export type CompositeProfileRequestedBackend =
  | 'Auto'
  | 'CpuOnly'
  | 'GpuKernel'

export interface CompositeProfileGpuInputs {
  trendUseAdx?: boolean
  breakoutUseAdx?: boolean
}

export interface CompositeProfileRoutePreview {
  kind: CompositeProfileKind
  requestedBackend: CompositeProfileRequestedBackend
  targetBackend: CompositeProfileGpuTarget
  usesHybrid: boolean
  supportsExportAll: boolean
  supportsHeatmap: boolean
  supportsTopK: boolean
  supportsIncludeAll: boolean
}

export function isCompositeProfileKind(value: unknown): value is CompositeProfileKind {
  return value === 'CompositeTrend'
    || value === 'RegimeHybrid'
    || value === 'BreakoutMomentum'
}

export function normalizeCompositeRequestedBackend(
  value: unknown,
  fallback: CompositeProfileRequestedBackend = 'Auto',
): CompositeProfileRequestedBackend {
  const text = String(value ?? '').trim()
  if (!text) return fallback
  if (text === 'CpuOnly') return 'CpuOnly'
  if (text === 'Auto') return 'Auto'
  if (text === 'GpuOnly' || text === 'GpuHybridKernel' || text === 'GpuKernel') {
    return 'GpuKernel'
  }
  return fallback
}

export function resolveCompositeProfileGpuTarget(
  kind: CompositeProfileKind,
  inputs: CompositeProfileGpuInputs = {},
): CompositeProfileGpuTarget {
  if (kind === 'CompositeTrend') {
    return inputs.trendUseAdx ? 'GpuHybridKernel' : 'GpuKernel'
  }
  if (kind === 'BreakoutMomentum') {
    return inputs.breakoutUseAdx ? 'GpuHybridKernel' : 'GpuKernel'
  }
  return 'GpuHybridKernel'
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
      targetBackend: 'CpuOnly',
      usesHybrid: false,
      supportsExportAll: false,
      supportsHeatmap: true,
      supportsTopK: true,
      supportsIncludeAll: true,
    }
  }

  const targetBackend = resolveCompositeProfileGpuTarget(kind, inputs)
  return {
    kind,
    requestedBackend,
    targetBackend,
    usesHybrid: targetBackend === 'GpuHybridKernel',
    supportsExportAll: targetBackend === 'GpuKernel',
    supportsHeatmap: true,
    supportsTopK: true,
    supportsIncludeAll: true,
  }
}

export function describeCompositeProfileGpuPolicy(kind: CompositeProfileKind): string {
  if (kind === 'CompositeTrend') {
    return 'CompositeTrend targets GpuKernel when ADX is off, GpuHybridKernel when ADX is on, with CpuOnly fallback.'
  }
  if (kind === 'BreakoutMomentum') {
    return 'BreakoutMomentum targets GpuKernel when ADX is off, GpuHybridKernel when ADX is on, with CpuOnly fallback.'
  }
  return 'RegimeHybrid targets GpuHybridKernel in its current ADX-bearing form, with CpuOnly fallback and future GpuKernel support only if a true no-ADX form is implemented.'
}
