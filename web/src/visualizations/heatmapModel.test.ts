
import { describe, expect, it } from 'vitest'
import {
  buildDerivedPlateauSurface,
  fastForHeatmapCol,
  finiteHeatmapValues,
  finiteValues,
  heatmapColForFast,
  heatmapDisplayLabel,
  heatmapHigherIsBetter,
  heatmapIndex,
  heatmapLogicalRowForSlow,
  heatmapLegendNote,
  heatmapModeFromValue,
  heatmapNeighborValues,
  heatmapPalette,
  heatmapSampleCountAt,
  heatmapValueAt,
  labelDerivedPlateau,
  readHeatmapModel,
  resolveHeatmapDisplayModel,
  resolveHeatmapRange,
  slowForHeatmapLogicalRow,
} from './heatmapModel'

describe('heatmapModel', () => {
  it('parses backend heatmap payloads and normalizes optional vectors', () => {
    const model = readHeatmapModel({
      heatmap: {
        bins_fast: 2,
        bins_slow: 2,
        fast_min: 5,
        fast_max: 9,
        slow_min: 20,
        slow_max: 30,
        cell_order: 'slow_fast_row_major_v1',
        values: [0.1, '0.2', null, 'bad'],
        mean_values: [0.05, null, 0.15, 'bad'],
        sample_counts: [2, '3.8', -1, 'bad'],
      },
    })

    expect(model).not.toBeNull()
    expect(model?.values).toEqual([0.1, 0.2, null, null])
    expect(model?.meanValues).toEqual([0.05, null, 0.15, null])
    expect(model?.sampleCounts).toEqual([2, null, null, null])
    expect(finiteHeatmapValues(model!)).toEqual([0.1, 0.2])
  })

  it('rejects missing, malformed, oversized, or unsupported heatmaps', () => {
    const valid = {
      bins_fast: 2,
      bins_slow: 2,
      fast_min: 5,
      fast_max: 9,
      slow_min: 20,
      slow_max: 30,
      cell_order: 'slow_fast_row_major_v1',
      values: [0.1, 0.2, 0.3, 0.4],
    }
    expect(readHeatmapModel({})).toBeNull()
    expect(readHeatmapModel({ heatmap: { ...valid, values: [] } })).toBeNull()
    expect(readHeatmapModel({ heatmap: { ...valid, values: [1, 2, 3] } })).toBeNull()
    expect(readHeatmapModel({ heatmap: { ...valid, values: [1, 2, 3, 4, 5] } })).toBeNull()
    expect(
      readHeatmapModel({
        heatmap: { ...valid, cell_order: 'fast_slow_column_major_v1' },
      }),
    ).toBeNull()
    expect(
      readHeatmapModel({
        heatmap: { ...valid, bins_fast: 513, values: [1, 2, 3, 4] },
      }),
    ).toBeNull()
    expect(
      readHeatmapModel({
        heatmap: { ...valid, fast_min: 10, fast_max: 9 },
      }),
    ).toBeNull()
    expect(
      readHeatmapModel({
        heatmap: { ...valid, mean_values: [1, 2] },
      }),
    ).toBeNull()
    expect(
      readHeatmapModel({
        heatmap: { ...valid, sample_counts: [1, 2, 3] },
      }),
    ).toBeNull()
  })

  it('keeps finite values and zero-centered heatmap ranges deterministic', () => {
    expect(finiteValues([1, null, Number.NaN, -3, 2])).toEqual([1, -3, 2])

    const range = resolveHeatmapRange([-0.2, 0.1, 0.4])
    expect(range.label).toBe('zero-centered')
    expect(range.min).toBeLessThanOrEqual(-0.4)
    expect(range.max).toBeGreaterThanOrEqual(0.4)
  })

  it('normalizes mode labels and legend notes', () => {
    expect(heatmapModeFromValue('plateau')).toBe('plateau')
    expect(heatmapModeFromValue('other')).toBe('objective')
    expect(heatmapDisplayLabel('objective', 'Sharpe')).toBe('Sharpe')
    expect(heatmapDisplayLabel('plateau', 'Sharpe')).toBe('Derived plateau')
    expect(heatmapLegendNote('plateau')).toContain('Sequential')
  })

  it('maps heatmap coordinates and cell access consistently', () => {
    const model = readHeatmapModel({
      heatmap: {
        bins_fast: 3,
        bins_slow: 3,
        fast_min: 5,
        fast_max: 15,
        slow_min: 20,
        slow_max: 40,
        values: [1, 2, 3, 4, 5, 6, 7, 8, 9],
        sample_counts: [1, 2, 3, 4, 5, 6, 7, 8, 9],
      },
    })!

    expect(heatmapIndex(model, 1, 2)).toBe(5)
    expect(heatmapValueAt(model, 1, 2)).toBe(6)
    expect(heatmapSampleCountAt(model, 2, 1)).toBe(8)
    expect(heatmapNeighborValues(model, 1, 1, true)).toHaveLength(8)
    expect(fastForHeatmapCol(model, 2)).toBe(15)
    expect(slowForHeatmapLogicalRow(model, 2)).toBe(40)
    expect(heatmapColForFast(model, 14)).toBe(1)
    expect(heatmapColForFast(model, 15)).toBe(2)
    expect(heatmapLogicalRowForSlow(model, 21)).toBe(0)
  })

  it('matches backend floor binning and chooses a representative inside every bin', () => {
    const model = readHeatmapModel({
      heatmap: {
        bins_fast: 4,
        bins_slow: 5,
        fast_min: 5,
        fast_max: 15,
        slow_min: 17,
        slow_max: 30,
        cell_order: 'slow_fast_row_major_v1',
        values: new Array(20).fill(0),
      },
    })!

    expect(heatmapColForFast(model, 8)).toBe(0)
    expect(heatmapColForFast(model, 9)).toBe(1)
    expect(heatmapColForFast(model, 11)).toBe(1)
    expect(heatmapColForFast(model, 12)).toBe(2)
    expect(heatmapLogicalRowForSlow(model, 20)).toBe(0)
    expect(heatmapLogicalRowForSlow(model, 21)).toBe(1)

    for (let col = 0; col < model.binsFast; col += 1) {
      expect(heatmapColForFast(model, fastForHeatmapCol(model, col))).toBe(col)
    }
    for (let row = 0; row < model.binsSlow; row += 1) {
      expect(
        heatmapLogicalRowForSlow(model, slowForHeatmapLogicalRow(model, row)),
      ).toBe(row)
    }
  })

  it('builds derived plateau display models with diagnostics', () => {
    const model = readHeatmapModel({
      heatmap: {
        bins_fast: 3,
        bins_slow: 3,
        fast_min: 5,
        fast_max: 15,
        slow_min: 20,
        slow_max: 40,
        values: [1, 1.1, 1, 1.2, 1.25, 1.18, 1, 1.05, 1],
        mean_values: [1, 1.1, 1, 1.2, 1.22, 1.18, 1, 1.05, 1],
        sample_counts: [1, 1, 1, 1, 3, 1, 1, 1, 1],
      },
    })!
    const surface = buildDerivedPlateauSurface(model, { higherIsBetter: true })

    expect(heatmapHigherIsBetter('Sharpe')).toBe(true)
    expect(heatmapHigherIsBetter('Volatility')).toBe(true)
    expect(heatmapHigherIsBetter('MaxDrawdown')).toBe(true)
    expect(labelDerivedPlateau(0.8, 6, 8)).toBe('Strong')
    expect(surface.values[4]).toBeGreaterThan(0)
    expect(surface.diagnostics[4]?.sampleCount).toBe(3)
    expect(surface.multiSampleCellCount).toBe(1)

    const display = resolveHeatmapDisplayModel({
      mode: 'plateau',
      objectiveLabel: 'Sharpe',
      model,
      higherIsBetter: true,
    })
    expect(display.range).toEqual({ min: 0, max: 1, label: 'derived plateau score' })
    expect(display.label).toBe('Derived plateau')
    expect(display.palette).toEqual(heatmapPalette('plateau'))
    expect(display.plateauDiagnostics?.[4]).toBeTruthy()
  })
})
