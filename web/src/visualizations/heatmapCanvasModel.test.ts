import { describe, expect, it } from 'vitest'
import {
  buildHeatmapCellParams,
  buildHeatmapCanvasSummaryItems,
  buildHeatmapSliceCanvasRenderModel,
  fitHeatmapViewportToFiniteDomain,
  heatmapTooltipText,
  type HeatmapCanvasRenderModel,
} from './heatmapCanvasModel'
import type { HeatmapModel } from './heatmapModel'
import { buildHeatmapSlice3dModel, type HeatmapSliceSourceRow } from './heatmapSliceModel'

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
  it('builds a 2D delta surface from retained slice rows and keeps exact selected-slice params', () => {
    const base = renderModel()
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
    expect(summary.find((item) => item.key === 'best')?.detail).toContain('Fast 5')
  })
})
