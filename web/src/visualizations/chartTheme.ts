export interface ChartSize {
  width: number
  height: number
}

export type LightweightChartTheme = 'light' | 'dark'

export interface LightweightChartSeriesPalette {
  canvas: string
  positive: string
  negative: string
  fast: string
  slow: string
  ott: string
  neutral: string
  highlight: string
}

export type LightweightChartSeriesTone =
  | 'positive'
  | 'negative'
  | 'fast'
  | 'slow'
  | 'ott'
  | 'neutral'
  | 'highlight'

export type LightweightChartLegendPaletteKey =
  | 'candles'
  | 'fast'
  | 'slow'
  | 'halftrend'
  | 'ott'
  | 'macz'
  | 'entries'
  | 'exits'
  | 'orders'

export interface LightweightChartLegendItem {
  label: string
  paletteKey?: LightweightChartLegendPaletteKey
  color?: string
  value?: string
}

export function normalizeChartSize(widthRaw: unknown, heightRaw: unknown): ChartSize {
  const width = Math.max(240, Math.floor(Number(widthRaw) || 820))
  const height = Math.max(220, Math.floor(Number(heightRaw) || 360))
  return { width, height }
}

export function resolveLightweightChartTheme(
  root: Pick<HTMLElement, 'dataset'> | null | undefined =
    typeof document !== 'undefined' ? document.documentElement : null,
): LightweightChartTheme {
  return String(root?.dataset.theme || '').trim().toLowerCase() === 'dark'
    ? 'dark'
    : 'light'
}

export function lightweightChartSeriesPalette(
  theme: LightweightChartTheme,
): Readonly<LightweightChartSeriesPalette> {
  if (theme === 'dark') {
    return {
      canvas: 'rgb(2, 6, 23)',
      positive: '#22c55e',
      negative: '#fb7185',
      fast: '#38bdf8',
      slow: '#fbbf24',
      ott: '#c084fc',
      neutral: '#cbd5e1',
      highlight: '#fde047',
    }
  }
  return {
    canvas: 'rgb(255, 253, 250)',
    positive: '#047857',
    negative: '#be123c',
    fast: '#0369a1',
    slow: '#92400e',
    ott: '#7e22ce',
    neutral: '#475569',
    highlight: '#854d0e',
  }
}

export function lightweightChartSeriesColor(
  theme: LightweightChartTheme,
  tone: LightweightChartSeriesTone,
): string {
  return lightweightChartSeriesPalette(theme)[tone]
}

export function lightweightChartLegendColor(
  theme: LightweightChartTheme,
  key: LightweightChartLegendPaletteKey,
): string {
  const palette = lightweightChartSeriesPalette(theme)
  if (key === 'fast' || key === 'slow' || key === 'ott') return palette[key]
  if (key === 'exits' || key === 'orders') {
    return `linear-gradient(90deg, ${palette.positive} 0 33.333%, ${palette.negative} 33.333% 66.666%, ${palette.neutral} 66.666% 100%)`
  }
  return `linear-gradient(90deg, ${palette.positive} 0 50%, ${palette.negative} 50% 100%)`
}

export function colorizeHistogramSeriesData<T extends { value: number }>(
  data: readonly T[],
  theme: LightweightChartTheme,
): Array<T & { color: string }> {
  const palette = lightweightChartSeriesPalette(theme)
  return data.map((point) => ({
    ...point,
    color: Number(point.value) >= 0 ? palette.positive : palette.negative,
  }))
}

export function lightweightChartThemeOptions(
  theme: LightweightChartTheme,
): Record<string, unknown> {
  const dark = theme === 'dark'
  const seriesPalette = lightweightChartSeriesPalette(theme)
  const textColor = dark
    ? 'rgba(226, 232, 240, 0.95)'
    : 'rgba(24, 34, 50, 0.94)'
  const gridColor = dark
    ? 'rgba(100, 116, 139, 0.34)'
    : 'rgba(90, 102, 119, 0.22)'
  const borderColor = dark
    ? 'rgba(148, 163, 184, 0.55)'
    : 'rgba(90, 102, 119, 0.48)'
  const crosshairColor = dark
    ? 'rgba(226, 232, 240, 0.58)'
    : 'rgba(24, 34, 50, 0.48)'
  const crosshairLabelBackground = dark ? 'rgb(15, 23, 42)' : 'rgb(59, 71, 87)'
  return {
    layout: {
      textColor,
      background: { type: 'solid', color: seriesPalette.canvas },
      attributionLogo: true,
    },
    grid: {
      vertLines: { color: gridColor },
      horzLines: { color: gridColor },
    },
    rightPriceScale: {
      borderColor,
      entireTextOnly: true,
    },
    timeScale: {
      borderColor,
    },
    crosshair: {
      vertLine: { color: crosshairColor, labelBackgroundColor: crosshairLabelBackground },
      horzLine: { color: crosshairColor, labelBackgroundColor: crosshairLabelBackground },
    },
  }
}

export function baseLightweightChartOptions(
  size: ChartSize,
  theme: LightweightChartTheme = resolveLightweightChartTheme(),
): Record<string, unknown> {
  const themeOptions = lightweightChartThemeOptions(theme)
  return {
    width: size.width,
    height: size.height,
    ...themeOptions,
    rightPriceScale: {
      ...(themeOptions.rightPriceScale as Record<string, unknown>),
    },
    overlayPriceScales: {
      borderVisible: false,
      scaleMargins: { top: 0.78, bottom: 0.03 },
    },
    timeScale: {
      ...(themeOptions.timeScale as Record<string, unknown>),
      fixLeftEdge: false,
      fixRightEdge: false,
      timeVisible: true,
      secondsVisible: false,
    },
  }
}

export function candlestickSeriesOptions(
  theme: LightweightChartTheme = resolveLightweightChartTheme(),
): Record<string, unknown> {
  const palette = lightweightChartSeriesPalette(theme)
  return {
    upColor: palette.positive,
    downColor: palette.negative,
    wickUpColor: palette.positive,
    wickDownColor: palette.negative,
    borderVisible: true,
    borderUpColor: palette.positive,
    borderDownColor: palette.negative,
  }
}

export function lineSeriesOptions(
  kind: 'fast' | 'slow' | 'ott',
  theme: LightweightChartTheme = resolveLightweightChartTheme(),
): Record<string, unknown> {
  const color = lightweightChartSeriesPalette(theme)[kind]
  return {
    color,
    lineWidth: 2,
    crosshairMarkerVisible: false,
    lastValueVisible: false,
    priceLineVisible: false,
    ...(kind === 'ott' ? { lineStyle: 2 } : {}),
  }
}

export type HistogramSeriesKind = 'halftrend' | 'macz'

export function histogramPriceScaleOptions(kind: HistogramSeriesKind): Record<string, unknown> {
  return {
    borderVisible: false,
    scaleMargins:
      kind === 'halftrend'
        ? { top: 0.18, bottom: 0.70 }
        : { top: 0.78, bottom: 0.03 },
  }
}

export function histogramSeriesOptions(
  kind: HistogramSeriesKind = 'macz',
  theme: LightweightChartTheme = resolveLightweightChartTheme(),
): Record<string, unknown> {
  const palette = lightweightChartSeriesPalette(theme)
  return {
    color: kind === 'halftrend' ? palette.positive : palette.neutral,
    priceScaleId: kind,
    priceLineVisible: false,
    lastValueVisible: false,
    base: 0,
  }
}
