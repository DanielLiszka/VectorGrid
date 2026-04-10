// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'

const resizeObserverState = vi.hoisted(() => ({
  callback: null as null | ((entries: Array<{ contentRect: DOMRectReadOnly }>) => void),
  stop: vi.fn(),
}))

vi.mock('@vueuse/core', () => ({
  useResizeObserver: (_target: unknown, callback: (entries: Array<{ contentRect: DOMRectReadOnly }>) => void) => {
    resizeObserverState.callback = callback
    resizeObserverState.stop = vi.fn()
    return {
      stop: resizeObserverState.stop,
    }
  },
}))

import { useLightweightChart } from './useLightweightChart'

describe('useLightweightChart', () => {
  beforeEach(() => {
    resizeObserverState.callback = null
    resizeObserverState.stop = vi.fn()
    class ResizeObserverMock {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }
    ;(globalThis as typeof globalThis & { ResizeObserver?: typeof ResizeObserver }).ResizeObserver =
      ResizeObserverMock as unknown as typeof ResizeObserver
  })

  it('observes host resizes and disposes the shared chart runtime cleanly', () => {
    const runtime = useLightweightChart<{ id: string }>()
    const host = document.createElement('div')
    const onResize = vi.fn()
    const removeChart = vi.fn()
    const chart = { id: 'chart_a' }

    runtime.setHost(host)
    runtime.setChart(chart)
    runtime.observeHostResize(onResize)

    const rect = { width: 640, height: 240 } as DOMRectReadOnly
    resizeObserverState.callback?.([{ contentRect: rect }])

    expect(onResize).toHaveBeenCalledWith(rect)

    runtime.dispose(removeChart)

    expect(resizeObserverState.stop).toHaveBeenCalledTimes(1)
    expect(removeChart).toHaveBeenCalledWith(chart)
    expect(runtime.chartRef.value).toBeNull()
    expect(runtime.hostRef.value).toBeNull()
  })

  it('stops the prior resize observer before registering a new one', () => {
    const runtime = useLightweightChart<{ id: string }>()
    runtime.setHost(document.createElement('div'))

    runtime.observeHostResize(() => undefined)
    const firstStop = resizeObserverState.stop

    runtime.observeHostResize(() => undefined)

    expect(firstStop).toHaveBeenCalledTimes(1)
  })
})
