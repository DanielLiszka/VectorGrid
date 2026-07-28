import { describe, expect, it } from 'vitest'
import {
  buildHeatmapSurface3dModel,
  resolveHeatmapSurface3dKeyboardIndex,
} from './heatmapSurface3dModel'
import type { HeatmapCanvasRenderModel } from './heatmapCanvasModel'

function renderModel(): HeatmapCanvasRenderModel {
  return {
    signature: 'heatmap:3d:test',
    meta: 'Sharpe heatmap',
    statusLine: 'Heatmap ready.',
    model: {
      binsFast: 4,
      binsSlow: 4,
      fastMin: 5,
      fastMax: 8,
      slowMin: 20,
      slowMax: 32,
      values: [
        null, 0.1, 0.2, 0.3,
        null, null, 0.4, 0.5,
        null, null, null, 0.6,
        null, null, null, null,
      ],
      meanValues: null,
      sampleCounts: null,
    },
    displayMode: 'objective',
    displayValues: [
      null, 0.1, 0.2, 0.3,
      null, null, 0.4, 0.5,
      null, null, null, 0.6,
      null, null, null, null,
    ],
    displayLabel: 'Sharpe',
    displayLegendNote: 'Zero-centered',
    request: {
      fastMaType: 'sma',
      slowMaType: 'ema',
      fastMaParams: null,
      slowMaParams: null,
      scoreLabel: 'Sharpe',
    },
    markers: [],
    range: { min: -0.6, max: 0.6, label: 'zero-centered' },
    palette: {
      kind: 'diverging',
      negative: '#b91c1c',
      zero: '#2563eb',
      positive: '#15803d',
    },
    plateauDiagnostics: null,
    slice3d: null,
  }
}

describe('heatmapSurface3dModel', () => {
  it('keeps finite cells, unavailable counts, and representative cells explicit', () => {
    const surface = buildHeatmapSurface3dModel(renderModel(), { maxBins: 4 })

    expect(surface.totalCells).toBe(16)
    expect(surface.finiteCells).toBe(6)
    expect(surface.unavailableCells).toBe(10)
    expect(surface.samples).toHaveLength(6)
    expect(surface.unavailableBlocks).toHaveLength(10)
    expect(surface.unavailableBlocks[0]).toMatchObject({
      colStart: 0,
      colEnd: 1,
      logicalRowStart: 0,
      logicalRowEnd: 1,
    })
    expect(surface.samples[0]).toMatchObject({
      colStart: 1,
      colEnd: 2,
      logicalRowStart: 0,
      logicalRowEnd: 1,
      fast: 6,
      slow: 20,
    })
  })

  it('downsamples oversized heatmaps without hiding the source-grid stride', () => {
    const surface = buildHeatmapSurface3dModel(renderModel(), { maxBins: 2 })

    expect(surface.colStride).toBe(2)
    expect(surface.rowStride).toBe(2)
    expect(surface.sampledBinsFast).toBe(2)
    expect(surface.sampledBinsSlow).toBe(2)
    expect(surface.samples.length).toBeGreaterThan(0)
  })

  it('moves keyboard focus across retained bins while skipping unavailable cells', () => {
    const samples = buildHeatmapSurface3dModel(renderModel(), { maxBins: 4 }).samples

    expect(resolveHeatmapSurface3dKeyboardIndex(samples, 0, 'ArrowRight')).toBe(1)
    expect(resolveHeatmapSurface3dKeyboardIndex(samples, 1, 'ArrowLeft')).toBe(0)
    expect(resolveHeatmapSurface3dKeyboardIndex(samples, 0, 'ArrowDown')).toBe(3)
    expect(resolveHeatmapSurface3dKeyboardIndex(samples, 3, 'ArrowUp')).toBe(1)
    expect(resolveHeatmapSurface3dKeyboardIndex(samples, 3, 'Home')).toBe(0)
    expect(resolveHeatmapSurface3dKeyboardIndex(samples, 0, 'End')).toBe(5)
  })
})
