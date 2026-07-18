export interface ChartSize {
  width: number
  height: number
}

export function normalizeChartSize(widthRaw: unknown, heightRaw: unknown): ChartSize {
  const width = Math.max(240, Math.floor(Number(widthRaw) || 820))
  const height = Math.max(220, Math.floor(Number(heightRaw) || 360))
  return { width, height }
}

export function baseLightweightChartOptions(size: ChartSize): Record<string, unknown> {
  return {
    width: size.width,
    height: size.height,
    layout: {
      textColor: 'rgba(226, 232, 240, 0.95)',
      background: { type: 'solid', color: 'rgb(2, 6, 23)' },
    },
    grid: {
      vertLines: { color: 'rgba(100, 116, 139, 0.34)' },
      horzLines: { color: 'rgba(100, 116, 139, 0.34)' },
    },
    rightPriceScale: {
      borderColor: 'rgba(148, 163, 184, 0.55)',
      entireTextOnly: true,
    },
    overlayPriceScales: {
      borderVisible: false,
      scaleMargins: { top: 0.78, bottom: 0.03 },
    },
    timeScale: {
      borderColor: 'rgba(148, 163, 184, 0.55)',
      fixLeftEdge: false,
      fixRightEdge: false,
      timeVisible: true,
      secondsVisible: false,
    },
    crosshair: {
      vertLine: { color: 'rgba(226, 232, 240, 0.58)', labelBackgroundColor: 'rgb(15, 23, 42)' },
      horzLine: { color: 'rgba(226, 232, 240, 0.58)', labelBackgroundColor: 'rgb(15, 23, 42)' },
    },
  }
}

export function candlestickSeriesOptions(): Record<string, unknown> {
  return {
    upColor: 'rgba(22, 163, 74, 0.9)',
    downColor: 'rgba(225, 29, 72, 0.9)',
    wickUpColor: 'rgba(22, 163, 74, 0.9)',
    wickDownColor: 'rgba(225, 29, 72, 0.9)',
    borderVisible: true,
    borderUpColor: 'rgba(22, 163, 74, 1)',
    borderDownColor: 'rgba(225, 29, 72, 1)',
  }
}

export function lineSeriesOptions(kind: 'fast' | 'slow' | 'ott'): Record<string, unknown> {
  const color =
    kind === 'fast'
      ? 'rgba(14, 165, 233, 0.95)'
      : kind === 'slow'
        ? 'rgba(245, 158, 11, 0.95)'
        : 'rgba(168, 85, 247, 0.95)'
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

export function histogramSeriesOptions(kind: HistogramSeriesKind = 'macz'): Record<string, unknown> {
  return {
    color: kind === 'halftrend' ? 'rgba(34, 197, 94, 0.42)' : 'rgba(148, 163, 184, 0.6)',
    priceScaleId: kind,
    priceLineVisible: false,
    lastValueVisible: false,
    base: 0,
  }
}
