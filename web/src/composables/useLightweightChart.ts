import { useResizeObserver } from '@vueuse/core'
import { markRaw, shallowRef, type ShallowRef } from 'vue'

export interface LightweightChartRuntime<TChart = unknown> {
  chartRef: ShallowRef<TChart | null>
  hostRef: ShallowRef<HTMLElement | null>
  setHost: (host: HTMLElement | null) => void
  setChart: (chart: TChart | null) => void
  observeHostResize: (onResize: (rect: DOMRectReadOnly) => void) => void
  stopHostResize: () => void
  dispose: (removeChart?: (chart: TChart) => void) => void
}

export function useLightweightChart<TChart = unknown>(): LightweightChartRuntime<TChart> {
  const chartRef = shallowRef<TChart | null>(null)
  const hostRef = shallowRef<HTMLElement | null>(null)
  let stopResizeObserver: (() => void) | null = null

  function setHost(host: HTMLElement | null): void {
    hostRef.value = host
  }

  function setChart(chart: TChart | null): void {
    if (chart && typeof chart === 'object') {
      chartRef.value = markRaw(chart as object) as TChart
      return
    }
    chartRef.value = chart
  }

  function stopHostResize(): void {
    if (!stopResizeObserver) return
    stopResizeObserver()
    stopResizeObserver = null
  }

  function observeHostResize(onResize: (rect: DOMRectReadOnly) => void): void {
    stopHostResize()
    if (typeof window === 'undefined' || typeof ResizeObserver === 'undefined') return
    const { stop } = useResizeObserver(hostRef, (entries) => {
      const first = entries[0]
      if (!first) return
      onResize(first.contentRect)
    })
    stopResizeObserver = stop
  }

  function dispose(removeChart?: (chart: TChart) => void): void {
    stopHostResize()
    const chart = chartRef.value
    if (chart && removeChart) {
      try {
        removeChart(chart)
      } catch {}
    }
    chartRef.value = null
    hostRef.value = null
  }

  return {
    chartRef,
    hostRef,
    setHost,
    setChart,
    observeHostResize,
    stopHostResize,
    dispose,
  }
}
