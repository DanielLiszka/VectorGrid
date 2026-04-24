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
      textColor: 'rgba(203, 213, 225, 0.92)',
      background: { type: 'solid', color: 'rgba(2, 6, 23, 0)' },
    },
    grid: {
      vertLines: { color: 'rgba(51, 65, 85, 0.35)' },
      horzLines: { color: 'rgba(51, 65, 85, 0.35)' },
    },
    rightPriceScale: { borderColor: 'rgba(100, 116, 139, 0.45)' },
    timeScale: {
      borderColor: 'rgba(100, 116, 139, 0.45)',
      fixLeftEdge: false,
      fixRightEdge: false,
    },
    crosshair: {
      vertLine: { color: 'rgba(148, 163, 184, 0.45)' },
      horzLine: { color: 'rgba(148, 163, 184, 0.45)' },
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

export function lineSeriesOptions(kind: 'fast' | 'slow' | 'halftrend'): Record<string, unknown> {
  const color =
    kind === 'fast'
      ? 'rgba(14, 165, 233, 0.95)'
      : kind === 'slow'
        ? 'rgba(245, 158, 11, 0.95)'
        : 'rgba(125, 211, 252, 0.95)'
  return {
    color,
    lineWidth: kind === 'halftrend' ? 1 : 2,
    crosshairMarkerVisible: false,
    lastValueVisible: false,
    priceLineVisible: false,
    ...(kind === 'halftrend' ? { lineStyle: 2 } : {}),
  }
}

export function histogramSeriesOptions(): Record<string, unknown> {
  return {
    color: 'rgba(148, 163, 184, 0.6)',
    priceLineVisible: false,
    lastValueVisible: false,
    base: 0,
  }
}
