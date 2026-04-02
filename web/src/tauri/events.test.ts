import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  listenEvent,
  resetSharedTauriEventListenersForTests,
} from './events'

type NativeEventCallback = (event: {
  event: string
  payload: unknown
  id?: number
}) => void

const nativeListeners = new Map<string, NativeEventCallback>()
const nativeOffByEvent = new Map<string, ReturnType<typeof vi.fn>>()
const isTauriMock = vi.fn(() => true)
const listenMock = vi.fn(async (event: string, handler: NativeEventCallback) => {
  nativeListeners.set(event, handler)
  const off = vi.fn(() => {
    nativeListeners.delete(event)
  })
  nativeOffByEvent.set(event, off)
  return off
})

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => isTauriMock(),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: NativeEventCallback) => listenMock(event, handler),
}))

describe('shared Tauri event listeners', () => {
  beforeEach(() => {
    resetSharedTauriEventListenersForTests()
    nativeListeners.clear()
    nativeOffByEvent.clear()
    listenMock.mockClear()
    isTauriMock.mockReset()
    isTauriMock.mockReturnValue(true)
  })

  afterEach(() => {
    resetSharedTauriEventListenersForTests()
  })

  it('reuses one native listener for multiple subscribers on the same event', async () => {
    const first = vi.fn()
    const second = vi.fn()

    const offFirst = await listenEvent('live_execution_runner_event', first)
    const offSecond = await listenEvent('live_execution_runner_event', second)

    expect(listenMock).toHaveBeenCalledTimes(1)

    nativeListeners.get('live_execution_runner_event')?.({
      event: 'live_execution_runner_event',
      id: 7,
      payload: { status: 'running' },
    })

    expect(first).toHaveBeenCalledWith({ status: 'running' }, {
      event: 'live_execution_runner_event',
      id: 7,
    })
    expect(second).toHaveBeenCalledWith({ status: 'running' }, {
      event: 'live_execution_runner_event',
      id: 7,
    })

    offFirst?.()
    expect(nativeOffByEvent.get('live_execution_runner_event')).not.toHaveBeenCalled()

    offSecond?.()
    expect(nativeOffByEvent.get('live_execution_runner_event')).toHaveBeenCalledTimes(1)
  })

  it('keeps sibling handlers alive when one subscriber throws', async () => {
    const noisy = vi.fn(() => {
      throw new Error('bad subscriber')
    })
    const stable = vi.fn()

    await listenEvent('polygon_live_status', noisy)
    await listenEvent('polygon_live_status', stable)

    nativeListeners.get('polygon_live_status')?.({
      event: 'polygon_live_status',
      payload: { active: true },
    })

    expect(noisy).toHaveBeenCalledTimes(1)
    expect(stable).toHaveBeenCalledWith({ active: true }, {
      event: 'polygon_live_status',
    })
  })

  it('tears down native listeners during explicit test reset', async () => {
    await listenEvent('polygon_live_bar', vi.fn())

    expect(listenMock).toHaveBeenCalledTimes(1)

    resetSharedTauriEventListenersForTests()

    expect(nativeOffByEvent.get('polygon_live_bar')).toHaveBeenCalledTimes(1)
    expect(nativeListeners.has('polygon_live_bar')).toBe(false)
  })
})
