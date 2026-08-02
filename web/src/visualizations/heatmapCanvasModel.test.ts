import { describe, expect, it, vi } from 'vitest'
import {
  buildHeatmapCellParams,
  buildHeatmapCanvasSummaryItems,
  buildHeatmapSliceCanvasRenderModel,
  fitHeatmapViewportToFiniteDomain,
  formatHeatmapDisplay,
  HEATMAP_MAX_CANVAS_PIXELS,
  heatmapTooltipText,
  paintHeatmapLegend,
  resolveHeatmapCanvasBackingStore,
  sampleHeatmapColor,
  visibleHeatmapIndexSamples,
  type HeatmapCanvasRenderModel,
} from './heatmapCanvasModel'
import type { HeatmapModel } from './heatmapModel'
import { buildHeatmapSlice3dModel, type HeatmapSliceSourceRow } from './heatmapSliceModel'

function legendPaintHarness(width = 400, height = 18) {
  const gradientStops: Array<{ offset: number; color: string }> = []
  const gradient = {
    addColorStop: vi.fn((offset: number, color: string) => {
      gradientStops.push({ offset, color })
    }),
  }
  const context = {
    createLinearGradient: vi.fn(() => gradient),
    clearRect: vi.fn(),
    fillRect: vi.fn(),
    strokeRect: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    fillStyle: '',
    strokeStyle: '',
    lineWidth: 0,
  }
  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  Object.defineProperty(canvas, 'getContext', {
    configurable: true,
    value: () => context,
  })
  return {
    targets: {
      canvas,
      minLabel: document.createElement('span'),
      midLabel: document.createElement('span'),
      maxLabel: document.createElement('span'),
    },
    context,
    gradientStops,
  }
}

function model(): HeatmapModel {
  return {
    binsFast: 3,
    binsSlow: 3,
    fastMin: 5,
    fastMax: 7,
    slowMin: 20,
    slowMax: 30,
    values: [
      0.2, null, null,
      null, 0.3, null,
      null, null, null,
    ],
    meanValues: null,
    sampleCounts: null,
  }
}

function row(
  offset: number,
  fast: number,
  slow: number,
  score: number,
): HeatmapSliceSourceRow {
  return {
    score,
    params: {
      fast_len: fast,
      slow_len: slow,
      fast_ma_type: 'alma',
      slow_ma_type: 'sma',
      fast_ma_params: { offset, sigma: 6 },
      slow_ma_params: null,
    },
    metrics: { sharpe: score },
  }
}

function renderModel(): HeatmapCanvasRenderModel {
  const heatmap = model()
  const slice3d = buildHeatmapSlice3dModel({
    model: heatmap,
    scoreLabel: 'Sharpe',
    rows: [
      row(0.75, 5, 20, 0.2),
      row(0.75, 6, 25, 0.3),
      row(0.85, 5, 20, 0.4),
      row(0.85, 6, 25, -0.1),
    ],
  })
  if (!slice3d) throw new Error('slice fixture failed')
  return {
    signature: 'heatmap:test',
    meta: 'Sharpe heatmap',
    statusLine: 'Heatmap ready',
    model: heatmap,
    displayMode: 'objective',
    displayValues: heatmap.values,
    displayLabel: 'Sharpe',
    displayLegendNote: 'Zero-centered',
    request: {
      fastAxisLabel: 'Fast ALMA length',
      slowAxisLabel: 'Slow SMA length',
      fastAxisDescription: 'Fast ALMA length used by each grid column.',
      slowAxisDescription: 'Slow SMA length used by each grid row.',
      supports3dSlices: true,
      threeDimensionalUnavailableReason: '',
      fastMaType: 'alma',
      slowMaType: 'sma',
      fastMaParams: null,
      slowMaParams: null,
      scoreLabel: 'Sharpe',
    },
    markers: [],
    range: { min: -1, max: 1, label: 'zero-centered' },
    palette: {
      kind: 'diverging',
      negative: '#b91c1c',
      zero: '#2563eb',
      positive: '#15803d',
    },
    plateauDiagnostics: null,
    slice3d,
  }
}

