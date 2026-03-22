import { describe, expect, it } from 'vitest'
import { describeStrategyOptimizationCapabilities } from './strategyOptimizationCapabilities'

describe('strategyOptimizationCapabilities', () => {
  it('describes KQSR, CARM, KVDF, and CDMC as real GPU-capable composite optimizer routes', () => {
    for (const kind of ['Kqsr', 'Carm', 'Kvdf', 'Cdmc'] as const) {
      const caps = describeStrategyOptimizationCapabilities(kind)
      expect(caps.kind).toBe(kind)
      expect(caps.usesGlobalBackend).toBe(true)
      expect(caps.usesGlobalMode).toBe(false)
      expect(caps.executionSummary).toContain('CPU')
      expect(caps.executionSummary).toContain('GPU')
      expect(caps.executionSummary).not.toContain('not implemented yet')
    }
  })

  it('describes DynamicDoubleMa as constrained builder-first dynamic GPU exposure', () => {
    const caps = describeStrategyOptimizationCapabilities('DynamicDoubleMa')
    expect(caps.kind).toBe('DynamicDoubleMa')
    expect(caps.usesGlobalBackend).toBe(true)
    expect(caps.usesGlobalMode).toBe(true)
    expect(caps.supportsHeatmap).toBe(true)
    expect(caps.supportsExportAll).toBe(true)
    expect(caps.executionSummary).toContain('runs through the normal optimizer path')
    expect(caps.executionSummary).toContain('GPU')
    expect(caps.outputSummary).toContain('runs through the normal optimizer path')
  })

  it('describes the full double-MA family instead of classifying overlay kinds as unknown', () => {
    for (const kind of ['DoubleMa', 'Pit', 'Set', 'Mts', 'Orc', 'Marrt', 'Vfct'] as const) {
      const caps = describeStrategyOptimizationCapabilities(kind)
      expect(caps.kind).toBe(kind)
      expect(caps.usesGlobalBackend).toBe(true)
      expect(caps.usesGlobalMode).toBe(true)
      expect(caps.supportsHeatmap).toBe(true)
      expect(caps.supportsExportAll).toBe(true)
      expect(caps.supportsTopK).toBe(true)
      expect(caps.supportsIncludeAll).toBe(true)
    }
  })

  it('describes Double-MA family mode selection without claiming grid-only execution', () => {
    for (const kind of ['DoubleMa', 'Vfct', 'CompositeTrend', 'RegimeHybrid', 'BreakoutMomentum'] as const) {
      const caps = describeStrategyOptimizationCapabilities(kind)
      expect(caps.usesGlobalMode).toBe(true)
      expect(caps.modeSummary).toContain('mode selector')
      expect(caps.modeSummary).toContain('Grid')
      expect(caps.modeSummary).not.toContain('exact Grid search')
    }
  })

  it('makes composite export support follow the current route preview', () => {
    const kernelCaps = describeStrategyOptimizationCapabilities('CompositeTrend', {
      requestedBackend: 'GpuKernel',
      trendUseAdx: false,
    })
    const hybridCaps = describeStrategyOptimizationCapabilities('CompositeTrend', {
      requestedBackend: 'GpuKernel',
      trendUseAdx: true,
    })
    const cpuCaps = describeStrategyOptimizationCapabilities('RegimeHybrid', {
      requestedBackend: 'CpuOnly',
    })

    expect(kernelCaps.supportsExportAll).toBe(true)
    expect(kernelCaps.outputSummary).toContain('export controls apply')
    expect(hybridCaps.supportsExportAll).toBe(false)
    expect(hybridCaps.supportsHeatmap).toBe(true)
    expect(hybridCaps.supportsTopK).toBe(true)
    expect(hybridCaps.supportsIncludeAll).toBe(true)
    expect(cpuCaps.supportsExportAll).toBe(false)
    expect(cpuCaps.supportsHeatmap).toBe(true)
    expect(cpuCaps.supportsTopK).toBe(true)
    expect(cpuCaps.supportsIncludeAll).toBe(true)
  })
})
