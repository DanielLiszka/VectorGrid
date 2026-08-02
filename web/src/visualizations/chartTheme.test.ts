
import { afterEach, describe, expect, it } from 'vitest'
import {
  baseLightweightChartOptions,
  candlestickSeriesOptions,
  colorizeHistogramSeriesData,
  histogramPriceScaleOptions,
  histogramSeriesOptions,
  lightweightChartLegendColor,
  lightweightChartSeriesPalette,
  lightweightChartThemeOptions,
  lineSeriesOptions,
  normalizeChartSize,
  resolveLightweightChartTheme,
} from './chartTheme'

function colorChannels(value: string): [number, number, number] {
  const hex = /^#([0-9a-f]{6})$/iu.exec(value)
  if (hex) {
    const hexValue = hex[1] ?? ''
    return [
      Number.parseInt(hexValue.slice(0, 2), 16),
      Number.parseInt(hexValue.slice(2, 4), 16),
      Number.parseInt(hexValue.slice(4, 6), 16),
    ]
  }
  const rgb = /^rgb\(\s*(\d+),\s*(\d+),\s*(\d+)\s*\)$/iu.exec(value)
  if (!rgb) throw new Error(`Unsupported test color: ${value}`)
  return [Number(rgb[1]), Number(rgb[2]), Number(rgb[3])]
}

function relativeLuminance(value: string): number {
  const channels = colorChannels(value).map((channel) => {
    const normalized = channel / 255
    return normalized <= 0.04045
      ? normalized / 12.92
      : ((normalized + 0.055) / 1.055) ** 2.4
  })
  const [red = 0, green = 0, blue = 0] = channels
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue
}

function contrastRatio(first: string, second: string): number {
  const firstLuminance = relativeLuminance(first)
  const secondLuminance = relativeLuminance(second)
  return (
    (Math.max(firstLuminance, secondLuminance) + 0.05) /
    (Math.min(firstLuminance, secondLuminance) + 0.05)
  )
}

describe('chartTheme', () => {
  afterEach(() => {
    delete document.documentElement.dataset.theme
  })

  it('normalizes chart dimensions with stable minimums', () => {
    expect(normalizeChartSize(100, 100)).toEqual({ width: 240, height: 220 })
    expect(normalizeChartSize(1024.8, 360.2)).toEqual({ width: 1024, height: 360 })
  })

  it('builds shared Lightweight Charts options', () => {
    const options = baseLightweightChartOptions({ width: 640, height: 280 }, 'dark')

    expect(options.width).toBe(640)
    expect(options.height).toBe(280)
    expect(options.layout).toMatchObject({
      textColor: 'rgba(226, 232, 240, 0.95)',
      background: { type: 'solid', color: 'rgb(2, 6, 23)' },
      attributionLogo: true,
    })
    expect(options.overlayPriceScales).toMatchObject({
      borderVisible: false,
      scaleMargins: { top: 0.78, bottom: 0.03 },
    })
    expect(options.timeScale).toMatchObject({ fixLeftEdge: false, fixRightEdge: false, timeVisible: true })
  })

  it('builds distinct light and dark palettes while preserving attribution', () => {
    const light = lightweightChartThemeOptions('light')
    const dark = lightweightChartThemeOptions('dark')

    expect(light.layout).toMatchObject({
      textColor: 'rgba(24, 34, 50, 0.94)',
      background: { type: 'solid', color: 'rgb(255, 253, 250)' },
      attributionLogo: true,
    })
    expect(dark.layout).toMatchObject({
      background: { type: 'solid', color: 'rgb(2, 6, 23)' },
      attributionLogo: true,
    })
    expect(light.grid).not.toEqual(dark.grid)
    expect(light.crosshair).not.toEqual(dark.crosshair)
  })

  it('reads the system-resolved theme from the root data attribute', () => {
    document.documentElement.dataset.theme = 'dark'
    expect(resolveLightweightChartTheme()).toBe('dark')

    document.documentElement.dataset.theme = 'light'
    expect(resolveLightweightChartTheme()).toBe('light')

    document.documentElement.dataset.theme = 'system'
    expect(resolveLightweightChartTheme()).toBe('light')
  })

  it('builds reusable series options', () => {
    expect(candlestickSeriesOptions('dark')).toMatchObject({ borderVisible: true })
    expect(lineSeriesOptions('fast', 'dark')).toMatchObject({ lineWidth: 2 })
    expect(lineSeriesOptions('ott', 'dark')).toMatchObject({ color: '#c084fc', lineStyle: 2 })
    expect(histogramSeriesOptions('macz', 'dark')).toMatchObject({ base: 0, priceScaleId: 'macz' })
    expect(histogramSeriesOptions('halftrend', 'dark')).toMatchObject({ base: 0, priceScaleId: 'halftrend' })
    expect(histogramPriceScaleOptions('halftrend')).toMatchObject({
      scaleMargins: { top: 0.18, bottom: 0.70 },
    })
  })

  it.each(['light', 'dark'] as const)(
    'keeps every %s series tone above 3:1 contrast against its exact canvas',
    (theme) => {
      const { canvas, ...tones } = lightweightChartSeriesPalette(theme)
      for (const [tone, color] of Object.entries(tones)) {
        expect(
          contrastRatio(canvas, color),
          `${theme}.${tone} ${color} against ${canvas}`,
        ).toBeGreaterThanOrEqual(3)
      }
    },
  )

  it('uses the same palette contract for series data and legend swatches', () => {
    const light = lightweightChartSeriesPalette('light')
    const points = colorizeHistogramSeriesData(
      [{ time: 1, value: 1 }, { time: 2, value: -1 }],
      'light',
    )

    expect(points).toEqual([
      { time: 1, value: 1, color: light.positive },
      { time: 2, value: -1, color: light.negative },
    ])
    expect(lightweightChartLegendColor('light', 'fast')).toBe(light.fast)
    expect(lightweightChartLegendColor('light', 'candles')).toContain(light.positive)
    expect(lightweightChartLegendColor('light', 'candles')).toContain(light.negative)
    expect(lightweightChartLegendColor('light', 'exits')).toContain(light.neutral)
  })
})