describe('heatmapCanvasModel slice enhancements', () => {
  it('positions the diverging zero anchor from an asymmetric range', () => {
    const harness = legendPaintHarness()
    const palette = {
      kind: 'diverging' as const,
      negative: '#B6405B',
      zero: '#F4F2EC',
      positive: '#006A73',
    }
    paintHeatmapLegend(
      harness.targets,
      { min: -2, max: 6, label: 'asymmetric' },
      palette,
      'light',
    )

    expect(harness.context.moveTo).toHaveBeenCalledWith(100, 1)
    expect(harness.targets.minLabel.textContent).toBe('-2')
    expect(harness.targets.midLabel.textContent).toBe('0')
    expect(harness.targets.midLabel.style.left).toBe('25%')
    expect(harness.targets.midLabel.dataset.legendAnchor).toBe('zero')
    expect(harness.targets.maxLabel.textContent).toBe('6')
    expect(harness.gradientStops.find((stop) => stop.offset === 0.25)?.color).toBe(
      sampleHeatmapColor(0, -2, 6, palette, 'light'),
    )
  })

  it('omits the diverging zero marker outside a one-sided range', () => {
    const harness = legendPaintHarness()
    const palette = {
      kind: 'diverging' as const,
      negative: '#FF8A73',
      zero: '#26313A',
      positive: '#50C4C8',
    }
    paintHeatmapLegend(
      harness.targets,
      { min: 2, max: 6, label: 'positive-only' },
      palette,
      'dark',
    )

    expect(harness.context.beginPath).not.toHaveBeenCalled()
    expect(harness.targets.midLabel.textContent).toBe('4')
    expect(harness.targets.midLabel.style.left).toBe('50%')
    expect(harness.targets.midLabel.dataset.legendAnchor).toBe('midpoint')
    expect(harness.gradientStops[0]?.color).toBe(
      sampleHeatmapColor(2, 2, 6, palette, 'dark'),
    )
  })

  it('preserves tiny non-zero scores instead of presenting them as zero', () => {
    expect(formatHeatmapDisplay(0.00001, 4)).toBe('1e-5')
    expect(formatHeatmapDisplay(-0.00001, 4)).toBe('-1e-5')
  })

  it('preserves full DPR for ordinary canvases and caps oversized backing stores', () => {
    expect(resolveHeatmapCanvasBackingStore({
      cssWidth: 512,
      cssHeight: 512,
      devicePixelRatio: 3,
    })).toEqual({
      width: 1536,
      height: 1536,
      pixelRatio: 3,
    })

    const large = resolveHeatmapCanvasBackingStore({
      cssWidth: 1152,
      cssHeight: 1152,
      devicePixelRatio: 3,
    })
    expect(large.width * large.height).toBeLessThanOrEqual(HEATMAP_MAX_CANVAS_PIXELS)
    expect(large.pixelRatio).toBeGreaterThan(1)
    expect(large.pixelRatio).toBeLessThan(3)
  })

  it('samples axis ticks from the visible viewport instead of the full grid', () => {
    expect(visibleHeatmapIndexSamples({
      bins: 512,
      logicalSize: 512,
      scale: 1,
      offset: 0,
    })).toEqual([0, 128, 256, 383, 511])

    expect(visibleHeatmapIndexSamples({
      bins: 512,
      logicalSize: 512,
      scale: 12,
      offset: -1200,
    })).toEqual([100, 111, 121, 132, 142])
  })

  it('builds a 2D delta surface from retained slice rows and keeps exact selected-slice params', () => {
    const base = renderModel()
    base.exactAggregateWinnerCells = [
      {
        score: 0.2,
        params: {
          fast_len: 5,
          slow_len: 20,
          fast_ma_type: 'alma',
          slow_ma_type: 'sma',
          fast_ma_params: { offset: 0.99, sigma: 6 },
        },
        metrics: { sharpe: 0.2 },
      },
    ]
    base.aggregateSelectionRequiresExactParams = true
    const selected = base.slice3d?.slices.find((slice) => slice.label.includes('offset=0.85'))
    const baseline = base.slice3d?.slices.find((slice) => slice.label.includes('offset=0.75'))
    const delta = buildHeatmapSliceCanvasRenderModel(base, {
      mode: 'delta',
      selectedSliceKey: selected!.key,
      baselineSliceKey: baseline!.key,
    })

    expect(delta).toBeTruthy()
    expect(delta?.displayLabel).toBe('Sharpe Δ')
    expect(delta?.displayValues[0]).toBeCloseTo(0.2)
    expect(delta?.displayValues[4]).toBeCloseTo(-0.4)
    expect(delta?.markers.map((marker) => marker.label)).toEqual(['Best', '+Δ', '-Δ'])

    const tooltip = heatmapTooltipText(delta!, {
      col: 0,
      drawRow: 2,
      logicalRow: 0,
      idx: 0,
    })
    expect(tooltip).toContain('Δ 0.2')
    expect(tooltip).toContain('selected 0.4')
    expect(tooltip).toContain('baseline 0.2')

    expect(buildHeatmapCellParams(delta!, {
      col: 0,
      drawRow: 2,
      logicalRow: 0,
      idx: 0,
    })).toMatchObject({
      fast_ma_type: 'alma',
      fast_ma_params: { offset: 0.85, sigma: 6 },
    })
    expect(tooltip).toContain('exact retained winner for the selected slice')
  })

  it('fits the viewport to populated display cells and summarizes the active slice', () => {
    const base = renderModel()
    const selected = base.slice3d?.slices.find((slice) => slice.label.includes('offset=0.85'))
    const slice = buildHeatmapSliceCanvasRenderModel(base, {
      mode: 'slice',
      selectedSliceKey: selected!.key,
    })

    expect(slice).toBeTruthy()
    const viewport = fitHeatmapViewportToFiniteDomain(slice!, 600, 600)
    expect(viewport.scale).toBeGreaterThan(1)

    const summary = buildHeatmapCanvasSummaryItems(slice!)
    expect(summary.map((item) => item.key)).toContain('slice')
    expect(summary.find((item) => item.key === 'coverage')?.detail).toContain('2/9')
    expect(summary.find((item) => item.key === 'best')?.detail).toContain('Fast ALMA length 5')
    expect(summary.find((item) => item.key === 'best')?.label).toBe('Best slice bin')
    expect(summary.find((item) => item.key === 'best')?.detail).toContain('retained-row maximum')
  })

  it('describes aggregate values as bin maxima with bounds and sample metadata', () => {
    const aggregate = renderModel()
    aggregate.model = {
      ...aggregate.model,
      fastMin: 5,
      fastMax: 15,
      slowMin: 20,
      slowMax: 40,
      values: [0.2, null, null, null, 0.3, null, null, null, null],
      meanValues: [0.1, null, null, null, 0.2, null, null, null, null],
      sampleCounts: [3, null, null, null, 2, null, null, null, null],
    }
    aggregate.displayValues = aggregate.model.values

    const tooltip = heatmapTooltipText(aggregate, {
      col: 0,
      drawRow: 2,
      logicalRow: 0,
      idx: 0,
    })
    expect(tooltip).toContain('Fast ALMA length bin 5–9')
    expect(tooltip).toContain('Slow SMA length bin 20–29')
    expect(tooltip).toContain('best-in-bin Sharpe 0.2')
    expect(tooltip).toContain('mean 0.1')
    expect(tooltip).toContain('samples 3')
    expect(tooltip).toContain('representative parameter pair')

    expect(buildHeatmapCellParams(aggregate, {
      col: 0,
      drawRow: 2,
      logicalRow: 0,
      idx: 0,
    })).toMatchObject({
      fast_len: 5,
      fast_ma_type: 'alma',
      slow_len: 20,
      slow_ma_type: 'sma',
    })

    const summary = buildHeatmapCanvasSummaryItems(aggregate)
    expect(summary.find((item) => item.key === 'best')?.label).toBe('Best bin')
    expect(summary.find((item) => item.key === 'best')?.detail).toContain('aggregate maximum')
  })

  it('uses strategy-local labels for presentation while retaining MA fields for drilldown', () => {
    const strategyModel = renderModel()
    strategyModel.request = {
      ...strategyModel.request,
      fastAxisLabel: 'Squeeze / momentum periods',
      slowAxisLabel: 'Trend period',
      fastAxisDescription: 'Grid columns set the squeeze and momentum periods.',
      slowAxisDescription: 'Grid rows set the trend period.',
      supports3dSlices: false,
      threeDimensionalUnavailableReason: 'Standalone heatmap.',
    }
    const cell = {
      col: 0,
      drawRow: 2,
      logicalRow: 0,
      idx: 0,
    }

    expect(heatmapTooltipText(strategyModel, cell)).toContain(
      'Squeeze / momentum periods bin 5',
    )
    expect(heatmapTooltipText(strategyModel, cell)).toContain('Trend period bin 20')
    expect(heatmapTooltipText(strategyModel, cell)).not.toContain('ALMA bin')
    expect(heatmapTooltipText(strategyModel, cell)).toContain(
      'representative parameter pair',
    )
    expect(buildHeatmapCellParams(strategyModel, cell)).toMatchObject({
      fast_ma_type: 'alma',
      slow_ma_type: 'sma',
    })
  })

  it('opens the exact retained aggregate winner when its score matches the best-in-bin value', () => {
    const aggregate = renderModel()
    aggregate.aggregateSelectionRequiresExactParams = true
    aggregate.exactAggregateWinnerCells = [
      {
        score: 0.2,
        params: {
          fast_len: 5,
          slow_len: 20,
          fast_ma_type: 'alma',
          slow_ma_type: 'sma',
          fast_ma_params: { offset: 0.85, sigma: 6 },
        },
        metrics: { sharpe: 0.2 },
      },
    ]
    const cell = {
      col: 0,
      drawRow: 2,
      logicalRow: 0,
      idx: 0,
    }

    expect(buildHeatmapCellParams(aggregate, cell)).toMatchObject({
      fast_ma_type: 'alma',
      fast_ma_params: { offset: 0.85, sigma: 6 },
    })
    expect(heatmapTooltipText(aggregate, cell)).toContain(
      'exact retained best-in-bin winner',
    )
  })

  it('disables ambiguous aggregate drilldown when the exact retained winner is unavailable', () => {
    const aggregate = renderModel()
    aggregate.aggregateSelectionRequiresExactParams = true
    aggregate.aggregateSelectionUnavailableReason =
      'Exact winning MA variant was not retained. Use Top Results or rerun with Return all results.'
    aggregate.exactAggregateWinnerCells = [
      {
        score: 0.19,
        params: {
          fast_len: 5,
          slow_len: 20,
          fast_ma_type: 'alma',
          slow_ma_type: 'sma',
          fast_ma_params: { offset: 0.85 },
        },
        metrics: { sharpe: 0.19 },
      },
    ]
    const cell = {
      col: 0,
      drawRow: 2,
      logicalRow: 0,
      idx: 0,
    }

    expect(buildHeatmapCellParams(aggregate, cell)).toBeNull()
    expect(heatmapTooltipText(aggregate, cell)).toContain(
      'Exact winning MA variant was not retained',
    )
    expect(heatmapTooltipText(aggregate, cell)).toContain('Return all results')
  })
})
