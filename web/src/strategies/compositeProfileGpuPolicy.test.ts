import { describe, expect, it } from 'vitest'
import {
  describeCompositeProfileGpuPolicy,
  resolveCompositeProfileGpuTarget,
} from './compositeProfileGpuPolicy'

describe('resolveCompositeProfileGpuTarget', () => {
  it('routes CompositeTrend by ADX flag', () => {
    expect(resolveCompositeProfileGpuTarget('CompositeTrend', { trendUseAdx: false })).toBe('GpuKernel')
    expect(resolveCompositeProfileGpuTarget('CompositeTrend', { trendUseAdx: true })).toBe('GpuHybridKernel')
  })

  it('routes BreakoutMomentum by ADX flag', () => {
    expect(resolveCompositeProfileGpuTarget('BreakoutMomentum', { breakoutUseAdx: false })).toBe('GpuKernel')
    expect(resolveCompositeProfileGpuTarget('BreakoutMomentum', { breakoutUseAdx: true })).toBe('GpuHybridKernel')
  })

  it('routes RegimeHybrid to hybrid in current form', () => {
    expect(resolveCompositeProfileGpuTarget('RegimeHybrid')).toBe('GpuHybridKernel')
  })

  it('describes the route policy', () => {
    expect(describeCompositeProfileGpuPolicy('CompositeTrend')).toContain('GpuKernel')
    expect(describeCompositeProfileGpuPolicy('BreakoutMomentum')).toContain('GpuHybridKernel')
    expect(describeCompositeProfileGpuPolicy('RegimeHybrid')).toContain('GpuHybridKernel')
  })
})
