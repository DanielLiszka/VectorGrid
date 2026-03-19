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

  it('describes Double-MA family mode selection without claiming grid-only execution', () => {
    for (const kind of ['DoubleMa', 'Vfct', 'CompositeTrend', 'RegimeHybrid', 'BreakoutMomentum'] as const) {
      const caps = describeStrategyOptimizationCapabilities(kind)
      expect(caps.usesGlobalMode).toBe(true)
      expect(caps.modeSummary).toContain('mode selector')
      expect(caps.modeSummary).toContain('Grid')
      expect(caps.modeSummary).not.toContain('exact Grid search')
    }
  })
})
