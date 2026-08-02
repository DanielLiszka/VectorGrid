import { describe, expect, it } from 'vitest'
import {
  buildHeatmapAggregateWinnerCells,
  buildHeatmapSlice3dModel,
  heatmapRequestHasMaVariantAmbiguity,
  type HeatmapSliceSourceRow,
} from './heatmapSliceModel'
import type { HeatmapModel } from './heatmapModel'

function heatmapModel(): HeatmapModel {
  return {
    binsFast: 4,
    binsSlow: 4,
    fastMin: 5,
    fastMax: 8,
    slowMin: 20,
    slowMax: 32,
    values: new Array<number | null>(16).fill(null),
    meanValues: null,
    sampleCounts: null,
  }
}

function row(
  fast: number,
  slow: number,
  score: number,
  fastParams: Record<string, unknown> | null,
): HeatmapSliceSourceRow {
  return {
    score,
    params: {
      fast_len: fast,
      slow_len: slow,
      fast_ma_type: 'alma',
      slow_ma_type: 'sma',
      fast_ma_params: fastParams,
      slow_ma_params: null,
    },
    metrics: { sharpe: score },
  }
}

describe('heatmapSliceModel', () => {
  it('retains the exact aggregate winner using backend bins and first-on-tie semantics', () => {
    const coarseModel: HeatmapModel = {
      ...heatmapModel(),
      binsFast: 2,
      binsSlow: 2,
      values: new Array<number | null>(4).fill(null),
    }
    const cells = buildHeatmapAggregateWinnerCells({
      model: coarseModel,
      rows: [
        row(5, 20, 0.4, { offset: 0.7 }),
        row(6, 21, 0.9, { offset: 0.8 }),
        row(6, 21, 0.9, { offset: 0.9 }),
      ],
    })

    expect(cells).toHaveLength(4)
    expect(cells[0]?.score).toBe(0.9)
    expect(cells[0]?.params.fast_len).toBe(6)
    expect(cells[0]?.params.fast_ma_params).toEqual({ offset: 0.8 })
    expect(cells[0]?.metrics).toEqual({ sharpe: 0.9 })
    expect(cells.slice(1)).toEqual([null, null, null])
  })

  it('ignores rows that could not have populated the backend heatmap domain', () => {
    const cells = buildHeatmapAggregateWinnerCells({
      model: heatmapModel(),
      rows: [
        row(5, 20, Number.NaN, { offset: 0.7 }),
        row(4, 20, 0.8, { offset: 0.8 }),
        row(8, 8, 0.9, { offset: 0.9 }),
      ],
    })

    expect(cells.every((cell) => cell == null)).toBe(true)
  })

  it('detects MA identity ambiguity conservatively, including single-valued sweeps', () => {
    expect(heatmapRequestHasMaVariantAmbiguity({
      fast_ma_types: ['sma'],
      slow_ma_types: ['ema'],
      fast_ma_params: { alpha: 0.2 },
      slow_ma_param_sweeps: null,
    })).toBe(false)
    expect(heatmapRequestHasMaVariantAmbiguity({
      fast_ma_types: ['sma', 'EMA'],
      slow_ma_types: ['ema'],
    })).toBe(true)
    expect(heatmapRequestHasMaVariantAmbiguity({
      fast_ma_types: ['sma'],
      slow_ma_types: ['ema'],
      fast_ma_param_sweeps: {
        offset: [0.85, 0.85, 0.1],
      },
    })).toBe(true)
    expect(heatmapRequestHasMaVariantAmbiguity({
      fastMaTypes: ['SMA', 'sma'],
      slowMaTypes: ['ema'],
      slowMaParamSweeps: {
        sigma: { start: 6, end: 6, step: 1 },
      },
    })).toBe(true)
    expect(heatmapRequestHasMaVariantAmbiguity({
      fast_ma_param_variants: [{ offset: 0.85 }],
    })).toBe(true)
    expect(heatmapRequestHasMaVariantAmbiguity({
      fast_ma_param_sweeps: {},
      slowMaParamVariants: [],
    })).toBe(false)
  })

  it('builds separate retained-row slices for MA parameter variants', () => {
    const model = buildHeatmapSlice3dModel({
      model: heatmapModel(),
      scoreLabel: 'Sharpe',
      rows: [
        row(5, 20, 0.4, { offset: 0.75 }),
        row(6, 21, 0.9, { offset: 0.75 }),
        row(5, 20, -0.2, { offset: 0.85 }),
        row(7, 32, 0.3, { offset: 0.85 }),
      ],
    })

    expect(model).toBeTruthy()
    expect(model?.slices).toHaveLength(2)
    expect(model?.slices[0]?.label).toContain('offset=0.75')
    expect(model?.slices[0]?.bestScore).toBe(0.9)
    expect(model?.slices[0]?.sourceRowCount).toBe(2)
    expect(model?.slices[1]?.sourceRowCount).toBe(2)
    expect(model?.sourceRowCount).toBe(4)
    expect(model?.retainedSliceRowCount).toBe(4)
    expect(model?.range.min).toBeLessThan(0)
    expect(model?.range.max).toBeGreaterThan(0)
  })

  it('keeps large mostly-empty slice buffers sparse', () => {
    const largeModel: HeatmapModel = {
      ...heatmapModel(),
      binsFast: 512,
      binsSlow: 512,
      fastMin: 1,
      fastMax: 512,
      slowMin: 513,
      slowMax: 1024,
      values: new Array<number | null>(512 * 512),
    }
    const model = buildHeatmapSlice3dModel({
      model: largeModel,
      scoreLabel: 'Sharpe',
      rows: [
        row(5, 520, 0.4, { offset: 0.75 }),
        row(6, 521, 0.9, { offset: 0.75 }),
        row(7, 522, -0.2, { offset: 0.85 }),
        row(8, 523, 0.3, { offset: 0.85 }),
      ],
    })

    expect(model?.slices).toHaveLength(2)
    for (const slice of model?.slices ?? []) {
      expect(slice.values).toHaveLength(512 * 512)
      expect(slice.sampleCounts).toHaveLength(512 * 512)
      expect(slice.cells).toHaveLength(512 * 512)
      expect(Object.keys(slice.values)).toHaveLength(slice.finiteCells)
      expect(Object.keys(slice.sampleCounts)).toHaveLength(slice.finiteCells)
      expect(Object.keys(slice.cells)).toHaveLength(slice.finiteCells)
    }
  })

  it('returns null when full-output rows do not expose a meaningful third dimension', () => {
    const model = buildHeatmapSlice3dModel({
      model: heatmapModel(),
      scoreLabel: 'Sharpe',
      rows: [
        row(5, 20, 0.4, null),
        row(6, 21, 0.9, null),
      ],
    })

    expect(model).toBeNull()
  })
})
