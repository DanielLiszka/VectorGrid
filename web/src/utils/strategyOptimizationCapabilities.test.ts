import { describe, expect, it } from 'vitest'
import { describeStrategyOptimizationCapabilities } from './strategyOptimizationCapabilities'

describe('strategyOptimizationCapabilities', () => {
  it('describes the staged GPU-native expansion kinds with the composite optimizer contract', () => {
    for (const kind of ['Kqsr', 'Carm', 'Kvdf', 'Cdmc'] as const) {
      const caps = describeStrategyOptimizationCapabilities(kind)
      expect(caps.kind).toBe(kind)
      expect(caps.usesGlobalBackend).toBe(true)
      expect(caps.usesGlobalMode).toBe(false)
      expect(caps.supportsHeatmap).toBe(false)
      expect(caps.supportsExportAll).toBe(false)
      expect(caps.supportsTopK).toBe(true)
      expect(caps.supportsIncludeAll).toBe(true)
      expect(caps.executionSummary).toContain('GpuKernel')
      expect(caps.executionSummary).toContain('not implemented yet')
    }
  })
})
