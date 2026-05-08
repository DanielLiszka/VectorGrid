import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const hasTauriEventApiMock = vi.fn(() => false)
const listenEventMock = vi.fn<
  (event: string, handler: unknown) => Promise<() => void>
>(async (_event, _handler) => () => undefined)
const resetSharedListenersMock = vi.fn()
const setInvokeLifecycleHooksMock = vi.fn<(hooks: unknown) => void>()
const resetInvokeLifecycleHooksMock = vi.fn()

vi.mock('./events', () => ({
  hasTauriEventApi: () => hasTauriEventApiMock(),
  listenEvent: (event: string, handler: unknown) => listenEventMock(event, handler),
  resetSharedTauriEventListenersForTests: () => resetSharedListenersMock(),
}))

vi.mock('./invoke', () => ({
  setInvokeLifecycleHooks: (hooks: unknown) => setInvokeLifecycleHooksMock(hooks),
  resetInvokeLifecycleHooksForTests: () => resetInvokeLifecycleHooksMock(),
}))

import {
  INSTRUMENTED_EVENT_CHANNELS,
  installTauriInstrumentation,
  resetTauriInstrumentationForTests,
} from './instrumentation'

async function flushMicrotasks(): Promise<void> {
  for (let i = 0; i < 10; i += 1) {
    await Promise.resolve()
  }
}

describe('tauri instrumentation retry wiring', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    resetTauriInstrumentationForTests()
    hasTauriEventApiMock.mockReset()
    hasTauriEventApiMock.mockReturnValue(false)
    listenEventMock.mockReset()
    listenEventMock.mockImplementation(async (_event, _handler) => () => undefined)
    resetSharedListenersMock.mockClear()
    setInvokeLifecycleHooksMock.mockClear()
    resetInvokeLifecycleHooksMock.mockClear()
  })

  afterEach(async () => {
    resetTauriInstrumentationForTests()
    await vi.runOnlyPendingTimersAsync()
    vi.useRealTimers()
  })

  it('retries event subscriptions until the Tauri runtime becomes available', async () => {
    installTauriInstrumentation()
    await flushMicrotasks()

    expect(listenEventMock).not.toHaveBeenCalled()
    expect(setInvokeLifecycleHooksMock).toHaveBeenCalledTimes(1)

    hasTauriEventApiMock.mockReturnValue(true)
    await vi.advanceTimersByTimeAsync(100)
    await flushMicrotasks()

    expect(listenEventMock.mock.calls.map(([event]) => event)).toEqual([
      ...INSTRUMENTED_EVENT_CHANNELS,
    ])
  })

  it('retries only event channels that fail during the initial subscription', async () => {
    hasTauriEventApiMock.mockReturnValue(true)
    const failingEvent = 'polygon_live_status'
    const attempts = new Map<string, number>()

    listenEventMock.mockImplementation(async (event, _handler) => {
      attempts.set(event, (attempts.get(event) ?? 0) + 1)
      if (event === failingEvent && attempts.get(event) === 1) {
        throw new Error('temporary event API failure')
      }
      return () => undefined
    })

    installTauriInstrumentation()
    await flushMicrotasks()

    expect(listenEventMock.mock.calls.map(([event]) => event)).toEqual([
      ...INSTRUMENTED_EVENT_CHANNELS,
    ])
    expect(attempts.get(failingEvent)).toBe(1)

    await vi.advanceTimersByTimeAsync(100)
    await flushMicrotasks()

    expect(attempts.get(failingEvent)).toBe(2)
    for (const event of INSTRUMENTED_EVENT_CHANNELS.filter((event) => event !== failingEvent)) {
      expect(attempts.get(event)).toBe(1)
    }
  })

  it('clears pending retry work during explicit test reset', async () => {
    installTauriInstrumentation()
    await flushMicrotasks()

    resetTauriInstrumentationForTests()
    hasTauriEventApiMock.mockReturnValue(true)
    await vi.advanceTimersByTimeAsync(500)

    expect(listenEventMock).not.toHaveBeenCalled()
    expect(resetSharedListenersMock).toHaveBeenCalled()
    expect(setInvokeLifecycleHooksMock).toHaveBeenLastCalledWith(null)
    expect(resetInvokeLifecycleHooksMock).toHaveBeenCalled()
  })
})
