import { describe, expect, it } from 'vitest'
import {
  buildMetricDistributionModel,
  chooseMetricDistributionKey,
  metricDistributionOptions,
} from './metricDistributionModel'

const columns = [
  { key: 'name', label: 'Name' },
  { key: 'pnl', label: 'PnL' },
  { key: 'max_dd', label: 'Max DD' },
  { key: 'gate', label: 'Gate' },
]

const rows = [
  {
    cells: [
      { text: 'A' },
      { text: '10' },
      { text: '2' },
      { text: 'pass' },
    ],
  },
  {
    cells: [
      { text: 'B' },
      { text: '20' },
      { text: '4' },
      { text: 'fail' },
    ],
  },
  {
    cells: [
      { text: 'C' },
      { text: '40' },
      { text: '10' },
      { text: 'pass' },
    ],
  },
]

describe('metricDistributionModel', () => {
  it('offers only numeric metric columns', () => {
    expect(metricDistributionOptions(columns, rows)).toEqual([
      { key: 'pnl', label: 'PnL' },
      { key: 'max_dd', label: 'Max DD' },
    ])
  })

  it('keeps option order with late numeric cells and source-column indirection', () => {
    expect(metricDistributionOptions(
      [
        { key: 'rank', label: 'Rank' },
        { key: 'pnl', label: 'PnL' },
        { key: 'gate', label: 'Gate' },
        { key: 'score', label: 'Score', sourceColumnIndex: 4 },
        { key: 'omega', label: 'Omega', sourceColumnIndex: 5 },
        { key: 'calmar', label: 'Calmar' },
      ],
      [
        {
          cells: [
            { text: '1' },
            { text: '-' },
            { text: 'pass' },
            { text: 'display only' },
            { text: '-' },
          ],
        },
        {
          cells: [
            { text: '2' },
            { text: '7' },
            { text: 'fail' },
            { text: 'display only' },
            { text: '0.91' },
          ],
        },
      ],
    )).toEqual([
      { key: 'pnl', label: 'PnL' },
      { key: 'score', label: 'Score' },
    ])
  })

  it('keeps a valid current metric before preferred fallbacks', () => {
    const options = metricDistributionOptions(columns, rows)
    expect(chooseMetricDistributionKey(options, ['pnl'], 'max_dd')).toBe('max_dd')
    expect(chooseMetricDistributionKey(options, ['pnl'], 'missing')).toBe('pnl')
    expect(chooseMetricDistributionKey([], ['pnl'], 'pnl')).toBe('')
  })

  it('builds a compact histogram summary for a selected metric', () => {
    const model = buildMetricDistributionModel({
      columns,
      rows,
      metricKey: 'pnl',
      binCount: 4,
    })

    expect(model).toMatchObject({
      key: 'pnl',
      label: 'PnL',
      values: [10, 20, 40],
      min: 10,
      max: 40,
      mean: 70 / 3,
      median: 20,
    })
    expect(model?.bins).toHaveLength(4)
    expect(model?.bins.reduce((sum, bin) => sum + bin.count, 0)).toBe(3)
  })
})
