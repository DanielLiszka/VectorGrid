
import { describe, expect, it } from 'vitest'
import {
  baseLightweightChartOptions,
  candlestickSeriesOptions,
  histogramPriceScaleOptions,
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
      textColor: 'rgba(226, 232, 240, 0.95)',
      background: { type: 'solid', color: 'rgb(2, 6, 23)' },
    })
    expect(options.overlayPriceScales).toMatchObject({
      borderVisible: false,
      scaleMargins: { top: 0.78, bottom: 0.03 },
    })
    expect(options.timeScale).toMatchObject({ fixLeftEdge: false, fixRightEdge: false, timeVisible: true })
  })

  it('builds reusable series options', () => {
    expect(candlestickSeriesOptions()).toMatchObject({ borderVisible: true })
    expect(lineSeriesOptions('fast')).toMatchObject({ lineWidth: 2 })
    expect(lineSeriesOptions('ott')).toMatchObject({ color: expect.stringContaining('168, 85, 247'), lineStyle: 2 })
    expect(histogramSeriesOptions('macz')).toMatchObject({ base: 0, priceScaleId: 'macz' })
    expect(histogramSeriesOptions('halftrend')).toMatchObject({ base: 0, priceScaleId: 'halftrend' })
    expect(histogramPriceScaleOptions('halftrend')).toMatchObject({
      scaleMargins: { top: 0.18, bottom: 0.70 },
    })
  })
})
