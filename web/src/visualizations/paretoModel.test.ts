
import { describe, expect, it } from 'vitest'
import {
  buildParetoAxisOptions,
  buildParetoRepresentativeSummary,
  buildParetoViewModel,
  paddedParetoRange,
  paretoAxisValue,
  toParetoRenderModel,
  type ParetoSourceRow,
} from './paretoModel'

const rows: ParetoSourceRow[] = [
  {
    rank: 1,
    metrics: { pnl: 0.12, sharpe: 1.4, max_dd: 0.18, trades: 28, exposure: 0.45 },
    mo: { maximize_value: 1.2, minimize_value: 0.18, violation: 0 },
  },
  {
    rank: 2,
    metrics: { pnl: 0.18, sharpe: 1.1, max_dd: 0.1, trades: 18, exposure: 0.55 },
    mo: { maximize_value: 1.1, minimize_value: 0.1, violation: 0.02 },
  },
  {
    rank: 3,
    metrics: { pnl: 'bad', sharpe: 0.7, max_dd: 0.16, trades: 7, exposure: 0.2 },
    mo: { maximize_value: 0.8, minimize_value: 0.16, violation: 0 },
  },
]

describe('paretoModel', () => {
  it('adds multi-objective axis options only when available', () => {
    expect(buildParetoAxisOptions(false).map((option) => option.value)).toEqual([
      'pnl',
      'sharpe',
      'max_dd',
      'trades',
      'exposure',
      'score',
    ])
    expect(buildParetoAxisOptions(true).map((option) => option.value)).toContain('maximize_value')
    expect(buildParetoAxisOptions(true).map((option) => option.value)).toContain('violation')
  })

  it('derives metric, multi-objective, and supplied score-axis values', () => {
    expect(paretoAxisValue(rows[0]!, 'pnl')).toBe(0.12)
    expect(paretoAxisValue(rows[0]!, 'maximize_value')).toBe(1.2)
    expect(paretoAxisValue(rows[0]!, 'score', 42)).toBe(42)
    expect(Number.isNaN(paretoAxisValue(rows[2]!, 'pnl'))).toBe(true)
  })

  it('pads constant ranges so canvas scaling never divides by zero', () => {
    const range = paddedParetoRange([5, 5, 5])

    expect(range.min).toBeLessThan(5)
    expect(range.max).toBeGreaterThan(5)
  })

  it('filters non-finite points, caps source rows, and keeps only non-dominated frontier points', () => {
    const model = buildParetoViewModel({
      rows,
      xKey: 'max_dd',
      yKey: 'pnl',
      limit: 2,
      axisValueFor: (row, key) => paretoAxisValue(row, key),
      rowIdFor: (_row, index) => `row-${index + 1}`,
      rankFor: (row) => row.rank,
    })

    expect(model.status).toBe('ready')
    expect(model.points).toHaveLength(2)
    expect(model.frontier.map((point) => point.rank)).toEqual([2])
    expect(model.points.find((point) => point.rank === 1)?.frontier).toBe(false)
    expect(model.meta).toContain('2 points')
    expect(model.meta).toContain('x=max_dd')
  })

  it('summarizes representative and selected frontier points', () => {
    const model = buildParetoViewModel({
      rows,
      xKey: 'max_dd',
      yKey: 'pnl',
      axisValueFor: (row, key) => paretoAxisValue(row, key),
      rowIdFor: (_row, index) => `row-${index + 1}`,
      rankFor: (row) => row.rank,
      selectedRowId: 'row-2',
      selectedRank: 2,
      hasValidationResult: true,
    })

    expect(model.representativeSummary).toContain('x-extreme #2')
    expect(model.representativeSummary).toContain('y-extreme #2')
    expect(model.representativeSummary).toContain('selected #2')
    expect(model.confidenceNote).toContain('Validation adds admission evidence')
  })

  it('summarizes selected modeled rows that are dominated as off-frontier', () => {
    const model = buildParetoViewModel({
      rows,
      xKey: 'max_dd',
      yKey: 'pnl',
      axisValueFor: (row, key) => paretoAxisValue(row, key),
      rowIdFor: (_row, index) => `row-${index + 1}`,
      rankFor: (row) => row.rank,
      selectedRowId: 'row-1',
      selectedRank: 1,
    })

    expect(model.representativeSummary).toContain('selected off-frontier #1')
  })

  it('preserves selected off-frontier context', () => {
    const summary = buildParetoRepresentativeSummary(
      [
        {
          index: 0,
          row: rows[0],
          rowId: 'row-1',
          rank: 1,
          x: 0.18,
          y: 0.12,
          frontier: true,
          summary: '#1',
        },
      ],
      'row-99',
      99,
    )

    expect(summary).toContain('selected off-frontier #99')
  })

  it('strips source rows from render models', () => {
    const model = buildParetoViewModel({
      rows,
      xKey: 'max_dd',
      yKey: 'pnl',
      axisValueFor: (row, key) => paretoAxisValue(row, key),
      rowIdFor: (_row, index) => `row-${index + 1}`,
      rankFor: (row) => row.rank,
    })

    const renderModel = toParetoRenderModel(model)

    expect(renderModel.points[0]).toMatchObject({
      rowId: 'row-1',
      rank: 1,
      summary: expect.stringContaining('#1'),
    })
    expect('row' in renderModel.points[0]!).toBe(false)
  })
})
