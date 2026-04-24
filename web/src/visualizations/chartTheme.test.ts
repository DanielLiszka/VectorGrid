// @vitest-environment jsdom
import { describe, expect, it } from 'vitest'
import {
  baseLightweightChartOptions,
  candlestickSeriesOptions,
  histogramSeriesOptions,
  lineSeriesOptions,
  normalizeChartSize,
} from './chartTheme'

describe('chartTheme', () => {
  it('normalizes chart dimensions with stable minimums', () => {
    expect(normalizeChartSize(100, 100)).toEqual({ width: 240, height: 220 })
    expect(normalizeChartSize(1024.8, 360.2)).toEqual({ width: 1024, height: 360 })
  })

  it('builds shared Lightweight Charts options', () => {
    const options = baseLightweightChartOptions({ width: 640, height: 280 })

    expect(options.width).toBe(640)
    expect(options.height).toBe(280)
    expect(options.layout).toMatchObject({
      textColor: 'rgba(203, 213, 225, 0.92)',
    })
    expect(options.timeScale).toMatchObject({ fixLeftEdge: false, fixRightEdge: false })
  })

  it('builds reusable series options', () => {
    expect(candlestickSeriesOptions()).toMatchObject({ borderVisible: true })
    expect(lineSeriesOptions('fast')).toMatchObject({ lineWidth: 2 })
    expect(lineSeriesOptions('halftrend')).toMatchObject({ lineWidth: 1, lineStyle: 2 })
    expect(histogramSeriesOptions()).toMatchObject({ base: 0 })
  })
})
