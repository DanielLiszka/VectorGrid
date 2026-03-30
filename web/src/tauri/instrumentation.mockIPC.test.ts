// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import { emit } from '@tauri-apps/api/event'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useTaskStore } from '../stores/taskStore'
import { installTauriInstrumentation } from './instrumentation'
import { installLegacyTauriWindowBridge, installMockIpc, resetTauriApiMocks } from '../test/tauriMocks'

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

function setMemoryStorage(): void {
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: new MemoryStorage(),
  })
}

async function flushAsyncWork(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
  await Promise.resolve()
}

describe('Tauri instrumentation with mockIPC', () => {
  let pinia: Pinia
  let warnSpy: ReturnType<typeof vi.spyOn>
  let errorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    resetTauriApiMocks()
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    setMemoryStorage()
    warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const store = useTaskStore(pinia)
    store.$reset()
  })

  afterEach(() => {
    warnSpy.mockRestore()
    errorSpy.mockRestore()
    resetTauriApiMocks()
  })

  it('tracks a successful mocked invoke from begin to completion', async () => {
    installMockIpc(
      {
        run_meta_selection: async () => ({ ok: true }),
      },
      { shouldMockEvents: true },
    )
    installLegacyTauriWindowBridge()
    installTauriInstrumentation()

    await expect(window.__TAURI__?.core?.invoke?.('run_meta_selection')).resolves.toEqual({ ok: true })

    const store = useTaskStore(pinia)
    expect(store.history[0]).toMatchObject({
      command: 'run_meta_selection',
      status: 'success',
    })
    expect(store.centerStatus).toBe('success')
  })

  it('tracks a failed mocked invoke through the task store', async () => {
    installMockIpc(
      {
        run_meta_selection: () => {
          throw new Error('backend panic')
        },
      },
      { shouldMockEvents: true },
    )
    installLegacyTauriWindowBridge()
    installTauriInstrumentation()

    await expect(window.__TAURI__?.core?.invoke?.('run_meta_selection')).rejects.toThrow('backend panic')

    const store = useTaskStore(pinia)
    expect(store.history[0]).toMatchObject({
      command: 'run_meta_selection',
      status: 'failed',
    })
    expect(store.events[0]?.message).toContain('backend panic')
  })

  it('ignores malformed events and records valid runner events', async () => {
    installMockIpc({}, { allowUnknown: true, shouldMockEvents: true })
    installLegacyTauriWindowBridge()
    installTauriInstrumentation()
    await flushAsyncWork()

    await emit('live_execution_runner_event', 'bad payload')
    await emit('live_execution_runner_event', {
      status: 'running',
      message: 'heartbeat',
      symbol: 'AAPL',
      at_unix_ms: 1_700_000_000_000,
    })
    await flushAsyncWork()

    const store = useTaskStore(pinia)
    const runnerEvents = store.events.filter((event) => event.source === 'live_execution_runner_event')
    expect(runnerEvents).toHaveLength(1)
    expect(runnerEvents[0]?.message).toContain('heartbeat')
  })
})
