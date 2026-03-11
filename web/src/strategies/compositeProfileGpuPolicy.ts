export type CompositeProfileKind =
  | 'CompositeTrend'
  | 'RegimeHybrid'
  | 'BreakoutMomentum'

export type CompositeProfileGpuTarget =
  | 'CpuOnly'
  | 'GpuKernel'
  | 'GpuHybridKernel'

export interface CompositeProfileGpuInputs {
  trendUseAdx?: boolean
  breakoutUseAdx?: boolean
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

export function describeCompositeProfileGpuPolicy(kind: CompositeProfileKind): string {
  if (kind === 'CompositeTrend') {
    return 'CompositeTrend targets GpuKernel when ADX is off, GpuHybridKernel when ADX is on, with CpuOnly fallback.'
  }
  if (kind === 'BreakoutMomentum') {
    return 'BreakoutMomentum targets GpuKernel when ADX is off, GpuHybridKernel when ADX is on, with CpuOnly fallback.'
  }
  return 'RegimeHybrid targets GpuHybridKernel in its current ADX-bearing form, with CpuOnly fallback and future GpuKernel support only if a true no-ADX form is implemented.'
}
