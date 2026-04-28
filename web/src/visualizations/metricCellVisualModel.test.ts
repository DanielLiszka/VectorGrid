import { describe, expect, it } from 'vitest'
import {
  buildMetricColumnVisualRanges,
  metricColumnVisualRole,
  parseMetricCellNumber,
  resolveMetricCellVisual,
} from './metricCellVisualModel'

describe('metricCellVisualModel', () => {
  it('classifies high-signal metric columns without decorating identity columns', () => {
    expect(metricColumnVisualRole('pnl')).toBe('higher_better')
    expect(metricColumnVisualRole('max_dd')).toBe('lower_better')
    expect(metricColumnVisualRole('exposure')).toBe('neutral')
    expect(metricColumnVisualRole('gate')).toBe('status')
    expect(metricColumnVisualRole('rank')).toBeNull()
    expect(metricColumnVisualRole('fast')).toBeNull()
  })

  it('parses formatted numeric cell text without mutating display text', () => {
    expect(parseMetricCellNumber({ text: '$1,234.50', title: '' })).toBe(1234.5)
    expect(parseMetricCellNumber({ text: '12.5%', title: '' })).toBe(12.5)
    expect(parseMetricCellNumber({ text: '(7.25%)', title: '' })).toBe(-7.25)
    expect(parseMetricCellNumber({ text: 'display', title: '42.125' })).toBe(42.125)
    expect(parseMetricCellNumber({ text: '—', title: '' })).toBeNull()
  })

  it('builds visual ranges through source column indexes after column reordering', () => {
    const ranges = buildMetricColumnVisualRanges(
      [
        { key: 'pnl', label: 'PnL', sourceColumnIndex: 2 },
        { key: 'name', label: 'Name', sourceColumnIndex: 0 },
        { key: 'max_dd', label: 'Max DD', sourceColumnIndex: 1 },
      ],
      [
        {
          cells: [
            { text: 'A' },
            { text: '5' },
            { text: '100' },
          ],
        },
        {
          cells: [
            { text: 'B' },
            { text: '20' },
            { text: '300' },
          ],
        },
      ],
    )

    expect(ranges.pnl).toMatchObject({ min: 100, max: 300, role: 'higher_better' })
    expect(ranges.max_dd).toMatchObject({ min: 5, max: 20, role: 'lower_better' })
    expect(ranges.name).toBeUndefined()
  })

  it('resolves bar and status badge visuals from existing cell payloads', () => {
    const ranges = buildMetricColumnVisualRanges(
      [
        { key: 'pnl', label: 'PnL' },
        { key: 'max_dd', label: 'Max DD' },
      ],
      [
        { cells: [{ text: '100' }, { text: '5' }] },
        { cells: [{ text: '300' }, { text: '20' }] },
      ],
    )

    expect(resolveMetricCellVisual('pnl', { text: '300' }, ranges)).toMatchObject({
      kind: 'bar',
      tone: 'good',
      fillPct: 100,
    })
    expect(resolveMetricCellVisual('max_dd', { text: '5' }, ranges)).toMatchObject({
      kind: 'bar',
      tone: 'good',
      fillPct: 100,
    })
    expect(resolveMetricCellVisual('gate', { text: 'pass' }, ranges)).toMatchObject({
      kind: 'badge',
      tone: 'good',
      label: 'gate pass',
    })
  })
})
