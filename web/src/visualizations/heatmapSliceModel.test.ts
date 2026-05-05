import { describe, expect, it } from 'vitest'
import { buildHeatmapSlice3dModel, type HeatmapSliceSourceRow } from './heatmapSliceModel'
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
    expect(model?.sourceRowCount).toBe(4)
    expect(model?.retainedSliceRowCount).toBe(4)
    expect(model?.range.min).toBeLessThan(0)
    expect(model?.range.max).toBeGreaterThan(0)
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
