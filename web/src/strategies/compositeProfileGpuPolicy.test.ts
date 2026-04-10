import { describe, expect, it } from 'vitest'
import {
  describeCompositeProfileGpuPolicy,
  normalizeCompositeRequestedBackend,
  resolveCompositeProfileGpuTarget,
  resolveCompositeProfileRoute,
} from './compositeProfileGpuPolicy'

describe('compositeProfileGpuPolicy', () => {
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

  it('normalizes requested backend aliases without changing the supported vocabulary', () => {
    expect(normalizeCompositeRequestedBackend('GpuOnly')).toBe('GpuKernel')
    expect(normalizeCompositeRequestedBackend('GpuHybridKernel')).toBe('GpuKernel')
    expect(normalizeCompositeRequestedBackend('')).toBe('Auto')
    expect(normalizeCompositeRequestedBackend('mystery')).toBe('Auto')
  })

  it('resolves composite route previews without dropping CPU heatmap/top-k/include-all support', () => {
    expect(resolveCompositeProfileRoute('CompositeTrend', 'CpuOnly')).toMatchObject({
      targetBackend: 'CpuOnly',
      supportsExportAll: false,
      supportsHeatmap: true,
      supportsTopK: true,
      supportsIncludeAll: true,
    })
    expect(resolveCompositeProfileRoute('CompositeTrend', 'GpuKernel', { trendUseAdx: false })).toMatchObject({
      targetBackend: 'GpuKernel',
      supportsExportAll: true,
      usesHybrid: false,
    })
    expect(resolveCompositeProfileRoute('CompositeTrend', 'Auto', { trendUseAdx: true })).toMatchObject({
      targetBackend: 'GpuHybridKernel',
      supportsExportAll: false,
      usesHybrid: true,
    })
  })

  it('keeps the three composite profile route previews aligned with the current product route matrix', () => {
    expect(resolveCompositeProfileRoute('CompositeTrend', 'GpuKernel', { trendUseAdx: false })).toMatchObject({
      targetBackend: 'GpuKernel',
      usesHybrid: false,
      supportsExportAll: true,
    })
    expect(resolveCompositeProfileRoute('CompositeTrend', 'GpuKernel', { trendUseAdx: true })).toMatchObject({
      targetBackend: 'GpuHybridKernel',
      usesHybrid: true,
      supportsExportAll: false,
    })
    expect(resolveCompositeProfileRoute('BreakoutMomentum', 'GpuKernel', { breakoutUseAdx: false })).toMatchObject({
      targetBackend: 'GpuKernel',
      usesHybrid: false,
      supportsExportAll: true,
    })
    expect(resolveCompositeProfileRoute('BreakoutMomentum', 'GpuKernel', { breakoutUseAdx: true })).toMatchObject({
      targetBackend: 'GpuHybridKernel',
      usesHybrid: true,
      supportsExportAll: false,
    })
    expect(resolveCompositeProfileRoute('RegimeHybrid', 'GpuKernel')).toMatchObject({
      targetBackend: 'GpuHybridKernel',
      usesHybrid: true,
      supportsExportAll: false,
    })
    expect(resolveCompositeProfileRoute('RegimeHybrid', 'CpuOnly')).toMatchObject({
      targetBackend: 'CpuOnly',
      usesHybrid: false,
      supportsExportAll: false,
      supportsHeatmap: true,
      supportsTopK: true,
      supportsIncludeAll: true,
    })
  })

  it('describes the route policy', () => {
    expect(describeCompositeProfileGpuPolicy('CompositeTrend')).toContain('GpuKernel')
    expect(describeCompositeProfileGpuPolicy('BreakoutMomentum')).toContain('GpuHybridKernel')
    expect(describeCompositeProfileGpuPolicy('RegimeHybrid')).toContain('GpuHybridKernel')
  })
})
