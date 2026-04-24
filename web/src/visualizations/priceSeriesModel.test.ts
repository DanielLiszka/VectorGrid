import { describe, expect, it } from 'vitest'
import {
  normalizeChartTimeSec,
  normalizeCompositeIndicatorBars,
  normalizeIndexedPriceBars,
  normalizeMovingAverageIndicatorBars,
  normalizePriceBars,
} from './priceSeriesModel'

describe('priceSeriesModel', () => {
  it('normalizes second, millisecond, microsecond, and nanosecond timestamps', () => {
    expect(normalizeChartTimeSec(1_744_500_000)).toBe(1_744_500_000)
    expect(normalizeChartTimeSec(1_744_500_000_123)).toBe(1_744_500_000)
    expect(normalizeChartTimeSec(1_744_500_000_123_000)).toBe(1_744_500_000)
    expect(normalizeChartTimeSec(1_744_500_000_123_000_000)).toBe(1_744_500_000)
    expect(Number.isNaN(normalizeChartTimeSec('bad'))).toBe(true)
  })

  it('normalizes, sorts, clamps wicks, and dedupes price bars', () => {
    const bars = normalizePriceBars([
      { t: 20, o: 10, h: 10.5, l: 9.5, c: 11 },
      { t: 10, o: 8, h: 8.2, l: 7.5, c: 7.8 },
      { t: 20, o: 12, h: 11, l: 13, c: 12.5 },
      { t: 'bad', o: 1, h: 1, l: 1, c: 1 },
    ])

    expect(bars).toEqual([
      { time: 10, open: 8, high: 8.2, low: 7.5, close: 7.8 },
      { time: 20, open: 12, high: 12.5, low: 12, close: 12.5 },
    ])
  })

  it('normalizes indexed drill bars with index aliases', () => {
    const bars = normalizeIndexedPriceBars([
      {
        timestamp: 30,
        open: 20,
        high: 21,
        low: 19,
        close: 20.5,
        volume: '42',
        fromIndex: '5',
        to: '8',
      },
    ])

    expect(bars[0]).toMatchObject({
      time: 30,
      volume: 42,
      fromIndex: 5,
      toIndex: 8,
    })
  })

  it('normalizes moving average and composite indicator bars', () => {
    expect(
      normalizeMovingAverageIndicatorBars([
        { time: 3, fast: '10.5', slow: null },
        { time: 1, fast: 'bad', slow: '20' },
        { time: 2, fast: 'bad', slow: 'bad' },
      ]),
    ).toEqual([
      { time: 1, fast: null, slow: 20 },
      { time: 3, fast: 10.5, slow: 0 },
    ])

    expect(
      normalizeCompositeIndicatorBars([
        { time: 2, halftrend: '101', macz_hist: '-0.2' },
        { time: 1, halftrend: null, macz: '0.3' },
      ]),
    ).toEqual([
      { time: 1, halftrend: 0, macz: 0.3 },
      { time: 2, halftrend: 101, macz: -0.2 },
    ])
  })
})
