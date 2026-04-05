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
    expect(caps.supportsExtraShortlistObjectives).toBe(false)
    expect(caps.supportsShortlistConfidenceMode).toBe(false)
    expect(caps.supportsShortlistUnionMode).toBe(false)
    expect(caps.supportsAdvancedExactKernelControls).toBe(false)
    expect(caps.executionSummary).toContain('runs through the normal optimizer path')
    expect(caps.executionSummary).toContain('GPU')
    expect(caps.outputSummary).toContain('runs through the normal optimizer path')
    expect(caps.outputSummary).toContain('Explicit 17-row and 22-row metric-contract overrides are available')
  })

  it('describes the full double-MA family instead of classifying overlay kinds as unknown', () => {
    for (const kind of ['DoubleMa', 'Pit', 'Set', 'Mts', 'Orc', 'Marrt', 'Vfct'] as const) {
      const caps = describeStrategyOptimizationCapabilities(kind, {
        requestedBackend: 'GpuKernel',
      })
      expect(caps.kind).toBe(kind)
      expect(caps.usesGlobalBackend).toBe(true)
      expect(caps.usesGlobalMode).toBe(true)
      expect(caps.supportsHeatmap).toBe(true)
      expect(caps.supportsExportAll).toBe(true)
      expect(caps.supportsTopK).toBe(true)
      expect(caps.supportsIncludeAll).toBe(true)
      expect(caps.supportsExtraShortlistObjectives).toBe(true)
      expect(caps.supportsCore17MetricSurface).toBe(true)
      expect(caps.supportsMetricContract).toBe(true)
      expect(caps.supportsShortlistConfidenceMode).toBe(true)
      expect(caps.supportsShortlistUnionMode).toBe(true)
      expect(caps.supportsAdvancedExactKernelControls).toBe(true)
    }
  })

  it('makes non-composite Double-MA advanced controls follow the requested backend preview', () => {
    const kernelCaps = describeStrategyOptimizationCapabilities('DoubleMa', {
      requestedBackend: 'GpuKernel',
    })
    const autoCaps = describeStrategyOptimizationCapabilities('DoubleMa', {
      requestedBackend: 'Auto',
    })
    const cpuCaps = describeStrategyOptimizationCapabilities('DoubleMa', {
      requestedBackend: 'CpuOnly',
    })
    const dynamicKernelCaps = describeStrategyOptimizationCapabilities('DynamicDoubleMa', {
      requestedBackend: 'GpuKernel',
    })

    expect(kernelCaps.supportsExtraShortlistObjectives).toBe(true)
    expect(kernelCaps.supportsCore17MetricSurface).toBe(true)
    expect(kernelCaps.supportsMetricContract).toBe(true)
    expect(kernelCaps.supportsShortlistConfidenceMode).toBe(true)
    expect(kernelCaps.supportsShortlistUnionMode).toBe(true)
    expect(kernelCaps.supportsAdvancedExactKernelControls).toBe(true)
    expect(autoCaps.supportsExtraShortlistObjectives).toBe(false)
    expect(autoCaps.supportsTradeMetrics).toBe(true)
    expect(autoCaps.supportsCore17MetricSurface).toBe(true)
    expect(autoCaps.supportsMetricContract).toBe(true)
    expect(autoCaps.supportsShortlistConfidenceMode).toBe(false)
    expect(autoCaps.supportsShortlistUnionMode).toBe(false)
    expect(autoCaps.supportsAdvancedExactKernelControls).toBe(false)
    expect(cpuCaps.supportsExtraShortlistObjectives).toBe(false)
    expect(cpuCaps.supportsTradeMetrics).toBe(true)
    expect(cpuCaps.supportsCore17MetricSurface).toBe(true)
    expect(cpuCaps.supportsMetricContract).toBe(true)
    expect(cpuCaps.supportsShortlistConfidenceMode).toBe(false)
    expect(cpuCaps.supportsShortlistUnionMode).toBe(false)
    expect(cpuCaps.supportsAdvancedExactKernelControls).toBe(false)
    expect(dynamicKernelCaps.supportsExtraShortlistObjectives).toBe(true)
    expect(dynamicKernelCaps.supportsTradeMetrics).toBe(true)
    expect(dynamicKernelCaps.supportsCore17MetricSurface).toBe(true)
    expect(dynamicKernelCaps.supportsMetricContract).toBe(true)
    expect(dynamicKernelCaps.supportsShortlistConfidenceMode).toBe(true)
    expect(dynamicKernelCaps.supportsShortlistUnionMode).toBe(true)
    expect(dynamicKernelCaps.supportsAdvancedExactKernelControls).toBe(true)

    const dynamicCpuCaps = describeStrategyOptimizationCapabilities('DynamicDoubleMa', {
      requestedBackend: 'CpuOnly',
    })
    expect(dynamicCpuCaps.supportsTradeMetrics).toBe(true)
    expect(dynamicCpuCaps.supportsCore17MetricSurface).toBe(true)
    expect(dynamicCpuCaps.supportsMetricContract).toBe(true)
    expect(dynamicCpuCaps.supportsShortlistConfidenceMode).toBe(false)
    expect(dynamicCpuCaps.supportsShortlistUnionMode).toBe(false)
    expect(dynamicCpuCaps.supportsAdvancedExactKernelControls).toBe(false)
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
    expect(kernelCaps.supportsExtraShortlistObjectives).toBe(true)
    expect(kernelCaps.supportsCore17MetricSurface).toBe(true)
    expect(kernelCaps.supportsMetricContract).toBe(true)
    expect(kernelCaps.supportsShortlistConfidenceMode).toBe(true)
    expect(kernelCaps.supportsShortlistUnionMode).toBe(true)
    expect(kernelCaps.supportsAdvancedExactKernelControls).toBe(true)
    expect(kernelCaps.outputSummary).toContain('export')
    expect(kernelCaps.outputSummary).toContain('advanced exact-kernel shortlist/score/metric-contract controls apply')
    expect(hybridCaps.supportsExportAll).toBe(false)
    expect(hybridCaps.supportsHeatmap).toBe(true)
    expect(hybridCaps.supportsTopK).toBe(true)
    expect(hybridCaps.supportsIncludeAll).toBe(true)
    expect(hybridCaps.supportsExtraShortlistObjectives).toBe(false)
    expect(hybridCaps.supportsTradeMetrics).toBe(true)
    expect(hybridCaps.supportsCore17MetricSurface).toBe(true)
    expect(hybridCaps.supportsMetricContract).toBe(true)
    expect(hybridCaps.supportsShortlistConfidenceMode).toBe(false)
    expect(hybridCaps.supportsShortlistUnionMode).toBe(false)
    expect(hybridCaps.supportsAdvancedExactKernelControls).toBe(false)
    expect(hybridCaps.outputSummary).toContain('Explicit Full17 and Trade22 metric-contract requests are available')
    expect(hybridCaps.outputSummary).toContain('advanced exact-kernel shortlist/score controls remain disabled')
    expect(cpuCaps.supportsExportAll).toBe(false)
    expect(cpuCaps.supportsHeatmap).toBe(true)
    expect(cpuCaps.supportsTopK).toBe(true)
    expect(cpuCaps.supportsIncludeAll).toBe(true)
    expect(cpuCaps.supportsExtraShortlistObjectives).toBe(false)
    expect(cpuCaps.supportsTradeMetrics).toBe(true)
    expect(cpuCaps.supportsCore17MetricSurface).toBe(true)
    expect(cpuCaps.supportsMetricContract).toBe(true)
    expect(cpuCaps.supportsShortlistConfidenceMode).toBe(false)
    expect(cpuCaps.supportsShortlistUnionMode).toBe(false)
    expect(cpuCaps.supportsAdvancedExactKernelControls).toBe(false)
    expect(cpuCaps.outputSummary).toContain('Explicit Full17 and Trade22 metric-contract requests are available')
    expect(cpuCaps.outputSummary).toContain('advanced exact-kernel shortlist/score controls remain disabled')
  })

  it('describes include-all native candidate return on standalone strategy routes and CompositeCpu', () => {
    for (const kind of ['Asr', 'Vcc', 'Vemr', 'Rocb', 'Vcet', 'Kqsr', 'Carm', 'Mkcs', 'Kvdf', 'Cdmc'] as const) {
      const caps = describeStrategyOptimizationCapabilities(kind)
      expect(caps.supportsIncludeAll).toBe(true)
      expect(caps.supportsExtraShortlistObjectives).toBe(false)
      expect(caps.supportsTradeMetrics).toBe(true)
      expect(caps.supportsCore17MetricSurface).toBe(true)
      expect(caps.supportsMetricContract).toBe(true)
      expect(caps.supportsShortlistConfidenceMode).toBe(false)
      expect(caps.supportsShortlistUnionMode).toBe(false)
      expect(caps.supportsAdvancedExactKernelControls).toBe(false)
      expect(caps.outputSummary).toContain('include-all')
      expect(caps.outputSummary).toContain('native candidate returns')
      expect(caps.outputSummary).toContain('Explicit Full17 and Trade22 metric-contract requests are available')
    }

    const compositeCpuCaps = describeStrategyOptimizationCapabilities('CompositeCpu')
    expect(compositeCpuCaps.supportsIncludeAll).toBe(true)
    expect(compositeCpuCaps.supportsExtraShortlistObjectives).toBe(false)
    expect(compositeCpuCaps.supportsTradeMetrics).toBe(true)
    expect(compositeCpuCaps.supportsCore17MetricSurface).toBe(true)
    expect(compositeCpuCaps.supportsMetricContract).toBe(true)
    expect(compositeCpuCaps.supportsShortlistConfidenceMode).toBe(false)
    expect(compositeCpuCaps.supportsShortlistUnionMode).toBe(false)
    expect(compositeCpuCaps.supportsAdvancedExactKernelControls).toBe(false)
    expect(compositeCpuCaps.outputSummary).toContain('include-all')
    expect(compositeCpuCaps.outputSummary).toContain('native candidate returns')
    expect(compositeCpuCaps.outputSummary).toContain('Explicit Full17 and Trade22 metric-contract requests are available')
  })

  it('enables standalone shortlist extras only on requested standalone GpuKernel previews', () => {
    for (const kind of ['Cdmc'] as const) {
      const kernelCaps = describeStrategyOptimizationCapabilities(kind, {
        requestedBackend: 'GpuKernel',
      })
      const cpuCaps = describeStrategyOptimizationCapabilities(kind, {
        requestedBackend: 'CpuOnly',
      })

      expect(kernelCaps.supportsIncludeAll).toBe(true)
      expect(kernelCaps.supportsExtraShortlistObjectives).toBe(true)
      expect(kernelCaps.supportsTradeMetrics).toBe(true)
      expect(kernelCaps.supportsCore17MetricSurface).toBe(true)
      expect(kernelCaps.supportsMetricContract).toBe(true)
      expect(kernelCaps.supportsShortlistConfidenceMode).toBe(true)
      expect(kernelCaps.supportsShortlistUnionMode).toBe(true)
      expect(kernelCaps.supportsAdvancedExactKernelControls).toBe(false)
      expect(kernelCaps.outputSummary).toContain('host-side extra shortlist heads')
      expect(kernelCaps.outputSummary).toContain('retained-shortlist confidence reranking')
      expect(kernelCaps.outputSummary).toContain('Trade22 on standalone GpuKernel uses bounded host-side enrichment after GPU search')

      expect(cpuCaps.supportsIncludeAll).toBe(true)
      expect(cpuCaps.supportsExtraShortlistObjectives).toBe(false)
      expect(cpuCaps.supportsTradeMetrics).toBe(true)
      expect(cpuCaps.supportsCore17MetricSurface).toBe(true)
      expect(cpuCaps.supportsMetricContract).toBe(true)
      expect(cpuCaps.supportsShortlistConfidenceMode).toBe(false)
      expect(cpuCaps.supportsShortlistUnionMode).toBe(false)
      expect(cpuCaps.supportsAdvancedExactKernelControls).toBe(false)
      expect(cpuCaps.outputSummary).toContain('Explicit Full17 and Trade22 metric-contract requests are available')
    }
  })

  it('treats ASR, VCC, VEMR, ROCB, VCET, KQSR, CARM, MKCS, KVDF, and CDMC as Core17-capable on both CPU and standalone GpuKernel previews', () => {
    for (const kind of ['Asr', 'Vcc', 'Vemr', 'Rocb', 'Vcet', 'Kqsr', 'Carm', 'Mkcs', 'Kvdf', 'Cdmc'] as const) {
      const kernelCaps = describeStrategyOptimizationCapabilities(kind, {
        requestedBackend: 'GpuKernel',
      })
      const cpuCaps = describeStrategyOptimizationCapabilities(kind, {
        requestedBackend: 'CpuOnly',
      })

      expect(kernelCaps.supportsIncludeAll).toBe(true)
      expect(kernelCaps.supportsExtraShortlistObjectives).toBe(true)
      expect(kernelCaps.supportsTradeMetrics).toBe(true)
      expect(kernelCaps.supportsCore17MetricSurface).toBe(true)
      expect(kernelCaps.supportsMetricContract).toBe(true)
      expect(kernelCaps.supportsShortlistConfidenceMode).toBe(true)
      expect(kernelCaps.supportsShortlistUnionMode).toBe(true)
      expect(kernelCaps.supportsAdvancedExactKernelControls).toBe(false)
      expect(kernelCaps.outputSummary).toContain('Trade22 on standalone GpuKernel uses bounded host-side enrichment after GPU search')

      expect(cpuCaps.supportsIncludeAll).toBe(true)
      expect(cpuCaps.supportsExtraShortlistObjectives).toBe(false)
      expect(cpuCaps.supportsTradeMetrics).toBe(true)
      expect(cpuCaps.supportsCore17MetricSurface).toBe(true)
      expect(cpuCaps.supportsMetricContract).toBe(true)
      expect(cpuCaps.supportsShortlistConfidenceMode).toBe(false)
      expect(cpuCaps.supportsShortlistUnionMode).toBe(false)
      expect(cpuCaps.supportsAdvancedExactKernelControls).toBe(false)
      expect(cpuCaps.outputSummary).toContain('Explicit Full17 and Trade22 metric-contract requests are available')
    }
  })

  it('treats legacy standalone GPU aliases as kernel-capable previews', () => {
    const gpuOnlyCaps = describeStrategyOptimizationCapabilities('Vemr', {
      requestedBackend: 'GpuOnly',
    })
    const hybridAliasCaps = describeStrategyOptimizationCapabilities('Vemr', {
      requestedBackend: 'GpuHybridKernel',
    })

    expect(gpuOnlyCaps.supportsExtraShortlistObjectives).toBe(true)
    expect(gpuOnlyCaps.supportsTradeMetrics).toBe(true)
    expect(gpuOnlyCaps.supportsCore17MetricSurface).toBe(true)
    expect(gpuOnlyCaps.supportsMetricContract).toBe(true)
    expect(gpuOnlyCaps.supportsShortlistConfidenceMode).toBe(true)
    expect(gpuOnlyCaps.supportsShortlistUnionMode).toBe(true)
    expect(gpuOnlyCaps.outputSummary).toContain('include_all=true')
    expect(gpuOnlyCaps.outputSummary).toContain('per-pair CSV export')
    expect(gpuOnlyCaps.outputSummary).toContain('Trade22 on standalone GpuKernel uses bounded host-side enrichment after GPU search')
    expect(hybridAliasCaps.supportsExtraShortlistObjectives).toBe(true)
    expect(hybridAliasCaps.supportsTradeMetrics).toBe(true)
    expect(hybridAliasCaps.supportsCore17MetricSurface).toBe(true)
    expect(hybridAliasCaps.supportsMetricContract).toBe(true)
    expect(hybridAliasCaps.supportsShortlistConfidenceMode).toBe(true)
    expect(hybridAliasCaps.supportsShortlistUnionMode).toBe(true)
    expect(hybridAliasCaps.outputSummary).toContain('include_all=true')
    expect(hybridAliasCaps.outputSummary).toContain('per-pair CSV export')
    expect(hybridAliasCaps.outputSummary).toContain('Trade22 on standalone GpuKernel uses bounded host-side enrichment after GPU search')
  })

  it('describes standalone shortlist prerequisites and export limits explicitly', () => {
    const kernelCaps = describeStrategyOptimizationCapabilities('Vemr', {
      requestedBackend: 'GpuOnly',
    })
    const cpuCaps = describeStrategyOptimizationCapabilities('Vemr', {
      requestedBackend: 'CpuOnly',
    })

    expect(kernelCaps.outputSummary).toContain('include_all=true')
    expect(kernelCaps.outputSummary).toContain('top_k > 0')
    expect(kernelCaps.outputSummary).toContain('per-pair CSV export')
    expect(kernelCaps.outputSummary).toContain('not emit the Double MA heatmap')
    expect(kernelCaps.outputSummary).toContain('Trade22 on standalone GpuKernel uses bounded host-side enrichment after GPU search')
    expect(cpuCaps.outputSummary).toContain('Include-all remains meaningful and not free')
    expect(cpuCaps.outputSummary).toContain('per-pair CSV export')
    expect(cpuCaps.outputSummary).toContain('Explicit Full17 and Trade22 metric-contract requests are available')
  })

  it('describes CompositeMo as CPU-evaluated with optional GPU prescan and CPU validation limits', () => {
    const caps = describeStrategyOptimizationCapabilities('CompositeMo')

    expect(caps.kind).toBe('CompositeMo')
    expect(caps.usesGlobalBackend).toBe(false)
    expect(caps.usesGlobalMode).toBe(false)
    expect(caps.supportsHeatmap).toBe(false)
    expect(caps.supportsExportAll).toBe(false)
    expect(caps.supportsTopK).toBe(false)
    expect(caps.supportsIncludeAll).toBe(false)
    expect(caps.supportsTradeMetrics).toBe(true)
    expect(caps.supportsCore17MetricSurface).toBe(true)
    expect(caps.supportsMetricContract).toBe(false)
    expect(caps.executionSummary).toContain('CPU-side multi-objective evaluator')
    expect(caps.executionSummary).toContain('optional GPU prescan')
    expect(caps.modeSummary).toContain('does not use the global backend selector')
    expect(caps.modeSummary).toContain('stage-0 Double-MA narrowing pass')
    expect(caps.outputSummary).toContain('Pareto points')
    expect(caps.outputSummary).toContain('canonical Trade22 surface')
    expect(caps.outputSummary).toContain('Validation currently uses the CPU route')
    expect(caps.outputSummary).toContain('CompositeCpu HardGate candidates')
  })
})
