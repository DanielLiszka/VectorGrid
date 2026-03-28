import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import {
  recordLiveExecutionRunnerEvent,
  recordPolygonLiveStatusEvent,
  useTaskStore,
} from './taskStore'
import { bindRuntimePinia } from './runtimePinia'

class MemoryStorage implements Storage {
  private readonly values = new Map<string, string>()

  get length(): number {
    return this.values.size
  }

  clear(): void {
    this.values.clear()
  }

  getItem(key: string): string | null {
    return this.values.has(key) ? this.values.get(key) ?? null : null
  }

  key(index: number): string | null {
    const keys = Array.from(this.values.keys())
    return index >= 0 && index < keys.length ? (keys[index] ?? null) : null
  }

  removeItem(key: string): void {
    this.values.delete(key)
  }

  setItem(key: string, value: string): void {
    this.values.set(key, String(value))
  }
}

describe('task-store event throttling', () => {
  let pinia: Pinia
  let warnSpy: ReturnType<typeof vi.spyOn>
  let errorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    ;(globalThis as Record<string, unknown>).window = { localStorage: new MemoryStorage() }
    const store = useTaskStore(pinia)
    store.$reset()
  })

  afterEach(() => {
    warnSpy.mockRestore()
    errorSpy.mockRestore()
  })

  it('coalesces repeated live runner events inside the throttle window', () => {
    let now = Date.now() + 10_000
    const nowSpy = vi.spyOn(Date, 'now').mockImplementation(() => now)

    recordLiveExecutionRunnerEvent({
      status: 'running',
      message: 'tick',
      symbol: 'AAPL',
      at_unix_ms: now,
    })

    now += 50
    recordLiveExecutionRunnerEvent({
      status: 'running',
      message: 'tick',
      symbol: 'AAPL',
      at_unix_ms: now,
    })

    now += 50
    recordLiveExecutionRunnerEvent({
      status: 'running',
      message: 'tick',
      symbol: 'AAPL',
      at_unix_ms: now,
    })

    now += 10
    recordLiveExecutionRunnerEvent({
      status: 'running',
      message: 'tick-2',
      symbol: 'AAPL',
      at_unix_ms: now,
    })

    const store = useTaskStore(pinia)
    const runnerEvents = store.events.filter((evt) => evt.source === 'live_execution_runner_event')
    expect(runnerEvents.length).toBe(2)
    expect(runnerEvents[0]?.message).toContain('tick-2')
    expect(runnerEvents[1]?.message).toContain('tick')

    nowSpy.mockRestore()
  })

  it('coalesces repeated polygon status events inside the throttle window', () => {
    let now = Date.now() + 20_000
    const nowSpy = vi.spyOn(Date, 'now').mockImplementation(() => now)

    recordPolygonLiveStatusEvent({
      status: 'connected',
      message: 'ok',
      market: 'stocks',
      symbol: 'MSFT',
      delayed: false,
      channel: 'A',
      at_unix_ms: now,
    })

    now += 80
    recordPolygonLiveStatusEvent({
      status: 'connected',
      message: 'ok',
      market: 'stocks',
      symbol: 'MSFT',
      delayed: false,
      channel: 'A',
      at_unix_ms: now,
    })

    now += 90
    recordPolygonLiveStatusEvent({
      status: 'error',
      message: 'disconnect',
      market: 'stocks',
      symbol: 'MSFT',
      delayed: false,
      channel: 'A',
      at_unix_ms: now,
    })

    const store = useTaskStore(pinia)
    const polygonEvents = store.events.filter((evt) => evt.source === 'polygon_live_status')
    expect(polygonEvents.length).toBe(2)
    expect(polygonEvents[0]?.level).toBe('error')
    expect(polygonEvents[0]?.message).toContain('disconnect')

    nowSpy.mockRestore()
  })
})
